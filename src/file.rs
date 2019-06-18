use super::config::*;
use super::decoder::*;
use super::encoder::*;
use futures::future::Future;
use futures::stream::Stream;
use futures_fs::FsPool;

pub fn encrypt(config: Config) {
    let fs = FsPool::default();

    // our source file
    let read = fs.read(config.input_file.unwrap(), Default::default());

    let encoder = Encoder::new(config.key, config.chunk_size, Box::new(read));

    // default writes options to create a new file
    let write = fs.write(config.output_file.unwrap(), Default::default());

    // block this thread!
    // the reading and writing however will happen off-thread
    encoder
        .forward(write)
        .wait()
        .expect("IO error piping foo.txt to out.txt");
}

pub fn decrypt(config: Config) {
    let fs = FsPool::default();

    // our source file
    let read = fs.read(config.input_file.unwrap(), Default::default());

    let decoder = Decoder::new(config.key, Box::new(read));

    // default writes options to create a new file
    let write = fs.write(config.output_file.unwrap(), Default::default());

    // block this thread!
    // the reading and writing however will happen off-thread
    decoder
        .forward(write)
        .wait()
        .expect("IO error piping foo.txt to out.txt");
}
