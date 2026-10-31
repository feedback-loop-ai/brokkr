//! The served box's limits through the public session (decision 0065
//! slice two U6f1b; MB3, MB4, SD3), on real boxes: each server's private
//! HOME and TMPDIR are its own, beside another's and a replacement's; its
//! network reaches the host's loopback and abstract Unix sockets only where
//! it is shared; and what its installed package holds, a credential canary
//! and bytes installed long before, it reads as it is. Each server reports
//! what it found on its own standard error, which the broker drains and
//! masks.

use std::io::{PipeReader, PipeWriter, Read, Write};
use std::os::linux::net::SocketAddrExt;
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};

use brokkr_protocol::broker::ledger::{Disposition, Failure, Outcome};
use brokkr_protocol::secret::{BoundSecret, Guard};
use rustix::fs::OFlags;
use serde_json::{json, Value};

use super::ledger::sealed_ledger;
use super::observer::within;
use super::protocol::{after, conversed, greeting, relayed, speaking, talking, talking_beside};
use super::protocol::{unheld, SERVER};
use super::readiness::boxed;
use super::{set, skip, Sealed, SPEAKS};

/// A sealed fixture whose plan retains nothing and whose box's network is
/// `network`, its server running `probe` once called ([`after`]), and its
/// plan's digest; none, declared skipped, where no box stands.
fn probing(probe: &str, network: &str) -> Option<(Sealed, String)> {
    serving(&after(probe), network)
}

/// A sealed fixture whose plan retains nothing and whose box's network is
/// `network`, its server running `said` in the box's own shell, and its
/// plan's digest; none, declared skipped, where no box stands.
fn serving(said: &str, network: &str) -> Option<(Sealed, String)> {
    let (sealed, _) = speaking(said)?;
    let mut plan = sealed.authored();
    set(&mut plan, "/retained", json!(false));
    set(&mut plan, "/box/network", json!(network));
    let plan = sealed.observing(plan);
    let digest = sealed.seal_bytes(plan.to_string().as_bytes());
    Some((sealed, digest))
}

/// What a probe of its own private directories reports: the names of the
/// environment it was started with, its directory, then what its TMPDIR
/// and HOME held before it left a marker in each.
const PRIVATE: &str = r#"tr "\0" "\n" < /proc/$$/environ | cut -d= -f1 | sort | tr "\n" " " >&2
echo >&2
pwd >&2
ls -A "$TMPDIR" "$HOME" >&2
touch "$TMPDIR/u6f1b-mark" "$HOME/u6f1b-mark""#;

/// What [`PRIVATE`] reports from a fresh box: MB4's eight fixed names and
/// the one declared binding's, its private HOME, and both directories, in
/// the order `ls` lists them, empty.
fn fresh() -> String {
    let names = "BROKKR_HANDS_BOX DOCS_TOKEN HOME LANG LC_ALL LOGNAME PATH TMPDIR USER ";
    format!("{names}\n/runtime/home\n/runtime/home:\n\n/tmp:\n")
}

/// A regular file of the system set every box binds that is multiply
/// linked, as installed systems hold them, root's and written by no one
/// else: its path, link count and inode; none where this host holds none.
pub(crate) fn system_hard_link() -> Option<(std::path::PathBuf, u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    ["/usr/bin", "/usr/sbin", "/usr/lib"]
        .iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flatten()
        .flatten()
        .find_map(|entry| {
            let held = std::fs::symlink_metadata(entry.path()).ok()?;
            let ordinary = (held.uid() == 0) & (held.mode() & 0o022 == 0);
            (held.is_file() & (held.nlink() > 1) & ordinary)
                .then(|| (entry.path(), held.nlink(), held.ino()))
        })
}

#[test]
fn a_dedicated_native_server_in_a_package_is_served_beside_the_systems_own_hard_links() {
    // The package's server is a native program of its own, singly linked:
    // a copy of the host's `/bin/sh`, an ELF whose loader and libraries are
    // the system set's, run on the script it serves. The system set holds
    // ordinary multiply-linked files, which pass admission as installed,
    // neither renamed nor copied nor pruned: the server sees the very link
    // count and inode the host does. It starts with MB4's fixed names and
    // its binding's alone, is served, and closes Clean (MB3, MB4, SD4).
    let Some((linked, links, inode)) = system_hard_link() else {
        return skip("this host's system set holds no multiply-linked file");
    };
    let Some((sealed, _)) = speaking("") else {
        return;
    };
    let native = std::fs::read("/bin/sh").unwrap();
    assert_eq!(&native[..4], b"\x7fELF");
    let program = sealed.path("opt/docs/bin/docs-mcp");
    std::fs::write(&program, native).unwrap();
    let script = sealed.path("opt/docs/share/serve");
    std::fs::create_dir_all(script.parent().unwrap()).unwrap();
    let probe = format!(
        "{}\nstat -c '%h %i' '{}' >&2",
        PRIVATE.lines().take(2).collect::<Vec<_>>().join("\n"),
        linked.display()
    );
    std::fs::write(&script, after(&probe)).unwrap();
    let mut plan = sealed.authored();
    set(&mut plan, "/retained", json!(false));
    set(&mut plan, "/connection/argv", json!([program, script]));
    let plan = sealed.observing(plan);
    let digest = sealed.seal_bytes(plan.to_string().as_bytes());
    let ran = conversed(&sealed, &digest, &greeting(), 3);
    let names = fresh().lines().next().unwrap().to_string();
    let said = relayed(&format!("{names}\n{links} {inode}\n"));
    assert_eq!((ran.0, ran.1.len(), ran.3), (Some(0), 3, said));
    let single = std::os::unix::fs::MetadataExt::nlink(&std::fs::metadata(&program).unwrap());
    assert_eq!(single, 1);
    closed_clean(&sealed);
}

