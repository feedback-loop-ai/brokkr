use super::*;
use std::io::Cursor;

/// Inspect a synthetic image held in memory.
fn inspected(bytes: &[u8]) -> Result<Native, Unloadable> {
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
/// as a candidate and never execute it. Those tests are Unix-only.
#[cfg(unix)]
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
    // its loader is a real absolute path.
    let loader = native.loader.expect("a dynamic image names its loader");
    assert_eq!(loader[0], b'/', "{}", String::from_utf8_lossy(&loader));
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
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
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
    let cases: Vec<(Vec<u8>, Unloadable)> = vec![
        (
            elf64_with([3, ELF_DATA, 1], ET_DYN, ARCH.elf, &[load()], b""),
            Unloadable::ElfClass(3),
        ),
        (
            elf64_with([2, 3 - ELF_DATA, 1], ET_DYN, ARCH.elf, &[load()], b""),
            Unloadable::ElfByteOrder(3 - ELF_DATA),
        ),
        (
            elf64_with([2, ELF_DATA, 2], ET_DYN, ARCH.elf, &[load()], b""),
            Unloadable::ElfVersion(2),
        ),
        (elf64(1, ARCH.elf, &[load()], b""), Unloadable::ElfType(1)),
        (
            elf64(ET_DYN, ARCH.elf.wrapping_add(1), &[load()], b""),
            Unloadable::ElfMachine(ARCH.elf.wrapping_add(1)),
        ),
        (
            {
                let mut bytes = elf64(ET_DYN, ARCH.elf, &[load()], b"");
                bytes[54] = 55;
                bytes
            },
            Unloadable::ElfHeaderSize {
                found: 55,
                required: 56,
            },
        ),
        (elf64(ET_DYN, ARCH.elf, &[], b""), Unloadable::ElfNoHeaders),
        (
            {
                let mut bytes = elf64(ET_DYN, ARCH.elf, &[load()], b"");
                bytes[56..58].copy_from_slice(&2000u16.to_le_bytes());
                bytes
            },
            Unloadable::ElfTableBound(112_000),
        ),
        (
            {
                let mut bytes = elf64(ET_DYN, ARCH.elf, &[load()], b"");
                bytes[56..58].copy_from_slice(&2u16.to_le_bytes());
                bytes
            },
            Unloadable::Beyond(Region::ElfProgramHeaders),
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
            Unloadable::ElfSegmentSize {
                filesz: 65,
                memsz: 64,
            },
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
            Unloadable::Beyond(Region::ElfLoadSegment),
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
            Unloadable::Beyond(Region::ElfLoadSegment),
        ),
        (
            elf64(
                ET_DYN,
                ARCH.elf,
                &[interp_at(payload_at(2), 4), interp_at(payload_at(2), 4)],
                &path,
            ),
            Unloadable::ElfInterpreters,
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[interp_at(payload_at(1), 1)], &path),
            Unloadable::ElfInterpreterLength(1),
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[interp_at(payload_at(1), 4097)], &path),
            Unloadable::ElfInterpreterLength(4097),
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[interp_at(payload_at(1), 5)], &path),
            Unloadable::Beyond(Region::ElfInterpreter),
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[interp_at(payload_at(1), 3)], &path),
            Unloadable::ElfInterpreterUnterminated,
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[interp_at(payload_at(1), 3)], b"\0\0\0"),
            Unloadable::ElfInterpreterEmpty,
        ),
        (
            elf32(ET_EXEC, ARCH.elf, &[load()], b""),
            Unloadable::ElfForeignClass(1),
        ),
        (
            elf64(ET_DYN, ARCH.elf, &[load()], b"")[..60].to_vec(),
            Unloadable::Beyond(Region::ElfHeader),
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
        Unloadable::ElfSegmentSize {
            filesz: 9,
            memsz: 8
        }
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
        Unloadable::ElfForeignClass(1)
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

/// A well-formed Mach-O executable of this target's CPU type with no
/// load commands, for the parent module's tests that plant another
/// format's image as a loader, and never execute it.
pub(in crate::adapters::composite) fn synthetic_macho() -> Vec<u8> {
    macho(ARCH.macho, MH_EXECUTE, &[])
}

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
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn every_macho_rule_refuses_by_name() {
    let malformed =
        |command: Vec<u8>, reason: Unloadable| (macho(ARCH.macho, MH_EXECUTE, &[command]), reason);
    let cases: Vec<(Vec<u8>, Unloadable)> = vec![
        (
            vec![0xce, 0xfa, 0xed, 0xfe, 0, 0, 0, 0],
            Unloadable::MachO32,
        ),
        (
            vec![0xfe, 0xed, 0xfa, 0xce, 0, 0, 0, 0],
            Unloadable::MachOSwapped,
        ),
        (
            vec![0xfe, 0xed, 0xfa, 0xcf, 0, 0, 0, 0],
            Unloadable::MachOSwapped,
        ),
        (
            vec![0xca, 0xfe, 0xba, 0xbf, 0, 0, 0, 0],
            Unloadable::Universal64,
        ),
        (
            macho(ARCH.macho.wrapping_add(1), MH_EXECUTE, &[]),
            Unloadable::MachOCpu(ARCH.macho.wrapping_add(1)),
        ),
        (macho(ARCH.macho, 1, &[]), Unloadable::MachOFileType(1)),
        (
            {
                let mut bytes = macho(ARCH.macho, MH_EXECUTE, &[]);
                bytes[20..24].copy_from_slice(&(MACHO_COMMANDS_BOUND + 1).to_le_bytes());
                bytes
            },
            Unloadable::MachOTableBound(MACHO_COMMANDS_BOUND + 1),
        ),
        (
            {
                let mut bytes = macho(ARCH.macho, MH_EXECUTE, &[]);
                bytes[20..24].copy_from_slice(&8u32.to_le_bytes());
                bytes
            },
            Unloadable::Beyond(Region::MachOCommands),
        ),
        (
            {
                let mut bytes = macho(ARCH.macho, MH_EXECUTE, &[segment()]);
                bytes[16..20].copy_from_slice(&2u32.to_le_bytes());
                bytes
            },
            Unloadable::MachOCommandBeyond,
        ),
        malformed(
            {
                let mut command = segment();
                command[4..8].copy_from_slice(&4u32.to_le_bytes());
                command
            },
            Unloadable::MachOCommand,
        ),
        malformed(
            {
                let mut command = segment();
                command[4..8].copy_from_slice(&80u32.to_le_bytes());
                command
            },
            Unloadable::MachOCommand,
        ),
        (
            macho(
                ARCH.macho,
                MH_EXECUTE,
                &[dylinker(b"/usr/lib/dyld"), dylinker(b"/usr/lib/dyld")],
            ),
            Unloadable::MachODylinkers,
        ),
        malformed(
            {
                let mut command = dylinker(b"/usr/lib/dyld");
                command[8..12].copy_from_slice(&11u32.to_le_bytes());
                command
            },
            Unloadable::MachODylinker,
        ),
        // `cmdsize` 8 on the dynamic-linker command: the generic bound
        // admits it, and the offset field at 8 is not there to read. This
        // 40-byte image made the reader panic (review 2026-09-20, R4).
        malformed(
            {
                let mut command = dylinker(b"/usr/lib/dyld");
                command.truncate(8);
                command[4..8].copy_from_slice(&8u32.to_le_bytes());
                command
            },
            Unloadable::MachODylinker,
        ),
        malformed(
            {
                let mut command = dylinker(b"/usr/lib/dyld");
                let size = command.len() as u32;
                command[8..12].copy_from_slice(&size.to_le_bytes());
                command
            },
            Unloadable::MachODylinker,
        ),
        malformed(dylinker(b""), Unloadable::MachODylinkerEmpty),
        (
            macho(ARCH.macho, MH_EXECUTE, &[])[..20].to_vec(),
            Unloadable::Beyond(Region::MachOHeader),
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
    let cases: Vec<(Vec<u8>, Unloadable)> = vec![
        (fat(&[]), Unloadable::UniversalCount(0)),
        (
            {
                let mut bytes = fat(&[(ARCH.macho, ours.clone())]);
                bytes[4..8].copy_from_slice(&65u32.to_be_bytes());
                bytes
            },
            Unloadable::UniversalCount(65),
        ),
        (
            fat(&[(ARCH.macho, ours.clone())])[..20].to_vec(),
            Unloadable::Beyond(Region::UniversalTable),
        ),
        (
            fat(&[(ARCH.macho.wrapping_add(1), other.clone())]),
            Unloadable::UniversalNoSlice,
        ),
        (
            {
                let mut bytes = fat(&[(ARCH.macho, ours.clone())]);
                bytes[20..24].copy_from_slice(&u32::MAX.to_be_bytes());
                bytes
            },
            Unloadable::Beyond(Region::UniversalSlice),
        ),
        (
            fat(&[(ARCH.macho, fat(&[(ARCH.macho, ours.clone())]))]),
            Unloadable::UniversalSliceNotMachO,
        ),
        (
            fat(&[(ARCH.macho, ours[..20].to_vec())]),
            Unloadable::Beyond(Region::MachOHeader),
        ),
        (
            fat(&[(ARCH.macho, vec![0; 4])]),
            Unloadable::UniversalSliceNotMachO,
        ),
    ];
    for (bytes, reason) in cases {
        assert_eq!(inspected(&bytes).unwrap_err(), reason, "{bytes:?}");
    }
}

#[test]
fn an_unknown_head_and_a_failing_device_are_refused_by_name() {
    assert_eq!(inspected(b"#!/bin/sh\n").unwrap_err(), Unloadable::Unknown);
    assert_eq!(inspected(b"").unwrap_err(), Unloadable::Unknown);
    assert_eq!(inspected(b"MZ").unwrap_err(), Unloadable::Unknown);
    // A PE's magic is not a native image's: no supported host loads one
    // (decision 0063).
    assert_eq!(inspected(b"MZ\x90\0").unwrap_err(), Unloadable::Unknown);
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
    let eio = |region| Unloadable::Unread {
        region,
        cause: "the device answered EIO".to_string(),
    };
    assert_eq!(
        inspect(&mut failing, dynamic.len() as u64).unwrap_err(),
        eio(Region::ElfHeader)
    );
    let mut failing = Failing {
        bytes: dynamic.clone(),
        at: 0,
        ok: 0,
    };
    assert_eq!(
        inspect(&mut failing, dynamic.len() as u64).unwrap_err(),
        eio(Region::Magic)
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
        eio(Region::UniversalSlice)
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

/// The one place each refusal's bytes are pinned (#353): the resolver
/// folds them into its own refusal, so they are what an operator reads.
#[test]
fn every_unloadable_image_reads_as_it_always_has() {
    let regions = [
        (Region::Magic, "the first four bytes"),
        (Region::ElfHeader, "an ELF header"),
        (Region::ElfProgramHeaders, "an ELF program header table"),
        (Region::ElfLoadSegment, "an ELF load segment"),
        (Region::ElfInterpreter, "an ELF interpreter path"),
        (Region::UniversalHeader, "a universal header"),
        (Region::UniversalTable, "a universal architecture table"),
        (Region::UniversalSlice, "a universal slice"),
        (Region::MachOHeader, "a Mach-O header"),
        (Region::MachOCommands, "a Mach-O load-command table"),
    ];
    for (region, spelled) in regions {
        assert_eq!(
            Unloadable::Beyond(region).to_string(),
            format!("{spelled} beyond the end of the file")
        );
    }
    for (refusal, text) in elf_pins().into_iter().chain(macho_pins()) {
        assert_eq!(refusal.to_string(), text);
    }
}

/// The head's and the ELF reader's refusals, each with its bytes.
fn elf_pins() -> Vec<(Unloadable, String)> {
    let elf = ARCH.elf;
    vec![
        (
            Unloadable::Unread {
                region: Region::ElfHeader,
                cause: "EIO".into(),
            },
            "an ELF header cannot be read: EIO".to_string(),
        ),
        (
            Unloadable::MachO32,
            "an unsupported 32-bit Mach-O image".into(),
        ),
        (
            Unloadable::MachOSwapped,
            "an unsupported byte-swapped Mach-O image".into(),
        ),
        (
            Unloadable::Universal64,
            "an unsupported 64-bit universal image".into(),
        ),
        (
            Unloadable::Unknown,
            "neither a #! script nor a native image".into(),
        ),
        (Unloadable::ElfClass(3), "an unsupported ELF class 3".into()),
        (
            Unloadable::ElfByteOrder(2),
            "an unsupported ELF byte order 2".into(),
        ),
        (
            Unloadable::ElfVersion(2),
            "an unsupported ELF version 2".into(),
        ),
        (
            Unloadable::ElfType(1),
            "ELF type 1, which is neither an executable nor a shared object".into(),
        ),
        (
            Unloadable::ElfMachine(7),
            format!("ELF machine 7, which is not this target's {elf}"),
        ),
        (
            Unloadable::ElfHeaderSize {
                found: 55,
                required: 56,
            },
            "an ELF program header size 55 where 56 is required".into(),
        ),
        (Unloadable::ElfNoHeaders, "no ELF program headers".into()),
        (
            Unloadable::ElfTableBound(112_000),
            "an ELF program header table of 112000 bytes, above the 65536-byte bound".into(),
        ),
        (
            Unloadable::ElfSegmentSize {
                filesz: 65,
                memsz: 64,
            },
            "an ELF load segment whose file size 65 exceeds its memory size 64".into(),
        ),
        (
            Unloadable::ElfInterpreters,
            "more than one ELF interpreter".into(),
        ),
        (
            Unloadable::ElfInterpreterLength(1),
            "an ELF interpreter path of 1 bytes".into(),
        ),
        (
            Unloadable::ElfInterpreterUnterminated,
            "an ELF interpreter path that is not NUL-terminated".into(),
        ),
        (
            Unloadable::ElfInterpreterEmpty,
            "an empty ELF interpreter path".into(),
        ),
        (
            Unloadable::ElfForeignClass(1),
            "ELF class 1, which is not this target's".into(),
        ),
    ]
}

/// The Mach-O readers' refusals, universal and thin, each with its bytes.
fn macho_pins() -> Vec<(Unloadable, String)> {
    let cpu = ARCH.macho;
    vec![
        (
            Unloadable::UniversalCount(65),
            "a universal image with 65 architectures".into(),
        ),
        (
            Unloadable::UniversalSliceNotMachO,
            "a universal slice that is not a 64-bit Mach-O image".into(),
        ),
        (
            Unloadable::UniversalNoSlice,
            format!("a universal image with no slice for this target's CPU type {cpu}"),
        ),
        (
            Unloadable::MachOCpu(8),
            format!("Mach-O CPU type 8, which is not this target's {cpu}"),
        ),
        (
            Unloadable::MachOFileType(1),
            "Mach-O file type 1, which is neither an executable nor a dynamic linker".into(),
        ),
        (
            Unloadable::MachOTableBound(1_048_577),
            "a Mach-O load-command table of 1048577 bytes, above the 1048576-byte bound".into(),
        ),
        (
            Unloadable::MachOCommandBeyond,
            "a Mach-O load command beyond its table".into(),
        ),
        (
            Unloadable::MachOCommand,
            "a malformed Mach-O load command".into(),
        ),
        (
            Unloadable::MachODylinkers,
            "more than one Mach-O dynamic linker".into(),
        ),
        (
            Unloadable::MachODylinker,
            "a malformed Mach-O dynamic linker command".into(),
        ),
        (
            Unloadable::MachODylinkerEmpty,
            "an empty Mach-O dynamic linker path".into(),
        ),
    ]
}
