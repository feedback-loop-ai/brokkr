//! The final guard (decision 0065 slice two, U6f1a; MB4, CR2): whether the
//! very bytes about to leave the broker, on its output edge or into its
//! ledger, hold no bound secret. Decision 0012 keeps every read of a
//! secret's value in this module, so the one guard both of them pass is
//! here, beside the masker it shares; it hands back nothing derived from a
//! value but its answer, and never reads one through `expose_for_spawn`,
//! whose one call site stays the spawn injector.
//!
//! Three things hold of bytes that pass it, each read from those bytes and
//! nothing before them: no needle the shared masker knows, a bound value
//! or any of its shared encodings, occurs in them ([`Guard::holds`]); none
//! occurs in them with every JSON escape decoded ([`unescaped`]), so a
//! string's keys and values are seen as what they mean, `\u0000`, `\/` and
//! a surrogate pair included; and no number they write ([`Numbers`]) is a
//! bound value by what it is worth ([`Guard::valued`]). A needle is found
//! where it occurs, never by whether masking would change the bytes: a
//! binding whose value is its own placeholder masks to the very bytes it
//! was. A number is any written in the bytes, read as the text they decode
//! to, on every surface alike (operator ruling 2026-10-13): on a JSON
//! surface each written outside its strings, and every one its strings,
//! keys and values alike, spell; on a text surface every one written in
//! it, as it is and decoded, so a text that is itself a JSON string is read
//! as the text it is.

use std::borrow::Cow;
use std::convert::Infallible;

use super::{needles, wipe, BoundSecret, NEEDLE_ENCODINGS};

/// How many numbers the guard compares between two reads of its caller's
/// deadline and cancellation.
const BATCH: usize = 64;

/// How many bytes the guard reads for numbers between two reads of its
/// caller's deadline and cancellation, whatever numbers they write.
pub(super) const STRETCH: usize = 64 << 10;

/// The most significant digits of a number's exponent that
/// [`Guard::valued`] reads exactly, and of a binding's: a number's
/// exponent within 10^28 and its digits, at most `usize::MAX` of them,
/// place its value below 10^29, and a binding's past 10^30 places it above.
const NUMBER_POWER_DIGITS: usize = 28;
const BOUND_POWER_DIGITS: usize = 30;

/// One session's final guard: the bindings the masker reads, the needles
/// it replaces, and each binding that is itself a decimal number read
/// once, as its `Magnitude`, the last two wiped when the guard is dropped.
/// None is ever handed out, and the guard has no `Debug`.
pub struct Guard<'b> {
    bindings: &'b [BoundSecret],
    needles: Vec<Vec<u8>>,
    numbers: Vec<Magnitude>,
}

impl<'b> Guard<'b> {
    /// The guard of `bindings`, each read once.
    pub fn new(bindings: &'b [BoundSecret]) -> Guard<'b> {
        let read =
            |binding: &BoundSecret| Magnitude::read(&binding.secret.bytes, BOUND_POWER_DIGITS);
        let needles = needles(bindings, &NEEDLE_ENCODINGS);
        Guard {
            bindings,
            needles: needles.into_iter().map(|(needle, _)| needle).collect(),
            numbers: bindings.iter().filter_map(read).collect(),
        }
    }

    /// The bindings the guard reads, which the ledger masks by.
    pub(crate) fn bindings(&self) -> &'b [BoundSecret] {
        self.bindings
    }

    /// Whether the JSON text `bytes` passes the guard.
    pub fn json(&self, bytes: &[u8]) -> bool {
        let Ok(passed) = self.passes(bytes, true, never);
        passed
    }

    /// [`Guard::json`], `poll` read before every `BATCH` of the numbers
    /// compared: the first error it answers ends the scan, and is handed
    /// back, so the caller's deadline and cancellation hold within it.
    pub fn json_within<B>(
        &self,
        bytes: &[u8],
        poll: impl FnMut() -> Result<(), B>,
    ) -> Result<bool, B> {
        self.passes(bytes, true, poll)
    }

    /// Whether the text `bytes` passes the guard.
    pub fn text(&self, bytes: &[u8]) -> bool {
        let Ok(passed) = self.passes(bytes, false, never);
        passed
    }

