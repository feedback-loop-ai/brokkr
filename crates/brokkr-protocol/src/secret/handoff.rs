//! The confined environment handoff (decision 0065 slice two, U6c8; MB3,
//! MB4): the broker's sender and the server box's receiver of the one
//! binding frame, inside the secret boundary. The sender binds through the
//! parent's one injector, [`bind_environment`], and serialises that
//! prepared environment; the receiver builds the server's final
//! environment and execs it. Neither reads a value but through the
//! parent's own types.

use std::ffi::{CStr, CString, OsString};
use std::fmt;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use super::{bind_environment, store, wipe, BoundSecret, Secret};
use crate::broker::Refusal;
use crate::native_controls::bounded_line;

/// The most bytes the binding frame's body holds (MB4).
pub(super) const HANDOFF_MAX: usize = 1 << 20;

/// The most bindings one frame carries (MB4).
pub(super) const BINDINGS_MAX: usize = 256;

/// Look the declared `names` up in the store the admitted path handle
/// `store` holds, reopened through that very handle, which `path` only
/// names in a cause; bind them with [`bind_environment`] on a cleared
/// command that is never run; and hand `send` that command's environment as
/// one binding frame: a four-byte big-endian length, then one JSON object
/// of names to values. No name, no lookup; off Linux, where no store can
/// be reopened through its handle, a name is the store's read cause. A
/// store or binding cause is
/// decision 0012's own; a frame past MB4's bounds is the handoff cause, as
/// is whatever `send` answers. The frame is reserved whole before a byte is
/// written, and wiped once sent. Its caller bounds the whole of it, the
/// store's open and read included, by the box's startup. The bindings it
/// looked up are handed back beside the answer, whether or not anything was
/// sent, so the caller's final guard holds them from the moment they are
/// known (MB4, CR2; operator ruling 2026-10-13); none where the lookup
/// itself refused.
pub fn send_environment(
    store: std::os::fd::BorrowedFd<'_>,
    path: &Path,
    names: &[String],
    send: impl FnOnce(&[u8]) -> Result<(), Refusal>,
) -> (Vec<BoundSecret>, Result<(), Refusal>) {
    let io = |source| store::StoreError::Io {
        path: path.to_path_buf(),
        source,
    };
    let bindings = match names.is_empty() {
        true => Ok(Vec::new()),
        false => reopened(store)
            .map_err(io)
            .and_then(|file| store::read_store_file(file, path))
            .and_then(|entries| store::bind_names(entries, names, path))
            .map_err(|error| named(&error)),
    };
    match bindings {
        Ok(bindings) => {
            let sent = framed(&bindings, send);
            (bindings, sent)
        }
        Err(refusal) => (Vec::new(), Err(refusal)),
    }
}

/// The binding cause `error` reads as.
fn named(error: &dyn fmt::Display) -> Refusal {
    Refusal::Name(bounded_line(&error.to_string()))
}

/// Bind `bindings` with [`bind_environment`] on a cleared command that is
/// never run, and hand `send` that command's environment as one binding
/// frame ([`send_environment`]).
fn framed(
    bindings: &[BoundSecret],
    send: impl FnOnce(&[u8]) -> Result<(), Refusal>,
) -> Result<(), Refusal> {
    let mut prepared = std::process::Command::new("");
    prepared.env_clear();
    bind_environment(&mut prepared, bindings).map_err(|error| named(&error))?;
    // The cleared command holds only what the injector set, each a UTF-8
    // name and value, so no entry is skipped here; one that were would
    // leave the frame short of a binding, which refuses below rather than
    // send it (bind_environment's own refusal of a non-UTF-8 value is
    // pinned by the sender's invalid-value test).
    let environment: std::collections::BTreeMap<&str, &str> = prepared
        .get_envs()
        .filter_map(|(name, value)| Some((name.to_str()?, value?.to_str()?)))
        .collect();
    let mut frame = Vec::with_capacity(4 + HANDOFF_MAX);
    frame.extend_from_slice(&[0; 4]);
    let written = serde_json::to_writer(&mut frame, &environment);
    let length = u32::try_from(frame.len() - 4).unwrap_or(u32::MAX);
    frame[..4].copy_from_slice(&length.to_be_bytes());
    let whole = environment.len() == bindings.len();
    let bounded = (length as usize <= HANDOFF_MAX) & (environment.len() <= BINDINGS_MAX);
    let sent = match written.is_ok() & bounded & whole {
        true => send(&frame),
        false => Err(Refusal::Handoff),
    };
    wipe(&mut frame);
    sent
}

