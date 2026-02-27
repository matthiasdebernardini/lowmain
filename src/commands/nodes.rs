use agcli::{ActionParam, Command, CommandOutput, NextAction};
use serde_json::json;

use crate::convert;
use crate::error::{AppError, map_neo4j_error};
use crate::neo4j_client;

fn find_command() -> Command {
    Command::new("find", "Find nodes by label and optional filters")
        .usage("lowmain node find --label=<label> [--where=<prop=val>] [--limit=<n>]")
        .handler(|req, ctx| {
            Box::pin(async move {
                let label = req.flag("label").ok_or(AppError::InvalidParams {
                    reason: "Missing --label. Usage: lowmain node find --label=Person".into(),
                })?;

                let limit: usize = req
                    .flag("limit")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(100);

                let graph = neo4j_client::from_request(req, ctx).await?;

                let (cypher, q) = if let Some(where_clause) = req.flag("where") {
                    // Parse prop=val
                    let (prop, val) = where_clause.split_once('=').ok_or(AppError::InvalidParams {
                        reason: "Invalid --where format. Use prop=value".into(),
                    })?;
                    let cypher = format!(
                        "MATCH (n:`{label}`) WHERE n.`{prop}` = $val RETURN n LIMIT {limit}"
                    );
                    let q = neo4rs::query(&cypher).param("val", val);
                    (cypher, q)
                } else {
                    let cypher = format!("MATCH (n:`{label}`) RETURN n LIMIT {limit}");
                    let q = neo4rs::query(&cypher);
                    (cypher, q)
                };

                let mut result = graph.execute(q).await.map_err(map_neo4j_error)?;
                let mut nodes = Vec::new();

                while let Some(row) = result.next().await.map_err(map_neo4j_error)? {
                    if let Ok(node) = row.get::<neo4rs::Node>("n") {
                        nodes.push(convert::node_to_json(&node));
                    }
                }

                let count = nodes.len();
                let node_ids: Vec<_> = nodes
                    .iter()
                    .filter_map(|n| n.get("_id").and_then(|v| v.as_i64()))
                    .collect();

                let mut next_actions: Vec<NextAction> = node_ids
                    .iter()
                    .take(5)
                    .map(|id| {
                        NextAction::new(
                            format!("lowmain node get {id}"),
                            format!("Get node {id} details"),
                        )
                    })
                    .collect();

                next_actions.push(
                    NextAction::new(
                        format!("lowmain node create --label={label}"),
                        format!("Create a new {label} node"),
                    )
                    .with_param("--props", ActionParam::new().description("JSON properties").required(true)),
                );

                Ok(CommandOutput::new(json!({
                    "cypher": cypher,
                    "nodes": nodes,
                    "count": count,
                    "label": label,
                }))
                .next_actions(next_actions))
            })
        })
}

fn get_command() -> Command {
    Command::new("get", "Get a node by internal ID")
        .usage("lowmain node get <id>")
        .handler(|req, ctx| {
            Box::pin(async move {
                let id_str = req.arg(0).ok_or(AppError::InvalidParams {
                    reason: "Missing node ID. Usage: lowmain node get <id>".into(),
                })?;
                let id: i64 = id_str.parse().map_err(|_| AppError::InvalidParams {
                    reason: format!("Invalid node ID: {id_str}"),
                })?;

                let graph = neo4j_client::from_request(req, ctx).await?;

                let mut result = graph
                    .execute(
                        neo4rs::query("MATCH (n) WHERE elementId(n) = toString($id) OR id(n) = $id RETURN n")
                            .param("id", id),
                    )
                    .await
                    .map_err(map_neo4j_error)?;

                let row = result
                    .next()
                    .await
                    .map_err(map_neo4j_error)?
                    .ok_or(AppError::NodeNotFound { id: id_str.to_string() })?;

                let node = row.get::<neo4rs::Node>("n").map_err(|e| AppError::QueryFailed {
                    reason: e.to_string(),
                })?;
                let node_json = convert::node_to_json(&node);

                Ok(CommandOutput::new(json!({ "node": node_json }))
                    .next_action(
                        NextAction::new(format!("lowmain node update {id}"), "Update this node")
                            .with_param("--set", ActionParam::new().description("JSON properties to set").required(true)),
                    )
                    .next_action(
                        NextAction::new(format!("lowmain node delete {id}"), "Delete this node"),
                    )
                    .next_action(
                        NextAction::new(
                            format!("lowmain rel find --from={id}"),
                            "Find outgoing relationships",
                        ),
                    )
                    .next_action(
                        NextAction::new(
                            format!("lowmain rel find --to={id}"),
                            "Find incoming relationships",
                        ),
                    )
                    .next_action(
                        NextAction::new(format!("lowmain rel create --from={id}"), "Create relationship from this node")
                            .with_param("--to", ActionParam::new().description("Target node ID").required(true))
                            .with_param("--type", ActionParam::new().description("Relationship type").required(true)),
                    ))
            })
        })
}