    /// Whether `bytes`, JSON where `json` and else text, pass the guard;
    /// or `poll`'s error. Their numbers are read as JSON writes them where
    /// `json`, and, on every surface, as the text they decode to, so no
    /// number a string spells is read by its spelling alone. `poll` is read
    /// before every `BATCH` of the numbers compared and after every
    /// `STRETCH` of the bytes read, so neither a long reading nor many
    /// numbers outlast the caller's deadline or cancellation.
    fn passes<B>(
        &self,
        bytes: &[u8],
        json: bool,
        mut poll: impl FnMut() -> Result<(), B>,
    ) -> Result<bool, B> {
        let decoded = unescaped(bytes);
        let escaped = matches!(decoded, Cow::Owned(_));
        if self.holds(bytes) | (escaped && self.holds(&decoded)) {
            return Ok(false);
        }
        let mut compared = 0;
        for reading in readings(bytes, &decoded, escaped, json) {
            let number = match reading {
                Reading::Number(number) => number,
                Reading::Stretch => {
                    poll()?;
                    continue;
                }
            };
            if compared % BATCH == 0 {
                poll()?;
            }
            compared += 1;
            if self.valued(number) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether any needle the shared masker knows occurs in `bytes`.
    fn holds(&self, bytes: &[u8]) -> bool {
        let occurs = |needle: &Vec<u8>| bytes.windows(needle.len()).any(|at| at == needle);
        self.needles.iter().any(occurs)
    }

    /// Whether the number `number` is, by what it is worth, the plaintext
    /// of any one of the bindings that is itself a decimal number (MB4):
    /// masking text finds a bound value only as it is spelled, and `1e+20`,
    /// `876543210.0` or `8.7654321e8` spell one the operator bound as
    /// `100000000000000000000` or `0876543210`.
    ///
    /// Each side is read as an exact decimal, in arbitrary precision and
    /// never through a float: its significant digits, with no leading or
    /// trailing zero, and the power of ten they stand at, every zero the
    /// one zero. A value and its negation disclose the same digits, so the
    /// sign is read as spelling (operator ruling 2026-10-13). Each binding
    /// is read on its own, so a binding of zeros alone refuses only zero.
    /// `number` and a binding are each read whole, as a sign, digits with
    /// at most one point, and an exponent, leading zeros and a `+` allowed;
    /// any other text is no number. A number whose exponent is written with
    /// more digits than [`NUMBER_POWER_DIGITS`] cannot be told apart from a
    /// bound value of such a scale, and is answered as one wherever any
    /// binding is a number (operator ruling 2026-10-13); one within it
    /// never equals a binding whose exponent passes [`BOUND_POWER_DIGITS`].
    pub(super) fn valued(&self, number: &[u8]) -> bool {
        let Some(read) = Magnitude::read(number, NUMBER_POWER_DIGITS) else {
            return false;
        };
        self.numbers.iter().any(|bound| read.equals(bound))
    }
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        for needle in &mut self.needles {
            wipe(needle);
        }
        for number in &mut self.numbers {
            match number {
                Magnitude::Exact(digits, _) => wipe(digits),
                Magnitude::Vast => {}
            }
        }
    }
}

/// A poll that never ends a scan.
fn never() -> Result<(), Infallible> {
    Ok(())
}

/// Each number the JSON text `bytes` writes outside its strings, as
/// written ([`Numbers`]): the guard's own reading, which the broker's
/// precision check shares.
pub fn json_numbers(bytes: &[u8]) -> Vec<&[u8]> {
    let number = |reading| match reading {
        Reading::Number(number) => Some(number),
        Reading::Stretch => None,
    };
    Numbers::new(bytes, true).filter_map(number).collect()
}

/// Every reading the guard compares of `bytes`, whose escapes decode to
/// `decoded`: their numbers as JSON writes them where `json`, and as text
/// where not, then the numbers `decoded` spells. Where nothing in `bytes`
/// is `escaped`, `decoded` is `bytes`, and a text's second reading would be
/// its first again, so it is read once.
fn readings<'r>(
    bytes: &'r [u8],
    decoded: &'r [u8],
    escaped: bool,
    json: bool,
) -> impl Iterator<Item = Reading<'r>> {
    let again = json | escaped;
    let spelled = again.then(|| Numbers::new(decoded, false));
    Numbers::new(bytes, json).chain(spelled.into_iter().flatten())
}

