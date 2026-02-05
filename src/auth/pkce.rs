//! PKCE (Proof Key for Code Exchange) utilities for OAuth2
//!
//! Implements RFC 7636 for secure authorization code exchange.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};

/// Length of the code verifier (must be 43-128 characters)
const CODE_VERIFIER_LENGTH: usize = 64;

/// Characters allowed in code verifier (unreserved URI characters per RFC 7636)
const VERIFIER_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

/// Generate a cryptographically random code verifier
///
/// The code verifier is a high-entropy random string between 43-128 characters
/// using unreserved URI characters as defined in RFC 7636.
pub fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    (0..CODE_VERIFIER_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..VERIFIER_CHARSET.len());
            VERIFIER_CHARSET[idx] as char
        })
        .collect()
}

/// Generate a code challenge from the code verifier
///
/// Uses S256 method: BASE64URL(SHA256(code_verifier))
pub fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

/// PKCE pair containing both verifier and challenge
#[derive(Debug, Clone)]
pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
}

impl PkcePair {
    /// Generate a new PKCE pair
    pub fn new() -> Self {
        let verifier = generate_code_verifier();
        let challenge = generate_code_challenge(&verifier);
        Self { verifier, challenge }
    }
}

impl Default for PkcePair {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_verifier_length() {
        let verifier = generate_code_verifier();
        assert!(verifier.len() >= 43 && verifier.len() <= 128);
        assert_eq!(verifier.len(), CODE_VERIFIER_LENGTH);
    }

    #[test]
    fn test_code_verifier_characters() {
        let verifier = generate_code_verifier();
        let charset = std::str::from_utf8(VERIFIER_CHARSET).unwrap();
        for c in verifier.chars() {
            assert!(charset.contains(c), "Invalid character in verifier: {}", c);
        }
    }

    #[test]
    fn test_code_challenge_format() {
        let verifier = generate_code_verifier();
        let challenge = generate_code_challenge(&verifier);
        
        // SHA256 produces 32 bytes, Base64URL encoding produces 43 characters (no padding)
        assert_eq!(challenge.len(), 43);
        
        // Should only contain Base64URL characters
        for c in challenge.chars() {
            assert!(
                c.is_ascii_alphanumeric() || c == '-' || c == '_',
                "Invalid Base64URL character: {}", c
            );
        }
    }

    #[test]
    fn test_pkce_pair() {
        let pair = PkcePair::new();
        assert!(!pair.verifier.is_empty());
        assert!(!pair.challenge.is_empty());
        
        // Verify challenge matches verifier
        let expected_challenge = generate_code_challenge(&pair.verifier);
        assert_eq!(pair.challenge, expected_challenge);
    }

    #[test]
    fn test_verifiers_are_unique() {
        let v1 = generate_code_verifier();
        let v2 = generate_code_verifier();
        assert_ne!(v1, v2, "Verifiers should be unique");
    }
}
