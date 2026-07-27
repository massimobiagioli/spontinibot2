use thiserror::Error;

#[derive(Error, Debug)]
pub enum KbStoreError {
    #[error("database error: {0}")]
    Database(#[from] libsql::Error),

    #[error("invalid embedding dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },

    #[error("not found: {0}")]
    NotFound(String),

    #[error("migration error: {0}")]
    Migration(String),

    #[error("conflict: {0}")]
    Conflict(String),
}

pub type Result<T> = std::result::Result<T, KbStoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_display_database_error_when_wrapping_libsql_error() {
        let err = KbStoreError::NotFound("document 42".into());
        let msg = err.to_string();
        assert!(
            msg.contains("document 42"),
            "expected '{msg}' to contain document id"
        );
    }

    #[test]
    fn should_display_invalid_dimension_error() {
        let err = KbStoreError::InvalidDimension {
            expected: 768,
            actual: 512,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("768"),
            "expected '{msg}' to mention expected dimension"
        );
        assert!(
            msg.contains("512"),
            "expected '{msg}' to mention actual dimension"
        );
    }

    #[test]
    fn should_display_not_found_error() {
        let err = KbStoreError::NotFound("persona 'gaspare'".into());
        assert_eq!(err.to_string(), "not found: persona 'gaspare'");
    }

    #[test]
    fn should_display_migration_error() {
        let err = KbStoreError::Migration("V2 failed".into());
        assert_eq!(err.to_string(), "migration error: V2 failed");
    }

    #[test]
    fn should_convert_from_libsql_error() {
        fn accepts_kbstore_result<T>(_: &Result<T>) {}
        let err = KbStoreError::NotFound("test".into());
        accepts_kbstore_result::<()>(&Err(err));
    }
}
