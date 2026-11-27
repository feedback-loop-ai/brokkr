//! Strict minimal JSON5 reader for Renovate configuration guards (#1417,
//! decision 0071 rulings 3 and 8).
//!
//! Reads the JSON5 subset used by `.github/renovate.json5` and
//! `.github/renovate-global.json5`: objects (with unquoted identifier keys or
//! quoted keys, trailing commas, and duplicate-key rejection), arrays (with
//! trailing commas), single- and double-quoted strings with standard escapes,
//! booleans (`true`/`false`), and `//` line comments. Any token or syntax
//! outside this subset is refused with a typed [`Json5Error`].

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
            while let Some(b) = self.bytes.get(self.pos) {
                if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
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
            if matches!(ch, '\n' | '\r') {
                return Err(Json5Error::UnescapedNewline { offset });
            }
            if ch == '\\' {
                let esc_offset = self.pos;
                let esc = self.next_char()?;
                match esc {
                    '\\' => out.push('\\'),
                    '\'' => out.push('\''),
                    '"' => out.push('"'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    other => {
                        return Err(Json5Error::UnsupportedEscape {
                            offset: esc_offset,
                            escape: other,
                        });
                    }
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
    fn json5_refuses_unknown_syntax_duplicate_keys_and_trailing_garbage() {
        assert_eq!(
            parse_json5("{ a: true, a: false }"),
            Err(Json5Error::DuplicateKey {
                offset: 11,
                key: "a".to_string(),
            })
        );
        assert_eq!(
            parse_json5("{ a: 42 }"),
            Err(Json5Error::UnexpectedChar {
                offset: 5,
                found: '4',
            })
        );
        assert_eq!(
            parse_json5("{ a: null }"),
            Err(Json5Error::UnsupportedToken {
                offset: 5,
                token: "null".to_string(),
            })
        );
        assert_eq!(
            parse_json5("{ a: '\\u0041' }"),
            Err(Json5Error::UnsupportedEscape {
                offset: 7,
                escape: 'u',
            })
        );
        assert_eq!(
            parse_json5("{ a: true } extra"),
            Err(Json5Error::TrailingContent { offset: 12 })
        );
        assert_eq!(
            parse_json5("/* block */ {}"),
            Err(Json5Error::UnsupportedToken {
                offset: 0,
                token: "/*".to_string(),
            })
        );
    }
}
