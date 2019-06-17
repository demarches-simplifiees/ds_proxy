use bytes::Bytes;
use bytes::BytesMut;
use futures::prelude::*;
use futures::stream::Stream;
use sodiumoxide::crypto::secretstream::xchacha20poly1305;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::Key;
use sodiumoxide::crypto::secretstream::Tag;
use log::trace;
use super::header::{ Header, HEADER_SIZE };

pub struct Encoder<E> {
    inner: Box<Stream<Item = Bytes, Error = E>>,
    inner_ended: bool,
    stream_encoder: Option<xchacha20poly1305::Stream<xchacha20poly1305::Push>>,
    buffer: BytesMut,
    chunk_size: usize,
    key: Key,
}

impl<E> Encoder<E> {
    pub fn new(key: Key, chunk_size: usize, s: Box<Stream<Item = Bytes, Error = E>>) -> Encoder<E> {
        Encoder {
            inner: s,
            inner_ended: false,
            stream_encoder: None,
            buffer: BytesMut::with_capacity(chunk_size),
            chunk_size,
            key,
        }
    }

    pub fn encrypt_buffer(&mut self) -> Poll<Option<Bytes>, E> {
        if self.buffer.is_empty() {
            trace!("buffer empty, stop");
            Ok(Async::Ready(None))
        } else {
            trace!("buffer not empty");
            match self.stream_encoder {
                None => {
                    trace!("no stream encoder");
                    let (enc_stream, encryption_header) =
                        xchacha20poly1305::Stream::init_push(&self.key).unwrap();

                    self.stream_encoder = Some(enc_stream);

                    let encryption_header_bytes = Bytes::from(encryption_header.as_ref());

                    let mut buf = Bytes::with_capacity(HEADER_SIZE + encryption_header_bytes.len());

                    let ds_header = Header::new(self.chunk_size);
                    let ds_header_bytes: Vec<u8> = ds_header.into();
                    buf.extend(&ds_header_bytes[..]);
                    buf.extend(encryption_header_bytes);

                    Ok(Async::Ready(Some(buf)))
                },

                Some(ref mut stream) => {
                    trace!("stream encoder present !");
                    if self.chunk_size <= self.buffer.len() {
                        trace!("encoding a whole chunk");
                        let encoded = stream
                            .push(&self.buffer[0..self.chunk_size], None, Tag::Message)
                            .unwrap();
                        self.buffer.advance(self.chunk_size);
                        Ok(Async::Ready(Some(Bytes::from(encoded))))
                    } else {
                        trace!("the chunk is not complete");
                        if self.inner_ended {
                            trace!("the stream is closed, encoding whats left");
                            let rest = self.buffer.len();
                            let encoded = stream
                                .push(&self.buffer[0..rest], None, Tag::Message)
                                .unwrap();
                            self.buffer.advance(rest);
                            Ok(Async::Ready(Some(Bytes::from(encoded))))
                        } else {
                            trace!("waiting for more data");
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
                trace!("poll: not ready");
                Ok(Async::NotReady)
            }
            Ok(Async::Ready(Some(bytes))) => {
                trace!("poll: bytes");
                self.buffer.extend(bytes);
                self.encrypt_buffer()
            }
            Ok(Async::Ready(None)) => {
                trace!("poll: over");
                self.inner_ended = true;
                self.encrypt_buffer()
            }
            Err(e) => {
                trace!("poll: error");
                Err(e)
            }
        }
    }
}
