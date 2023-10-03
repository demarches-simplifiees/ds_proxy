#! /bin/bash
PASSWORD='a good password'
SALT='12345678901234567890123456789012'
KEYRING_FILE=/tmp/keyring.toml

DS_PROXY_LOG=/tmp/ds_proxy_log
NODE_LOG=/tmp/node_log

echo 'compiling ds_proxy'
cargo build --release

echo 'building simple node server which mimics a backend storage'
npm install --prefix tests/fixtures/server-static

echo 'building keyring file'
./target/release/ds_proxy add-key --password-file <(echo -n "$PASSWORD") --keyring-file "$KEYRING_FILE" --salt "$SALT"

if [ "$1" = "aws" ]; then
  echo 'launching ds_proxy in aws mode listenning on real s3 backend'
  RUST_LOG=info ./target/release/ds_proxy proxy --address "127.0.0.1:4444" --password-file <(echo -n "$PASSWORD") --salt "$SALT" --keyring-file "$KEYRING_FILE" --upstream-url "https://test-de-proxy.s3-eu-west-1.amazonaws.com" --aws-access-key $ACCESS_KEY --aws-secret-key $SECRET_KEY --aws-region "eu-west-1" > "$DS_PROXY_LOG" 2>&1 &
elif [ "$1" = "fake_aws" ]; then
  echo 'launching ds_proxy in aws mode listenning on 4444 binded on node server'
  RUST_LOG=info ./target/release/ds_proxy proxy --address "127.0.0.1:4444" --password-file <(echo -n "$PASSWORD") --salt "$SALT" --keyring-file "$KEYRING_FILE" --upstream-url "http://localhost:3333" --aws-access-key $ACCESS_KEY --aws-secret-key $SECRET_KEY --aws-region "eu-west-1" > "$DS_PROXY_LOG" 2>&1 &
else
  echo 'launching ds_proxy listenning on 4444 binded on node server'
  RUST_LOG=info,ds_proxy::http::handlers::fetch=trace,ds_proxy::http::handlers::forward=trace ./target/release/ds_proxy proxy --address "127.0.0.1:4444" --password-file <(echo -n "$PASSWORD") --salt "$SALT" --keyring-file "$KEYRING_FILE" --upstream-url "http://localhost:3333" > "$DS_PROXY_LOG" 2>&1 &
fi

echo 'launching fake backend storage with node listenning on 3333'
DEBUG=express:* node tests/fixtures/server-static/server.js > "$NODE_LOG" 2>&1 &

cat << EOF

ds_proxy is now running, and a basic node js server mimics a backend storage.
their logs are $DS_PROXY_LOG and $NODE_LOG

you can add a clear file in the fake storage, and fetch it
curl -X PUT localhost:3333/clear --data-binary @<(echo -n 'I am clear')
cat tests/fixtures/server-static/uploads/clear 
curl localhost:3333/clear

you can encrypt a file by using the ds_proxy, fetch it
curl -X PUT localhost:4444/upstream/cyphered --data-binary @<(echo -n 'What a secret')
curl localhost:4444/upstream/cyphered

you can even try to decrypt the cyphered version by hand
cat tests/fixtures/server-static/uploads/cyphered 
curl localhost:3333/cyphered

EOF

wait
