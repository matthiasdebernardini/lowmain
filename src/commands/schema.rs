use agcli::{ActionParam, Command, CommandOutput, NextAction};
use serde_json::json;

use crate::error::map_neo4j_error;
use crate::neo4j_client;

fn labels_command() -> Command {
    Command::new("labels", "List all node labels")
        .usage("lowmain schema labels")
        .handler(|req, ctx| {
            Box::pin(async move {
                let graph = neo4j_client::from_request(req, ctx).await?;
                let labels = fetch_labels(&graph).await?;

                let next_actions = labels
                    .iter()
                    .map(|l| {
                        NextAction::new(
                            format!("lowmain node find --label={l}"),
                            format!("Find {l} nodes"),
                        )
                    })
                    .collect::<Vec<_>>();

                Ok(CommandOutput::new(json!({ "labels": labels })).next_actions(next_actions))
            })
        })
}

fn types_command() -> Command {
    Command::new("types", "List all relationship types")
        .usage("lowmain schema types")
        .handler(|req, ctx| {
            Box::pin(async move {
                let graph = neo4j_client::from_request(req, ctx).await?;
                let types = fetch_rel_types(&graph).await?;

                let next_actions = types
                    .iter()
                    .map(|t| {
                        NextAction::new(
                            format!("lowmain rel find --type={t}"),
                            format!("Find {t} relationships"),
                        )
                    })
                    .collect::<Vec<_>>();

                Ok(CommandOutput::new(json!({ "relationship_types": types }))
                    .next_actions(next_actions))
            })
        })
}

fn indexes_command() -> Command {
    Command::new("indexes", "List all indexes")
        .usage("lowmain schema indexes")
        .handler(|req, ctx| {
            Box::pin(async move {
                let graph = neo4j_client::from_request(req, ctx).await?;
                let indexes = fetch_indexes(&graph).await?;
                Ok(CommandOutput::new(json!({ "indexes": indexes }))
                    .next_action(NextAction::new("lowmain schema constraints", "View constraints")))
            })
        })
}

fn constraints_command() -> Command {
    Command::new("constraints", "List all constraints")
        .usage("lowmain schema constraints")
        .handler(|req, ctx| {
            Box::pin(async move {
                let graph = neo4j_client::from_request(req, ctx).await?;
                let constraints = fetch_constraints(&graph).await?;
                Ok(CommandOutput::new(json!({ "constraints": constraints }))
                    .next_action(NextAction::new("lowmain schema indexes", "View indexes")))
            })
        })
}

fn count_command() -> Command {
    Command::new("count", "Count nodes and relationships")
        .usage("lowmain schema count")
        .handler(|req, ctx| {
            Box::pin(async move {
                let graph = neo4j_client::from_request(req, ctx).await?;

                let mut result = graph
                    .execute(neo4rs::query(
                        "MATCH (n) RETURN count(n) AS node_count",
                    ))
                    .await
                    .map_err(map_neo4j_error)?;
                let node_count: i64 = result
                    .next()
                    .await
                    .map_err(map_neo4j_error)?
                    .and_then(|r| r.get("node_count").ok())
                    .unwrap_or(0);

                let mut result = graph
                    .execute(neo4rs::query(
                        "MATCH ()-[r]->() RETURN count(r) AS rel_count",
                    ))
                    .await
                    .map_err(map_neo4j_error)?;
                let rel_count: i64 = result
                    .next()
                    .await
                    .map_err(map_neo4j_error)?
                    .and_then(|r| r.get("rel_count").ok())
                    .unwrap_or(0);

                Ok(CommandOutput::new(json!({
                    "node_count": node_count,
                    "relationship_count": rel_count,
                }))
                .next_action(NextAction::new("lowmain schema labels", "View labels"))
                .next_action(NextAction::new("lowmain schema types", "View relationship types")))
            })
        })
}

pub fn register() -> Command {
    Command::new("schema", "Introspect database structure")
        .usage("lowmain schema [labels|types|indexes|constraints|count]")
        .subcommand(labels_command())
        .subcommand(types_command())
        .subcommand(indexes_command())
        .subcommand(constraints_command())
        .subcommand(count_command())
        .handler(|req, ctx| {
            Box::pin(async move {
                let graph = neo4j_client::from_request(req, ctx).await?;

                let labels = fetch_labels(&graph).await?;
                let types = fetch_rel_types(&graph).await?;
                let indexes = fetch_indexes(&graph).await?;
                let constraints = fetch_constraints(&graph).await?;

                let mut next_actions: Vec<NextAction> = labels
                    .iter()
                    .map(|l| {
                        NextAction::new(
                            format!("lowmain node find --label={l}"),
                            format!("Find {l} nodes"),
                        )
                    })
                    .collect();

                next_actions.push(
                    NextAction::new("lowmain node create", "Create a new node")
                        .with_param(
                            "--label",
                            ActionParam::new()
                                .description("Node label")
                                .enum_values(labels.clone())
                                .required(true),
                        )
                        .with_param(
                            "--props",
                            ActionParam::new()
                                .description("JSON properties")
                                .required(true),
                        ),
                );

                next_actions.push(
                    NextAction::new("lowmain query", "Execute a Cypher query")
                        .with_param("cypher", ActionParam::new().required(true)),
                );

                Ok(CommandOutput::new(json!({
                    "labels": labels,
                    "relationship_types": types,
                    "indexes": indexes,
                    "constraints": constraints,
                }))
                .next_actions(next_actions))
            })
        })
}

async fn fetch_labels(graph: &neo4rs::Graph) -> Result<Vec<String>, agcli::CommandError> {
    let mut result = graph
        .execute(neo4rs::query("CALL db.labels() YIELD label RETURN label ORDER BY label"))
        .await
        .map_err(map_neo4j_error)?;

    let mut labels = Vec::new();
    while let Some(row) = result.next().await.map_err(map_neo4j_error)? {
        if let Ok(label) = row.get::<String>("label") {
            labels.push(label);
        }
    }
    Ok(labels)
}

async fn fetch_rel_types(graph: &neo4rs::Graph) -> Result<Vec<String>, agcli::CommandError> {
    let mut result = graph
        .execute(neo4rs::query(
            "CALL db.relationshipTypes() YIELD relationshipType RETURN relationshipType ORDER BY relationshipType",
        ))
        .await
        .map_err(map_neo4j_error)?;

    let mut types = Vec::new();
    while let Some(row) = result.next().await.map_err(map_neo4j_error)? {
        if let Ok(t) = row.get::<String>("relationshipType") {
            types.push(t);
        }
    }
    Ok(types)
}

async fn fetch_indexes(graph: &neo4rs::Graph) -> Result<Vec<serde_json::Value>, agcli::CommandError> {
    let mut result = graph
        .execute(neo4rs::query("SHOW INDEXES YIELD name, type, labelsOrTypes, properties, state"))
        .await
        .map_err(map_neo4j_error)?;

    let mut indexes = Vec::new();
    while let Some(row) = result.next().await.map_err(map_neo4j_error)? {
        indexes.push(crate::convert::row_to_json(&row));
    }
    Ok(indexes)
}

async fn fetch_constraints(graph: &neo4rs::Graph) -> Result<Vec<serde_json::Value>, agcli::CommandError> {
    let mut result = graph
        .execute(neo4rs::query("SHOW CONSTRAINTS YIELD name, type, labelsOrTypes, properties"))
        .await
        .map_err(map_neo4j_error)?;

    let mut constraints = Vec::new();
    while let Some(row) = result.next().await.map_err(map_neo4j_error)? {
        constraints.push(crate::convert::row_to_json(&row));
    }
    Ok(constraints)
}
