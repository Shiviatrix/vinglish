use std::collections::HashMap;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use vinglish_lexer::{tokenize, LexError, Span, Spanned, Token};
use vinglish_parser::ast::{Block, Item, LetStmt, Module, Stmt, TypeExpr};
use vinglish_parser::parse;

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
    Position {
        line,
        character: col,
    }
}

fn position_to_offset(src: &str, pos: Position) -> u32 {
    let mut current_line = 0;
    let mut current_col = 0;
    for (i, c) in src.char_indices() {
        if current_line == pos.line && current_col == pos.character {
            return i as u32;
        }
        if c == '\n' {
            current_line += 1;
            current_col = 0;
        } else {
            current_col += 1;
        }
    }
    src.len() as u32
}

fn span_to_range(src: &str, span: Span) -> Range {
    Range {
        start: offset_to_position(src, span.start),
        end: offset_to_position(src, span.end),
    }
}

fn format_type(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Named(id) => id.name.clone(),
        TypeExpr::List(inner) => format!("List of {}", format_type(inner)),
        TypeExpr::Dict { key, val } => format!("Dictionary from {} to {}", format_type(key), format_type(val)),
        TypeExpr::Optional(inner) => format!("Optional {}", format_type(inner)),
        TypeExpr::Result(inner) => format!("Result of {}", format_type(inner)),
        TypeExpr::Reference { mutable, inner } => {
            if *mutable {
                format!("borrow mutable {}", format_type(inner))
            } else {
                format!("borrow {}", format_type(inner))
            }
        },
        TypeExpr::Generic { base, args } => {
            let args_str: Vec<String> = args.iter().map(format_type).collect();
            format!("{}<{}>", base.name, args_str.join(", "))
        }
    }
}

