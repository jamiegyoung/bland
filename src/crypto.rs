use crate::{Error, Result};
use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Aes256Gcm, Key, Nonce};

/// Encrypts a message with AES-GCM.
/// The first 12 bytes of the message are the nonce.
/// The rest of the message is the plaintext.
pub fn encrypt_data(data: &str, key: [u8; 32]) -> Result<Vec<u8>> {
    let key = Key::from_slice(&key);
    let cipher = Aes256Gcm::new(key);
    let nonce_array: [u8; 12] = rand::random();
    let nonce = &Nonce::from(nonce_array);
    let mut encrypted_data = cipher
        .encrypt(nonce, data.as_ref())
        .map_err(|_| Error::Encryption)?;
    let mut final_vec = nonce_array.to_vec();
    final_vec.append(&mut encrypted_data);
    Ok(final_vec)
}

/// Decrypts a message with AES-GCM.
/// The first 12 bytes of the message are the nonce.
/// The rest of the message is the ciphertext.
pub fn decrypt_data(data: Vec<u8>, key: [u8; 32]) -> Result<String> {
    let key = Key::from_slice(&key);
    let cipher = Aes256Gcm::new(key);
    let (nonce_slice, data_slice) = data.split_at(12);
    let nonce = Nonce::from_slice(nonce_slice);
    let encrypted_data = cipher
        .decrypt(nonce, data_slice.as_ref())
        .map_err(|_| Error::Decryption)?;
    String::from_utf8(encrypted_data).map_err(|_| Error::Decryption)
}
