use sodiumoxide::crypto::secretstream::xchacha20poly1305::Key;
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
}
