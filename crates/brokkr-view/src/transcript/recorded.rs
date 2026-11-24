const DATE_TIME: &[u8] = b"####-##-##T##:##:##";
const OFFSET: &[u8] = b"##:##";

/// Check each byte of the recorded RFC 3339 date-time shape, without
/// validating calendar or clock ranges.
pub(super) fn timestamp(word: &str) -> bool {
    if word.len() > 35 {
        return false;
    }
    let Some((date_time, rest)) = word.as_bytes().split_at_checked(DATE_TIME.len()) else {
        return false;
    };
    if !matches_shape(date_time, DATE_TIME) {
        return false;
    }
    let zone = if let Some(fraction) = rest.strip_prefix(b".") {
        let digits = fraction
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if !(1..=9).contains(&digits) {
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

/// A role outside the built-in vocabulary can preserve one short ASCII word.
pub(super) fn role_word(word: &str) -> bool {
    (1..=32).contains(&word.len())
        && word
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(test)]
mod tests;
