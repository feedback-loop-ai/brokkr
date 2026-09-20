use super::*;
use std::io::Cursor;

/// Inspect a synthetic image held in memory.
fn inspected(bytes: &[u8]) -> Result<Native, String> {
    inspect(&mut Cursor::new(bytes.to_vec()), bytes.len() as u64)
}

/// A reader that answers every read after the first `ok` bytes with an
/// error, so the I/O-failure arms are reached without a device that
/// fails on cue.
struct Failing {
    bytes: Vec<u8>,
    at: u64,
    ok: u64,
}

impl Read for Failing {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.at >= self.ok {
            return Err(std::io::Error::other("the device answered EIO"));
        }
        let remaining = &self.bytes[self.at as usize..];
        let count = buf
            .len()
            .min(remaining.len())
            .min((self.ok - self.at) as usize);
        buf[..count].copy_from_slice(&remaining[..count]);
        self.at += count as u64;
        Ok(count)
    }
}

impl Seek for Failing {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        let SeekFrom::Start(at) = position else {
            unreachable!("the readers seek from the start only")
        };
        self.at = at;
        Ok(at)
    }
}

fn le16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn le32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn le64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn be32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

/// One program header, in the fields the reader consults.
struct Phdr {
    p_type: u32,
    offset: u64,
    filesz: u64,
    memsz: u64,
}

const PT_LOAD: u32 = 1;
const PT_INTERP: u32 = 3;
const ET_EXEC: u16 = 2;
const ET_DYN: u16 = 3;

/// A 64-bit ELF: a 64-byte header whose identity fields are `ident`,
/// the program-header table at 64, then `payload`.
fn elf64_with(
    ident: [u8; 3],
    e_type: u16,
    machine: u16,
    phdrs: &[Phdr],
    payload: &[u8],
) -> Vec<u8> {
    let mut bytes = vec![0x7f, b'E', b'L', b'F', ident[0], ident[1], ident[2]];
    bytes.resize(16, 0);
    le16(&mut bytes, e_type);
    le16(&mut bytes, machine);
    le32(&mut bytes, 1);
    le64(&mut bytes, 0x1000);
    le64(&mut bytes, 64);
    le64(&mut bytes, 0);
    le32(&mut bytes, 0);
    le16(&mut bytes, 64);
    le16(&mut bytes, 56);
    le16(&mut bytes, phdrs.len() as u16);
    le16(&mut bytes, 64);
    le16(&mut bytes, 0);
    le16(&mut bytes, 0);
    assert_eq!(bytes.len(), 64);
    for phdr in phdrs {
        le32(&mut bytes, phdr.p_type);
        le32(&mut bytes, 5);
        le64(&mut bytes, phdr.offset);
        le64(&mut bytes, 0);
        le64(&mut bytes, 0);
        le64(&mut bytes, phdr.filesz);
        le64(&mut bytes, phdr.memsz);
        le64(&mut bytes, 0x1000);
    }
    bytes.extend_from_slice(payload);
    bytes
}

fn elf64(e_type: u16, machine: u16, phdrs: &[Phdr], payload: &[u8]) -> Vec<u8> {
    elf64_with([2, ELF_DATA, 1], e_type, machine, phdrs, payload)
}

/// The offset of the payload's first byte in an `elf64` with `phdrs`
/// program headers.
fn payload_at(phdrs: usize) -> u64 {
    64 + 56 * phdrs as u64
}

/// A 32-bit ELF, whose header fields sit at different offsets and whose
/// program headers order their fields differently.
fn elf32(e_type: u16, machine: u16, phdrs: &[Phdr], payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x7f, b'E', b'L', b'F', 1, ELF_DATA, 1];
    bytes.resize(16, 0);
    le16(&mut bytes, e_type);
    le16(&mut bytes, machine);
    le32(&mut bytes, 1);
    le32(&mut bytes, 0x1000);
    le32(&mut bytes, 52);
    le32(&mut bytes, 0);
    le32(&mut bytes, 0);
    le16(&mut bytes, 52);
    le16(&mut bytes, 32);
    le16(&mut bytes, phdrs.len() as u16);
    le16(&mut bytes, 40);
    le16(&mut bytes, 0);
    le16(&mut bytes, 0);
    assert_eq!(bytes.len(), 52);
    // The reader takes a 64-byte header for both classes; pad the
    // 32-bit one so the table still begins at its declared offset by
    // placing the table at 52 inside the padded region.
    for phdr in phdrs {
        le32(&mut bytes, phdr.p_type);
        le32(&mut bytes, phdr.offset as u32);
        le32(&mut bytes, 0);
        le32(&mut bytes, 0);
        le32(&mut bytes, phdr.filesz as u32);
        le32(&mut bytes, phdr.memsz as u32);
        le32(&mut bytes, 5);
        le32(&mut bytes, 0x1000);
    }
    bytes.extend_from_slice(payload);
    // A header read is 64 bytes whatever the class.
    if bytes.len() < 64 {
        bytes.resize(64, 0);
    }
    bytes
}

