//! Recorded metadata for Claude and Codex and the projected role vocabulary.

/// Who speaks a turn. A no-turn outcome from `RowOutcome::nothing` has an empty role.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Role {
    User,
    Assistant,
    Tool,
    Developer,
    System,
    InvalidRecorded,
    NoTurn,
}

impl Role {
    /// Classify a recorded role using the harness vocabulary.
    pub(super) fn recorded(word: &str) -> Role {
        match word {
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            "developer" => Role::Developer,
            "system" => Role::System,
            _ => Role::InvalidRecorded,
        }
    }

    /// The spelling every surface shows.
    pub(super) fn into_wire(self) -> String {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
            Role::Developer => "developer",
            Role::System => "system",
            Role::InvalidRecorded => "unknown",
            Role::NoTurn => "",
        }
        .to_string()
    }
}

const DATE_TIME: &[u8] = b"####-##-##T##:##:##";
const OFFSET: &[u8] = b"##:##";
const MAX_FRACTION_DIGITS: usize = 9;

/// Check each byte of the recorded RFC 3339 date-time shape, without
/// validating calendar or clock ranges.
pub(super) fn timestamp(word: &str) -> bool {
    let Some((date_time, rest)) = word.as_bytes().split_at_checked(DATE_TIME.len()) else {
        return false;
    };
    if !matches_shape(date_time, DATE_TIME) {
        return false;
    }
    let zone = if let Some(fraction) = rest.strip_prefix(b".") {
        let digits = fraction
            .iter()
            .take(MAX_FRACTION_DIGITS + 1)
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if !(1..=MAX_FRACTION_DIGITS).contains(&digits) {
            return false;
        }
        &fraction[digits..]
    } else {
        rest
    };
    match zone {
        b"Z" => true,
        [b'+' | b'-', offset @ ..] => matches_shape(offset, OFFSET),
        _ => false,
    }
}

fn matches_shape(bytes: &[u8], shape: &[u8]) -> bool {
    bytes.len() == shape.len()
        && bytes.iter().zip(shape).all(|(byte, class)| match class {
            b'#' => byte.is_ascii_digit(),
            literal => byte == literal,
        })
}

#[cfg(test)]
mod tests;
