use super::keyring::Keyring;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::pwhash;
use sodiumoxide::crypto::pwhash::scryptsalsa208sha256::Salt;
use sodiumoxide::crypto::secretbox;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::{Key, KEYBYTES};
use std::collections::HashMap;
use std::convert::TryInto;

pub fn load_keyring(keyring_file: &str, master_password: String, salt: String) -> Keyring {
    let master_key = build_master_key(master_password, salt);

    let hash_map = load_secrets(keyring_file)
        .cipher_keyring
        .iter()
        .map(|(id, base64_cipher)| (to_u64(id), decode64(base64_cipher)))
        .map(|(id, cipher)| (id, decrypt(&master_key, cipher)))
        .map(|(id, byte_key)| (id, Key(byte_key)))
        .collect();

    Keyring::new(hash_map)
}

pub fn add_random_key_to_keyring(keyring_file: &str, master_password: String, salt: String) {
    let new_key = random_key();
    let master_key = build_master_key(master_password, salt);
    add_key(keyring_file, &master_key, new_key);
}

fn add_key(keyring_file: &str, master_key: &secretbox::Key, key: [u8; 32]) {
    let new_base64_cipher = base64_cipher(master_key, key);

    let mut secrets = load_secrets(keyring_file);
    secrets
        .cipher_keyring
        .insert(next_id(&secrets), new_base64_cipher);

    save_secrets(keyring_file, &secrets)
}

fn random_key() -> [u8; 32] {
    sodiumoxide::randombytes::randombytes(KEYBYTES)
        .try_into()
        .unwrap()
}

fn to_u64(id: &str) -> u64 {
    id.parse::<u64>().unwrap()
}

fn decode64(text: &str) -> Vec<u8> {
    STANDARD.decode(text).unwrap()
}

fn load_secrets(keyring_file: &str) -> Secrets {
    if let Ok(text_secrets) = std::fs::read_to_string(keyring_file) {
        toml::from_str(&text_secrets).unwrap()
    } else {
        Secrets {
            cipher_keyring: HashMap::new(),
        }
    }
}

fn decrypt(master_key: &secretbox::Key, nonce_cipher: Vec<u8>) -> [u8; KEYBYTES] {
    let nonce = secretbox::Nonce::from_slice(&nonce_cipher[0..24]).unwrap();
    let cipher = &nonce_cipher[24..];

    secretbox::open(cipher, &nonce, master_key)
        .expect("could not decipher a key")
        .try_into()
        .unwrap()
}

fn build_master_key(master_password: String, salt: String) -> secretbox::Key {
    let mut key = [0u8; KEYBYTES];

    let typed_salt = Salt::from_slice(salt.as_bytes()).unwrap();

    pwhash::derive_key(
        &mut key,
        master_password.as_bytes(),
        &typed_salt,
        pwhash::OPSLIMIT_INTERACTIVE,
        pwhash::MEMLIMIT_INTERACTIVE,
    )
    .unwrap();

    secretbox::Key::from_slice(&key).unwrap()
}

fn next_id(secrets: &Secrets) -> String {
    if let Some(max) = last_id(secrets) {
        (max + 1).to_string()
    } else {
        "0".to_string()
    }
}

fn last_id(secrets: &Secrets) -> Option<u64> {
    secrets
        .cipher_keyring
        .keys()
        .max()
        .map(|x| x.parse::<u64>().unwrap())
}

fn base64_cipher(master_key: &secretbox::Key, key: [u8; 32]) -> String {
    let (cipher, nonce) = encrypt(master_key, key);
    let nonce_cipher = concat(nonce, cipher);
    STANDARD.encode(nonce_cipher)
}

fn encrypt(master_key: &secretbox::Key, byte_key: [u8; 32]) -> (Vec<u8>, [u8; 24]) {
    let nonce_bytes: [u8; 24] = sodiumoxide::randombytes::randombytes(24)
        .try_into()
        .unwrap();
    let nonce = secretbox::Nonce::from_slice(&nonce_bytes).unwrap();
    let cipher = secretbox::seal(&byte_key, &nonce, master_key);

    (cipher, nonce_bytes)
}

fn concat(nonce: [u8; 24], mut cipher: Vec<u8>) -> Vec<u8> {
    let mut serialized = Vec::<u8>::from(nonce);
    serialized.append(&mut cipher);
    serialized
}

fn save_secrets(keyring_file: &str, secrets: &Secrets) {
    let text_secrets = toml::to_string(secrets).unwrap();
    std::fs::write(keyring_file, text_secrets).unwrap()
}

#[derive(Serialize, Deserialize, Debug)]
struct Secrets {
    #[serde(rename = "keys")]
    cipher_keyring: HashMap<String, String>,
}
