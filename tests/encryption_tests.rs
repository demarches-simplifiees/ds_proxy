extern crate encrypt;

#[cfg(test)]
mod tests {
    use encrypt::key::*;
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
        let key: Key = build_key();
        let key2: Key = build_key();

        let source: Bytes = Bytes::from(&[22 as u8, 23 as u8, 24 as u8][..]);
        let source_stream = stream::once::<Bytes, Error>(Ok(source));


        let encoder = Encoder::new(key, 512, Box::new(source_stream));

        let mut decoder = Decoder::new(key2, 512, Box::new(encoder));

        decoder.poll().unwrap();

        let target_bytes: Bytes = decoder.concat2().wait().unwrap();

        let mut source_vec: Vec<Bytes> = Vec::new();
        source_vec.push(Bytes::from(&[22 as u8][..]));

        assert_eq!(Bytes::from(&[22 as u8, 23 as u8, 24 as u8][..]), target_bytes);
    }

}
