use futures_fs::FsPool;
use futures::stream::Stream;
use futures::future::Future;
use super::decoder::*;
use super::key::*;
use super::encrypt::*;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::Key;

pub fn encrypt(input_path: String, output_path: String, noop: bool) {
    let fs = FsPool::default();

    // our source file
    let read = fs.read(input_path, Default::default());

    let key: Key = build_key();
    let encoder = Encoder::new(key, 512, Box::new(read));

    // default writes options to create a new file
    let write = fs.write(output_path, Default::default());

    // block this thread!
    // the reading and writing however will happen off-thread
    encoder.forward(write).wait()
        .expect("IO error piping foo.txt to out.txt");
}

pub fn decrypt(input_path: String, output_path: String, noop: bool) {
    let fs = FsPool::default();

    // our source file
    let read = fs.read(input_path, Default::default());

    let key: Key = build_key();
    let decoder = Decoder::new(key, 512, Box::new(read));

    // default writes options to create a new file
    let write = fs.write(output_path, Default::default());

    // block this thread!
    // the reading and writing however will happen off-thread
    decoder.forward(write).wait()
        .expect("IO error piping foo.txt to out.txt");
}