#[test]
fn two_servers_and_a_replacement_keep_their_private_directories_apart() {
    // Two admitted brokers stand at once, each serving its harness: what
    // one server leaves in its HOME and TMPDIR the other never sees, nor
    // does the plan's replacement lifetime, nor the host (MB4).
    let Some((first, first_digest)) = probing(PRIVATE, "shared") else {
        return;
    };
    let Some((second, second_digest)) = probing(PRIVATE, "shared") else {
        return;
    };
    let held = talking(&first, &first_digest, &greeting(), 3);
    let beside = conversed(&second, &second_digest, &greeting(), 3);
    let first_ran = held.ended();
    let replaced = conversed(&first, &first_digest, &greeting(), 3);
    let reported = |ran: &super::protocol::Ran| (ran.0, ran.1.len(), ran.3.clone());
    let fresh = (Some(0), 3, relayed(&fresh()));
    for (case, ran) in [
        ("first", first_ran),
        ("beside", beside),
        ("replaced", replaced),
    ] {
        assert_eq!((case, reported(&ran)), (case, fresh.clone()));
    }
    for sealed in [&first, &second] {
        assert!(within(|| boxed(sealed).is_empty()));
        assert!(!sealed.home.join("u6f1b-mark").exists());
    }
    assert!(!std::path::Path::new("/tmp/u6f1b-mark").exists());
}

/// A probe of the test endpoint `external`, the host's loopback listener on
/// `port` and its abstract Unix socket `name`, each reported reached or
/// denied, by the box's own `bash` and `python3`.
fn sentinels(external: std::net::SocketAddr, port: u16, name: &str) -> String {
    let (host, external) = (external.ip(), external.port());
    format!(
        r#"if bash -c 'exec 3<>/dev/tcp/{host}/{external}' 2>/dev/null; then echo external=reached >&2; else echo external=denied >&2; fi
if bash -c 'exec 3<>/dev/tcp/127.0.0.1/{port}' 2>/dev/null; then echo loopback=reached >&2; else echo loopback=denied >&2; fi
if python3 -c 'import socket; socket.socket(socket.AF_UNIX).connect(b"\0{name}")' 2>/dev/null; then echo abstract=reached >&2; else echo abstract=denied >&2; fi"#
    )
}

/// How many connections `accept` takes without waiting.
fn accepted<T>(mut accept: impl FnMut() -> std::io::Result<T>) -> usize {
    std::iter::from_fn(|| accept().ok()).count()
}

/// The host's own address on its route out, which no loopback reaches:
/// the test endpoint a networked box reaches as it reaches any other, and
/// none where the host has no route. A UDP connect sends nothing.
fn routed() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    let address = socket.local_addr().ok()?.ip();
    (!address.is_loopback()).then_some(address)
}

/// The box's own tools a probe runs, or the first a host lacks, skipped.
fn tooled() -> bool {
    let tools = ["/usr/bin/bash", "/usr/bin/python3"];
    let missing = tools
        .iter()
        .find(|tool| !std::path::Path::new(tool).exists());
    missing.inspect(|missing| skip(&format!("{missing} is not installed here")));
    missing.is_none()
}

#[test]
fn only_a_shared_network_reaches_an_external_endpoint_the_hosts_loopback_and_abstract_sockets() {
    // A local dialect's box is isolated: neither the test endpoint, on the
    // host's address on its route out, nor the host's loopback listener,
    // nor its abstract Unix socket answers it. A networked one's box shares
    // the host's network, and reaches all three (MB4). The probes are the
    // box's own tools, which a host without them cannot give.
    let Some(host) = routed().filter(|_| tooled()) else {
        return skip("this host has no route out, or lacks a probe's tool");
    };
    for (network, reach) in [("isolated", "denied"), ("shared", "reached")] {
        let endpoint = std::net::TcpListener::bind((host, 0)).unwrap();
        endpoint.set_nonblocking(true).unwrap();
        let loopback = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        loopback.set_nonblocking(true).unwrap();
        let name = format!("brokkr-u6f1b-{}-{network}", std::process::id());
        let address = std::os::unix::net::SocketAddr::from_abstract_name(&name).unwrap();
        let abstract_socket = std::os::unix::net::UnixListener::bind_addr(&address).unwrap();
        abstract_socket.set_nonblocking(true).unwrap();
        let port = loopback.local_addr().unwrap().port();
        let probe = sentinels(endpoint.local_addr().unwrap(), port, &name);
        let Some((sealed, digest)) = probing(&probe, network) else {
            return;
        };
        let ran = conversed(&sealed, &digest, &greeting(), 3);
        let said = relayed(&format!(
            "external={reach}\nloopback={reach}\nabstract={reach}\n"
        ));
        let taken = usize::from(reach == "reached");
        let connected = (
            accepted(|| endpoint.accept()),
            accepted(|| loopback.accept()),
            accepted(|| abstract_socket.accept()),
        );
        let answer = (ran.0, ran.1.len(), ran.3, connected);
        assert_eq!(
            (network, answer),
            (network, (Some(0), 3, said, (taken, taken, taken)))
        );
        assert!(within(|| boxed(&sealed).is_empty()));
    }
}

