use sodiumoxide::crypto::secretstream::{Tag};
use sodiumoxide::crypto::secretstream::xchacha20poly1305::{Key, Header};
use sodiumoxide::crypto::secretstream::xchacha20poly1305;
use futures::stream;
use futures::stream::Stream;

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

        let (mut enc_stream, header) = xchacha20poly1305::Stream::init_push(&key).unwrap();
        let mut target_file_bytes: Vec<u8> = header[0..].to_vec();

        let chunck_size = 2;

        let source  = [22 as u8, 23 as u8, 24 as u8];

        source.chunks(chunck_size).for_each(|slice| {
            target_file_bytes.append(&mut encrypt(&mut enc_stream, slice));
        });

        let decrypted_header = Header::from_slice(&target_file_bytes[0..xchacha20poly1305::HEADERBYTES]).unwrap();

        let cipher = &target_file_bytes[xchacha20poly1305::HEADERBYTES..];

        let mut result: Vec<u8>  = [].to_vec();

        let mut dec_stream = xchacha20poly1305::Stream::init_pull(&decrypted_header, &key).unwrap();


        cipher.chunks(xchacha20poly1305::ABYTES + chunck_size).for_each(|s| {
            result.append(&mut decrypt(&mut dec_stream, s))
        });

        assert_eq!(source.to_vec(), result);
    }

    #[test]
    fn test_encrypt_and_decrypt_stream() {
        let key: Key = build_key();

        let (mut enc_stream, header) = xchacha20poly1305::Stream::init_push(&key).unwrap();

        let chunck_size = 2;

        use bytes::Bytes;
        let source  =  Bytes::from(&[22 as u8, 23 as u8, 24 as u8][..]);

        let stream = stream::iter_ok::<_, ()>(source.iter());

        let header_bytes = Bytes::from(header.as_ref());

        let header_stream = stream::once::<Bytes, ()>(Ok(header_bytes));


        use futures::future::Future;

        let encoder = stream
            .map(|slice: &u8| *slice)
            .chunks(chunck_size)
            .and_then(|slice: Vec<u8>| {
                Ok(Bytes::from(encrypt(&mut enc_stream, &slice)))
            });

        let result_stream = header_stream.chain(encoder);

        let target_bytes: Bytes = result_stream.concat2().wait().unwrap();

        let decrypted_header = Header::from_slice(&target_bytes[0..xchacha20poly1305::HEADERBYTES]).unwrap();

        let cipher = &target_bytes[xchacha20poly1305::HEADERBYTES..];

        let mut result: Vec<u8>  = [].to_vec();

        let mut dec_stream = xchacha20poly1305::Stream::init_pull(&decrypted_header, &key).unwrap();


        cipher.chunks(xchacha20poly1305::ABYTES + chunck_size).for_each(|s| {
            result.append(&mut decrypt(&mut dec_stream, s))
        });

        assert_eq!(source.to_vec(), result);
    }
}
