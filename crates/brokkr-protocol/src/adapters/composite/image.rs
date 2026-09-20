//! Bounded readers of the LOADING DECLARATIONS a native image carries:
//! what a kernel opens before it agrees to run the file, read here
//! without running it (design D10, security hold 2026-09-20, finding 4).
//!
//! `execvp` walks past `A/dsh` when `execve(A/dsh)` fails, and a native
//! image whose dynamic loader is missing fails exactly there — the kernel
//! opens the `PT_INTERP` path and answers `ENOENT` — so the child runs
//! `B/dsh` where a resolver that had admitted A on its metadata alone would
//! probe A and report a generic not-found. Metadata is not what the loader
//! reads; these readers read what it reads, with every offset, count and
//! range checked against the file before it is used.
//!
//! Every format is parsed on every target, by its magic, so a Mach-O read on
//! Linux is a parsed Mach-O refused for being another target's image rather
//! than a branch no Linux run can reach. Which format THIS target loads is
//! one datum, `NATIVE`. Nothing here relocates, resolves a symbol, walks a
//! shared-library graph or emulates a loader: an image is admitted when its
//! declared prerequisites are established, and refused by name otherwise.

use std::io::{Read, Seek, SeekFrom};

/// The three native formats the readers know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Elf,
    MachO,
    Pe,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Kind::Elf => "ELF",
            Kind::MachO => "Mach-O",
            Kind::Pe => "PE",
        })
    }
}

/// What an image declares about how it is loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Native {
    pub(super) kind: Kind,
    /// The dynamic loader the image names — `PT_INTERP` or
    /// `LC_LOAD_DYLINKER` — as the bytes of its path, without the NUL.
    /// `None` for a static image, for a loader itself and for every PE.
    pub(super) loader: Option<Vec<u8>>,
    /// Whether the image is itself a dynamic loader: an ELF loader is any
    /// executable or shared image; a Mach-O loader is `MH_DYLINKER`.
    pub(super) loads: bool,
}

impl Native {
    /// Whether this image is what a `kind` image may name as its loader:
    /// a loader of the same format. A PE names no loader, so no Windows
    /// production path asks; the reader's own tests ask on every target.
    #[cfg(any(unix, test))]
    pub(super) fn is_loader_for(&self, kind: Kind) -> bool {
        self.kind == kind && self.loads
    }
}

/// The format the running target executes natively.
pub(super) const NATIVE: Kind = if cfg!(windows) {
    Kind::Pe
} else if cfg!(target_vendor = "apple") {
    Kind::MachO
} else {
    Kind::Elf
};

/// This target's machine identifiers in each format's own vocabulary:
/// ELF `e_machine`, Mach-O `cputype` and PE `Machine`. A target the table
/// does not name matches no image, so every native image refuses by name
/// rather than being admitted on a guess.
struct Arch {
    elf: u16,
    macho: u32,
    pe: u16,
}

const ARCH: Arch = if cfg!(target_arch = "x86_64") {
    Arch {
        elf: 62,
        macho: 0x0100_0007,
        pe: 0x8664,
    }
} else if cfg!(target_arch = "aarch64") {
    Arch {
        elf: 183,
        macho: 0x0100_000c,
        pe: 0xAA64,
    }
} else if cfg!(target_arch = "x86") {
    Arch {
        elf: 3,
        macho: 7,
        pe: 0x14c,
    }
} else {
    Arch {
        elf: 0,
        macho: 0,
        pe: 0,
    }
};

/// `EI_DATA` for this target's byte order: the kernel refuses an image of
/// the other order (`elf_check_arch` and the `ELF_DATA` comparison).
const ELF_DATA: u8 = if cfg!(target_endian = "little") { 1 } else { 2 };

/// `EI_CLASS` for this target's pointer width.
const ELF_CLASS: u8 = if cfg!(target_pointer_width = "64") {
    2
} else {
    1
};

/// The kernel's bound on a program-header table (`ELF_MIN_ALIGN` on
/// every supported architecture is at most this).
const ELF_PHDR_TABLE_BOUND: u64 = 65_536;

/// `PATH_MAX`: the kernel's bound on a `PT_INTERP` path.
const ELF_INTERP_BOUND: u64 = 4096;

/// A bound on a Mach-O load-command table. `dyld` and every system
/// binary carry a few kilobytes; a table above this is not read whole.
const MACHO_COMMANDS_BOUND: u32 = 1 << 20;

/// A bound on a universal image's architecture count.
const FAT_ARCH_BOUND: u32 = 64;

