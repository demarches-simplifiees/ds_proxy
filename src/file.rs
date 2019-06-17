use super::config::*;
use super::decoder::*;
use super::encoder::*;
use futures::future::Future;
use futures::stream::Stream;
use futures_fs::FsPool;

pub fn encrypt(input_path: String, output_path: String, config: &Config) {
    let fs = FsPool::default();

    // our source file
    let read = fs.read(input_path, Default::default());

    let encoder = Encoder::new(config.key.clone(), config.chunk_size, Box::new(read));

    // default writes options to create a new file
    let write = fs.write(output_path, Default::default());

    // block this thread!
    // the reading and writing however will happen off-thread
    encoder
        .forward(write)
        .wait()
        .expect("IO error piping foo.txt to out.txt");
}

pub fn decrypt(input_path: String, output_path: String, config: &Config) {
    let fs = FsPool::default();

    // our source file
    let read = fs.read(input_path, Default::default());

    let decoder = Decoder::new(config.key.clone(), Box::new(read));

    // default writes options to create a new file
    let write = fs.write(output_path, Default::default());

    // block this thread!
    // the reading and writing however will happen off-thread
    decoder
        .forward(write)
        .wait()
        .expect("IO error piping foo.txt to out.txt");
}
