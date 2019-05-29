use bytes::Bytes;
use bytes::BytesMut;
use futures::prelude::*;
use futures::stream::Stream;
use sodiumoxide::crypto::secretstream::xchacha20poly1305;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::Key;
use sodiumoxide::crypto::secretstream::Tag;

pub struct Encoder<E> {
    inner: Box<Stream<Item = Bytes, Error = E>>,
    inner_ended: bool,
    encrypt_stream: Option<xchacha20poly1305::Stream<xchacha20poly1305::Push>>,
    buffer: BytesMut,
    //taille du chunk sans le header
    chunk_size: usize,
    key: Key,
}

impl<E> Encoder<E> {
    pub fn new(key: Key, chunk_size: usize, s: Box<Stream<Item = Bytes, Error = E>>) -> Encoder<E> {
        Encoder {
            inner: s,
            inner_ended: false,
            encrypt_stream: None,
            buffer: BytesMut::with_capacity(chunk_size),
            chunk_size,
            key,
        }
    }

    pub fn encrypt_buffer(&mut self) -> Poll<Option<Bytes>, E> {
        println!("---- encrypt ----");
        if self.buffer.is_empty() {
            println!("encrypt: buffer empty, on arrete");
            Ok(Async::Ready(None))
        } else {
            println!("encrypt: buffer non vide");
            match self.encrypt_stream {
                None => {
                    println!("pas de encrypt stream");
                    let (enc_stream, header) =
                        xchacha20poly1305::Stream::init_push(&self.key).unwrap();

                    self.encrypt_stream = Some(enc_stream);

                    let header_bytes = Bytes::from(header.as_ref());

                    let mut buf = Bytes::with_capacity(super::HEADER_DS_PROXY.len() + header_bytes.len());
                    buf.extend(super::HEADER_DS_PROXY);
                    buf.extend(header_bytes);

                    Ok(Async::Ready(Some(buf.into())))
                },
                // on a un encrypt stream, on essaye de chiffrer
                Some(ref mut stream) => {
                    // on a un chunk complet, on chiffre
                    println!("encrypt stream present !");
                    if self.chunk_size <= self.buffer.len() {
                        println!("on encrypt un buffer complet");
                        let encoded = stream
                            .push(&self.buffer[0..self.chunk_size], None, Tag::Message)
                            .unwrap();
                        self.buffer.advance(self.chunk_size);
                        Ok(Async::Ready(Some(Bytes::from(encoded))))
                    // on a pas de chunk complet
                    } else {
                        // si inner stream est clos, on essaye d'envoyer ce qui reste
                        if self.inner_ended {
                            println!("on encrypt la partie restante du stream");
                            let rest = self.buffer.len();
                            let encoded = stream
                                .push(&self.buffer[0..rest], None, Tag::Message)
                                .unwrap();
                            self.buffer.advance(rest);
                            Ok(Async::Ready(Some(Bytes::from(encoded))))
                        } else {
                            // si inner stream n'est pas clos, on repart pour un tour
                            println!("on attend plus de données avant de chiffrer");
                            self.poll()
                        }
                    }
                }
            }
        }
    }
}

impl<E> Stream for Encoder<E> {
    type Item = Bytes;
    type Error = E;

    fn poll(&mut self) -> Poll<Option<Self::Item>, E> {
        println!("===================");
        match self.inner.poll() {
            Ok(Async::NotReady) => {
                println!("poll: pas pret on attend");
                Ok(Async::NotReady)
            }
            Ok(Async::Ready(Some(bytes))) => {
                println!("poll: bytes");
                self.buffer.extend(bytes);
                self.encrypt_buffer()
            }
            Ok(Async::Ready(None)) => {
                println!("poll: Fini");
                self.inner_ended = true;
                self.encrypt_buffer()
            }
            Err(e) => {
                println!("poll: erreur");
                Err(e)
            }
        }
    }
}
