//! The confined environment handoff through `serve` (decision 0065 slice
//! two U6c8; MB3, MB4, MB5, SD3), on a real box: once the box is ready the
//! store is read, its declared bindings cross to the bootstrap as one
//! frame, and the bootstrap execs the sealed server directly, then the box
//! is ended and serving still refuses before any MCP message. A tracer
//! records each exec of the run, so the server's exact environment and
//! every other process's argv and environment are observed at the exec
//! itself, by no shell's account; or holds the server's exec at its entry,
//! so the descriptors it inherits are read from the host.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use brokkr_protocol::secret::NEEDLE_ENCODINGS;
use serde_json::json;

use super::ledger::{failed_with, sealed_ledger};
use super::observer::NO_TRACER;
use super::observer::{awaited, processes, started_to, status, traced, tracer, within, Starter};
use super::readiness::{bootstraps, boxed, Reads};
use super::{admitted, refused, skip, unservable, Refusal, Sealed, TOKEN};

/// A sealed fixture whose server is `script`, its store holding `store`
/// and its plan declaring `names`; the plan's digest, and the tracer. None,
/// declared skipped, where no box stands or no tracer traces.
fn serving(
    script: impl AsRef<[u8]>,
    store: &[u8],
    names: &[&str],
) -> Option<(Sealed, String, PathBuf)> {
    if let Some(reason) = unservable() {
        skip(&reason);
        return None;
    }
    let Some(tracer) = tracer() else {
        skip(NO_TRACER);
        return None;
    };
    let sealed = Sealed::new();
    std::fs::write(sealed.path("opt/docs/bin/docs-mcp"), script).unwrap();
    std::fs::write(sealed.path("store/secrets.env"), store).unwrap();
    let mut plan = sealed.plan();
    plan["secrets"] = json!(names);
    let digest = sealed.seal_bytes(plan.to_string().as_bytes());
    Some((sealed, digest, tracer))
}

/// The server's path, as the plan seals its argv and executable.
fn server(sealed: &Sealed) -> String {
    sealed.path("opt/docs/bin/docs-mcp").display().to_string()
}

/// The tracer's arguments when it records each exec of the run whole:
/// every process followed, each to its own file so no call is split
/// around another's, stopped by a seccomp filter at `execve` alone, no
/// signal written, every string and environment in full, to files named
/// by the prefix next.
const EXECS: [&str; 11] = [
    "-ff",
    "--seccomp-bpf",
    "-qqq",
    "-v",
    "-s",
    "1048576",
    "-e",
    "trace=execve",
    "-e",
    "signal=none",
    "-o",
];

/// The lines of each file a tracer told `-ff -o <prefix>` wrote, one
/// process's each.
fn traces(prefix: &Path) -> Vec<Vec<String>> {
    let name = format!("{}.", prefix.file_name().unwrap().to_str().unwrap());
    let written = std::fs::read_dir(prefix.parent().unwrap()).unwrap();
    let ours = written.map(|entry| entry.unwrap().path()).filter(|path| {
        path.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with(&name)
    });
    let read = ours.map(|path| std::fs::read_to_string(path).unwrap());
    read.map(|text| text.lines().map(str::to_string).collect())
        .collect()
}

/// The broker's exit and its own stderr for the plan sealed at `digest`,
/// started as a harness starts it, all of it run by `tracer` given
/// `tracing`.
fn traced_serve(
    sealed: &Sealed,
    digest: &str,
    tracer: &Path,
    tracing: &[&OsStr],
) -> (Option<i32>, String) {
    let apart = sealed.path("broker.err");
    let command = traced(tracer, tracing);
    let mut command = started_to(command, sealed, digest, (Starter::Harness, Some(&apart)));
    let ran = command.stderr(Stdio::null()).status().unwrap();
    (ran.code(), std::fs::read_to_string(apart).unwrap())
}

/// The broker's answer for `sealed`'s plan at `digest` with every exec of
/// the run recorded, and each recorded exec's line.
fn recorded(sealed: &Sealed, digest: &str, tracer: &Path) -> ((Option<i32>, String), Vec<String>) {
    let log = sealed.path("execs.log");
    let tracing: Vec<&OsStr> = EXECS
        .iter()
        .map(OsStr::new)
        .chain([log.as_os_str()])
        .collect();
    let answer = traced_serve(sealed, digest, tracer, &tracing);
    let execs = traces(&log).into_iter().flatten();
    (
        answer,
        execs.filter(|line| line.starts_with("execve(")).collect(),
    )
}

