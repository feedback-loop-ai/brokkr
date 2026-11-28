//! Structured shell-command tokenizer for workflow `run:` scripts (#1417,
//! decision 0071 rulings 3 and 5).
//!
//! Splits shell scripts into executed simple commands (as unquoted word lists),
//! stripping `#` comments, joining `\\\n` continuations, entering `case` arms,
//! and recursing into `$(...)` command substitutions so guards never match
//! workflow scripts by raw substring.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaseState {
    Header,
    Pattern,
    Body,
}

#[derive(Debug, Default)]
struct CaseFrame {
    state: Option<CaseState>,
    patterns: Vec<String>,
    body: Vec<Vec<String>>,
}

struct ShellLexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    cur_word: String,
    in_word: bool,
    cur_cmd: Vec<String>,
    commands: Vec<Vec<String>>,
    case_stack: Vec<CaseFrame>,
    arms: BTreeMap<String, Vec<Vec<String>>>,
}

fn is_control_keyword(word: &str) -> bool {
    matches!(
        word,
        "if" | "then" | "elif" | "else" | "fi" | "do" | "done" | "while" | "until" | "!"
    )
}

fn is_env_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn normalize_command(words: Vec<String>) -> Option<Vec<String>> {
    let mut idx = 0;
    while idx < words.len() && is_control_keyword(&words[idx]) {
        idx += 1;
    }
    while idx < words.len() && is_env_assignment(&words[idx]) {
        idx += 1;
    }
    (idx < words.len()).then(|| words[idx..].to_vec())
}

impl<'a> ShellLexer<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            cur_word: String::new(),
            in_word: false,
            cur_cmd: Vec::new(),
            commands: Vec::new(),
            case_stack: Vec::new(),
            arms: BTreeMap::new(),
        }
    }

    fn top_case_state(&self) -> Option<CaseState> {
        self.case_stack.last().and_then(|f| f.state)
    }

    fn finish_word(&mut self) {
        if !self.in_word {
            return;
        }
        let word = std::mem::take(&mut self.cur_word);
        self.in_word = false;
        match self.top_case_state() {
            Some(CaseState::Header) => {
                if word == "in" {
                    if let Some(frame) = self.case_stack.last_mut() {
                        frame.state = Some(CaseState::Pattern);
                    }
                }
                return;
            }
            Some(CaseState::Pattern) => {
                if word == "esac" {
                    self.case_stack.pop();
                } else if let Some(frame) = self.case_stack.last_mut() {
                    frame.patterns.push(word);
                }
                return;
            }
            Some(CaseState::Body) if self.cur_cmd.is_empty() && word == "esac" => {
                self.record_case_arm();
                self.case_stack.pop();
                return;
            }
            _ => {}
        }
        if self.cur_cmd.is_empty() && word == "case" {
            self.case_stack.push(CaseFrame {
                state: Some(CaseState::Header),
                patterns: Vec::new(),
                body: Vec::new(),
            });
            return;
        }
        self.cur_cmd.push(word);
    }

    fn record_case_arm(&mut self) {
        let Some(frame) = self.case_stack.last_mut() else {
            return;
        };
        let patterns = std::mem::take(&mut frame.patterns);
        let body = std::mem::take(&mut frame.body);
        if body.is_empty() {
            return;
        }
        for pat in patterns {
            if pat != "*" {
                self.arms.entry(pat).or_default().extend(body.clone());
            }
        }
    }

    fn finish_cmd(&mut self) {
        self.finish_word();
        let raw = std::mem::take(&mut self.cur_cmd);
        let Some(cmd) = normalize_command(raw) else {
            return;
        };
        if let Some(frame) = self.case_stack.last_mut() {
            if frame.state == Some(CaseState::Body) {
                frame.body.push(cmd.clone());
            }
        }
        self.commands.push(cmd);
    }

    fn push_char(&mut self) {
        self.in_word = true;
        let ch = self.src[self.pos..].chars().next().unwrap();
        self.cur_word.push(ch);
        self.pos += ch.len_utf8();
    }

    fn read_single_quoted(&mut self) {
        self.pos += 1;
        self.in_word = true;
        while self.pos < self.bytes.len() {
            if self.bytes[self.pos] == b'\'' {
                self.pos += 1;
                return;
            }
            self.push_char();
        }
    }

    fn read_cmd_subst(&mut self) {
        self.pos += 2; // skip `$(`
        let start = self.pos;
        let mut depth = 1usize;
        let mut in_single = false;
        let mut in_double = false;
        while self.pos < self.bytes.len() && depth > 0 {
            let b = self.bytes[self.pos];
            if b == b'\\' && !in_single && self.pos + 1 < self.bytes.len() {
                self.pos += 2;
                continue;
            }
            if b == b'\'' && !in_double {
                in_single = !in_single;
            } else if b == b'"' && !in_single {
                in_double = !in_double;
            } else if !in_single && !in_double && b == b'(' {
                depth += 1;
            } else if !in_single && !in_double && b == b')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            self.pos += 1;
        }
        let inner = &self.src[start..self.pos];
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b')' {
            self.pos += 1;
        }
        self.in_word = true;
        self.cur_word.push_str("$(");
        self.cur_word.push_str(inner);
        self.cur_word.push(')');
        let mut nested = ShellLexer::new(inner);
        nested.run();
        self.commands.extend(nested.commands);
    }

    fn read_double_quoted(&mut self) {
        self.pos += 1;
        self.in_word = true;
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'"' {
                self.pos += 1;
                return;
            }
            if b == b'\\' && self.pos + 1 < self.bytes.len() {
                if self.bytes[self.pos + 1] == b'\n' {
                    self.pos += 2;
                    continue;
                }
                let next = self.src[self.pos + 1..].chars().next().unwrap();
                self.cur_word.push(next);
                self.pos += 1 + next.len_utf8();
                continue;
            }
            if b == b'$' && self.bytes.get(self.pos + 1) == Some(&b'(') {
                self.read_cmd_subst();
                continue;
            }
            self.push_char();
        }
    }

    fn skip_redirect(&mut self) {
        if self.in_word && self.cur_word.bytes().all(|b| b.is_ascii_digit()) {
            self.cur_word.clear();
            self.in_word = false;
        } else {
            self.finish_word();
        }
        let b = self.bytes[self.pos];
        self.pos += 1;
        if self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b'>' | b'<' | b'&') {
            self.pos += 1;
        }
        if b == b'>' && self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
    }

    fn handle_separator(&mut self) {
        let two = self.bytes.get(self.pos..self.pos + 2);
        if two == Some(b";;") {
            self.finish_cmd();
            if self.top_case_state() == Some(CaseState::Body) {
                self.record_case_arm();
                if let Some(frame) = self.case_stack.last_mut() {
                    frame.state = Some(CaseState::Pattern);
                }
            }
            self.pos += 2;
            return;
        }
        if matches!(two, Some(b"&&" | b"||")) {
            self.finish_cmd();
            self.pos += 2;
            return;
        }
        let b = self.bytes[self.pos];
        if b == b')' && self.top_case_state() == Some(CaseState::Pattern) {
            self.finish_word();
            if let Some(frame) = self.case_stack.last_mut() {
                frame.state = Some(CaseState::Body);
            }
            self.pos += 1;
            return;
        }
        if b == b'|' && self.top_case_state() == Some(CaseState::Pattern) {
            self.finish_word();
            self.pos += 1;
            return;
        }
        self.finish_cmd();
        self.pos += 1;
    }

    fn run(&mut self) {
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            match b {
                b' ' | b'\t' | b'\r' => {
                    self.finish_word();
                    self.pos += 1;
                }
                b'#' if !self.in_word => {
                    while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                }
                b'\\' if self.bytes.get(self.pos + 1) == Some(&b'\n') => {
                    self.pos += 2;
                }
                b'\\' if self.pos + 1 < self.bytes.len() => {
                    self.in_word = true;
                    let next = self.src[self.pos + 1..].chars().next().unwrap();
                    self.cur_word.push(next);
                    self.pos += 1 + next.len_utf8();
                }
                b'\'' => self.read_single_quoted(),
                b'"' => self.read_double_quoted(),
                b'$' if self.bytes.get(self.pos + 1) == Some(&b'(') => self.read_cmd_subst(),
                b'>' | b'<' => self.skip_redirect(),
                b'\n' | b';' | b'&' | b'|' | b'(' | b')' | b'{' | b'}' => self.handle_separator(),
                _ => self.push_char(),
            }
        }
        self.finish_cmd();
    }
}