fn interp(path: &[u8]) -> Vec<u8> {
    let mut bytes = path.to_vec();
    bytes.push(0);
    bytes
}

/// A well-formed ELF64 of this target's machine with one load segment
/// and, when `loader` is given, a `PT_INTERP` naming it — for the
/// resolver's own loader-arm tests in the parent module, which plant it
/// as a candidate and never execute it.
pub(in crate::adapters::composite) fn synthetic_elf(loader: Option<&[u8]>) -> Vec<u8> {
    let load = Phdr {
        p_type: PT_LOAD,
        offset: 0,
        filesz: 64,
        memsz: 64,
    };
    match loader {
        None => elf64(ET_DYN, ARCH.elf, &[load], b""),
        Some(loader) => {
            let path = interp(loader);
            elf64(
                ET_DYN,
                ARCH.elf,
                &[
                    load,
                    Phdr {
                        p_type: PT_INTERP,
                        offset: payload_at(2),
                        filesz: path.len() as u64,
                        memsz: path.len() as u64,
                    },
                ],
                &path,
            )
        }
    }
}

#[test]
fn this_test_binary_is_this_targets_native_image() {
    let exe = std::env::current_exe().unwrap();
    let mut file = std::fs::File::open(&exe).unwrap();
    let len = file.metadata().unwrap().len();
    let native = inspect(&mut file, len).unwrap();
    assert_eq!(native.kind, NATIVE);
    assert_eq!(native.kind.to_string(), NATIVE.to_string());
    // A Rust test binary is dynamically linked on Linux and macOS, and
    // its loader is a real absolute path; a PE names none.
    match NATIVE {
        Kind::Pe => assert_eq!(native.loader, None),
        Kind::Elf | Kind::MachO => {
            let loader = native.loader.expect("a dynamic image names its loader");
            assert_eq!(loader[0], b'/', "{}", String::from_utf8_lossy(&loader));
        }
    }
}

#[test]
fn an_elf_declares_its_interpreter_or_none() {
    let path = interp(b"/lib64/ld-linux-x86-64.so.2");
    let dynamic = elf64(
        ET_DYN,
        ARCH.elf,
        &[
            Phdr {
                p_type: PT_LOAD,
                offset: 0,
                filesz: 64,
                memsz: 64,
            },
            Phdr {
                p_type: PT_INTERP,
                offset: payload_at(2),
                filesz: path.len() as u64,
                memsz: path.len() as u64,
            },
        ],
        &path,
    );
    assert_eq!(
        inspected(&dynamic).unwrap(),
        Native {
            kind: Kind::Elf,
            loader: Some(b"/lib64/ld-linux-x86-64.so.2".to_vec()),
            loads: true,
        }
    );
    // A NUL-padded interpreter segment names the C string the kernel
    // opens, however the segment is padded after it.
    let padded = elf64(
        ET_DYN,
        ARCH.elf,
        &[Phdr {
            p_type: PT_INTERP,
            offset: payload_at(1),
            filesz: 8,
            memsz: 8,
        }],
        b"/ld\0\0\0\0\0",
    );
    assert_eq!(inspected(&padded).unwrap().loader, Some(b"/ld".to_vec()));
    // A static image has no loader prerequisite, and an unknown
    // program-header type is passed over.
    let stat = elf64(
        ET_EXEC,
        ARCH.elf,
        &[
            Phdr {
                p_type: 0x6474e551,
                offset: 0,
                filesz: 0,
                memsz: 0,
            },
            Phdr {
                p_type: PT_LOAD,
                offset: 0,
                filesz: 64,
                memsz: 4096,
            },
        ],
        b"",
    );
    assert_eq!(inspected(&stat).unwrap().loader, None);
}

