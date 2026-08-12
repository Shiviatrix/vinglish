const { spawn } = require('child_process');
const lsp = spawn('/Users/babayaga/.cargo/bin/vng', ['lsp']);

lsp.stdout.on('data', (data) => {
  console.log('STDOUT:', data.toString());
});

lsp.stderr.on('data', (data) => {
  console.error('STDERR:', data.toString());
});

lsp.on('close', (code) => {
  console.log('LSP exited with code', code);
});

const req = {
  jsonrpc: "2.0",
  id: 1,
  method: "initialize",
  params: {
    processId: process.pid,
    rootUri: null,
    capabilities: {}
  }
};
const reqStr = JSON.stringify(req);
const payload = `Content-Length: ${Buffer.byteLength(reqStr)}\r\n\r\n${reqStr}`;
lsp.stdin.write(payload);