/// What a box's resolver reads, reported by its own tools: the names under
/// its `/etc`, its generated `nsswitch.conf` and `hosts`, its
/// `resolv.conf` or that it has none, what `localhost` resolves to, and
/// what `_localdnsstub`, which no hosts file holds, does, or that it does
/// not resolve.
const RESOLVER: &str = r#"echo "etc=$(ls -A /etc | tr "\n" " ")" >&2
cat /etc/nsswitch.conf /etc/hosts >&2
if [ -e /etc/resolv.conf ]; then cat /etc/resolv.conf >&2; else echo resolv=absent >&2; fi
python3 -c 'import socket; print("localhost=" + socket.gethostbyname("localhost"))' >&2
python3 -c 'import socket; print("stub=" + socket.gethostbyname("_localdnsstub"))' >&2 2>/dev/null || echo stub=unresolved >&2"#;

/// The address systemd-resolved's DNS stub answers `_localdnsstub` with,
/// its own: a fixed answer no hosts file gives, so a box that resolves that
/// name has asked the stub its `resolv.conf` names.
const STUB: &str = "127.0.0.53";

/// Whether this host's checked resolver input names systemd-resolved's
/// stub, the one DNS server a test can know the answer of; declared
/// skipped where not.
fn stubbed() -> bool {
    let resolv = std::fs::read_to_string("/etc/resolv.conf").unwrap_or_default();
    let stubbed = resolv
        .lines()
        .any(|line| line.split_whitespace().eq(["nameserver", STUB]));
    if !stubbed {
        skip("this host's resolv.conf names no systemd-resolved stub");
    }
    stubbed
}

/// The names a server box holds under `/etc` on this host: its generated
/// identity and resolver files, each system source under `/etc` this host
/// has, and, where its network is `shared`, the host's `resolv.conf`.
fn etc(shared: bool) -> Vec<String> {
    let generated = ["group", "hosts", "nsswitch.conf", "passwd"].map(String::from);
    let system = brokkr_protocol::hands::HOST_TOOLCHAIN_BINDS
        .iter()
        .filter_map(|bind| bind.strip_prefix("/etc/"))
        .map(|name| (name, if name == "ssl" { "ssl/certs" } else { name }))
        .filter(|(_, held)| std::path::Path::new("/etc").join(held).exists())
        .map(|(name, _)| name.to_string());
    let resolver = (shared & std::path::Path::new("/etc/resolv.conf").exists())
        .then(|| "resolv.conf".to_string());
    let mut names: Vec<String> = generated
        .into_iter()
        .chain(system)
        .chain(resolver)
        .collect();
    names.sort();
    names
}

#[test]
fn a_box_resolves_names_from_its_generated_files_and_a_shared_one_through_the_checked_resolver_alone(
) {
    // An isolated box resolves from its generated files alone: `files`,
    // and no resolv.conf. A shared one adds DNS through the host's
    // resolv.conf, bound read-only as it is, the one file of the host's
    // /etc it binds beside the system set: nothing else under its /etc is
    // a resolver's input (MB3, MB4). `localhost` resolves from the
    // generated hosts in both. A name no hosts file holds resolves in the
    // shared box alone, by a DNS query to the server its resolv.conf names,
    // whose fixed answer it reports exactly.
    if !(tooled() && stubbed()) {
        return;
    }
    let hosts = "127.0.0.1 localhost\n::1 localhost ip6-localhost ip6-loopback\n";
    for (network, shared) in [("isolated", false), ("shared", true)] {
        let Some((sealed, digest)) = probing(RESOLVER, network) else {
            return;
        };
        let ran = conversed(&sealed, &digest, &greeting(), 3);
        let (listed, rest) = ran.3.split_once('\n').unwrap();
        let etc_line = relayed("etc=");
        let mut listed: Vec<String> = listed
            .strip_prefix(etc_line.as_str())
            .unwrap()
            .split_whitespace()
            .map(String::from)
            .collect();
        listed.sort();
        let (switch, resolv, stub) = match shared {
            true => (
                "files dns",
                std::fs::read_to_string("/etc/resolv.conf").unwrap(),
                STUB,
            ),
            false => ("files", "resolv=absent\n".to_string(), "unresolved"),
        };
        let read = format!("hosts: {switch}\n{hosts}{resolv}localhost=127.0.0.1\nstub={stub}\n");
        let read = relayed(&read);
        assert_eq!(
            (network, ran.0, ran.1.len(), listed, rest.to_string()),
            (network, Some(0), 3, etc(shared), read)
        );
        assert!(within(|| boxed(&sealed).is_empty()));
    }
}

#[test]
fn a_served_session_keeps_no_startup_deadline_and_answers_a_call_made_past_it() {
    // The startup has thirty seconds from `serve`'s start (MB3); serving
    // keeps none of the startup's, and each call has its own 120. The
    // harness is greeted and listed, idles past the startup's deadline, and
    // only then calls: the call is forwarded and answered under its own id,
    // and the session closes Clean with it.
    let Some((sealed, digest)) = probing("", "isolated") else {
        return;
    };
    let begun = Instant::now();
    let frames = greeting();
    let mut talk = talking(&sealed, &digest, &frames[..3], 2);
    let past = begun + Duration::from_secs(31);
    std::thread::sleep(past.saturating_duration_since(Instant::now()));
    talk.asked(&frames[3]);
    let ran = talk.ended();
    let content = json!({"content": [{"type": "text", "text": "resolved"}]});
    let answer = json!({"jsonrpc": "2.0", "id": "c", "result": content});
    assert_eq!(
        (ran.0, ran.1.len(), ran.1.last(), ran.3),
        (Some(0), 3, Some(&answer), String::new())
    );
    closed_clean(&sealed);
}