#[test]
fn every_elf_rule_refuses_by_name() {
    let load = || Phdr {
        p_type: PT_LOAD,
        offset: 0,
        filesz: 64,
        memsz: 64,
    };
    let interp_at = |offset: u64, len: u64| Phdr {
        p_type: PT_INTERP,
        offset,
        filesz: len,
        memsz: len,
    };
    let path = interp(b"/ld");
    let cases: Vec<(Vec<u8>, String)> = vec![
        (
            elf64_with([3, ELF_DATA, 1], ET_DYN, ARCH.elf, &[load()], b""),
            "an unsupported ELF class 3".into(),
        ),
        (
            elf64_with([2, 3 - ELF_DATA, 1], ET_DYN, ARCH.elf, &[load()], b""),
            format!("an unsupported ELF byte order {}", 3 - ELF_DATA),
        ),
        (
            elf64_with([2, ELF_DATA, 2], ET_DYN, ARCH.elf, &[load()], b""),
            "an unsupported ELF version 2".into(),
        ),
        (
            elf64(1, ARCH.elf, &[load()], b""),
            "ELF type 1, which is neither an executable nor a shared object".into(),
        ),
        (
            elf64(ET_DYN, ARCH.elf.wrapping_add(1), &[load()], b""),
            format!(
                "ELF machine {}, which is not this target's {}",
                ARCH.elf.wrapping_add(1),
                ARCH.elf
            ),
        ),
        (
            {
                let mut bytes = elf64(ET_DYN, ARCH.elf, &[load()], b"");
                bytes[54] = 55;
                bytes
            },
            "an ELF program header size 55 where 56 is required".into(),
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[], b""),
            "no ELF program headers".into(),
        ),
        (
            {
                let mut bytes = elf64(ET_DYN, ARCH.elf, &[load()], b"");
                bytes[56..58].copy_from_slice(&2000u16.to_le_bytes());
                bytes
            },
            "an ELF program header table of 112000 bytes, above the 65536-byte bound".into(),
        ),
        (
            {
                let mut bytes = elf64(ET_DYN, ARCH.elf, &[load()], b"");
                bytes[56..58].copy_from_slice(&2u16.to_le_bytes());
                bytes
            },
            "an ELF program header table beyond the end of the file".into(),
        ),
        (
            elf64(
                ET_DYN,
                ARCH.elf,
                &[Phdr {
                    p_type: PT_LOAD,
                    offset: 0,
                    filesz: 65,
                    memsz: 64,
                }],
                b"",
            ),
            "an ELF load segment whose file size 65 exceeds its memory size 64".into(),
        ),
        (
            elf64(
                ET_DYN,
                ARCH.elf,
                &[Phdr {
                    p_type: PT_LOAD,
                    offset: 0,
                    filesz: 4096,
                    memsz: 4096,
                }],
                b"",
            ),
            "an ELF load segment beyond the end of the file".into(),
        ),
        (
            elf64(
                ET_DYN,
                ARCH.elf,
                &[Phdr {
                    p_type: PT_LOAD,
                    offset: u64::MAX,
                    filesz: 1,
                    memsz: 1,
                }],
                b"",
            ),
            "an ELF load segment beyond the end of the file".into(),
        ),
        (
            elf64(
                ET_DYN,
                ARCH.elf,
                &[interp_at(payload_at(2), 4), interp_at(payload_at(2), 4)],
                &path,
            ),
            "more than one ELF interpreter".into(),
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[interp_at(payload_at(1), 1)], &path),
            "an ELF interpreter path of 1 bytes".into(),
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[interp_at(payload_at(1), 4097)], &path),
            "an ELF interpreter path of 4097 bytes".into(),
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[interp_at(payload_at(1), 5)], &path),
            "an ELF interpreter path beyond the end of the file".into(),
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[interp_at(payload_at(1), 3)], &path),
            "an ELF interpreter path that is not NUL-terminated".into(),
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[interp_at(payload_at(1), 3)], b"\0\0\0"),
            "an empty ELF interpreter path".into(),
        ),
        (
            elf32(ET_EXEC, ARCH.elf, &[load()], b""),
            "ELF class 1, which is not this target's".into(),
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[load()], b"")[..60].to_vec(),
            "an ELF header beyond the end of the file".into(),
        ),
    ];
    for (bytes, reason) in cases {
        assert_eq!(inspected(&bytes).unwrap_err(), reason);
    }
    // The 32-bit table is read whole before the class is compared, so a
    // 32-bit image with a malformed table refuses by the table's reason,
    // proving the 32-bit layout is parsed and not merely rejected.
    let bad_32 = elf32(
        ET_EXEC,
        ARCH.elf,
        &[Phdr {
            p_type: PT_LOAD,
            offset: 0,
            filesz: 9,
            memsz: 8,
        }],
        b"",
    );
    assert_eq!(
        inspected(&bad_32).unwrap_err(),
        "an ELF load segment whose file size 9 exceeds its memory size 8"
    );
    let interp_32 = elf32(
        ET_EXEC,
        ARCH.elf,
        &[Phdr {
            p_type: PT_INTERP,
            offset: 84,
            filesz: 4,
            memsz: 4,
        }],
        &path,
    );
    assert_eq!(
        inspected(&interp_32).unwrap_err(),
        "ELF class 1, which is not this target's"
    );
}