/// The store the admitted path handle `store` holds, opened for reading
/// through that very handle, by the name Linux's `/proc` gives it, and
/// without waiting: a FIFO, terminal or other special file whose open or
/// read would wait on a peer answers at once, its read cause, so no peer
/// can hold the sender past the startup.
#[cfg(target_os = "linux")]
fn reopened(store: std::os::fd::BorrowedFd<'_>) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    let handle = format!("/proc/self/fd/{}", std::os::fd::AsRawFd::as_raw_fd(&store));
    let mut opening = std::fs::OpenOptions::new();
    opening
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(handle)
}

/// Off Linux no `/proc` names a descriptor, and no box stands to receive
/// a frame: no store is read.
#[cfg(not(target_os = "linux"))]
fn reopened(_: std::os::fd::BorrowedFd<'_>) -> std::io::Result<std::fs::File> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// The waiting bootstrap's last step: receive the one binding frame from
/// `from` and then its end, its names exactly the declared `names`, none
/// twice, no NUL and no value past MB4's bounds; build the server's final
/// environment, the `fixed` one this process was started with and the
/// bindings, which may not replace a fixed name; run `prepare` and then
/// mark every descriptor above the standard streams close-on-exec
/// ([`sealing`]); and exec the sealed `server` (its executable and argv)
/// directly, never through a shell, so the kernel alone judges a script's
/// interpreter. It returns only where the server did not start: the
/// handoff cause where the frame was not received whole, the boxed-exec
/// cause after.
pub fn exec_confined(
    from: impl Read,
    names: &[String],
    fixed: Vec<(OsString, OsString)>,
    server: (&Path, &[String]),
    prepare: impl FnOnce() -> bool,
) -> Refusal {
    let prepared = sealing(prepare, close_on_exec_above_stdio);
    exec_confined_with(from, names, fixed, server, prepared, execve)
}

/// `prepare`, and then `seal`, which marks every descriptor above the
/// standard streams close-on-exec: prepared only where both held. A seal
/// that failed may have left any descriptor open across the exec, the
/// status pipe's among them, so it refuses; nothing falls back to listing
/// the open descriptors, which could miss one.
pub(super) fn sealing(
    prepare: impl FnOnce() -> bool,
    seal: impl FnOnce() -> bool,
) -> impl FnOnce() -> bool {
    move || prepare() && seal()
}

/// Whether every descriptor from 3 up, whichever are open, was marked
/// close-on-exec by one `close_range(2)` over all of them, which no
/// descriptor opened or numbered meanwhile escapes. Async-signal-safe: one
/// system call. A kernel without its close-on-exec form (before 5.11)
/// refuses it.
#[cfg(target_os = "linux")]
pub(crate) fn close_on_exec_above_stdio() -> bool {
    // SAFETY: `close_range` sets one flag of every descriptor from 3 up,
    // whichever are open; it reads and writes no memory.
    let sealed = unsafe {
        libc::syscall(
            libc::SYS_close_range,
            3,
            libc::c_uint::MAX,
            libc::CLOSE_RANGE_CLOEXEC,
        )
    };
    sealed == 0
}

/// Off Linux no box stands, so no server is started.
#[cfg(not(target_os = "linux"))]
pub(crate) fn close_on_exec_above_stdio() -> bool {
    false
}

/// [`exec_confined`], with `exec` standing for the exec.
pub(super) fn exec_confined_with(
    from: impl Read,
    names: &[String],
    fixed: Vec<(OsString, OsString)>,
    server: (&Path, &[String]),
    prepare: impl FnOnce() -> bool,
    exec: impl FnOnce(&CStr, &[&CStr], &[&CStr]),
) -> Refusal {
    let environment = received(from, names).and_then(|bindings| confined(fixed, &bindings));
    let Ok(environment) = environment else {
        return Refusal::Handoff;
    };
    let refusal = started(&environment, server, prepare, exec);
    for entry in environment {
        wipe(&mut entry.into_bytes_with_nul());
    }
    refusal
}

/// The bindings of the one complete frame on `from`, which must then end.
fn received(mut from: impl Read, names: &[String]) -> Result<Vec<(String, Secret)>, Refusal> {
    let mut length = [0; 4];
    from.read_exact(&mut length).or(Err(Refusal::Handoff))?;
    let length = usize::try_from(u32::from_be_bytes(length)).unwrap_or(usize::MAX);
    if length > HANDOFF_MAX {
        return Err(Refusal::Handoff);
    }
    let mut body = vec![0; length];
    let mut rest = Vec::new();
    let read = from.read_exact(&mut body);
    let ended = read.and_then(|()| from.by_ref().take(1).read_to_end(&mut rest));
    let bindings = match ended {
        Ok(0) => decoded(&body, names),
        _ => Err(Refusal::Handoff),
    };
    wipe(&mut body);
    bindings
}

/// The frame's bindings, where their names are exactly the declared ones,
/// which hold no NUL.
fn decoded(body: &[u8], names: &[String]) -> Result<Vec<(String, Secret)>, Refusal> {
    let Bindings(bindings) = serde_json::from_slice(body).or(Err(Refusal::Handoff))?;
    let mut sent: Vec<&str> = bindings.iter().map(|(name, _)| name.as_str()).collect();
    let mut declared: Vec<&str> = names.iter().map(String::as_str).collect();
    sent.sort_unstable();
    declared.sort_unstable();
    (sent == declared)
        .then_some(bindings)
        .ok_or(Refusal::Handoff)
}

/// The final environment: each `fixed` entry, then each binding, as
/// `NAME=value`; none where a binding would replace a fixed name, or a
/// value holds NUL, which no entry can carry. An entry refused is wiped.
fn confined(
    fixed: Vec<(OsString, OsString)>,
    bindings: &[(String, Secret)],
) -> Result<Vec<CString>, Refusal> {
    let replaced = bindings
        .iter()
        .any(|(name, _)| fixed.iter().any(|(key, _)| key == name.as_str()));
    if replaced {
        return Err(Refusal::Handoff);
    }
    let entry = |name: &[u8], value: &[u8]| {
        let mut entry = Vec::with_capacity(name.len() + value.len() + 2);
        entry.extend_from_slice(name);
        entry.push(b'=');
        entry.extend_from_slice(value);
        CString::new(entry).map_err(|refused| {
            wipe(&mut refused.into_vec());
            Refusal::Handoff
        })
    };
    let fixed = fixed
        .iter()
        .map(|(key, value)| entry(key.as_bytes(), value.as_bytes()));
    let bound = bindings
        .iter()
        .map(|(name, value)| entry(name.as_bytes(), &value.bytes));
    fixed.chain(bound).collect()
}

/// Exec `executable` with `argv` and `environment` once `prepare` holds;
/// the boxed-exec cause where it cannot, or the exec returns.
fn started(
    environment: &[CString],
    (executable, argv): (&Path, &[String]),
    prepare: impl FnOnce() -> bool,
    exec: impl FnOnce(&CStr, &[&CStr], &[&CStr]),
) -> Refusal {
    let program = CString::new(executable.as_os_str().as_bytes());
    let argv: Result<Vec<CString>, _> = argv
        .iter()
        .map(|arg| CString::new(arg.as_bytes()))
        .collect();
    let (Ok(program), Ok(argv)) = (program, argv) else {
        return Refusal::Exec;
    };
    if argv.is_empty() || !prepare() {
        return Refusal::Exec;
    }
    let argv: Vec<&CStr> = argv.iter().map(CString::as_c_str).collect();
    let environment: Vec<&CStr> = environment.iter().map(CString::as_c_str).collect();
    exec(&program, &argv, &environment);
    Refusal::Exec
}

/// `execve(2)` itself: no `PATH` search and no shell for a file the kernel
/// will not run, as `execvp` would try. It returns only where it failed.
fn execve(program: &CStr, argv: &[&CStr], environment: &[&CStr]) {
    let pointers = |strings: &[&CStr]| -> Vec<*const libc::c_char> {
        let pointers = strings.iter().map(|string| string.as_ptr());
        pointers.chain([std::ptr::null()]).collect()
    };
    let (argv, environment) = (pointers(argv), pointers(environment));
    // SAFETY: each pointer is a NUL-terminated string borrowed for this
    // call, and both arrays end with a null pointer, as execve(2) requires.
    unsafe { libc::execve(program.as_ptr(), argv.as_ptr(), environment.as_ptr()) };
}

/// One binding frame's object, decoded in order, refused where a name
/// repeats or past [`BINDINGS_MAX`] names; each value a [`Secret`].
struct Bindings(Vec<(String, Secret)>);

impl<'de> serde::Deserialize<'de> for Bindings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Bindings, D::Error> {
        deserializer.deserialize_map(Bindings(Vec::new()))
    }
}

impl<'de> serde::de::Visitor<'de> for Bindings {
    type Value = Bindings;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("one object of binding names and values")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(mut self, mut map: A) -> Result<Bindings, A::Error> {
        while let Some((name, value)) = map.next_entry::<String, String>()? {
            let repeated = self.0.iter().any(|(seen, _)| *seen == name);
            if repeated | (self.0.len() == BINDINGS_MAX) {
                return Err(serde::de::Error::custom("not one binding frame"));
            }
            self.0.push((name, Secret::new(value.into_bytes())));
        }
        Ok(self)
    }
}
