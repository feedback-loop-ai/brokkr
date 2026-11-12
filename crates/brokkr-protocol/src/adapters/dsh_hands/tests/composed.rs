//! The #1052 part 1 follow-ups: a hands launch reads its release once
//! (F4), the overlay's bounded reader reads every row the writer emits,
//! 0.2.2's programmatic-executor pin included (F5), and the composed-tree
//! gate judges the sandbox mode the seat actually runs under.

#![cfg(unix)]

use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Output};

use super::super::super::route_overlay;
use super::super::composition::{composed_with, judge_mode, rows, Composition};
use super::super::{hands_rows, Mode, Release, Selection, PTC_ROW, SANDBOX_POLICY};
use super::{invoke_hands, measured_dump, measured_dump_0_2_2, s, sealed};
use crate::env_guard::EnvGuard;
use crate::native_controls::SealedBoundary;

/// A fake dsh that records the first argument of every invocation, answers
/// the one version probe, composes a valid read-only hands tree, and lets
/// the session exit cleanly. The recorded first words make the number of
/// `--version` spawns a fact the test reads, not a claim.
const ONE_READ_DSH: &str = r#"#!/bin/sh
printf '%s\n' "$1" >> "$BROKKR_TEST_DSH_PROBES"
case "$1" in
  --version|-V|-v) printf '0.2.1-rc.2\n'; exit 0 ;;
esac
for a in "$@"; do
  if [ "$a" = "--dump-config" ]; then
    printf '%s\n' \
      "- id: sandbox-policy" "  name: '@deepseek-ai/dsh-sandbox-policy'" \
      "  config:" "    mode: read-only" \
      "- id: brokkr-hands" "  name: '@deepseek-ai/dsh-mcp-client'"
    exit 0
  fi
done
exit 0
"#;

/// The pinned rows (everything before the `brokkr-hands` insert) a seat on
/// `release` writes, read through the production writer so the test sees
/// the exact bytes the reader is asked to admit.
fn pinned_block(release: Release, mode: Mode) -> String {
    let rows = hands_rows(release, mode, "/bin/brokkr", &s(&["hands", "serve"])).unwrap();
    rows.split("- insert:")
        .next()
        .expect("the writer always appends the insert")
        .to_string()
}

/// Judge a dump as the composed-tree gate would, through its closure seam.
fn gate(dump: String, release: Release, mode: Mode) -> Result<(), Composition> {
    composed_with(Command::new("dsh"), Some(release), mode, move |_| {
        Ok(Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: dump.into_bytes(),
            stderr: Vec::new(),
        })
    })
}

/// A locked environment, a fresh temporary root and the probe log both
/// launch tests share: `HOME` and `DSH_HOME` under the root, and the fake
/// dsh's recorder named by `BROKKR_TEST_DSH_PROBES`. The guard and the
/// tempdir are returned so the caller holds them for the test's life.
fn probe_home() -> (EnvGuard, tempfile::TempDir, PathBuf, PathBuf) {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let home = root.join("home");
    std::fs::create_dir(&home).unwrap();
    let log = root.join("probes");
    env.set("HOME", &home);
    env.set("DSH_HOME", &home);
    env.set("BROKKR_TEST_DSH_PROBES", &log);
    (env, dir, root, log)
}

/// A measured dump with the seat's `sandbox-policy` mode written into its
/// one row, as the real composed tree carries it. The row shape is the one
/// dsh wrote (docs/evidence/adapters/dsh-hands-round2).
pub(super) fn with_sandbox_mode(dump: &str, mode: &str) -> String {
    let row = "- id: sandbox-policy\n  name: '@deepseek-ai/dsh-sandbox-policy'\n";
    let replaced = dump.replacen(row, &format!("{row}  config:\n    mode: {mode}\n"), 1);
    assert_ne!(replaced, dump, "the fixture names the sandbox row");
    replaced
}

/// F4: the launch's one `--version` read is the one both the rows and the
/// composed-tree gate use, so a hands launch spawns the probe exactly once.
/// The fake records every invocation; the three lines are the version
/// probe, the composition dump and the session.
#[test]
fn a_hands_launch_reads_its_release_exactly_once() {
    let (_env, _dir, root, log) = probe_home();
    let dsh = super::super::super::tests::executable(&root, "dsh-once", ONE_READ_DSH);
    let workdir = root.to_str().unwrap();
    let (_, result) = invoke_hands(
        dsh.to_str().unwrap(),
        workdir,
        sealed(SealedBoundary::Namespace, Some("read-only")),
        &[],
        false,
    );
    result.expect("the measured read-only tree is admitted");
    let probes = std::fs::read_to_string(&log).expect("the fake recorded its invocations");
    assert_eq!(
        probes.lines().filter(|line| *line == "--version").count(),
        1,
        "a hands launch probes its release once: {probes}"
    );
    assert_eq!(
        probes.lines().count(),
        3,
        "the one probe, the dump and the session: {probes}"
    );
}

