use super::config::*;
use super::crypto::*;
use actix_web::web::{BufMut, Bytes, BytesMut};
use actix_web::Error;
use futures::executor::block_on_stream;

pub fn encrypt(config: EncryptConfig) {
    let input: Vec<u8> = std::fs::read(config.input_file).unwrap();

    let source: Result<Bytes, Error> = Ok(Bytes::from(input));
    let source_stream = futures::stream::once(Box::pin(async { source }));

    let encoder = Encoder::new(config.key, config.chunk_size, Box::new(source_stream));

    let buf = block_on_stream(encoder).map(|r| r.unwrap()).fold(
        BytesMut::with_capacity(64),
        |mut acc, x| {
            acc.put(x);
            acc
        },
    );

    std::fs::write(config.output_file, &buf[..]).unwrap();
}

pub fn decrypt(config: DecryptConfig) {
    let input: Vec<u8> = std::fs::read(config.input_file).unwrap();

    let source: Result<Bytes, Error> = Ok(Bytes::from(input));
    let source_stream = futures::stream::once(Box::pin(async { source }));

    let decoder = Decoder::new(config.key, Box::new(source_stream));

    let buf = block_on_stream(decoder).map(|r| r.unwrap()).fold(
        BytesMut::with_capacity(64),
        |mut acc, x| {
            acc.put(x);
            acc
        },
    );

    std::fs::write(config.output_file, &buf[..]).unwrap();
}
