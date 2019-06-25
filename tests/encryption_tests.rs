extern crate encrypt;

#[cfg(test)]
mod tests {
    use encrypt::config::create_key;
    use encrypt::decoder::*;
    use encrypt::encoder::*;

    use actix_web::Error;
    use bytes::Bytes;
    use futures::future::Future;
    use futures::stream;
    use futures::stream::Stream;
    use sodiumoxide::crypto::secretstream::xchacha20poly1305::Key;
    use proptest::prelude::*;

    #[test]
    fn test_decrypt_clear_stream() {
        let key: Key = build_key();

        let clear: &[u8] = b"something not encrypted";

        let source: Bytes = Bytes::from(&clear[..]);
        let source_stream = stream::once::<Bytes, Error>(Ok(source));

        let decoder = Decoder::new(key, Box::new(source_stream));

        let target_bytes: Bytes = decoder.concat2().wait().unwrap();

        assert_eq!(clear, &target_bytes[..]);
    }

    fn build_key() -> Key {
        let password = "Correct Horse Battery Staple".to_string();
        let salt = "abcdefghabcdefghabcdefghabcdefgh".to_string();
        create_key(salt, password).unwrap()
    }

    proptest! {
        #[test]
        fn encoding_then_decoding_doesnt_crash_and_returns_source_data(source_bytes:Vec<u8>, chunk_size in 1usize..10000) {
            let key: Key = build_key();
            let input: Bytes = Bytes::from(&source_bytes[..]);

            let source_stream = stream::once::<Bytes, Error>(Ok(input));

            let encoder = Encoder::new(key.clone(), chunk_size, Box::new(source_stream));
            let decoder = Decoder::new(key.clone(), Box::new(encoder));

            let target_bytes: Bytes = decoder.concat2().wait().unwrap();

            assert_eq!(source_bytes, target_bytes);
        }

        #[test]
        fn decrypting_plaintext_doesnt_crash_and_returns_plaintext(clear:Vec<u8>) {
            let key: Key = build_key();
            let source: Bytes = Bytes::from(&clear[..]);
            let source_stream = stream::once::<Bytes, Error>(Ok(source));

            let decoder = Decoder::new(key, Box::new(source_stream));

            let target_bytes: Bytes = decoder.concat2().wait().unwrap();

            assert_eq!(clear, target_bytes);
        }
    }
}