/// F4: a hands seat qualifies a resume offer from the one read it already
/// made for its rows, never a second probe. The read is used even when no
/// binary could answer now, which is what makes the arms distinguishable.
#[test]
fn a_hands_seat_qualifies_a_resume_from_its_one_read() {
    let selection = Selection::Hands {
        rows: String::new(),
        mode: Mode::ReadOnly,
        release: Release::V0_2_1,
        observed: "0.2.1-rc.3".into(),
    };
    assert_eq!(
        selection.resume_version("/nonexistent/dsh"),
        Some("0.2.1-rc.3".to_string())
    );
}

/// F5: the pinned block 0.2.2's writer emits carries the programmatic
/// executor and the bounded reader admits the whole block. Because the
/// reader requires that release's pin, moving the reader before the pin is
/// appended makes the writer itself refuse.
#[test]
fn the_bounded_reader_admits_the_0_2_2_ptc_pin_it_writes() {
    let block = pinned_block(Release::V0_2_2, Mode::ReadOnly);
    assert!(
        block.contains(&format!("- id: {PTC_ROW}\n  disabled: true\n")),
        "{block}"
    );
    route_overlay::hands(block.as_bytes(), Release::V0_2_2)
        .expect("the whole 0.2.2 block is read, the ptc pin included");
    // 0.2.1 writes no such row, and its block is still the reader's one
    // shape: the requirement follows the release, never the reader alone.
    let older = pinned_block(Release::V0_2_1, Mode::ReadOnly);
    assert!(!older.contains(PTC_ROW), "{older}");
    route_overlay::hands(older.as_bytes(), Release::V0_2_1).expect("0.2.1's block is read");
}

/// F5: a 0.2.2 PTC pin carrying a key the one shape does not name is
/// refused by the bounded reader, by its id, so the pin is read and not
/// merely written.
#[test]
fn a_malformed_0_2_2_ptc_pin_is_refused_by_the_bounded_reader() {
    let block = pinned_block(Release::V0_2_2, Mode::ReadOnly);
    let malformed = block.replace(
        "- id: tool-executor-ptc\n  disabled: true\n",
        "- id: tool-executor-ptc\n  disabled: true\n  x: y\n",
    );
    assert_ne!(malformed, block, "the fixture mutates the ptc pin");
    let refusal = route_overlay::hands(malformed.as_bytes(), Release::V0_2_2)
        .expect_err("an extra key is not the pin's one shape");
    assert_eq!(
        refusal.to_string(),
        format!(
            "refusing to invoke the dsh driver: the hands rows carry row '{PTC_ROW}' in a shape \
             the measured hands set does not name"
        )
    );
}

/// The gate refuses a composed tree whose `sandbox-policy` mode is not the
/// seat's, even when every row and `disabled` flag is measured: the third
/// dsh mode never reaches a hands seat, and neither does the other allowed
/// word on the wrong seat.
#[test]
fn a_composed_mode_that_is_not_the_seats_is_refused() {
    for (composed, expected) in [
        ("danger-full-access", Mode::ReadOnly),
        ("workspace-write", Mode::ReadOnly),
        ("read-only", Mode::WorkspaceWrite),
    ] {
        let dump = with_sandbox_mode(&measured_dump(), composed);
        let parsed = rows(dump.as_bytes()).unwrap();
        let refusal = judge_mode(&parsed, expected).expect_err("the mode differs");
        assert_eq!(
            refusal,
            Composition::Mode {
                composed: composed.into(),
                expected: expected.word(),
            },
            "{composed} on {expected:?}"
        );
        assert_eq!(
            gate(dump, Release::V0_2_1, expected).err(),
            Some(Composition::Mode {
                composed: composed.into(),
                expected: expected.word(),
            }),
            "{composed} on {expected:?} through the gate"
        );
    }
    let text = Composition::Mode {
        composed: "danger-full-access".into(),
        expected: "read-only",
    }
    .to_string();
    assert_eq!(
        text,
        "refusing to invoke the dsh driver: the composed dsh tree's 'sandbox-policy' mode is \
         'danger-full-access', not the seat's 'read-only', so dsh's own sandbox would not be the \
         mode the office's access selected (#896 D1)"
    );
}

