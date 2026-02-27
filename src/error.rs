use agcli::CommandError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Connection failed: {reason}")]
    ConnectionFailed { reason: String },

    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    #[error("Cypher syntax error: {detail}")]
    CypherSyntaxError { detail: String },

    #[error("Constraint violation: {detail}")]
    ConstraintViolation { detail: String },

    #[error("Query failed: {reason}")]
    QueryFailed { reason: String },

    #[error("Node not found: {id}")]
    NodeNotFound { id: String },

    #[error("Relationship not found: {id}")]
    RelNotFound { id: String },

    #[error("Connection not configured — NEO4J_PASSWORD is required")]
    ConnectionNotConfigured,

    #[error("Invalid parameters: {reason}")]
    InvalidParams { reason: String },
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ConnectionFailed { .. } => "CONNECTION_FAILED",
            Self::AuthenticationFailed { .. } => "AUTH_FAILED",
            Self::CypherSyntaxError { .. } => "CYPHER_SYNTAX_ERROR",
            Self::ConstraintViolation { .. } => "CONSTRAINT_VIOLATION",
            Self::QueryFailed { .. } => "QUERY_FAILED",
            Self::NodeNotFound { .. } => "NODE_NOT_FOUND",
            Self::RelNotFound { .. } => "REL_NOT_FOUND",
            Self::ConnectionNotConfigured => "CONNECTION_NOT_CONFIGURED",
            Self::InvalidParams { .. } => "INVALID_PARAMS",
        }
    }

    pub fn retryable(&self) -> bool {
        matches!(self, Self::ConnectionFailed { .. })
    }

    pub fn fix(&self) -> String {
        match self {
            Self::ConnectionFailed { .. } => {
                "Check that Neo4j is running and the URI is correct. Default: bolt://localhost:7687"
                    .to_string()
            }
            Self::AuthenticationFailed { .. } => {
                "Check NEO4J_USER and NEO4J_PASSWORD, or pass --user and --password".to_string()
            }
            Self::CypherSyntaxError { .. } => {
                "Check Cypher syntax. Run `lowmain schema` to see available labels and types"
                    .to_string()
            }
            Self::ConstraintViolation { .. } => {
                "Check `lowmain schema constraints` for active constraints".to_string()
            }
            Self::QueryFailed { .. } => {
                "Check the query and parameters. Run `lowmain schema` to explore the database"
                    .to_string()
            }
            Self::NodeNotFound { id } => {
                format!("No node with ID {id}. Run `lowmain node find` to list nodes")
            }
            Self::RelNotFound { id } => {
                format!("No relationship with ID {id}. Run `lowmain rel find` to list relationships")
            }
            Self::ConnectionNotConfigured => {
                "Set NEO4J_PASSWORD env var or pass --password. Example: NEO4J_PASSWORD=secret lowmain ping"
                    .to_string()
            }
            Self::InvalidParams { .. } => {
                "Check parameter format. --params expects a JSON object, --props expects a JSON object"
                    .to_string()
            }
        }
    }
}

impl From<AppError> for CommandError {
    fn from(err: AppError) -> Self {
        CommandError::new(err.to_string(), err.code(), err.fix()).retryable(err.retryable())
    }
}

