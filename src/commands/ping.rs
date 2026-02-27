use agcli::{ActionParam, Command, CommandOutput, NextAction};
use serde_json::json;

use crate::neo4j_client;

pub fn register() -> Command {
    Command::new("ping", "Test Neo4j connection health")
        .usage("lowmain ping [--uri=<uri>] [--user=<user>] [--password=<pw>] [--db=<db>]")
        .handler(|req, ctx| {
            Box::pin(async move {
                let graph = neo4j_client::from_request(req, ctx).await?;
                let (uri, db) = neo4j_client::connection_info(req);

                let mut result = graph.execute(neo4rs::query("RETURN 1 AS ok")).await
                    .map_err(crate::error::map_neo4j_error)?;

                let _row = result.next().await
                    .map_err(crate::error::map_neo4j_error)?;

                Ok(CommandOutput::new(json!({
                    "connected": true,
                    "uri": uri,
                    "db": db,
                }))
                .next_action(NextAction::new("lowmain schema", "Explore database structure"))
                .next_action(
                    NextAction::new("lowmain query", "Execute a Cypher query")
                        .with_param("cypher", ActionParam::new().description("Cypher query to run").required(true)),
                )
                .next_action(
                    NextAction::new("lowmain node find", "Find nodes by label")
                        .with_param("--label", ActionParam::new().description("Node label to search").required(true)),
                ))
            })
        })
}
