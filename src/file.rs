use futures_fs::FsPool;
use futures::stream::Stream;
use futures::future::Future;
use super::decoder::*;
use super::key::*;
use super::encrypt::*;
use super::config::*;

pub fn encrypt(input_path: String, output_path: String, _config: &Config) {
    let fs = FsPool::default();

    // our source file
    let read = fs.read(input_path, Default::default());

    let key = build_key("some_key".to_string().as_bytes(), &[170, 111, 168, 154, 69, 120, 180, 73, 145, 157, 199, 205, 254, 227, 149, 8, 204, 185, 14, 56, 249, 178, 47, 47, 189, 158, 227, 250, 192, 13, 41, 76]); 

    let encoder = Encoder::new(key, 512, Box::new(read));

    // default writes options to create a new file
    let write = fs.write(output_path, Default::default());

    // block this thread!
    // the reading and writing however will happen off-thread
    encoder.forward(write).wait()
        .expect("IO error piping foo.txt to out.txt");
}

pub fn decrypt(input_path: String, output_path: String, _config: &Config) {
    let fs = FsPool::default();

    // our source file
    let read = fs.read(input_path, Default::default());

    let key = build_key("some_key".to_string().as_bytes(), &[170, 111, 168, 154, 69, 120, 180, 73, 145, 157, 199, 205, 254, 227, 149, 8, 204, 185, 14, 56, 249, 178, 47, 47, 189, 158, 227, 250, 192, 13, 41, 76]); 
    let decoder = Decoder::new(key, 512, Box::new(read));

    // default writes options to create a new file
    let write = fs.write(output_path, Default::default());

    // block this thread!
    // the reading and writing however will happen off-thread
    decoder.forward(write).wait()
        .expect("IO error piping foo.txt to out.txt");
}