/// Map a neo4rs error to an AppError by inspecting the error message.
pub fn map_neo4j_error(err: neo4rs::Error) -> AppError {
    let msg = err.to_string();
    if msg.contains("authentication")
        || msg.contains("Unauthorized")
        || msg.contains("credentials")
    {
        AppError::AuthenticationFailed { reason: msg }
    } else if msg.contains("SyntaxError") || msg.contains("Invalid input") {
        AppError::CypherSyntaxError { detail: msg }
    } else if msg.contains("ConstraintValidationFailed") || msg.contains("already exists") {
        AppError::ConstraintViolation { detail: msg }
    } else if msg.contains("connection") || msg.contains("Connection") || msg.contains("refused")
    {
        AppError::ConnectionFailed { reason: msg }
    } else {
        AppError::QueryFailed { reason: msg }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_connection_failed() {
        let e = AppError::ConnectionFailed {
            reason: "refused".into(),
        };
        assert_eq!(e.code(), "CONNECTION_FAILED");
    }

    #[test]
    fn code_auth_failed() {
        let e = AppError::AuthenticationFailed {
            reason: "bad pw".into(),
        };
        assert_eq!(e.code(), "AUTH_FAILED");
    }

    #[test]
    fn code_cypher_syntax() {
        let e = AppError::CypherSyntaxError {
            detail: "oops".into(),
        };
        assert_eq!(e.code(), "CYPHER_SYNTAX_ERROR");
    }

    #[test]
    fn code_constraint_violation() {
        let e = AppError::ConstraintViolation {
            detail: "dup".into(),
        };
        assert_eq!(e.code(), "CONSTRAINT_VIOLATION");
    }

    #[test]
    fn code_query_failed() {
        let e = AppError::QueryFailed {
            reason: "fail".into(),
        };
        assert_eq!(e.code(), "QUERY_FAILED");
    }

    #[test]
    fn code_node_not_found() {
        let e = AppError::NodeNotFound { id: "42".into() };
        assert_eq!(e.code(), "NODE_NOT_FOUND");
    }

    #[test]
    fn code_rel_not_found() {
        let e = AppError::RelNotFound { id: "7".into() };
        assert_eq!(e.code(), "REL_NOT_FOUND");
    }

    #[test]
    fn code_connection_not_configured() {
        assert_eq!(AppError::ConnectionNotConfigured.code(), "CONNECTION_NOT_CONFIGURED");
    }

    #[test]
    fn code_invalid_params() {
        let e = AppError::InvalidParams {
            reason: "bad json".into(),
        };
        assert_eq!(e.code(), "INVALID_PARAMS");
    }

    #[test]
    fn connection_failed_is_retryable() {
        let e = AppError::ConnectionFailed {
            reason: "timeout".into(),
        };
        assert!(e.retryable());
    }

    #[test]
    fn non_connection_errors_not_retryable() {
        assert!(!AppError::AuthenticationFailed { reason: "x".into() }.retryable());
        assert!(!AppError::CypherSyntaxError { detail: "x".into() }.retryable());
        assert!(!AppError::ConstraintViolation { detail: "x".into() }.retryable());
        assert!(!AppError::QueryFailed { reason: "x".into() }.retryable());
        assert!(!AppError::NodeNotFound { id: "x".into() }.retryable());
        assert!(!AppError::RelNotFound { id: "x".into() }.retryable());
        assert!(!AppError::ConnectionNotConfigured.retryable());
        assert!(!AppError::InvalidParams { reason: "x".into() }.retryable());
    }

    #[test]
    fn fix_strings_all_non_empty() {
        let variants: Vec<AppError> = vec![
            AppError::ConnectionFailed { reason: "r".into() },
            AppError::AuthenticationFailed { reason: "r".into() },
            AppError::CypherSyntaxError { detail: "d".into() },
            AppError::ConstraintViolation { detail: "d".into() },
            AppError::QueryFailed { reason: "r".into() },
            AppError::NodeNotFound { id: "42".into() },
            AppError::RelNotFound { id: "7".into() },
            AppError::ConnectionNotConfigured,
            AppError::InvalidParams { reason: "r".into() },
        ];
        for v in variants {
            assert!(!v.fix().is_empty(), "fix() empty for {}", v.code());
        }
    }

    #[test]
    fn command_error_from_app_error_preserves_fields() {
        let app_err = AppError::ConnectionFailed {
            reason: "timeout".into(),
        };
        let cmd_err = CommandError::from(app_err);
        assert_eq!(cmd_err.code, "CONNECTION_FAILED");
        assert!(cmd_err.retryable);
        assert!(cmd_err.message.contains("timeout"));
    }
}
