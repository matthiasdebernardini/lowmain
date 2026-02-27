use agcli::{ActionParam, Command, CommandOutput, NextAction};
use serde_json::json;

use crate::convert;
use crate::error::{AppError, map_neo4j_error};
use crate::neo4j_client;

pub fn register() -> Command {
    Command::new("query", "Execute a raw Cypher query")
        .usage("lowmain query <cypher> [--params=<json>] [--limit=<n>] [--write]")
        .handler(|req, ctx| {
            Box::pin(async move {
                let cypher = req.arg(0).ok_or(AppError::InvalidParams {
                    reason: "Missing Cypher query. Usage: lowmain query \"MATCH (n) RETURN n\"".into(),
                })?;

                let limit: usize = req
                    .flag("limit")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(100);

                let is_write = req.flag("write").is_some();

                let graph = neo4j_client::from_request(req, ctx).await?;

                // Build parameterized query
                let mut q = neo4rs::query(cypher);

                if let Some(params_str) = req.flag("params") {
                    let params: serde_json::Map<String, serde_json::Value> =
                        serde_json::from_str(params_str).map_err(|e| AppError::InvalidParams {
                            reason: format!("Invalid --params JSON: {e}"),
                        })?;
                    for (key, val) in params {
                        q = match val {
                            serde_json::Value::String(s) => q.param(&key, s),
                            serde_json::Value::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    q.param(&key, i)
                                } else if let Some(f) = n.as_f64() {
                                    q.param(&key, f)
                                } else {
                                    q.param(&key, n.to_string())
                                }
                            }
                            serde_json::Value::Bool(b) => q.param(&key, b),
                            _ => q.param(&key, val.to_string()),
                        };
                    }
                }

                if is_write {
                    graph.run(q).await.map_err(map_neo4j_error)?;
                    Ok(CommandOutput::new(json!({
                        "executed": true,
                        "cypher": cypher,
                        "mode": "write",
                    }))
                    .next_action(NextAction::new("lowmain schema", "Check schema after mutation"))
                    .next_action(
                        NextAction::new("lowmain query", "Run another query")
                            .with_param("cypher", ActionParam::new().required(true)),
                    ))
                } else {
                    let mut result = graph.execute(q).await.map_err(map_neo4j_error)?;
                    let mut rows = Vec::new();

                    while let Some(row) = result.next().await.map_err(map_neo4j_error)? {
                        if rows.len() >= limit {
                            break;
                        }
                        rows.push(convert::row_to_json(&row));
                    }

                    let count = rows.len();
                    let truncated = count >= limit;

                    Ok(CommandOutput::new(json!({
                        "cypher": cypher,
                        "rows": rows,
                        "count": count,
                        "truncated": truncated,
                        "limit": limit,
                    }))
                    .next_action(
                        NextAction::new("lowmain query", "Run another query")
                            .with_param("cypher", ActionParam::new().required(true)),
                    )
                    .next_action(NextAction::new("lowmain schema", "Explore database structure")))
                }
            })
        })
}