/// The recorded execs of `server`.
fn of<'l>(execs: &'l [String], server: &str) -> Vec<&'l str> {
    let call = format!("execve(\"{server}\", ");
    let calls = execs.iter().filter(|line| line.starts_with(&call));
    calls.map(String::as_str).collect()
}

/// The environment an exec's recorded `call` passed, entry by entry.
fn environment(call: &str) -> Vec<String> {
    let entries = call.split_once("], [").unwrap().1;
    let entries = entries.rsplit_once("]) = ").unwrap().0;
    let entries = entries.trim_matches('"').split("\", \"");
    entries.map(str::to_string).collect()
}

/// MB4's fixed entries, each with the exact value the box's launcher sets:
/// pinned here, so a wrong value fails as a missing name would.
const CONFINED: [&str; 8] = [
    "PATH=/usr/local/bin:/usr/bin:/bin",
    "HOME=/runtime/home",
    "TMPDIR=/tmp",
    "USER=runner",
    "LOGNAME=runner",
    "LANG=C.UTF-8",
    "LC_ALL=C.UTF-8",
    "BROKKR_HANDS_BOX=1",
];

/// The server's whole confined environment: [`CONFINED`] and `bindings`,
/// sorted.
fn confined(bindings: &[String]) -> Vec<String> {
    let mut whole: Vec<String> = CONFINED.map(String::from).to_vec();
    whole.extend_from_slice(bindings);
    whole.sort();
    whole
}

/// Whether `text` holds the canary in any encoding decision 0012 lists.
fn leaks(text: &str) -> bool {
    NEEDLE_ENCODINGS.iter().any(|(_, encode)| {
        let needle = String::from_utf8(encode(TOKEN.as_bytes())).unwrap();
        text.contains(&needle)
    })
}

#[test]
fn a_protected_installed_server_alone_receives_its_bindings_beside_its_fixed_environment() {
    let store = format!("DOCS_TOKEN={TOKEN}\nPYTHONUSERBASE=/seat/site\nUNDECLARED=x\n");
    let names = ["DOCS_TOKEN", "PYTHONUSERBASE"];
    let Some((sealed, digest, tracer)) = serving("#!/bin/sh\n:\n", store.as_bytes(), &names) else {
        return;
    };
    let (answer, execs) = recorded(&sealed, &digest, &tracer);
    assert_eq!(answer, admitted());
    // One exec of the server, the kernel running its valid shebang inside
    // the box, with the sealed argv and exactly MB4's fixed entries and the
    // declared bindings: no undeclared or ambient name.
    let server = server(&sealed);
    let started = of(&execs, &server);
    let argv = format!("execve(\"{server}\", [\"{server}\", \"--stdio\"], [");
    assert_eq!(started.len(), 1, "{execs:#?}");
    assert!(
        started[0].starts_with(&argv) & started[0].ends_with("]) = 0"),
        "{started:?}"
    );
    // Every entry whole, name and value.
    let mut passed = environment(started[0]);
    passed.sort();
    let bindings = [
        format!("DOCS_TOKEN={TOKEN}"),
        "PYTHONUSERBASE=/seat/site".into(),
    ];
    assert_eq!(passed, confined(&bindings));
    // No other exec of the run, the host launcher's and the bootstrap's
    // included, carries the value in its argv or environment, in any listed
    // encoding, nor a loading binding's name; nor does the server's argv,
    // nor the broker's stderr.
    let others: Vec<&String> = execs.iter().filter(|line| !line.contains(&argv)).collect();
    assert!(others.len() > 4, "{execs:#?}");
    let exposed = |line: &&String| leaks(line) | line.contains("PYTHONUSERBASE");
    assert_eq!(
        others.into_iter().filter(exposed).collect::<Vec<_>>(),
        Vec::<&String>::new()
    );
    assert!(!leaks(started[0].split_once("], [").unwrap().0));
    assert!(!leaks(&answer.1));
    assert!(within(|| boxed(&sealed).is_empty()));
}