/// A bound on a PE optional header. The two admitted layouts are 224 and
/// 240 bytes; the field is read, never trusted for allocation beyond this.
const PE_OPTIONAL_BOUND: u16 = 4096;

/// The loader's bound on a PE section table (`ntoskrnl` refuses an
/// image with more sections than this).
const PE_SECTIONS_BOUND: u16 = 96;

/// The PE optional-header magic this target's loader runs as a native
/// process: PE32+ on a 64-bit target, PE32 on a 32-bit one. The other
/// magic is another word size, which a 64-bit Windows runs under WOW64
/// as a different runtime and a 32-bit Windows does not run at all.
const PE_MAGIC: u16 = if cfg!(target_pointer_width = "64") {
    0x20b
} else {
    0x10b
};

/// Little-endian and big-endian field readers over a checked slice.
/// Every accessor answers `None` past the end rather than panicking, so
/// a truncated table is a refusal a caller names.
struct Bytes<'a>(&'a [u8]);

impl Bytes<'_> {
    fn u16_le(&self, at: usize) -> Option<u16> {
        self.0
            .get(at..at.checked_add(2)?)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32_le(&self, at: usize) -> Option<u32> {
        self.0
            .get(at..at.checked_add(4)?)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64_le(&self, at: usize) -> Option<u64> {
        self.0
            .get(at..at.checked_add(8)?)
            .map(|b| u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }

    fn u32_be(&self, at: usize) -> Option<u32> {
        self.0
            .get(at..at.checked_add(4)?)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// Read exactly `count` bytes at `offset`, having checked the range
/// against the file's length first: a table the header places beyond the
/// end is refused before any read is attempted.
fn read_range(
    source: &mut (impl Read + Seek),
    len: u64,
    offset: u64,
    count: u64,
    what: &str,
) -> Result<Vec<u8>, String> {
    if offset.checked_add(count).is_none_or(|end| end > len) {
        return Err(format!("{what} beyond the end of the file"));
    }
    // Every caller bounds `count` by its format's own limit before
    // asking, so the allocation is at most that bound.
    let mut bytes = vec![0; count as usize];
    source
        .seek(SeekFrom::Start(offset))
        .and_then(|_| source.read_exact(&mut bytes))
        .map_err(|error| format!("{what} cannot be read: {error}"))?;
    Ok(bytes)
}

/// Inspect the image `source` of `len` bytes: parse it by its magic and
/// answer its loading declaration, or the reason it is not a loadable
/// image of any known format.
pub(super) fn inspect(source: &mut (impl Read + Seek), len: u64) -> Result<Native, String> {
    let mut magic = Vec::with_capacity(4);
    source
        .seek(SeekFrom::Start(0))
        .and_then(|_| source.take(4).read_to_end(&mut magic))
        .map_err(|error| format!("the first four bytes cannot be read: {error}"))?;
    match magic.as_slice() {
        [0x7f, b'E', b'L', b'F'] => elf(source, len),
        [0xcf, 0xfa, 0xed, 0xfe] => macho_thin(source, 0, len),
        [0xca, 0xfe, 0xba, 0xbe] => macho_fat(source, len),
        [0xce, 0xfa, 0xed, 0xfe] => Err("an unsupported 32-bit Mach-O image".to_string()),
        [0xfe, 0xed, 0xfa, 0xce] | [0xfe, 0xed, 0xfa, 0xcf] => {
            Err("an unsupported byte-swapped Mach-O image".to_string())
        }
        [0xca, 0xfe, 0xba, 0xbf] => Err("an unsupported 64-bit universal image".to_string()),
        [b'M', b'Z', _, _] => pe(source, len),
        _ => Err("neither a #! script nor a native image".to_string()),
    }
}

/// The ELF rule, as `load_elf_binary` applies it: header identity, type,
/// machine, a bounded program-header table inside the file, every load
/// segment's sizes consistent, and at most one well-formed `PT_INTERP`.
fn elf(source: &mut (impl Read + Seek), len: u64) -> Result<Native, String> {
    let header = read_range(source, len, 0, 64, "an ELF header")?;
    let header = Bytes(&header);
    let class = header.0[4];
    let data = header.0[5];
    let version = header.0[6];
    if class != 1 && class != 2 {
        return Err(format!("an unsupported ELF class {class}"));
    }
    if data != ELF_DATA {
        return Err(format!("an unsupported ELF byte order {data}"));
    }
    if version != 1 {
        return Err(format!("an unsupported ELF version {version}"));
    }
    // Every multi-byte field below is native-order, which the byte-order
    // check above has just established is little-endian on this target.
    let e_type = header.u16_le(16).expect("64-byte header");
    let e_machine = header.u16_le(18).expect("64-byte header");
    if e_type != 2 && e_type != 3 {
        return Err(format!(
            "ELF type {e_type}, which is neither an executable nor a shared object"
        ));
    }
    if e_machine != ARCH.elf {
        return Err(format!(
            "ELF machine {e_machine}, which is not this target's {}",
            ARCH.elf
        ));
    }
    // The layouts differ between the two classes in field widths and, in
    // the program header, in field order. Both are read whole before the
    // class is compared with the target's, so the table rule is one rule
    // over both layouts rather than a layout no run of this target reads.
    let (phoff, phentsize, phnum, expected_phentsize) = match class {
        2 => (
            header.u64_le(32).expect("64-byte header"),
            header.u16_le(54).expect("64-byte header"),
            header.u16_le(56).expect("64-byte header"),
            56,
        ),
        _ => (
            u64::from(header.u32_le(28).expect("64-byte header")),
            header.u16_le(42).expect("64-byte header"),
            header.u16_le(44).expect("64-byte header"),
            32,
        ),
    };
    if phentsize != expected_phentsize {
        return Err(format!(
            "an ELF program header size {phentsize} where {expected_phentsize} is required"
        ));
    }
    if phnum == 0 {
        return Err("no ELF program headers".to_string());
    }
    let table_len = u64::from(phnum) * u64::from(phentsize);
    if table_len > ELF_PHDR_TABLE_BOUND {
        return Err(format!(
            "an ELF program header table of {table_len} bytes, above the {ELF_PHDR_TABLE_BOUND}-byte bound"
        ));
    }
    let table = read_range(source, len, phoff, table_len, "an ELF program header table")?;
    let table = Bytes(&table);
    let mut interpreter: Option<(u64, u64)> = None;
    for index in 0..usize::from(phnum) {
        let at = index * usize::from(phentsize);
        let p_type = table.u32_le(at).expect("table read whole");
        let (offset, filesz, memsz) = match class {
            2 => (
                table.u64_le(at + 8).expect("table read whole"),
                table.u64_le(at + 32).expect("table read whole"),
                table.u64_le(at + 40).expect("table read whole"),
            ),
            _ => (
                u64::from(table.u32_le(at + 4).expect("table read whole")),
                u64::from(table.u32_le(at + 16).expect("table read whole")),
                u64::from(table.u32_le(at + 20).expect("table read whole")),
            ),
        };
        match p_type {
            // PT_LOAD: the kernel refuses a segment whose file image is
            // larger than its memory image, and this refuses one that
            // reaches past the file.
            1 => {
                if filesz > memsz {
                    return Err(format!(
                        "an ELF load segment whose file size {filesz} exceeds its memory size {memsz}"
                    ));
                }
                if offset.checked_add(filesz).is_none_or(|end| end > len) {
                    return Err("an ELF load segment beyond the end of the file".to_string());
                }
            }
            // PT_INTERP: exactly one, of a bounded length, NUL-terminated.
            3 => {
                if interpreter.is_some() {
                    return Err("more than one ELF interpreter".to_string());
                }
                if !(2..=ELF_INTERP_BOUND).contains(&filesz) {
                    return Err(format!("an ELF interpreter path of {filesz} bytes"));
                }
                interpreter = Some((offset, filesz));
            }
            _ => {}
        }
    }
    let loader = match interpreter {
        None => None,
        Some((offset, filesz)) => {
            let mut path = read_range(source, len, offset, filesz, "an ELF interpreter path")?;
            if path.pop() != Some(0) {
                return Err("an ELF interpreter path that is not NUL-terminated".to_string());
            }
            // The kernel opens the C string: the bytes up to the first
            // NUL, however the segment is padded after it.
            path.truncate(
                path.iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(path.len()),
            );
            if path.is_empty() {
                return Err("an empty ELF interpreter path".to_string());
            }
            Some(path)
        }
    };
    if class != ELF_CLASS {
        return Err(format!("ELF class {class}, which is not this target's"));
    }
    Ok(Native {
        kind: Kind::Elf,
        loader,
        loads: true,
    })
}

/// A universal (fat) Mach-O: the slice for this target's CPU type,
/// parsed as a thin image at its offset. Fields are big-endian.
fn macho_fat(source: &mut (impl Read + Seek), len: u64) -> Result<Native, String> {
    let head = read_range(source, len, 0, 8, "a universal header")?;
    let count = Bytes(&head).u32_be(4).expect("8-byte header");
    if count == 0 || count > FAT_ARCH_BOUND {
        return Err(format!("a universal image with {count} architectures"));
    }
    let table = read_range(
        source,
        len,
        8,
        u64::from(count) * 20,
        "a universal architecture table",
    )?;
    let table = Bytes(&table);
    for index in 0..count as usize {
        let at = index * 20;
        let cputype = table.u32_be(at).expect("table read whole");
        if cputype != ARCH.macho {
            continue;
        }
        let offset = u64::from(table.u32_be(at + 8).expect("table read whole"));
        let size = u64::from(table.u32_be(at + 12).expect("table read whole"));
        let end = offset
            .checked_add(size)
            .filter(|end| *end <= len)
            .ok_or_else(|| "a universal slice beyond the end of the file".to_string())?;
        let mut magic = [0u8; 4];
        source
            .seek(SeekFrom::Start(offset))
            .and_then(|_| source.read_exact(&mut magic))
            .map_err(|error| format!("a universal slice cannot be read: {error}"))?;
        return match magic {
            [0xcf, 0xfa, 0xed, 0xfe] => macho_thin(source, offset, end),
            _ => Err("a universal slice that is not a 64-bit Mach-O image".to_string()),
        };
    }
    Err(format!(
        "a universal image with no slice for this target's CPU type {}",
        ARCH.macho
    ))
}

/// The 64-bit Mach-O rule, as the XNU loader applies it: CPU type, a
/// file type that executes or loads, a bounded load-command table inside
/// the image and at most one `LC_LOAD_DYLINKER`. `base` and `end` bound
/// the image inside the file, so a universal slice reads its own bytes.
fn macho_thin(source: &mut (impl Read + Seek), base: u64, end: u64) -> Result<Native, String> {
    let header = read_range(source, end, base, 32, "a Mach-O header")?;
    let header = Bytes(&header);
    let cputype = header.u32_le(4).expect("32-byte header");
    let filetype = header.u32_le(12).expect("32-byte header");
    let ncmds = header.u32_le(16).expect("32-byte header");
    let sizeofcmds = header.u32_le(20).expect("32-byte header");
    if cputype != ARCH.macho {
        return Err(format!(
            "Mach-O CPU type {cputype}, which is not this target's {}",
            ARCH.macho
        ));
    }
    let loads = match filetype {
        2 => false,
        7 => true,
        _ => {
            return Err(format!(
                "Mach-O file type {filetype}, which is neither an executable nor a dynamic linker"
            ))
        }
    };
    if sizeofcmds > MACHO_COMMANDS_BOUND {
        return Err(format!(
            "a Mach-O load-command table of {sizeofcmds} bytes, above the {MACHO_COMMANDS_BOUND}-byte bound"
        ));
    }
    let commands = read_range(
        source,
        end,
        base + 32,
        u64::from(sizeofcmds),
        "a Mach-O load-command table",
    )?;
    let commands = Bytes(&commands);
    let mut at = 0usize;
    let mut loader: Option<Vec<u8>> = None;
    for _ in 0..ncmds {
        let (Some(cmd), Some(cmdsize)) = (commands.u32_le(at), commands.u32_le(at + 4)) else {
            return Err("a Mach-O load command beyond its table".to_string());
        };
        let cmdsize = cmdsize as usize;
        if cmdsize < 8
            || at
                .checked_add(cmdsize)
                .is_none_or(|next| next > commands.0.len())
        {
            return Err("a malformed Mach-O load command".to_string());
        }
        // LC_LOAD_DYLINKER: the dynamic linker's path, an `lc_str`
        // offset from the command's start to a NUL-terminated string.
        // The command is twelve bytes at least — `cmd`, `cmdsize` and
        // the offset — and the generic eight-byte bound above does not
        // establish the third field: a 40-byte image whose one command
        // declared `cmdsize` 8 made the offset read panic inside doctor
        // where the kernel answers an exec-format error (review
        // 2026-09-20, R4). The command-specific read is bounded by the
        // command's own declared size before it is made.
        if cmd == 0xe {
            if loader.is_some() {
                return Err("more than one Mach-O dynamic linker".to_string());
            }
            let name = match cmdsize >= 12 {
                true => commands.u32_le(at + 8).expect("command bounds checked") as usize,
                false => return Err("a malformed Mach-O dynamic linker command".to_string()),
            };
            if name < 12 || name >= cmdsize {
                return Err("a malformed Mach-O dynamic linker command".to_string());
            }
            let string = &commands.0[at + name..at + cmdsize];
            let path = &string[..string.iter().position(|b| *b == 0).unwrap_or(string.len())];
            if path.is_empty() {
                return Err("an empty Mach-O dynamic linker path".to_string());
            }
            loader = Some(path.to_vec());
        }
        at += cmdsize;
    }
    Ok(Native {
        kind: Kind::MachO,
        loader: if loads { None } else { loader },
        loads,
    })
}

/// The PE rule, as far as a bounded header read establishes it: the
/// signature, this target's machine, an executable that is not a DLL, an
/// optional header of this target's magic inside the file, a subsystem
/// the OS runs as a process, a bounded section table inside the file
/// whose every raw-data range lies inside the file too, and a declared
/// header size that covers those headers and no more than the file
/// (review 2026-09-20, R7). What the OS binary-type query adds is asked
/// of the OS at admission, on Windows, by the caller.
fn pe(source: &mut (impl Read + Seek), len: u64) -> Result<Native, String> {
    let dos = read_range(source, len, 0, 64, "a DOS header")?;
    let lfanew = u64::from(Bytes(&dos).u32_le(60).expect("64-byte header"));
    let coff = read_range(source, len, lfanew, 24, "a PE header")?;
    let coff = Bytes(&coff);
    if &coff.0[..4] != b"PE\0\0" {
        return Err("no PE signature".to_string());
    }
    let machine = coff.u16_le(4).expect("24-byte header");
    let sections = coff.u16_le(6).expect("24-byte header");
    let optional_size = coff.u16_le(20).expect("24-byte header");
    let characteristics = coff.u16_le(22).expect("24-byte header");
    if machine != ARCH.pe {
        return Err(format!(
            "PE machine {machine:#x}, which is not this target's {:#x}",
            ARCH.pe
        ));
    }
    if characteristics & 0x0002 == 0 {
        return Err("a PE image that is not marked executable".to_string());
    }
    if characteristics & 0x2000 != 0 {
        return Err("a PE DLL, not an executable".to_string());
    }
    if !(70..=PE_OPTIONAL_BOUND).contains(&optional_size) {
        return Err(format!("a PE optional header of {optional_size} bytes"));
    }
    let optional = read_range(
        source,
        len,
        lfanew + 24,
        u64::from(optional_size),
        "a PE optional header",
    )?;
    let optional = Bytes(&optional);
    let magic = optional.u16_le(0).expect("70 bytes read");
    if magic != 0x10b && magic != 0x20b {
        return Err(format!("PE optional header magic {magic:#x}"));
    }
    if magic != PE_MAGIC {
        return Err(format!(
            "PE optional header magic {magic:#x}, which is not this target's {PE_MAGIC:#x}"
        ));
    }
    let size_of_headers = u64::from(optional.u32_le(60).expect("70 bytes read"));
    let subsystem = optional.u16_le(68).expect("70 bytes read");
    if subsystem != 2 && subsystem != 3 {
        return Err(format!(
            "PE subsystem {subsystem}, which is neither the console nor the GUI subsystem"
        ));
    }
    if sections == 0 || sections > PE_SECTIONS_BOUND {
        return Err(format!(
            "a PE image with {sections} sections, outside the loader's 1 to {PE_SECTIONS_BOUND}"
        ));
    }
    let table_at = lfanew + 24 + u64::from(optional_size);
    let table_len = u64::from(sections) * 40;
    let table = read_range(source, len, table_at, table_len, "a PE section table")?;
    let table = Bytes(&table);
    for index in 0..usize::from(sections) {
        let at = index * 40;
        let raw_size = u64::from(table.u32_le(at + 16).expect("table read whole"));
        let raw_at = u64::from(table.u32_le(at + 20).expect("table read whole"));
        if raw_size != 0 && raw_at.checked_add(raw_size).is_none_or(|end| end > len) {
            return Err("a PE section beyond the end of the file".to_string());
        }
    }
    let headers_end = table_at + table_len;
    if size_of_headers < headers_end || size_of_headers > len {
        return Err(format!(
            "a PE header size of {size_of_headers} bytes where the headers end at {headers_end} \
             in a file of {len}"
        ));
    }
    Ok(Native {
        kind: Kind::Pe,
        loader: None,
        loads: false,
    })
}

/// This target's Mach-O CPU type, for a sibling test that plants a
/// synthetic image of another format as a candidate.
#[cfg(all(test, unix))]
pub(super) fn tests_cputype() -> u32 {
    ARCH.macho
}

#[cfg(test)]
pub(super) mod tests;