fn create_command() -> Command {
    Command::new("create", "Create a new node")
        .usage("lowmain node create --label=<label> --props=<json>")
        .handler(|req, ctx| {
            Box::pin(async move {
                let label = req.flag("label").ok_or(AppError::InvalidParams {
                    reason: "Missing --label. Usage: lowmain node create --label=Person --props='{\"name\":\"Alice\"}'".into(),
                })?;

                let props_str = req.flag("props").ok_or(AppError::InvalidParams {
                    reason: "Missing --props. Provide a JSON object of properties".into(),
                })?;

                let props: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_str(props_str).map_err(|e| AppError::InvalidParams {
                        reason: format!("Invalid --props JSON: {e}"),
                    })?;

                let graph = neo4j_client::from_request(req, ctx).await?;

                // Build SET clause from properties
                let set_clause: String = props
                    .keys()
                    .map(|k| format!("n.`{k}` = $`{k}`"))
                    .collect::<Vec<_>>()
                    .join(", ");

                let cypher = if set_clause.is_empty() {
                    format!("CREATE (n:`{label}`) RETURN n")
                } else {
                    format!("CREATE (n:`{label}`) SET {set_clause} RETURN n")
                };

                let mut q = neo4rs::query(&cypher);
                for (key, val) in &props {
                    q = match val {
                        serde_json::Value::String(s) => q.param(key.as_str(), s.clone()),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                q.param(key.as_str(), i)
                            } else if let Some(f) = n.as_f64() {
                                q.param(key.as_str(), f)
                            } else {
                                q.param(key.as_str(), n.to_string())
                            }
                        }
                        serde_json::Value::Bool(b) => q.param(key.as_str(), *b),
                        _ => q.param(key.as_str(), val.to_string()),
                    };
                }

                let mut result = graph.execute(q).await.map_err(map_neo4j_error)?;
                let row = result
                    .next()
                    .await
                    .map_err(map_neo4j_error)?
                    .ok_or(AppError::QueryFailed {
                        reason: "CREATE did not return a node".into(),
                    })?;

                let node = row.get::<neo4rs::Node>("n").map_err(|e| AppError::QueryFailed {
                    reason: e.to_string(),
                })?;
                let node_json = convert::node_to_json(&node);
                let new_id = node.id();

                Ok(CommandOutput::new(json!({
                    "created": true,
                    "node": node_json,
                }))
                .next_action(NextAction::new(
                    format!("lowmain node get {new_id}"),
                    "View created node",
                ))
                .next_action(
                    NextAction::new(format!("lowmain rel create --from={new_id}"), "Create relationship from this node")
                        .with_param("--to", ActionParam::new().description("Target node ID").required(true))
                        .with_param("--type", ActionParam::new().description("Relationship type").required(true)),
                )
                .next_action(NextAction::new(
                    format!("lowmain node find --label={label}"),
                    format!("Find all {label} nodes"),
                )))
            })
        })
}

