use agcli::{ActionParam, Command, CommandOutput, NextAction};
use serde_json::json;

use crate::convert;
use crate::error::{AppError, map_neo4j_error};
use crate::neo4j_client;

fn find_command() -> Command {
    Command::new("find", "Find relationships by type and/or endpoints")
        .usage("lowmain rel find [--from=<id>] [--to=<id>] [--type=<type>] [--limit=<n>]")
        .handler(|req, ctx| {
            Box::pin(async move {
                let limit: usize = req
                    .flag("limit")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(100);

                let from_id = req.flag("from").and_then(|v| v.parse::<i64>().ok());
                let to_id = req.flag("to").and_then(|v| v.parse::<i64>().ok());
                let rel_type = req.flag("type");

                let graph = neo4j_client::from_request(req, ctx).await?;

                // Build Cypher dynamically
                let rel_pattern = rel_type
                    .map(|t| format!("[r:`{t}`]"))
                    .unwrap_or_else(|| "[r]".to_string());

                let mut where_clauses = Vec::new();
                if from_id.is_some() {
                    where_clauses.push("id(a) = $from_id".to_string());
                }
                if to_id.is_some() {
                    where_clauses.push("id(b) = $to_id".to_string());
                }

                let where_str = if where_clauses.is_empty() {
                    String::new()
                } else {
                    format!(" WHERE {}", where_clauses.join(" AND "))
                };

                let cypher = format!(
                    "MATCH (a)-{rel_pattern}->(b){where_str} RETURN r, id(a) AS from_id, id(b) AS to_id LIMIT {limit}"
                );

                let mut q = neo4rs::query(&cypher);
                if let Some(fid) = from_id {
                    q = q.param("from_id", fid);
                }
                if let Some(tid) = to_id {
                    q = q.param("to_id", tid);
                }

                let mut result = graph.execute(q).await.map_err(map_neo4j_error)?;
                let mut rels = Vec::new();

                while let Some(row) = result.next().await.map_err(map_neo4j_error)? {
                    if let Ok(rel) = row.get::<neo4rs::Relation>("r") {
                        rels.push(convert::relation_to_json(&rel));
                    }
                }

                let count = rels.len();

                Ok(CommandOutput::new(json!({
                    "relationships": rels,
                    "count": count,
                }))
                .next_action(
                    NextAction::new("lowmain rel create", "Create a relationship")
                        .with_param("--from", ActionParam::new().description("Source node ID").required(true))
                        .with_param("--to", ActionParam::new().description("Target node ID").required(true))
                        .with_param("--type", ActionParam::new().description("Relationship type").required(true)),
                )
                .next_action(NextAction::new("lowmain schema types", "View relationship types")))
            })
        })
}

