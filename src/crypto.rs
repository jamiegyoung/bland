use crate::{Error, Result};
use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Aes256Gcm, Key, Nonce};

/// Encrypts a message with AES-GCM.
/// The first 12 bytes of the message are the nonce.
/// The rest of the message is the plaintext.
pub fn encrypt_data(data: &String, key: [u8; 32]) -> Result<Vec<u8>> {
    let key = Key::from_slice(&key);
    let cipher = Aes256Gcm::new(key);
    let nonce_array: [u8; 12] = rand::random();
    let nonce = &Nonce::from(nonce_array);
    match cipher.encrypt(nonce, data.as_ref()) {
        Ok(mut encrypted_data) => {
            let mut final_vec = nonce_array.to_vec();
            final_vec.append(&mut encrypted_data);
            return Ok(final_vec);
        }
        Err(_) => Err(Error::Encryption),
    }
}

/// Decrypts a message with AES-GCM.
/// The first 12 bytes of the message are the nonce.
/// The rest of the message is the ciphertext.
pub fn decrypt_data(data: Vec<u8>, key: [u8; 32]) -> Result<String> {
    let key = Key::from_slice(&key);
    let cipher = Aes256Gcm::new(key);
    let (nonce_slice, data_slice) = data.split_at(12);
    let nonce = Nonce::from_slice(nonce_slice);
    println!("nonce: {:?}", nonce);
    match cipher.decrypt(nonce, data_slice.as_ref()) {
        Ok(encrypted_data) => match String::from_utf8(encrypted_data) {
            Ok(encryted_string) => Ok(encryted_string),
            Err(_) => Err(Error::Decryption),
        },
        Err(_) => Err(Error::Decryption),
    }
}
