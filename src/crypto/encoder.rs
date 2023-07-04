use super::header::{Header, HEADER_SIZE};
use actix_web::web::{Bytes, BytesMut};
use core::pin::Pin;
use core::task::{Context, Poll};
use data_encoding::HEXLOWER;
use futures_core::stream::Stream;
use log::trace;
use md5::{digest::DynDigest, Digest, Md5};
use sodiumoxide::crypto::secretstream::xchacha20poly1305;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::Key;
use sodiumoxide::crypto::secretstream::Tag;

pub struct Encoder<E> {
    inner: Box<dyn Stream<Item = Result<Bytes, E>> + Unpin>,
    inner_ended: bool,
    stream_encoder: Option<xchacha20poly1305::Stream<xchacha20poly1305::Push>>,
    buffer: BytesMut,
    chunk_size: usize,
    key: Key,
    key_id: u64,
    md5_hasher: Box<dyn DynDigest>,
}

impl<E> Encoder<E> {
    pub fn new(
        key: Key,
        key_id: u64,
        chunk_size: usize,
        s: Box<dyn Stream<Item = Result<Bytes, E>> + Unpin>,
    ) -> Encoder<E> {
        Encoder {
            inner: s,
            inner_ended: false,
            stream_encoder: None,
            buffer: BytesMut::with_capacity(chunk_size),
            chunk_size,
            key,
            key_id,
            md5_hasher: Box::new(Md5::new()),
        }
    }

    pub fn input_md5(self) -> String {
        HEXLOWER.encode(&self.md5_hasher.finalize()[..])
    }

    fn encrypt_buffer(&mut self, cx: &mut Context) -> Poll<Option<Result<Bytes, E>>> {
        if self.buffer.is_empty() {
            trace!("buffer empty, stop");
            Poll::Ready(None)
        } else {
            trace!("buffer not empty");
            match self.stream_encoder {
                None => {
                    trace!("no stream encoder");
                    let (enc_stream, encryption_header) =
                        xchacha20poly1305::Stream::init_push(&self.key).unwrap();

                    self.stream_encoder = Some(enc_stream);

                    let encryption_header_bytes =
                        Bytes::copy_from_slice(encryption_header.as_ref());

                    let mut buf =
                        BytesMut::with_capacity(HEADER_SIZE + encryption_header_bytes.len());

                    let ds_header = Header::new(self.chunk_size, self.key_id);
                    let ds_header_bytes: Vec<u8> = ds_header.into();
                    buf.extend(&ds_header_bytes[..]);
                    buf.extend(encryption_header_bytes);

                    Poll::Ready(Some(Ok(buf.freeze())))
                }

                Some(ref mut stream) => {
                    trace!("stream encoder present !");
                    if self.chunk_size <= self.buffer.len() {
                        trace!("encoding a whole chunk");
                        let encoded = stream
                            .push(&self.buffer.split_to(self.chunk_size), None, Tag::Message)
                            .unwrap();
                        Poll::Ready(Some(Ok(Bytes::from(encoded))))
                    } else {
                        trace!("the chunk is not complete");
                        if self.inner_ended {
                            trace!("the stream is closed, encoding whats left");
                            let rest = self.buffer.len();
                            let encoded = stream
                                .push(&self.buffer.split_to(rest), None, Tag::Message)
                                .unwrap();
                            Poll::Ready(Some(Ok(Bytes::from(encoded))))
                        } else {
                            trace!("waiting for more data");
                            Pin::new(self).poll_next(cx)
                        }
                    }
                }
            }
        }
    }
}

impl<E> Stream for Encoder<E> {
    type Item = Result<Bytes, E>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut encoder = self.get_mut();

        match Pin::new(encoder.inner.as_mut()).poll_next(cx) {
            Poll::Pending => {
                trace!("poll: not ready");
                Poll::Pending
            }
            Poll::Ready(Some(Ok(bytes))) => {
                trace!("poll: bytes");
                encoder.md5_hasher.update(&bytes);
                encoder.buffer.extend_from_slice(&bytes);
                encoder.encrypt_buffer(cx)
            }
            Poll::Ready(Some(Err(e))) => {
                trace!("poll: error");
                Poll::Ready(Some(Err(e)))
            }
            Poll::Ready(None) => {
                trace!("poll: over");
                encoder.inner_ended = true;
                encoder.encrypt_buffer(cx)
            }
        }
    }
}
