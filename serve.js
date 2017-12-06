const WebSocket = require('ws');
const http = require('http');

let ws = null;
wss = new WebSocket.Server({ port: parseInt(`${process.argv[2]}001`) });
wss.on('connection', s => ws = s);

process.openStdin().on('data', chunk => {
  if (!ws) return;
  const colour = chunk.toString('utf-8').split('\n').shift();
  ws.send(colour);
});

http.createServer((_, res) => res.write(`<script>
new WebSocket('ws://localhost:${process.argv[2]}001').onmessage = m =>
  document.body.style.backgroundColor = 'rgb(' + m.data + ')';
</script>`) && res.end()).listen(parseInt(`${process.argv[2]}000`));