fn create_command() -> Command {
    Command::new("create", "Create a relationship between two nodes")
        .usage("lowmain rel create --from=<id> --to=<id> --type=<type> [--props=<json>]")
        .handler(|req, ctx| {
            Box::pin(async move {
                let from_str = req.flag("from").ok_or(AppError::InvalidParams {
                    reason: "Missing --from. Usage: lowmain rel create --from=1 --to=2 --type=KNOWS".into(),
                })?;
                let to_str = req.flag("to").ok_or(AppError::InvalidParams {
                    reason: "Missing --to node ID".into(),
                })?;
                let rel_type = req.flag("type").ok_or(AppError::InvalidParams {
                    reason: "Missing --type relationship type".into(),
                })?;

                let from_id: i64 = from_str.parse().map_err(|_| AppError::InvalidParams {
                    reason: format!("Invalid --from ID: {from_str}"),
                })?;
                let to_id: i64 = to_str.parse().map_err(|_| AppError::InvalidParams {
                    reason: format!("Invalid --to ID: {to_str}"),
                })?;

                let graph = neo4j_client::from_request(req, ctx).await?;

                let (_cypher, q) = if let Some(props_str) = req.flag("props") {
                    let props: serde_json::Map<String, serde_json::Value> =
                        serde_json::from_str(props_str).map_err(|e| AppError::InvalidParams {
                            reason: format!("Invalid --props JSON: {e}"),
                        })?;

                    let set_clause: String = props
                        .keys()
                        .map(|k| format!("r.`{k}` = $`{k}`"))
                        .collect::<Vec<_>>()
                        .join(", ");

                    let cypher = format!(
                        "MATCH (a), (b) WHERE id(a) = $from_id AND id(b) = $to_id CREATE (a)-[r:`{rel_type}`]->(b) SET {set_clause} RETURN r"
                    );
                    let mut q = neo4rs::query(&cypher)
                        .param("from_id", from_id)
                        .param("to_id", to_id);

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

                    (cypher, q)
                } else {
                    let cypher = format!(
                        "MATCH (a), (b) WHERE id(a) = $from_id AND id(b) = $to_id CREATE (a)-[r:`{rel_type}`]->(b) RETURN r"
                    );
                    let q = neo4rs::query(&cypher)
                        .param("from_id", from_id)
                        .param("to_id", to_id);
                    (cypher, q)
                };

                let mut result = graph.execute(q).await.map_err(map_neo4j_error)?;
                let row = result
                    .next()
                    .await
                    .map_err(map_neo4j_error)?
                    .ok_or(AppError::QueryFailed {
                        reason: "CREATE did not return a relationship — check that both nodes exist".into(),
                    })?;

                let rel = row.get::<neo4rs::Relation>("r").map_err(|e| AppError::QueryFailed {
                    reason: e.to_string(),
                })?;
                let rel_json = convert::relation_to_json(&rel);
                let rel_id = rel.id();

                Ok(CommandOutput::new(json!({
                    "created": true,
                    "relationship": rel_json,
                }))
                .next_action(NextAction::new(
                    format!("lowmain node get {from_id}"),
                    "View source node",
                ))
                .next_action(NextAction::new(
                    format!("lowmain node get {to_id}"),
                    "View target node",
                ))
                .next_action(NextAction::new(
                    format!("lowmain rel delete {rel_id}"),
                    "Delete this relationship",
                )))
            })
        })
}

fn delete_command() -> Command {
    Command::new("delete", "Delete a relationship by ID")
        .usage("lowmain rel delete <id>")
        .handler(|req, ctx| {
            Box::pin(async move {
                let id_str = req.arg(0).ok_or(AppError::InvalidParams {
                    reason: "Missing relationship ID. Usage: lowmain rel delete <id>".into(),
                })?;
                let id: i64 = id_str.parse().map_err(|_| AppError::InvalidParams {
                    reason: format!("Invalid relationship ID: {id_str}"),
                })?;

                let graph = neo4j_client::from_request(req, ctx).await?;

                let mut result = graph
                    .execute(
                        neo4rs::query(
                            "MATCH ()-[r]->() WHERE id(r) = $id DELETE r RETURN count(r) AS deleted",
                        )
                        .param("id", id),
                    )
                    .await
                    .map_err(map_neo4j_error)?;

                let deleted: i64 = result
                    .next()
                    .await
                    .map_err(map_neo4j_error)?
                    .and_then(|r| r.get("deleted").ok())
                    .unwrap_or(0);

                if deleted == 0 {
                    return Err(AppError::RelNotFound { id: id_str.to_string() }.into());
                }

                Ok(CommandOutput::new(json!({
                    "deleted": true,
                    "id": id,
                }))
                .next_action(NextAction::new("lowmain rel find", "Find relationships"))
                .next_action(NextAction::new("lowmain schema types", "View relationship types")))
            })
        })
}

pub fn register() -> Command {
    Command::new("rel", "Relationship CRUD operations")
        .usage("lowmain rel [find|create|delete]")
        .subcommand(find_command())
        .subcommand(create_command())
        .subcommand(delete_command())
}
