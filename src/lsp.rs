use crate::ast::*;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::sema::Sema;
use crate::token::Span;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};

// ============================================================================
// Minimal, robust JSON Value, Serializer & Parser (Zero External Dependencies)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_f64().map(|n| n as i64)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn to_json_string(&self) -> String {
        match self {
            JsonValue::Null => "null".to_string(),
            JsonValue::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
            JsonValue::Number(n) => {
                if n.is_finite() && n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            JsonValue::String(s) => {
                let mut out = String::with_capacity(s.len() + 2);
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        '\u{0008}' => out.push_str("\\b"),
                        '\u{000C}' => out.push_str("\\f"),
                        c if (c as u32) < 0x20 => {
                            out.push_str(&format!("\\u{:04x}", c as u32));
                        }
                        _ => out.push(c),
                    }
                }
                out.push('"');
                out
            }
            JsonValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_json_string()).collect();
                format!("[{}]", items.join(","))
            }
            JsonValue::Object(map) => {
                let mut items = Vec::with_capacity(map.len());
                for (k, v) in map {
                    let mut escaped_k = String::with_capacity(k.len() + 2);
                    escaped_k.push('"');
                    for c in k.chars() {
                        if c == '"' {
                            escaped_k.push_str("\\\"");
                        } else if c == '\\' {
                            escaped_k.push_str("\\\\");
                        } else {
                            escaped_k.push(c);
                        }
                    }
                    escaped_k.push('"');
                    items.push(format!("{}:{}", escaped_k, v.to_json_string()));
                }
                format!("{{{}}}", items.join(","))
            }
        }
    }
}

pub struct JsonParser<'a> {
    chars: Vec<char>,
    pos: usize,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> JsonParser<'a> {
    pub fn parse(input: &'a str) -> Result<JsonValue, String> {
        let mut p = Self {
            chars: input.chars().collect(),
            pos: 0,
            _marker: std::marker::PhantomData,
        };
        p.skip_whitespace();
        let val = p.parse_value()?;
        p.skip_whitespace();
        Ok(val)
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if self.pos < self.chars.len() {
            self.pos += 1;
        }
        c
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string(),
            Some('t') | Some('f') => self.parse_bool(),
            Some('n') => self.parse_null(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(format!("Unexpected character '{}' at pos {}", c, self.pos)),
            None => Err("Unexpected end of JSON".to_string()),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.advance(); // consume '{'
        let mut map = HashMap::new();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.advance();
            return Ok(JsonValue::Object(map));
        }

        loop {
            self.skip_whitespace();
            let key = match self.parse_string()? {
                JsonValue::String(s) => s,
                _ => return Err("Expected string key in object".to_string()),
            };

            self.skip_whitespace();
            if self.advance() != Some(':') {
                return Err("Expected ':' after key".to_string());
            }

            let val = self.parse_value()?;
            map.insert(key, val);

            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some('}') => {
                    self.advance();
                    break;
                }
                _ => return Err("Expected ',' or '}' in object".to_string()),
            }
        }
        Ok(JsonValue::Object(map))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.advance(); // consume '['
        let mut arr = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.advance();
            return Ok(JsonValue::Array(arr));
        }

        loop {
            let val = self.parse_value()?;
            arr.push(val);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some(']') => {
                    self.advance();
                    break;
                }
                _ => return Err("Expected ',' or ']' in array".to_string()),
            }
        }
        Ok(JsonValue::Array(arr))
    }

    fn parse_string(&mut self) -> Result<JsonValue, String> {
        self.advance(); // consume '"'
        let mut s = String::new();
        while let Some(c) = self.advance() {
            if c == '"' {
                return Ok(JsonValue::String(s));
            } else if c == '\\' {
                match self.advance() {
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some('/') => s.push('/'),
                    Some('b') => s.push('\u{0008}'),
                    Some('f') => s.push('\u{000C}'),
                    Some('n') => s.push('\n'),
                    Some('r') => s.push('\r'),
                    Some('t') => s.push('\t'),
                    Some('u') => {
                        let mut hex = String::new();
                        for _ in 0..4 {
                            if let Some(h) = self.advance() {
                                hex.push(h);
                            }
                        }
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                s.push(ch);
                            } else {
                                s.push('?');
                            }
                        }
                    }
                    _ => s.push('?'),
                }
            } else {
                s.push(c);
            }
        }
        Err("Unterminated string".to_string())
    }

    fn parse_bool(&mut self) -> Result<JsonValue, String> {
        if self.chars[self.pos..].starts_with(&['t', 'r', 'u', 'e']) {
            self.pos += 4;
            Ok(JsonValue::Bool(true))
        } else if self.chars[self.pos..].starts_with(&['f', 'a', 'l', 's', 'e']) {
            self.pos += 5;
            Ok(JsonValue::Bool(false))
        } else {
            Err("Expected boolean".to_string())
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        if self.chars[self.pos..].starts_with(&['n', 'u', 'l', 'l']) {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err("Expected null".to_string())
        }
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.advance();
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-' {
                self.advance();
            } else {
                break;
            }
        }
        let num_str: String = self.chars[start..self.pos].iter().collect();
        num_str
            .parse::<f64>()
            .map(JsonValue::Number)
            .map_err(|e| format!("Invalid number: {}", e))
    }
}

// ============================================================================
// URI & Safe UTF-8 String Positioning Helpers
// ============================================================================

/// Decode percent-encoded `file://` URI into a real filesystem `PathBuf`.
pub fn decode_uri_to_path(uri: &str) -> PathBuf {
    let raw = if let Some(stripped) = uri.strip_prefix("file://") {
        stripped
    } else {
        uri
    };

    let bytes = raw.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
                decoded.push(b);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }

    let path_str = String::from_utf8_lossy(&decoded).to_string();
    PathBuf::from(path_str)
}

/// Convert a filesystem Path into a standard `file://` URI.
pub fn path_to_uri(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    if path_str.starts_with('/') {
        format!("file://{}", path_str)
    } else {
        format!("file:///{}", path_str)
    }
}

/// Safely convert a 0-based character index into a byte index for string slicing.
/// Prevents panics when working with Cyrillic / multi-byte UTF-8 sequences.
pub fn char_to_byte_index(line: &str, char_idx: usize) -> usize {
    line.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(line.len())
}

/// Safely convert a byte index into a 0-based character index.
pub fn byte_to_char_index(line: &str, byte_idx: usize) -> usize {
    let safe_byte = byte_idx.min(line.len());
    line[..safe_byte].chars().count()
}

/// Safely extract an identifier or word around (line_idx, char_idx) without byte slicing crashes.
pub fn get_word_at_pos(text: &str, line_idx: usize, char_idx: usize) -> Option<String> {
    let line = text.lines().nth(line_idx)?;
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let target = char_idx.min(chars.len());
    let check_idx = if target >= chars.len() && target > 0 {
        target - 1
    } else {
        target
    };

    let is_ident = |c: char| c.is_alphanumeric() || c == '_' || c == '@';
    if check_idx >= chars.len() || !is_ident(chars[check_idx]) {
        return None;
    }

    let mut start = check_idx;
    while start > 0 && is_ident(chars[start - 1]) {
        start -= 1;
    }

    let mut end = check_idx;
    while end < chars.len() && is_ident(chars[end]) {
        end += 1;
    }

    if start == end {
        None
    } else {
        Some(chars[start..end].iter().collect())
    }
}

/// Convert TypeNode into human-readable Cez syntax string.
pub fn type_node_to_string(ty: &TypeNode) -> String {
    match ty {
        TypeNode::Named(name, _) => name.clone(),
        TypeNode::Pointer(inner, _) => format!("*{}", type_node_to_string(inner)),
        TypeNode::Array(inner, size, _) => format!("[{}]{}", size, type_node_to_string(inner)),
        TypeNode::Slice(inner, _) => format!("[]{}", type_node_to_string(inner)),
    }
}

// ============================================================================
// Symbol Origin & Scope Tracking
// ============================================================================

