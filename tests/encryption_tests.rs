extern crate encrypt;

#[cfg(test)]
mod tests {
    use encrypt::key::*;
    use encrypt::decoder::*;
    use encrypt::encrypt::*;

    use sodiumoxide::crypto::secretstream::{Tag};
    use sodiumoxide::crypto::secretstream::xchacha20poly1305::{Key, Header};
    use sodiumoxide::crypto::secretstream::xchacha20poly1305;
    use futures::stream;
    use futures::stream::Stream;
    use futures::future::Future;
    use actix_web::{Error};
    use bytes::Bytes;
    use futures::prelude::*;
    use bytes::{BytesMut, BufMut};

    #[test]
    fn test_encrypt_decrypt_file() {
        let key: Key = build_key();

        let source: Bytes = Bytes::from(&[22 as u8][..]);
        let source_stream = stream::once::<Bytes, Error>(Ok(source));

        let mut encrypted_stream = encrypt_stream(source_stream);

        let mut decoder = Decoder::new(key, &mut encrypted_stream);

        decoder.poll();

        let target_bytes: Bytes = decoder.concat2().wait().unwrap();

        let mut source_vec: Vec<Bytes> = Vec::new();
        source_vec.push(Bytes::from(&[22 as u8][..]));

        assert_eq!(Bytes::from(&[22 as u8][..]), target_bytes);
    }

    #[test]
    fn test_encrypt_stream() {
        let key: Key = build_key();

        let source: Bytes = Bytes::from(&[22 as u8][..]);
        let source_stream = stream::once::<Bytes, Error>(Ok(source));

        let mut encrypted_stream = encrypt_stream(source_stream);

        let mut decoder = Decoder::new(key, &mut encrypted_stream);

        decoder.poll();

        let target_bytes: Bytes = decoder.concat2().wait().unwrap();

        let mut source_vec: Vec<Bytes> = Vec::new();
        source_vec.push(Bytes::from(&[22 as u8][..]));

        assert_eq!(Bytes::from(&[22 as u8][..]), target_bytes);
    }

    #[test]
    fn test_encrypt_and_decrypt() {
        let key: Key = build_key();

        let (mut enc_stream, header) = xchacha20poly1305::Stream::init_push(&key).unwrap();
        let mut target_file_bytes: Vec<u8> = header[0..].to_vec();

        let chunck_size = 2;

        let source  = [22 as u8, 23 as u8, 24 as u8];

        source.chunks(chunck_size).for_each(|slice| {
            target_file_bytes.append(&mut enc_stream.push(&slice, None, Tag::Message).unwrap());
        });

        let decrypted_header = Header::from_slice(&target_file_bytes[0..xchacha20poly1305::HEADERBYTES]).unwrap();

        let cipher = &target_file_bytes[xchacha20poly1305::HEADERBYTES..];

        let mut result: Vec<u8>  = [].to_vec();

        let mut dec_stream = xchacha20poly1305::Stream::init_pull(&decrypted_header, &key).unwrap();


        cipher.chunks(xchacha20poly1305::ABYTES + chunck_size).for_each(|s| {

            result.append(&mut dec_stream.pull(s, None).unwrap().0)
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
                Ok(Bytes::from(enc_stream.push(&slice, None, Tag::Message).unwrap()))
            });

        let result_stream = header_stream.chain(encoder);

        let target_bytes: Bytes = result_stream.concat2().wait().unwrap();

        let decrypted_header = Header::from_slice(&target_bytes[0..xchacha20poly1305::HEADERBYTES]).unwrap();

        let cipher = &target_bytes[xchacha20poly1305::HEADERBYTES..];

        let mut result: Vec<u8>  = [].to_vec();

        let mut dec_stream = xchacha20poly1305::Stream::init_pull(&decrypted_header, &key).unwrap();


        cipher.chunks(xchacha20poly1305::ABYTES + chunck_size).for_each(|s| {
            result.append(&mut dec_stream.pull(s, None).unwrap().0)
        });

        assert_eq!(source.to_vec(), result);
    }
}
