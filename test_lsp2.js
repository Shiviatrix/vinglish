const { spawn } = require('child_process');
const lsp = spawn('/Users/babayaga/.cargo/bin/vng', ['lsp']);

let buffer = '';

lsp.stdout.on('data', (data) => {
  console.log('STDOUT:', data.toString());
});

lsp.stderr.on('data', (data) => {
  console.error('STDERR:', data.toString());
});

function sendReq(req) {
  const reqStr = JSON.stringify(req);
  const payload = `Content-Length: ${Buffer.byteLength(reqStr)}\r\n\r\n${reqStr}`;
  lsp.stdin.write(payload);
}

sendReq({
  jsonrpc: "2.0",
  id: 1,
  method: "initialize",
  params: {
    processId: process.pid,
    rootUri: null,
    capabilities: {}
  }
});

setTimeout(() => {
  sendReq({
    jsonrpc: "2.0",
    method: "initialized",
    params: {}
  });

  sendReq({
    jsonrpc: "2.0",
    method: "textDocument/didOpen",
    params: {
      textDocument: {
        uri: "file:///Users/babayaga/test.ving",
        languageId: "vinglish",
        version: 1,
        text: "function main() returns number begin\nreturn 0\nend\n"
      }
    }
  });

  setTimeout(() => {
    sendReq({
      jsonrpc: "2.0",
      id: 2,
      method: "textDocument/completion",
      params: {
        textDocument: { uri: "file:///Users/babayaga/test.ving" },
        position: { line: 1, character: 2 }
      }
    });

    setTimeout(() => lsp.kill(), 1000);
  }, 500);
}, 500);