/// Returns every simple command executed by `script` as unquoted shell words,
/// skipping `#` comments and expanding `case` arms and `$(...)` substitutions.
pub(crate) fn commands(script: &str) -> Vec<Vec<String>> {
    let mut lexer = ShellLexer::new(script);
    lexer.run();
    lexer.commands
}

/// Maps each non-wildcard `case` arm pattern in `script` to the simple commands
/// executed inside that arm.
pub(crate) fn case_arms(script: &str) -> BTreeMap<String, Vec<Vec<String>>> {
    let mut lexer = ShellLexer::new(script);
    lexer.run();
    lexer.arms
}

/// Extracts simple command word lists from `var=(...)` bash array assignments
/// in non-comment lines of `script`.
pub(crate) fn bash_array_words(script: &str, var: &str) -> Vec<Vec<String>> {
    let prefix = format!("{var}=(");
    script
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.strip_prefix(&prefix)?.strip_suffix(')'))
        .flat_map(commands)
        .collect()
}

/// Unwraps leading command wrappers (`sudo`, `env`, `scripts/run-it-tests.sh`)
/// so callers inspect the executed command directly.
pub(crate) fn unwrap_command(mut words: &[String]) -> &[String] {
    loop {
        match words.first().map(String::as_str) {
            Some("sudo") => {
                let mut i = 1;
                while i < words.len() && words[i].starts_with('-') {
                    i += 1;
                }
                words = &words[i..];
            }
            Some("env") => {
                let mut i = 1;
                while i < words.len() && (words[i].starts_with('-') || is_env_assignment(&words[i]))
                {
                    i += 1;
                }
                words = &words[i..];
            }
            Some("scripts/run-it-tests.sh" | "./scripts/run-it-tests.sh") => {
                words = &words[1..];
            }
            _ => return words,
        }
    }
}

/// Returns the script path invoked directly or via `bash`/`sh`, stripped of any
/// leading `./`.
pub(crate) fn invoked_script(words: &[String]) -> Option<&str> {
    let raw = match words.first().map(String::as_str)? {
        "bash" | "sh" => {
            let mut i = 1;
            while i < words.len() && words[i].starts_with('-') {
                i += 1;
            }
            words.get(i)?.as_str()
        }
        other => other,
    };
    Some(raw.strip_prefix("./").unwrap_or(raw))
}

/// For a `cargo [+toolchain] <subcommand> ...` invocation, returns
/// `(subcommand, remaining_args)`.
pub(crate) fn cargo_subcommand(words: &[String]) -> Option<(&str, &[String])> {
    if words.first().map(String::as_str) != Some("cargo") {
        return None;
    }
    let idx = if words.get(1).is_some_and(|w| w.starts_with('+')) {
        2
    } else {
        1
    };
    let sub = words.get(idx)?.as_str();
    Some((sub, &words[idx + 1..]))
}
