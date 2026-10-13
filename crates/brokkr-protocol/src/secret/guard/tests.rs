//! The guard's number reader against the one it replaced (U6f1a's eighth
//! visit): the reader skips to the bytes that can begin a number and counts
//! each run of digits once, and must still read exactly the numbers, in
//! the order, that run 7's reader read.

use super::*;

/// Run 7's reader, kept as the reference the guard's reader must agree
/// with: it reads [`spelled`] from every byte it may begin a number at.
fn numbers(bytes: &[u8], quoted: bool) -> Vec<&[u8]> {
    let (mut found, mut at, mut text, mut escaped) = (Vec::new(), 0, false, false);
    while let Some(byte) = bytes.get(at) {
        let before = at.checked_sub(1).and_then(|before| bytes.get(before));
        let inside = !quoted & before.is_some_and(u8::is_ascii_digit);
        let length = match text | inside {
            true => 0,
            false => spelled(&bytes[at..]),
        };
        match (text, escaped, byte) {
            (true, true, _) => escaped = false,
            (true, false, b'\\') => escaped = true,
            (_, false, b'"') => text = quoted & !text,
            _ => {}
        }
        found.extend((length > 0).then(|| &bytes[at..at + length]));
        at += match quoted {
            true => length.max(1),
            false => 1,
        };
    }
    found
}

/// Run 7's count of how many of `bytes`' first bytes write a number.
fn spelled(bytes: &[u8]) -> usize {
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let signed = usize::from(matches!(bytes.first(), Some(b'-' | b'+')));
    let whole = digits(signed);
    let pointed = usize::from(bytes.get(signed + whole) == Some(&b'.'));
    let fraction = digits(signed + whole + pointed);
    let mantissa = signed + whole + pointed + fraction;
    let letter = bytes
        .get(mantissa)
        .is_some_and(|b| b.eq_ignore_ascii_case(&b'e'));
    let sign = letter & matches!(bytes.get(mantissa + 1), Some(b'-' | b'+'));
    let power = match letter {
        true => digits(mantissa + 1 + usize::from(sign)),
        false => 0,
    };
    match (whole + fraction, power) {
        (0, _) => 0,
        (_, 0) => mantissa,
        (_, power) => mantissa + 1 + usize::from(sign) + power,
    }
}

/// What a reading yields, a number as written or `None` for a stretch.
fn yielded(reading: Reading<'_>) -> Option<&[u8]> {
    match reading {
        Reading::Number(number) => Some(number),
        Reading::Stretch => None,
    }
}

/// That the guard's reader reads `bytes` as the reference does, in both
/// modes, and that the guard's readings of them are the reference's two:
/// as written, then decoded, the second read once where it is the first.
fn agrees(bytes: &[u8]) {
    for quoted in [true, false] {
        let read: Vec<_> = Numbers::new(bytes, quoted).map(yielded).collect();
        let reference: Vec<_> = numbers(bytes, quoted).into_iter().map(Some).collect();
        assert_eq!((bytes, quoted, read), (bytes, quoted, reference));
    }
    let decoded = unescaped(bytes);
    let escaped = matches!(decoded, Cow::Owned(_));
    assert_eq!((bytes, escaped), (bytes, bytes.contains(&b'\\')));
    for json in [true, false] {
        let mut reference = numbers(bytes, json);
        let spelled = numbers(&decoded, false);
        match json | escaped {
            true => reference.extend(spelled),
            false => assert_eq!(spelled, reference),
        }
        let reference: Vec<_> = reference.into_iter().map(Some).collect();
        let read: Vec<_> = readings(bytes, &decoded, escaped, json)
            .map(yielded)
            .collect();
        assert_eq!((bytes, json, read), (bytes, json, reference));
    }
}

#[test]
fn the_reader_reads_every_number_the_reference_reads_in_order() {
    let cases: &[&[u8]] = &[
        b"",
        b"1",
        b"-",
        b"+",
        b".",
        b"e",
        b"\\",
        b"\"",
        b"a",
        b"-5",
        b"+5",
        b"--5",
        b"-+5",
        b".5",
        b"-.5",
        b"5.",
        b"1.5",
        b"1.5.3",
        b"..5",
        b"1e5",
        b"1E5",
        b"1e+5",
        b"1e-5",
        b"-1.5e-3",
        b"1e",
        b"1e+",
        b"1e-x",
        b"e5",
        b"1ee5",
        b"1e5e6",
        b"1e5.3",
        b"1.e5",
        b".e5",
        b"abc0876543210def",
        b"x1.5y",
        b"a-1b",
        b"12-3",
        b"1-2+3",
        b"[\"a1\",2,-3.5e+2]",
        b"{\"k3\":4,\"8.7654321e8\":\"876543210\"}",
        b"\"\\\"5\"6",
        b"\"a\\\\\"7",
        b"\"\\u0031\\u00352\"",
        b"\"\\ud83d\\ude00 9\"",
        b"\"\\ud83d 1\\ude00\"",
        b"\"\\n12\\t3\"",
        b"5\\",
        b"\"1\\",
    ];
    for bytes in cases {
        agrees(bytes);
    }
}

#[test]
fn the_reader_agrees_with_the_reference_on_generated_inputs() {
    // A fixed-seed xorshift over pieces that can begin, continue, quote or
    // escape a number, each input up to 24 of them.
    let pieces: [&[u8]; 24] = [
        b"0", b"1", b"5", b"9", b"+", b"-", b".", b"e", b"E", b"\"", b"\\", b" ", b"u", b"a", b"Z",
        b"x", b"7", b"0", b"\\u0035", b"\\ud83d", b"\\ude00", b"\\n", b"\\\"", b"3",
    ];
    let mut state: u64 = 0x2026_1013_0065_6f1a;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..4000 {
        let length = next() % 25;
        let bytes: Vec<u8> = (0..length)
            .flat_map(|_| pieces[(next() % 24) as usize].iter().copied())
            .collect();
        agrees(&bytes);
    }
}

#[test]
fn a_long_reading_is_marked_every_stretch_and_its_numbers_read_whole() {
    // Letters past three stretches are marked three times.
    let letters = [&b"\"1 "[..], &vec![b'a'; 3 * STRETCH], b"\" 2"].concat();
    for quoted in [true, false] {
        let read: Vec<_> = Numbers::new(&letters, quoted).map(yielded).collect();
        let first = (!quoted).then_some(&b"1"[..]);
        let expected: Vec<_> = first
            .into_iter()
            .map(Some)
            .chain([None, None, None, Some(&b"2"[..])])
            .collect();
        assert_eq!((quoted, read), (quoted, expected));
    }
    // A run of digits as long is marked within it, at each stretch it
    // passes before its end, and is then one number, read whole.
    let digits = [vec![b'7'; 3 * STRETCH], b" 2".to_vec()].concat();
    for quoted in [true, false] {
        let read: Vec<_> = Numbers::new(&digits, quoted).map(yielded).collect();
        let run = &digits[..3 * STRETCH];
        let expected = vec![None, None, Some(run), None, Some(&b"2"[..])];
        assert_eq!((quoted, read), (quoted, expected));
    }
    // So is an exponent's run, the count going on where it stopped; where
    // not quoted, the digits after the letter are a number of their own.
    let power = [&b"1e"[..], &vec![b'7'; 3 * STRETCH]].concat();
    for quoted in [true, false] {
        let read: Vec<_> = Numbers::new(&power, quoted).map(yielded).collect();
        let mut expected = vec![None, None, None, Some(&power[..])];
        expected.extend((!quoted).then_some(Some(&power[2..])));
        assert_eq!((quoted, read), (quoted, expected));
    }
}
