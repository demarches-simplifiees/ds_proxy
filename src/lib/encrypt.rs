#![allow(unused_imports)]

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

pub fn encrypt_stream<S, E>(stream: S) -> impl Stream<Item = Bytes, Error = E> + 'static
where S: Stream<Item = Bytes, Error = E> + 'static,
      E: Into<Error> + 'static,
{
    let key: Key = build_key();
    let (mut enc_stream, header) = xchacha20poly1305::Stream::init_push(&key).unwrap();

    let header_bytes = Bytes::from(header.as_ref());
    let header_stream = stream::once::<Bytes, E>(Ok(header_bytes));

    let encoder = stream
        .map(move |slice: Bytes| {
            println!("encoding: {:?}", slice.len());
            let encoded = encrypt(&mut enc_stream, &slice);
            Bytes::from(encoded)
        });

    header_stream.chain(encoder)
}


pub struct Decoder <'a, E> {
    inner: &'a mut Stream<Item = Bytes, Error = E>,
    inner_ended: bool,
    decrypt_stream: Option<xchacha20poly1305::Stream<xchacha20poly1305::Pull>>,
    buffer: BytesMut,
    //taille du chunk sans le header
    chunk_size: usize,
    key: Key

}

impl<'a, E> Decoder<'a, E> {
    pub fn new(key: Key, s : &mut Stream<Item = Bytes, Error = E>) -> Decoder<E> {
        Decoder { inner: s, inner_ended: false, decrypt_stream: None, buffer: BytesMut::with_capacity(4096), chunk_size: 4096, key: key }
    }

    pub fn decrypt_buffer(&mut self) -> Poll<Option<Bytes>, E> {
        println!("---- decrypt ----");
        if self.buffer.is_empty() {
            println!("decrypt: buffer empty, on arrete");
            Ok(Async::Ready(None))
        }
        else {
            println!("decrypt: buffer non vide");
            match self.decrypt_stream {
                None => {
                    println!("pas de header");
                    // si assez d'info pour déchiffrer le header
                    if xchacha20poly1305::HEADERBYTES <= self.buffer.len() {
                        println!("decryption du header");
                        // TODO: emettre une erreur
                        let header = Header::from_slice(&self.buffer[0..xchacha20poly1305::HEADERBYTES]).unwrap();

                        // TODO: emettre une erreur
                        self.decrypt_stream = Some(xchacha20poly1305::Stream::init_pull(&header, &self.key).unwrap());

                        self.buffer.advance(xchacha20poly1305::HEADERBYTES);

                        self.decrypt_buffer()
                    }
                    // pas assez d'info pour déchiffrer le header
                    else {
                        println!("pas assez pour décrypter");
                        // si inner stream est clos, on clos le stream
                        if self.inner_ended {
                            // TODO: emettre une erreur
                            Ok(Async::Ready(None))
                        } else {
                            // si inner stream n'est pas clos, on attend
                            self.poll()
                        }
                    }
                },
                // on a un decrypt stream, on essaye de dechiffrer
                Some(ref mut stream) => {
                    // on a un chunk complet, on dechiffre
                    println!("header !");
                    if (xchacha20poly1305::ABYTES + self.chunk_size) <= self.buffer.len() {
                        println!("on decrypt un buffer complet");
                        let (decrypted1, _tag1) = stream.pull(&self.buffer[0..(xchacha20poly1305::ABYTES + self.chunk_size)], None).unwrap();
                        self.buffer.advance(xchacha20poly1305::ABYTES + self.chunk_size);
                        Ok(Async::Ready(Some(Bytes::from(&decrypted1[..]))))
                    // on a pas de chunk complet
                    } else {
                        // si inner stream est clos, on essaye d'envoyer ce qui reste
                        if self.inner_ended {
                            println!("on decrypt la partie restante du stream");
                            let rest = self.buffer.len();
                            let (decrypted1, _tag1) = stream.pull(&self.buffer[..], None).unwrap();
                            self.buffer.advance(rest);
                            Ok(Async::Ready(Some(Bytes::from(&decrypted1[..]))))
                        } else {
                            // si inner stream n'est pas clos, on repart pour un tour
                            println!("on attend plus de données avant de decrypter");
                            self.poll()
                        }
                    }
                }
            }
        }
    }
}

impl <'a, E> Stream for Decoder <'a, E> {
    type Item = Bytes;
    type Error = E;

    fn poll(&mut self) -> Poll<Option<Self::Item>, E> {
        println!("===================");
        match self.inner.poll() {
            Ok(Async::NotReady) => {
                println!("poll: pas pret on attend");
                Ok(Async::NotReady)
            },
            Ok(Async::Ready(Some(bytes))) => {
                println!("poll: bytes");
                self.buffer.extend(bytes);
                self.decrypt_buffer()
            },
            Ok(Async::Ready(None)) => {
                println!("poll: Fini");
                self.inner_ended = true;
                self.decrypt_buffer()
            },
            Err(e) => {
                println!("poll: erreur");
                Err(e)
            }
        }
    }
}



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
    let salt = sodiumoxide::crypto::pwhash::scryptsalsa208sha256::Salt::from_slice(&[170, 111, 168, 154, 69, 120, 180, 73, 145, 157, 199, 205, 254, 227, 149, 8, 204, 185, 14, 56, 249, 178, 47, 47, 189, 158, 227, 250, 192, 13, 41, 76]).unwrap();

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
    fn test_encrypt_decrypt_file() {
        let key: Key = build_key();

        let source: Bytes = Bytes::from(&[22 as u8][..]);
        let source_stream = stream::once::<Bytes, _>(Ok(source));

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
        let source_stream = stream::once::<Bytes, _>(Ok(source));

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
