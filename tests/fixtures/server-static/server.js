var http = require('http');
var fs = require('fs');

var express = require('express');
var app = express();
const { Readable } = require('stream');

// FIXME: actix-web always way 5s for the PUT route response.
// See https://github.com/betagouv/ds_proxy/issues/36
app.put('*', function(req, res) {
  req.pipe(fs.createWriteStream(__dirname + '/uploads/' +req.url));

  res.writeHead(200, {'Content-Type': 'text/plain'});
  res.end('OK!');
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

app.get('/chunked/*', function(req, res){
  const path = req.url.substr(8)

  const readStream = fs.createReadStream(__dirname + '/uploads/' + path, { highWaterMark: 1 * 1024});

  res.writeHead(200, {'Content-Type': 'text/plain'});
  readStream.pipe(res);
});

app.get('/get/500', function(req, res){
  res.writeHead(500, {'Content-Type': 'text/plain'});
  res.end('KO: 500');
});

app.get('/get/400', function(req, res){
  res.writeHead(400, {'Content-Type': 'text/plain'});
  res.end('KO: 400');
});

app.use(express.static(__dirname + '/uploads'));
app.listen(3333);
