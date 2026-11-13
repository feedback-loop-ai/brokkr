//! Fix round 1052g7 (#1052): the dump's line-property closure, independent
//! of the row grammar and of its block bookkeeping. No line may leave a
//! quote or a flow collection open, whether a YAML reader reads it as
//! structure or as block content, so no line can carry a following row's
//! keys away. The closure admits nothing the row grammar refuses; these
//! tests read it alone and through `gate`.

#![cfg(unix)]

use super::super::composition::{closure, Composition};
use super::super::{Mode, Release};
use super::composed::{gate, with_sandbox_mode};
use super::composed_rows::{policy_dump, tool_bash_row};
use super::{measured_dump, measured_dump_0_2_2};

/// The measured sandbox row's own id and name, the row C1e and C2 mutate.
const SANDBOX_ROW: &str = "- id: sandbox\n  name: '@deepseek-ai/dsh-sandbox-local'\n";

/// The measured timer row's name line, the adversarial HIGH mutates.
const TIMER_NAME: &str = "  name: '@deepseek-ai/cordis-plugin-timer'\n";

/// Refuse `what` through the closure alone and through the whole gate. The
/// gate case is the composed tree the probe setup names: release 0.2.1 and
/// the seat's read-only mode.
fn refuses(what: &str, dump: &str) {
    assert_eq!(
        closure(dump),
        Err(Composition::Unreadable),
        "{what}: closure"
    );
    assert_eq!(
        gate(dump.to_string(), Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unreadable),
        "{what}: gate"
    );
}

/// C1a to C1f, C2 and the adversarial HIGH: the carrier class the hold
/// found. Each is a block-scalar header behind a marker whose content
/// reaches the node's column, a two-space `disabled:` value, or a quoted
/// scalar in a block; each leaves a quote or a flow collection open on a
/// line the gate used to skip, and each refuses `Unreadable` both by the
/// closure and by the whole gate.
#[test]
fn the_closure_refuses_every_carrier_case() {
    let dump = policy_dump();
    let cases = [
        (
            "C1a: a marker, a key header, and a quote at the key's column",
            "  config:\n    - k: >-\n      j: \"x\n  disabled: true\n  # \"\n",
        ),
        (
            "C1b: an inject marker, a literal header, and a quote at the key's column",
            "  inject:\n    - k: |-\n      j: 'x\n  disabled: true\n  # '\n",
        ),
        (
            "C1c: two markers, a bare header, and a quote at the inner marker's column",
            "  config:\n    - - >-\n      - \"x\n  disabled: true\n  # \"\n",
        ),
        (
            "C1d: a quoted key header and a quote at the key's column",
            "  config:\n    - \"k\": >\n      j: \"x\n  disabled: true\n  # \"\n",
        ),
        (
            "C1f: a marker header and an open flow at the key's column",
            "  config:\n    - k: >-\n      j: [a,\n  disabled: true\n  inject:\n    ]\n",
        ),
    ];
    for (what, body) in cases {
        refuses(what, &tool_bash_row(&dump, body));
    }

    // C1e: the B2 form, on the sandbox row before the whole real policy row.
    let c1e = dump.replacen(
        SANDBOX_ROW,
        &format!("{SANDBOX_ROW}  config:\n    - k: >-\n      j: \"x\n"),
        1,
    );
    assert_ne!(c1e, dump, "the fixture names the sandbox row");
    refuses("C1e: a quote opened on the preceding sandbox row", &c1e);

    // C2: a two-space `disabled:` value on the sandbox row, the real policy
    // row kept, with the closing comment after its mode.
    let c2 = dump.replacen(SANDBOX_ROW, &format!("{SANDBOX_ROW}  disabled: \"x\n"), 1);
    let c2 = c2.replacen("    mode: read-only\n", "    mode: read-only\n# \"\n", 1);
    assert_ne!(c2, dump, "the fixture names the sandbox row");
    refuses("C2: a quoted two-space disabled value", &c2);

    // The adversarial HIGH: the timer row's `disabled:` carries an open
    // quote, with the closing comment after the last row.
    let adv = dump.replacen(TIMER_NAME, &format!("{TIMER_NAME}  disabled: \"x\n"), 1);
    let adv = format!("{adv}# \"\n");
    assert_ne!(adv, dump, "the fixture names the timer row");
    refuses("ADV: a quoted disabled value on the timer row", &adv);
}

/// The closure's own line shapes: a quoted key with spaces before its
/// colon, an open flow after a quoted key, a double-quoted scalar whose
/// last quote is escaped, and a backslash continuation all leave something
/// open on the line alone.
#[test]
fn the_closure_refuses_a_quoted_key_and_the_quote_escapes() {
    let dump = policy_dump();
    for (what, body) in [
        (
            "a quoted key with a space before its colon",
            "  config:\n    \"j: \" : \"x\n  disabled: true\n",
        ),
        (
            "an open flow after a quoted key",
            "  config:\n    \"k\": [a,\n  disabled: true\n",
        ),
        (
            "a double-quoted scalar ending in an escaped quote",
            "  config:\n    k: \"x\\\"\n  disabled: true\n",
        ),
        (
            "a backslash-continued double quote",
            "  config:\n    k: \"x\\\n  disabled: true\n",
        ),
    ] {
        refuses(what, &tool_bash_row(&dump, body));
    }
}

