use super::decipher_type::DecipherType;
use super::header;
use actix_web::web::{Bytes, BytesMut};
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::stream::Stream;
use futures_core::Future;
use log::{error, trace};
use std::convert::TryFrom;
use std::fmt::Debug;

pub struct HeaderDecoder<'a, E> {
    inner: &'a mut Box<dyn Stream<Item = Result<Bytes, E>> + Unpin>,
    buffer: BytesMut,
}

impl<E> HeaderDecoder<'_, E> {
    pub fn new(s: &mut Box<dyn Stream<Item = Result<Bytes, E>> + Unpin>) -> HeaderDecoder<E> {
        HeaderDecoder {
            inner: s,
            buffer: BytesMut::new(),
        }
    }
}

impl<E> Future for HeaderDecoder<'_, E>
where
    E: Debug,
{
    type Output = (DecipherType, Option<BytesMut>);

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let decoder = self.get_mut();

        match Pin::new(decoder.inner.as_mut()).poll_next(cx) {
            Poll::Pending => {
                trace!("poll: not ready");
                Poll::Pending
            }
            Poll::Ready(None) => {
                trace!("poll: over");
                Poll::Ready((DecipherType::Plaintext, None))
            }
            Poll::Ready(Some(Err(e))) => {
                error!("poll: error {:?}", e);
                Poll::Ready((DecipherType::Plaintext, None))
            }
            Poll::Ready(Some(Ok(bytes))) => {
                trace!("poll: bytes, + {:?}", bytes.len());
                decoder.buffer.extend(bytes);

                if header::HEADER_SIZE <= decoder.buffer.len() {
                    trace!("enough byte to decide decypher type");

                    match header::Header::try_from(&decoder.buffer[0..header::HEADER_SIZE]) {
                        Ok(header) => {
                            trace!("the file is encrypted !");
                            let _ = decoder.buffer.split_to(header::HEADER_SIZE);
                            trace!("header_size : {:?}", header::HEADER_SIZE);
                            trace!("buffer size left : {:?}", decoder.buffer.len());
                            Poll::Ready((
                                DecipherType::Encrypted {
                                    chunk_size: header.chunk_size,
                                },
                                Some(decoder.buffer.clone()),
                            ))
                        }
                        Err(header::HeaderParsingError::WrongPrefix) => {
                            trace!("the file is not encrypted !");
                            Poll::Ready((DecipherType::Plaintext, Some(decoder.buffer.clone())))
                        }
                        e => {
                            error!("{:?}", e);
                            panic!()
                        }
                    }
                } else {
                    Poll::Pending
                }
            }
        }
    }
}
