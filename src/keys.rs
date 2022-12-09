use sodiumoxide::crypto::pwhash;

use sodiumoxide::crypto::pwhash::scryptsalsa208sha256::Salt;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::{Key, KEYBYTES};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Keyring {
    keys: HashMap<u64, Key>,
}

impl Keyring {
    pub fn new(keys: HashMap<u64, Key>) -> Keyring {
        Keyring { keys }
    }

    pub fn get_last_key(&self) -> Key {
        self.keys.get(&0).unwrap().to_owned()
    }

    pub fn get_key_by_id(&self, _id: u64) -> Key {
        self.get_last_key()
    }

    pub fn load(salt: String, password: String) -> Result<Keyring, &'static str> {
        if let Some(salt) = Salt::from_slice(salt.as_bytes()) {
            let mut raw_key = [0u8; KEYBYTES];

            pwhash::derive_key(
                &mut raw_key,
                password.as_bytes(),
                &salt,
                pwhash::OPSLIMIT_INTERACTIVE,
                pwhash::MEMLIMIT_INTERACTIVE,
            )
            .unwrap();

            let mut keys = HashMap::new();

            keys.insert(0, Key(raw_key));

            Ok(Keyring::new(keys))
        } else {
            Err("Unable to derive a key from the salt")
        }
    }
}