/// A 64-bit Mach-O with the load commands given.
fn macho(cputype: u32, filetype: u32, commands: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = vec![0xcf, 0xfa, 0xed, 0xfe];
    le32(&mut bytes, cputype);
    le32(&mut bytes, 0);
    le32(&mut bytes, filetype);
    le32(&mut bytes, commands.len() as u32);
    le32(
        &mut bytes,
        commands.iter().map(|command| command.len() as u32).sum(),
    );
    le32(&mut bytes, 0);
    le32(&mut bytes, 0);
    for command in commands {
        bytes.extend_from_slice(command);
    }
    bytes
}

fn dylinker(path: &[u8]) -> Vec<u8> {
    let mut command = Vec::new();
    le32(&mut command, 0xe);
    le32(&mut command, 12 + path.len() as u32 + 1);
    le32(&mut command, 12);
    command.extend_from_slice(path);
    command.push(0);
    command
}

fn segment() -> Vec<u8> {
    let mut command = Vec::new();
    le32(&mut command, 0x19);
    le32(&mut command, 72);
    command.resize(72, 0);
    command
}

const MH_EXECUTE: u32 = 2;
const MH_DYLINKER: u32 = 7;

#[test]
fn a_macho_declares_its_dynamic_linker_or_is_one() {
    let executable = macho(
        ARCH.macho,
        MH_EXECUTE,
        &[segment(), dylinker(b"/usr/lib/dyld")],
    );
    assert_eq!(
        inspected(&executable).unwrap(),
        Native {
            kind: Kind::MachO,
            loader: Some(b"/usr/lib/dyld".to_vec()),
            loads: false,
        }
    );
    let stat = macho(ARCH.macho, MH_EXECUTE, &[segment()]);
    assert_eq!(inspected(&stat).unwrap().loader, None);
    let dyld = macho(ARCH.macho, MH_DYLINKER, &[segment(), dylinker(b"/ignored")]);
    let dyld = inspected(&dyld).unwrap();
    assert_eq!(
        dyld,
        Native {
            kind: Kind::MachO,
            loader: None,
            loads: true,
        }
    );
    // A loader is one of the same format that loads: an executable of
    // the format is not, and a loader of another format is not.
    assert!(dyld.is_loader_for(Kind::MachO));
    assert!(!dyld.is_loader_for(Kind::Elf));
    assert!(!inspected(&stat).unwrap().is_loader_for(Kind::MachO));
}