#[derive(Debug)]
struct Backend {
    client: Client,
    cache: RwLock<HashMap<Url, (String, Vec<Spanned<Token>>, Module)>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "vinglish-lsp".to_string(),
                version: Some("0.2.0".to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                    completion_item: None,
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "vinglish-lsp fully initialized with hover, completion, and definition support!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(
            params.text_document.uri,
            params.text_document.text,
            params.text_document.version,
        )
        .await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.pop() {
            self.on_change(params.text_document.uri, change.text, params.text_document.version)
                .await;
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let cache = self.cache.read().await;
        let Some((src, tokens, ast)) = cache.get(&uri) else {
            return Ok(None);
        };

        let offset = position_to_offset(src, pos);
        
        let mut is_field_access = false;
        let mut object_ident = None;
        
        let mut prev_token_idx = None;
        for (i, t) in tokens.iter().enumerate() {
            if t.span.start >= offset {
                break;
            }
            prev_token_idx = Some(i);
        }
        
        if let Some(idx) = prev_token_idx {
            let t = &tokens[idx];
            if matches!(t.node, Token::Dot) {
                is_field_access = true;
                if idx > 0 {
                    let prev_t = &tokens[idx - 1];
                    if let Token::Ident(ref name) = prev_t.node {
                        object_ident = Some(name.clone());
                    }
                }
            }
        }

        let mut items = vec![];

        fn walk_block(block: &Block, offset: u32, locals: &mut Vec<(String, Option<TypeExpr>)>) {
            for stmt in &block.stmts {
                if stmt.span().start >= offset {
                    break;
                }
                match stmt {
                    Stmt::Let(LetStmt { name, ty, .. }) => {
                        locals.push((name.name.clone(), ty.clone()));
                    }
                    Stmt::If(s) => {
                        walk_block(&s.then_block, offset, locals);
                        if let Some(else_b) = &s.otherwise {
                            walk_block(else_b, offset, locals);
                        }
                    }
                    Stmt::When(s) => {
                        walk_block(&s.then_block, offset, locals);
                        if let Some(else_b) = &s.otherwise {
                            walk_block(else_b, offset, locals);
                        }
                    }
                    Stmt::Repeat(vinglish_parser::ast::RepeatStmt::ForEvery { var, body, .. }) => {
                        if body.span.start < offset && offset <= body.span.end {
                            locals.push((var.name.clone(), None));
                        }
                        walk_block(body, offset, locals);
                    }
                    Stmt::Repeat(vinglish_parser::ast::RepeatStmt::While { body, .. }) => {
                        walk_block(body, offset, locals);
                    }
                    Stmt::Repeat(vinglish_parser::ast::RepeatStmt::Count { body, .. }) => {
                        walk_block(body, offset, locals);
                    }
                    Stmt::ParallelRepeat(vinglish_parser::ast::RepeatStmt::ForEvery { var, body, .. }) => {
                        if body.span.start < offset && offset <= body.span.end {
                            locals.push((var.name.clone(), None));
                        }
                        walk_block(body, offset, locals);
                    }
                    Stmt::Match(s) => {
                        for case in &s.cases {
                            walk_block(&case.body, offset, locals);
                        }
                        if let Some(else_b) = &s.otherwise {
                            walk_block(else_b, offset, locals);
                        }
                    }
                    Stmt::Transaction(s) => walk_block(&s.body, offset, locals),
                    _ => {}
                }
            }
        }

        if is_field_access {
            if let Some(obj_name) = object_ident {
                let mut obj_type = None;
                for item in &ast.items {
                    if let Item::Function(f) = item {
                        if offset >= f.span.start && offset <= f.span.end {
                            let mut locals = vec![];
                            for p in &f.params {
                                locals.push((p.name.name.clone(), Some(p.ty.clone())));
                            }
                            walk_block(&f.body, offset, &mut locals);
                            
                            for (name, ty) in locals.into_iter().rev() {
                                if name == obj_name {
                                    obj_type = ty;
                                    break;
                                }
                            }
                        }
                    }
                }
                
                if let Some(TypeExpr::Named(type_id)) = obj_type {
                    for item in &ast.items {
                        if let Item::Type(t) = item {
                            if t.name.name == type_id.name {
                                for field in &t.fields {
                                    items.push(CompletionItem {
                                        label: field.name.name.clone(),
                                        kind: Some(CompletionItemKind::FIELD),
                                        detail: Some(format_type(&field.ty)),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                }
            }
            return Ok(Some(CompletionResponse::Array(items)));
        }

        let mut in_function = false;
        for item in &ast.items {
            if let Item::Function(f) = item {
                if offset >= f.span.start && offset <= f.span.end {
                    in_function = true;
                    let mut locals = vec![];
                    for p in &f.params {
                        locals.push((p.name.name.clone(), Some(p.ty.clone())));
                    }
                    walk_block(&f.body, offset, &mut locals);
                    
                    for (name, ty) in locals {
                        let detail = ty.map(|t| format_type(&t)).unwrap_or_else(|| "Local".to_string());
                        items.push(CompletionItem {
                            label: name,
                            kind: Some(CompletionItemKind::VARIABLE),
                            detail: Some(detail),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        if !in_function {
            let keywords = vec![
                "let", "be", "function", "returns", "if", "then", "otherwise", "when", "repeat", "for",
                "every", "while", "match", "case", "parallel", "spawn", "send", "receive",
                "transaction", "commit", "compile", "use", "package", "module", "public", "private",
                "internal", "type", "requires", "effects", "foreign", "export", "using", "arena",
            ];
            for kw in keywords {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    ..Default::default()
                });
            }
        }

        for item in &ast.items {
            match item {
                Item::Function(f) => {
                    items.push(CompletionItem {
                        label: f.name.name.clone(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some("Function".to_string()),
                        ..Default::default()
                    });
                }
                Item::Type(t) => {
                    items.push(CompletionItem {
                        label: t.name.name.clone(),
                        kind: Some(CompletionItemKind::STRUCT),
                        detail: Some("Type".to_string()),
                        ..Default::default()
                    });
                }
                Item::Enum(e) => {
                    items.push(CompletionItem {
                        label: e.name.name.clone(),
                        kind: Some(CompletionItemKind::ENUM),
                        detail: Some("Enum".to_string()),
                        ..Default::default()
                    });
                }
                _ => {}
            }
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let cache = self.cache.read().await;
        let Some((src, tokens, ast)) = cache.get(&uri) else {
            return Ok(None);
        };

        let offset = position_to_offset(src, pos);
        
        let mut target_ident = None;
        for t in tokens {
            if offset >= t.span.start && offset <= t.span.end {
                if let Token::Ident(ref name) = t.node {
                    target_ident = Some(name.clone());
                } else {
                    match &t.node {
                        Token::Spawn => {
                            return Ok(Some(Hover {
                                contents: HoverContents::Scalar(MarkedString::String("`spawn` creates a new lightweight actor running concurrently.".to_string())),
                                range: Some(span_to_range(src, t.span)),
                            }));
                        },
                        Token::Transaction => {
                            return Ok(Some(Hover {
                                contents: HoverContents::Scalar(MarkedString::String("`transaction` starts a Software Transactional Memory (STM) block.".to_string())),
                                range: Some(span_to_range(src, t.span)),
                            }));
                        },
                        _ => {}
                    }
                }
                break;
            }
        }

        let Some(target) = target_ident else {
            return Ok(None);
        };

        for item in &ast.items {
            match item {
                Item::Function(f) if f.name.name == target => {
                    let mut sig = format!("function {}", f.name.name);
                    let params_str: Vec<String> = f.params.iter().map(|p| format!("{}: {}", p.name.name, format_type(&p.ty))).collect();
                    sig.push_str(&format!("({})", params_str.join(", ")));
                    if let Some(ret) = &f.ret_type {
                        sig.push_str(&format!(" returns {}", format_type(ret)));
                    }
                    let markdown = format!("```vinglish\n{}\n```", sig);
                    return Ok(Some(Hover {
                        contents: HoverContents::Scalar(MarkedString::String(markdown)),
                        range: None,
                    }));
                }
                Item::Type(t) if t.name.name == target => {
                    let markdown = format!("```vinglish\ntype {}\n```", t.name.name);
                    return Ok(Some(Hover {
                        contents: HoverContents::Scalar(MarkedString::String(markdown)),
                        range: None,
                    }));
                }
                Item::Enum(e) if e.name.name == target => {
                    let markdown = format!("```vinglish\nenum {}\n```", e.name.name);
                    return Ok(Some(Hover {
                        contents: HoverContents::Scalar(MarkedString::String(markdown)),
                        range: None,
                    }));
                }
                _ => {}
            }
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let cache = self.cache.read().await;
        let Some((src, tokens, ast)) = cache.get(&uri) else {
            return Ok(None);
        };

        let offset = position_to_offset(src, pos);
        
        let mut target_ident = None;
        for t in tokens {
            if offset >= t.span.start && offset <= t.span.end {
                if let Token::Ident(ref name) = t.node {
                    target_ident = Some(name.clone());
                }
                break;
            }
        }

        let Some(target) = target_ident else {
            return Ok(None);
        };

        for item in &ast.items {
            let (name_ident, span) = match item {
                Item::Function(f) => (&f.name, f.span),
                Item::Type(t) => (&t.name, t.span),
                Item::Enum(e) => (&e.name, e.span),
                _ => continue,
            };

            if name_ident.name == target {
                let range = span_to_range(src, span);
                return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                    uri: uri.clone(),
                    range,
                })));
            }
        }

        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let cache = self.cache.read().await;
        let Some((src, _tokens, ast)) = cache.get(&uri) else {
            return Ok(None);
        };

        let formatted = vinglish_fmt::format_module(ast);
        
        let lines: Vec<&str> = src.lines().collect();
        let end_line = if lines.is_empty() { 0 } else { (lines.len() - 1) as u32 };
        let end_char = if lines.is_empty() { 0 } else { lines.last().unwrap().len() as u32 };

        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: end_line, character: end_char },
        };

        Ok(Some(vec![TextEdit {
            range,
            new_text: formatted,
        }]))
    }
}

impl Backend {
    async fn on_change(&self, uri: Url, text: String, version: i32) {
        let (tokens, lex_errors) = tokenize(&text);
        let mut diagnostics = Vec::new();

        for err in &lex_errors {
            let offset = match err {
                LexError::UnexpectedChar { offset, .. } => *offset,
                LexError::UnterminatedString { offset } => *offset,
                LexError::InvalidNumber { offset, .. } => *offset,
            };
            let pos = offset_to_position(&text, offset);
            diagnostics.push(Diagnostic {
                range: Range {
                    start: pos,
                    end: Position {
                        line: pos.line,
                        character: pos.character + 1,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: err.to_string(),
                ..Default::default()
            });
        }

        let (ast, parse_errors) = parse(&tokens);
        for err in &parse_errors {
            let span = err.span();
            diagnostics.push(Diagnostic {
                range: Range {
                    start: offset_to_position(&text, span.start),
                    end: offset_to_position(&text, span.end),
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: err.to_string(),
                ..Default::default()
            });
        }

        if parse_errors.is_empty() {
            let (_, type_errors, _) = vinglish_types::infer_module(&ast);
            for err in type_errors {
                let span = err.span();
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: offset_to_position(&text, span.start),
                        end: offset_to_position(&text, span.end),
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: err.message(),
                    ..Default::default()
                });
            }
        }

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(version))
            .await;

        self.cache.write().await.insert(uri, (text, tokens, ast));
    }
}

pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        cache: RwLock::new(HashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
