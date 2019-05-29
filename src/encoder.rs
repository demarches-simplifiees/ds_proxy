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
        if self.buffer.is_empty() {
            println!("buffer empty, stop");
            Ok(Async::Ready(None))
        } else {
            println!("buffer not empty");
            match self.encrypt_stream {
                None => {
                    println!("no stream encoder");
                    let (enc_stream, header) =
                        xchacha20poly1305::Stream::init_push(&self.key).unwrap();

                    self.encrypt_stream = Some(enc_stream);

                    let header_bytes = Bytes::from(header.as_ref());

                    let mut buf = Bytes::with_capacity(super::HEADER_DS_PROXY.len() + header_bytes.len());
                    buf.extend(super::HEADER_DS_PROXY);
                    buf.extend(header_bytes);

                    Ok(Async::Ready(Some(buf.into())))
                },

                Some(ref mut stream) => {
                    println!("stream encoder present !");
                    if self.chunk_size <= self.buffer.len() {
                        println!("encoding a whole chunk");
                        let encoded = stream
                            .push(&self.buffer[0..self.chunk_size], None, Tag::Message)
                            .unwrap();
                        self.buffer.advance(self.chunk_size);
                        Ok(Async::Ready(Some(Bytes::from(encoded))))
                    } else {
                        println!("the chunk is not complete");
                        if self.inner_ended {
                            println!("the stream is closed, encoding whats left");
                            let rest = self.buffer.len();
                            let encoded = stream
                                .push(&self.buffer[0..rest], None, Tag::Message)
                                .unwrap();
                            self.buffer.advance(rest);
                            Ok(Async::Ready(Some(Bytes::from(encoded))))
                        } else {
                            println!("waiting for more data");
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
        match self.inner.poll() {
            Ok(Async::NotReady) => {
                println!("poll: not ready");
                Ok(Async::NotReady)
            }
            Ok(Async::Ready(Some(bytes))) => {
                println!("poll: bytes");
                self.buffer.extend(bytes);
                self.encrypt_buffer()
            }
            Ok(Async::Ready(None)) => {
                println!("poll: over");
                self.inner_ended = true;
                self.encrypt_buffer()
            }
            Err(e) => {
                println!("poll: error");
                Err(e)
            }
        }
    }
}