/// Check that `sealed`'s session closed Clean with its one call to
/// `resolve` succeeded, and that nothing of its box is left.
fn closed_clean(sealed: &Sealed) {
    let one = vec![("resolve".to_string(), Some(Outcome::Succeeded))];
    assert_eq!(sealed_ledger(sealed), (one, Some(Disposition::Clean), None));
    assert!(within(|| boxed(sealed).is_empty()));
}

/// What a server says on its standard error before it answers anything:
/// 4,096 lines of 64 bytes, 256 KiB, past any pipe's room.
const FLOOD: &str = r#"i=0
while [ $i -lt 4096 ]; do printf '%063d\n' $i >&2; i=$((i+1)); done
"#;

/// The masked tail the broker keeps of [`FLOOD`]: its last 64 KiB, as many
/// whole lines as fit once each is labelled the server's ([`relayed`]).
fn flooded() -> String {
    let kept = (64 << 10) / relayed(&format!("{:063}\n", 0)).len();
    let lines: String = (4096 - kept..4096)
        .map(|line| format!("{line:063}\n"))
        .collect();
    relayed(&lines)
}

#[test]
fn a_server_that_floods_its_standard_error_before_it_answers_is_drained_and_its_tail_kept() {
    // The server writes 256 KiB to its standard error before it answers
    // initialize, past a Linux pipe's 64 KiB and macOS's 16 KiB: drained
    // from its start, it never waits on that pipe, so it is initialized,
    // listed and served, and closes Clean, its masked tail, the last 64 KiB,
    // written to the broker's own standard error (MB3, MB4).
    let Some((open, open_digest)) = serving(&format!("{FLOOD}{}", after("")), "isolated") else {
        return;
    };
    let ran = conversed(&open, &open_digest, &greeting(), 3);
    assert_eq!((ran.0, ran.1.len(), ran.3), (Some(0), 3, flooded()));
    closed_clean(&open);
    // A server that floods it and then speaks no MCP ends the session with
    // the protocol's cause: the tail is written too, before the broker's
    // own last line.
    let corrupt = format!("{FLOOD}read -r line\necho no\n");
    let Some((failing, failing_digest)) = serving(&corrupt, "isolated") else {
        return;
    };
    let ran = conversed(&failing, &failing_digest, &greeting(), 0);
    let (code, cause) = super::ended(Failure::Protocol);
    assert_eq!((ran.0, ran.1.len(), ran.3), (code, 0, flooded() + &cause));
    assert!(within(|| boxed(&failing).is_empty()));
}

#[test]
fn a_servers_terminal_sequences_are_shown_inert_and_a_value_they_split_is_masked_whole() {
    // The server's standard error clears the screen, sets the clipboard by
    // OSC 52 and returns the carriage over its own line, and writes the
    // bound value split by a control byte. Each line is sanitized for
    // display as every rendered line is, before the shared masker reads it
    // again and the final guard reads what is kept: no control byte reaches
    // the broker's standard error, and the value the sanitizer joined is
    // masked whole (U6f1b; MB4).
    let sequences = r"printf 'clear \033[2J copy \033]52;c;aGk=\007 back\rover\n' >&2
printf 'split %s\001%s end\n' u6c8-ca nary-Qx7Zv >&2";
    assert_eq!(super::TOKEN, "u6c8-canary-Qx7Zv");
    let Some((sealed, digest)) = probing(sequences, "isolated") else {
        return;
    };
    let ran = conversed(&sealed, &digest, &greeting(), 3);
    let shown = relayed("clear [2J copy ]52;c;aGk= backover\nsplit [secret:DOCS_TOKEN] end\n");
    assert_eq!((ran.0, ran.1.len(), ran.3), (Some(0), 3, shown));
    closed_clean(&sealed);
}

/// A pipe whose room is filled, its writer blocking again, as a standard
/// error nobody reads is: its reader, its writer and the bytes it holds.
fn unread() -> (PipeReader, PipeWriter, Vec<u8>) {
    let (reader, mut writer) = std::io::pipe().unwrap();
    let held = filled(&mut writer);
    (reader, writer, held)
}

/// Fill the room of the pipe `writer` writes, which nothing writes
/// meanwhile, leaving its writer blocking again: the bytes it took.
fn filled(writer: &mut PipeWriter) -> Vec<u8> {
    rustix::io::ioctl_fionbio(&*writer, true).unwrap();
    let mut held = Vec::new();
    for chunk in [4096, 1] {
        while writer.write(&vec![b'x'; chunk]).is_ok() {
            held.extend(std::iter::repeat_n(b'x', chunk));
        }
    }
    rustix::io::ioctl_fionbio(&*writer, false).unwrap();
    held
}

#[test]
fn a_standard_error_nobody_reads_never_holds_the_sessions_end() {
    // The broker's standard error is a full pipe nobody reads, and its
    // server says a line there: the masked tail and, where the session
    // fails, the broker's own last line are offered for their bound, and
    // then given up on, so the session still ends, refused or failed with
    // its cause latched, well within the twenty seconds after which a
    // broker that waited on it is killed; nothing past the pipe's room is
    // ever written (MB3). A session served to its end is never Clean so:
    // its server's tail was not taken whole, so it fails with the delivery
    // cause (operator ruling 2026-10-14). So does a plan that does not
    // bind, which opens no ledger. The pipe's status flags, which every
    // holder of it shares, never change while the broker holds it (U6f1b).
    let says = "echo 'server says this' >&2\n";
    let Some((clean, clean_digest)) = probing(says, "isolated") else {
        return;
    };
    let one = vec![("resolve".to_string(), Some(Outcome::Succeeded))];
    let failed = |failure| (vec![], Some(Disposition::Failed(failure)), None);
    let Some((corrupt, corrupt_digest)) =
        serving(&format!("{says}read -r line\necho no\n"), "isolated")
    else {
        return;
    };
    for (case, sealed, digest, (code, answers), proved) in [
        (
            "served",
            &clean,
            &clean_digest,
            (1, 3),
            (one, Some(Disposition::Failed(Failure::Undelivered)), None),
        ),
        (
            "protocol",
            &corrupt,
            &corrupt_digest,
            (1, 0),
            failed(Failure::Protocol),
        ),
    ] {
        let ran = unheard(sealed, digest, (&greeting(), answers));
        assert_eq!((case, ran), (case, (Some(code), answers, true, true)));
        assert_eq!((case, sealed_ledger(sealed)), (case, proved));
        assert!(within(|| boxed(sealed).is_empty()));
    }
    let unbound = (&clean, super::DIGEST);
    let ran = unheard(unbound.0, unbound.1, (&[], 0));
    assert_eq!(ran, (Some(1), 0, true, true));
}

