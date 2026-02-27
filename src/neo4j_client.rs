use agcli::{CommandError, CommandRequest, ExecutionContext};
use neo4rs::Graph;
use std::env;

use crate::error::AppError;

const DEFAULT_URI: &str = "bolt://localhost:7687";
const DEFAULT_USER: &str = "neo4j";
const DEFAULT_DB: &str = "neo4j";

/// Resolve a connection value from: CLI flag > env var > default.
fn resolve(req: &CommandRequest<'_>, flag: &str, env_key: &str, default: Option<&str>) -> Option<String> {
    req.flag(flag)
        .map(String::from)
        .or_else(|| env::var(env_key).ok())
        .or_else(|| default.map(String::from))
}

/// Build a Neo4j Graph connection from CLI flags, env vars, and defaults.
pub async fn from_request(req: &CommandRequest<'_>, _ctx: &ExecutionContext) -> Result<Graph, CommandError> {
    let uri = resolve(req, "uri", "NEO4J_URI", Some(DEFAULT_URI))
        .expect("default URI always present");
    let user = resolve(req, "user", "NEO4J_USER", Some(DEFAULT_USER))
        .expect("default user always present");
    let password = resolve(req, "password", "NEO4J_PASSWORD", None)
        .ok_or(AppError::ConnectionNotConfigured)?;
    let db = resolve(req, "db", "NEO4J_DB", Some(DEFAULT_DB))
        .expect("default db always present");

    let config = neo4rs::ConfigBuilder::default()
        .uri(&uri)
        .user(&user)
        .password(&password)
        .db(db.as_str())
        .build()
        .map_err(|e| AppError::ConnectionFailed {
            reason: e.to_string(),
        })?;

    Graph::connect(config).await.map_err(|e| {
        let err = crate::error::map_neo4j_error(e);
        CommandError::from(err)
    })
}

/// Return the URI and database name for display (from flags/env/defaults).
pub fn connection_info(req: &CommandRequest<'_>) -> (String, String) {
    let uri = resolve(req, "uri", "NEO4J_URI", Some(DEFAULT_URI))
        .expect("default URI always present");
    let db = resolve(req, "db", "NEO4J_DB", Some(DEFAULT_DB))
        .expect("default db always present");
    (uri, db)
}
