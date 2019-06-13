use bytes::Bytes;
use bytes::BytesMut;
use futures::prelude::*;
use futures::stream::Stream;
use sodiumoxide::crypto::secretstream::xchacha20poly1305;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::{Header, Key};
use log::trace;
use std::convert::TryInto;
use super::{HEADER_SIZE, HEADER_PREFIX, HEADER_PREFIX_SIZE, HEADER_VERSION_NB, HEADER_VERSION_NB_SIZE};

pub struct Decoder <E> {
    inner: Box<Stream<Item = Bytes, Error = E>>,
    inner_ended: bool,
    decipher_type: DecipherType,
    stream_decoder: Option<xchacha20poly1305::Stream<xchacha20poly1305::Pull>>,
    buffer: BytesMut,
    chunk_size: usize,
    key: Key,
}

enum DecipherType {
    DontKnowYet,
    Encrypted,
    Plaintext
}

impl<E> Decoder<E> {
    pub fn new(key: Key, s: Box<Stream<Item = Bytes, Error = E>>) -> Decoder<E> {
        Decoder {
            inner: s,
            inner_ended: false,
            decipher_type: DecipherType::DontKnowYet,
            stream_decoder: None,
            buffer: BytesMut::new(),
            chunk_size: 0,
            key,
        }
    }

    pub fn decrypt_buffer(&mut self) -> Poll<Option<Bytes>, E> {
        if self.inner_ended && self.buffer.is_empty() {
            trace!("buffer empty and stream ended, stop");
            Ok(Async::Ready(None))
        } else {
            match &self.decipher_type {
                DecipherType::DontKnowYet =>  self.read_header(),

                DecipherType::Encrypted => self.decrypt(),

                DecipherType::Plaintext => { Ok(Async::Ready(Some(self.buffer.take().into()))) }
            }
        }
    }

    fn read_header(&mut self) -> Poll<Option<Bytes>, E> {
        trace!("Decypher type unknown");

        if HEADER_SIZE <= self.buffer.len() {
            trace!("not enough byte to decide decypher type");

            let stream_header = &self.buffer[0..HEADER_PREFIX_SIZE];
            let version_nb_header: u32 = u32::from_le_bytes(self.buffer[HEADER_PREFIX_SIZE..HEADER_PREFIX_SIZE + HEADER_VERSION_NB_SIZE].try_into().expect("slice with incorrect length"));

            if stream_header == HEADER_PREFIX && version_nb_header == HEADER_VERSION_NB {
                trace!("the file is encrypted !");
                self.decipher_type = DecipherType::Encrypted;
                self.chunk_size = usize::from_le_bytes(self.buffer[HEADER_PREFIX_SIZE + HEADER_VERSION_NB_SIZE..HEADER_SIZE].try_into().expect("slice with incorrect length"));
                self.buffer.advance(HEADER_SIZE);
            } else {
                trace!("the file is not encrypted !");
                self.decipher_type = DecipherType::Plaintext;
            }

            self.poll()
        }
        else if self.inner_ended {
            trace!("the stream is over, so the file is not encrypted !");
            Ok(Async::Ready(Some(self.buffer.take().into())))
        } else {
            self.poll()
        }
    }

    fn decrypt(&mut self) -> Poll<Option<Bytes>, E> {
        match self.stream_decoder {
            None => {
                trace!("no stream_decoder");

                if xchacha20poly1305::HEADERBYTES <= self.buffer.len() {
                    trace!("decrypting the header");
                    // TODO: throw error
                    let header = Header::from_slice(&self.buffer[0..xchacha20poly1305::HEADERBYTES]).unwrap();

                    // TODO: throw error
                    self.stream_decoder = Some(xchacha20poly1305::Stream::init_pull(&header, &self.key).unwrap());

                    self.buffer.advance(xchacha20poly1305::HEADERBYTES);

                    self.decrypt_buffer()
                } else {
                    trace!("not enough data to decrypt the header");
                    if self.inner_ended {
                        // TODO: throw error
                        Ok(Async::Ready(None))
                    } else {
                        // waiting for more data
                        self.poll()
                    }
                }
            },

            Some(ref mut stream) => {
                trace!("stream_decoder present !");

                if (xchacha20poly1305::ABYTES + self.chunk_size) <= self.buffer.len() {
                    trace!("decrypting a whole buffer");
                    let (decrypted1, _tag1) = stream.pull(&self.buffer[0..(xchacha20poly1305::ABYTES + self.chunk_size)], None).unwrap();
                    self.buffer.advance(xchacha20poly1305::ABYTES + self.chunk_size);
                    Ok(Async::Ready(Some(Bytes::from(&decrypted1[..]))))
                } else if self.inner_ended {
                    trace!("inner stream over, decrypting whats left");
                    let rest = self.buffer.len();
                    let (decrypted1, _tag1) = stream.pull(&self.buffer[..], None).unwrap();
                    self.buffer.advance(rest);
                    Ok(Async::Ready(Some(Bytes::from(&decrypted1[..]))))
                } else {
                    trace!("waiting for more data");
                    self.poll()
                }
            }
        }
    }
}

impl<E> Stream for Decoder<E> {
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
                self.decrypt_buffer()
            }
            Ok(Async::Ready(None)) => {
                trace!("poll: over");
                self.inner_ended = true;
                self.decrypt_buffer()
            }
            Err(e) => {
                trace!("poll: error");
                Err(e)
            }
        }
    }
}
