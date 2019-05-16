extern crate encrypt;

#[cfg(test)]
mod tests {
    use encrypt::key::*;
    use encrypt::config::*;
    use encrypt::decoder::*;
    use encrypt::encrypt::*;

    use sodiumoxide::crypto::secretstream::xchacha20poly1305::Key;
    use futures::stream;
    use futures::stream::Stream;
    use futures::future::Future;
    use actix_web::{Error};
    use bytes::Bytes;

    #[test]
    fn test_encrypt_decrypt_stream() {
        let passwd = "Correct Horse Battery Staple";
        let salt = &[170, 111, 168, 154, 69, 120, 180, 73, 145, 157, 199, 205, 254, 227, 149, 8, 204, 185, 14, 56, 249, 178, 47, 47, 189, 158, 227, 250, 192, 13, 41, 76];
        //let salt = b"abcdefghabcdefghabcdefghabcdefgh";
        let chunk_size = 512;
        let config = Config::new(*salt, passwd, chunk_size);
        let config2 = Config::new(*salt, passwd, chunk_size);

        let key: Key = create_key(config).unwrap();
        let key2: Key = create_key(config2).unwrap();

        let source: Bytes = Bytes::from(&[22 as u8, 23 as u8, 24 as u8][..]);
        let source_stream = stream::once::<Bytes, Error>(Ok(source));

        let encoder = Encoder::new(key, chunk_size, Box::new(source_stream));

        let mut decoder = Decoder::new(key2, chunk_size, Box::new(encoder));

        decoder.poll().unwrap();

        let target_bytes: Bytes = decoder.concat2().wait().unwrap();

        let mut source_vec: Vec<Bytes> = Vec::new();
        source_vec.push(Bytes::from(&[22 as u8][..]));

        assert_eq!(Bytes::from(&[22 as u8, 23 as u8, 24 as u8][..]), target_bytes);
    }

    #[test]
    fn test_key_creation() {
        let passwd = "Correct Horse Battery Staple";
        let salt = b"abcdefghabcdefghabcdefghabcdefgh";
        let config_ok = Config::new(*salt, passwd, 512);
        let config_no_salt = Config{
            password: Some(passwd.to_string()),
            ..Config::default()
        };
        let config_no_password = Config{
            salt: Some(*salt),
            ..Config::default()
        };

        assert_eq!(true, create_key(config_ok).is_ok());
        assert_eq!(true, create_key(config_no_salt).is_err());
        assert_eq!(true, create_key(config_no_password).is_err());
    }
}
