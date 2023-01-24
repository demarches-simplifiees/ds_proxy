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

    pub fn get_last_key(&self) -> Option<Key> {
        if let Some(id) = self.keys.keys().max() {
            self.get_key_by_id(id)
        } else {
            None
        }
    }

    pub fn get_key_by_id(&self, id: &u64) -> Option<Key> {
        self.keys.get(id).map(|k| k.to_owned())
    }
}
