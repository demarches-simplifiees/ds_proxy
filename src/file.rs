use futures_fs::FsPool;
use futures::stream::Stream;
use futures::future::Future;
use super::decoder::*;
use super::key::*;
use super::encrypt::*;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::Key;

pub fn encrypt() {
    let fs = FsPool::default();

    // our source file
    let read = fs.read("clear.txt", Default::default());

    // default writes options to create a new file
    let write = fs.write("encrypted.txt", Default::default());

    // block this thread!
    // the reading and writing however will happen off-thread
    encrypt_stream(read).forward(write).wait()
        .expect("IO error piping foo.txt to out.txt");
}

pub fn decrypt() {
    let fs = FsPool::default();

    // our source file
    let mut read = fs.read("encrypted.txt", Default::default());

    let key: Key = build_key();
    let decoder = Decoder::new(key, &mut read);

    // default writes options to create a new file
    let write = fs.write("decrypted.txt", Default::default());

    // block this thread!
    // the reading and writing however will happen off-thread
    decoder.forward(write).wait()
        .expect("IO error piping foo.txt to out.txt");
}
