/// A business rule was violated. Domain errors are always the caller's fault, never an I/O problem.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("the name must not be empty")]
    EmptyName,
    #[error("the name must not be longer than {max} characters")]
    NameTooLong { max: usize },
    #[error("'{0}' is not a valid id")]
    InvalidId(String),
}
