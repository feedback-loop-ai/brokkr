//! Fix round 1052g4 (#1052): the `sandbox-policy` span refuses a line
//! holding a break or control character before it skips anything, skips
//! only ASCII space, tab and a `#` after them, and trims every value with
//! ASCII space and tab only, so no line the gate skips or trims can hold
//! what a YAML reader reads as structure.

#![cfg(unix)]

use super::super::composition::Composition;
use super::super::{Mode, Release, SANDBOX_POLICY};
use super::composed::{gate, refuses_planted, with_sandbox_mode};
use super::measured_dump;

/// The measured 0.2.1 tree with the seat's read-only `sandbox-policy`
/// mode written into its config block, as the real composed tree carries
/// it.
fn policy_dump() -> String {
    with_sandbox_mode(&measured_dump(), "read-only")
}

/// S1: a span line carrying a character a YAML reader may break on
/// refuses `Unreadable` before it is skipped, whether the character sits
/// in a comment, a skipped-looking line or a line of its own. A comment
/// whose text after `#` is non-ASCII is the next test's skip case.
#[test]
fn a_policy_span_line_holding_a_break_or_control_is_refused() {
    let dump = policy_dump();
    for (what, from, to) in [
        (
            "a CR comment after config",
            "  config:\n",
            "  config:\n# c\r    mode: danger-full-access\n",
        ),
        (
            "a CR comment after the mode",
            "    mode: read-only\n",
            "    mode: read-only\n    # c\r    allowNetwork: true\n",
        ),
        (
            "a CR comment after the mode at column zero",
            "    mode: read-only\n",
            "    mode: read-only\n# c\r    mode: danger-full-access\n",
        ),
        (
            "a CR line before config",
            "  config:\n",
            "#\r  disabled: true\n  config:\n",
        ),
        (
            "a CR comment before config",
            "  config:\n",
            "  # c\r  disabled: true\n  config:\n",
        ),
        (
            "a NEL comment before config",
            "  config:\n",
            "#\u{85}  disabled: true\n  config:\n",
        ),
        (
            "a VT line after the mode",
            "    mode: read-only\n",
            "    mode: read-only\n\u{b}\n",
        ),
        (
            "a FF line after the mode",
            "    mode: read-only\n",
            "    mode: read-only\n\u{c}\n",
        ),
        (
            "a NUL line after the mode",
            "    mode: read-only\n",
            "    mode: read-only\n\u{0}\n",
        ),
        (
            "a DEL line after the mode",
            "    mode: read-only\n",
            "    mode: read-only\n\u{7f}\n",
        ),
        (
            "a C1 line after the mode",
            "    mode: read-only\n",
            "    mode: read-only\n\u{80}\n",
        ),
        (
            "an LS line after the mode",
            "    mode: read-only\n",
            "    mode: read-only\n\u{2028}\n",
        ),
        (
            "a PS line after the mode",
            "    mode: read-only\n",
            "    mode: read-only\n\u{2029}\n",
        ),
        (
            "a BOM line after the mode",
            "    mode: read-only\n",
            "    mode: read-only\n\u{feff}\n",
        ),
    ] {
        refuses_planted(&dump, what, from, to);
    }
}

/// S1: only ASCII space and tab make a line skip material. A line of NBSP,
/// or NBSP then `#`, is not a blank or a comment line to the span, so it
/// refuses instead of hiding structure behind it.
#[test]
fn a_policy_span_line_of_only_non_ascii_space_is_refused() {
    let dump = policy_dump();
    for (what, planted) in [
        ("an NBSP after six spaces", "      \u{a0}\n"),
        ("an NBSP then a comment", "    \u{a0}#x: 1\n"),
        ("a column-zero NBSP", "\u{a0}\n"),
        ("an NBSP then a key at column zero", "\u{a0}#x: 1\n"),
        ("an EM SPACE line", "\u{2003}\n"),
    ] {
        refuses_planted(
            &dump,
            what,
            "    mode: read-only\n",
            &format!("    mode: read-only\n{planted}"),
        );
    }
}

/// S2: the config block closes on an admitted two-space key, so a `mode:`
/// under `disabled: false` is not under `config:` and refuses. No YAML
/// reader accepts that shape.
#[test]
fn a_policy_span_refuses_a_mode_after_a_two_space_key_closed_config() {
    let dump = policy_dump();
    let closed = dump.replace(
        "  config:\n    mode: read-only\n",
        "  config:\n  disabled: false\n    mode: read-only\n",
    );
    assert_ne!(closed, dump);
    assert_eq!(
        gate(closed, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable)
    );
}

