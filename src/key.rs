use sodiumoxide::crypto::secretstream::xchacha20poly1305::*;
use sodiumoxide::crypto::pwhash::scryptsalsa208sha256::Salt;
use sodiumoxide::crypto::pwhash;
use super::config::Config;

pub fn create_key(config: Config) -> Result<Key, &'static str> {
    return match (config.password, config.salt) {
        (Some(password), Some(input_salt)) => {
            if let Some(salt) = Salt::from_slice(&input_salt) {
                let mut raw_key = [0u8; KEYBYTES];

                pwhash::derive_key(&mut raw_key, password.as_bytes(), &salt,
                                   pwhash::OPSLIMIT_INTERACTIVE,
                                   pwhash::MEMLIMIT_INTERACTIVE).unwrap();

                Ok(Key(raw_key))
            } else {
                Err("Unable to derive a key from the salt")
            }
        },
        _ => Err("Password or salt is missing. Impossible to derive a key")
    }
}

// @deprecated
pub fn build_key(password: &[u8], input_salt: &[u8]) -> Key {
    // let passwd = b"Correct Horse Battery Staple";
    let salt = Salt::from_slice(input_salt).unwrap();
    let mut raw_key = [0u8; KEYBYTES];

    pwhash::derive_key(&mut raw_key, password, &salt,
                       pwhash::OPSLIMIT_INTERACTIVE,
                       pwhash::MEMLIMIT_INTERACTIVE).unwrap();

    Key(raw_key)
}