#[test]
fn a_handoff_that_fails_after_the_lookup_starts_no_server() {
    // A name the store lacks and a value no environment holds are decision
    // 0012's own causes; a value with NUL the bootstrap refuses after the
    // whole frame, and one past the frame's bound the broker never sends:
    // each the handoff's cause. None starts the server.
    let cause = |case: &str, sealed: &Sealed| match case {
        "missing" => Refusal::Name(format!(
            "secret 'DOCS_TOKEN' is not in the store at {} (brokkr secrets set DOCS_TOKEN)",
            sealed.path("store/secrets.env").display()
        )),
        "not UTF-8" => Refusal::Name("secret 'DOCS_TOKEN' is not valid UTF-8".into()),
        _ => Refusal::Handoff,
    };
    let over = format!("DOCS_TOKEN={}\n", "v".repeat(1 << 20));
    let cases: [(&str, &[u8]); 4] = [
        ("missing", b"OTHER=value\n"),
        ("not UTF-8", b"DOCS_TOKEN=\xff\xfe\n"),
        ("NUL", b"DOCS_TOKEN=ab\0cd\n"),
        ("over", over.as_bytes()),
    ];
    for (case, store) in cases {
        let Some((sealed, digest, tracer)) = serving("#!/bin/sh\n:\n", store, &["DOCS_TOKEN"])
        else {
            return;
        };
        let (answer, execs) = recorded(&sealed, &digest, &tracer);
        assert_eq!((case, answer), (case, refused(cause(case, &sealed))));
        assert_eq!(
            (case, of(&execs, &server(&sealed))),
            (case, Vec::<&str>::new())
        );
        let closed = failed_with(cause(case, &sealed));
        assert_eq!((case, sealed_ledger(&sealed)), (case, closed));
        assert!(within(|| boxed(&sealed).is_empty()), "{case}");
    }
}

/// The host's `/bin/sh`, a dynamically linked ELF, its program interpreter
/// (`PT_INTERP`, the loader) renamed to one no box holds.
fn loaderless() -> Vec<u8> {
    let mut elf = std::fs::read("/bin/sh").unwrap();
    let field = |at: usize, size: usize| {
        let bytes = &elf[at..at + size];
        bytes
            .iter()
            .rev()
            .fold(0, |value, byte| (value << 8) | usize::from(*byte))
    };
    assert_eq!(&elf[..6], b"\x7fELF\x02\x01", "a 64-bit little-endian ELF");
    let (table, size, count) = (field(0x20, 8), field(0x36, 2), field(0x38, 2));
    let interp = (0..count)
        .map(|at| table + at * size)
        .find(|at| field(*at, 4) == 3);
    let header = interp.expect("/bin/sh names a loader");
    let (offset, length) = (field(header + 8, 8), field(header + 32, 8));
    let absent = b"/nonexistent/ld.so";
    assert!(absent.len() < length);
    elf[offset..offset + length].fill(0);
    elf[offset..offset + absent.len()].copy_from_slice(absent);
    elf
}

#[test]
fn the_kernel_alone_runs_a_script_in_the_box_and_nothing_retries_one_it_refuses() {
    // An interpreter absent from the box, one only the host holds, a
    // script with no interpreter line, and a program whose loader no box
    // holds: each exec fails inside the box, the exec's own cause, and
    // nothing runs the server another way.
    let host = Sealed::new();
    let interpreter = host.path("interpreter");
    std::fs::copy("/bin/sh", &interpreter).unwrap();
    let cases = [
        (
            "absent",
            b"#!/nonexistent/interpreter\n:\n".to_vec(),
            "ENOENT",
        ),
        (
            "host",
            format!("#!{}\n:\n", interpreter.display()).into_bytes(),
            "ENOENT",
        ),
        ("no shebang", b":\n".to_vec(), "ENOEXEC"),
        ("no loader", loaderless(), "ENOENT"),
    ];
    for (case, script, errno) in cases {
        let Some((sealed, digest, tracer)) = serving(&script, b"", &[]) else {
            return;
        };
        let (answer, execs) = recorded(&sealed, &digest, &tracer);
        assert_eq!((case, answer), (case, refused(Refusal::Exec)));
        let server = server(&sealed);
        let tried = of(&execs, &server);
        assert_eq!((case, tried.len()), (case, 1), "{execs:#?}");
        assert!(
            tried[0].contains(&format!("= -1 {errno} ")),
            "{case}: {tried:?}"
        );
        let retried = execs
            .iter()
            .filter(|line| line.contains(&format!("\"{server}\"")));
        assert_eq!((case, retried.count()), (case, 1), "{execs:#?}");
        let closed = failed_with(Refusal::Exec);
        assert_eq!((case, sealed_ledger(&sealed)), (case, closed));
        assert!(within(|| boxed(&sealed).is_empty()), "{case}");
    }
}