#[derive(Debug, Clone)]
pub struct LocatedDecl {
    pub decl: Decl,
    pub origin_uri: String,
    pub package_alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LocalVarInfo {
    pub name: String,
    pub ty_str: String,
    pub span: Span,
    pub is_param: bool,
}

fn collect_locals_from_stmts(stmts: &[Stmt], locals: &mut Vec<LocalVarInfo>) {
    for stmt in stmts {
        match stmt {
            Stmt::VarDecl { name, ty, span, .. } => {
                let ty_s = ty.as_ref().map(type_node_to_string).unwrap_or_else(|| "auto".to_string());
                locals.push(LocalVarInfo {
                    name: name.clone(),
                    ty_str: ty_s,
                    span: *span,
                    is_param: false,
                });
            }
            Stmt::ShortVarDecl { name, span, .. } => {
                locals.push(LocalVarInfo {
                    name: name.clone(),
                    ty_str: "inferred".to_string(),
                    span: *span,
                    is_param: false,
                });
            }
            Stmt::ConstDecl { name, ty, span, .. } => {
                let ty_s = ty.as_ref().map(type_node_to_string).unwrap_or_else(|| "auto".to_string());
                locals.push(LocalVarInfo {
                    name: name.clone(),
                    ty_str: ty_s,
                    span: *span,
                    is_param: false,
                });
            }
            Stmt::If { init, then_block, else_block, .. } => {
                if let Some(i) = init {
                    collect_locals_from_stmts(&[*(i.clone())], locals);
                }
                collect_locals_from_stmts(then_block, locals);
                if let Some(eb) = else_block {
                    collect_locals_from_stmts(eb, locals);
                }
            }
            Stmt::For { init, body, .. } => {
                if let Some(i) = init {
                    collect_locals_from_stmts(&[*(i.clone())], locals);
                }
                collect_locals_from_stmts(body, locals);
            }
            Stmt::Block(inner, _) => {
                collect_locals_from_stmts(inner, locals);
            }
            Stmt::Switch { cases, default_case, .. } => {
                for c in cases {
                    collect_locals_from_stmts(&c.body, locals);
                }
                if let Some(d) = default_case {
                    collect_locals_from_stmts(d, locals);
                }
            }
            _ => {}
        }
    }
}

// ============================================================================
// Cez Language Server State
// ============================================================================

pub struct CezLsp {
    documents: HashMap<String, String>,
    parsed_programs: HashMap<String, Program>,
    imported_symbols: HashMap<String, Vec<LocatedDecl>>,
}

impl CezLsp {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            parsed_programs: HashMap::new(),
            imported_symbols: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        loop {
            // Read HTTP-like headers: Content-Length: <n>\r\n\r\n
            let mut content_length = None;
            loop {
                let mut line = String::new();
                if handle.read_line(&mut line).unwrap_or(0) == 0 {
                    return; // EOF
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    break; // Header boundary
                }
                if trimmed.to_lowercase().starts_with("content-length:") {
                    if let Some(val) = trimmed.split(':').nth(1) {
                        content_length = val.trim().parse::<usize>().ok();
                    }
                }
            }

            let len = match content_length {
                Some(l) => l,
                None => continue,
            };

            let mut body = vec![0u8; len];
            if handle.read_exact(&mut body).is_err() {
                return;
            }

            let text = match String::from_utf8(body) {
                Ok(t) => t,
                Err(_) => continue,
            };

            if let Ok(msg) = JsonParser::parse(&text) {
                self.handle_message(&msg);
            }
        }
    }

    fn send_response(&self, id: &JsonValue, result: JsonValue) {
        let mut resp = HashMap::new();
        resp.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
        resp.insert("id".to_string(), id.clone());
        resp.insert("result".to_string(), result);
        let resp_val = JsonValue::Object(resp);
        self.send_payload(&resp_val.to_json_string());
    }

    fn send_notification(&self, method: &str, params: JsonValue) {
        let mut notif = HashMap::new();
        notif.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
        notif.insert("method".to_string(), JsonValue::String(method.to_string()));
        notif.insert("params".to_string(), params);
        let notif_val = JsonValue::Object(notif);
        self.send_payload(&notif_val.to_json_string());
    }

    fn send_payload(&self, payload: &str) {
        let msg = format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload);
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(msg.as_bytes());
        let _ = handle.flush();
    }

    fn handle_message(&mut self, msg: &JsonValue) {
        let method = match msg.get("method").and_then(|m| m.as_str()) {
            Some(m) => m,
            None => {
                // If it has an ID but no method, respond with null
                if let Some(id) = msg.get("id") {
                    self.send_response(id, JsonValue::Null);
                }
                return;
            }
        };
        let id = msg.get("id");

        match method {
            "initialize" => {
                if let Some(id) = id {
                    let mut caps = HashMap::new();
                    caps.insert("textDocumentSync".to_string(), JsonValue::Number(1.0)); // Full sync
                    caps.insert("hoverProvider".to_string(), JsonValue::Bool(true));
                    caps.insert("definitionProvider".to_string(), JsonValue::Bool(true));
                    caps.insert("documentSymbolProvider".to_string(), JsonValue::Bool(true));
                    caps.insert("documentFormattingProvider".to_string(), JsonValue::Bool(true));

                    let mut comp = HashMap::new();
                    comp.insert(
                        "triggerCharacters".to_string(),
                        JsonValue::Array(vec![
                            JsonValue::String(".".to_string()),
                            JsonValue::String("@".to_string()),
                            JsonValue::String(":".to_string()),
                        ]),
                    );
                    caps.insert("completionProvider".to_string(), JsonValue::Object(comp));

                    let mut server_info = HashMap::new();
                    server_info.insert("name".to_string(), JsonValue::String("cez-lsp".to_string()));
                    server_info.insert("version".to_string(), JsonValue::String("0.2.0".to_string()));

                    let mut result = HashMap::new();
                    result.insert("capabilities".to_string(), JsonValue::Object(caps));
                    result.insert("serverInfo".to_string(), JsonValue::Object(server_info));

                    self.send_response(id, JsonValue::Object(result));
                }
            }
            "initialized" => {
                // Client confirmed initialization
            }
            "textDocument/didOpen" => {
                if let Some(params) = msg.get("params") {
                    if let Some(doc) = params.get("textDocument") {
                        if let (Some(uri), Some(text)) = (
                            doc.get("uri").and_then(|u| u.as_str()),
                            doc.get("text").and_then(|t| t.as_str()),
                        ) {
                            self.documents.insert(uri.to_string(), text.to_string());
                            self.recheck_document(uri, text);
                        }
                    }
                }
            }
            "textDocument/didChange" => {
                if let Some(params) = msg.get("params") {
                    if let (Some(doc), Some(changes)) = (
                        params.get("textDocument"),
                        params.get("contentChanges").and_then(|c| match c {
                            JsonValue::Array(arr) => Some(arr),
                            _ => None,
                        }),
                    ) {
                        if let Some(uri) = doc.get("uri").and_then(|u| u.as_str()) {
                            if let Some(last_change) = changes.last() {
                                if let Some(new_text) = last_change.get("text").and_then(|t| t.as_str()) {
                                    self.documents.insert(uri.to_string(), new_text.to_string());
                                    self.recheck_document(uri, new_text);
                                }
                            }
                        }
                    }
                }
            }
            "textDocument/didClose" => {
                if let Some(params) = msg.get("params") {
                    if let Some(doc) = params.get("textDocument") {
                        if let Some(uri) = doc.get("uri").and_then(|u| u.as_str()) {
                            self.documents.remove(uri);
                            self.parsed_programs.remove(uri);
                            self.imported_symbols.remove(uri);
                            // Clear diagnostics
                            let mut p = HashMap::new();
                            p.insert("uri".to_string(), JsonValue::String(uri.to_string()));
                            p.insert("diagnostics".to_string(), JsonValue::Array(Vec::new()));
                            self.send_notification("textDocument/publishDiagnostics", JsonValue::Object(p));
                        }
                    }
                }
            }
            "textDocument/hover" => {
                if let Some(id) = id {
                    let res = self.handle_hover(msg.get("params"));
                    self.send_response(id, res);
                }
            }
            "textDocument/completion" => {
                if let Some(id) = id {
                    let res = self.handle_completion(msg.get("params"));
                    self.send_response(id, res);
                }
            }
            "textDocument/definition" => {
                if let Some(id) = id {
                    let res = self.handle_definition(msg.get("params"));
                    self.send_response(id, res);
                }
            }
            "textDocument/documentSymbol" => {
                if let Some(id) = id {
                    let res = self.handle_document_symbols(msg.get("params"));
                    self.send_response(id, res);
                }
            }
            "textDocument/formatting" => {
                if let Some(id) = id {
                    let res = self.handle_formatting(msg.get("params"));
                    self.send_response(id, res);
                }
            }
            "shutdown" => {
                if let Some(id) = id {
                    self.send_response(id, JsonValue::Null);
                }
            }
            "exit" => {
                std::process::exit(0);
            }
            _ => {
                // IMPORTANT: Respond to any unhandled request with an ID so client editors never hang!
                if let Some(id) = id {
                    self.send_response(id, JsonValue::Null);
                }
            }
        }
    }

    // ========================================================================
    // Robust Import Resolution with Origin Tracking
    // ========================================================================

    fn resolve_imports_for_program(&mut self, program: &mut Program, uri: &str) {
        let mut loaded = HashSet::new();
        let file_path = decode_uri_to_path(uri);
        let file_dir = file_path.parent();

        let mut queue: VecDeque<String> = program.imports.iter().cloned().collect();
        let mut located_decls = Vec::new();

        while let Some(imp) = queue.pop_front() {
            if loaded.contains(&imp) {
                continue;
            }
            loaded.insert(imp.clone());

            let mut candidates = Vec::new();

            // 1. CEZ_ROOT environment variable
            if let Ok(root) = std::env::var("CEZ_ROOT") {
                let root_path = PathBuf::from(&root);
                candidates.push(root_path.join(format!("core/{}.cez", imp)));
                candidates.push(root_path.join(format!("{}.cez", imp)));
                candidates.push(root_path.join(format!("core/{}", imp)));
            }

            // 2. Relative to running binary (e.g. ~/.local/bin/cez-lsp)
            if let Ok(exe) = std::env::current_exe() {
                if let Some(exe_dir) = exe.parent() {
                    candidates.push(exe_dir.join(format!("../lib/cez/core/{}.cez", imp)));
                    candidates.push(exe_dir.join(format!("../lib/cez/{}.cez", imp)));
                    candidates.push(exe_dir.join(format!("../core/{}.cez", imp)));
                    candidates.push(exe_dir.join(format!("core/{}.cez", imp)));
                }
            }

            // 3. User-local installs (~/.local/lib/cez/core/)
            if let Ok(home) = std::env::var("HOME") {
                candidates.push(PathBuf::from(format!("{}/.local/lib/cez/core/{}.cez", home, imp)));
                candidates.push(PathBuf::from(format!("{}/.local/lib/cez/{}.cez", home, imp)));
            }

            // 4. Relative to source file
            if let Some(parent) = file_dir {
                candidates.push(parent.join(format!("{}.cez", imp)));
                candidates.push(parent.join(format!("core/{}.cez", imp)));
                candidates.push(parent.join(&imp));
            }

            // 5. Relative to working directory
            candidates.push(PathBuf::from(format!("core/{}.cez", imp)));
            candidates.push(PathBuf::from(format!("core/{}", imp)));
            candidates.push(PathBuf::from(format!("std/{}.cez", imp)));
            candidates.push(PathBuf::from(format!("{}.cez", imp)));
            candidates.push(PathBuf::from(&imp));

            // 6. System paths
            candidates.push(PathBuf::from(format!("/usr/local/lib/cez/core/{}.cez", imp)));
            candidates.push(PathBuf::from(format!("/usr/local/lib/cez/{}.cez", imp)));
            candidates.push(PathBuf::from(format!("/usr/lib/cez/core/{}.cez", imp)));
            candidates.push(PathBuf::from(format!("/usr/lib/cez/{}.cez", imp)));

            for cand in candidates {
                if cand.is_file() {
                    let canon = cand.canonicalize().unwrap_or_else(|_| cand.clone());
                    let origin_uri = path_to_uri(&canon);

                    if let Ok(content) = std::fs::read_to_string(&cand) {
                        let mut lex = Lexer::new(&content);
                        if let Ok(toks) = lex.tokenize_all() {
                            let mut p = Parser::new(toks);
                            if let Ok(imported_prog) = p.parse_program() {
                                // Extract package/alias name: e.g. "fmt" from "core/fmt"
                                let pkg_alias = Path::new(&imp)
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .map(|s| s.to_string());

                                for sub_imp in &imported_prog.imports {
                                    if !loaded.contains(sub_imp) {
                                        queue.push_back(sub_imp.clone());
                                    }
                                }

                                for decl in &imported_prog.decls {
                                    located_decls.push(LocatedDecl {
                                        decl: decl.clone(),
                                        origin_uri: origin_uri.clone(),
                                        package_alias: pkg_alias.clone(),
                                    });
                                }

                                program.decls.extend(imported_prog.decls);
                            }
                        }
                    }
                    break;
                }
            }
        }

        self.imported_symbols.insert(uri.to_string(), located_decls);
    }
}