#[test]
fn every_macho_rule_refuses_by_name() {
    let malformed = |command: Vec<u8>, reason: &str| {
        (
            macho(ARCH.macho, MH_EXECUTE, &[command]),
            reason.to_string(),
        )
    };
    let cases: Vec<(Vec<u8>, String)> = vec![
        (
            vec![0xce, 0xfa, 0xed, 0xfe, 0, 0, 0, 0],
            "an unsupported 32-bit Mach-O image".into(),
        ),
        (
            vec![0xfe, 0xed, 0xfa, 0xce, 0, 0, 0, 0],
            "an unsupported byte-swapped Mach-O image".into(),
        ),
        (
            vec![0xfe, 0xed, 0xfa, 0xcf, 0, 0, 0, 0],
            "an unsupported byte-swapped Mach-O image".into(),
        ),
        (
            vec![0xca, 0xfe, 0xba, 0xbf, 0, 0, 0, 0],
            "an unsupported 64-bit universal image".into(),
        ),
        (
            macho(ARCH.macho.wrapping_add(1), MH_EXECUTE, &[]),
            format!(
                "Mach-O CPU type {}, which is not this target's {}",
                ARCH.macho.wrapping_add(1),
                ARCH.macho
            ),
        ),
        (
            macho(ARCH.macho, 1, &[]),
            "Mach-O file type 1, which is neither an executable nor a dynamic linker".into(),
        ),
        (
            {
                let mut bytes = macho(ARCH.macho, MH_EXECUTE, &[]);
                bytes[20..24].copy_from_slice(&(MACHO_COMMANDS_BOUND + 1).to_le_bytes());
                bytes
            },
            format!(
                "a Mach-O load-command table of {} bytes, above the {MACHO_COMMANDS_BOUND}-byte bound",
                MACHO_COMMANDS_BOUND + 1
            ),
        ),
        (
            {
                let mut bytes = macho(ARCH.macho, MH_EXECUTE, &[]);
                bytes[20..24].copy_from_slice(&8u32.to_le_bytes());
                bytes
            },
            "a Mach-O load-command table beyond the end of the file".into(),
        ),
        (
            {
                let mut bytes = macho(ARCH.macho, MH_EXECUTE, &[segment()]);
                bytes[16..20].copy_from_slice(&2u32.to_le_bytes());
                bytes
            },
            "a Mach-O load command beyond its table".into(),
        ),
        malformed(
            {
                let mut command = segment();
                command[4..8].copy_from_slice(&4u32.to_le_bytes());
                command
            },
            "a malformed Mach-O load command",
        ),
        malformed(
            {
                let mut command = segment();
                command[4..8].copy_from_slice(&80u32.to_le_bytes());
                command
            },
            "a malformed Mach-O load command",
        ),
        (
            macho(
                ARCH.macho,
                MH_EXECUTE,
                &[dylinker(b"/usr/lib/dyld"), dylinker(b"/usr/lib/dyld")],
            ),
            "more than one Mach-O dynamic linker".into(),
        ),
        malformed(
            {
                let mut command = dylinker(b"/usr/lib/dyld");
                command[8..12].copy_from_slice(&11u32.to_le_bytes());
                command
            },
            "a malformed Mach-O dynamic linker command",
        ),
        malformed(
            {
                let mut command = dylinker(b"/usr/lib/dyld");
                let size = command.len() as u32;
                command[8..12].copy_from_slice(&size.to_le_bytes());
                command
            },
            "a malformed Mach-O dynamic linker command",
        ),
        malformed(dylinker(b""), "an empty Mach-O dynamic linker path"),
        (
            macho(ARCH.macho, MH_EXECUTE, &[])[..20].to_vec(),
            "a Mach-O header beyond the end of the file".into(),
        ),
    ];
    for (bytes, reason) in cases {
        assert_eq!(inspected(&bytes).unwrap_err(), reason, "{bytes:?}");
    }
}

/// A universal image holding `slices`, each `(cputype, bytes)`, laid out
/// after the architecture table.
fn fat(slices: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut bytes = vec![0xca, 0xfe, 0xba, 0xbe];
    be32(&mut bytes, slices.len() as u32);
    let mut offset = 8 + 20 * slices.len() as u32;
    for (cputype, slice) in slices {
        be32(&mut bytes, *cputype);
        be32(&mut bytes, 0);
        be32(&mut bytes, offset);
        be32(&mut bytes, slice.len() as u32);
        be32(&mut bytes, 0);
        offset += slice.len() as u32;
    }
    for (_, slice) in slices {
        bytes.extend_from_slice(slice);
    }
    bytes
}