/// What a reading of bytes comes to next: a number they write, as written,
/// or the end of a `STRETCH` of them read, where the guard polls its
/// caller.
enum Reading<'a> {
    Number(&'a [u8]),
    Stretch,
}

/// Each number `bytes` write, as written and as far as [`Numbers::spelled`]
/// reads, one after another, each `STRETCH` of the bytes read marked. Where
/// `quoted`, each JSON number outside JSON's strings. Where not, every
/// number written anywhere, from each byte no digit is written just before,
/// whatever else is: a number glued to a word, a point or another number's
/// exponent is read as the number it spells, and a run of digits once, from
/// its first (operator ruling 2026-10-13: refusing more is never a leak).
/// Only a digit, a sign or a point can begin a number, so the bytes between
/// them are passed over without reading one. No byte is read past the end
/// of the `STRETCH` it is in before that stretch is marked, a run of digits'
/// bytes included: a run longer than a stretch is counted a stretch at a
/// time, and its number handed back whole once its last stretch is read.
struct Numbers<'a> {
    bytes: &'a [u8],
    quoted: bool,
    /// The byte read next, whether it is inside a JSON string, and where
    /// the last `STRETCH` ended.
    at: usize,
    text: bool,
    marked: usize,
    runs: Runs,
}

impl<'a> Numbers<'a> {
    fn new(bytes: &'a [u8], quoted: bool) -> Numbers<'a> {
        let runs = Runs {
            known: [None; 3],
            next: 0,
            counting: None,
        };
        Numbers {
            bytes,
            quoted,
            at: 0,
            text: false,
            marked: 0,
            runs,
        }
    }

    /// The next number written before `end`, read up to it; the end of the
    /// stretch where a run of digits passes `end` before its number is
    /// read, the reading to go on from that number; none where none is.
    fn before(&mut self, end: usize) -> Option<Reading<'a>> {
        let bytes = self.bytes;
        while self.at < end.min(bytes.len()) {
            let at = self.at;
            match (self.text, bytes[at]) {
                (true, b'\\') => self.at += 2,
                (_, b'"') if self.quoted => {
                    self.text = !self.text;
                    self.at += 1;
                }
                (false, b'0'..=b'9' | b'+' | b'-' | b'.') => match self.read(at, end) {
                    Some(0) => {}
                    Some(length) => return Some(Reading::Number(&bytes[at..at + length])),
                    None => {
                        self.marked = end;
                        return Some(Reading::Stretch);
                    }
                },
                _ => self.at += 1,
            }
        }
        None
    }

    /// How many bytes from `at`, where a number may begin, write one, the
    /// reading moved on past it; none, the reading left at `at`, where a
    /// run of digits passes `end` first. Where `quoted`, past the whole of
    /// it, or one byte. Where not, past the run of digits it begins with
    /// and the byte after them, which a digit is written just before, or
    /// one byte.
    fn read(&mut self, at: usize, end: usize) -> Option<usize> {
        let length = self.spelled(at, end)?;
        self.at = match (self.quoted, self.bytes[at].is_ascii_digit()) {
            (true, _) => at + length.max(1),
            (false, true) => self.runs.end(self.bytes, at, end)? + 1,
            (false, false) => at + 1,
        };
        Some(length)
    }

    /// How many bytes from `at` write a number: a sign, digits with at most
    /// one point and one digit or more in all, and an exponent where a
    /// digit follows its letter and sign; none where they write none. Every
    /// JSON number is read whole. `None` where a run of digits in it passes
    /// `end` before it ends.
    fn spelled(&mut self, at: usize, end: usize) -> Option<usize> {
        let bytes = self.bytes;
        let signed = at + usize::from(matches!(bytes[at], b'-' | b'+'));
        let whole = self.runs.end(bytes, signed, end)?;
        let pointed = whole + usize::from(bytes.get(whole) == Some(&b'.'));
        let fraction = self.runs.end(bytes, pointed, end)?;
        let letter = bytes
            .get(fraction)
            .is_some_and(|b| b.eq_ignore_ascii_case(&b'e'));
        let sign = letter & matches!(bytes.get(fraction + 1), Some(b'-' | b'+'));
        let exponent = fraction + 1 + usize::from(sign);
        let power = match letter {
            true => self.runs.end(bytes, exponent, end)?,
            false => exponent,
        };
        Some(
            match (whole - signed + fraction - pointed, power - exponent) {
                (0, _) => 0,
                (_, 0) => fraction - at,
                (_, _) => power - at,
            },
        )
    }
}

