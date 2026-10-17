//! The strict reading of a judged mount record (decision 0065 slice two,
//! U8a; H6): its super options, an overlay's layers as the kernel shows
//! them, and the paths a record and its layers name. Each reading is
//! `None` wherever the table spells anything the kernel does not write,
//! and the verdict refuses on it, never guessing a second spelling.

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

use crate::hands::namespace::sources::host::unescaped;

/// Whether every super option of a judged mount is whole: none empty, and
/// each one's escapes three octal digits up to `\377`, none a NUL.
pub(super) fn whole(supers: &[Vec<u8>]) -> bool {
    supers.iter().all(|option| unescaped(option).is_some())
}

/// The path `bytes` spell, strictly: absolute, with no NUL, no empty, `.`
/// or `..` component, and not ending in the kernel's mark of a deleted
/// entry. Only `/` itself ends in `/`: the kernel writes no `//` and no
/// trailing `/` for a live path, and marks a deleted root `//deleted`.
pub(super) fn path(bytes: &[u8]) -> Option<PathBuf> {
    let strict = bytes.starts_with(b"/") & !bytes.contains(&0) & !dotted(bytes);
    let mut parts = (bytes.get(1..).unwrap_or_default()).split(|byte| *byte == b'/');
    let whole = (bytes == b"/") | !parts.any(<[u8]>::is_empty);
    let named = strict & whole & !bytes.ends_with(b" (deleted)");
    named.then(|| PathBuf::from(OsString::from_vec(bytes.to_vec())))
}

/// Whether the path `bytes` spell has a `.` or `..` component, which no
/// path the kernel writes has and no prefix can place.
pub(super) fn dotted(bytes: &[u8]) -> bool {
    let mut parts = bytes.split(|byte| *byte == b'/');
    parts.any(|part| matches!(part, b"." | b".."))
}

/// The super option that names a layer.
enum Layer {
    /// A legacy `lowerdir`, every lower and data layer in one value.
    Lowers,
    /// One `lowerdir+` or `datadir+`, which the kernel takes as given.
    Read,
    /// The `upperdir`, which the kernel unescapes.
    Upper,
    /// The `workdir`, which the kernel unescapes.
    Work,
}

impl Layer {
    /// The layer the super option `key` names; `None` for any other.
    fn named(key: &[u8]) -> Option<Layer> {
        match key {
            b"lowerdir" => Some(Layer::Lowers),
            b"lowerdir+" | b"datadir+" => Some(Layer::Read),
            b"upperdir" => Some(Layer::Upper),
            b"workdir" => Some(Layer::Work),
            _ => None,
        }
    }
}

/// The upper and work directories an overlay writes, both or neither, from
/// its super options, strictly: each value as `seq_show_option` writes it
/// ([`shown`]), then read as the kernel reads that option: a legacy
/// `lowerdir` split into its layers ([`lowers`]), an upper or work
/// directory unescaped ([`plain`]), a `lowerdir+` or `datadir+` as it is;
/// every layer a strict [`path`]; at least one layer read, and an upper
/// directory exactly where there is a work directory, once. Any other
/// option is not a layer.
pub(super) fn written(supers: &[Vec<u8>]) -> Option<Vec<PathBuf>> {
    let (mut read, mut upper, mut work) = (Vec::new(), Vec::new(), Vec::new());
    for option in supers {
        let at = option.iter().position(|byte| *byte == b'=');
        let Some(layer) = Layer::named(&option[..at.unwrap_or(option.len())]) else {
            continue;
        };
        let value = shown(&option[at? + 1..])?;
        match layer {
            Layer::Lowers => read.extend(lowers(&value)?),
            Layer::Read => read.push(path(&value)?),
            Layer::Upper => upper.push(path(&plain(&value)?)?),
            Layer::Work => work.push(path(&plain(&value)?)?),
        }
    }
    let whole = !read.is_empty() & matches!((upper.len(), work.len()), (0, 0) | (1, 1));
    upper.append(&mut work);
    whole.then_some(upper)
}

/// The bytes the kernel's `seq_show_option` escapes in an option value,
/// each as `\ooo`, and no other.
const SHOWN: &[u8] = b",= \t\n\\";

/// An option value decoded, where `raw` is exactly how `seq_show_option`
/// writes it: an escape of any byte it does not escape, or a bare byte it
/// does, is not the kernel's spelling.
fn shown(raw: &[u8]) -> Option<Vec<u8>> {
    let decoded = unescaped(raw)?.into_os_string().into_vec();
    let escape = |byte: &u8| match SHOWN.contains(byte) {
        true => format!("\\{byte:03o}").into_bytes(),
        false => vec![*byte],
    };
    let respelled: Vec<u8> = decoded.iter().flat_map(escape).collect();
    (respelled == raw).then_some(decoded)
}

/// `value` with each `\` taking the byte after it as itself, as the
/// kernel's `ovl_unescape` reads an upper or work directory; none where a
/// `\` ends it.
fn plain(value: &[u8]) -> Option<Vec<u8>> {
    let mut plain = Vec::with_capacity(value.len());
    let mut bytes = value.iter();
    while let Some(byte) = bytes.next() {
        plain.push(match byte {
            b'\\' => *bytes.next()?,
            _ => *byte,
        });
    }
    Some(plain)
}

/// The layers of a legacy `lowerdir` value: `:` between layers, one `::`
/// at most before the data layers, and `\` taking the byte after it as
/// itself, as the kernel splits and unescapes it. No other layer may be
/// empty, and a `\` may not end the value.
fn lowers(value: &[u8]) -> Option<Vec<PathBuf>> {
    let (mut found, mut layer) = (Vec::new(), Vec::new());
    let mut bytes = value.iter();
    while let Some(byte) = bytes.next() {
        match byte {
            b'\\' => layer.push(*bytes.next()?),
            b':' => found.push(std::mem::take(&mut layer)),
            _ => layer.push(*byte),
        }
    }
    found.push(layer);
    let empty = found.iter().filter(|layer| layer.is_empty()).count();
    let ends = found.first().zip(found.last());
    let ended = ends.is_some_and(|(first, last)| !first.is_empty() & !last.is_empty());
    (ended & (empty <= 1)).then_some(())?;
    let named = found.iter().filter(|layer| !layer.is_empty());
    named.map(|layer| path(layer)).collect()
}