#[test]
fn a_universal_image_is_read_at_this_targets_slice() {
    let other = macho(ARCH.macho.wrapping_add(1), MH_EXECUTE, &[segment()]);
    let ours = macho(
        ARCH.macho,
        MH_EXECUTE,
        &[segment(), dylinker(b"/usr/lib/dyld")],
    );
    let image = fat(&[
        (ARCH.macho.wrapping_add(1), other.clone()),
        (ARCH.macho, ours.clone()),
    ]);
    assert_eq!(
        inspected(&image).unwrap().loader,
        Some(b"/usr/lib/dyld".to_vec())
    );
    let cases: Vec<(Vec<u8>, String)> = vec![
        (fat(&[]), "a universal image with 0 architectures".into()),
        (
            {
                let mut bytes = fat(&[(ARCH.macho, ours.clone())]);
                bytes[4..8].copy_from_slice(&65u32.to_be_bytes());
                bytes
            },
            "a universal image with 65 architectures".into(),
        ),
        (
            fat(&[(ARCH.macho, ours.clone())])[..20].to_vec(),
            "a universal architecture table beyond the end of the file".into(),
        ),
        (
            fat(&[(ARCH.macho.wrapping_add(1), other.clone())]),
            format!(
                "a universal image with no slice for this target's CPU type {}",
                ARCH.macho
            ),
        ),
        (
            {
                let mut bytes = fat(&[(ARCH.macho, ours.clone())]);
                bytes[20..24].copy_from_slice(&u32::MAX.to_be_bytes());
                bytes
            },
            "a universal slice beyond the end of the file".into(),
        ),
        (
            fat(&[(ARCH.macho, fat(&[(ARCH.macho, ours.clone())]))]),
            "a universal slice that is not a 64-bit Mach-O image".into(),
        ),
        (
            fat(&[(ARCH.macho, ours[..20].to_vec())]),
            "a Mach-O header beyond the end of the file".into(),
        ),
        (
            fat(&[(ARCH.macho, vec![0; 4])]),
            "a universal slice that is not a 64-bit Mach-O image".into(),
        ),
    ];
    for (bytes, reason) in cases {
        assert_eq!(inspected(&bytes).unwrap_err(), reason, "{bytes:?}");
    }
}

/// A PE image: a DOS stub pointing at 64, the signature, a COFF header
/// and an optional header of `magic` and `subsystem`.
fn pe_image(machine: u16, characteristics: u16, magic: u16, subsystem: u16) -> Vec<u8> {
    let mut bytes = vec![b'M', b'Z'];
    bytes.resize(60, 0);
    le32(&mut bytes, 64);
    bytes.extend_from_slice(b"PE\0\0");
    le16(&mut bytes, machine);
    le16(&mut bytes, 1);
    le32(&mut bytes, 0);
    le32(&mut bytes, 0);
    le32(&mut bytes, 0);
    let optional_size: u16 = if magic == 0x20b { 240 } else { 224 };
    le16(&mut bytes, optional_size);
    le16(&mut bytes, characteristics);
    let optional_at = bytes.len();
    le16(&mut bytes, magic);
    bytes.resize(optional_at + 68, 0);
    le16(&mut bytes, subsystem);
    bytes.resize(optional_at + usize::from(optional_size), 0);
    bytes
}

