use sodiumoxide::crypto::pwhash;
use sodiumoxide::crypto::pwhash::Salt;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::*;

pub type DsKey = Key;

pub fn build_key(password: &[u8], input_salt: &[u8]) -> Key {
    // let passwd = b"Correct Horse Battery Staple";
    let salt = Salt::from_slice(input_salt).unwrap();
    let mut raw_key = [0u8; KEYBYTES];

    pwhash::derive_key(
        &mut raw_key,
        password,
        &salt,
        pwhash::OPSLIMIT_INTERACTIVE,
        pwhash::MEMLIMIT_INTERACTIVE,
    )
    .unwrap();

    Key(raw_key)
}
