use super::keyring::Keyring;
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::pwhash;
use sodiumoxide::crypto::pwhash::scryptsalsa208sha256::Salt;
use sodiumoxide::crypto::secretbox;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::{Key, KEYBYTES};
use std::collections::HashMap;
use std::convert::TryInto;

pub fn load_keyring(keyring_file: &str, master_password: String, salt: String) -> Keyring {
    let mut raw_master_password = [0u8; KEYBYTES];

    let typed_salt = Salt::from_slice(salt.as_bytes()).unwrap();

    pwhash::derive_key(
        &mut raw_master_password,
        master_password.as_bytes(),
        &typed_salt,
        pwhash::OPSLIMIT_INTERACTIVE,
        pwhash::MEMLIMIT_INTERACTIVE,
    )
    .unwrap();

    let master_key = secretbox::Key::from_slice(&raw_master_password.clone()).unwrap();

    let hash_map = load_secrets(keyring_file)
        .cipher_keyring
        .iter()
        .map(|(id, base64_cipher)| (to_u64(id), decode64(base64_cipher)))
        .map(|(id, cipher)| (id, decrypt(&master_key, cipher)))
        .map(|(id, byte_key)| (id, Key(byte_key)))
        .collect();

    Keyring::new(hash_map)
}

fn to_u64(id: &str) -> u64 {
    id.parse::<u64>().unwrap()
}

fn decode64(text: &str) -> Vec<u8> {
    base64::decode(text).unwrap()
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

#[derive(Serialize, Deserialize, Debug)]
struct Secrets {
    #[serde(rename = "keys")]
    cipher_keyring: HashMap<String, String>,
}