/// The status flags of the pipe `writer` writes, as every holder of it
/// shares them, read every millisecond on a thread of its own until it is
/// told to stop, and once more then: each value read that differs from
/// the one before.
fn watched(writer: PipeWriter) -> (std::sync::mpsc::Sender<()>, JoinHandle<Vec<OFlags>>) {
    let (stop, stopped) = std::sync::mpsc::channel::<()>();
    let mut seen = vec![rustix::fs::fcntl_getfl(&writer).unwrap()];
    let reads = std::thread::spawn(move || loop {
        let waited = stopped.recv_timeout(Duration::from_millis(1));
        let flags = rustix::fs::fcntl_getfl(&writer).unwrap();
        if seen.last() != Some(&flags) {
            seen.push(flags);
        }
        if !matches!(waited, Err(std::sync::mpsc::RecvTimeoutError::Timeout)) {
            return seen;
        }
    });
    (stop, reads)
}

/// `serve` of the plan `sealed` sealed at `digest`, its harness saying
/// `frames` and reading `answers` frames, its standard error a full pipe
/// nobody reads ([`unread`]), killed with the harness shell after twenty
/// seconds: its exit, none where it was killed; the frames its harness
/// read; whether the pipe held only what it held before; and whether its
/// status flags read the same from before the broker started until after
/// it ended ([`watched`]).
fn unheard(
    sealed: &Sealed,
    digest: &str,
    said: (&[Value], usize),
) -> (Option<i32>, usize, bool, bool) {
    let (mut reader, writer, held) = unread();
    let (stop, flags) = watched(writer.try_clone().unwrap());
    let talk = talking_beside(sealed, digest, said, Stdio::from(writer));
    let (done, waited) = std::sync::mpsc::channel::<()>();
    // The broker is the harness shell's child: both are killed.
    let pid = talk.id().to_string();
    let watchdog = std::thread::spawn(move || {
        if waited.recv_timeout(Duration::from_secs(20)).is_err() {
            Command::new("pkill")
                .args(["-KILL", "-P", &pid])
                .status()
                .ok();
            Command::new("kill").args(["-KILL", &pid]).status().ok();
        }
    });
    let ran = talk.ended();
    done.send(()).ok();
    watchdog.join().unwrap();
    stop.send(()).ok();
    let flags = flags.join().unwrap();
    let steady = flags.len() == 1;
    (ran.0, ran.1.len(), left(&mut reader) == held, steady)
}

/// What the pipe `reader` reads still holds, read without waiting.
fn left(reader: &mut PipeReader) -> Vec<u8> {
    rustix::io::ioctl_fionbio(&*reader, true).unwrap();
    let mut said = Vec::new();
    let mut chunk = [0; 4096];
    while let Ok(read @ 1..) = reader.read(&mut chunk) {
        said.extend_from_slice(&chunk[..read]);
    }
    said
}

#[test]
fn a_standard_error_that_takes_one_write_holds_only_a_piece_the_guard_passed() {
    // 0084726193 bound, the server's line, labelled, ends in 847261933 past
    // what one write of POSIX's 512 bytes takes, which passes whole, as
    // itself and spelled apart from the binding; cut where one write ends,
    // its first piece would end in 84726193, the bound value. So the line is
    // cut before its last space instead, where no number is, and a standard
    // error with room for one write and no more holds that first piece
    // alone after what it held, and the rest, the session's own last line
    // included, is given up on at its bound. So too with 0012 bound and
    // the line ending in 123 written as the escapes the guard decodes,
    // whose first 511 bytes, label included, end in the escapes of 12: it
    // is cut before the space ahead of them, never inside an escape (U6f1b,
    // U6f2; MB4; operator rulings 2026-10-13 and 2026-10-14).
    let escaped = ["0031", "0032", "0033"]
        .map(|hex| format!("\\u{hex}"))
        .concat();
    let label = SERVER.len();
    let cases = [
        (
            "plain",
            "0084726193",
            format!("printf '%{}s847261933\\n' '' >&2", 503 - label),
            502 - label,
        ),
        (
            "escaped",
            "0012",
            format!("printf '%{}s%s\\n' '' '{escaped}' >&2", 499 - label),
            498 - label,
        ),
    ];
    for (case, value, line, spaces) in cases {
        let said = format!("{line}\nread -r line\necho no\n");
        let Some((sealed, digest)) = unheld(|| serving(&said, "isolated"), &[value]) else {
            return;
        };
        let store = format!("DOCS_TOKEN={value}\n");
        std::fs::write(sealed.path("store/secrets.env"), store).unwrap();
        let bindings = bound(value);
        let guard = Guard::new(&bindings);
        if case == "escaped" {
            let written = [SERVER, &" ".repeat(499 - label), &escaped].concat();
            let one_write = &written.as_bytes()[..ATOMIC - 1];
            let guarded = (guard.text(written.as_bytes()), guard.text(one_write));
            assert_eq!(guarded, (true, false));
        }
        let (mut reader, writer, held) = unread();
        let mut room = vec![0; 4096];
        reader.read_exact(&mut room).unwrap();
        let talk = talking_beside(&sealed, &digest, (&greeting(), 0), Stdio::from(writer));
        let ran = talk.ended();
        let piece = relayed(&[" ".repeat(spaces), "\n".to_string()].concat());
        let said = (ran.0, ran.1.len(), left(&mut reader));
        let expected = (Some(1), 0, [&held[4096..], piece.as_bytes()].concat());
        assert_eq!((case, said), (case, expected));
        assert_eq!((case, guard.text(piece.as_bytes())), (case, true));
        assert!(within(|| boxed(&sealed).is_empty()));
    }
}

