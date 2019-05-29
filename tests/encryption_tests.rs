extern crate encrypt;

#[cfg(test)]
mod tests {
    use encrypt::config::Config;
    use encrypt::decoder::*;
    use encrypt::encoder::*;

    use actix_web::Error;
    use bytes::Bytes;
    use futures::future::Future;
    use futures::stream;
    use futures::stream::Stream;
    use sodiumoxide::crypto::secretstream::xchacha20poly1305::Key;
    use rand;
    use rand::{thread_rng, Rng};
    use rand::distributions::{Standard};

    #[test]
    fn test_encrypt_decrypt_stream() {
        let mut rng = thread_rng();

        let key: Key = build_key();

        for chunk_size in 5..20 {
            for input_length in 0..(chunk_size * 10) {
                let v: Vec<u8> = rng.sample_iter(&Standard).take(input_length).collect();

                let input: Bytes = Bytes::from(&v[..]);

                let source_stream = stream::once::<Bytes, Error>(Ok(input));

                let encoder = Encoder::new(key.clone(), chunk_size, Box::new(source_stream));
                let decoder = Decoder::new(key.clone(), chunk_size, Box::new(encoder));

                let target_bytes: Bytes = decoder.concat2().wait().unwrap();

                assert_eq!(&v[..], &target_bytes[..]);
            }
        }
    }

    #[test]
    fn test_decrypt_clear_stream() {
        let key: Key = build_key();

        let clear: &[u8] = b"something not encrypted";

        let source: Bytes = Bytes::from(&clear[..]);
        let source_stream = stream::once::<Bytes, Error>(Ok(source));

        let decoder = Decoder::new(key, 50, Box::new(source_stream));

        let target_bytes: Bytes = decoder.concat2().wait().unwrap();

        assert_eq!(clear, &target_bytes[..]);
    }

    fn build_key() -> Key {
        let passwd = "Correct Horse Battery Staple";
        let salt = "abcdefghabcdefghabcdefghabcdefgh";
        let config = Config::new(salt, passwd, 5);
        config.create_key().unwrap()
    }
}
