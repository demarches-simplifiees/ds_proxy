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

    #[test]
    fn test_encrypt_decrypt_stream() {
        let passwd = "Correct Horse Battery Staple";

        let salt = "abcdefghabcdefghabcdefghabcdefgh";
        let chunk_size = 512;
        let config = Config::new(salt, passwd, chunk_size);
        let config2 = Config::new(salt, passwd, chunk_size);

        let key: Key = config.create_key().unwrap();
        let key2: Key = config2.create_key().unwrap();

        let clear: &[u8] = b"something to be encrypted";

        let source: Bytes = Bytes::from(&clear[..]);
        let source_stream = stream::once::<Bytes, Error>(Ok(source));

        let encoder = Encoder::new(key, chunk_size, Box::new(source_stream));
        let decoder = Decoder::new(key2, chunk_size, Box::new(encoder));

        let target_bytes: Bytes = decoder.concat2().wait().unwrap();

        assert_eq!(clear, &target_bytes[..]);
    }
}