/// S2: a two-space key after the mode closes a block that has already
/// given its mode, so that order still passes: config, mode, then
/// `disabled: false`.
#[test]
fn a_policy_span_admits_a_mode_before_a_later_two_space_key() {
    let dump = policy_dump();
    let ordered = dump.replace(
        "  config:\n    mode: read-only\n",
        "  config:\n    mode: read-only\n  disabled: false\n",
    );
    assert_ne!(ordered, dump);
    assert_eq!(gate(ordered, Release::V0_2_1, Mode::ReadOnly), Ok(()));
}

/// S1: a tab-only line and a comment line are still skipped, and a comment
/// whose text after `#` holds non-ASCII characters is comment content, not
/// a refused line: the tree passes with the mode behind it.
#[test]
fn a_policy_span_still_skips_ascii_space_and_a_non_ascii_comment() {
    let dump = policy_dump();
    for (what, separator) in [
        ("a tab-only line", "\t\n"),
        (
            "a comment holding a patch path",
            "# /var/tmp/patch\u{e9}\u{a0}.yml\n",
        ),
        ("a blank then a comment", "\n# c\n"),
    ] {
        let mutated = dump.replace(
            "  config:\n    mode: read-only\n",
            &format!("  config:\n{separator}    mode: read-only\n"),
        );
        assert_ne!(mutated, dump, "{what}");
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Ok(()),
            "{what}"
        );
    }
}

/// S1: the value trim is ASCII only, so a non-ASCII-padded word is not the
/// word the gate compares. The mode refuses as the padded scalar; the
/// disabled word is not `false`; and the `- id:` value no longer names a
/// measured row.
#[test]
fn a_policy_span_refuses_a_non_ascii_padded_value() {
    let dump = policy_dump();
    let mode = dump.replace("    mode: read-only\n", "    mode: read-only\u{a0}\n");
    assert_ne!(mode, dump);
    assert_eq!(
        gate(mode, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Mode {
            composed: "read-only\u{a0}".into(),
            expected: "read-only",
        })
    );
    let disabled = dump.replace(
        "    mode: read-only\n",
        "    mode: read-only\n  disabled: false\u{a0}\n",
    );
    assert_ne!(disabled, dump);
    assert_eq!(
        gate(disabled, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::PolicyDisabled {
            id: SANDBOX_POLICY.into(),
        })
    );
    let id = dump.replace("- id: sandbox-policy\n", "- id: sandbox-policy\u{a0}\n");
    assert_ne!(id, dump);
    assert_eq!(
        gate(id, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unmeasured {
            id: "sandbox-policy\u{a0}".into(),
            name: "@deepseek-ai/dsh-sandbox-policy".into(),
            release: Release::V0_2_1.label(),
        })
    );
}

/// S1: the ASCII trim still strips ASCII spaces and tabs, so a value the
/// dump pads with them is the word the gate compares.
#[test]
fn a_policy_span_trims_ascii_padding() {
    let dump = policy_dump();
    let id = dump.replace("- id: sandbox-policy\n", "- id: sandbox-policy  \n");
    assert_ne!(id, dump);
    assert_eq!(gate(id, Release::V0_2_1, Mode::ReadOnly), Ok(()));
    let mode = dump.replace("    mode: read-only\n", "    mode: read-only\t \n");
    assert_ne!(mode, dump);
    assert_eq!(gate(mode, Release::V0_2_1, Mode::ReadOnly), Ok(()));
    let disabled = dump.replace(
        "    mode: read-only\n",
        "    mode: read-only\n  disabled: false  \n",
    );
    assert_ne!(disabled, dump);
    assert_eq!(gate(disabled, Release::V0_2_1, Mode::ReadOnly), Ok(()));
    let name = dump.replace(
        "  name: '@deepseek-ai/dsh-sandbox-policy'\n",
        "  name: '@deepseek-ai/dsh-sandbox-policy'  \n",
    );
    assert_ne!(name, dump);
    assert_eq!(gate(name, Release::V0_2_1, Mode::ReadOnly), Ok(()));
}