/// A composed tree with no `sandbox-policy` mode at all is refused: an
/// unread mode cannot be shown to be the seat's, whether the row is absent
/// or present without a `config.mode`.
#[test]
fn a_composed_tree_with_no_mode_is_refused() {
    let unread = measured_dump();
    assert_eq!(
        judge_mode(&rows(unread.as_bytes()).unwrap(), Mode::ReadOnly),
        Err(Composition::NoMode)
    );
    let absent = unread.replace(
        "- id: sandbox-policy\n  name: '@deepseek-ai/dsh-sandbox-policy'\n",
        "",
    );
    assert!(!absent.contains(SANDBOX_POLICY), "{absent}");
    assert_eq!(
        judge_mode(&rows(absent.as_bytes()).unwrap(), Mode::ReadOnly),
        Err(Composition::NoMode)
    );
    assert_eq!(
        gate(unread, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::NoMode)
    );
    assert_eq!(
        Composition::NoMode.to_string(),
        "refusing to invoke the dsh driver: the composed dsh tree carries no 'sandbox-policy' \
         mode, so the seat's sandbox mode cannot be read and the seat is not launched (#896 D1)"
    );
}

/// A `sandbox-policy` `config` block is read for exactly one `mode`: a
/// block carrying another key and no mode reads as no mode, and a second
/// mode is refused as an unreadable row.
#[test]
fn a_sandbox_config_reads_one_mode_and_refuses_a_second() {
    let dump = with_sandbox_mode(&measured_dump(), "read-only");
    let other = dump.replace("    mode: read-only\n", "    timeout: 1\n");
    let parsed = rows(other.as_bytes()).unwrap();
    let row = parsed
        .iter()
        .find(|row| row.id == SANDBOX_POLICY)
        .expect("the fixture names the sandbox row");
    assert_eq!(row.mode, None);
    assert_eq!(
        judge_mode(&parsed, Mode::ReadOnly),
        Err(Composition::NoMode)
    );
    let twice = dump.replace(
        "    mode: read-only\n",
        "    mode: read-only\n    mode: workspace-write\n",
    );
    assert_eq!(rows(twice.as_bytes()).err(), Some(Composition::Unreadable));
}

/// The measured trees still pass when the seat's own mode is written into
/// their `sandbox-policy` row: read-only 0.2.1, workspace-write 0.2.1 and
/// read-only 0.2.2 all compose and judge as before.
#[test]
fn the_measured_trees_still_pass_their_own_mode() {
    for (dump, release, mode) in [
        (measured_dump(), Release::V0_2_1, Mode::ReadOnly),
        (measured_dump(), Release::V0_2_1, Mode::WorkspaceWrite),
        (measured_dump_0_2_2(), Release::V0_2_2, Mode::ReadOnly),
    ] {
        let dump = with_sandbox_mode(&dump, mode.word());
        assert_eq!(gate(dump, release, mode), Ok(()), "{mode:?}");
    }
}

/// A tree with no threaded release keeps the old `UnknownRelease` refusal,
/// byte for byte: the variant is retained for the gate's own unread case.
#[test]
fn a_tree_with_no_threaded_release_keeps_the_unknown_release_words() {
    let dump = with_sandbox_mode(&measured_dump(), "read-only");
    let refused = composed_with(Command::new("dsh"), None, Mode::ReadOnly, move |_| {
        Ok(Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: dump.into_bytes(),
            stderr: Vec::new(),
        })
    })
    .unwrap_err();
    assert_eq!(refused, Composition::UnknownRelease);
    assert_eq!(
        refused.to_string(),
        "refusing to invoke the dsh driver: the composed dsh tree's release cannot be read as \
         one the hands rows were measured on (0.2.1-rc.2, 0.2.1-rc.3, 0.2.2-rc.1), so the seat \
         is admitted on no measured composition (#896 D1)"
    );
}

/// The launch itself carries the seat's mode to the gate: a read-only seat
/// whose composed tree says `danger-full-access` is refused before the
/// session spawns, and the fake records exactly the version probe and the
/// dump.
#[test]
fn a_launch_refuses_a_composed_mode_that_differs_from_the_seats() {
    let (_env, _dir, root, log) = probe_home();
    let dump = with_sandbox_mode(&measured_dump(), "danger-full-access");
    let dump_path = root.join("mode-dump.yml");
    std::fs::write(&dump_path, dump).unwrap();
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$1\" >> \"$BROKKR_TEST_DSH_PROBES\"\n\
         case \"$1\" in --version|-V|-v) printf '0.2.1-rc.2\\n'; exit 0 ;; esac\n\
         for a in \"$@\"; do [ \"$a\" = --dump-config ] && cat '{}' && exit 0; done\nexit 0\n",
        dump_path.display()
    );
    let dsh = super::super::super::tests::executable(&root, "dsh-mode", &script);
    let (_, result) = invoke_hands(
        dsh.to_str().unwrap(),
        root.to_str().unwrap(),
        sealed(SealedBoundary::Namespace, Some("read-only")),
        &[],
        false,
    );
    assert_eq!(
        result
            .err()
            .expect("the composed mode differs from the seat's"),
        Composition::Mode {
            composed: "danger-full-access".into(),
            expected: "read-only",
        }
        .to_string()
    );
    let probes = std::fs::read_to_string(&log).unwrap();
    assert_eq!(probes.lines().count(), 2, "no session spawns: {probes}");
}