/// The admitted shapes, each through the closure alone and through the
/// whole gate. The line-77 shape is the measured 0.2.1 dump's own block
/// content: a closed quote followed by junk is no carrier under the
/// structure reading. The closed flow is block-scalar content, so the row
/// grammar still decides admission and skips it unread.
#[test]
fn the_closure_admits_every_closed_shape() {
    let dump = policy_dump();
    let bodies = [
        (
            "the measured dump's line-77 shape in a tagged block",
            concat!(
                "  config:\n",
                "    desktopPlatform: !!js >-\n",
                "      ctx.get('profileContext')?.name === 'desktop' && ['darwin',\n",
                "      'win32'].includes(process.platform) ? process.platform : null\n",
                "  disabled: true\n",
            ),
        ),
        (
            "a closed quoted key with a space before its colon",
            "  config:\n    \"k\" : v\n  disabled: true\n",
        ),
        (
            "a tagged single-quoted value",
            "  config:\n    c: !!js '!ctx.get(''p'')'\n  disabled: true\n",
        ),
        (
            "a closed flow holding a closed quote as block content",
            "  config:\n    e: >-\n      [a, \"b\"]\n  disabled: true\n",
        ),
    ];
    for (what, body) in bodies {
        let mutated = tool_bash_row(&dump, body);
        assert_eq!(closure(&mutated), Ok(()), "{what}: closure");
        assert_eq!(
            gate(mutated, Release::V0_2_1, Mode::ReadOnly),
            Ok(()),
            "{what}: gate"
        );
    }
    assert_eq!(
        closure("    k: [a, \"b\"]\n"),
        Ok(()),
        "the closure admits a closed flow at a value too"
    );
}

/// The two committed real trees still pass. The raw 0.2.1 dump has no
/// composed hands row, so the closure reads it alone; the composed 0.2.1
/// and 0.2.2 fixtures pass the whole gate.
#[test]
fn the_real_tree_fixtures_pass_the_closure_and_the_gate() {
    let raw = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/evidence/adapters/dsh-hands-round2/dump-stdio.txt"),
    )
    .expect("the controller's measured dump is in the workspace");
    assert_eq!(closure(&raw), Ok(()), "the committed 0.2.1 dump");
    assert_eq!(
        gate(
            with_sandbox_mode(&measured_dump(), "read-only"),
            Release::V0_2_1,
            Mode::ReadOnly
        ),
        Ok(()),
        "the composed 0.2.1 fixture"
    );
    assert_eq!(
        gate(
            with_sandbox_mode(&measured_dump_0_2_2(), "read-only"),
            Release::V0_2_2,
            Mode::ReadOnly
        ),
        Ok(()),
        "the composed 0.2.2 fixture"
    );
}

/// C1's fix: judge block content from the header's node column. A header
/// behind a marker has its key at column 6, so a line at 6 is not content:
/// the grammar reads it and refuses its non-dump flow value. Content two
/// columns past the node's column is the real js-yaml form and is skipped.
/// Recording the line's spaces again makes the first case pass, which is
/// the removal control.
#[test]
fn the_block_content_judgement_reads_a_line_at_the_nodes_column() {
    let dump = policy_dump();
    for (what, body) in [
        (
            "a key at the node's column is read",
            "  config:\n    - k: >-\n      j: [a]\n  disabled: true\n",
        ),
        (
            "a bare item at the last marker's column is read",
            "  config:\n    - - >-\n      - [a]\n  disabled: true\n",
        ),
    ] {
        assert_eq!(
            gate(tool_bash_row(&dump, body), Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
    let admitted = "  config:\n    - k: |-\n        j: [a]\n  disabled: true\n";
    assert_eq!(
        gate(
            tool_bash_row(&dump, admitted),
            Release::V0_2_1,
            Mode::ReadOnly
        ),
        Ok(()),
        "js-yaml's real form has its content at the node column plus two"
    );
}

/// C3: a tab anywhere in the dump refuses `Unreadable`, as a member of the
/// dump-wide break-and-control set. Each case is a shape a lenient reader
/// takes for structure: a tag ended by a tab, a tab before a quoted key,
/// and a tab inside block-scalar content.
#[test]
fn a_tab_anywhere_in_the_dump_is_refused() {
    let dump = policy_dump();
    for (what, body) in [
        (
            "C3a: a tab ends a tag and opens a quote",
            "  config:\n    k: !!str\t\"x y\n  disabled: true\n  # \"\n",
        ),
        (
            "C3b: a tab before a quoted key",
            "  config:\n    \t\"x: y\n  disabled: true\n  # \"\n",
        ),
        (
            "a tab inside block-scalar content",
            "  config:\n    k: >-\n      a\tb\n  disabled: true\n",
        ),
    ] {
        assert_eq!(
            gate(tool_bash_row(&dump, body), Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
}

/// C3c, C3d and the adversarial LOW: plain values the dump never writes
/// refuse, while `-1` and `a-b` stay plain values. The old plain-start set
/// admits `k: - a`, which is the removal control.
#[test]
fn plain_values_the_dump_never_writes_refuse() {
    let dump = policy_dump();
    for (what, value) in [
        ("a dash item", "- a"),
        ("a dash marker", "- \"x"),
        ("a key inside a value", "j: \"x"),
        ("a flow closer", "]"),
        ("a comma", ","),
        ("a bare colon", ":"),
    ] {
        let body = format!("  config:\n    k: {value}\n  disabled: true\n");
        assert_eq!(
            gate(tool_bash_row(&dump, &body), Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
    for (what, value) in [("a negative number", "-1"), ("a hyphenated word", "a-b")] {
        let body = format!("  config:\n    k: {value}\n  disabled: true\n");
        assert_eq!(
            gate(tool_bash_row(&dump, &body), Release::V0_2_1, Mode::ReadOnly),
            Ok(()),
            "{what}"
        );
    }
}