/// `value` bound as DOCS_TOKEN from an owner-only store on a canonical
/// temporary root.
fn bound(value: &str) -> Vec<BoundSecret> {
    let root = tempfile::tempdir().unwrap();
    let store = root.path().canonicalize().unwrap().join("store.env");
    brokkr_protocol::secret::store_set(&store, "DOCS_TOKEN", value).unwrap();
    let declared = ["DOCS_TOKEN".to_string()];
    brokkr_protocol::secret::resolve_bindings(&store, &declared).unwrap()
}

/// The frame the harness is told for its call `c` answered with `result`,
/// as the broker writes it: JSON-RPC's version and that id, then the result
/// with its keys in order, and its newline.
fn answered(result: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(&json!({"result": result})).unwrap();
    [&br#"{"jsonrpc":"2.0","id":"c","#[..], &body[1..], b"\n"].concat()
}

/// A result whose 123 is written in its frame from byte 510, as a JSON
/// number after its key's colon, or after a tab's escape in a string, so
/// that the frame's first 512 bytes end in 12; and where its first piece
/// ends: before that colon, or before the x that precedes the escape.
fn straddling(number: bool) -> (Value, usize) {
    let result = |pad: &str| match number {
        true => json!({"content": [{"type": "text", "text": "n"}],
                       "structuredContent": {"a": pad, "n": 123}}),
        false => json!({"content": [{"type": "text", "text": format!("{pad}\t123")}]}),
    };
    let (marker, from, ends): (&[u8], _, _) = match number {
        true => (b":123}", 509, 509),
        false => (b"\\t123", 508, 507),
    };
    let unpadded = answered(&result(""));
    let at = unpadded.windows(marker.len()).position(|at| at == marker);
    (result(&"x".repeat(from - at.unwrap())), ends)
}

/// What one write of POSIX's least `PIPE_BUF` takes.
const ATOMIC: usize = 512;

/// `serve` of the plan `sealed` sealed at `digest`, as a harness starts it,
/// greeted and listed and then calling, its output a pipe filled but for
/// one write's room once the greeting and listing are read; and its
/// harness shell ended, which cancels the attempt, once the broker has
/// written there: what the pipe holds past what filled it once the broker
/// has ended.
fn one_write(sealed: &Sealed, digest: &str) -> Vec<u8> {
    let frames = greeting();
    let (input, mut asking) = std::io::pipe().unwrap();
    let (mut answers, mut output) = std::io::pipe().unwrap();
    let mut command = super::protocol::harnessing(sealed, digest);
    let held = Stdio::from(output.try_clone().unwrap());
    let errors = tempfile::NamedTempFile::new().unwrap();
    let said = Stdio::from(errors.reopen().unwrap());
    command.stdin(input).stdout(held).stderr(said);
    let mut shell = command.spawn().unwrap();
    drop(command);
    for frame in &frames[..3] {
        writeln!(asking, "{frame}").unwrap();
    }
    let mut told = || std::fs::read_to_string(errors.path()).unwrap_or_default();
    heard_within(&mut answers, Some(2), (&mut shell, &mut told));
    let waiting = filled(&mut output).len() - 4096;
    answers.read_exact(&mut [0; 4096]).unwrap();
    drop(output);
    writeln!(asking, "{}", frames[3]).unwrap();
    let until = Instant::now() + Duration::from_secs(20);
    while rustix::io::ioctl_fionread(&answers).unwrap() as usize <= waiting {
        assert!(
            Instant::now() < until,
            "the broker wrote nothing: {}",
            told()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    std::thread::sleep(Duration::from_millis(200));
    shell.kill().unwrap();
    shell.wait().unwrap();
    let mut held = heard_within(&mut answers, None, (&mut shell, &mut told));
    drop(asking);
    held.split_off(waiting)
}

/// What `answers` holds through its `lines`th newline, or through its end
/// where `lines` is none, read within the suite's wait for a broker
/// ([`super::TALK_MAX`]) from the call, made as the broker starts, so past
/// the startup's deadline that bounds the box's start and the server's
/// answers from it, the deadline read again before every read, while the
/// broker's harness shell `shell` runs,
/// or, read to its end, however long it has run. A broker that ends before
/// those lines, or says too little or too much in time, fails the test,
/// naming the shell's status and the broker's standard error, as `told`
/// reads it, rather than leaving it waiting on a pipe the test holds a
/// writer of (the controller's findings on U6f1b's fourth and fifth runs).
fn heard_within(
    answers: &mut PipeReader,
    lines: Option<usize>,
    (shell, told): (&mut std::process::Child, &mut dyn FnMut() -> String),
) -> Vec<u8> {
    let until = Instant::now() + super::TALK_MAX;
    let (mut heard, mut newlines) = (Vec::new(), 0);
    let mut failed = |why: &str, shell: &mut std::process::Child, bytes: usize| -> ! {
        let status = shell.try_wait();
        panic!("{why}: {status:?} after {bytes} bytes\n{}", told())
    };
    while lines.is_none_or(|lines| newlines < lines) {
        if Instant::now() >= until {
            let why = "the broker fell silent or said on past its bound";
            failed(why, shell, heard.len());
        }
        let mut polled = [rustix::event::PollFd::new(
            &*answers,
            rustix::event::PollFlags::IN,
        )];
        let wait = rustix::event::Timespec {
            tv_sec: 0,
            tv_nsec: 50_000_000,
        };
        if rustix::event::poll(&mut polled, Some(&wait)).unwrap_or(0) > 0 {
            let mut byte = [0];
            match (answers.read(&mut byte), lines) {
                (Ok(1), _) => {
                    heard.push(byte[0]);
                    newlines += usize::from(byte[0] == b'\n');
                }
                (Ok(_), None) => return heard,
                (read, Some(_)) | (read @ Err(_), None) => {
                    let why = format!("the broker's output ended: {read:?}");
                    failed(&why, shell, heard.len())
                }
            }
        } else if lines.is_some() && shell.try_wait().unwrap().is_some() {
            failed("the broker ended", shell, heard.len());
        }
    }
    heard
}

#[test]
fn an_answer_its_harness_takes_one_write_of_holds_no_part_of_a_number() {
    // 0012 bound, the server answers the call with 123 written from byte
    // 510 of the harness's frame: as a JSON number, read raw, and in a
    // string after a tab's escape, read decoded. The whole frame passes
    // the guard, as 123; its first 512 bytes, ending in 12, would not. The
    // harness's output, a pipe with room for one write, holds the frame's
    // first piece alone, cut where no number or escape spans it, which the
    // guard passes; cancelled there by its harness shell's end, the broker
    // writes nothing more, and its ledger closes with the timeout cause, the
    // call succeeded (U6f1b; MB4; operator ruling 2026-10-13).
    let bindings = bound("0012");
    let guard = Guard::new(&bindings);
    for (case, number) in [("number", true), ("escaped", false)] {
        let (result, ends) = straddling(number);
        let reply = json!({"jsonrpc": "2.0", "id": 3, "result": result});
        let script = format!("{SPEAKS}printf '%s\\n' '{reply}'\nread -r line\n");
        let Some((sealed, digest)) = unheld(|| serving(&script, "isolated"), &["0012"]) else {
            return;
        };
        std::fs::write(sealed.path("store/secrets.env"), "DOCS_TOKEN=0012\n").unwrap();
        let frame = answered(&result);
        let guarded = (guard.json(&frame), guard.json(&frame[..ATOMIC]));
        assert_eq!((case, guarded), (case, (true, false)));
        assert_eq!(
            (case, one_write(&sealed, &digest)),
            (case, frame[..ends].to_vec())
        );
        let piece = &frame[..ends];
        assert_eq!(
            (case, guard.json(piece), guard.text(piece)),
            (case, true, true)
        );
        let one = vec![("resolve".to_string(), Some(Outcome::Succeeded))];
        let timed_out = Some(Disposition::Failed(Failure::Timeout));
        assert_eq!(
            (case, sealed_ledger(&sealed)),
            (case, (one, timed_out, None))
        );
        assert!(within(|| boxed(&sealed).is_empty()));
    }
}

/// `serve` of the plan `sealed` sealed at `digest`, its harness saying
/// `frames` and reading `answers` frames, over pipes the test holds an end
/// of beside the broker's, so that their status flags, which every holder
/// of each shares, are read before it starts and once it has ended: its
/// exit, and the flags of its standard input, output and error then.
fn flagged(
    sealed: &Sealed,
    digest: &str,
    (frames, answers): (&[Value], usize),
) -> (Option<i32>, [Vec<OFlags>; 3]) {
    let (input, mut asking) = std::io::pipe().unwrap();
    let (mut answered, output) = std::io::pipe().unwrap();
    let (mut said, errors) = std::io::pipe().unwrap();
    let ends: [&dyn std::os::fd::AsFd; 3] = [&input, &output, &errors];
    let flags = || ends.map(|end| rustix::fs::fcntl_getfl(end).unwrap());
    let before = flags();
    let held = |end: &dyn std::os::fd::AsFd| Stdio::from(end.as_fd().try_clone_to_owned().unwrap());
    let mut command = super::protocol::harnessing(sealed, digest);
    command
        .stdin(held(&input))
        .stdout(held(&output))
        .stderr(held(&errors));
    let mut child = command.spawn().unwrap();
    drop(command);
    for frame in frames {
        writeln!(asking, "{frame}").unwrap();
    }
    let mut told = || String::from_utf8_lossy(&left(&mut said)).into_owned();
    heard_within(&mut answered, Some(answers), (&mut child, &mut told));
    drop(asking);
    let code = exited_within(&mut child, &mut told);
    let after = flags();
    (code, [0, 1, 2].map(|end| vec![before[end], after[end]]))
}

/// The exit of `serve` started as `child`, within twenty seconds; else it
/// is killed, and the test fails naming its status and its standard error,
/// as `told` reads it.
fn exited_within(child: &mut std::process::Child, told: &mut dyn FnMut() -> String) -> Option<i32> {
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status.code();
        }
        if Instant::now() >= until {
            super::killed(child.id());
            panic!("the broker did not end: {:?}\n{}", child.wait(), told());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn the_brokers_standard_streams_keep_their_status_flags_whether_served_or_failed() {
    // The broker waits for its standard input, output and error by poll,
    // and never changes the status flags every holder of each shares: they
    // read the same once a served session has closed Clean and once a
    // failed one has ended with its cause (U6f1b; MB3).
    let Some((clean, clean_digest)) = probing("", "isolated") else {
        return;
    };
    let Some((corrupt, corrupt_digest)) = serving("read -r line\necho no\n", "isolated") else {
        return;
    };
    for (case, sealed, digest, (code, answers)) in [
        ("clean", &clean, &clean_digest, (0, 3)),
        ("protocol", &corrupt, &corrupt_digest, (1, 0)),
    ] {
        let (ended, flags) = flagged(sealed, digest, (&greeting(), answers));
        let steady = flags.iter().all(|flags| flags[0] == flags[1]);
        assert_eq!((case, ended, steady), (case, Some(code), true), "{flags:?}");
        assert!(within(|| boxed(sealed).is_empty()));
    }
    closed_clean(&clean);
}

#[test]
fn the_server_reads_its_installed_bytes_as_they_are_and_admission_vouches_for_none() {
    // The package holds an unrelated credential canary and bytes installed
    // long before this attempt: the trusted server reads both as they are.
    // Masking covers the bound values alone, so the canary reaches the
    // broker's output unredacted, and admission says nothing of when or by
    // whom the old bytes were written (MB3's stated limits).
    let canary = "aws_secret_access_key=u6f1b-installation-canary";
    let old = "installed before this attempt's reach was established";
    let Some((sealed, _)) = speaking(SPEAKS) else {
        return;
    };
    let share = sealed.path("opt/docs/share");
    std::fs::create_dir_all(&share).unwrap();
    std::fs::write(share.join("credentials"), format!("{canary}\n")).unwrap();
    let held = share.join("old");
    std::fs::write(&held, format!("{old}\n")).unwrap();
    let long_ago = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
    let file = std::fs::File::options().write(true).open(&held).unwrap();
    file.set_modified(long_ago).unwrap();
    let probe = format!("cat '{0}/credentials' '{0}/old' >&2", share.display());
    let server = sealed.path("opt/docs/bin/docs-mcp");
    std::fs::write(&server, format!("#!/bin/sh\n{}", after(&probe))).unwrap();
    let mut plan = sealed.authored();
    set(&mut plan, "/retained", json!(false));
    let plan = sealed.observing(plan);
    let digest = sealed.seal_bytes(plan.to_string().as_bytes());
    let ran = conversed(&sealed, &digest, &greeting(), 3);
    let read = relayed(&format!("{canary}\n{old}\n"));
    assert_eq!((ran.0, ran.1.len(), ran.3), (Some(0), 3, read));
    let mut kept = String::new();
    std::fs::File::open(&held)
        .unwrap()
        .read_to_string(&mut kept)
        .unwrap();
    assert_eq!(kept, format!("{old}\n"));
    assert!(within(|| boxed(&sealed).is_empty()));
}

/// `/dev/full`, which poll finds writable and which refuses every write
/// with ENOSPC, as a standard stream; Linux's alone, as this suite is.
fn full() -> Stdio {
    let device = std::fs::File::options().write(true).open("/dev/full");
    Stdio::from(device.unwrap())
}

#[test]
fn a_standard_output_on_a_full_device_fails_the_session_undelivered() {
    // The harness greets the broker, whose standard output is /dev/full: its
    // answer is refused, never taken, so the session fails with the delivery
    // cause, said last on its standard error, and its ledger closes with it,
    // never Clean, as it would where the harness had closed its end
    // (operator ruling 2026-10-14).
    let Some((sealed, digest)) = probing("echo 'server says this' >&2", "isolated") else {
        return;
    };
    let mut command = super::protocol::harnessing(&sealed, &digest);
    command.stdin(Stdio::piped()).stdout(full());
    let child = command.stderr(Stdio::piped()).spawn().unwrap();
    let frames: String = greeting()
        .iter()
        .map(|frame| format!("{frame}\n"))
        .collect();
    let ran = super::bounded(child, frames.as_bytes());
    let said = (ran.status.code(), String::from_utf8(ran.stderr).unwrap());
    assert_eq!(said, super::ended(Failure::Undelivered));
    let undelivered = Some(Disposition::Failed(Failure::Undelivered));
    assert_eq!(sealed_ledger(&sealed), (vec![], undelivered, None));
    assert!(within(|| boxed(&sealed).is_empty()));
}

#[test]
fn a_standard_error_on_a_full_device_fails_a_served_session_undelivered() {
    // The broker's standard error is /dev/full, and its server says a line
    // there once called: the session is served whole, its call succeeded,
    // but the server's tail is refused, never taken, so the session fails
    // with the delivery cause and its ledger closes with it, never Clean
    // (operator ruling 2026-10-14).
    let Some((sealed, digest)) = probing("echo 'server says this' >&2", "isolated") else {
        return;
    };
    let ran = talking_beside(&sealed, &digest, (&greeting(), 3), full()).ended();
    assert_eq!((ran.0, ran.1.len(), ran.2), (Some(1), 3, String::new()));
    let one = vec![("resolve".to_string(), Some(Outcome::Succeeded))];
    let undelivered = Some(Disposition::Failed(Failure::Undelivered));
    assert_eq!(sealed_ledger(&sealed), (one, undelivered, None));
    assert!(within(|| boxed(&sealed).is_empty()));
}
