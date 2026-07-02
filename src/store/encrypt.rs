use crate::errors::LedgerError;
use argon2::Argon2;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chacha20poly1305::aead::{generic_array::GenericArray, Aead};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

const CANARY_PLAINTEXT: &[u8] = b"ft-ledger-v1-ok";

/// Persisted alongside the encrypted ledger. Contains nothing secret on its
/// own — the salt is meant to be public, and the canary only proves whether
/// a *derived* key is correct without ever touching real ledger data.
#[derive(Serialize, Deserialize)]
pub struct Meta {
    pub salt: String,
    pub canary_nonce: String,
    pub canary_ciphertext: String,
}

/// Holds the derived 32-byte key in memory only as long as needed, and
/// zeroes it on drop so a passphrase-derived key doesn't linger in freed
/// memory for a finance app.
pub struct Cipher {
    key: [u8; 32],
}

impl Drop for Cipher {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl Cipher {
    pub fn derive(passphrase: &str, salt: &[u8]) -> Result<Self, LedgerError> {
        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(passphrase.as_bytes(), salt, &mut key)
            .map_err(|e| LedgerError::Crypto(format!("key derivation failed: {e}")))?;
        Ok(Self { key })
    }

    fn aead(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new(GenericArray::from_slice(&self.key))
    }

    /// Encrypts one record's worth of plaintext, returning a self-contained
    /// base64 blob (nonce prefix + ciphertext) safe to write as a single
    /// JSONL line. Every record gets a fresh random nonce, so appending a
    /// new line never needs to touch or re-encrypt previous ones.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<String, LedgerError> {
        let mut nonce_bytes = [0u8; 24];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .aead()
            .encrypt(nonce, plaintext)
            .map_err(|e| LedgerError::Crypto(format!("encryption failed: {e}")))?;

        let mut out = Vec::with_capacity(24 + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(B64.encode(out))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<Vec<u8>, LedgerError> {
        let raw = B64
            .decode(encoded.trim())
            .map_err(|e| LedgerError::Corrupt(format!("invalid base64: {e}")))?;
        if raw.len() < 24 {
            return Err(LedgerError::Corrupt("ciphertext too short".into()));
        }
        let (nonce_bytes, ciphertext) = raw.split_at(24);
        let nonce = XNonce::from_slice(nonce_bytes);
        self.aead().decrypt(nonce, ciphertext).map_err(|_| {
            LedgerError::Crypto("decryption failed — wrong passphrase or corrupt data".into())
        })
    }
}

pub fn new_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

pub fn create_meta(cipher: &Cipher, salt: &[u8]) -> Result<Meta, LedgerError> {
    let canary = cipher.encrypt(CANARY_PLAINTEXT)?;
    // canary encoding already carries its own nonce prefix; store it whole.
    Ok(Meta {
        salt: B64.encode(salt),
        canary_nonce: String::new(),
        canary_ciphertext: canary,
    })
}

/// Confirms a derived key can decrypt the stored canary before it's ever
/// used on real events, so a wrong passphrase fails fast with a clear
/// message instead of surfacing as "corrupt data" on line 1 of the log.
pub fn verify_canary(cipher: &Cipher, meta: &Meta) -> Result<(), LedgerError> {
    let plaintext = cipher
        .decrypt(&meta.canary_ciphertext)
        .map_err(|_| LedgerError::Crypto("incorrect passphrase".into()))?;
    if plaintext != CANARY_PLAINTEXT {
        return Err(LedgerError::Crypto("incorrect passphrase".into()));
    }
    Ok(())
}

pub fn decode_salt(meta: &Meta) -> Result<Vec<u8>, LedgerError> {
    B64.decode(&meta.salt)
        .map_err(|e| LedgerError::Corrupt(format!("invalid salt in meta file: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let salt = new_salt();
        let cipher = Cipher::derive("correct horse battery staple", &salt).unwrap();
        let blob = cipher.encrypt(b"hello ledger").unwrap();
        let plaintext = cipher.decrypt(&blob).unwrap();
        assert_eq!(plaintext, b"hello ledger");
    }

    #[test]
    fn wrong_passphrase_fails_canary() {
        let salt = new_salt();
        let cipher = Cipher::derive("correct passphrase", &salt).unwrap();
        let meta = create_meta(&cipher, &salt).unwrap();

        let wrong_cipher = Cipher::derive("wrong passphrase", &salt).unwrap();
        assert!(verify_canary(&wrong_cipher, &meta).is_err());
        assert!(verify_canary(&cipher, &meta).is_ok());
    }
}
