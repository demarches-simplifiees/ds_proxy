extern crate ds_proxy;

#[cfg(test)]
mod tests {
    use ds_proxy::config::create_key;
    use ds_proxy::decoder::*;
    use ds_proxy::encoder::*;

    use actix_web::web::{BufMut, Bytes, BytesMut};
    use actix_web::Error;
    use futures::executor::block_on_stream;

    use proptest::prelude::*;
    use sodiumoxide::crypto::secretstream::xchacha20poly1305::Key;

    #[test]
    fn test_decrypt_clear_stream() {
        let key: Key = build_key();

        let clear: &[u8] = b"something not encrypted";

        let source: Result<Bytes, Error> = Ok(Bytes::from(clear));
        let source_stream = futures::stream::once(Box::pin(async { source }));

        let decoder = Decoder::new(key, Box::new(source_stream));

        let buf = block_on_stream(decoder).map(|r| r.unwrap()).fold(
            BytesMut::with_capacity(64),
            |mut acc, x| {
                acc.put(x);
                acc
            },
        );

        assert_eq!(clear, &buf[..]);
    }

    fn build_key() -> Key {
        let password = "Correct Horse Battery Staple".to_string();
        let salt = "abcdefghabcdefghabcdefghabcdefgh".to_string();
        create_key(salt, password).unwrap()
    }

    proptest! {
        #[test]
        fn encoding_then_decoding_doesnt_crash_and_returns_source_data(source_bytes: Vec<u8>, chunk_size in 1usize..10000) {
            let key: Key = build_key();

            let source : Result<Bytes, Error> = Ok(Bytes::from(source_bytes.clone()));
            let source_stream  = futures::stream::once(Box::pin(async { source }));

            let encoder = Encoder::new(key.clone(), chunk_size, Box::new(source_stream));
            let decoder = Decoder::new(key, Box::new(encoder));

            let buf = block_on_stream(decoder)
                .map(|r| r.unwrap())
                .fold(BytesMut::with_capacity(64), |mut acc, x| { acc.put(x); acc });

            assert_eq!(source_bytes, &buf[..]);
        }

        #[test]
        fn decrypting_plaintext_doesnt_crash_and_returns_plaintext(clear: Vec<u8>) {
            let key: Key = build_key();

            let source : Result<Bytes, Error> = Ok(Bytes::from(clear.clone()));
            let source_stream  = futures::stream::once(Box::pin(async { source }));

            let decoder = Decoder::new(key, Box::new(source_stream));

            let buf = block_on_stream(decoder)
                .map(|r| r.unwrap())
                .fold(BytesMut::with_capacity(64), |mut acc, x| { acc.put(x); acc });

            assert_eq!(clear, &buf[..]);
        }
    }
}
