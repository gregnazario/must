const http = require("http");

const port = process.env.PORT || 8080;

const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "text/html" });
  res.end("<html><body><h1>Web Frontend</h1></body></html>");
});

server.listen(port, () => {
  console.log(`Web server listening on port ${port}`);
});
