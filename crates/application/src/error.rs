use starter_domain::DomainError;

use crate::StorageError;

/// Every way [`crate::App::execute`] can fail.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The input broke a business rule. The caller can fix it.
    #[error(transparent)]
    Validation(#[from] DomainError),
    /// The addressed entity does not exist.
    #[error("{what} {id} not found")]
    NotFound { what: &'static str, id: String },
    /// The storage adapter failed. Usually not the caller's fault.
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl AppError {
    /// Stable, machine-readable error code for JSON output, exit codes and agents.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation",
            Self::NotFound { .. } => "not_found",
            Self::Storage(_) => "storage",
        }
    }

    /// `true` when the caller caused the error (bad input, unknown id).
    pub fn is_user_error(&self) -> bool {
        matches!(self, Self::Validation(_) | Self::NotFound { .. })
    }
}