fn parse_text(t: &str) -> Result<Program, String> {
    let mut lexer = Lexer::new(t);
    let tokens = lexer.tokenize_all()?;
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

fn sanitize_for_completion(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 32);
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_str = false;
    let mut in_char = false;

    while i < len {
        let c = chars[i];
        if c == '"' && !in_char {
            in_str = !in_str;
            result.push(c);
            i += 1;
            continue;
        }
        if c == '\'' && !in_str {
            in_char = !in_char;
            result.push(c);
            i += 1;
            continue;
        }

        if !in_str && !in_char && c == '.' {
            // Check if followed by identifier character
            let next = if i + 1 < len { chars[i + 1] } else { ' ' };
            let is_ident_start = next.is_alphabetic() || next == '_';
            if !is_ident_start {
                // Dangling dot! Insert dummy identifier and semicolon
                result.push('.');
                result.push_str("__cez_dummy__;");
                i += 1;
                continue;
            }
        }

        result.push(c);
        i += 1;
    }
    result
}

impl CezLsp {
    // ========================================================================
    // Safe Diagnostics Checking
    // ========================================================================

    fn recheck_document(&mut self, uri: &str, text: &str) {
        let mut diagnostics = Vec::new();

        let parse_result = parse_text(text);
        match parse_result {
            Ok(mut program) => {
                self.resolve_imports_for_program(&mut program, uri);
                let mut sema = Sema::new();
                if let Err(err_msg) = sema.analyze_program(program.clone()) {
                    diagnostics.push(self.create_diagnostic(&err_msg, uri, text));
                }
                self.parsed_programs.insert(uri.to_string(), program);
            }
            Err(err_msg) => {
                // If standard parse failed (e.g. user is actively typing member access `pt.`),
                // recover AST by sanitizing incomplete tokens so completion, hover and symbols still work!
                let sanitized = sanitize_for_completion(text);
                if let Ok(mut recovered_prog) = parse_text(&sanitized) {
                    self.resolve_imports_for_program(&mut recovered_prog, uri);
                    self.parsed_programs.insert(uri.to_string(), recovered_prog);
                }
                diagnostics.push(self.create_diagnostic(&err_msg, uri, text));
            }
        }

        let mut p = HashMap::new();
        p.insert("uri".to_string(), JsonValue::String(uri.to_string()));
        p.insert("diagnostics".to_string(), JsonValue::Array(diagnostics));
        self.send_notification("textDocument/publishDiagnostics", JsonValue::Object(p));
    }

    fn create_diagnostic(&self, err_msg: &str, _uri: &str, doc_text: &str) -> JsonValue {
        let mut line = 0usize;
        let mut col = 0usize;
        let mut found_pos = false;

        // Try to locate `line:col` in error string
        for word in err_msg.split_whitespace() {
            let trimmed = word.trim_matches(|c: char| !c.is_ascii_digit() && c != ':');
            if let Some((l_str, c_str)) = trimmed.split_once(':') {
                if let (Ok(l), Ok(c)) = (l_str.parse::<usize>(), c_str.parse::<usize>()) {
                    line = l.saturating_sub(1);
                    col = c.saturating_sub(1);
                    found_pos = true;
                    break;
                }
            }
        }

        if !found_pos {
            let words: Vec<&str> = err_msg.split_whitespace().collect();
            for (i, w) in words.iter().enumerate() {
                if *w == "line" && i + 1 < words.len() {
                    let num_str = words[i + 1].trim_matches(|c: char| !c.is_ascii_digit());
                    if let Ok(l) = num_str.parse::<usize>() {
                        line = l.saturating_sub(1);
                        break;
                    }
                }
            }
        }

        let line_str = doc_text.lines().nth(line).unwrap_or("");
        let line_chars = line_str.chars().count();
        let safe_col = col.min(line_chars);

        // Safely determine token length in chars
        let end_col = if safe_col < line_chars {
            let start_byte = char_to_byte_index(line_str, safe_col);
            let rem = &line_str[start_byte..];
            let token_chars = rem
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '(' && *c != ';' && *c != ',' && *c != ')')
                .count()
                .max(1);
            safe_col + token_chars
        } else {
            safe_col + 1
        };

        let mut range = HashMap::new();
        let mut start = HashMap::new();
        start.insert("line".to_string(), JsonValue::Number(line as f64));
        start.insert("character".to_string(), JsonValue::Number(safe_col as f64));
        let mut end = HashMap::new();
        end.insert("line".to_string(), JsonValue::Number(line as f64));
        end.insert("character".to_string(), JsonValue::Number(end_col as f64));

