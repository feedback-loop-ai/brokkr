//! Fix round 1052g8 (#1052): the dump's line-property closure, independent
//! of the row grammar and of its block bookkeeping. No line may leave a
//! quote, a non-empty flow collection or an empty sequence item open,
//! whether a YAML reader reads it as structure or as block content, so no
//! line can carry a following row's keys away. The empty item is js-yaml
//! 4.3.2's own third carrier: it folds the next less-indented `- ` line into
//! it. The closure admits nothing the row grammar refuses; these tests read
//! it alone and through `gate`.

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

/// Admit `what` through the closure alone and through the whole gate.
fn admits(what: &str, dump: String) {
    assert_eq!(closure(&dump), Ok(()), "{what}: closure");
    assert_eq!(
        gate(dump, Release::V0_2_1, Mode::ReadOnly),
        Ok(()),
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

/// The third carrier, the value read after every node kind, and flows
/// admitted only empty. Each case refuses `Unreadable` through the closure
/// alone and through the whole gate. The empty item is js-yaml 4.3.2's own
/// carrier: it folds a less-indented `- ` line into the empty item, so the
/// policy row disappears; the adversarial HIGH combines it with a wider
/// marker and an open quote after a flow key.
#[test]
fn the_closure_refuses_the_empty_item_and_every_open_node() {
    let dump = policy_dump();

    let high = tool_bash_row(
        &dump,
        concat!(
            "  config:\n",
            "    -   k: >-\n",
            "        deep\n",
            "      []: \"x\n",
            "  disabled: true\n",
            "# \"\n",
        ),
    );
    refuses("the adversarial HIGH", &high);

    for (what, body) in [
        (
            "a flow key with an open value",
            "  config:\n    []: \"x\n  disabled: true\n",
        ),
        (
            "a mapping key with an open value",
            "  config:\n    {}: \"x\n  disabled: true\n",
        ),
        (
            "an alias key with an open value",
            "  config:\n    *a : \"x\n  disabled: true\n",
        ),
        (
            "an alias key with no space before its colon",
            "  config:\n    *a: \"x\n  disabled: true\n",
        ),
        (
            "an empty flow followed by junk",
            "  config:\n    []x: \"y\n  disabled: true\n",
        ),
        (
            "an anchor ended by a flow indicator",
            "  config:\n    k: &a[b,\n  disabled: true\n",
        ),
        (
            "an anchor then an open quote",
            "  config:\n    k: &a \"x\n  disabled: true\n",
        ),
        (
            "an explicit key with an open quote",
            "  config:\n    ? \"x\n  disabled: true\n",
        ),
        (
            "a colon indicator with an open quote",
            "  config:\n    : \"x\n  disabled: true\n",
        ),
        (
            "a non-empty flow sequence",
            "  config:\n    k: [a]\n  disabled: true\n",
        ),
        (
            "a non-empty flow mapping",
            "  config:\n    k: {a: b}\n  disabled: true\n",
        ),
        (
            "a flow with a comment inside",
            "  config:\n    k: [a, # ]\n  disabled: true\n",
        ),
        (
            "a flow after a quoted key",
            "  config:\n    \"k\": [a, # ]\n  disabled: true\n",
        ),
        (
            "a flow with a quote-shaped key",
            "  config:\n    k: [?'a]', b\n  disabled: true\n",
        ),
        (
            "a closed flow holding a quote as content",
            "  config:\n    e: >-\n      [a, \"b\"]\n  disabled: true\n",
        ),
    ] {
        refuses(what, &tool_bash_row(&dump, body));
    }

    // js-yaml folds the next less-indented row into each empty item, so the
    // policy row is gone; each tree refuses at the empty item.
    for (what, item) in [
        ("a bare item", "    -\n"),
        ("an item of spaces", "    -   \n"),
        ("an item then a comment", "    -   # c\n"),
        ("a nested item", "    - - a\n      -\n"),
    ] {
        let opened = format!("{SANDBOX_ROW}  inject:\n{item}");
        let mutated = dump.replacen(SANDBOX_ROW, &opened, 1);
        assert_ne!(mutated, dump, "the fixture names the sandbox row");
        refuses(what, &mutated);
    }

    // The empty-item rule's comment marker takes any first character after
    // `#`, not only `# `: the row grammar reads `#b` as a key with a block
    // header, while both readers read an empty item and a comment. The
    // block is unread, so only the closure refuses this tree.
    let comment_key = tool_bash_row(&dump, "  disabled: true\n  config:\n    - #b: >-\n");
    refuses("an empty item whose comment names a key", &comment_key);
}

/// The closed shapes the rebuild admits, each through the closure alone and
/// through the gate: a quoted value holding ` #` behind a plain key, the
/// dump's empty items and empty flows, a quoted key with spaces before its
/// colon, the measured lines 76 and 77 in a `!!js >-` block, and ` #` in
/// folded content.
#[test]
fn the_closure_admits_the_closed_dump_lines() {
    let dump = policy_dump();
    for (what, body) in [
        (
            "a single-quoted value holding a comment marker",
            "  config:\n    k: 'a #b'\n  disabled: true\n",
        ),
        (
            "a double-quoted value holding a comment marker",
            "  config:\n    k: \"a #b\"\n  disabled: true\n",
        ),
        ("a null item", "  config:\n    - null\n  disabled: true\n"),
        (
            "an empty flow item",
            "  config:\n    - []\n  disabled: true\n",
        ),
        (
            "an empty flow sequence value",
            "  config:\n    k: []\n  disabled: true\n",
        ),
        (
            "an empty flow mapping value",
            "  config:\n    k: {}\n  disabled: true\n",
        ),
        (
            "a quoted key with spaces before its colon",
            "  config:\n    \"k\" : v\n  disabled: true\n",
        ),
        (
            "a comment marker in folded content",
            "  config:\n    k: >-\n      with a # title. Make\n  disabled: true\n",
        ),
    ] {
        let mutated = tool_bash_row(&dump, body);
        admits(what, mutated);
    }

    // The measured 0.2.1 dump's lines 76 and 77, in a tagged block.
    let lines = tool_bash_row(
        &dump,
        concat!(
            "  config:\n",
            "    desktopPlatform: !!js >-\n",
            "      ctx.get('profileContext')?.name === 'desktop' && ['darwin',\n",
            "      'win32'].includes(process.platform) ? process.platform : null\n",
            "  disabled: true\n",
        ),
    );
    admits("lines 76 and 77", lines);

    // A closed row id holding ` #` is read by both passes; the judge then
    // refuses the id it names, so the line is not unreadable.
    assert_eq!(closure("- id: 'a #b'\n"), Ok(()), "a closed row id");
    let id = dump.replacen("- id: tool-bash\n", "- id: 'a #b'\n", 1);
    assert_ne!(id, dump, "the fixture names tool-bash");
    assert_eq!(
        gate(id, Release::V0_2_1, Mode::ReadOnly),
        Err(Composition::Unmeasured {
            id: "a #b".into(),
            name: "@deepseek-ai/dsh-tool-bash".into(),
            release: Release::V0_2_1.label(),
        }),
        "the passes read the closed id"
    );
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
/// structure reading. The closure-only lines read the `? ` marker, bare
/// `?`, a tag with no space, a comment after a plain scalar, and a quoted
/// key with an empty or a jammed value.
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
    ];
    for (what, body) in bodies {
        let mutated = tool_bash_row(&dump, body);
        admits(what, mutated);
    }
    assert_eq!(closure(""), Ok(()), "the empty dump has no open line");
    assert_eq!(closure("\n\n"), Ok(()), "empty lines leave nothing open");
    for (what, text) in [
        ("a `? ` marker and a bare `?`", "? x\n?\n"),
        ("a bare colon and a colon then a comment", ":\n: # c\n"),
        (
            "a tag with no space after it, and a tag alone",
            "k: !tag\nk: !!js\nk: !\n",
        ),
        ("a plain scalar cut at a comment", "k: a # c\n"),
        (
            "a quoted key with an empty and a jammed value",
            "\"k\":\n\"k\":x\n",
        ),
    ] {
        assert_eq!(closure(text), Ok(()), "{what}");
    }
}

/// A plain scalar that starts with a dash at the start of a line is no
/// empty item: `-1` and `--flag x` leave no quote, flow or item open, so the
/// closure admits them, and so does the gate in a block scalar's content.
/// Only a bare `-` (or `- ` with nothing after it) is the empty item the
/// rule refuses. Removal control: `body == "-"` widened to
/// `body.starts_with('-')` refuses all three.
#[test]
fn the_closure_admits_a_dash_led_plain_scalar() {
    assert_eq!(closure("-1\n"), Ok(()), "a negative number at a line start");
    assert_eq!(
        closure("      --flag x\n"),
        Ok(()),
        "a dash-led flag in block content"
    );
    let dump = policy_dump();
    admits(
        "a dash-led line in a literal block",
        tool_bash_row(&dump, "  config:\n    k: |-\n      -1\n  disabled: true\n"),
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
/// the grammar reads it and refuses its non-dump value. A bare item's
/// header records the last marker's column, not the line's spaces plus
/// every marker width: a line at column 7 is content, and recording 8 would
/// make the grammar read it. Content two columns past the node's column is
/// the real js-yaml form and is skipped. Recording the line's spaces again
/// makes the first case pass, which is the removal control; recording
/// `spaces + width` for a bare item makes the last case fail.
#[test]
fn the_block_content_judgement_reads_a_line_at_the_nodes_column() {
    let dump = policy_dump();
    for (what, body) in [
        (
            "a key at the node's column is read",
            "  config:\n    - k: >-\n      j: a: b\n  disabled: true\n",
        ),
        (
            "a bare item at the last marker's column is read",
            "  config:\n    - - >-\n      a: b: c\n  disabled: true\n",
        ),
    ] {
        assert_eq!(
            gate(tool_bash_row(&dump, body), Release::V0_2_1, Mode::ReadOnly),
            Err(Composition::Unreadable),
            "{what}"
        );
    }
    let admitted = "  config:\n    - k: |-\n        j: []\n  disabled: true\n";
    assert_eq!(
        gate(
            tool_bash_row(&dump, admitted),
            Release::V0_2_1,
            Mode::ReadOnly
        ),
        Ok(()),
        "js-yaml's real form has its content at the node column plus two"
    );
    let item_content = "  config:\n    - - >-\n       j: a: b\n  disabled: true\n";
    assert_eq!(
        gate(
            tool_bash_row(&dump, item_content),
            Release::V0_2_1,
            Mode::ReadOnly
        ),
        Ok(()),
        "a bare item's content at column 7 stays content"
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
        ("a bare dash", "-"),
        ("a dash marker", "- \"x"),
        ("a colon item", ": a"),
        ("a key inside a value", "j: \"x"),
        ("a flow closer", "]"),
        ("a flow mapping closer", "}"),
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

/// The closure's markers, properties and property ends, each bound by the
/// line it changes. A `: ` indicator whose body is a bare item, a `!`
/// property that leaves an open quote, and a `{` that ends an anchor before
/// an open flow all refuse; a property ended by `,`, `]` or `}` leaves a
/// node the pass reads as closed, as the rule says. Dropping the `: ` arm,
/// the `!` member, or any one of the flow indicators flips its case.
/// Every marker is stripped, not only the first, and so is every property;
/// the spaces after a quoted node and around a property are a run; and a
/// property token ends at its first space. A `#` inside a plain scalar cuts
/// a comment only after a space. Dropping any one of those flips its case.
#[test]
fn the_closure_reads_a_colon_marker_a_tag_and_every_property_end() {
    for (what, text) in [
        ("a colon marker holding a bare item", ": -\n"),
        (
            "a colon marker with an extra space before a bare item",
            ":  -\n",
        ),
        (
            "a question marker with an extra space before a bare item",
            "?  -\n",
        ),
        ("a tag then an open quote", "    k: !!str \"x\n"),
        ("a brace ends an anchor and opens a flow", "    k: &a{b,\n"),
        ("a second item marker kept as plain text", "- - \"x\n"),
        (
            "a comment marker jammed against a plain scalar",
            "    k: a#b: \"x\n",
        ),
        (
            "two properties before an open quote",
            "    k: !!js &a \"x\n",
        ),
        (
            "a quoted key with two spaces before its colon",
            "    \"k\"  : \"x\n",
        ),
        ("a one-character property before a quote", "    k: ! \"x\n"),
        ("two spaces before an open quoted value", "    k:  \"x\n"),
        (
            "a tagged value with two spaces before an open quote",
            "    k: !!str  \"x\n",
        ),
    ] {
        assert_eq!(closure(text), Err(Composition::Unreadable), "{what}");
    }
    for (what, text) in [
        (
            "a comma ends an anchor before text a reader refuses",
            "    k: &a,[b\n",
        ),
        (
            "a flow closer ends an anchor before a quote",
            "    k: &a] \"x\n",
        ),
        (
            "a mapping closer ends an anchor before a quote",
            "    k: &a} \"x\n",
        ),
    ] {
        assert_eq!(closure(text), Ok(()), "{what}");
    }

    // The repeated-marker rule's carrier: without it the second marker stays
    // plain text, so the tree's empty item is admitted both by the pass and
    // by the whole gate, which reads the block as content and never sees it.
    let dump = policy_dump();
    let repeated = tool_bash_row(&dump, "  disabled: true\n  config:\n    - - # c\n");
    refuses("a repeated item marker before a comment", &repeated);
}

/// The closure's admitted line starts: a comment, a block-scalar header and
/// a plain scalar cut at ` #` all leave nothing open even when a key and a
/// quote follow, and a `? ` or a `: ` after a `- ` leaves the dash marker
/// closed. A flow node's space or colon tail, a colon with no space after it
/// (plain content, not a key separator), and a closed quote with no space
/// before its colon leave nothing open too. Dropping the `#` arm, either
/// half of the `|`/`>` arm, the ` #` cut, the `dash = false` reset after
/// `? ` or `: `, either flow tail, or `after_node`'s space guard refuses
/// one of them, a false refusal. The `:` arm's byte check is what keeps
/// `k:"x` from splitting a key, and `|` is a header exactly as `>` is.
#[test]
fn the_closure_admits_a_comment_a_header_and_a_cut_comment() {
    for (what, text) in [
        ("a comment at a node's start", "    # t: \"x\n"),
        ("a block header with a key-shaped tail", "    > k: \"x\n"),
        ("a literal header with a key-shaped tail", "    | k: \"x\n"),
        ("a plain scalar cut before its key", "    a # b: \"x\n"),
        (
            "a question after an item marker and a comment",
            "    - ? # c\n",
        ),
        ("a colon after an item marker and a comment", "- : # c\n"),
        ("an empty flow cut at a comment", "    k: [] # c\n"),
        ("an empty flow sequence used as a key", "    []: x\n"),
        (
            "a closed quoted key jammed against an open quote",
            "    \"k\":\"x\n",
        ),
        ("a colon jammed against an open quote", "    k:\"x\n"),
    ] {
        assert_eq!(closure(text), Ok(()), "{what}");
    }
}
