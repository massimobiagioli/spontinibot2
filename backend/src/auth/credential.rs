use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct OperatorCredential {
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, PartialEq)]
pub enum CredentialError {
    NotFound,
    Invalid(String),
}

impl OperatorCredential {
    pub fn load_from_file(path: &str) -> Result<Self, CredentialError> {
        let contents = std::fs::read_to_string(path).map_err(|_| CredentialError::NotFound)?;
        serde_json::from_str(&contents).map_err(|e| CredentialError::Invalid(e.to_string()))
    }

    /// Create the credential file from env vars if it doesn't already exist.
    /// Returns Ok(true) if a file was created, Ok(false) if it already existed.
    pub fn ensure_from_env(
        path: &str,
        username: &str,
        password: &str,
    ) -> Result<bool, CredentialError> {
        if std::fs::metadata(path).is_ok() {
            return Ok(false);
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| CredentialError::Invalid(e.to_string()))?
            .to_string();

        let json = serde_json::json!({
            "username": username,
            "password_hash": password_hash,
        });

        std::fs::write(path, serde_json::to_string_pretty(&json).expect("serialize failed"))
            .map_err(|e| CredentialError::Invalid(e.to_string()))?;

        Ok(true)
    }
}

/// Verifies `password` against an argon2 PHC-formatted `hash`. A malformed
/// hash string is treated as a verification failure, never a panic.
pub fn verify_password(hash: &str, password: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::PasswordHasher;
    use argon2::password_hash::SaltString;
    use rand::rngs::OsRng;

    fn hash_password(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .expect("hash_password failed")
            .to_string()
    }

    #[test]
    fn should_verify_correct_password() {
        let hash = hash_password("correct horse battery staple");
        assert!(verify_password(&hash, "correct horse battery staple"));
    }

    #[test]
    fn should_reject_incorrect_password() {
        let hash = hash_password("correct horse battery staple");
        assert!(!verify_password(&hash, "wrong password"));
    }

    #[test]
    fn should_reject_malformed_hash_without_panicking() {
        assert!(!verify_password("not-a-valid-argon2-hash", "anything"));
    }

    #[test]
    fn should_return_not_found_for_missing_credential_file() {
        let result = OperatorCredential::load_from_file("/nonexistent-spontini-credential.json");
        assert_eq!(result.unwrap_err(), CredentialError::NotFound);
    }

    #[test]
    fn should_return_invalid_for_malformed_credential_file() {
        let path = std::env::temp_dir().join(format!(
            "spontini_bad_credential_{}.json",
            std::process::id()
        ));
        std::fs::write(&path, "not json").expect("write failed");

        let result = OperatorCredential::load_from_file(path.to_str().unwrap());
        assert!(matches!(result, Err(CredentialError::Invalid(_))));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn should_load_valid_credential_file() {
        let path = std::env::temp_dir().join(format!(
            "spontini_good_credential_{}.json",
            std::process::id()
        ));
        let hash = hash_password("s3cret");
        std::fs::write(
            &path,
            format!(r#"{{"username":"operator","password_hash":"{hash}"}}"#),
        )
        .expect("write failed");

        let credential =
            OperatorCredential::load_from_file(path.to_str().unwrap()).expect("load failed");
        assert_eq!(credential.username, "operator");
        assert!(verify_password(&credential.password_hash, "s3cret"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn should_create_credential_file_when_missing() {
        let path = std::env::temp_dir().join(format!(
            "spontini_ensure_credential_{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        let created = OperatorCredential::ensure_from_env(
            path.to_str().unwrap(),
            "operator",
            "test-password",
        )
        .expect("ensure_from_env failed");
        assert!(created);

        let credential =
            OperatorCredential::load_from_file(path.to_str().unwrap()).expect("load failed");
        assert_eq!(credential.username, "operator");
        assert!(verify_password(&credential.password_hash, "test-password"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn should_not_overwrite_existing_credential_file() {
        let path = std::env::temp_dir().join(format!(
            "spontini_ensure_existing_{}.json",
            std::process::id()
        ));
        let hash = hash_password("original-password");
        std::fs::write(
            &path,
            format!(r#"{{"username":"operator","password_hash":"{hash}"}}"#),
        )
        .expect("write failed");

        let created = OperatorCredential::ensure_from_env(
            path.to_str().unwrap(),
            "operator",
            "new-password",
        )
        .expect("ensure_from_env failed");
        assert!(!created);

        let credential =
            OperatorCredential::load_from_file(path.to_str().unwrap()).expect("load failed");
        assert!(verify_password(&credential.password_hash, "original-password"));

        let _ = std::fs::remove_file(&path);
    }
}
