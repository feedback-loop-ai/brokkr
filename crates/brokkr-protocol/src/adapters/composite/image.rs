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

use thiserror::Error;

/// The two native formats the readers know: Linux's and macOS's, the
/// supported hosts (decision 0063). Any other magic, a PE's included, is
/// not a native image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Elf,
    MachO,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Kind::Elf => "ELF",
            Kind::MachO => "Mach-O",
        })
    }
}

/// What an image declares about how it is loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Native {
    pub(super) kind: Kind,
    /// The dynamic loader the image names — `PT_INTERP` or
    /// `LC_LOAD_DYLINKER` — as the bytes of its path, without the NUL.
    /// `None` for a static image and for a loader itself.
    pub(super) loader: Option<Vec<u8>>,
    /// Whether the image is itself a dynamic loader: an ELF loader is any
    /// executable or shared image; a Mach-O loader is `MH_DYLINKER`.
    pub(super) loads: bool,
}

impl Native {
    /// Whether this image is what a `kind` image may name as its loader:
    /// a loader of the same format.
    pub(super) fn is_loader_for(&self, kind: Kind) -> bool {
        self.kind == kind && self.loads
    }
}

/// The format the running target executes natively.
pub(super) const NATIVE: Kind = if cfg!(target_vendor = "apple") {
    Kind::MachO
} else {
    Kind::Elf
};

/// This target's machine identifiers in each format's own vocabulary:
/// ELF `e_machine` and Mach-O `cputype`. A target the table does not
/// name matches no image, so every native image refuses by name rather
/// than being admitted on a guess.
struct Arch {
    elf: u16,
    macho: u32,
}

const ARCH: Arch = if cfg!(target_arch = "x86_64") {
    Arch {
        elf: 62,
        macho: 0x0100_0007,
    }
} else if cfg!(target_arch = "aarch64") {
    Arch {
        elf: 183,
        macho: 0x0100_000c,
    }
} else if cfg!(target_arch = "x86") {
    Arch { elf: 3, macho: 7 }
} else {
    Arch { elf: 0, macho: 0 }
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

/// A stretch of the file a reader reads, as a refusal names it: the one a
/// read failed on, or the one the file ends inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Region {
    Magic,
    ElfHeader,
    ElfProgramHeaders,
    ElfLoadSegment,
    ElfInterpreter,
    UniversalHeader,
    UniversalTable,
    UniversalSlice,
    MachOHeader,
    MachOCommands,
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Region::Magic => "the first four bytes",
            Region::ElfHeader => "an ELF header",
            Region::ElfProgramHeaders => "an ELF program header table",
            Region::ElfLoadSegment => "an ELF load segment",
            Region::ElfInterpreter => "an ELF interpreter path",
            Region::UniversalHeader => "a universal header",
            Region::UniversalTable => "a universal architecture table",
            Region::UniversalSlice => "a universal slice",
            Region::MachOHeader => "a Mach-O header",
            Region::MachOCommands => "a Mach-O load-command table",
        })
    }
}

/// Why a file is not a loadable image of a format the readers know (#353):
/// each refusal is a variant, and its `Display` is the reason the resolver
/// has always folded into its own refusal, byte for byte. An I/O failure
/// is carried as its text.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum Unloadable {
    #[error("{0} beyond the end of the file")]
    Beyond(Region),
    #[error("{region} cannot be read: {cause}")]
    Unread { region: Region, cause: String },
    #[error("an unsupported 32-bit Mach-O image")]
    MachO32,
    #[error("an unsupported byte-swapped Mach-O image")]
    MachOSwapped,
    #[error("an unsupported 64-bit universal image")]
    Universal64,
    #[error("neither a #! script nor a native image")]
    Unknown,
    #[error("an unsupported ELF class {0}")]
    ElfClass(u8),
    #[error("an unsupported ELF byte order {0}")]
    ElfByteOrder(u8),
    #[error("an unsupported ELF version {0}")]
    ElfVersion(u8),
    #[error("ELF type {0}, which is neither an executable nor a shared object")]
    ElfType(u16),
    #[error("ELF machine {0}, which is not this target's {elf}", elf = ARCH.elf)]
    ElfMachine(u16),
    #[error("an ELF program header size {found} where {required} is required")]
    ElfHeaderSize { found: u16, required: u16 },
    #[error("no ELF program headers")]
    ElfNoHeaders,
    #[error(
        "an ELF program header table of {0} bytes, above the {ELF_PHDR_TABLE_BOUND}-byte bound"
    )]
    ElfTableBound(u64),
    #[error("an ELF load segment whose file size {filesz} exceeds its memory size {memsz}")]
    ElfSegmentSize { filesz: u64, memsz: u64 },
    #[error("more than one ELF interpreter")]
    ElfInterpreters,
    #[error("an ELF interpreter path of {0} bytes")]
    ElfInterpreterLength(u64),
    #[error("an ELF interpreter path that is not NUL-terminated")]
    ElfInterpreterUnterminated,
    #[error("an empty ELF interpreter path")]
    ElfInterpreterEmpty,
    #[error("ELF class {0}, which is not this target's")]
    ElfForeignClass(u8),
    #[error("a universal image with {0} architectures")]
    UniversalCount(u32),
    #[error("a universal slice that is not a 64-bit Mach-O image")]
    UniversalSliceNotMachO,
    #[error("a universal image with no slice for this target's CPU type {cpu}", cpu = ARCH.macho)]
    UniversalNoSlice,
    #[error("Mach-O CPU type {0}, which is not this target's {cpu}", cpu = ARCH.macho)]
    MachOCpu(u32),
    #[error("Mach-O file type {0}, which is neither an executable nor a dynamic linker")]
    MachOFileType(u32),
    #[error(
        "a Mach-O load-command table of {0} bytes, above the {MACHO_COMMANDS_BOUND}-byte bound"
    )]
    MachOTableBound(u32),
    #[error("a Mach-O load command beyond its table")]
    MachOCommandBeyond,
    #[error("a malformed Mach-O load command")]
    MachOCommand,
    #[error("more than one Mach-O dynamic linker")]
    MachODylinkers,
    #[error("a malformed Mach-O dynamic linker command")]
    MachODylinker,
    #[error("an empty Mach-O dynamic linker path")]
    MachODylinkerEmpty,
}

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
    region: Region,
) -> Result<Vec<u8>, Unloadable> {
    if offset.checked_add(count).is_none_or(|end| end > len) {
        return Err(Unloadable::Beyond(region));
    }
    // Every caller bounds `count` by its format's own limit before
    // asking, so the allocation is at most that bound.
    let mut bytes = vec![0; count as usize];
    source
        .seek(SeekFrom::Start(offset))
        .and_then(|_| source.read_exact(&mut bytes))
        .map_err(|error| unread(region, &error))?;
    Ok(bytes)
}