fn update_command() -> Command {
    Command::new("update", "Update a node's properties")
        .usage("lowmain node update <id> --set=<json>")
        .handler(|req, ctx| {
            Box::pin(async move {
                let id_str = req.arg(0).ok_or(AppError::InvalidParams {
                    reason: "Missing node ID. Usage: lowmain node update <id> --set='{\"name\":\"Bob\"}'".into(),
                })?;
                let id: i64 = id_str.parse().map_err(|_| AppError::InvalidParams {
                    reason: format!("Invalid node ID: {id_str}"),
                })?;

                let set_str = req.flag("set").ok_or(AppError::InvalidParams {
                    reason: "Missing --set. Provide a JSON object of properties to update".into(),
                })?;

                let props: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_str(set_str).map_err(|e| AppError::InvalidParams {
                        reason: format!("Invalid --set JSON: {e}"),
                    })?;

                let graph = neo4j_client::from_request(req, ctx).await?;

                let set_clause: String = props
                    .keys()
                    .map(|k| format!("n.`{k}` = $`{k}`"))
                    .collect::<Vec<_>>()
                    .join(", ");

                let cypher = format!("MATCH (n) WHERE id(n) = $id SET {set_clause} RETURN n");
                let mut q = neo4rs::query(&cypher).param("id", id);

                for (key, val) in &props {
                    q = match val {
                        serde_json::Value::String(s) => q.param(key.as_str(), s.clone()),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                q.param(key.as_str(), i)
                            } else if let Some(f) = n.as_f64() {
                                q.param(key.as_str(), f)
                            } else {
                                q.param(key.as_str(), n.to_string())
                            }
                        }
                        serde_json::Value::Bool(b) => q.param(key.as_str(), *b),
                        _ => q.param(key.as_str(), val.to_string()),
                    };
                }

                let mut result = graph.execute(q).await.map_err(map_neo4j_error)?;
                let row = result
                    .next()
                    .await
                    .map_err(map_neo4j_error)?
                    .ok_or(AppError::NodeNotFound { id: id_str.to_string() })?;

                let node = row.get::<neo4rs::Node>("n").map_err(|e| AppError::QueryFailed {
                    reason: e.to_string(),
                })?;
                let node_json = convert::node_to_json(&node);

                Ok(CommandOutput::new(json!({
                    "updated": true,
                    "node": node_json,
                }))
                .next_action(NextAction::new(
                    format!("lowmain node get {id}"),
                    "View updated node",
                ))
                .next_action(NextAction::new(
                    format!("lowmain node delete {id}"),
                    "Delete this node",
                )))
            })
        })
}

fn delete_command() -> Command {
    Command::new("delete", "Delete a node by ID")
        .usage("lowmain node delete <id> [--detach]")
        .handler(|req, ctx| {
            Box::pin(async move {
                let id_str = req.arg(0).ok_or(AppError::InvalidParams {
                    reason: "Missing node ID. Usage: lowmain node delete <id>".into(),
                })?;
                let id: i64 = id_str.parse().map_err(|_| AppError::InvalidParams {
                    reason: format!("Invalid node ID: {id_str}"),
                })?;

                let detach = req.flag("detach").is_some();
                let graph = neo4j_client::from_request(req, ctx).await?;

                let cypher = if detach {
                    "MATCH (n) WHERE id(n) = $id DETACH DELETE n RETURN count(n) AS deleted"
                } else {
                    "MATCH (n) WHERE id(n) = $id DELETE n RETURN count(n) AS deleted"
                };

                let mut result = graph
                    .execute(neo4rs::query(cypher).param("id", id))
                    .await
                    .map_err(map_neo4j_error)?;

                let deleted: i64 = result
                    .next()
                    .await
                    .map_err(map_neo4j_error)?
                    .and_then(|r| r.get("deleted").ok())
                    .unwrap_or(0);

                if deleted == 0 {
                    return Err(AppError::NodeNotFound { id: id_str.to_string() }.into());
                }

                Ok(CommandOutput::new(json!({
                    "deleted": true,
                    "id": id,
                    "detach": detach,
                }))
                .next_action(NextAction::new("lowmain schema", "Explore database structure"))
                .next_action(
                    NextAction::new("lowmain node find", "Find nodes")
                        .with_param("--label", ActionParam::new().description("Node label").required(true)),
                ))
            })
        })
}

pub fn register() -> Command {
    Command::new("node", "Node CRUD operations")
        .usage("lowmain node [find|get|create|update|delete]")
        .subcommand(find_command())
        .subcommand(get_command())
        .subcommand(create_command())
        .subcommand(update_command())
        .subcommand(delete_command())
}
