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
use super::key::*;


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
            let encoded = enc_stream.push(&slice, None, Tag::Message).unwrap();
            Bytes::from(encoded)
        });

    header_stream.chain(encoder)
}
