//! The #1052 part 1 follow-ups: a hands launch reads its release once
//! (F4), the overlay's bounded reader reads every row the writer emits,
//! 0.2.2's programmatic-executor pin included (F5), and the composed-tree
//! gate judges the sandbox mode the seat actually runs under.

#![cfg(unix)]

use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::json;

use super::super::super::tests::{dsh_enabled_input, synthetic_dsh_composite};
use super::super::super::{dsh_launch_with, route_overlay};
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
pub(super) fn gate(dump: String, release: Release, mode: Mode) -> Result<(), Composition> {
    composed_with(Command::new("dsh"), release, mode, move |_| {
        Ok(Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: dump.into_bytes(),
            stderr: Vec::new(),
        })
    })
}

/// Plant `to` in place of `from` and require the span to refuse it as
/// unreadable. One home for the refusal cases the whole-span tests share,
/// so this round's module plants its lines through the same helper.
pub(super) fn refuses_planted(dump: &str, what: &str, from: &str, to: &str) {
    let mutated = dump.replace(from, to);
    assert_ne!(mutated, dump, "{what}");
    assert_eq!(
        gate(mutated, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable),
        "{what}"
    );
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

/// C6: the resume path's read is the same read. A hands seat with an offer
/// and a declared composite spawns `--version` exactly once — the rows'
/// read, reused to qualify the offer — never a second probe.
#[test]
fn a_hands_resume_reads_its_release_from_the_launch_not_a_second_probe() {
    let (_env, _dir, root, log) = probe_home();
    let dsh = super::super::super::tests::executable(&root, "dsh-resume", ONE_READ_DSH);
    let digest = "0".repeat(64);
    let enabled = dsh_enabled_input("0.2.1-rc.2", &digest, &root);
    let mut input = sealed(SealedBoundary::Namespace, Some("read-only"));
    input["workdir"] = json!(root);
    for key in ["boundary", "hands", "resume_context"] {
        input[key] = enabled[key].clone();
    }
    dsh_launch_with(
        dsh.to_str().unwrap(),
        &[],
        root.to_str().unwrap(),
        Some("session-019c4b7e"),
        &input,
        |_| Ok(synthetic_dsh_composite(&digest)),
    )
    .expect("the resume-capable hands launch is planned");
    let probes = std::fs::read_to_string(&log).expect("the fake recorded its invocations");
    assert_eq!(
        probes.lines().filter(|line| *line == "--version").count(),
        1,
        "the resume reuses the one read: {probes}"
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

/// F2: the bounded reader requires the 0.2.2 programmatic-executor pin
/// because the release's row table names it after the sandbox row. A block
/// with that row removed still names the sandbox row, so only the release
/// table can want it: the reader refuses before the pin-less overlay runs.
#[test]
fn the_0_2_2_reader_requires_the_ptc_pin_after_the_sandbox_row() {
    let block = pinned_block(Release::V0_2_2, Mode::ReadOnly);
    let without = block.replace("- id: tool-executor-ptc\n  disabled: true\n", "");
    assert_ne!(without, block, "the fixture drops the ptc pin");
    let refusal = route_overlay::hands(without.as_bytes(), Release::V0_2_2)
        .expect_err("the release pins the dropped row");
    assert_eq!(
        refusal.to_string(),
        format!(
            "refusing to invoke the dsh driver: the hands rows carry no row '{PTC_ROW}', which \
             the measured hands set names"
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

/// C1 (a and a2): the column-zero rule runs FIRST, in or out of the
/// `sandbox-policy` block, so a line 8daf25b66's reader refused is never
/// swallowed by the block. The same lines before the row refuse as before.
#[test]
fn a_column_zero_line_after_the_mode_is_refused_as_the_old_gate_refused_it() {
    let dump = with_sandbox_mode(&measured_dump(), "read-only");
    for planted in [
        "- insert:\nstray: line\n",
        "- name: '@evil/unmeasured'\n  disabled: false\n",
    ] {
        let after = dump.replace(
            "    mode: read-only\n",
            &format!("    mode: read-only\n{planted}"),
        );
        assert_ne!(after, dump, "{planted}");
        assert_eq!(
            gate(after, Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{planted} after the mode"
        );
        let before = dump.replace(
            "- id: sandbox-policy\n",
            &format!("{planted}- id: sandbox-policy\n"),
        );
        assert_ne!(before, dump, "{planted}");
        assert_eq!(
            gate(before, Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{planted} before the row"
        );
    }
}

/// C1 (b), under 1052g3's closed span: the `sandbox-policy` row's whole
/// span is read as a closed grammar, so a `mode:` under any later line is
/// not read as the composed mode; a second mode, a deeper line, a `Mode:`
/// spelling and any other key refuse the tree.
#[test]
fn a_sandbox_policy_span_refuses_a_second_mode_and_any_other_key() {
    let dump = with_sandbox_mode(&measured_dump(), "read-only");
    // `  other:` is not an admitted key, so the `mode:` behind it is never
    // read as the composed mode under the closed span.
    let closed = dump.replace("    mode: read-only\n", "  other:\n    mode: read-only\n");
    assert_ne!(closed, dump);
    assert_eq!(
        gate(closed, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable)
    );
    // Another key inside the block is not one of its admitted lines.
    let other_key = dump.replace("    mode: read-only\n", "    timeout: 1\n");
    assert_ne!(other_key, dump);
    assert_eq!(
        gate(other_key, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable)
    );
    for planted in ["      mode: read-only\n", "    Mode: danger-full-access\n"] {
        let mutated = dump.replace(
            "    mode: read-only\n",
            &format!("    mode: read-only\n{planted}"),
        );
        assert_ne!(mutated, dump, "{planted}");
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{planted}"
        );
    }
    let twice = dump.replace(
        "    mode: read-only\n",
        "    mode: read-only\n    mode: workspace-write\n",
    );
    assert_eq!(rows(twice.as_bytes()).err(), Some(Composition::Unreadable));
}

/// F1: every line shape the review planted after the mode refuses the
/// tree. A blank, comment or whitespace line is skipped, so it never ends
/// the `config:` block; the second mode or extra key behind it is still
/// read, and refused.
#[test]
fn every_unadmitted_line_in_the_sandbox_policy_span_refuses() {
    let dump = with_sandbox_mode(&measured_dump(), "read-only");
    for (what, planted) in [
        (
            "a blank then a second mode",
            "\n    mode: danger-full-access\n",
        ),
        (
            "a comment then a second mode",
            "# c\n    mode: danger-full-access\n",
        ),
        (
            "whitespace then a second mode",
            "  \n    mode: danger-full-access\n",
        ),
        ("a blank then another key", "\n    allowNetwork: true\n"),
        ("a three-space key", "   x: 1\n"),
        (
            "a flow-style config",
            "  config: {mode: danger-full-access}\n",
        ),
        ("a second empty config", "  config:\n"),
        ("a mode with no space", "    mode:read-only\n"),
        ("a quoted disabled key", "  \"disabled\": true\n"),
        ("a spaced disabled key", "  disabled : true\n"),
        ("a second name", "  name: another\n"),
        (
            "a second disabled",
            "  disabled: false\n  disabled: false\n",
        ),
    ] {
        refuses_planted(
            &dump,
            what,
            "    mode: read-only\n",
            &format!("    mode: read-only\n{planted}"),
        );
    }
    // A mode before its `config:` is not after it.
    let early = dump.replace("  config:\n", "    mode: read-only\n  config:\n");
    assert_ne!(early, dump);
    assert_eq!(
        gate(early, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable)
    );
}

/// F1: a blank, comment or whitespace-only line between `config:` and the
/// mode is skipped, so the block stays open and the mode is still read. The
/// tree passes, which the old indent close would turn into `NoMode`.
#[test]
fn a_skipped_line_does_not_close_the_sandbox_config() {
    let dump = with_sandbox_mode(&measured_dump(), "read-only");
    for separator in ["\n", "# c\n", "  # c\n", "  \n", "\t\n"] {
        let mutated = dump.replace(
            "  config:\n    mode: read-only\n",
            &format!("  config:\n{separator}    mode: read-only\n"),
        );
        assert_ne!(mutated, dump, "{separator:?}");
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Ok(()),
            "{separator:?}"
        );
    }
}

/// C2: the `sandbox-policy` row is refused unless its `disabled:` is absent
/// or exactly `false`, whatever order the flag takes and whether or not the
/// bounded parse can evaluate it; an explicit `false` passes.
#[test]
fn a_disabled_sandbox_policy_row_is_refused() {
    let dump = with_sandbox_mode(&measured_dump(), "read-only");
    let expected = Composition::PolicyDisabled {
        id: SANDBOX_POLICY.into(),
    };
    for (anchor, planted) in [
        (
            "    mode: read-only\n",
            "    mode: read-only\n  disabled: true\n",
        ),
        ("  config:\n", "  disabled: true\n  config:\n"),
        (
            "    mode: read-only\n",
            "    mode: read-only\n  disabled: !!js process.platform === 'win32'\n",
        ),
    ] {
        let mutated = dump.replace(anchor, planted);
        assert_ne!(mutated, dump, "{planted}");
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Err(expected.clone()),
            "{planted}"
        );
    }
    assert_eq!(
        expected.to_string(),
        "refusing to invoke the dsh driver: the composed dsh tree carries the 'sandbox-policy' \
         row disabled, so dsh's own sandbox policy is not in force and the mode the gate reads \
         is not the mode the seat runs under (#896 D1)"
    );
    let explicit = dump.replace(
        "    mode: read-only\n",
        "    mode: read-only\n  disabled: false\n",
    );
    assert_ne!(explicit, dump);
    assert_eq!(gate(explicit, Release::V0_2_1, Mode::ReadOnly), Ok(()));
}

/// F3: the `sandbox-policy` `disabled:` line is recognized without the
/// space after its colon, as the reader has always spelled it. A word
/// jammed against the colon and a bare key both name a disabled policy, so
/// both refuse `PolicyDisabled`, not an unread line.
#[test]
fn a_sandbox_policy_disabled_line_is_read_with_or_without_its_space() {
    let dump = with_sandbox_mode(&measured_dump(), "read-only");
    let expected = Composition::PolicyDisabled {
        id: SANDBOX_POLICY.into(),
    };
    for planted in ["  disabled:true\n", "  disabled:\n"] {
        let mutated = dump.replace("  config:\n", &format!("{planted}  config:\n"));
        assert_ne!(mutated, dump, "{planted}");
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Err(expected.clone()),
            "{planted}"
        );
    }
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