        range.insert("start".to_string(), JsonValue::Object(start));
        range.insert("end".to_string(), JsonValue::Object(end));

        let mut diag = HashMap::new();
        diag.insert("range".to_string(), JsonValue::Object(range));
        diag.insert("severity".to_string(), JsonValue::Number(1.0)); // 1 = Error
        diag.insert("source".to_string(), JsonValue::String("cez".to_string()));
        diag.insert("message".to_string(), JsonValue::String(err_msg.to_string()));

        JsonValue::Object(diag)
    }

    // ========================================================================
    // Rich Hover Tooltips
    // ========================================================================

    fn handle_hover(&self, params: Option<&JsonValue>) -> JsonValue {
        let params = match params {
            Some(p) => p,
            None => return JsonValue::Null,
        };

        let uri = params.get("textDocument").and_then(|d| d.get("uri")).and_then(|u| u.as_str());
        let pos = params.get("position");
        let line = pos.and_then(|p| p.get("line")).and_then(|l| l.as_i64());
        let char_idx = pos.and_then(|p| p.get("character")).and_then(|c| c.as_i64());

        if let (Some(uri), Some(line), Some(char_idx)) = (uri, line, char_idx) {
            if let Some(text) = self.documents.get(uri) {
                if let Some(word) = get_word_at_pos(text, line as usize, char_idx as usize) {
                    let doc_text = self.get_documentation_for_word(uri, &word, line as usize);
                    if let Some(doc) = doc_text {
                        let mut contents = HashMap::new();
                        contents.insert("kind".to_string(), JsonValue::String("markdown".to_string()));
                        contents.insert("value".to_string(), JsonValue::String(doc));

                        let mut res = HashMap::new();
                        res.insert("contents".to_string(), JsonValue::Object(contents));
                        return JsonValue::Object(res);
                    }
                }
            }
        }
        JsonValue::Null
    }

    fn get_documentation_for_word(&self, uri: &str, word: &str, line_idx: usize) -> Option<String> {
        // 1. Built-in keywords & OSDev attributes
        let kw_doc = match word {
            "defer" => "### `defer <expr>`\nОтложенное исполнение выражения в порядке **LIFO** (Last-In First-Out) при выходе из функции. Гарантирует освобождение ресурсов при любом пути возврата.",
            "func" => "### `func [ (recv *Type) ] Name(params...) [ReturnType]`\nОбъявление функции или метода структуры.",
            "package" => "### `package <name>`\nОбъявление пакета (модуля) программы.",
            "var" => "### `var name type [ = init]`\nОбъявление типизированной переменной на стеке или в глобальной памяти.",
            "const" => "### `const NAME type = val`\nОбъявление константы времени компиляции.",
            "type" => "### `type Name struct { ... }`\nОбъявление типа структуры или псевдонима.",
            "inline_asm" => "### `inline_asm(\"assembly\" [ : outputs : inputs ])`\nВстроенный ассемблер x86_64 без накладных расходов. Используется в OSDev для `cli`, `sti`, `hlt`, `lidt`, `in`, `out`.",
            "@packed" => "### `@packed` *(OSDev Attribute)*\nОтключает выравнивание (padding) между полями структуры. Размер структуры в точности равен сумме размеров полей. Необходим для дескрипторов IDT, GDT, TSS и сетевых пакетов.",
            "@align" => "### `@align(N)` *(OSDev Attribute)*\nЗадает строгое выравнивание структуры или глобальной переменной по границе $N$ байт (например, 4096 байт для таблиц страниц PML4/IDT).",
            "@section" => "### `@section(\"name\")` *(OSDev Attribute)*\nПомещает глобальную переменную или функцию в заданную секцию ELF (например, `.multiboot` или `.text.boot`).",
            "@naked" => "### `@naked` *(OSDev Attribute)*\nОтключает генерацию стандартного пролога стека (`push rbp; mov rbp, rsp`) и эпилога. Используется для точек входа ядер ОС (`_start`) и обработчиков прерываний.",
            "@interrupt" => "### `@interrupt` *(OSDev Attribute)*\nПомечает функцию как обработчик аппаратного прерывания CPU.",
            "f32" => "### `f32` (32-bit Float)\nВещественное число одинарной точности (IEEE 754 float). Соответствует типу `float` в LLVM IR.",
            "f64" => "### `f64` (64-bit Float)\nВещественное число двойной точности (IEEE 754 double). Соответствует типу `double` в LLVM IR.",
            "uintptr" => "### `uintptr`\nБеззнаковое целое размера указателя (64 бит на x86_64). Используется для низкоуровневой арифметики указателей и MMIO.",
            "fmt" => "### Пакет `fmt`\nСтандартный модуль ввода/вывода Cez.\n\n* `fmt.Println(s string)` — вывод строки с переносом строки\n* `fmt.Printf(format, ...)` — форматированный вывод (универсальный `%d` для всех int/uint, `%s`, `%f`)\n* `fmt.Print(s string)` — вывод строки без переноса\n* `fmt.Scanln(&buf, max_len)` — ввод строки из stdin\n* `fmt.Scan(&buf)` — ввод слова из stdin\n* `fmt.ReadInt()` — ввод целого числа из stdin\n* `fmt.PrintInt(n int)` — вывод целого числа",
            "Println" => "### `fmt.Println(s string)`\nВыводит строку в стандартный поток вывода `stdout` с автоматическим переводом строки.",
            "Printf" => "### `fmt.Printf(format *i8, ...) i32`\nФорматированный вывод в `stdout` (поддерживает универсальный `%d` для любых целых чисел, `%s`, `%f`, `%p`, `%x` и др.).",
            "Print" => "### `fmt.Print(s string)`\nВыводит строку в `stdout` без перевода строки.",
            "Scanln" => "### `fmt.Scanln(buf *i8, max_len usize) usize`\nСчитывает строку со стандартного ввода `stdin` в буфер `buf` (без символа переноса строки).",
            "ReadLine" => "### `fmt.ReadLine(buf *i8, max_len usize) usize`\nСчитывает строку со стандартного ввода `stdin` в буфер `buf` (псевдоним для `Scanln`).",
            "Scan" => "### `fmt.Scan(buf *i8) i32`\nСчитывает одно слово (до первого пробела или переноса) в буфер `buf`.",
            "ReadInt" => "### `fmt.ReadInt() i64`\nСчитывает целое число из `stdin`.",
            "os" => "### Пакет `os`\nНизкоуровневые примитивы ОС и работа с файлами Cez.\n\n* `os.ReadFile(path)` — быстрое чтение файла в память\n* `os.WriteFile(path, data, len)` — запись буфера в файл\n* `os.StrToInt(s)` — преобразование строки в 64-битное число\n* `os.StrEq(a, b)` — сравнение двух строк на равенство\n* `os.GetEnv(name)` — получение переменной окружения\n* `os.Exit(code)` — немедленное завершение процесса",
            "mem" => "### Пакет `mem`\nУправление памятью и Bump-арены Cez.\n\n* `mem.Arena` — сверхбыстрый линейный аллокатор\n* `mem.Alloc(size)` — выделение памяти\n* `mem.Free(ptr)` — освобождение памяти",
            "std" => "### Пакет `std`\nГлавный пакет стандартной библиотеки Cez (включает `fmt`, `os`, `mem`).",
            _ => "",
        };

        if !kw_doc.is_empty() {
            return Some(kw_doc.to_string());
        }

        // 2. Check local variables and parameters within the active function
        if let Some(prog) = self.parsed_programs.get(uri) {
            let cursor_line = line_idx + 1; // 1-indexed line
            for decl in &prog.decls {
                if let Decl::Func(f) = decl {
                    if cursor_line >= f.span.line {
                        // Check function parameters
                        for p in &f.params {
                            if p.name == word {
                                return Some(format!(
                                    "```go\n(parameter) {}: {}\n```\n*Parameter of function `{}`*",
                                    p.name,
                                    type_node_to_string(&p.ty),
                                    f.name
                                ));
                            }
                        }
                        if let Some(r) = &f.receiver {
                            if r.name == word {
                                return Some(format!(
                                    "```go\n(receiver) {}: {}\n```\n*Method receiver of `{}`*",
                                    r.name,
                                    type_node_to_string(&r.ty),
                                    f.name
                                ));
                            }
                        }
                        // Check local variables
                        if let Some(body) = &f.body {
                            let mut locals = Vec::new();
                            collect_locals_from_stmts(body, &mut locals);
                            for loc in locals {
                                if loc.name == word {
                                    return Some(format!(
                                        "```go\nvar {}: {}\n```\n*Local variable in `{}`*",
                                        loc.name, loc.ty_str, f.name
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Check document top-level declarations
        if let Some(prog) = self.parsed_programs.get(uri) {
            for decl in &prog.decls {
                if let Some(doc) = self.format_decl_doc(decl, word) {
                    return Some(doc);
                }
            }
        }

        // 4. Check imported symbols
        if let Some(loc_decls) = self.imported_symbols.get(uri) {
            for loc in loc_decls {
                if let Some(doc) = self.format_decl_doc(&loc.decl, word) {
                    let origin_name = Path::new(&loc.origin_uri)
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("module");
                    return Some(format!("{}\n\n*Imported from `{}`*", doc, origin_name));
                }
            }
        }

        None
    }

    fn format_decl_doc(&self, decl: &Decl, word: &str) -> Option<String> {
        match decl {
            Decl::Func(f) => {
                if f.name == word {
                    let p_str: Vec<String> = f
                        .params
                        .iter()
                        .map(|p| format!("{} {}", p.name, type_node_to_string(&p.ty)))
                        .collect();
                    let recv_str = if let Some(ref r) = f.receiver {
                        format!("({} {}) ", r.name, type_node_to_string(&r.ty))
                    } else {
                        "".to_string()
                    };
                    let ret_str = if let Some(ref rt) = f.ret_type {
                        format!(" {}", type_node_to_string(rt))
                    } else {
                        "".to_string()
                    };
                    let ext_str = if f.is_extern { "extern " } else { "" };
                    return Some(format!(
                        "```go\n{}func {}{}({}){}\n```\n*Cez Function / Method*",
                        ext_str,
                        recv_str,
                        f.name,
                        p_str.join(", "),
                        ret_str
                    ));
                }
            }
            Decl::TypeDecl { name, def, .. } => {
                if name == word {
                    if let TypeDef::Struct { fields, attributes } = def {
                        let f_str: Vec<String> = fields
                            .iter()
                            .map(|f| format!("    {} {}", f.name, type_node_to_string(&f.ty)))
                            .collect();
                        let attr_str: Vec<String> = attributes.iter().map(|a| format!("{:?}", a)).collect();
                        return Some(format!(
                            "```go\ntype {} struct {} {{\n{}\n}}\n```\n*Cez Struct Definition*",
                            name,
                            attr_str.join(" "),
                            f_str.join("\n")
                        ));
                    }
                }
            }
            Decl::GlobalVar { name, ty, .. } => {
                if name == word {
                    let ty_s = ty.as_ref().map(type_node_to_string).unwrap_or_else(|| "auto".to_string());
                    return Some(format!("```go\nvar {} {}\n```\n*Cez Global Variable*", name, ty_s));
                }
            }
            Decl::Const { name, ty, .. } => {
                if name == word {
                    let ty_s = ty.as_ref().map(type_node_to_string).unwrap_or_else(|| "auto".to_string());
                    return Some(format!("```go\nconst {} {}\n```\n*Cez Constant*", name, ty_s));
                }
            }
        }
        None
    }

    // ========================================================================
    // Context-Aware Completion Engine
    // ========================================================================

    fn handle_completion(&self, params: Option<&JsonValue>) -> JsonValue {
        let uri = params
            .and_then(|p| p.get("textDocument"))
            .and_then(|d| d.get("uri"))
            .and_then(|u| u.as_str());

        let pos = params.and_then(|p| p.get("position"));
        let line_num = pos
            .and_then(|p| p.get("line"))
            .and_then(|l| l.as_i64())
            .map(|n| n as usize);
        let char_num = pos
            .and_then(|p| p.get("character"))
            .and_then(|c| c.as_i64())
            .map(|n| n as usize);

        let mut prefix_before_dot = String::new();
        let mut is_dot_completion = false;
        let mut replace_start_char = char_num.unwrap_or(0);

        if let (Some(u), Some(line_idx), Some(char_idx)) = (uri, line_num, char_num) {
            if let Some(doc) = self.documents.get(u) {
                if let Some(line) = doc.lines().nth(line_idx) {
                    let chars: Vec<char> = line.chars().collect();
                    let end = char_idx.min(chars.len());
                    let line_prefix: String = chars[..end].iter().collect();

                    if let Some(dot_idx) = line_prefix.rfind('.') {
                        let before_dot = &line_prefix[..dot_idx];
                        let ident_start = before_dot
                            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        let ident = &before_dot[ident_start..];
                        if !ident.is_empty() {
                            prefix_before_dot = ident.trim().to_string();
                            is_dot_completion = true;
                            replace_start_char = dot_idx + 1;
                        }
                    } else {
                        let word_start = line_prefix
                            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '@')
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        replace_start_char = word_start;
                    }
                }
            }
        }

        let make_item = |label: &str, kind: f64, detail: &str, snippet: Option<&str>| -> JsonValue {
            let mut item = HashMap::new();
            item.insert("label".to_string(), JsonValue::String(label.to_string()));
            item.insert("kind".to_string(), JsonValue::Number(kind));
            item.insert("detail".to_string(), JsonValue::String(detail.to_string()));
            item.insert("filterText".to_string(), JsonValue::String(label.to_string()));

            let snip = snippet.unwrap_or(label);
            item.insert("insertText".to_string(), JsonValue::String(snip.to_string()));
            if snippet.is_some() {
                item.insert("insertTextFormat".to_string(), JsonValue::Number(2.0)); // Snippet
            }

            if let (Some(line), Some(end_c)) = (line_num, char_num) {
                let mut text_edit = HashMap::new();
                let mut range = HashMap::new();
                let mut start = HashMap::new();
                start.insert("line".to_string(), JsonValue::Number(line as f64));
                start.insert("character".to_string(), JsonValue::Number(replace_start_char as f64));
                let mut end = HashMap::new();
                end.insert("line".to_string(), JsonValue::Number(line as f64));
                end.insert("character".to_string(), JsonValue::Number(end_c as f64));
                range.insert("start".to_string(), JsonValue::Object(start));
                range.insert("end".to_string(), JsonValue::Object(end));
                text_edit.insert("range".to_string(), JsonValue::Object(range));
                text_edit.insert("newText".to_string(), JsonValue::String(snip.to_string()));
                item.insert("textEdit".to_string(), JsonValue::Object(text_edit));
            }

            JsonValue::Object(item)
        };

        let mut items = Vec::new();

        // --- Dot completion (`obj.`) ---
        if is_dot_completion {
            // 1. Built-in packages
            match prefix_before_dot.as_str() {
                "fmt" => {
                    let fmt_members = [
                        ("Println", "func(s string)", "Println(${1:\"\"})"),
                        ("Printf", "func(format *i8, ...) i32", "Printf(\"${1:%s\\n}\", ${2:val})"),
                        ("Print", "func(s string)", "Print(${1:\"\"})"),
                        ("PrintInt", "func(n int)", "PrintInt(${1:n})"),
                        ("PrintChar", "func(c i32)", "PrintChar(${1:c})"),
                        ("Scanln", "func(buf *i8, max_len usize) usize", "Scanln(&${1:buf}, ${2:64})"),
                        ("ReadLine", "func(buf *i8, max_len usize) usize", "ReadLine(&${1:buf}, ${2:64})"),
                        ("Scan", "func(buf *i8) i32", "Scan(&${1:buf})"),
                        ("ReadInt", "func() i64", "ReadInt()"),
                    ];
                    for (label, detail, snippet) in fmt_members {
                        items.push(make_item(label, 3.0, detail, Some(snippet)));
                    }
                    return JsonValue::Array(items);
                }
                "os" => {
                    let os_members = [
                        ("ReadFile", "func(path *i8) FileData", "ReadFile(\"${1:path}\")"),
                        ("WriteFile", "func(path *i8, data *u8, len usize) bool", "WriteFile(\"${1:path}\", ${2:buf}, ${3:len})"),
                        ("Exit", "func(code i32)", "Exit(${1:0})"),
                        ("StrToInt", "func(s *i8) i64", "StrToInt(${1:str})"),
                        ("StrEq", "func(a *i8, b *i8) bool", "StrEq(${1:a}, ${2:b})"),
                        ("GetEnv", "func(name *i8) *i8", "GetEnv(\"${1:VAR}\")"),
                        ("FileData", "type FileData struct", "FileData"),
                    ];
                    for (label, detail, snippet) in os_members {
                        items.push(make_item(label, 3.0, detail, Some(snippet)));
                    }
                    return JsonValue::Array(items);
                }
                "mem" => {
                    let mem_members = [
                        ("Arena", "type Arena struct", "Arena"),
                        ("Alloc", "func(size usize) *u8", "Alloc(${1:size})"),
                        ("Free", "func(ptr *u8)", "Free(${1:ptr})"),
                    ];
                    for (label, detail, snippet) in mem_members {
                        items.push(make_item(label, 3.0, detail, Some(snippet)));
                    }
                    return JsonValue::Array(items);
                }
                _ => {}
            }

            // 2. Check if prefix is an imported package alias
            if let Some(uri_str) = uri {
                if let Some(loc_decls) = self.imported_symbols.get(uri_str) {
                    for loc in loc_decls {
                        if loc.package_alias.as_deref() == Some(&prefix_before_dot) {
                            match &loc.decl {
                                Decl::Func(f) => {
                                    let snip = format!("{}($1)", f.name);
                                    let detail = format!("func {}", f.name);
                                    items.push(make_item(&f.name, 3.0, &detail, Some(&snip)));
                                }
                                Decl::TypeDecl { name, .. } => {
                                    items.push(make_item(name, 22.0, "type", None));
                                }
                                Decl::Const { name, .. } => {
                                    items.push(make_item(name, 21.0, "const", None));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            // 3. Check struct fields and methods on a variable
            if let Some(uri_str) = uri {
                let mut struct_type_name = None;

                // Look for local variable with this name in current function
                if let Some(prog) = self.parsed_programs.get(uri_str) {
                    let cursor_line = line_num.unwrap_or(0) + 1;
                    for decl in &prog.decls {
                        if let Decl::Func(f) = decl {
                            if cursor_line >= f.span.line {
                                // Receiver
                                if let Some(r) = &f.receiver {
                                    if r.name == prefix_before_dot {
                                        struct_type_name = Some(extract_base_type_name(&r.ty));
                                    }
                                }
                                // Params
                                for p in &f.params {
                                    if p.name == prefix_before_dot {
                                        struct_type_name = Some(extract_base_type_name(&p.ty));
                                    }
                                }
                                // Locals
                                if let Some(body) = &f.body {
                                    let mut locals = Vec::new();
                                    collect_locals_from_stmts(body, &mut locals);
                                    for loc in locals {
                                        if loc.name == prefix_before_dot {
                                            struct_type_name = Some(loc.ty_str.trim_start_matches('*').to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // If we know the struct type name, suggest its fields and methods
                let type_target = struct_type_name.unwrap_or_else(|| prefix_before_dot.clone());

                // Find type declaration in current doc or imports
                let mut found_fields = Vec::new();
                if let Some(prog) = self.parsed_programs.get(uri_str) {
                    for decl in &prog.decls {
                        if let Decl::TypeDecl { name, def, .. } = decl {
                            if name == &type_target {
                                if let TypeDef::Struct { fields, .. } = def {
                                    for field in fields {
                                        found_fields.push((field.name.clone(), type_node_to_string(&field.ty)));
                                    }
                                }
                            }
                        }
                        // Methods
                        if let Decl::Func(f) = decl {
                            if let Some(recv) = &f.receiver {
                                if extract_base_type_name(&recv.ty) == type_target {
                                    let snip = format!("{}($1)", f.name);
                                    items.push(make_item(&f.name, 2.0, &format!("method {}", f.name), Some(&snip)));
                                }
                            }
                        }
                    }
                }

                for (fname, fty) in found_fields {
                    items.push(make_item(&fname, 5.0, &fty, None)); // Kind 5 = Field
                }

                if !items.is_empty() {
                    return JsonValue::Array(items);
                }
            }
        }

        // --- Standard Scope Completion ---

        // 1. Keywords (Kind 14)
        let keywords = [
            "package", "import", "func", "type", "struct", "var", "const",
            "if", "else", "for", "return", "defer", "break", "continue",
            "switch", "case", "default", "extern", "inline_asm",
        ];
        for kw in keywords {
            items.push(make_item(kw, 14.0, "Cez keyword", None));
        }

        // 2. Types (Kind 7)
        let types = [
            "int", "uint", "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64",
            "f32", "f64", "bool", "void", "uintptr", "usize", "isize", "string",
        ];
        for t in types {
            items.push(make_item(t, 7.0, "Primitive type", None));
        }

        // 3. OSDev Attributes (Kind 15)
        let attrs = [
            "@packed", "@align(4096)", "@align(4)", "@align(8)",
            "@section(\".multiboot\")", "@section(\".text.boot\")", "@section(\".bss\")",
            "@naked", "@interrupt", "@export",
        ];
        for a in attrs {
            items.push(make_item(a, 15.0, "OSDev attribute", None));
        }

        // 4. Standard library packages (Kind 9)
        let std_packages = [
            ("fmt", "Core formatting & I/O package"),
            ("os", "Core OS, process & file I/O package"),
            ("mem", "Core memory management & Bump arena package"),
            ("std", "Standard library umbrella package"),
        ];
        for (pkg, detail) in std_packages {
            items.push(make_item(pkg, 9.0, detail, None));
        }

        // Standard global functions (Kind 3)
        let std_global_funcs = [
            ("Println", "func(s string)", "Println(${1:\"\"})"),
            ("Printf", "func(format *i8, ...) i32", "Printf(\"${1:%s\\n}\", ${2:val})"),
            ("Print", "func(s string)", "Print(${1:\"\"})"),
            ("Scanln", "func(buf *i8, max_len usize) usize", "Scanln(&${1:buf}, ${2:64})"),
            ("ReadLine", "func(buf *i8, max_len usize) usize", "ReadLine(&${1:buf}, ${2:64})"),
            ("Scan", "func(buf *i8) i32", "Scan(&${1:buf})"),
            ("ReadInt", "func() i64", "ReadInt()"),
            ("ReadFile", "func(path *i8) FileData", "ReadFile(\"${1:path}\")"),
            ("WriteFile", "func(path *i8, data *u8, len usize) bool", "WriteFile(\"${1:path}\", ${2:buf}, ${3:len})"),
            ("Exit", "func(code i32)", "Exit(${1:0})"),
        ];
        for (f, detail, snippet) in std_global_funcs {
            items.push(make_item(f, 3.0, detail, Some(snippet)));
        }

        // 5. Local variables and parameters from current function
        if let Some(u) = uri {
            if let Some(prog) = self.parsed_programs.get(u) {
                let cursor_line = line_num.unwrap_or(0) + 1;
                for decl in &prog.decls {
                    if let Decl::Func(f) = decl {
                        if cursor_line >= f.span.line {
                            for p in &f.params {
                                items.push(make_item(&p.name, 6.0, &format!("param: {}", type_node_to_string(&p.ty)), None));
                            }
                            if let Some(r) = &f.receiver {
                                items.push(make_item(&r.name, 6.0, &format!("receiver: {}", type_node_to_string(&r.ty)), None));
                            }
                            if let Some(body) = &f.body {
                                let mut locals = Vec::new();
                                collect_locals_from_stmts(body, &mut locals);
                                for loc in locals {
                                    items.push(make_item(&loc.name, 6.0, &format!("var: {}", loc.ty_str), None));
                                }
                            }
                        }
                    }
                }
            }
        }

        // 6. Top-level Document Symbols
        if let Some(u) = uri {
            if let Some(prog) = self.parsed_programs.get(u) {
                for decl in &prog.decls {
                    match decl {
                        Decl::Func(f) => {
                            let ret_s = f.ret_type.as_ref().map(type_node_to_string).unwrap_or_else(|| "void".to_string());
                            let snip = format!("{}($1)", f.name);
                            items.push(make_item(&f.name, 3.0, &format!("func -> {}", ret_s), Some(&snip)));
                        }
                        Decl::TypeDecl { name, .. } => {
                            items.push(make_item(name, 22.0, "type", None));
                        }
                        Decl::GlobalVar { name, ty, .. } => {
                            let ty_s = ty.as_ref().map(type_node_to_string).unwrap_or_else(|| "auto".to_string());
                            items.push(make_item(name, 6.0, &ty_s, None));
                        }
                        Decl::Const { name, ty, .. } => {
                            let ty_s = ty.as_ref().map(type_node_to_string).unwrap_or_else(|| "auto".to_string());
                            items.push(make_item(name, 21.0, &ty_s, None));
                        }
                    }
                }
            }

            // 7. Imported declarations
            if let Some(loc_decls) = self.imported_symbols.get(u) {
                for loc in loc_decls {
                    match &loc.decl {
                        Decl::Func(f) => {
                            let snip = format!("{}($1)", f.name);
                            items.push(make_item(&f.name, 3.0, "imported func", Some(&snip)));
                        }
                        Decl::TypeDecl { name, .. } => {
                            items.push(make_item(name, 22.0, "imported type", None));
                        }
                        _ => {}
                    }
                }
            }
        }

        JsonValue::Array(items)
    }

    // ========================================================================
    // Accurate Go to Definition (Multi-file + Local Scopes)
    // ========================================================================

    fn handle_definition(&self, params: Option<&JsonValue>) -> JsonValue {
        let params = match params {
            Some(p) => p,
            None => return JsonValue::Null,
        };

        let uri = params.get("textDocument").and_then(|d| d.get("uri")).and_then(|u| u.as_str());
        let pos = params.get("position");
        let line = pos.and_then(|p| p.get("line")).and_then(|l| l.as_i64());
        let char_idx = pos.and_then(|p| p.get("character")).and_then(|c| c.as_i64());

        if let (Some(uri), Some(line), Some(char_idx)) = (uri, line, char_idx) {
            if let Some(text) = self.documents.get(uri) {
                if let Some(word) = get_word_at_pos(text, line as usize, char_idx as usize) {
                    let make_loc = |loc_uri: &str, span: Span, name_len: usize| -> JsonValue {
                        let mut start = HashMap::new();
                        start.insert("line".to_string(), JsonValue::Number(span.line.saturating_sub(1) as f64));
                        start.insert("character".to_string(), JsonValue::Number(span.col.saturating_sub(1) as f64));

                        let mut end = HashMap::new();
                        end.insert("line".to_string(), JsonValue::Number(span.line.saturating_sub(1) as f64));
                        end.insert("character".to_string(), JsonValue::Number((span.col.saturating_sub(1) + name_len) as f64));

                        let mut range = HashMap::new();
                        range.insert("start".to_string(), JsonValue::Object(start));
                        range.insert("end".to_string(), JsonValue::Object(end));

                        let mut loc = HashMap::new();
                        loc.insert("uri".to_string(), JsonValue::String(loc_uri.to_string()));
                        loc.insert("range".to_string(), JsonValue::Object(range));
                        JsonValue::Object(loc)
                    };

                    // 1. Check local variables and parameters inside active function
                    if let Some(prog) = self.parsed_programs.get(uri) {
                        let cursor_line = (line as usize) + 1;
                        for decl in &prog.decls {
                            if let Decl::Func(f) = decl {
                                if cursor_line >= f.span.line {
                                    for p in &f.params {
                                        if p.name == word {
                                            return make_loc(uri, p.span, word.len());
                                        }
                                    }
                                    if let Some(r) = &f.receiver {
                                        if r.name == word {
                                            return make_loc(uri, r.span, word.len());
                                        }
                                    }
                                    if let Some(body) = &f.body {
                                        let mut locals = Vec::new();
                                        collect_locals_from_stmts(body, &mut locals);
                                        for loc in locals {
                                            if loc.name == word {
                                                return make_loc(uri, loc.span, word.len());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 2. Check declarations in current file
                    if let Some(prog) = self.parsed_programs.get(uri) {
                        for decl in &prog.decls {
                            let (d_name, span) = match decl {
                                Decl::Func(f) => (&f.name, f.span),
                                Decl::TypeDecl { name, span, .. } => (name, *span),
                                Decl::GlobalVar { name, span, .. } => (name, *span),
                                Decl::Const { name, span, .. } => (name, *span),
                            };

                            if d_name == &word {
                                return make_loc(uri, span, word.len());
                            }
                        }
                    }

                    // 3. Check imported symbols (Jumps to the real origin file!)
                    if let Some(loc_decls) = self.imported_symbols.get(uri) {
                        for loc in loc_decls {
                            let (d_name, span) = match &loc.decl {
                                Decl::Func(f) => (&f.name, f.span),
                                Decl::TypeDecl { name, span, .. } => (name, *span),
                                Decl::GlobalVar { name, span, .. } => (name, *span),
                                Decl::Const { name, span, .. } => (name, *span),
                            };

                            if d_name == &word {
                                return make_loc(&loc.origin_uri, span, word.len());
                            }
                        }
                    }
                }
            }
        }
        JsonValue::Null
    }

    // ========================================================================
    // Document Symbols
    // ========================================================================

    fn handle_document_symbols(&self, params: Option<&JsonValue>) -> JsonValue {
        let uri = match params
            .and_then(|p| p.get("textDocument"))
            .and_then(|d| d.get("uri"))
            .and_then(|u| u.as_str())
        {
            Some(u) => u,
            None => return JsonValue::Array(Vec::new()),
        };

        let mut symbols = Vec::new();
        if let Some(prog) = self.parsed_programs.get(uri) {
            for decl in &prog.decls {
                match decl {
                    Decl::Func(f) => {
                        let sym_name = if let Some(r) = &f.receiver {
                            format!("({}) {}", type_node_to_string(&r.ty), f.name)
                        } else {
                            f.name.clone()
                        };
                        symbols.push(self.create_symbol(&sym_name, 12.0, f.span, Vec::new())); // 12 = Function
                    }
                    Decl::TypeDecl { name, def, span } => {
                        let mut children = Vec::new();
                        if let TypeDef::Struct { fields, .. } = def {
                            for f in fields {
                                children.push(self.create_symbol(&f.name, 5.0, f.span, Vec::new())); // 5 = Field
                            }
                        }
                        symbols.push(self.create_symbol(name, 23.0, *span, children)); // 23 = Struct
                    }
                    Decl::GlobalVar { name, span, .. } => {
                        symbols.push(self.create_symbol(name, 13.0, *span, Vec::new())); // 13 = Variable
                    }
                    Decl::Const { name, span, .. } => {
                        symbols.push(self.create_symbol(name, 14.0, *span, Vec::new())); // 14 = Constant
                    }
                }
            }
        }
        JsonValue::Array(symbols)
    }

    fn create_symbol(&self, name: &str, kind: f64, span: Span, children: Vec<JsonValue>) -> JsonValue {
        let mut start = HashMap::new();
        start.insert("line".to_string(), JsonValue::Number(span.line.saturating_sub(1) as f64));
        start.insert("character".to_string(), JsonValue::Number(span.col.saturating_sub(1) as f64));

        let mut end = HashMap::new();
        end.insert("line".to_string(), JsonValue::Number(span.line.saturating_sub(1) as f64));
        end.insert("character".to_string(), JsonValue::Number((span.col.saturating_sub(1) + name.len()) as f64));

        let mut range = HashMap::new();
        range.insert("start".to_string(), JsonValue::Object(start));
        range.insert("end".to_string(), JsonValue::Object(end));

        let mut sym = HashMap::new();
        sym.insert("name".to_string(), JsonValue::String(name.to_string()));
        sym.insert("kind".to_string(), JsonValue::Number(kind));
        sym.insert("range".to_string(), JsonValue::Object(range.clone()));
        sym.insert("selectionRange".to_string(), JsonValue::Object(range));
        if !children.is_empty() {
            sym.insert("children".to_string(), JsonValue::Array(children));
        }

        JsonValue::Object(sym)
    }

    // ========================================================================
    // Document Formatting Provider
    // ========================================================================

    fn handle_formatting(&self, params: Option<&JsonValue>) -> JsonValue {
        let uri = match params
            .and_then(|p| p.get("textDocument"))
            .and_then(|d| d.get("uri"))
            .and_then(|u| u.as_str())
        {
            Some(u) => u,
            None => return JsonValue::Null,
        };

        let doc_text = match self.documents.get(uri) {
            Some(t) => t,
            None => return JsonValue::Null,
        };

        let formatted = format_cez_source(doc_text);
        if formatted == *doc_text {
            return JsonValue::Array(Vec::new());
        }

        let line_count = doc_text.lines().count().max(1);
        let last_line_len = doc_text.lines().last().map(|l| l.chars().count()).unwrap_or(0);

        let mut start = HashMap::new();
        start.insert("line".to_string(), JsonValue::Number(0.0));
        start.insert("character".to_string(), JsonValue::Number(0.0));

        let mut end = HashMap::new();
        end.insert("line".to_string(), JsonValue::Number(line_count as f64));
        end.insert("character".to_string(), JsonValue::Number(last_line_len as f64));

        let mut range = HashMap::new();
        range.insert("start".to_string(), JsonValue::Object(start));
        range.insert("end".to_string(), JsonValue::Object(end));

        let mut text_edit = HashMap::new();
        text_edit.insert("range".to_string(), JsonValue::Object(range));
        text_edit.insert("newText".to_string(), JsonValue::String(formatted));

        JsonValue::Array(vec![JsonValue::Object(text_edit)])
    }
}

// ============================================================================
// Cez Code Formatter
// ============================================================================

/// Format Cez source code with clean indentation (4 spaces) and whitespace normalization.
pub fn format_cez_source(source: &str) -> String {
    let mut out = Vec::new();
    let mut indent_level: usize = 0;
    let mut consecutive_empty_lines = 0;

    for raw_line in source.lines() {
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            consecutive_empty_lines += 1;
            if consecutive_empty_lines <= 1 && !out.is_empty() {
                out.push(String::new());
            }
            continue;
        }
        consecutive_empty_lines = 0;

        // Decrease indent if the line starts with closing brace
        let starts_with_close = trimmed.starts_with('}') || trimmed.starts_with(']');
        if starts_with_close && indent_level > 0 {
            indent_level -= 1;
        }

        let indent_spaces = "    ".repeat(indent_level);
        out.push(format!("{}{}", indent_spaces, trimmed));

        // Count open vs close braces ignoring strings/comments
        let (opens, closes) = count_effective_braces(trimmed);
        if !starts_with_close {
            indent_level = (indent_level + opens).saturating_sub(closes);
        } else {
            // Already subtracted 1 for starting with close
            indent_level = (indent_level + opens).saturating_sub(closes.saturating_sub(1));
        }
    }

    let mut result = out.join("\n");
    if source.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn count_effective_braces(line: &str) -> (usize, usize) {
    let mut opens = 0;
    let mut closes = 0;
    let mut in_str = false;
    let mut in_char = false;
    let mut escaped = false;
    let chars: Vec<char> = line.chars().collect();

    for i in 0..chars.len() {
        let c = chars[i];
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' && !in_char {
            in_str = !in_str;
            continue;
        }
        if c == '\'' && !in_str {
            in_char = !in_char;
            continue;
        }
        if !in_str && !in_char {
            if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                break; // Line comment starts, stop counting
            }
            if c == '{' {
                opens += 1;
            } else if c == '}' {
                closes += 1;
            }
        }
    }

    (opens, closes)
}

fn extract_base_type_name(ty: &TypeNode) -> String {
    match ty {
        TypeNode::Named(n, _) => n.clone(),
        TypeNode::Pointer(inner, _) => extract_base_type_name(inner),
        TypeNode::Array(inner, _, _) => extract_base_type_name(inner),
        TypeNode::Slice(inner, _) => extract_base_type_name(inner),
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_parser_and_serializer() {
        let json_str = r#"{"name":"cez","count":42,"valid":true,"items":["a","b"],"nested":{"k":"v"}}"#;
        let val = JsonParser::parse(json_str).expect("Valid JSON");
        assert_eq!(val.get("name").and_then(|v| v.as_str()), Some("cez"));
        assert_eq!(val.get("count").and_then(|v| v.as_i64()), Some(42));
        assert_eq!(val.get("valid").and_then(|v| v.as_bool()), Some(true));

        let serialized = val.to_json_string();
        let val2 = JsonParser::parse(&serialized).expect("Serialized is valid JSON");
        assert_eq!(val2.get("name").and_then(|v| v.as_str()), Some("cez"));
    }

    #[test]
    fn test_uri_decoding_with_percents() {
        let uri = "file:///home/user/my%20projects/app.cez";
        let path = decode_uri_to_path(uri);
        assert_eq!(path.to_str().unwrap(), "/home/user/my projects/app.cez");

        let roundtrip = path_to_uri(&path);
        assert_eq!(roundtrip, "file:///home/user/my projects/app.cez");
    }

    #[test]
    fn test_utf8_char_positioning_and_word_extraction() {
        let russian_text = "func ПриветМир() {\n    var счетчик = 10;\n}";
        // Line 1: "    var счетчик = 10;"
        // Character index on "счетчик" (approx char 8)
        let word = get_word_at_pos(russian_text, 1, 9);
        assert_eq!(word.as_deref(), Some("счетчик"));

        // Safe slicing without panics
        let line = russian_text.lines().nth(1).unwrap();
        let b = char_to_byte_index(line, 10);
        let c = byte_to_char_index(line, b);
        assert_eq!(c, 10);
    }

    #[test]
    fn test_cez_formatter() {
        let unformatted = "func test(){\nvar x = 1;\nif x == 1{\nreturn;\n}\n}\n";
        let formatted = format_cez_source(unformatted);
        let expected = "func test(){\n    var x = 1;\n    if x == 1{\n        return;\n    }\n}\n";
        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_local_var_completion_and_hover() {
        let mut server = CezLsp::new();
        let uri = "file:///test/main.cez";
        let text = "package main;\n\nfunc calculate(alpha int) int {\n    var beta = 20;\n    return alpha + beta;\n}";

        server.documents.insert(uri.to_string(), text.to_string());
        server.recheck_document(uri, text);

        // Hover over `beta` (line 3, char 8)
        let hover_doc = server.get_documentation_for_word(uri, "beta", 3);
        assert!(hover_doc.is_some());
        assert!(hover_doc.unwrap().contains("Local variable in `calculate`"));

        // Hover over `alpha` (line 3, char 8)
        let param_doc = server.get_documentation_for_word(uri, "alpha", 3);
        assert!(param_doc.is_some());
        assert!(param_doc.unwrap().contains("Parameter of function `calculate`"));
    }

    #[test]
    fn test_struct_field_completion() {
        let mut server = CezLsp::new();
        let uri = "file:///test/main.cez";
        let text = "package main;\n\ntype Point struct {\n    x int\n    y int\n}\n\nfunc main() {\n    var pt Point;\n    pt.\n}";

        server.documents.insert(uri.to_string(), text.to_string());
        server.recheck_document(uri, text);

        // Simulate completion at `pt.` (line 9, character 7)
        let mut pos = HashMap::new();
        pos.insert("line".to_string(), JsonValue::Number(9.0));
        pos.insert("character".to_string(), JsonValue::Number(7.0));

        let mut doc = HashMap::new();
        doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));

        let mut params = HashMap::new();
        params.insert("textDocument".to_string(), JsonValue::Object(doc));
        params.insert("position".to_string(), JsonValue::Object(pos));

        let comp_res = server.handle_completion(Some(&JsonValue::Object(params)));
        if let JsonValue::Array(items) = comp_res {
            let labels: Vec<&str> = items
                .iter()
                .filter_map(|it| it.get("label").and_then(|l| l.as_str()))
                .collect();
            assert!(labels.contains(&"x"), "Should contain field 'x', found: {:?}", labels);
            assert!(labels.contains(&"y"), "Should contain field 'y', found: {:?}", labels);
        } else {
            panic!("Expected array of completion items");
        }
    }

    #[test]
    fn test_definition_local_and_func() {
        let mut server = CezLsp::new();
        let uri = "file:///test/main.cez";
        let text = "package main;\n\nfunc greet() {}\n\nfunc main() {\n    var message = 42;\n    greet();\n}";

        server.documents.insert(uri.to_string(), text.to_string());
        server.recheck_document(uri, text);

        // Definition on `greet` at line 6, col 5
        let mut pos = HashMap::new();
        pos.insert("line".to_string(), JsonValue::Number(6.0));
        pos.insert("character".to_string(), JsonValue::Number(5.0));

        let mut doc = HashMap::new();
        doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));

        let mut params = HashMap::new();
        params.insert("textDocument".to_string(), JsonValue::Object(doc));
        params.insert("position".to_string(), JsonValue::Object(pos));

        let def_res = server.handle_definition(Some(&JsonValue::Object(params)));
        assert_ne!(def_res, JsonValue::Null);
        assert_eq!(def_res.get("uri").and_then(|u| u.as_str()), Some(uri));
    }

    #[test]
    fn test_formatting_via_lsp() {
        let mut server = CezLsp::new();
        let uri = "file:///test/main.cez";
        let unformatted = "func main(){\nvar x = 10;\n}";
        server.documents.insert(uri.to_string(), unformatted.to_string());

        let mut doc = HashMap::new();
        doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));

        let mut params = HashMap::new();
        params.insert("textDocument".to_string(), JsonValue::Object(doc));

        let fmt_res = server.handle_formatting(Some(&JsonValue::Object(params)));
        if let JsonValue::Array(edits) = fmt_res {
            assert_eq!(edits.len(), 1);
            let new_text = edits[0].get("newText").and_then(|t| t.as_str()).unwrap();
            assert!(new_text.contains("    var x = 10;"));
        } else {
            panic!("Expected array of text edits for formatting");
        }
    }
}
