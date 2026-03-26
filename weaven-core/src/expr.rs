/// Named Table (§2.8) and Expression Language (§5) runtime.

use std::collections::BTreeMap;
use crate::types::Context;

// ---------------------------------------------------------------------------
// Named Table
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TableValue {
    Num(f64),
    Str(String),
    Bool(bool),
    Table(NamedTableData),
}

impl TableValue {
    pub fn as_f64(&self) -> Option<f64> {
        if let TableValue::Num(n) = self { Some(*n) } else { None }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NamedTableData(pub BTreeMap<String, TableValue>);

impl NamedTableData {
    pub fn new() -> Self { Self::default() }
    pub fn insert(&mut self, key: impl Into<String>, value: TableValue) {
        self.0.insert(key.into(), value);
    }
    /// Owned multi-key lookup: table[key1][key2]...
    pub fn get_nested(&self, keys: &[&str]) -> Option<TableValue> {
        if keys.is_empty() { return None; }
        let first = self.0.get(keys[0])?.clone();
        if keys.len() == 1 { return Some(first); }
        match first {
            TableValue::Table(sub) => sub.get_nested(&keys[1..]),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TableRegistry(pub BTreeMap<String, NamedTableData>);

impl TableRegistry {
    pub fn new() -> Self { Self::default() }
    pub fn register(&mut self, name: impl Into<String>, data: NamedTableData) {
        self.0.insert(name.into(), data);
    }
    pub fn lookup(&self, table: &str, keys: &[&str]) -> Option<TableValue> {
        self.0.get(table)?.get_nested(keys)
    }
}

// ---------------------------------------------------------------------------
// Expression Language AST (§5)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Bool(bool),
    Str(String),
    CtxField(String),
    SigField(String),
    PortReceived(String),
    TableLookup { table: String, keys: Vec<Box<Expr>> },
    BinOp { op: BinOpKind, left: Box<Expr>, right: Box<Expr> },
    Not(Box<Expr>),
    If { cond: Box<Expr>, then_: Box<Expr>, else_: Box<Expr> },
    /// Collection operations (§5.1)
    CollectionAny   { array_field: String, predicate: Box<Expr> },
    CollectionCount { array_field: String, predicate: Box<Expr> },
    CollectionSum   { array_field: String, sum_field: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpKind {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Gt, Lte, Gte,
    And, Or,
}

// ---------------------------------------------------------------------------
// Evaluation context
// ---------------------------------------------------------------------------

pub struct EvalCtx<'a> {
    pub context:        &'a Context,
    pub signal:         Option<&'a BTreeMap<String, f64>>,
    pub received_ports: &'a [String],
    pub tables:         &'a TableRegistry,
}

// ---------------------------------------------------------------------------
// Evaluator (§5.3: strict left-to-right)
// ---------------------------------------------------------------------------

pub fn eval(expr: &Expr, ctx: &EvalCtx) -> f64 {
    match expr {
        Expr::Num(n)  => *n,
        Expr::Bool(b) => if *b { 1.0 } else { 0.0 },
        Expr::Str(_)  => 0.0,

        Expr::CtxField(field) => ctx.context.get(field),

        Expr::SigField(field) => ctx
            .signal
            .and_then(|p| p.get(field.as_str()).copied())
            .unwrap_or(0.0),

        Expr::PortReceived(port) => {
            if ctx.received_ports.iter().any(|p| p == port) { 1.0 } else { 0.0 }
        }

        Expr::TableLookup { table, keys } => {
            let key_strs: Vec<String> = keys.iter().map(|k| eval_to_str(k, ctx)).collect();
            let key_refs: Vec<&str> = key_strs.iter().map(|s| s.as_str()).collect();
            ctx.tables.lookup(table, &key_refs)
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
        }

        Expr::BinOp { op, left, right } => {
            let l = eval(left, ctx);   // left before right (§5.3)
            let r = eval(right, ctx);
            match op {
                BinOpKind::Add => l + r,
                BinOpKind::Sub => l - r,
                BinOpKind::Mul => l * r,
                BinOpKind::Div => if r != 0.0 { l / r } else { 0.0 },
                BinOpKind::Mod => if r != 0.0 { l % r } else { 0.0 },
                BinOpKind::Eq  => if l == r { 1.0 } else { 0.0 },
                BinOpKind::Neq => if l != r { 1.0 } else { 0.0 },
                BinOpKind::Lt  => if l <  r { 1.0 } else { 0.0 },
                BinOpKind::Gt  => if l >  r { 1.0 } else { 0.0 },
                BinOpKind::Lte => if l <= r { 1.0 } else { 0.0 },
                BinOpKind::Gte => if l >= r { 1.0 } else { 0.0 },
                BinOpKind::And => if l != 0.0 && r != 0.0 { 1.0 } else { 0.0 },
                BinOpKind::Or  => if l != 0.0 || r != 0.0 { 1.0 } else { 0.0 },
            }
        }

        Expr::Not(inner) => if eval(inner, ctx) == 0.0 { 1.0 } else { 0.0 },

        Expr::If { cond, then_, else_ } => {
            if eval(cond, ctx) != 0.0 { eval(then_, ctx) } else { eval(else_, ctx) }
        }

        // Collection operations (§5.1) — operate on context array fields.
        // Each element's fields are passed as the `signal` in a fresh EvalCtx,
        // so predicates can reference element fields via `SigField`.
        Expr::CollectionAny { array_field, predicate } => {
            let elements = ctx.context.get_array(array_field);
            let found = elements.iter().any(|elem| {
                let child_ctx = EvalCtx {
                    context: ctx.context,
                    signal: Some(elem),
                    received_ports: ctx.received_ports,
                    tables: ctx.tables,
                };
                eval_bool(predicate, &child_ctx)
            });
            if found { 1.0 } else { 0.0 }
        }

        Expr::CollectionCount { array_field, predicate } => {
            let elements = ctx.context.get_array(array_field);
            elements.iter().filter(|elem| {
                let child_ctx = EvalCtx {
                    context: ctx.context,
                    signal: Some(elem),
                    received_ports: ctx.received_ports,
                    tables: ctx.tables,
                };
                eval_bool(predicate, &child_ctx)
            }).count() as f64
        }

        Expr::CollectionSum { array_field, sum_field } => {
            let elements = ctx.context.get_array(array_field);
            elements.iter()
                .map(|elem| elem.get(sum_field).copied().unwrap_or(0.0))
                .sum()
        }
    }
}

pub fn eval_bool(expr: &Expr, ctx: &EvalCtx) -> bool {
    eval(expr, ctx) != 0.0
}

fn eval_to_str(expr: &Expr, ctx: &EvalCtx) -> String {
    match expr {
        Expr::Str(s)         => s.clone(),
        Expr::CtxField(f)    => ctx.context.get(f).to_string(),
        Expr::SigField(f)    => ctx.signal
            .and_then(|p| p.get(f.as_str()).copied())
            .unwrap_or(0.0).to_string(),
        other                => eval(other, ctx).to_string(),
    }
}

/// Convenience: evaluate a guard expression for a Transition.
pub fn eval_guard(
    expr: &Expr,
    context: &Context,
    signal: Option<&BTreeMap<String, f64>>,
    tables: &TableRegistry,
) -> bool {
    eval_bool(expr, &EvalCtx {
        context,
        signal,
        received_ports: &[],
        tables,
    })
}

// ---------------------------------------------------------------------------
// §11.1 Expression Language — Formal Grammar (BNF) and Parser
//
// Grammar (recursive descent, operator precedence encoded in rule hierarchy):
//
//   expr        ::= if_expr | or_expr
//   if_expr     ::= "if" expr "then" expr "else" expr
//   or_expr     ::= and_expr ("OR" and_expr)*
//   and_expr    ::= not_expr ("AND" not_expr)*
//   not_expr    ::= "NOT" not_expr | cmp_expr
//   cmp_expr    ::= add_expr (cmp_op add_expr)?
//   cmp_op      ::= "==" | "!=" | "<" | ">" | "<=" | ">="
//   add_expr    ::= mul_expr (("+"|"-") mul_expr)*
//   mul_expr    ::= unary (("*"|"/"|"%") unary)*
//   unary       ::= "-" unary | primary
//   primary     ::= "(" expr ")"
//                 | "true" | "false"
//                 | NUMBER
//                 | "context" "." IDENT ("." ("any"|"count") "(" expr ")"
//                                           | "." "sum" "(" IDENT ")")?
//                 | "signal"  "." IDENT
//                 | "port"    "." IDENT "." "received"
//                 | "table"   "." IDENT ("[" expr "]")+
//
//   NUMBER  ::= ["-"]? [0-9]+ ("." [0-9]+)?
//   IDENT   ::= [a-zA-Z_][a-zA-Z0-9_]*
//   STRING  ::= '"' [^"]* '"'
//
// Operator precedence (lowest → highest):
//   OR < AND < NOT < cmp < add/sub < mul/div/mod < unary minus < primary
// ---------------------------------------------------------------------------

/// Parse error produced by the Expression Language parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub pos:     usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error at position {}: {}", self.pos, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse an Expression Language source string into an `Expr` AST.
///
/// # Errors
/// Returns `ParseError` if the input is syntactically invalid or contains
/// constructs not permitted by §5.2 (variable assignment, loops, etc.).
pub fn parse(src: &str) -> Result<Expr, ParseError> {
    let tokens = lex(src)?;
    let mut p = Parser::new(tokens, src);
    let expr = p.parse_expr()?;
    if p.pos < p.tokens.len() {
        return Err(p.err("unexpected token after expression"));
    }
    Ok(expr)
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f64),
    Str(String),
    Ident(String),
    // Punctuation
    LParen, RParen, LBracket, RBracket, Dot, Comma,
    // Arithmetic
    Plus, Minus, Star, Slash, Percent,
    // Comparison
    EqEq, Neq, Lt, Lte, Gt, Gte,
}

fn lex(src: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = src.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        // Skip whitespace
        if bytes[i].is_ascii_whitespace() { i += 1; continue; }

        // String literal "..."
        if bytes[i] == b'"' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' { i += 1; }
            if i >= bytes.len() {
                return Err(ParseError { message: "unterminated string literal".into(), pos: start });
            }
            tokens.push(Token::Str(src[start..i].to_string()));
            i += 1; // consume closing "
            continue;
        }

        // Number literal
        if bytes[i].is_ascii_digit() || (bytes[i] == b'.' && i + 1 < bytes.len() && bytes[i+1].is_ascii_digit()) {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') { i += 1; }
            let s = &src[start..i];
            let n: f64 = s.parse().map_err(|_| ParseError { message: format!("invalid number: {s}"), pos: start })?;
            tokens.push(Token::Num(n));
            continue;
        }

        // Identifier or keyword
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') { i += 1; }
            tokens.push(Token::Ident(src[start..i].to_string()));
            continue;
        }

        // Two-char tokens
        if i + 1 < bytes.len() {
            let two = &src[i..i+2];
            match two {
                "==" => { tokens.push(Token::EqEq);  i += 2; continue; }
                "!=" => { tokens.push(Token::Neq);   i += 2; continue; }
                "<=" => { tokens.push(Token::Lte);   i += 2; continue; }
                ">=" => { tokens.push(Token::Gte);   i += 2; continue; }
                _ => {}
            }
        }

        // Single-char tokens
        let tok = match bytes[i] {
            b'(' => Token::LParen,   b')' => Token::RParen,
            b'[' => Token::LBracket, b']' => Token::RBracket,
            b'.' => Token::Dot,      b',' => Token::Comma,
            b'+' => Token::Plus,     b'-' => Token::Minus,
            b'*' => Token::Star,     b'/' => Token::Slash,
            b'%' => Token::Percent,  b'<' => Token::Lt,
            b'>' => Token::Gt,
            other => return Err(ParseError {
                message: format!("unexpected character '{}'", other as char),
                pos: i,
            }),
        };
        tokens.push(tok);
        i += 1;
    }
    Ok(tokens)
}

// ---------------------------------------------------------------------------
// Parser (recursive descent)
// ---------------------------------------------------------------------------

struct Parser {
    tokens: Vec<Token>,
    pos:    usize,
    src:    String,
}

impl Parser {
    fn new(tokens: Vec<Token>, src: &str) -> Self {
        Self { tokens, pos: 0, src: src.to_string() }
    }

