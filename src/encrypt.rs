use sodiumoxide::crypto::secretstream::{Stream, Tag};
use sodiumoxide::crypto::secretstream::xchacha20poly1305::{Key, Header};

#[allow(dead_code)]
pub fn encrypt_and_decrypt(key: Key, array: [u8; 1]) -> [u8; 1] {
    let (mut enc_stream, header) = Stream::init_push(&key).unwrap();

    let ciphertext1 = enc_stream.push(&array, None, Tag::Message).unwrap();

    let mut dec_stream = Stream::init_pull(&header, &key).unwrap();
    let (decrypted1, _tag1) = dec_stream.pull(&ciphertext1, None).unwrap();
    [decrypted1[0]]
}

#[allow(dead_code)]
pub fn encrypt(key: &Key, clear: [u8; 1]) -> (Vec<u8>, Header) {
    let (mut enc_stream, header) = Stream::init_push(key).unwrap();

    (enc_stream.push(&clear, None, Tag::Message).unwrap(), header)
}

#[allow(dead_code)]
pub fn decrypt(key: &Key, header: &Header, encrypted: Vec<u8>) -> [u8; 1] {
    let mut dec_stream = Stream::init_pull(&header, key).unwrap();

    let (decrypted1, _tag1) = dec_stream.pull(&encrypted, None).unwrap();
    [decrypted1[0]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use sodiumoxide::crypto::secretstream::{gen_key};

    #[test]
    fn test_encrypt_and_decrypt() {
        let array: [u8; 1] = [22 as u8];

        let key: Key = gen_key();
        assert_eq!(encrypt_and_decrypt(key, array), array);
    }

    #[test]
    fn test_encrypt_and_decrypt2() {
        use sodiumoxide::crypto::pwhash;

        let passwd = b"Correct Horse Battery Staple";
        let salt = pwhash::gen_salt();

        let mut raw_key = [0u8; 32];

        pwhash::derive_key(&mut raw_key, passwd, &salt,
                           pwhash::OPSLIMIT_INTERACTIVE,
                           pwhash::MEMLIMIT_INTERACTIVE).unwrap();
        
        let array: [u8; 1] = [22 as u8];
        let key: Key = Key(raw_key);

        let (cipher, header) = encrypt(&key, array);
        let decipher: [u8; 1] = decrypt(&key, &header, cipher);
        assert_eq!(array, decipher);
    }
}