/// The refusal for a read of `region` that the device failed.
fn unread(region: Region, error: &std::io::Error) -> Unloadable {
    Unloadable::Unread {
        region,
        cause: error.to_string(),
    }
}

/// Inspect the image `source` of `len` bytes: parse it by its magic and
/// answer its loading declaration, or the reason it is not a loadable
/// image of any known format.
pub(super) fn inspect(source: &mut (impl Read + Seek), len: u64) -> Result<Native, Unloadable> {
    let mut magic = Vec::with_capacity(4);
    source
        .seek(SeekFrom::Start(0))
        .and_then(|_| source.take(4).read_to_end(&mut magic))
        .map_err(|error| unread(Region::Magic, &error))?;
    match magic.as_slice() {
        [0x7f, b'E', b'L', b'F'] => elf(source, len),
        [0xcf, 0xfa, 0xed, 0xfe] => macho_thin(source, 0, len),
        [0xca, 0xfe, 0xba, 0xbe] => macho_fat(source, len),
        [0xce, 0xfa, 0xed, 0xfe] => Err(Unloadable::MachO32),
        [0xfe, 0xed, 0xfa, 0xce] | [0xfe, 0xed, 0xfa, 0xcf] => Err(Unloadable::MachOSwapped),
        [0xca, 0xfe, 0xba, 0xbf] => Err(Unloadable::Universal64),
        _ => Err(Unloadable::Unknown),
    }
}

/// The ELF rule, as `load_elf_binary` applies it: header identity, type,
/// machine, a bounded program-header table inside the file, every load
/// segment's sizes consistent, and at most one well-formed `PT_INTERP`.
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn elf(source: &mut (impl Read + Seek), len: u64) -> Result<Native, Unloadable> {
    let header = read_range(source, len, 0, 64, Region::ElfHeader)?;
    let header = Bytes(&header);
    let class = header.0[4];
    let data = header.0[5];
    let version = header.0[6];
    if class != 1 && class != 2 {
        return Err(Unloadable::ElfClass(class));
    }
    if data != ELF_DATA {
        return Err(Unloadable::ElfByteOrder(data));
    }
    if version != 1 {
        return Err(Unloadable::ElfVersion(version));
    }
    // Every multi-byte field below is native-order, which the byte-order
    // check above has just established is little-endian on this target.
    let e_type = header.u16_le(16).expect("64-byte header");
    let e_machine = header.u16_le(18).expect("64-byte header");
    if e_type != 2 && e_type != 3 {
        return Err(Unloadable::ElfType(e_type));
    }
    if e_machine != ARCH.elf {
        return Err(Unloadable::ElfMachine(e_machine));
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
        return Err(Unloadable::ElfHeaderSize {
            found: phentsize,
            required: expected_phentsize,
        });
    }
    if phnum == 0 {
        return Err(Unloadable::ElfNoHeaders);
    }
    let table_len = u64::from(phnum) * u64::from(phentsize);
    if table_len > ELF_PHDR_TABLE_BOUND {
        return Err(Unloadable::ElfTableBound(table_len));
    }
    let table = read_range(source, len, phoff, table_len, Region::ElfProgramHeaders)?;
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
                    return Err(Unloadable::ElfSegmentSize { filesz, memsz });
                }
                if offset.checked_add(filesz).is_none_or(|end| end > len) {
                    return Err(Unloadable::Beyond(Region::ElfLoadSegment));
                }
            }
            // PT_INTERP: exactly one, of a bounded length, NUL-terminated.
            3 => {
                if interpreter.is_some() {
                    return Err(Unloadable::ElfInterpreters);
                }
                if !(2..=ELF_INTERP_BOUND).contains(&filesz) {
                    return Err(Unloadable::ElfInterpreterLength(filesz));
                }
                interpreter = Some((offset, filesz));
            }
            _ => {}
        }
    }
    let loader = match interpreter {
        None => None,
        Some((offset, filesz)) => {
            let mut path = read_range(source, len, offset, filesz, Region::ElfInterpreter)?;
            if path.pop() != Some(0) {
                return Err(Unloadable::ElfInterpreterUnterminated);
            }
            // The kernel opens the C string: the bytes up to the first
            // NUL, however the segment is padded after it.
            path.truncate(
                path.iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(path.len()),
            );
            if path.is_empty() {
                return Err(Unloadable::ElfInterpreterEmpty);
            }
            Some(path)
        }
    };
    if class != ELF_CLASS {
        return Err(Unloadable::ElfForeignClass(class));
    }
    Ok(Native {
        kind: Kind::Elf,
        loader,
        loads: true,
    })
}

