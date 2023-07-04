const http = require('http');
const fs = require('fs');
const path = require('path');

const express = require('express');
const app = express();
const { Readable } = require('stream');

let last_put_headers = {};

app.put('*', function(req, res) {
  last_put_headers = req.headers;

  const filePath = path.join(__dirname, 'uploads', req.url)
  const fileDirectory = path.dirname(filePath);

  fs.mkdirSync(fileDirectory, { recursive: true })

  writeStream = fs.createWriteStream(filePath);
  req.pipe(writeStream);

  // After all the data is saved, respond Ok
  req.on('end', function () {
    res.writeHead(200, {"content-type":"text/html"});
    res.end('Ok!');
  });

  // This is here incase any errors occur
  writeStream.on('error', function (err) {
    console.log(err);
  });
});

// Add extra latency to all requests
// Usage: node server.js --latency=1000
//
// NB: the latency middleware is added right after the `app.put("*")` route,
// because due to a strange bug actix-web always way 5s for the PUT route response.
// We don't have a fix yet, so for now we don't apply the extra latency to the
// PUT route.
let latencyArg = process.argv.slice(2).find(arg => arg.startsWith('--latency='));
if (latencyArg) {
  const latency = parseInt(latencyArg.split('=')[1], 10);
  if (latency > 0) {
    console.log('Add latency middleware with: ' + latency  + 'ms');
    let latencyMiddleware = function(req,res,next) { setTimeout(next, latency) };
    app.use(latencyMiddleware);
  }
}

app.get('/last_put_headers', function(req, res){
  res.json(last_put_headers);
});

app.get('/get/500', function(req, res){
  res.writeHead(500, {'Content-Type': 'text/plain'});
  res.end('KO: 500');
});

app.get('/get/400', function(req, res){
  res.writeHead(400, {'Content-Type': 'text/plain'});
  res.end('KO: 400');
});

// return a file by chunked if the query param chunked is present
const chunked_static = function (req, res, next) {
  if (!req.query.chunked) {
    return next();
  }

  const path = req.path.substr(1);
  const readStream = fs.createReadStream(__dirname + '/uploads/' + path, { highWaterMark: 1 * 1024});
  res.writeHead(200, {'Content-Type': 'text/plain'});
  readStream.pipe(res);
}

app.use(chunked_static);
app.use(express.static(__dirname + '/uploads'));
app.listen(3333);
