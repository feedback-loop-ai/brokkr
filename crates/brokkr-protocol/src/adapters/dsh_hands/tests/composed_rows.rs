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

/// Replace the measured `tool-bash` row with its id and name followed by
/// `body`, so a test plants a block exactly as a composed tree carries it.
fn tool_bash_row(dump: &str, body: &str) -> String {
    let row = format!("- id: tool-bash\n  name: '@deepseek-ai/dsh-tool-bash'\n{body}");
    let mutated = dump.replacen(TOOL_BASH, &row, 1);
    assert_ne!(mutated, dump, "the fixture names tool-bash");
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

/// F1 (HIGH, security): a quoted scalar in an unread block must close on
/// its own line, because a YAML reader lets it carry the row's following
/// keys away as its content. In each form the block's next `disabled:` is
/// swallowed by the quote PyYAML and js-yaml read, so the tree must refuse
/// `Unreadable` rather than admit a row the reader then enables.
#[test]
fn an_unread_block_refuses_a_quoted_scalar_that_does_not_close() {
    let dump = policy_dump();
    for (what, body) in [
        (
            "a double quote in config",
            "  config:\n    k: \"x\n  disabled: true\n  # \"\n",
        ),
        (
            "a single quote in config",
            "  config:\n    x: 'open\n  disabled: true\n  # '\n",
        ),
        (
            "a backslash-continued double quote",
            "  config:\n    k: \"x\\\n  disabled: true\n  # \"\n",
        ),
        (
            "a quote in an inject list",
            "  inject:\n    - 'x\n  disabled: true\n  # '\n",
        ),
        (
            "a quote across a later key",
            "  config:\n    k: \"x\n  disabled: true\n  inject:\n    \"\n",
        ),
        (
            "a quote closed on a column-zero comment",
            "  config:\n    k: \"x\n  disabled: true\n# \"\n",
        ),
    ] {
        let mutated = tool_bash_row(&dump, body);
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
}

/// F1: the policy row's `inject:` and the preceding `sandbox` row's
/// `config:` are unread blocks too. A quote there swallows the real policy
/// row's keys in a YAML reader, so both refuse; the mode the gate would
/// read is never the mode the seat runs under.
#[test]
fn a_quote_in_the_policy_or_preceding_sandbox_block_refuses() {
    let dump = policy_dump();
    let configured = "  config:\n    mode: read-only\n";
    let injected = "  inject:\n    - \"x\n  config:\n    mode: read-only\n  # \"\n";
    let policy = dump.replacen(configured, injected, 1);
    assert_ne!(policy, dump, "the fixture names the policy row");
    assert_eq!(
        gate(policy, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable),
        "a quote in the policy inject block"
    );

    let sandbox = "- id: sandbox\n  name: '@deepseek-ai/dsh-sandbox-local'\n";
    let opened = format!("{sandbox}  config:\n    k: \"x\n");
    let preceding = dump.replacen(sandbox, &opened, 1);
    assert_ne!(preceding, dump, "the fixture names the sandbox row");
    let preceding = preceding.replacen(configured, &format!("{configured}# \"\n"), 1);
    assert_eq!(
        gate(preceding, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable),
        "a quote opened in the preceding sandbox row"
    );
}

/// F1: a flow collection in an unread block must be empty. A collection
/// left open is read by js-yaml with lenient indentation, so a sequence or
/// mapping that continues over the row's keys refuses `Unreadable`, at a
/// value and at a sequence item.
#[test]
fn an_unread_block_refuses_an_open_flow_collection() {
    let dump = policy_dump();
    for (what, body) in [
        (
            "an open flow sequence",
            "  config:\n    k: [a,\n  disabled: true\n  inject:\n    ]\n",
        ),
        (
            "an open flow mapping",
            "  config:\n    k: {a: b,\n  disabled: true\n  inject:\n    }\n",
        ),
        ("a flow sequence item", "  config:\n    - [a]\n"),
        ("a flow mapping item", "  config:\n    - {a: b}\n"),
    ] {
        let mutated = tool_bash_row(&dump, body);
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
}

/// F1: an anchor, an alias, an explicit key, a directive, a reserved
/// indicator and a block-scalar indentation indicator are not lines
/// js-yaml's dump writes, so each refuses `Unreadable`. So do the tab and
/// double-space separators a lenient reader takes for structure: js-yaml
/// reads `k:\t"x` and `k:  "x` as an open quoted value, so the gate does
/// not carry either as a plain scalar.
#[test]
fn an_unread_block_refuses_every_other_non_dump_line() {
    let dump = policy_dump();
    for (what, body) in [
        ("an anchor value", "  config:\n    k: &a value\n"),
        ("an alias value", "  config:\n    k: *a\n"),
        ("an anchor item", "  config:\n    - &a value\n"),
        ("an alias item", "  config:\n    - *a\n"),
        ("a literal indicator", "  config:\n    k: |2\n      text\n"),
        ("a folded indicator", "  config:\n    k: >1-\n      text\n"),
        ("an explicit key", "  config:\n    ? disabled\n"),
        ("a directive", "  config:\n    %YAML 1.2\n"),
        ("a reserved indicator", "  config:\n    @reserved\n"),
        ("a backquote", "  config:\n    `reserved\n"),
        ("a tag with no space", "  config:\n    k: !!js'x'\n"),
        ("a tab after a key's colon", "  config:\n    k:\t\"x\n"),
        ("a tab after a sequence marker", "  config:\n    -\t'x\n"),
        ("a bare sequence marker", "  config:\n    -\n"),
        ("a quoted scalar followed by junk", "  config:\n    'x' y\n"),
        ("an anchored key", "  config:\n    &a key: value\n"),
        ("an empty key", "  config:\n    : value\n"),
        ("two spaces after a key's colon", "  config:\n    k:  \"x\n"),
    ] {
        let mutated = tool_bash_row(&dump, body);
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
}

/// F1: every dump line an unread block may carry is admitted. A balanced
/// quoted value and key, a tagged single-quoted value, a plain scalar with
/// an apostrophe and a double quote, empty flow collections, and `>-` block
/// scalars at a value and a sequence item are admitted. The scalar content
/// holds an open quote and an open bracket and a blank line, and the
/// two-space `inject:` after it is read again: content ends where the dump
/// returns to the row's level.
#[test]
fn an_unread_block_admits_every_dump_line_it_reads() {
    let dump = policy_dump();
    let body = concat!(
        "  config:\n",
        "    a: \"v\"\n",
        "    \"b\": v\n",
        "    c: !!js '!ctx.get(''p'')'\n",
        "    d: it's a \"quote\"\n",
        "    h: \"a\\\"b\"\n",
        "    e: >-\n",
        "      \"open [\n",
        "\n",
        "      ] close\n",
        "    f: []\n",
        "    g: {}\n",
        "    z:\n",
        "  inject:\n",
        "    - '-I'\n",
        "    - >-\n",
        "      'open [\n",
        "  disabled: true\n",
    );
    assert_eq!(
        gate(tool_bash_row(&dump, body), Release::V0_2_1, Mode::ReadOnly),
        Ok(())
    );
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
    // A dump of only skip lines has no row at all, so it is not a
    // composition either.
    assert_eq!(
        gate("# only a comment\n".into(), Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable)
    );
}

/// Rule 4: a row's two-space level is closed. An unknown key, an explicit
/// `?` key, and a flow-style `config:` or `inject:` all refuse
/// `Unreadable`. The policy row's bare `inject:` is admitted only as its
/// one measured item, which the next test binds.
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
}

/// Rule 5, with the measured 0.2.2-rc.1 shape: the `sandbox-policy` row
/// carries an `inject:` block, and it is admitted only as the one measured
/// `    - headlessStartup` line, before or after its `config:` block. Any
/// other line in it refuses `Unreadable`: a mode-looking line, a second
/// item, a quoted item, a wrongly indented item and an empty block all
/// refuse, so no policy `inject:` content is admitted unread.
#[test]
fn the_policy_inject_block_is_only_the_measured_one() {
    let dump = policy_dump();
    let configured = "  config:\n    mode: read-only\n";
    let measured = "  inject:\n    - headlessStartup\n";
    for (what, planted) in [
        (
            "the measured inject before config",
            format!("{measured}{configured}"),
        ),
        (
            "the measured inject after config",
            format!("{configured}{measured}"),
        ),
    ] {
        let mutated = dump.replacen(configured, &planted, 1);
        assert_ne!(mutated, dump, "{what}");
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Ok(()),
            "{what}"
        );
    }
    for (what, planted) in [
        (
            "a mode-looking line under inject",
            format!("  inject:\n    mode: danger-full-access\n{configured}"),
        ),
        (
            "a second item under inject",
            format!("  inject:\n    - headlessStartup\n    - extra\n{configured}"),
        ),
        (
            "the measured item twice",
            format!("  inject:\n    - headlessStartup\n    - headlessStartup\n{configured}"),
        ),
        (
            "a quoted item under inject",
            format!("  inject:\n    - \"x\n{configured}"),
        ),
        (
            "a wrongly indented measured item",
            format!("  inject:\n      - headlessStartup\n{configured}"),
        ),
        (
            "an empty inject before config",
            format!("  inject:\n{configured}"),
        ),
        (
            "an empty inject at the row's end",
            format!("{configured}  inject:\n"),
        ),
        (
            "a repeated inject",
            format!("{measured}  inject:\n    - headlessStartup\n{configured}"),
        ),
    ] {
        let mutated = dump.replacen(configured, &planted, 1);
        assert_ne!(mutated, dump, "{what}");
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
    // The row's end is the last chance to read the item: a dump whose policy
    // row closes the dump with an empty inject refuses there too.
    let minimal = "- id: sandbox-policy\n  name: '@deepseek-ai/dsh-sandbox-policy'\n  \
                   config:\n    mode: read-only\n  inject:\n";
    assert_eq!(
        gate(minimal.into(), Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable),
        "an empty inject at the end of the dump"
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