    fn err(&self, msg: &str) -> ParseError {
        ParseError { message: msg.to_string(), pos: self.pos }
    }

    fn peek(&self) -> Option<&Token> { self.tokens.get(self.pos) }

    fn advance(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        if t.is_some() { self.pos += 1; }
        t
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s.clone()),
            _ => Err(self.err("expected identifier")),
        }
    }

    fn expect_token(&mut self, expected: &Token) -> Result<(), ParseError> {
        match self.advance() {
            Some(t) if t == expected => Ok(()),
            _ => Err(self.err(&format!("expected {:?}", expected))),
        }
    }

    fn peek_ident(&self) -> Option<&str> {
        match self.peek() {
            Some(Token::Ident(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    // expr ::= if_expr | or_expr
    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        if self.peek_ident() == Some("if") {
            self.parse_if()
        } else {
            self.parse_or()
        }
    }

    // if_expr ::= "if" expr "then" expr "else" expr
    fn parse_if(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // consume "if"
        let cond = self.parse_expr()?;
        match self.advance() {
            Some(Token::Ident(s)) if s == "then" => {}
            _ => return Err(self.err("expected 'then'")),
        }
        let then_ = self.parse_expr()?;
        match self.advance() {
            Some(Token::Ident(s)) if s == "else" => {}
            _ => return Err(self.err("expected 'else'")),
        }
        let else_ = self.parse_expr()?;
        Ok(Expr::If { cond: Box::new(cond), then_: Box::new(then_), else_: Box::new(else_) })
    }

    // or_expr ::= and_expr ("OR" and_expr)*
    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while self.peek_ident() == Some("OR") {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinOp { op: BinOpKind::Or, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    // and_expr ::= not_expr ("AND" not_expr)*
    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_not()?;
        while self.peek_ident() == Some("AND") {
            self.advance();
            let right = self.parse_not()?;
            left = Expr::BinOp { op: BinOpKind::And, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    // not_expr ::= "NOT" not_expr | cmp_expr
    fn parse_not(&mut self) -> Result<Expr, ParseError> {
        if self.peek_ident() == Some("NOT") {
            self.advance();
            let inner = self.parse_not()?;
            Ok(Expr::Not(Box::new(inner)))
        } else {
            self.parse_cmp()
        }
    }

    // cmp_expr ::= add_expr (cmp_op add_expr)?
    fn parse_cmp(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_add()?;
        let op = match self.peek() {
            Some(Token::EqEq)  => BinOpKind::Eq,
            Some(Token::Neq)   => BinOpKind::Neq,
            Some(Token::Lt)    => BinOpKind::Lt,
            Some(Token::Lte)   => BinOpKind::Lte,
            Some(Token::Gt)    => BinOpKind::Gt,
            Some(Token::Gte)   => BinOpKind::Gte,
            _ => return Ok(left),
        };
        self.advance();
        let right = self.parse_add()?;
        Ok(Expr::BinOp { op, left: Box::new(left), right: Box::new(right) })
    }

    // add_expr ::= mul_expr (("+"|"-") mul_expr)*
    fn parse_add(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus)  => BinOpKind::Add,
                Some(Token::Minus) => BinOpKind::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_mul()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    // mul_expr ::= unary (("*"|"/"|"%") unary)*
    fn parse_mul(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star)    => BinOpKind::Mul,
                Some(Token::Slash)   => BinOpKind::Div,
                Some(Token::Percent) => BinOpKind::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    // unary ::= "-" unary | primary
    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if matches!(self.peek(), Some(Token::Minus)) {
            self.advance();
            let inner = self.parse_unary()?;
            // Optimise literal negation at parse time
            if let Expr::Num(n) = inner {
                return Ok(Expr::Num(-n));
            }
            // Represent as 0 - inner
            return Ok(Expr::BinOp {
                op:    BinOpKind::Sub,
                left:  Box::new(Expr::Num(0.0)),
                right: Box::new(inner),
            });
        }
        self.parse_primary()
    }

    // primary ::= "(" expr ")" | atom
    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            let inner = self.parse_expr()?;
            self.expect_token(&Token::RParen)?;
            return Ok(inner);
        }
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> Result<Expr, ParseError> {
        match self.peek().cloned() {
            Some(Token::Num(n))  => { self.advance(); Ok(Expr::Num(n)) }
            Some(Token::Str(s))  => { self.advance(); Ok(Expr::Str(s)) }
            Some(Token::Ident(ref kw)) => match kw.as_str() {
                "true"    => { self.advance(); Ok(Expr::Bool(true)) }
                "false"   => { self.advance(); Ok(Expr::Bool(false)) }
                "context" => { self.advance(); self.parse_context_ref() }
                "signal"  => { self.advance(); self.parse_signal_ref() }
                "port"    => { self.advance(); self.parse_port_ref() }
                "table"   => { self.advance(); self.parse_table_ref() }
                other     => Err(self.err(&format!("unexpected identifier '{other}'")))
            }
            _ => Err(self.err("expected expression")),
        }
    }

    // "context" already consumed
    // context.FIELD  |  context.FIELD.any(expr)  |  context.FIELD.count(expr)  |  context.FIELD.sum(IDENT)
    fn parse_context_ref(&mut self) -> Result<Expr, ParseError> {
        self.expect_token(&Token::Dot)?;
        let field = self.expect_ident()?;
        // Optional collection operation
        if matches!(self.peek(), Some(Token::Dot)) {
            // Peek ahead: is next token a collection keyword?
            if let Some(Token::Ident(op)) = self.tokens.get(self.pos + 1) {
                match op.as_str() {
                    "any" | "count" => {
                        self.advance(); // consume '.'
                        let op = self.expect_ident()?;
                        self.expect_token(&Token::LParen)?;
                        let pred = self.parse_expr()?;
                        self.expect_token(&Token::RParen)?;
                        return if op == "any" {
                            Ok(Expr::CollectionAny { array_field: field, predicate: Box::new(pred) })
                        } else {
                            Ok(Expr::CollectionCount { array_field: field, predicate: Box::new(pred) })
                        };
                    }
                    "sum" => {
                        self.advance(); // consume '.'
                        self.advance(); // consume "sum"
                        self.expect_token(&Token::LParen)?;
                        let sum_field = self.expect_ident()?;
                        self.expect_token(&Token::RParen)?;
                        return Ok(Expr::CollectionSum { array_field: field, sum_field });
                    }
                    _ => {}
                }
            }
        }
        Ok(Expr::CtxField(field))
    }

    // "signal" already consumed — signal.FIELD
    fn parse_signal_ref(&mut self) -> Result<Expr, ParseError> {
        self.expect_token(&Token::Dot)?;
        let field = self.expect_ident()?;
        Ok(Expr::SigField(field))
    }

    // "port" already consumed — port.IDENT.received
    fn parse_port_ref(&mut self) -> Result<Expr, ParseError> {
        self.expect_token(&Token::Dot)?;
        let name = self.expect_ident()?;
        self.expect_token(&Token::Dot)?;
        match self.advance() {
            Some(Token::Ident(s)) if s == "received" => Ok(Expr::PortReceived(name)),
            _ => Err(self.err("expected 'received' after port name")),
        }
    }

    // "table" already consumed — table.IDENT ("[" expr "]")+
    fn parse_table_ref(&mut self) -> Result<Expr, ParseError> {
        self.expect_token(&Token::Dot)?;
        let table = self.expect_ident()?;
        let mut keys = Vec::new();
        loop {
            if !matches!(self.peek(), Some(Token::LBracket)) { break; }
            self.advance(); // consume '['
            keys.push(Box::new(self.parse_expr()?));
            self.expect_token(&Token::RBracket)?;
        }
        if keys.is_empty() {
            return Err(self.err("table lookup requires at least one key in '['..']'"));
        }
        Ok(Expr::TableLookup { table, keys })
    }
}