#[test]
fn a_pe_image_is_admitted_by_its_bounded_header() {
    for magic in [0x20b, 0x10b] {
        assert_eq!(
            inspected(&pe_image(ARCH.pe, 0x0022, magic, 3)).unwrap(),
            Native {
                kind: Kind::Pe,
                loader: None,
                loads: false,
            }
        );
    }
    assert!(inspected(&pe_image(ARCH.pe, 0x0002, 0x20b, 2)).is_ok());
    let cases: Vec<(Vec<u8>, String)> = vec![
        (
            {
                let mut bytes = pe_image(ARCH.pe, 0x0022, 0x20b, 3);
                bytes[64..68].copy_from_slice(b"PE\0\x01");
                bytes
            },
            "no PE signature".into(),
        ),
        (
            pe_image(ARCH.pe.wrapping_add(1), 0x0022, 0x20b, 3),
            format!(
                "PE machine {:#x}, which is not this target's {:#x}",
                ARCH.pe.wrapping_add(1),
                ARCH.pe
            ),
        ),
        (
            pe_image(ARCH.pe, 0x0020, 0x20b, 3),
            "a PE image that is not marked executable".into(),
        ),
        (
            pe_image(ARCH.pe, 0x2002, 0x20b, 3),
            "a PE DLL, not an executable".into(),
        ),
        (
            {
                let mut bytes = pe_image(ARCH.pe, 0x0022, 0x20b, 3);
                bytes[84..86].copy_from_slice(&69u16.to_le_bytes());
                bytes
            },
            "a PE optional header of 69 bytes".into(),
        ),
        (
            {
                let mut bytes = pe_image(ARCH.pe, 0x0022, 0x20b, 3);
                bytes[84..86].copy_from_slice(&4097u16.to_le_bytes());
                bytes
            },
            "a PE optional header of 4097 bytes".into(),
        ),
        (
            {
                let mut bytes = pe_image(ARCH.pe, 0x0022, 0x20b, 3);
                bytes[84..86].copy_from_slice(&4096u16.to_le_bytes());
                bytes
            },
            "a PE optional header beyond the end of the file".into(),
        ),
        (
            pe_image(ARCH.pe, 0x0022, 0x107, 3),
            "PE optional header magic 0x107".into(),
        ),
        (
            pe_image(ARCH.pe, 0x0022, 0x20b, 1),
            "PE subsystem 1, which is neither the console nor the GUI subsystem".into(),
        ),
        (
            {
                let mut bytes = pe_image(ARCH.pe, 0x0022, 0x20b, 3);
                bytes[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
                bytes
            },
            "a PE header beyond the end of the file".into(),
        ),
        (
            b"MZ\0\0".to_vec(),
            "a DOS header beyond the end of the file".into(),
        ),
    ];
    for (bytes, reason) in cases {
        assert_eq!(inspected(&bytes).unwrap_err(), reason);
    }
}

#[test]
fn an_unknown_head_and_a_failing_device_are_refused_by_name() {
    assert_eq!(
        inspected(b"#!/bin/sh\n").unwrap_err(),
        "neither a #! script nor a native image"
    );
    assert_eq!(
        inspected(b"").unwrap_err(),
        "neither a #! script nor a native image"
    );
    assert_eq!(
        inspected(b"MZ").unwrap_err(),
        "neither a #! script nor a native image"
    );
    let dynamic = elf64(
        ET_DYN,
        ARCH.elf,
        &[Phdr {
            p_type: PT_LOAD,
            offset: 0,
            filesz: 64,
            memsz: 64,
        }],
        b"",
    );
    // A device that fails after the magic, and one that fails before it.
    let mut failing = Failing {
        bytes: dynamic.clone(),
        at: 0,
        ok: 4,
    };
    assert_eq!(
        inspect(&mut failing, dynamic.len() as u64).unwrap_err(),
        "an ELF header cannot be read: the device answered EIO"
    );
    let mut failing = Failing {
        bytes: dynamic.clone(),
        at: 0,
        ok: 0,
    };
    assert_eq!(
        inspect(&mut failing, dynamic.len() as u64).unwrap_err(),
        "the first four bytes cannot be read: the device answered EIO"
    );
    // A universal slice whose bytes the device will not answer.
    let image = fat(&[(ARCH.macho, macho(ARCH.macho, MH_EXECUTE, &[]))]);
    let mut failing = Failing {
        bytes: image.clone(),
        at: 0,
        ok: 28,
    };
    assert_eq!(
        inspect(&mut failing, image.len() as u64).unwrap_err(),
        "a universal slice cannot be read: the device answered EIO"
    );
    // The field readers answer `None` past the end and at an offset that
    // cannot be extended, rather than panicking.
    let bytes = Bytes(&[1, 2, 3]);
    assert_eq!(bytes.u16_le(0), Some(0x0201));
    assert_eq!(bytes.u16_le(2), None);
    assert_eq!(bytes.u16_le(usize::MAX), None);
    assert_eq!(bytes.u32_le(usize::MAX), None);
    assert_eq!(bytes.u64_le(usize::MAX), None);
    assert_eq!(bytes.u32_be(usize::MAX), None);
    assert_eq!(bytes.u32_be(0), None);
    assert_eq!(bytes.u64_le(0), None);
}
