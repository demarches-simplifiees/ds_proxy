#!/bin/sh
cargo build
file_to_encrypt=src/lib.rs
deciphered_text=plain.txt

cargo run --bin ds_fs_cipher encrypt $file_to_encrypt ciphertext.bin
cargo run --bin ds_fs_cipher decrypt ciphertext.bin $deciphered_text
diff plain.txt $file_to_encrypt
rm $deciphered_text ciphertext.bin

error=$?
if [ $error -eq 0 ]
then
   echo "$file_to_encrypt and $deciphered_text are the same file"
elif [ $error -eq 1 ]
then
   echo "$file_to_encrypt and $deciphered_text differ"
else
   echo "There was something wrong with the diff command"
fi