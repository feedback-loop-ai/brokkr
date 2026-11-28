//! Typed `runs-on` label and expression evaluator (#1417, decision 0071 rulings 3 and 5).
//!
//! Evaluates which runner labels a job's `runs-on` field can yield under its
//! matrix without misclassifying predicate-only matrix axes or quoted literals
//! inside comparisons (`==`, `!=`) or the left-hand side of `&&`.

use serde::Deserialize;

use super::condition::read_single_quoted_literal;
use super::{Matrix, RunsOn};

#[derive(Deserialize)]
#[serde(untagged)]
enum JsonLabels {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Or,
    And,
    Cmp,
    Bang,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Star,
    Str(String),
    Ident(String),
}

enum Expr {
    Or(Box<Self>, Box<Self>),
    BoolOp,
    Str(String),
    MatrixAxis(String),
    FromJson(Box<Self>),
    Call { name: String, args: Vec<Self> },
    Other,
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-')
}

fn lex_punct(bytes: &[u8], i: usize) -> Option<(Token, usize)> {
    match bytes.get(i..i + 2) {
        Some(b"||") => return Some((Token::Or, i + 2)),
        Some(b"&&") => return Some((Token::And, i + 2)),
        Some(b"==" | b"!=" | b"<=" | b">=") => return Some((Token::Cmp, i + 2)),
        _ => {}
    }
    let tok = match bytes[i] {
        b'<' | b'>' => Token::Cmp,
        b'!' => Token::Bang,
        b'(' => Token::LParen,
        b')' => Token::RParen,
        b'[' => Token::LBracket,
        b']' => Token::RBracket,
        b',' => Token::Comma,
        b'.' => Token::Dot,
        b'*' => Token::Star,
        _ => return None,
    };
    Some((tok, i + 1))
}

