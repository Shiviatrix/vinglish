use vinglish_lexer::{tokenize, LexError};
use vinglish_parser::parse;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

fn offset_to_position(src: &str, offset: u32) -> Position {
    let mut line = 0;
    let mut col = 0;
    for (i, c) in src.char_indices() {
        if i as u32 >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Position { line, character: col }
}

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "eng-lsp initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
            language_id: "vinglish".to_string(),
        })
        .await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.pop() {
            self.on_change(TextDocumentItem {
                uri: params.text_document.uri,
                text: change.text,
                version: params.text_document.version,
                language_id: "vinglish".to_string(),
            })
            .await;
        }
    }
}

impl Backend {
    async fn on_change(&self, params: TextDocumentItem) {
        let (tokens, lex_errors) = tokenize(&params.text);
        
        let mut diagnostics = Vec::new();

        for err in lex_errors {
            let offset = match &err {
                LexError::UnexpectedChar { offset, .. } => *offset,
                LexError::UnterminatedString { offset } => *offset,
                LexError::InvalidNumber { offset, .. } => *offset,
            };
            let pos = offset_to_position(&params.text, offset);
            diagnostics.push(Diagnostic {
                range: Range {
                    start: pos,
                    end: Position { line: pos.line, character: pos.character + 1 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: err.to_string(),
                ..Default::default()
            });
        }

        let (_ast, parse_errors) = parse(&tokens);
        for err in parse_errors {
            let span = err.span();
            diagnostics.push(Diagnostic {
                range: Range {
                    start: offset_to_position(&params.text, span.start),
                    end: offset_to_position(&params.text, span.end),
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: err.to_string(),
                ..Default::default()
            });
        }

        self.client
            .publish_diagnostics(params.uri, diagnostics, Some(params.version))
            .await;
    }
}

pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
