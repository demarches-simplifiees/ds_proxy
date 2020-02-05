use super::config::*;
use super::decoder::*;
use super::encoder::*;
use actix_web::Error;
use bytes::{BufMut, Bytes, BytesMut};
use futures::executor::block_on_stream;

pub fn encrypt(config: Config) {
    let input: Vec<u8> = std::fs::read(config.input_file.unwrap()).unwrap();

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

    std::fs::write(config.output_file.unwrap(), &buf[..]).unwrap();
}

pub fn decrypt(config: Config) {
    let input: Vec<u8> = std::fs::read(config.input_file.unwrap()).unwrap();

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

    std::fs::write(config.output_file.unwrap(), &buf[..]).unwrap();
}