impl<'a> Iterator for Numbers<'a> {
    type Item = Reading<'a>;

    fn next(&mut self) -> Option<Reading<'a>> {
        while self.at < self.bytes.len() {
            let end = self.marked + STRETCH;
            if self.at >= end {
                self.marked = self.at;
                return Some(Reading::Stretch);
            }
            if let Some(reading) = self.before(end) {
                return Some(reading);
            }
        }
        None
    }
}

/// The runs of digits a reading has counted most recently, each as where it
/// begins and ends: a number and the numbers it overlaps share their runs,
/// and every one they read is among the last three counted, so each run is
/// counted once. A run whose count stopped at a stretch's end is held as
/// where it begins and how far it was counted, and its count goes on from
/// there.
struct Runs {
    known: [Option<(usize, usize)>; 3],
    next: usize,
    counting: Option<(usize, usize)>,
}

impl Runs {
    /// Where the run of digits `bytes` write from `from` ends: `from`
    /// itself where they write none there; none where the run passes
    /// `limit`, which its count stops at.
    fn end(&mut self, bytes: &[u8], from: usize, limit: usize) -> Option<usize> {
        if !bytes.get(from).is_some_and(u8::is_ascii_digit) {
            return Some(from);
        }
        let mut counted = self.known.iter().flatten();
        if let Some(&(_, end)) = counted.find(|(start, _)| *start == from) {
            return Some(end);
        }
        let mut end = match self.counting {
            Some((start, reached)) if start == from => reached,
            _ => from + 1,
        };
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            if end >= limit {
                self.counting = Some((from, end));
                return None;
            }
            end += 1;
        }
        self.known[self.next % 3] = Some((from, end));
        self.next += 1;
        Some(end)
    }
}

/// `bytes` with every JSON escape in them decoded, in one pass from the
/// first: what a JSON reader takes each string in them to say, and the
/// rest as it is; `bytes` themselves where no backslash is in them. A `\u`
/// escape's UTF-16 units are read together, a lone surrogate as U+FFFD;
/// another escape is the byte after its backslash, `\b`, `\f`, `\n`, `\r`
/// and `\t` the control each names, and a last backslash itself.
fn unescaped(bytes: &[u8]) -> Cow<'_, [u8]> {
    if !bytes.contains(&b'\\') {
        return Cow::Borrowed(bytes);
    }
    let (mut decoded, mut at) = (Vec::with_capacity(bytes.len()), 0);
    while let Some(plain) = bytes[at..].iter().position(|byte| *byte == b'\\') {
        decoded.extend_from_slice(&bytes[at..at + plain]);
        at += plain;
        let units = units(&bytes[at..]);
        let read = match (bytes.get(at + 1), units.len()) {
            (_, 1..) => {
                decoded.extend_from_slice(String::from_utf16_lossy(&units).as_bytes());
                6 * units.len()
            }
            (Some(escaped), 0) => {
                decoded.push(control(*escaped));
                2
            }
            (None, 0) => {
                decoded.push(b'\\');
                1
            }
        };
        at += read;
    }
    decoded.extend_from_slice(&bytes[at..]);
    Cow::Owned(decoded)
}

/// The UTF-16 units of the `\u` escapes `bytes` begin with, one after
/// another.
fn units(bytes: &[u8]) -> Vec<u16> {
    let mut units = Vec::new();
    while let Some(unit) = bytes
        .get(6 * units.len()..6 * units.len() + 6)
        .and_then(unit)
    {
        units.push(unit);
    }
    units
}

/// The UTF-16 unit the escape `escape`, a `\u` and four hex digits, names.
fn unit(escape: &[u8]) -> Option<u16> {
    let hex = escape.strip_prefix(b"\\u")?;
    let digits = std::str::from_utf8(hex).ok()?;
    hex.iter()
        .all(u8::is_ascii_hexdigit)
        .then(|| u16::from_str_radix(digits, 16).ok())
        .flatten()
}