/// A universal (fat) Mach-O: the slice for this target's CPU type,
/// parsed as a thin image at its offset. Fields are big-endian.
fn macho_fat(source: &mut (impl Read + Seek), len: u64) -> Result<Native, Unloadable> {
    let head = read_range(source, len, 0, 8, Region::UniversalHeader)?;
    let count = Bytes(&head).u32_be(4).expect("8-byte header");
    if count == 0 || count > FAT_ARCH_BOUND {
        return Err(Unloadable::UniversalCount(count));
    }
    let table = read_range(
        source,
        len,
        8,
        u64::from(count) * 20,
        Region::UniversalTable,
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
            .ok_or(Unloadable::Beyond(Region::UniversalSlice))?;
        let mut magic = [0u8; 4];
        source
            .seek(SeekFrom::Start(offset))
            .and_then(|_| source.read_exact(&mut magic))
            .map_err(|error| unread(Region::UniversalSlice, &error))?;
        return match magic {
            [0xcf, 0xfa, 0xed, 0xfe] => macho_thin(source, offset, end),
            _ => Err(Unloadable::UniversalSliceNotMachO),
        };
    }
    Err(Unloadable::UniversalNoSlice)
}

/// The 64-bit Mach-O rule, as the XNU loader applies it: CPU type, a
/// file type that executes or loads, a bounded load-command table inside
/// the image and at most one `LC_LOAD_DYLINKER`. `base` and `end` bound
/// the image inside the file, so a universal slice reads its own bytes.
fn macho_thin(source: &mut (impl Read + Seek), base: u64, end: u64) -> Result<Native, Unloadable> {
    let header = read_range(source, end, base, 32, Region::MachOHeader)?;
    let header = Bytes(&header);
    let cputype = header.u32_le(4).expect("32-byte header");
    let filetype = header.u32_le(12).expect("32-byte header");
    let ncmds = header.u32_le(16).expect("32-byte header");
    let sizeofcmds = header.u32_le(20).expect("32-byte header");
    if cputype != ARCH.macho {
        return Err(Unloadable::MachOCpu(cputype));
    }
    let loads = match filetype {
        2 => false,
        7 => true,
        _ => return Err(Unloadable::MachOFileType(filetype)),
    };
    if sizeofcmds > MACHO_COMMANDS_BOUND {
        return Err(Unloadable::MachOTableBound(sizeofcmds));
    }
    let commands = read_range(
        source,
        end,
        base + 32,
        u64::from(sizeofcmds),
        Region::MachOCommands,
    )?;
    let commands = Bytes(&commands);
    let mut at = 0usize;
    let mut loader: Option<Vec<u8>> = None;
    for _ in 0..ncmds {
        let (Some(cmd), Some(cmdsize)) = (commands.u32_le(at), commands.u32_le(at + 4)) else {
            return Err(Unloadable::MachOCommandBeyond);
        };
        let cmdsize = cmdsize as usize;
        if cmdsize < 8
            || at
                .checked_add(cmdsize)
                .is_none_or(|next| next > commands.0.len())
        {
            return Err(Unloadable::MachOCommand);
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
                return Err(Unloadable::MachODylinkers);
            }
            let name = match cmdsize >= 12 {
                true => commands.u32_le(at + 8).expect("command bounds checked") as usize,
                false => return Err(Unloadable::MachODylinker),
            };
            if name < 12 || name >= cmdsize {
                return Err(Unloadable::MachODylinker);
            }
            let string = &commands.0[at + name..at + cmdsize];
            let path = &string[..string.iter().position(|b| *b == 0).unwrap_or(string.len())];
            if path.is_empty() {
                return Err(Unloadable::MachODylinkerEmpty);
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

/// This target's Mach-O CPU type, for a sibling test that plants a
/// synthetic image of another format as a candidate.
#[cfg(test)]
pub(super) fn tests_cputype() -> u32 {
    ARCH.macho
}

#[cfg(test)]
pub(super) mod tests;
