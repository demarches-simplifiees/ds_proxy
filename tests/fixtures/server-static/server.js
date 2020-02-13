var http = require('http');
var fs = require('fs');

var express = require('express');
var app = express();

app.put('*', function(req, res) {
  req.pipe(fs.createWriteStream(__dirname + '/uploads/' +req.url));

  res.writeHead(200, {'Content-Type': 'text/plain'});
  res.end('OK!');
});

app.use(express.static(__dirname + '/uploads'));
app.listen(3000);
