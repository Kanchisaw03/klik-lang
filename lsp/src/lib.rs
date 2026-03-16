// KLIK Language Server Protocol implementation

use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

pub struct KlikLanguageServer {
    client: Client,
    documents: DashMap<Url, String>,
}

impl KlikLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
        }
    }

    async fn analyze_document(&self, uri: &Url, text: &str) {
        let mut diagnostics = Vec::new();
        let file_name = uri.path().to_string();

        // Lex
        let tokens = match klik_lexer::Lexer::new(text, &file_name).tokenize() {
            Ok(t) => t,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("Lexer error: {:?}", e),
                    ..Default::default()
                });
                self.client
                    .publish_diagnostics(uri.clone(), diagnostics, None)
                    .await;
                return;
            }
        };

        // Parse
        let ast = match klik_parser::Parser::new(tokens, &file_name).parse_program() {
            Ok(a) => a,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("Parse error: {:?}", e),
                    ..Default::default()
                });
                self.client
                    .publish_diagnostics(uri.clone(), diagnostics, None)
                    .await;
                return;
            }
        };

        // Semantic analysis
        let mut analyzer = klik_semantic::SemanticAnalyzer::new();
        if let Err(e) = analyzer.analyze(&ast) {
            diagnostics.push(Diagnostic {
                range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!("Semantic error: {:?}", e),
                ..Default::default()
            });
        }

        // Type check
        let mut type_checker = klik_type_system::TypeChecker::new();
        if let Err(e) = type_checker.check_program(&ast) {
            diagnostics.push(Diagnostic {
                range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                severity: Some(DiagnosticSeverity::WARNING),
                message: format!("Type error: {:?}", e),
                ..Default::default()
            });
        }

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    fn collect_items_from_ast<'a>(
        &self,
        ast: &'a klik_ast::Program,
    ) -> Vec<(&'a str, &'a klik_ast::Item)> {
        let mut result = Vec::new();
        for module in &ast.modules {
            for item in &module.items {
                let name = match item {
                    klik_ast::Item::Function(f) => f.name.as_str(),
                    klik_ast::Item::Struct(s) => s.name.as_str(),
                    klik_ast::Item::Enum(e) => e.name.as_str(),
                    klik_ast::Item::Trait(t) => t.name.as_str(),
                    klik_ast::Item::Const(c) => c.name.as_str(),
                    klik_ast::Item::TypeAlias(t) => t.name.as_str(),
                    _ => continue,
                };
                result.push((name, item));
            }
        }
        result
    }

    fn get_completions(&self, text: &str, _position: &Position) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Keywords
        let keywords = [
            "fn", "let", "mut", "if", "else", "while", "for", "in", "return", "struct", "enum",
            "trait", "impl", "import", "from", "pub", "const", "type", "match", "async", "await",
            "spawn", "true", "false", "break", "continue", "test", "mod", "as",
        ];

        for kw in &keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("keyword".to_string()),
                ..Default::default()
            });
        }

        // Built-in functions
        let builtins = [
            ("print", "Print to stdout"),
            ("println", "Print line to stdout"),
            ("assert", "Assert a condition"),
            ("len", "Get length"),
            ("to_string", "Convert to string"),
        ];

        for (name, detail) in &builtins {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(detail.to_string()),
                ..Default::default()
            });
        }

        // Built-in types
        let types = [
            "int", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64", "bool",
            "char", "string", "void",
        ];

        for ty in &types {
            items.push(CompletionItem {
                label: ty.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some("type".to_string()),
                ..Default::default()
            });
        }

        // Try to parse and extract symbols from the document
        let file_name = "<completion>";
        if let Ok(tokens) = klik_lexer::Lexer::new(text, file_name).tokenize() {
            if let Ok(ast) = klik_parser::Parser::new(tokens, file_name).parse_program() {
                for (name, item) in self.collect_items_from_ast(&ast) {
                    match item {
                        klik_ast::Item::Function(f) => {
                            items.push(CompletionItem {
                                label: name.to_string(),
                                kind: Some(CompletionItemKind::FUNCTION),
                                detail: Some(format!("fn {}({} params)", f.name, f.params.len())),
                                ..Default::default()
                            });
                        }
                        klik_ast::Item::Struct(s) => {
                            items.push(CompletionItem {
                                label: name.to_string(),
                                kind: Some(CompletionItemKind::STRUCT),
                                detail: Some(format!("struct {}", s.name)),
                                ..Default::default()
                            });
                        }
                        klik_ast::Item::Enum(e) => {
                            items.push(CompletionItem {
                                label: name.to_string(),
                                kind: Some(CompletionItemKind::ENUM),
                                detail: Some(format!("enum {}", e.name)),
                                ..Default::default()
                            });
                        }
                        klik_ast::Item::Trait(t) => {
                            items.push(CompletionItem {
                                label: name.to_string(),
                                kind: Some(CompletionItemKind::INTERFACE),
                                detail: Some(format!("trait {}", t.name)),
                                ..Default::default()
                            });
                        }
                        klik_ast::Item::Const(c) => {
                            items.push(CompletionItem {
                                label: name.to_string(),
                                kind: Some(CompletionItemKind::CONSTANT),
                                detail: Some(format!("const {}", c.name)),
                                ..Default::default()
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        items
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for KlikLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "klik-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "KLIK Language Server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        self.documents.insert(uri.clone(), text.clone());
        self.analyze_document(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text.clone();
            self.documents.insert(uri.clone(), text.clone());
            self.analyze_document(&uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = &params.text_document_position.position;

        let items = if let Some(doc) = self.documents.get(uri) {
            self.get_completions(doc.value(), position)
        } else {
            Vec::new()
        };

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = &params.text_document_position_params.position;

        if let Some(doc) = self.documents.get(uri) {
            let text = doc.value();
            let lines: Vec<&str> = text.lines().collect();
            if let Some(line) = lines.get(position.line as usize) {
                let col = position.character as usize;
                let start = line[..col]
                    .rfind(|c: char| !c.is_alphanumeric() && c != '_')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let end = line[col..]
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .map(|i| i + col)
                    .unwrap_or(line.len());

                let word = &line[start..end];

                let info = match word {
                    "fn" => Some("Defines a function"),
                    "let" => Some("Declares a variable binding"),
                    "mut" => Some("Makes a binding mutable"),
                    "struct" => Some("Defines a struct type"),
                    "enum" => Some("Defines an enum type"),
                    "trait" => Some("Defines a trait (interface)"),
                    "impl" => Some("Implements methods for a type"),
                    "if" | "else" => Some("Conditional expression"),
                    "while" => Some("While loop"),
                    "for" => Some("For loop (iterates over a range or collection)"),
                    "match" => Some("Pattern matching expression"),
                    "async" => Some("Marks a function as asynchronous"),
                    "await" => Some("Awaits an async operation"),
                    "spawn" => Some("Spawns a concurrent task"),
                    "import" => Some("Imports items from a module"),
                    "return" => Some("Returns a value from a function"),
                    "break" => Some("Breaks out of a loop"),
                    "continue" => Some("Continues to the next loop iteration"),
                    "print" => Some("fn print(value: any) -> void\n\nPrints a value to stdout"),
                    "println" => Some(
                        "fn println(value: any) -> void\n\nPrints a value to stdout with newline",
                    ),
                    "assert" => Some(
                        "fn assert(condition: bool) -> void\n\nAsserts that a condition is true",
                    ),
                    "len" => {
                        Some("fn len(collection: any) -> int\n\nReturns the length of a collection")
                    }
                    _ => None,
                };

                if let Some(info) = info {
                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("```klik\n{}\n```", info),
                        }),
                        range: Some(Range::new(
                            Position::new(position.line, start as u32),
                            Position::new(position.line, end as u32),
                        )),
                    }));
                }
            }
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = &params.text_document_position_params.position;

        if let Some(doc) = self.documents.get(uri) {
            let text = doc.value();
            let lines: Vec<&str> = text.lines().collect();
            if let Some(line) = lines.get(position.line as usize) {
                let col = position.character as usize;
                let start = line[..col]
                    .rfind(|c: char| !c.is_alphanumeric() && c != '_')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let end = line[col..]
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .map(|i| i + col)
                    .unwrap_or(line.len());
                let word = &line[start..end];

                let file_name = uri.path();
                if let Ok(tokens) = klik_lexer::Lexer::new(text, file_name).tokenize() {
                    if let Ok(ast) = klik_parser::Parser::new(tokens, file_name).parse_program() {
                        for (name, item) in self.collect_items_from_ast(&ast) {
                            if name == word {
                                let span = item.span();
                                let loc = Location::new(
                                    uri.clone(),
                                    Range::new(
                                        Position::new(
                                            span.start.line as u32,
                                            span.start.column as u32,
                                        ),
                                        Position::new(span.end.line as u32, span.end.column as u32),
                                    ),
                                );
                                return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;

        if let Some(doc) = self.documents.get(uri) {
            let text = doc.value();
            let file_name = uri.path();

            if let Ok(tokens) = klik_lexer::Lexer::new(text, file_name).tokenize() {
                if let Ok(ast) = klik_parser::Parser::new(tokens, file_name).parse_program() {
                    let formatted = klik_formatter::format_program(&ast);

                    if formatted != *text {
                        let line_count = text.lines().count();
                        let last_line = text.lines().last().unwrap_or("");
                        let edit = TextEdit::new(
                            Range::new(
                                Position::new(0, 0),
                                Position::new(line_count as u32, last_line.len() as u32),
                            ),
                            formatted,
                        );
                        return Ok(Some(vec![edit]));
                    }
                }
            }
        }

        Ok(None)
    }
}

/// Run the LSP server on stdin/stdout
pub async fn run_server() -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(KlikLanguageServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}