/// The byte a backslash and `escaped` stand for.
fn control(escaped: u8) -> u8 {
    match escaped {
        b'b' => 0x08,
        b'f' => 0x0c,
        b'n' => b'\n',
        b'r' => b'\r',
        b't' => b'\t',
        other => other,
    }
}

/// A decimal number's magnitude: exact, as its significant digits and the
/// power of ten the last of them stands at; or vast, its exponent written
/// with more significant digits than it was read with.
#[derive(PartialEq)]
enum Magnitude {
    Exact(Vec<u8>, i128),
    Vast,
}

impl Magnitude {
    /// The magnitude `text` writes, its exponent read to at most `most`
    /// significant digits; none where it is no number.
    fn read(text: &[u8], most: usize) -> Option<Magnitude> {
        let unsigned = match text.first() {
            Some(b'-' | b'+') => &text[1..],
            _ => text,
        };
        let (mantissa, exponent) = match unsigned.iter().position(|b| b.eq_ignore_ascii_case(&b'e'))
        {
            Some(at) => (&unsigned[..at], Some(&unsigned[at + 1..])),
            None => (unsigned, None),
        };
        let (whole, fraction) = match mantissa.iter().position(|byte| *byte == b'.') {
            Some(at) => (&mantissa[..at], &mantissa[at + 1..]),
            None => (mantissa, &b""[..]),
        };
        let mut digits: Vec<u8> = whole.iter().chain(fraction).copied().collect();
        let numeral = !digits.is_empty() & digits.iter().all(u8::is_ascii_digit);
        let power = exponent.map_or(Some(Power::Exact(0)), |exponent| {
            Power::read(exponent, most)
        });
        let read = power
            .filter(|_| numeral)
            .map(|power| Magnitude::scaled(&digits, fraction.len(), power));
        // A binding's bytes, copied here, are wiped whatever they were.
        wipe(&mut digits);
        read
    }

    /// The magnitude of the decimal `digits`, the last `fraction` of them
    /// past its point, scaled by ten to `power`: zero whatever its power.
    fn scaled(digits: &[u8], fraction: usize, power: Power) -> Magnitude {
        let Some(first) = digits.iter().position(|digit| *digit != b'0') else {
            return Magnitude::Exact(Vec::new(), 0);
        };
        let Power::Exact(power) = power else {
            return Magnitude::Vast;
        };
        let last = digits
            .iter()
            .rposition(|digit| *digit != b'0')
            .unwrap_or(first);
        let trailing = (digits.len() - 1 - last) as i128;
        let at = power + trailing - fraction as i128;
        Magnitude::Exact(digits[first..=last].to_vec(), at)
    }

    /// Whether a number read as `self` may be the binding read as `bound`:
    /// a vast number may be any; an exact one is a vast binding never.
    fn equals(&self, bound: &Magnitude) -> bool {
        match (self, bound) {
            (Magnitude::Vast, _) => true,
            (Magnitude::Exact(..), Magnitude::Vast) => false,
            (Magnitude::Exact(..), Magnitude::Exact(..)) => self == bound,
        }
    }
}

/// A number's exponent: exact, or written with more significant digits
/// than it was read with.
#[derive(PartialEq)]
enum Power {
    Exact(i128),
    Vast,
}

impl Power {
    /// The exponent `text` writes, a sign and one digit or more, read to at
    /// most `most` significant digits; none where it is no exponent.
    fn read(text: &[u8], most: usize) -> Option<Power> {
        let (negative, unsigned) = match text.first() {
            Some(b'-') => (true, &text[1..]),
            Some(b'+') => (false, &text[1..]),
            _ => (false, text),
        };
        let numeral = !unsigned.is_empty() & unsigned.iter().all(u8::is_ascii_digit);
        let first = unsigned.iter().position(|digit| *digit != b'0');
        let significant = &unsigned[first.unwrap_or(unsigned.len())..];
        match (numeral, significant.len() > most) {
            (false, _) => None,
            (true, true) => Some(Power::Vast),
            (true, false) => {
                let value = significant
                    .iter()
                    .fold(0i128, |value, digit| value * 10 + i128::from(digit - b'0'));
                Some(Power::Exact(if negative { -value } else { value }))
            }
        }
    }
}

#[cfg(test)]
mod tests;
