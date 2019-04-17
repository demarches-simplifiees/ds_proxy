use sodiumoxide::crypto::secretstream::{Stream, Tag};
use sodiumoxide::crypto::secretstream::xchacha20poly1305::{Key, Header};

#[allow(dead_code)]
pub fn encrypt(key: &Key, clear: &[u8]) -> Vec<u8> {
    let (mut enc_stream, header) = Stream::init_push(key).unwrap();

    let mut result: Vec<u8> = header[0..].to_vec();

    let mut encrypted_message = enc_stream.push(clear, None, Tag::Message).unwrap();

    result.append(&mut encrypted_message);

    result
}

#[allow(dead_code)]
pub fn decrypt(key: &Key, header_cipher: &[u8]) -> Vec<u8> {
    let header = Header::from_slice(&header_cipher[0..24]).unwrap();

    let mut dec_stream = Stream::init_pull(&header, key).unwrap();

    let (decrypted1, _tag1) = dec_stream.pull(&header_cipher[24..], None).unwrap();
    decrypted1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_and_decrypt2() {
        use sodiumoxide::crypto::pwhash;

        let passwd = b"Correct Horse Battery Staple";
        let salt = pwhash::gen_salt();

        let mut raw_key = [0u8; 32];

        pwhash::derive_key(&mut raw_key, passwd, &salt,
                           pwhash::OPSLIMIT_INTERACTIVE,
                           pwhash::MEMLIMIT_INTERACTIVE).unwrap();

        let array: &[u8] = &[22 as u8];
        let key: Key = Key(raw_key);

        let header_cipher = encrypt(&key, &array);
        let decipher = decrypt(&key, &header_cipher);

        assert_eq!(array, &decipher[0..]);
    }
}
