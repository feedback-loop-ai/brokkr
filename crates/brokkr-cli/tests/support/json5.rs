//! Strict closed-form JSON5 reader for Renovate configuration guards (#1417,
//! decision 0071 rulings 3 and 8).
//!
//! Reads only the narrow ASCII subset used by `.github/renovate.json5` and
//! `.github/renovate-global.json5`: printable ASCII (`0x20..=0x7e`) and LF
//! (`0x0a`) only (never CR, tabs, other control bytes, or non-ASCII), objects
//! (with unquoted ASCII identifier keys or quoted keys, trailing commas, and
//! duplicate-key rejection), arrays (with trailing commas), single- and
//! double-quoted strings with `\\` as the only escape sequence, booleans
//! (`true`/`false`), and `//` line comments terminated by LF or EOF. Any byte
//! or construct outside this subset is refused with a typed [`Json5Error`]
//! naming the byte offset, so it reads less than JSON5 and never differently.

use std::collections::BTreeMap;

/// Parsed JSON5 value for the configuration subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Json5Value {
    Object(BTreeMap<String, Json5Value>),
    Array(Vec<Json5Value>),
    String(String),
    Bool(bool),
}

impl Json5Value {
    pub(crate) fn as_object(&self) -> Option<&BTreeMap<String, Json5Value>> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }

    pub(crate) fn as_array(&self) -> Option<&[Json5Value]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub(crate) fn as_string_list(&self) -> Option<Vec<&str>> {
        self.as_array()?.iter().map(Self::as_str).collect()
    }
}

/// Typed refusal returned by [`parse_json5`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Json5Error {
    #[error("offset {offset}: carriage return is not allowed")]
    CarriageReturn { offset: usize },
    #[error("offset {offset}: byte 0x{byte:02x} outside printable ASCII and LF")]
    DisallowedByte { offset: usize, byte: u8 },
    #[error("offset {offset}: unexpected end of input")]
    UnexpectedEof { offset: usize },
    #[error("offset {offset}: unexpected character `{found}`")]
    UnexpectedChar { offset: usize, found: char },
    #[error("offset {offset}: expected `{expected}`, found `{found}`")]
    ExpectedChar {
        offset: usize,
        expected: char,
        found: char,
    },
    #[error("offset {offset}: unsupported escape `\\{escape}`")]
    UnsupportedEscape { offset: usize, escape: char },
    #[error("offset {offset}: unescaped newline inside string")]
    UnescapedNewline { offset: usize },
    #[error("offset {offset}: duplicate object key `{key}`")]
    DuplicateKey { offset: usize, key: String },
    #[error("offset {offset}: unsupported token `{token}`")]
    UnsupportedToken { offset: usize, token: String },
    #[error("offset {offset}: trailing content after root value")]
    TrailingContent { offset: usize },
}