fn lex(src: &str) -> Option<Vec<Token>> {
    let bytes = src.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
        } else if bytes[i] == b'\'' {
            let (s, next) = read_single_quoted_literal(src, i)?;
            tokens.push(Token::Str(s));
            i = next;
        } else if is_ident_byte(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_ident_byte(bytes[i]) {
                i += 1;
            }
            tokens.push(Token::Ident(src[start..i].to_string()));
        } else {
            let (tok, next) = lex_punct(bytes, i)?;
            tokens.push(tok);
            i = next;
        }
    }
    Some(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn eat(&mut self, expected: &Token) -> bool {
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn parse_or(&mut self) -> Option<Expr> {
        let mut lhs = self.parse_and()?;
        while self.eat(&Token::Or) {
            let rhs = self.parse_and()?;
            lhs = Expr::Or(Box::new(lhs), Box::new(rhs));
        }
        Some(lhs)
    }

    fn parse_and(&mut self) -> Option<Expr> {
        let mut lhs = self.parse_cmp()?;
        // In `pred && rhs`, if `pred` is falsy the expression yields a falsy
        // value (never a runner label); if truthy it yields `rhs`.
        while self.eat(&Token::And) {
            lhs = self.parse_cmp()?;
        }
        Some(lhs)
    }

    fn parse_cmp(&mut self) -> Option<Expr> {
        let mut lhs = self.parse_unary()?;
        while self.eat(&Token::Cmp) {
            let _rhs = self.parse_unary()?;
            lhs = Expr::BoolOp;
        }
        Some(lhs)
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        if self.eat(&Token::Bang) {
            let _inner = self.parse_unary()?;
            return Some(Expr::BoolOp);
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        match self.peek()?.clone() {
            Token::LParen => {
                self.pos += 1;
                let inner = self.parse_or()?;
                self.eat(&Token::RParen).then_some(inner)
            }
            Token::Str(s) => {
                self.pos += 1;
                Some(Expr::Str(s))
            }
            Token::Ident(name) => {
                self.pos += 1;
                self.parse_ident_tail(name)
            }
            _ => None,
        }
    }

    fn parse_ident_tail(&mut self, name: String) -> Option<Expr> {
        if self.eat(&Token::LParen) {
            let args = self.parse_call_args()?;
            let expr = build_call(name, args);
            return self.consume_postfix(expr, None);
        }
        let base_is_matrix = name.eq_ignore_ascii_case("matrix");
        self.consume_postfix(Expr::Other, base_is_matrix.then_some(()))
    }

    fn parse_call_args(&mut self) -> Option<Vec<Expr>> {
        let mut args = Vec::new();
        if self.eat(&Token::RParen) {
            return Some(args);
        }
        loop {
            args.push(self.parse_or()?);
            if self.eat(&Token::Comma) {
                continue;
            }
            return self.eat(&Token::RParen).then_some(args);
        }
    }

    fn consume_postfix(&mut self, base: Expr, matrix_base: Option<()>) -> Option<Expr> {
        let mut segs: Vec<Option<String>> = Vec::new();
        loop {
            if self.eat(&Token::Dot) {
                match self.peek()?.clone() {
                    Token::Ident(seg) => {
                        self.pos += 1;
                        segs.push(Some(seg));
                    }
                    Token::Star => {
                        self.pos += 1;
                        segs.push(None);
                    }
                    _ => return None,
                }
            } else if self.eat(&Token::LBracket) {
                let idx = self.parse_or()?;
                if !self.eat(&Token::RBracket) {
                    return None;
                }
                match idx {
                    Expr::Str(s) => segs.push(Some(s)),
                    _ => segs.push(None),
                }
            } else {
                break;
            }
        }
        if matrix_base.is_some() && segs.len() == 1 {
            if let Some(axis) = segs.pop().flatten() {
                return Some(Expr::MatrixAxis(axis));
            }
        }
        Some(if segs.is_empty() { base } else { Expr::Other })
    }
}

fn build_call(name: String, mut args: Vec<Expr>) -> Expr {
    if name.eq_ignore_ascii_case("fromjson") && args.len() == 1 {
        return Expr::FromJson(Box::new(args.remove(0)));
    }
    if matches!(
        name.to_ascii_lowercase().as_str(),
        "contains" | "startswith" | "endswith" | "always" | "success" | "failure" | "cancelled"
    ) {
        return Expr::BoolOp;
    }
    Expr::Call { name, args }
}

fn parse_expr(src: &str) -> Option<Expr> {
    let tokens = lex(src)?;
    let mut parser = Parser {
        tokens: &tokens,
        pos: 0,
    };
    let expr = parser.parse_or()?;
    (parser.pos == tokens.len()).then_some(expr)
}

fn push_label_value(raw: &str, in_json: bool, out: &mut Vec<String>) {
    if !in_json {
        out.push(raw.to_string());
        return;
    }
    match serde_json::from_str::<JsonLabels>(raw) {
        Ok(JsonLabels::One(label)) => out.push(label),
        Ok(JsonLabels::Many(labels)) => out.extend(labels),
        Err(_) => {}
    }
}

fn eval_matrix_axis(matrix: &Matrix, axis: &str, in_json: bool, out: &mut Vec<String>) {
    for (k, vals) in &matrix.axes {
        if k.eq_ignore_ascii_case(axis) {
            for v in vals {
                if let Some(s) = v.as_str() {
                    push_label_value(s, in_json, out);
                }
            }
        }
    }
    for row in &matrix.include {
        for (k, v) in row {
            if k.eq_ignore_ascii_case(axis) {
                if let Some(s) = v.as_str() {
                    push_label_value(s, in_json, out);
                }
            }
        }
    }
}

fn eval_format_call(args: &[Expr], matrix: Option<&Matrix>, in_json: bool, out: &mut Vec<String>) {
    let Some(Expr::Str(fmt)) = args.first() else {
        return;
    };
    let mut expanded = vec![fmt.clone()];
    for (idx, arg) in args.iter().skip(1).enumerate() {
        let placeholder = format!("{{{idx}}}");
        let mut vals = Vec::new();
        eval_expr_labels(arg, matrix, false, &mut vals);
        if vals.is_empty() {
            vals.push(String::new());
        }
        let mut next = Vec::new();
        for cur in &expanded {
            for val in &vals {
                if next.len() < 64 {
                    next.push(cur.replace(&placeholder, val));
                }
            }
        }
        expanded = next;
    }
    for item in expanded {
        push_label_value(&item, in_json, out);
    }
}

fn eval_expr_labels(expr: &Expr, matrix: Option<&Matrix>, in_json: bool, out: &mut Vec<String>) {
    match expr {
        Expr::Or(a, b) => {
            eval_expr_labels(a, matrix, in_json, out);
            eval_expr_labels(b, matrix, in_json, out);
        }
        Expr::BoolOp | Expr::Other => {}
        Expr::Str(s) => push_label_value(s, in_json, out),
        Expr::MatrixAxis(axis) => {
            if let Some(m) = matrix {
                eval_matrix_axis(m, axis, in_json, out);
            }
        }
        Expr::FromJson(inner) => eval_expr_labels(inner, matrix, true, out),
        Expr::Call { name, args } if name.eq_ignore_ascii_case("format") => {
            eval_format_call(args, matrix, in_json, out);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                eval_expr_labels(arg, matrix, in_json, out);
            }
        }
    }
}

fn eval_template(trimmed: &str, matrix: Option<&Matrix>, out: &mut Vec<String>) {
    let mut parts: Vec<Vec<String>> = Vec::new();
    let mut rest = trimmed;
    while let Some((before, after_open)) = rest.split_once("${{") {
        if !before.is_empty() {
            parts.push(vec![before.to_string()]);
        }
        let Some((inner, after_close)) = after_open.split_once("}}") else {
            break;
        };
        let mut vals = Vec::new();
        if let Some(ast) = parse_expr(inner) {
            eval_expr_labels(&ast, matrix, false, &mut vals);
        }
        if vals.is_empty() {
            vals.push(String::new());
        }
        parts.push(vals);
        rest = after_close;
    }
    if !rest.is_empty() {
        parts.push(vec![rest.to_string()]);
    }
    let mut acc = vec![String::new()];
    for choices in parts {
        let mut next = Vec::new();
        for prefix in &acc {
            for choice in &choices {
                if next.len() < 64 {
                    next.push(format!("{prefix}{choice}"));
                }
            }
        }
        acc = next;
    }
    out.extend(acc);
}

fn eval_entry(entry: &str, matrix: Option<&Matrix>, out: &mut Vec<String>) {
    let trimmed = entry.trim();
    if !trimmed.contains("${{") {
        out.push(trimmed.to_string());
        return;
    }
    if let Some(inner) = trimmed
        .strip_prefix("${{")
        .and_then(|rest| rest.strip_suffix("}}"))
        .filter(|inner| !inner.contains("}}"))
    {
        if let Some(ast) = parse_expr(inner) {
            eval_expr_labels(&ast, matrix, false, out);
        }
        return;
    }
    eval_template(trimmed, matrix, out);
}

impl RunsOn {
    /// Returns the concrete runner labels this `runs-on` entry can evaluate to
    /// under `matrix`, ignoring predicate-only expressions and quoted literals
    /// inside comparisons or the left-hand side of `&&`.
    pub(crate) fn possible_labels(&self, matrix: Option<&Matrix>) -> Vec<String> {
        let mut out = Vec::new();
        match self {
            Self::Single(entry) => eval_entry(entry, matrix, &mut out),
            Self::Labels(labels) => {
                for entry in labels {
                    eval_entry(entry, matrix, &mut out);
                }
            }
        }
        out
    }
}
