//! RC4 encryption/decryption for Slime Online 2 protocol
//!
//! The client uses hardcoded RC4 keys:
//! - Client encrypts outgoing with CLIENT_ENCRYPT_KEY
//! - Client decrypts incoming with CLIENT_DECRYPT_KEY
//!
//! Server must reverse this:
//! - Server decrypts incoming with CLIENT_ENCRYPT_KEY
//! - Server encrypts outgoing with CLIENT_DECRYPT_KEY
//!
//! IMPORTANT: RC4 cipher state is re-initialized for each message.

use rc4::{KeyInit, Rc4, StreamCipher};

use crate::constants::{CLIENT_DECRYPT_KEY, CLIENT_ENCRYPT_KEY};

/// Decrypt a message received from the client.
///
/// Uses the CLIENT_ENCRYPT_KEY because the client encrypted with this key.
/// RC4 is symmetric, so the same key is used for encryption and decryption.
///
/// # Arguments
/// * `data` - The encrypted message bytes (modified in place)
pub fn decrypt_client_message(data: &mut [u8]) {
    let mut cipher = Rc4::new(CLIENT_ENCRYPT_KEY.into());
    cipher.apply_keystream(data);
}

/// Encrypt a message to send to the client.
///
/// Uses the CLIENT_DECRYPT_KEY because the client will decrypt with this key.
/// RC4 is symmetric, so the same key is used for encryption and decryption.
///
/// # Arguments
/// * `data` - The plaintext message bytes (modified in place)
pub fn encrypt_server_message(data: &mut [u8]) {
    let mut cipher = Rc4::new(CLIENT_DECRYPT_KEY.into());
    cipher.apply_keystream(data);
}

/// Decrypt a message with a custom key (for testing or alternative encryption).
pub fn decrypt_with_key(data: &mut [u8], key: &[u8]) {
    let mut cipher = Rc4::new(key.into());
    cipher.apply_keystream(data);
}

/// Encrypt a message with a custom key (for testing or alternative encryption).
pub fn encrypt_with_key(data: &mut [u8], key: &[u8]) {
    let mut cipher = Rc4::new(key.into());
    cipher.apply_keystream(data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc4_roundtrip() {
        let original = b"Hello, Slime World!";
        let mut data = original.to_vec();

        // Simulate client encrypting a message
        encrypt_with_key(&mut data, CLIENT_ENCRYPT_KEY);

        // Server decrypts
        decrypt_client_message(&mut data);

        assert_eq!(&data, original);
    }

    #[test]
    fn test_server_to_client_roundtrip() {
        let original = b"Welcome to the server!";
        let mut data = original.to_vec();

        // Server encrypts
        encrypt_server_message(&mut data);

        // Simulate client decrypting
        decrypt_with_key(&mut data, CLIENT_DECRYPT_KEY);

        assert_eq!(&data, original);
    }

    #[test]
    fn test_encryption_modifies_data() {
        let original = b"Test message";
        let mut data = original.to_vec();

        encrypt_server_message(&mut data);

        // Encrypted data should be different from original
        assert_ne!(&data, original);
    }

    #[test]
    fn test_different_keys_produce_different_output() {
        let original = b"Test message";
        let mut data1 = original.to_vec();
        let mut data2 = original.to_vec();

        encrypt_with_key(&mut data1, CLIENT_ENCRYPT_KEY);
        encrypt_with_key(&mut data2, CLIENT_DECRYPT_KEY);

        // Different keys should produce different encrypted output
        assert_ne!(data1, data2);
    }

    #[test]
    fn test_empty_data() {
        let mut data: Vec<u8> = vec![];
        decrypt_client_message(&mut data);
        encrypt_server_message(&mut data);
        assert!(data.is_empty());
    }

    #[test]
    fn test_binary_data() {
        // Test with binary data including null bytes
        let original: Vec<u8> = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0x00, 0x10];
        let mut data = original.clone();

        encrypt_server_message(&mut data);
        decrypt_with_key(&mut data, CLIENT_DECRYPT_KEY);

        assert_eq!(data, original);
    }
}
