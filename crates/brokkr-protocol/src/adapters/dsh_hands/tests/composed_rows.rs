//! Fix round 1052g5 (#1052): the whole composed dump is read by one closed
//! row grammar. Every line of every row is refused for a break or control
//! before anything else, every row's two-space keys are closed to `name:`,
//! `disabled:`, `config:` and `inject:` each at most once, and the hands
//! row's `disabled:` is admitted only absent or exactly `false`.

#![cfg(unix)]

use super::super::composition::Composition;
use super::super::{Mode, Release, HANDS_ID, SANDBOX_POLICY};
use super::composed::{gate, with_sandbox_mode};
use super::measured_dump;

/// The measured 0.2.1 tree with the seat's read-only `sandbox-policy` mode
/// written into its config block, as the real composed tree carries it.
fn policy_dump() -> String {
    with_sandbox_mode(&measured_dump(), "read-only")
}

/// The hand-written `tool-bash` row of the measured fixture, with its one
/// disabled flag, in the exact bytes the fixture writes.
const TOOL_BASH: &str = "- id: tool-bash\n  name: '@deepseek-ai/dsh-tool-bash'\n  disabled: true\n";

/// The measured fixture's one hands row.
const HANDS: &str = "- id: brokkr-hands\n  name: '@deepseek-ai/dsh-mcp-client'\n";

/// Plant `line` directly after the `tool-bash` row's `disabled: true`.
fn on_tool_bash(dump: &str, line: &str) -> String {
    let mutated = dump.replacen(TOOL_BASH, &format!("{TOOL_BASH}{line}"), 1);
    assert_ne!(mutated, dump, "the fixture names tool-bash");
    mutated
}

/// Plant `line` directly after the hands row's `name:` line.
fn on_hands(dump: &str, line: &str) -> String {
    let mutated = dump.replacen(HANDS, &format!("{HANDS}{line}"), 1);
    assert_ne!(mutated, dump, "the fixture names the hands row");
    mutated
}

/// C1 (HIGH, security): the hands row's `disabled:` is admitted only when
/// it is absent or exactly `false` after the ASCII trim. A Unicode space is
/// not trimmed, so the padded word is not `false` and the judge's
/// `HandsDisabled` refuses it, in either position, with its unchanged text.
#[test]
fn the_hands_row_disabled_is_admitted_only_absent_or_exactly_false() {
    let dump = policy_dump();
    let disabled = Composition::HandsDisabled {
        id: HANDS_ID.into(),
    };
    for (what, line) in [
        ("an NBSP after the word", "  disabled: true\u{a0}\n"),
        ("an NBSP before the word", "  disabled: \u{a0}true\n"),
        ("an EM SPACE after the word", "  disabled: true\u{2003}\n"),
        ("an EM SPACE before the word", "  disabled: \u{2003}true\n"),
        (
            "an IDEOGRAPHIC SPACE after the word",
            "  disabled: true\u{3000}\n",
        ),
        (
            "an IDEOGRAPHIC SPACE before the word",
            "  disabled: \u{3000}true\n",
        ),
    ] {
        assert_eq!(
            gate(on_hands(&dump, line), Release::V0_2_1, Mode::ReadOnly),
            Err(disabled.clone()),
            "{what}"
        );
    }
    assert_eq!(
        gate(dump.clone(), Release::V0_2_1, Mode::ReadOnly),
        Ok(()),
        "an absent disabled is admitted"
    );
    assert_eq!(
        gate(
            on_hands(&dump, "  disabled: false\n"),
            Release::V0_2_1,
            Mode::ReadOnly
        ),
        Ok(()),
        "an explicit false is admitted"
    );
    assert_eq!(
        disabled.to_string(),
        "refusing to invoke the dsh driver: the composed dsh tree carries row 'brokkr-hands' \
         disabled, so the seat's one effectful tool would not be served (#896 D1)"
    );
}