/// Parse one JSON5 document strictly, refusing unknown syntax and trailing tokens.
pub(crate) fn parse_json5(input: &str) -> Result<Json5Value, Json5Error> {
    for (offset, &byte) in input.as_bytes().iter().enumerate() {
        if byte == b'\r' {
            return Err(Json5Error::CarriageReturn { offset });
        }
        if byte != b'\n' && !(0x20..=0x7e).contains(&byte) {
            return Err(Json5Error::DisallowedByte { offset, byte });
        }
    }
    let mut parser = Parser {
        bytes: input.as_bytes(),
        src: input,
        pos: 0,
    };
    parser.skip_ws_and_comments()?;
    let value = parser.parse_value()?;
    parser.skip_ws_and_comments()?;
    if parser.pos != parser.bytes.len() {
        return Err(Json5Error::TrailingContent { offset: parser.pos });
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    src: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn next_char(&mut self) -> Result<char, Json5Error> {
        let ch = self
            .peek_char()
            .ok_or(Json5Error::UnexpectedEof { offset: self.pos })?;
        self.pos += ch.len_utf8();
        Ok(ch)
    }

    fn skip_ws_and_comments(&mut self) -> Result<(), Json5Error> {
        loop {
            while let Some(&b) = self.bytes.get(self.pos) {
                if matches!(b, b' ' | b'\n') {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if self.src[self.pos..].starts_with("//") {
                self.pos += 2;
                let rest = &self.bytes[self.pos..];
                let step = rest
                    .iter()
                    .position(|&b| b == b'\n')
                    .map_or(rest.len(), |i| i + 1);
                self.pos += step;
                continue;
            }
            if self.src[self.pos..].starts_with("/*") {
                return Err(Json5Error::UnsupportedToken {
                    offset: self.pos,
                    token: "/*".to_string(),
                });
            }
            return Ok(());
        }
    }

    fn parse_value(&mut self) -> Result<Json5Value, Json5Error> {
        self.skip_ws_and_comments()?;
        let ch = self
            .peek_char()
            .ok_or(Json5Error::UnexpectedEof { offset: self.pos })?;
        match ch {
            '{' => self.parse_object(),
            '[' => self.parse_array(),
            '\'' | '"' => self.parse_string().map(Json5Value::String),
            _ if is_ident_start(ch) => self.parse_bool(),
            _ => Err(Json5Error::UnexpectedChar {
                offset: self.pos,
                found: ch,
            }),
        }
    }

    fn parse_object(&mut self) -> Result<Json5Value, Json5Error> {
        self.expect_char('{')?;
        let mut map = BTreeMap::new();
        loop {
            self.skip_ws_and_comments()?;
            if self.peek_char() == Some('}') {
                self.pos += 1;
                return Ok(Json5Value::Object(map));
            }
            let key_offset = self.pos;
            let key = self.parse_key()?;
            self.skip_ws_and_comments()?;
            self.expect_char(':')?;
            let val = self.parse_value()?;
            if map.insert(key.clone(), val).is_some() {
                return Err(Json5Error::DuplicateKey {
                    offset: key_offset,
                    key,
                });
            }
            self.skip_ws_and_comments()?;
            match self.peek_char() {
                Some(',') => {
                    self.pos += 1;
                }
                Some('}') => {
                    self.pos += 1;
                    return Ok(Json5Value::Object(map));
                }
                Some(found) => {
                    return Err(Json5Error::ExpectedChar {
                        offset: self.pos,
                        expected: '}',
                        found,
                    });
                }
                None => return Err(Json5Error::UnexpectedEof { offset: self.pos }),
            }
        }
    }

    fn parse_array(&mut self) -> Result<Json5Value, Json5Error> {
        self.expect_char('[')?;
        let mut items = Vec::new();
        loop {
            self.skip_ws_and_comments()?;
            if self.peek_char() == Some(']') {
                self.pos += 1;
                return Ok(Json5Value::Array(items));
            }
            items.push(self.parse_value()?);
            self.skip_ws_and_comments()?;
            match self.peek_char() {
                Some(',') => {
                    self.pos += 1;
                }
                Some(']') => {
                    self.pos += 1;
                    return Ok(Json5Value::Array(items));
                }
                Some(found) => {
                    return Err(Json5Error::ExpectedChar {
                        offset: self.pos,
                        expected: ']',
                        found,
                    });
                }
                None => return Err(Json5Error::UnexpectedEof { offset: self.pos }),
            }
        }
    }

    fn parse_key(&mut self) -> Result<String, Json5Error> {
        let ch = self
            .peek_char()
            .ok_or(Json5Error::UnexpectedEof { offset: self.pos })?;
        if matches!(ch, '\'' | '"') {
            return self.parse_string();
        }
        if !is_ident_start(ch) {
            return Err(Json5Error::UnexpectedChar {
                offset: self.pos,
                found: ch,
            });
        }
        Ok(self.take_ident().to_string())
    }

    fn parse_bool(&mut self) -> Result<Json5Value, Json5Error> {
        let start = self.pos;
        let word = self.take_ident();
        match word {
            "true" => Ok(Json5Value::Bool(true)),
            "false" => Ok(Json5Value::Bool(false)),
            other => Err(Json5Error::UnsupportedToken {
                offset: start,
                token: other.to_string(),
            }),
        }
    }

    fn take_ident(&mut self) -> &'a str {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if is_ident_continue(ch) {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        &self.src[start..self.pos]
    }

    fn parse_string(&mut self) -> Result<String, Json5Error> {
        let quote = self.next_char()?;
        let mut out = String::new();
        loop {
            let offset = self.pos;
            let ch = self.next_char()?;
            if ch == quote {
                return Ok(out);
            }
            if ch == '\n' {
                return Err(Json5Error::UnescapedNewline { offset });
            }
            if ch == '\\' {
                let esc = self.next_char()?;
                if esc == '\\' {
                    out.push('\\');
                } else {
                    return Err(Json5Error::UnsupportedEscape {
                        offset,
                        escape: esc,
                    });
                }
            } else {
                out.push(ch);
            }
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), Json5Error> {
        let offset = self.pos;
        let found = self.next_char()?;
        if found != expected {
            return Err(Json5Error::ExpectedChar {
                offset,
                expected,
                found,
            });
        }
        Ok(())
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || matches!(ch, '_' | '$')
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json5_parses_comments_unquoted_keys_single_quotes_and_trailing_commas() {
        let doc = r#"
        // header comment
        {
          $schema: 'https://example.invalid/schema.json',
          enabled: true,
          items: [
            "it's",
            'a\\b',
          ],
        }
        "#;
        let parsed = parse_json5(doc).expect("valid json5 subset");
        let obj = parsed.as_object().expect("root object");
        assert_eq!(
            obj.get("$schema").and_then(Json5Value::as_str),
            Some("https://example.invalid/schema.json")
        );
        assert_eq!(obj.get("enabled"), Some(&Json5Value::Bool(true)));
        assert_eq!(
            obj.get("items").and_then(Json5Value::as_string_list),
            Some(vec!["it's", r"a\b"])
        );
    }

    #[test]
    fn json5_refuses_carriage_return_anywhere() {
        let comment_cr = "{ allowedCommands: [\n// hide\r'evil',\n] }";
        let cr_offset = comment_cr.find('\r').expect("cr");
        assert_eq!(
            parse_json5(comment_cr),
            Err(Json5Error::CarriageReturn { offset: cr_offset })
        );
        assert_eq!(
            parse_json5("{\r\n a: true\r\n}"),
            Err(Json5Error::CarriageReturn { offset: 1 })
        );
        assert_eq!(
            parse_json5("{ a: 'x\ry' }"),
            Err(Json5Error::CarriageReturn { offset: 7 })
        );
    }

    #[test]
    fn json5_refuses_bytes_outside_printable_ascii_and_lf() {
        for (input, offset, byte) in [
            ("{\ta: true}", 1, b'\t'),
            ("{ a: '\0' }", 6, 0x00),
            ("{ a: '\x7f' }", 6, 0x7f),
            ("// ls\u{2028}'evil'\n{}", 5, 0xe2),
            ("{ a: 'hello\u{00e9}' }", 11, 0xc3),
            ("\u{feff}{}", 0, 0xef),
        ] {
            assert_eq!(
                parse_json5(input),
                Err(Json5Error::DisallowedByte { offset, byte }),
                "input={input:?}"
            );
        }
    }

    #[test]
    fn json5_refuses_unsupported_backslash_escapes_in_string() {
        for (input, offset, escape) in [
            ("{ a: '\\u0041' }", 6, 'u'),
            ("{ a: '\\n' }", 6, 'n'),
            ("{ a: '\\r' }", 6, 'r'),
            ("{ a: '\\t' }", 6, 't'),
            ("{ a: '\\'' }", 6, '\''),
            ("{ a: '\\\"' }", 6, '"'),
            ("{ a: '\\\n' }", 6, '\n'),
        ] {
            assert_eq!(
                parse_json5(input),
                Err(Json5Error::UnsupportedEscape { offset, escape }),
                "input={input:?}"
            );
        }
    }

    #[test]
    fn json5_refuses_block_comments() {
        assert_eq!(
            parse_json5("/* block */ {}"),
            Err(Json5Error::UnsupportedToken {
                offset: 0,
                token: "/*".to_string(),
            })
        );
        assert_eq!(
            parse_json5("{ /* inner */ a: true }"),
            Err(Json5Error::UnsupportedToken {
                offset: 2,
                token: "/*".to_string(),
            })
        );
    }

    #[test]
    fn json5_refuses_number_forms() {
        for (input, found) in [
            ("{ a: 42 }", '4'),
            ("{ a: -1 }", '-'),
            ("{ a: +1 }", '+'),
            ("{ a: .5 }", '.'),
            ("{ a: 0x10 }", '0'),
        ] {
            assert_eq!(
                parse_json5(input),
                Err(Json5Error::UnexpectedChar { offset: 5, found }),
                "input={input:?}"
            );
        }
        for (input, token) in [("{ a: Infinity }", "Infinity"), ("{ a: NaN }", "NaN")] {
            assert_eq!(
                parse_json5(input),
                Err(Json5Error::UnsupportedToken {
                    offset: 5,
                    token: token.to_string(),
                }),
                "input={input:?}"
            );
        }
    }

    #[test]
    fn json5_refuses_null_and_unknown_identifiers() {
        assert_eq!(
            parse_json5("{ a: null }"),
            Err(Json5Error::UnsupportedToken {
                offset: 5,
                token: "null".to_string(),
            })
        );
        assert_eq!(
            parse_json5("{ a: undefined }"),
            Err(Json5Error::UnsupportedToken {
                offset: 5,
                token: "undefined".to_string(),
            })
        );
    }

    #[test]
    fn json5_refuses_duplicate_object_keys() {
        assert_eq!(
            parse_json5("{ a: true, a: false }"),
            Err(Json5Error::DuplicateKey {
                offset: 11,
                key: "a".to_string(),
            })
        );
    }

    #[test]
    fn json5_refuses_unescaped_newline_in_string() {
        assert_eq!(
            parse_json5("{ a: 'line1\nline2' }"),
            Err(Json5Error::UnescapedNewline { offset: 11 })
        );
    }

    #[test]
    fn json5_refuses_trailing_content_after_root() {
        assert_eq!(
            parse_json5("{ a: true } extra"),
            Err(Json5Error::TrailingContent { offset: 12 })
        );
    }

    #[test]
    fn json5_refuses_unexpected_eof_and_bad_delimiters() {
        assert_eq!(
            parse_json5(""),
            Err(Json5Error::UnexpectedEof { offset: 0 })
        );
        assert_eq!(
            parse_json5("{ a: 'unterminated"),
            Err(Json5Error::UnexpectedEof { offset: 18 })
        );
        assert_eq!(
            parse_json5("{ a: true b: false }"),
            Err(Json5Error::ExpectedChar {
                offset: 10,
                expected: '}',
                found: 'b',
            })
        );
        assert_eq!(
            parse_json5("[true false]"),
            Err(Json5Error::ExpectedChar {
                offset: 6,
                expected: ']',
                found: 'f',
            })
        );
    }
}
