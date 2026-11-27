//! Typed PostgreSQL client and protocol errors (decision 0071 ruling 8).

use std::io;

use brokkr_store::StoreError;
use thiserror::Error;

/// SQLSTATE for PostgreSQL unique-constraint violations (`unique_violation`).
pub(crate) const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

/// Typed error returned by the synchronous PostgreSQL v3 wire client.
#[derive(Debug, Error)]
pub enum PgError {
    #[error("invalid postgres url: {0}")]
    InvalidUrl(&'static str),
    #[error("postgres io: {0}")]
    Io(#[from] io::Error),
    #[error("postgres protocol: {0}")]
    Protocol(String),
    #[error("postgres authentication: {0}")]
    Auth(String),
    #[error("postgres server error ({sqlstate}): {message}")]
    Server {
        sqlstate: String,
        message: String,
        constraint: Option<String>,
    },
}

impl PgError {
    /// Return the 5-character PostgreSQL SQLSTATE if this is a server error.
    pub fn sqlstate(&self) -> Option<&str> {
        match self {
            PgError::Server { sqlstate, .. } => Some(sqlstate.as_str()),
            PgError::InvalidUrl(_) | PgError::Io(_) | PgError::Protocol(_) | PgError::Auth(_) => {
                None
            }
        }
    }

    /// Convert a PostgreSQL client error into a [`StoreError`] at the port boundary.
    pub fn into_store_error(self) -> StoreError {
        match self {
            PgError::Io(err) => StoreError::Io(err),
            other => StoreError::Io(io::Error::other(other)),
        }
    }
}
