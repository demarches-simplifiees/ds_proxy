var http = require('http');
var fs = require('fs');

var express = require('express');
var app = express();
const { Readable } = require('stream');

app.put('*', function(req, res) {
  req.pipe(fs.createWriteStream(__dirname + '/uploads/' +req.url));

  res.writeHead(200, {'Content-Type': 'text/plain'});
  res.end('OK!');
});

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
app.listen(3000);