/// The tracer's arguments when it holds the server just after its exec and
/// the broker at the box's end: every process followed, stopped by a
/// seccomp filter at `execve` and `kill` alone, nothing written; each
/// process's third exec (in the box: `env`, the bootstrap, the server)
/// held ten seconds as it returns, its new image loaded and every
/// close-on-exec descriptor closed, before the server runs an instruction;
/// and each process's second `kill` (the broker's: its observer's group,
/// then the box's launcher) skipped and held as long, so the box stands.
/// The tracer counts each process's calls apart.
const HOLD_SERVER: [&str; 11] = [
    "-f",
    "--seccomp-bpf",
    "-qqq",
    "-o",
    "/dev/null",
    "-e",
    "trace=execve,kill",
    "-e",
    "inject=execve:delay_exit=10000000:when=3",
    "-e",
    "inject=kill:retval=0:delay_exit=10000000:when=2",
];

/// Whether `pid` is held by its tracer, or stopped.
fn halted(pid: i32) -> bool {
    status(pid).is_some_and(|(state, _, _)| "tT".contains(state))
}

/// The broker and the server of `sealed`'s box, once the tracer holds
/// both: the broker at the box's end, the server as its exec returns.
fn stopped(sealed: &Sealed) -> (i32, i32) {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let arguments = |pid: i32| std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
    let named = |pid: i32, name: &[u8]| {
        arguments(pid)
            .split(|byte| *byte == 0)
            .any(|arg| arg == name)
    };
    let script = server(sealed).into_bytes();
    while std::time::Instant::now() < until {
        let server = boxed(sealed)
            .into_iter()
            .find(|pid| named(*pid, &script) & halted(*pid));
        let broker = processes().find(|pid| named(*pid, b"serve") & halted(*pid));
        if let (Some(broker), Some(server)) = (broker, server) {
            return (broker, server);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("no server of {} held", sealed.locator.display());
}

/// Each descriptor `pid` holds, by number, with the kind of what it is
/// open on.
fn descriptors(pid: i32) -> Vec<(u32, String)> {
    let open = std::fs::read_dir(format!("/proc/{pid}/fd")).unwrap();
    let mut held: Vec<(u32, String)> = open
        .map(|entry| {
            let number: u32 = entry
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            let link = std::fs::read_link(format!("/proc/{pid}/fd/{number}")).unwrap();
            let link = link.display().to_string();
            let kind = link
                .split_once(":[")
                .map_or(link.clone(), |(kind, _)| kind.to_string());
            (number, kind)
        })
        .collect();
    held.sort();
    held
}

/// Signal `pid` with `signal`.
fn signalled(pid: i32, signal: rustix::process::Signal) {
    rustix::process::kill_process(rustix::process::Pid::from_raw(pid).unwrap(), signal).unwrap();
}

#[test]
fn the_server_inherits_only_its_stdio_and_its_environment_from_the_private_home() {
    let store = format!("DOCS_TOKEN={TOKEN}\n");
    let Some((sealed, digest, tracer)) =
        serving("#!/bin/sh\n:\n", store.as_bytes(), &["DOCS_TOKEN"])
    else {
        return;
    };
    let apart = sealed.path("broker.err");
    let command = traced(&tracer, HOLD_SERVER);
    let mut command = started_to(command, &sealed, &digest, (Starter::Harness, Some(&apart)));
    let mut attempt = command.stderr(Stdio::null()).spawn().unwrap();
    // Held as its exec returns, before its interpreter has run: the server
    // holds its standard input and output pipes and no standard error, and
    // nothing else, the status, control, store and source descriptors all
    // gone; its environment is the confined one, its directory the private
    // HOME.
    let (broker, server) = stopped(&sealed);
    let stdio = [(0, "pipe"), (1, "pipe"), (2, "/dev/null")];
    assert_eq!(
        descriptors(server),
        stdio.map(|(number, kind)| (number, kind.to_string()))
    );
    let cwd = std::fs::read_link(format!("/proc/{server}/cwd")).unwrap();
    assert_eq!(cwd, Path::new("/runtime/home"));
    let environ = std::fs::read(format!("/proc/{server}/environ")).unwrap();
    let mut entries: Vec<String> = environ
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| String::from_utf8(entry.to_vec()).unwrap())
        .collect();
    entries.sort();
    assert_eq!(entries, confined(&[format!("DOCS_TOKEN={TOKEN}")]));
    // The server ended, the box follows it, and the broker let go ends
    // with serving still refused.
    assert!(halted(broker));
    signalled(server, rustix::process::Signal::KILL);
    let ended = attempt.wait().unwrap();
    let answer = (ended.code(), std::fs::read_to_string(apart).unwrap());
    assert_eq!(answer, admitted());
    assert!(within(|| boxed(&sealed).is_empty()));
}

/// The tracer's arguments when it holds the bootstrap at its terminal:
/// every process followed, stopped by a seccomp filter at `ioctl` alone,
/// nothing written, and each `ioctl` on `/dev/tty` held `seconds` at its
/// entry. Only the bootstrap makes one, `TIOCNOTTY` on the controlling
/// terminal it opened to give up, once it has the whole binding frame and
/// its end, and before its exec: after the sender has completed. No
/// `open` is stopped, so the observer's walk of the system set, an `open`
/// for each entry, runs at its own pace. A bootstrap killed while held
/// ends only once its hold does, as the tracer lets go.
fn hold_terminal(seconds: u32) -> Vec<String> {
    let args = ["-f", "--seccomp-bpf", "-qqq", "-o", "/dev/null"];
    let ioctl = ["-e", "trace=ioctl", "-P", "/dev/tty", "-e"];
    let held = format!("inject=ioctl:delay_enter={seconds}000000");
    let args = args.into_iter().chain(ioctl).map(String::from);
    args.chain([held]).collect()
}

/// The tracer's arguments when it holds the broker once it has read the
/// store: every process followed, stopped by a seccomp filter at `read`
/// alone, nothing written, and the first read of `store` held `seconds` as
/// it returns. Only the sender reads the store, after readiness and before
/// its frame: before the sender has completed.
fn hold_lookup(store: &Path, seconds: u32) -> Vec<String> {
    let store = store.display().to_string();
    let args = ["-f", "--seccomp-bpf", "-qqq", "-o", "/dev/null"];
    let read = ["-e", "trace=read", "-P", &store];
    let held = format!("inject=read:delay_exit={seconds}000000:when=1");
    args.into_iter()
        .chain(read)
        .map(String::from)
        .chain(["-e".to_string(), held])
        .collect()
}

/// A broker serving a plan that declares `DOCS_TOKEN` under a tracer that
/// holds one stage of its handoff: when it began, the reads of its store,
/// with those already counted, the reads of its server since the hold,
/// which only a started server makes, and its own TMPDIR, where the box's
/// generated identity's tree is made.
struct Handing {
    sealed: Sealed,
    attempt: Child,
    begun: Instant,
    store: (Reads, usize),
    server: Reads,
    scratch: tempfile::TempDir,
}

impl Handing {
    /// The broker started by a harness under the tracer given what
    /// `tracing` names for its fixture; none, declared skipped, where no
    /// box stands or no tracer traces.
    fn started(tracing: impl FnOnce(&Sealed) -> Vec<String>) -> Option<Handing> {
        Handing::started_by(Starter::Harness, tracing)
    }

    /// [`Handing::started`], the broker started by `starter`.
    fn started_by(
        starter: Starter,
        tracing: impl FnOnce(&Sealed) -> Vec<String>,
    ) -> Option<Handing> {
        Handing::begun(starter, tracing, |command| command)
    }

    /// [`Handing::started`], the harness and the broker on a terminal of
    /// their own ([`on_a_terminal`]), which the box's bootstrap holds until
    /// it gives it up; none, declared skipped, where no `script(1)` gives
    /// them one.
    fn at_a_terminal(tracing: impl FnOnce(&Sealed) -> Vec<String>) -> Option<Handing> {
        if !Path::new("/usr/bin/script").is_file() {
            skip(NO_TERMINAL);
            return None;
        }
        Handing::begun(Starter::Harness, tracing, |command| {
            let mut terminal = on_a_terminal(&command);
            terminal.stdin(Stdio::piped()).stdout(Stdio::null());
            terminal
        })
    }

    /// [`Handing::started_by`], the broker's whole command run as `run`
    /// makes it.
    fn begun(
        starter: Starter,
        tracing: impl FnOnce(&Sealed) -> Vec<String>,
        run: impl FnOnce(Command) -> Command,
    ) -> Option<Handing> {
        let (sealed, digest, tracer) = serving("#!/bin/sh\n:\n", b"", &["DOCS_TOKEN"])?;
        let store = Reads::of(&sealed);
        let server = Reads::on(sealed.path("opt/docs/bin/docs-mcp"));
        let apart = sealed.path("broker.err");
        let command = traced(&tracer, tracing(&sealed));
        let mut command = started_to(command, &sealed, &digest, (starter, Some(&apart)));
        let scratch = tempfile::tempdir().unwrap();
        command.env("TMPDIR", std::fs::canonicalize(scratch.path()).unwrap());
        let mut command = run(command);
        let begun = Instant::now();
        let attempt = command.stderr(Stdio::null()).spawn().unwrap();
        Some(Handing {
            sealed,
            attempt,
            begun,
            store: (store, 0),
            server,
            scratch,
        })
    }

    /// The box's bootstrap, where it stands: the process of the box whose
    /// parent, the box's first process, is of it too.
    fn bootstrap(&self) -> Option<i32> {
        let boxed = bootstraps(&self.sealed);
        let inner = |pid: &i32| status(*pid).is_some_and(|(_, parent, _)| boxed.contains(&parent));
        boxed.iter().copied().find(inner)
    }

    /// The bootstrap once `stage` holds, polled for 20 seconds with the
    /// store's reads counted meanwhile; the server's reads before then,
    /// the observer's among them, set aside.
    fn at(&mut self, stage: impl Fn(&Handing, i32) -> bool) -> i32 {
        let until = Instant::now() + Duration::from_secs(20);
        while Instant::now() < until {
            self.store.1 += self.store.0.seen();
            if let Some(bootstrap) = self.bootstrap().filter(|pid| stage(self, *pid)) {
                self.server.seen();
                return bootstrap;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let boxed: Vec<_> = boxed(&self.sealed)
            .into_iter()
            .map(|pid| (pid, status(pid)))
            .collect();
        panic!(
            "no stage of {} reached: {boxed:?}",
            self.sealed.locator.display()
        );
    }

    /// The broker's exit and its own stderr, once it has ended; the
    /// store's reads and the server's since the hold; and whether nothing
    /// of the box is left.
    fn ended(mut self) -> ((Option<i32>, String), (usize, usize), bool) {
        let code = self.attempt.wait().unwrap().code();
        let stderr = std::fs::read_to_string(self.sealed.path("broker.err")).unwrap();
        let reads = (self.store.1 + self.store.0.lookups(), self.server.lookups());
        let settled = within(|| boxed(&self.sealed).is_empty());
        ((code, stderr), reads, settled)
    }
}

/// Whether the bootstrap `pid` is held at its terminal's release: stopped,
/// still stopped a while later, which no passing stop at another call is,
/// and holding `/dev/tty` open, which it opens only to give it up, once the
/// whole frame and its end are received.
fn at_its_terminal(_: &Handing, pid: i32) -> bool {
    let stopped = halted(pid);
    std::thread::sleep(Duration::from_millis(200));
    let releasing = || descriptors(pid).iter().any(|(_, open)| open == "/dev/tty");
    stopped && halted(pid) && releasing()
}

#[test]
fn a_bootstrap_gone_before_the_sender_completes_is_the_handoffs_cause() {
    // Held once the sender has read the store, the bootstrap is killed:
    // the frame then has no reader, which is the handoff's cause, after
    // the one lookup and with no server started.
    let held = Handing::started(|sealed| hold_lookup(&sealed.path("store/secrets.env"), 4));
    let Some(mut handing) = held else {
        return;
    };
    let bootstrap = handing.at(|handing, _| handing.store.1 > 0);
    signalled(bootstrap, rustix::process::Signal::KILL);
    let (answer, reads, settled) = handing.ended();
    assert_eq!(answer, refused(Refusal::Handoff));
    assert_eq!(reads, (1, 0));
    assert!(settled);
}

/// A broker started by `starter` whose sender's first read of the store
/// is held `seconds`, the launcher of its box once that read is held (the
/// parent of the box's first process), and the broker itself (the
/// launcher's parent). None, declared skipped, where no box stands or no
/// tracer traces.
fn store_held(starter: Starter, seconds: u32) -> Option<(Handing, (i32, i32))> {
    let store = |sealed: &Sealed| hold_lookup(&sealed.path("store/secrets.env"), seconds);
    let mut handing = Handing::started_by(starter, store)?;
    let first = status(handing.at(|handing, _| handing.store.1 > 0))
        .unwrap()
        .1;
    let launcher = status(first).unwrap().1;
    Some((handing, (launcher, status(launcher).unwrap().1)))
}

/// When `handing`'s `broker` ended, after `from`, checking that by then it
/// had reaped its box's `launcher` and removed the generated identity's
/// tree it was handed: the box settled before the refusal, the held
/// sender not waited for.
fn returned(handing: &Handing, (launcher, broker): (i32, i32), from: Instant) -> Duration {
    let ended = awaited(broker) - from;
    assert_eq!(status(launcher), None);
    let trees = std::fs::read_dir(handing.scratch.path()).unwrap().count();
    assert_eq!(trees, 0);
    ended
}

/// Check that `handing`'s broker, once ended, refused with the handoff's
/// cause after one lookup, no server read and nothing of its box left; its
/// exit code.
fn refused_handoff(handing: Handing) -> Option<i32> {
    let ((code, stderr), reads, settled) = handing.ended();
    assert_eq!(stderr, refused(Refusal::Handoff).1);
    assert_eq!(reads, (1, 0));
    assert!(settled);
    code
}

#[test]
fn a_store_read_held_past_the_deadline_is_refused_at_the_deadline() {
    // The sender's read of the store held 40 seconds, past the startup's
    // absolute deadline: at that deadline, and no more than a slice after,
    // the broker ends the box's launcher, settles the box and refuses with
    // the handoff's cause, its own exit not waiting for the read, which is
    // still held until at least 40 seconds after the start (operator
    // ruling 2026-10-11). No server started.
    let Some((handing, pids)) = store_held(Starter::Harness, 40) else {
        return;
    };
    let ended = returned(&handing, pids, handing.begun);
    let bounded = (ended >= Duration::from_secs(30)) & (ended < Duration::from_secs(32));
    assert!(bounded, "{ended:?}");
    assert_eq!(refused_handoff(handing), refused(Refusal::Handoff).0);
}

#[test]
fn a_store_read_held_when_the_attempt_is_cancelled_is_refused_at_once() {
    // The attempt's own start ends while the sender's read of the store is
    // held 20 seconds: the broker ends the box's launcher, settles the box
    // and refuses with the handoff's cause at once, the read still held.
    // No server started.
    let Some((handing, pids)) = store_held(Starter::Attempt, 20) else {
        return;
    };
    signalled(status(pids.1).unwrap().1, rustix::process::Signal::KILL);
    let cancelled = Instant::now();
    let taken = returned(&handing, pids, cancelled);
    assert!(taken < Duration::from_secs(5), "{taken:?}");
    refused_handoff(handing);
}

#[test]
fn a_bootstrap_gone_after_the_sender_completed_certifies_nothing() {
    // Killed at its terminal, the whole frame and its end received, the
    // bootstrap closes its status pipe with no tag: that end permits no
    // more than initialization, which `serve` does not reach, and nothing
    // is certified. No server started.
    let Some(mut handing) = Handing::at_a_terminal(|_| hold_terminal(10)) else {
        return;
    };
    let bootstrap = handing.at(at_its_terminal);
    signalled(bootstrap, rustix::process::Signal::KILL);
    let (answer, reads, settled) = handing.ended();
    assert_eq!(answer, admitted());
    assert_eq!(reads, (1, 0));
    assert!(settled);
}

#[test]
fn a_status_pipe_open_after_the_whole_frame_is_the_handoffs_cause_at_the_deadline() {
    // Held at its terminal past the deadline, the whole frame and its end
    // received, the bootstrap neither execs nor reports: at the startup's
    // absolute deadline, and no more than a slice after, the broker ends
    // the box's launcher and refuses with the handoff's cause; no server
    // started. The box ends once the tracer lets its bootstrap go.
    let Some(mut handing) = Handing::at_a_terminal(|_| hold_terminal(40)) else {
        return;
    };
    let bootstrap = handing.at(at_its_terminal);
    let first = status(bootstrap).unwrap().1;
    let launcher = status(first).unwrap().1;
    let ended = awaited(launcher) - handing.begun;
    let (answer, reads, settled) = handing.ended();
    assert_eq!(answer, refused(Refusal::Handoff));
    let bounded = (ended >= Duration::from_secs(30)) & (ended < Duration::from_secs(32));
    assert!(bounded, "{ended:?}");
    assert_eq!(reads, (1, 0));
    assert!(settled);
}

/// Each word of `command`, quoted for a shell.
fn line(command: &Command) -> String {
    let words = std::iter::once(command.get_program()).chain(command.get_args());
    let quoted = words.map(|word| format!("'{}'", word.to_str().unwrap().replace('\'', "'\\''")));
    quoted.collect::<Vec<_>>().join(" ")
}

/// `command` run by `script` on a terminal of its own, which becomes the
/// controlling terminal of everything `command` starts.
fn on_a_terminal(command: &Command) -> Command {
    let mut terminal = Command::new("script");
    terminal.args(["-q", "-e", "-c", &line(command), "/dev/null"]);
    let set = command
        .get_envs()
        .filter_map(|(key, value)| Some((key, value?)));
    terminal.envs(set).env("SHELL", "/bin/sh");
    terminal.current_dir(command.get_current_dir().unwrap());
    terminal
}

/// Why a run skips the terminal proofs.
const NO_TERMINAL: &str = "no script(1) gives a broker a controlling terminal here";

/// The tracer's arguments when it records each exec of the run whole, as
/// [`EXECS`] does, and each `ioctl` and `kill` beside it, every descriptor
/// with the path it is open on; and each process's second `kill` (the
/// broker's: its observer's group, then the box's launcher) skipped, so the
/// box stands until its server ends of itself. No `open` is stopped, so the
/// observer's walk of the system set runs at its own pace.
fn terminal_tracing(log: &Path) -> Vec<&OsStr> {
    let widened = |arg: &&'static str| match *arg {
        "trace=execve" => "trace=execve,ioctl,kill",
        arg => arg,
    };
    let kept = ["-y", "-e", "inject=kill:retval=0:when=2"];
    let args = kept.into_iter().chain(EXECS.iter().map(widened));
    args.map(OsStr::new).chain([log.as_os_str()]).collect()
}

/// The server that tries its terminal: its shell opens `/dev/tty` in a
/// subshell of its own, which inherits its session, and passes the shell's
/// own account of that open, or `opened`, to an exec the tracer records.
const OPENER: &str = "#!/bin/sh\n\
    cause=$( (exec 3<>/dev/tty) 2>&1 ) && cause=opened\n\
    exec /bin/sh -c : tty \"$cause\"\n";

#[test]
fn a_server_started_from_a_terminal_cannot_open_it() {
    let Some((sealed, digest, tracer)) = serving(OPENER, b"", &[]) else {
        return;
    };
    if !Path::new("/usr/bin/script").is_file() {
        return skip(NO_TERMINAL);
    }
    let log = sealed.path("tty.log");
    let apart = sealed.path("broker.err");
    let command = traced(&tracer, terminal_tracing(&log));
    let command = started_to(command, &sealed, &digest, (Starter::Harness, Some(&apart)));
    let mut terminal = on_a_terminal(&command);
    let mut run = terminal
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let ended = run.wait().unwrap();
    assert_eq!(ended.code(), admitted().0);
    assert_eq!(std::fs::read_to_string(apart).unwrap(), admitted().1);
    // One process of the run gives a terminal up: the bootstrap, which
    // held it, and gave it up before it became the server, the same
    // process; the server's own shell then finds no terminal to open.
    let each = traces(&log);
    let releasing = each
        .iter()
        .filter(|lines| lines.iter().any(|line| line.contains("TIOCNOTTY")));
    let [bootstrap] = releasing.collect::<Vec<_>>()[..] else {
        panic!("{each:#?}");
    };
    let from = bootstrap.iter().position(|line| line.contains("TIOCNOTTY"));
    let after = bootstrap[from.unwrap()..].iter().filter(|line| {
        line.starts_with("execve(") | (line.starts_with("ioctl(") & line.contains("TIOCNOTTY"))
    });
    let after: Vec<&String> = after.collect();
    let [released, served, opened] = after[..] else {
        panic!("{bootstrap:#?}");
    };
    // strace pads a short call to its column before the result.
    let result = released.split_once("</dev/tty>, TIOCNOTTY)");
    let result = result.map(|(call, result)| (call.split_once('(').unwrap().0, result.trim()));
    assert_eq!(result, Some(("ioctl", "= 0")), "{released}");
    let server = server(&sealed);
    let argv = format!("execve(\"{server}\", [\"{server}\", \"--stdio\"], [");
    assert!(
        served.starts_with(&argv) & served.ends_with("]) = 0"),
        "{served}"
    );
    let cause = opened.split_once("\"tty\", \"").map(|(_, rest)| rest);
    let cause = cause.and_then(|rest| rest.split_once("\"], [")).unwrap().0;
    assert!(
        opened.starts_with("execve(\"/bin/sh\", [\"/bin/sh\", \"-c\", \":\", \"tty\", ")
            & cause.ends_with(" /dev/tty: No such device or address"),
        "{opened}"
    );
    assert!(within(|| boxed(&sealed).is_empty()));
}
