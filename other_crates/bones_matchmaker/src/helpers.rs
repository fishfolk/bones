use blake3::Hasher;
use rand::Rng;

/// Generates a randomness seed
pub fn generate_random_seed() -> u64 {
    rand::thread_rng().gen()
}

/// Generates a unique identifier string of 16 characters.
pub fn generate_unique_id() -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

/// Hashes a given password using Blake3.
pub fn hash_password(password: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(password.as_bytes());
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unique_id() {
        let id1 = generate_unique_id();
        let id2 = generate_unique_id();
        assert_eq!(id1.len(), 16);
        assert_eq!(id2.len(), 16);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_hash_password() {
        let password = "test_password";
        let hash1 = hash_password(password);
        let hash2 = hash_password(password);
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, password);
        assert_eq!(hash1.len(), 64); // Blake3 produces a 32-byte (64 hex characters) hash
    }
}
