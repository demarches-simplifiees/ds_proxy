use super::header;
use bytes::buf::Buf;
use bytes::Bytes;
use bytes::BytesMut;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::stream::Stream;
use log::{error, trace};
use sodiumoxide::crypto::secretstream::xchacha20poly1305;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::{Header, Key};
use std::convert::TryFrom;

pub struct Decoder<E> {
    inner: Box<dyn Stream<Item = Result<Bytes, E>> + Unpin>,
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
    Plaintext,
}

impl<E> Decoder<E> {
    pub fn new(key: Key, s: Box<dyn Stream<Item = Result<Bytes, E>> + Unpin>) -> Decoder<E> {
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

    pub fn decrypt_buffer(&mut self, cx: &mut Context) -> Poll<Option<Result<Bytes, E>>> {
        if self.inner_ended && self.buffer.is_empty() {
            trace!("buffer empty and stream ended, stop");
            Poll::Ready(None)
        } else {
            match &self.decipher_type {
                DecipherType::DontKnowYet => self.read_header(cx),

                DecipherType::Encrypted => self.decrypt(cx),

                DecipherType::Plaintext => Poll::Ready(Some(Ok(self.buffer.split().freeze()))),
            }
        }
    }

    fn read_header(&mut self, cx: &mut Context) -> Poll<Option<Result<Bytes, E>>> {
        trace!("Decypher type unknown");

        if header::HEADER_SIZE <= self.buffer.len() {
            trace!("enough byte to decide decypher type");

            match header::Header::try_from(&self.buffer[0..header::HEADER_SIZE]) {
                Ok(header) => {
                    trace!("the file is encrypted !");
                    self.chunk_size = header.chunk_size;
                    self.decipher_type = DecipherType::Encrypted;
                    self.buffer.advance(header::HEADER_SIZE);
                    self.decrypt_buffer(cx)
                }
                Err(header::HeaderParsingError::WrongPrefix) => {
                    trace!("the file is not encrypted !");
                    self.decipher_type = DecipherType::Plaintext;
                    self.decrypt_buffer(cx)
                }
                e => {
                    error!("{:?}", e);
                    panic!()
                }
            }
        } else if self.inner_ended {
            trace!("the stream is over, so the file is not encrypted !");

            Poll::Ready(Some(Ok(self.buffer.split().freeze())))
        } else {
            Pin::new(self).poll_next(cx)
        }
    }

    fn decrypt(&mut self, cx: &mut Context) -> Poll<Option<Result<Bytes, E>>> {
        match self.stream_decoder {
            None => {
                trace!("no stream_decoder");

                if xchacha20poly1305::HEADERBYTES <= self.buffer.len() {
                    trace!("decrypting the header");
                    // TODO: throw error
                    let header =
                        Header::from_slice(&self.buffer.split_to(xchacha20poly1305::HEADERBYTES))
                            .unwrap();

                    // TODO: throw error
                    self.stream_decoder =
                        Some(xchacha20poly1305::Stream::init_pull(&header, &self.key).unwrap());

                    self.decrypt_buffer(cx)
                } else {
                    trace!("not enough data to decrypt the header");
                    if self.inner_ended {
                        // TODO: throw error
                        Poll::Ready(None)
                    } else {
                        // waiting for more data
                        Pin::new(self).poll_next(cx)
                    }
                }
            }

            Some(ref mut stream) => {
                trace!("stream_decoder present !");
                trace!("self.buffer.len() : {:?}", self.buffer.len());
                trace!("self.chunk_size {:?}", self.chunk_size);

                let mut chunks = self
                    .buffer
                    .chunks_exact(xchacha20poly1305::ABYTES + self.chunk_size);

                let decrypted: Bytes = chunks
                    .by_ref()
                    .map(|encrypted_chunk| {
                        stream
                            .pull(encrypted_chunk, None)
                            .expect("Unable to decrypt chunk")
                            .0
                    })
                    .flatten()
                    .collect();

                self.buffer = chunks.remainder().into();

                if !decrypted.is_empty() {
                    Poll::Ready(Some(Ok(decrypted)))
                } else if self.inner_ended {
                    trace!("inner stream over, decrypting whats left");

                    let decrypted = stream
                        .pull(&self.buffer.split(), None)
                        .expect("Unable to decrypt last chunk")
                        .0;

                    Poll::Ready(Some(Ok(decrypted.into())))
                } else {
                    trace!("waiting for more data");

                    Pin::new(self).poll_next(cx)
                }
            }
        }
    }
}

impl<E> Stream for Decoder<E> {
    type Item = Result<Bytes, E>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut decoder = self.get_mut();

        match Pin::new(decoder.inner.as_mut()).poll_next(cx) {
            Poll::Pending => {
                trace!("poll: not ready");
                Poll::Pending
            }
            Poll::Ready(Some(Ok(bytes))) => {
                trace!("poll: bytes, + {:?}", bytes.len());
                decoder.buffer.extend(bytes);
                decoder.decrypt_buffer(cx)
            }
            Poll::Ready(None) => {
                trace!("poll: over");
                decoder.inner_ended = true;
                decoder.decrypt_buffer(cx)
            }
            Poll::Ready(Some(Err(e))) => {
                error!("poll: error");
                Poll::Ready(Some(Err(e)))
            }
        }
    }
}