/// C1 (HIGH, security): a break or control in the hands row's `disabled:`
/// line refuses earlier than the word rule, as `Unreadable`, by the
/// dump-wide rule, in either position.
#[test]
fn the_hands_row_refuses_a_break_or_control_before_its_word() {
    let dump = policy_dump();
    for (what, line) in [
        ("a VT after the word", "  disabled: true\u{b}\n"),
        ("a VT before the word", "  disabled: \u{b}true\n"),
        ("a FF after the word", "  disabled: true\u{c}\n"),
        ("a FF before the word", "  disabled: \u{c}true\n"),
        ("a NEL after the word", "  disabled: true\u{85}\n"),
        ("a NEL before the word", "  disabled: \u{85}true\n"),
        ("an LS after the word", "  disabled: true\u{2028}\n"),
        ("an LS before the word", "  disabled: \u{2028}true\n"),
        ("a CR and a space after the word", "  disabled: true\r \n"),
    ] {
        assert_eq!(
            gate(on_hands(&dump, line), Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
}

/// C2 (MEDIUM, ruling 9) and the dump-wide rule 1: every planted class is
/// carried inside a comment or an unread block, in five line positions, so
/// the refusal set is what refuses it and not the fallthrough. Every class
/// the round names is refused as `Unreadable`, before the line would be
/// skipped or read.
#[test]
fn the_whole_dump_refuses_a_break_or_control_on_every_line() {
    let dump = policy_dump();
    let classes = [
        ("a VT", '\u{b}'),
        ("a FF", '\u{c}'),
        ("a NUL", '\u{0}'),
        ("U+0001", '\u{1}'),
        ("DEL", '\u{7f}'),
        ("U+0080", '\u{80}'),
        ("U+009F", '\u{9f}'),
        ("NEL", '\u{85}'),
        ("LS", '\u{2028}'),
        ("PS", '\u{2029}'),
        ("a BOM", '\u{feff}'),
        ("U+FFFE", '\u{fffe}'),
        ("U+FFFF", '\u{ffff}'),
    ];
    for (what, class) in classes {
        let comment = format!("# c{class}  disabled: true\n");
        let near_policy = format!("# c{class}  disabled: true\n  config:\n    mode: read-only\n");
        let deeper = format!("    # c{class}  disabled: true\n");
        let unread = format!("  config:\n    key{class}: value\n");
        let cases = [
            ("before the first row", format!("{comment}{dump}")),
            (
                "on the policy row before config",
                dump.replacen("  config:\n    mode: read-only\n", &near_policy, 1),
            ),
            (
                "inside the policy config block",
                dump.replacen(
                    "    mode: read-only\n",
                    &format!("{deeper}    mode: read-only\n"),
                    1,
                ),
            ),
            ("on another row", on_tool_bash(&dump, &comment)),
            (
                "inside another row's unread block",
                on_tool_bash(&dump, &unread),
            ),
        ];
        for (position, mutated) in cases {
            assert_ne!(mutated, dump, "{what} {position}");
            assert_eq!(
                gate(mutated, Release::V0_2_1, Mode::ReadOnly),
                Err(Composition::Unreadable),
                "{what} {position}"
            );
        }
    }
}

/// C2 (MEDIUM, ruling 9): a non-ASCII-padded `name:` is not bound by
/// Unicode trim, so the padded word is not the measured package. The plain
/// `timer` row refuses by its padded name, and the policy row refuses the
/// same way.
#[test]
fn a_non_ascii_padded_name_is_not_trimmed() {
    let dump = policy_dump();
    let timer = dump.replacen(
        "  name: '@deepseek-ai/cordis-plugin-timer'\n",
        "  name: '@deepseek-ai/cordis-plugin-timer'\u{a0}\n",
        1,
    );
    assert_ne!(timer, dump, "the fixture names the timer row");
    assert_eq!(
        gate(timer, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unmeasured {
            id: "timer".into(),
            name: "'@deepseek-ai/cordis-plugin-timer'\u{a0}".into(),
            release: Release::V0_2_1.label(),
        })
    );
    let policy = dump.replacen(
        "  name: '@deepseek-ai/dsh-sandbox-policy'\n",
        "  name: '@deepseek-ai/dsh-sandbox-policy'\u{a0}\n",
        1,
    );
    assert_ne!(policy, dump, "the fixture names the policy row");
    assert_eq!(
        gate(policy, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unmeasured {
            id: SANDBOX_POLICY.into(),
            name: "'@deepseek-ai/dsh-sandbox-policy'\u{a0}".into(),
            release: Release::V0_2_1.label(),
        })
    );
}

/// C3 (LOW, ruling 9): the `name:` arm closes the config block, so a
/// `mode:` after it is not under `config:` and refuses. The row is
/// reordered rather than duplicated, so only the close can refuse it.
#[test]
fn a_mode_after_the_name_closed_the_policy_config_is_refused() {
    let dump = policy_dump();
    let reordered = "  config:\n  name: '@deepseek-ai/dsh-sandbox-policy'\n    mode: read-only\n";
    let mutated = dump.replacen(
        "  name: '@deepseek-ai/dsh-sandbox-policy'\n  config:\n    mode: read-only\n",
        reordered,
        1,
    );
    assert_ne!(mutated, dump);
    assert_eq!(
        gate(mutated, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable)
    );
}

/// S7: the row grammar covers every row, not only `sandbox-policy`. A
/// comment carrying a CR after `tool-bash`'s `disabled: true`, a quoted or
/// spaced key, a second `disabled:`, a continuation line and a merge key
/// all refuse, so a YAML reader's last-wins reading never reaches the gate.
/// Every case is reported, so one control shows each one refused only by
/// the grammar.
#[test]
fn the_s7_shapes_on_another_row_refuse() {
    let dump = policy_dump();
    let mut refused = Vec::new();
    for (what, planted) in [
        ("a CR comment after disabled", "# c\r  disabled: false\n"),
        ("a quoted disabled key", "  \"disabled\": false\n"),
        ("a spaced disabled key", "  disabled : false\n"),
        ("a second disabled", "  disabled: false\n"),
        ("a continuation after disabled", "    x\n"),
        ("a merge key", "  <<: {disabled: false}\n"),
    ] {
        let result = gate(
            on_tool_bash(&dump, planted),
            Release::V0_2_1,
            Mode::ReadOnly,
        );
        if result != Err(Composition::Unreadable) {
            refused.push((what, result));
        }
    }
    assert!(refused.is_empty(), "{refused:?}");
}

/// S7: before the first row only skip lines are admitted. A non-comment
/// line, and a leading malformed `-   id:` row whose block names the
/// third sandbox mode, refuse; a leading comment and blank line do not.
#[test]
fn a_line_before_the_first_row_that_is_not_a_skip_is_refused() {
    let dump = policy_dump();
    for (what, leading) in [
        ("a non-comment line", "stray: line\n"),
        (
            "a leading malformed row",
            "-   id: sandbox-policy\n  config:\n    mode: danger-full-access\n",
        ),
        (
            "a leading differently-spaced row",
            "- id:sandbox-policy\n  config:\n    mode: danger-full-access\n",
        ),
    ] {
        let mutated = format!("{leading}{dump}");
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
    assert_eq!(
        gate(
            format!("# banner\n\n{dump}"),
            Release::V0_2_1,
            Mode::ReadOnly
        ),
        Ok(())
    );
}

/// Rule 4: a row's two-space level is closed. An unknown key, an explicit
/// `?` key, a flow-style `config:` or `inject:`, and an `inject:` on the
/// policy row all refuse `Unreadable`.
#[test]
fn an_unadmitted_two_space_key_is_refused() {
    let dump = policy_dump();
    for (what, planted) in [
        ("an unknown key", "  x: y\n"),
        ("an explicit key", "  ? disabled\n"),
        ("a flow-style config", "  config: {}\n"),
        ("a flow-style inject", "  inject: {}\n"),
    ] {
        assert_eq!(
            gate(
                on_tool_bash(&dump, planted),
                Release::V0_2_1,
                Mode::ReadOnly
            ),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
    let injected = dump.replacen("  config:\n", "  inject:\n  config:\n", 1);
    assert_ne!(injected, dump);
    assert_eq!(
        gate(injected, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable),
        "the policy row admits no inject"
    );
}

/// Rule 4: each two-space key is admitted at most once per row, so no
/// last-wins reading exists. A second `name:`, `disabled:`, `config:` or
/// `inject:` refuses `Unreadable`.
#[test]
fn a_repeated_two_space_key_is_refused() {
    let dump = policy_dump();
    for (what, planted) in [
        ("a second name", "  name: '@deepseek-ai/dsh-tool-bash'\n"),
        ("a second disabled", "  disabled: true\n"),
        ("a second config", "  config:\n    x: 1\n  config:\n"),
        ("a second inject", "  inject:\n  inject:\n"),
    ] {
        assert_eq!(
            gate(
                on_tool_bash(&dump, planted),
                Release::V0_2_1,
                Mode::ReadOnly
            ),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
}

/// Rule 5: `config:` and `inject:` open the row's block, deeper lines are
/// unread and admitted only while one is open, and `name:`, `disabled:`
/// and the next row close it. A deeper line with no open block refuses.
#[test]
fn a_deeper_line_is_admitted_only_while_a_block_is_open() {
    let dump = policy_dump();
    for (what, planted) in [
        (
            "a config block",
            "  config:\n    any: thing\n      deeper: 1\n",
        ),
        (
            "an inject then a config block",
            "  inject:\n    - x\n  config:\n    y: z\n",
        ),
    ] {
        assert_eq!(
            gate(
                on_tool_bash(&dump, planted),
                Release::V0_2_1,
                Mode::ReadOnly
            ),
            Ok(()),
            "{what}"
        );
    }
    let after_id = dump.replacen("- id: tool-bash\n", "- id: tool-bash\n    x\n", 1);
    assert_ne!(after_id, dump);
    assert_eq!(
        gate(after_id, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable),
        "a continuation directly after the id"
    );
    let after_name = dump.replacen(
        "  name: '@deepseek-ai/dsh-tool-bash'\n",
        "  name: '@deepseek-ai/dsh-tool-bash'\n    x\n",
        1,
    );
    assert_ne!(after_name, dump);
    assert_eq!(
        gate(after_name, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable),
        "a continuation directly after the name"
    );
}

/// Rules 3 and 6: a one-space line, a line led by a tab, and a column-zero
/// line that is not `- id: ` all refuse `Unreadable`.
#[test]
fn a_one_space_tab_or_column_zero_line_is_refused() {
    let dump = policy_dump();
    for (what, planted) in [
        ("a one-space line", " x: y\n"),
        ("a tab-led line", "\tx: y\n"),
        ("a document marker", "---\n"),
        ("a bare key", "stray: line\n"),
        ("a spaced row marker", "-   id: tool-bash\n"),
        ("a row marker with no space", "- id:tool-bash\n"),
    ] {
        assert_eq!(
            gate(
                on_tool_bash(&dump, planted),
                Release::V0_2_1,
                Mode::ReadOnly
            ),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
}

/// Rule 7: the other rows keep their reading. Exactly `true` disables a
/// row; `false`, a `!!js` expression and a word jammed against the colon
/// do not, so a row that must be disabled still refuses `Enabled`.
#[test]
fn another_rows_disabled_keeps_its_reading() {
    let dump = policy_dump();
    for (what, word) in [
        ("the word false", "  disabled: false\n"),
        ("a js expression", "  disabled: !!js true\n"),
        ("a word jammed against the colon", "  disabled:true\n"),
    ] {
        let row = format!("- id: tool-bash\n  name: '@deepseek-ai/dsh-tool-bash'\n{word}");
        let mutated = dump.replacen(TOOL_BASH, &row, 1);
        assert_ne!(mutated, dump, "{what}");
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Enabled {
                id: "tool-bash".into(),
            }),
            "{what}"
        );
    }
}
