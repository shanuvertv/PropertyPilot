use renewal_core::{PermissionDenied, TransitionError};

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("not signed in")]
    Unauthenticated,
    #[error(transparent)]
    PermissionDenied(#[from] PermissionDenied),
    #[error("invalid email or password")]
    InvalidCredentials,
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error(transparent)]
    Transition(#[from] TransitionError),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("{0}")]
    Internal(String),
}

pub type ServiceResult<T> = Result<T, ServiceError>;

impl ServiceError {
    pub fn validation(msg: impl Into<String>) -> Self {
        ServiceError::Validation(msg.into())
    }
}
