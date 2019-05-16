use sodiumoxide::crypto::secretstream::xchacha20poly1305::{Key, Header};
use sodiumoxide::crypto::secretstream::xchacha20poly1305;
use futures::stream::Stream;
use bytes::Bytes;
use futures::prelude::*;
use bytes::{BytesMut};


pub struct Decoder <E> {
    inner: Box<Stream<Item = Bytes, Error = E>>,
    inner_ended: bool,
    decrypt_stream: Option<xchacha20poly1305::Stream<xchacha20poly1305::Pull>>,
    buffer: BytesMut,
    //taille du chunk sans le header
    chunk_size: usize,
    key: Key
}

impl<E> Decoder<E> {
    pub fn new(key: Key, chunk_size: usize, s : Box<Stream<Item = Bytes, Error = E>>) -> Decoder<E> {
        Decoder {
            inner: s,
            inner_ended: false,
            decrypt_stream: None,
            buffer: BytesMut::with_capacity(chunk_size),
            chunk_size,
            key
        }
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

impl <E> Stream for Decoder <E> {
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
