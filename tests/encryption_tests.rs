extern crate ds_proxy;

use ds_proxy::config::create_key;
use ds_proxy::crypto::*;

use actix_web::web::{BufMut, Bytes, BytesMut};
use actix_web::Error;
use futures::executor::block_on_stream;

use proptest::prelude::*;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::Key;

mod helpers;
pub use helpers::*;

#[test]
fn decrypt_clear_stream() {
    let clear: &[u8] = b"something not encrypted";

    let buf = decrypt_bytes(Bytes::from(clear));

    assert_eq!(clear, &buf[..]);
}

#[test]
fn encoding_then_decoding_returns_source_data() {
    let key: Key = build_key();

    proptest!(|(source_bytes: Vec<u8>, chunk_size in 1usize..10000)| {
        let source : Result<Bytes, Error> = Ok(Bytes::from(source_bytes.clone()));
        let source_stream  = futures::stream::once(Box::pin(async { source }));

        let encoder = Encoder::new(key.clone(), chunk_size, Box::new(source_stream));
        let decoder = Decoder::new(key.clone(), Box::new(encoder));

        let buf = block_on_stream(decoder)
            .map(|r| r.unwrap())
            .fold(BytesMut::with_capacity(64), |mut acc, x| { acc.put(x); acc });

        assert_eq!(source_bytes, &buf[..]);
    });
}

#[test]
fn decrypting_plaintext_returns_plaintext() {
    let key: Key = build_key();

    proptest!(|(clear: Vec<u8>)| {
        let source : Result<Bytes, Error> = Ok(Bytes::from(clear.clone()));
        let source_stream  = futures::stream::once(Box::pin(async { source }));

        let decoder = Decoder::new(key.clone(), Box::new(source_stream));

        let buf = block_on_stream(decoder)
            .map(|r| r.unwrap())
            .fold(BytesMut::with_capacity(64), |mut acc, x| { acc.put(x); acc });

        assert_eq!(clear, &buf[..]);
    });
}

fn build_key() -> Key {
    let password = "Correct Horse Battery Staple".to_string();
    let salt = "abcdefghabcdefghabcdefghabcdefgh".to_string();
    create_key(salt, password).unwrap()
}
