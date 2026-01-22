//! Authentication utilities

use sha2::{Digest, Sha256};

/// Hash a token using SHA-256
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Verify a token against a hash
#[allow(dead_code)]
pub fn verify_token(token: &str, hash: &str) -> bool {
    hash_token(token) == hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let token = "test_token_123";
        let hash = hash_token(token);

        assert!(verify_token(token, &hash));
        assert!(!verify_token("wrong_token", &hash));
    }

    #[test]
    fn test_hash_consistency() {
        let token = "my_secret_token";
        let hash1 = hash_token(token);
        let hash2 = hash_token(token);

        assert_eq!(hash1, hash2);
    }
}
