use sodiumoxide::crypto::secretstream::{Stream, Tag};
use sodiumoxide::crypto::secretstream::xchacha20poly1305::{Key, Header};
use sodiumoxide::crypto::secretstream::xchacha20poly1305;

#[allow(dead_code)]
pub fn encrypt(enc_stream: &mut xchacha20poly1305::Stream<xchacha20poly1305::Push>, clear: &[u8]) -> Vec<u8> {
    enc_stream.push(clear, None, Tag::Message).unwrap()
}

#[allow(dead_code)]
pub fn decrypt(dec_stream: &mut xchacha20poly1305::Stream<xchacha20poly1305::Pull>, encrypted: &[u8]) -> Vec<u8> {
    let (decrypted1, _tag1) = dec_stream.pull(encrypted, None).unwrap();
    decrypted1
}

#[allow(dead_code)]
pub fn build_key() -> Key {
    use sodiumoxide::crypto::pwhash;

    let passwd = b"Correct Horse Battery Staple";
    let salt = pwhash::gen_salt();

    let mut raw_key = [0u8; xchacha20poly1305::KEYBYTES];

    pwhash::derive_key(&mut raw_key, passwd, &salt,
                       pwhash::OPSLIMIT_INTERACTIVE,
                       pwhash::MEMLIMIT_INTERACTIVE).unwrap();

    Key(raw_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_and_decrypt() {
        let key: Key = build_key();

        let (mut enc_stream, header) = Stream::init_push(&key).unwrap();
        let mut target_file_bytes: Vec<u8> = header[0..].to_vec();

        let chunck_size = 2;

        let source  = [22 as u8, 23 as u8, 24 as u8];

        source.chunks(chunck_size).for_each(|slice| {
            target_file_bytes.append(&mut encrypt(&mut enc_stream, slice));
        });

        let decrypted_header = Header::from_slice(&target_file_bytes[0..xchacha20poly1305::HEADERBYTES]).unwrap();

        let cipher = &target_file_bytes[xchacha20poly1305::HEADERBYTES..];

        let mut result: Vec<u8>  = [].to_vec();

        let mut dec_stream = Stream::init_pull(&decrypted_header, &key).unwrap();


        cipher.chunks(xchacha20poly1305::ABYTES + chunck_size).for_each(|s| {
            result.append(&mut decrypt(&mut dec_stream, s))
        });

        assert_eq!(source.to_vec(), result);
    }
}
