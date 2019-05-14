use sodiumoxide::crypto::secretstream::xchacha20poly1305::*;
use sodiumoxide::crypto::pwhash::scryptsalsa208sha256::Salt;
use sodiumoxide::crypto::pwhash;

pub fn build_key(password: &[u8], input_salt: &[u8]) -> Key {

    // let passwd = b"Correct Horse Battery Staple";
    // let salt = Salt::from_slice(&[170, 111, 168, 154, 69, 120, 180, 73, 145, 157, 199, 205, 254, 227, 149, 8, 204, 185, 14, 56, 249, 178, 47, 47, 189, 158, 227, 250, 192, 13, 41, 76]).unwrap();
    let salt = Salt::from_slice(input_salt).unwrap();
    let mut raw_key = [0u8; KEYBYTES];

    pwhash::derive_key(&mut raw_key, password, &salt,
                       pwhash::OPSLIMIT_INTERACTIVE,
                       pwhash::MEMLIMIT_INTERACTIVE).unwrap();

    Key(raw_key)
}
