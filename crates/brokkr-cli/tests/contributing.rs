//! Decision 0033's executable documentation: the sixty-second guide is
//! the recipe library rendered as one table, and the contribution gate
//! keeps its declaration, evidence, head-binding, and visible escape
//! hatch in repository-owned platform data.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn guide_rows(guide: &str) -> Vec<(String, String, String, String)> {
    let table = guide
        .split_once("<!-- recipe-table:start -->")
        .expect("guide starts its recipe table")
        .1
        .split_once("<!-- recipe-table:end -->")
        .expect("guide ends its recipe table")
        .0;
    table
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells.len(), 4, "one four-column recipe row: {line}");
            (
                cells[0].trim_matches('`').to_string(),
                cells[1].to_string(),
                cells[2].to_string(),
                cells[3].to_string(),
            )
        })
        .collect()
}

#[test]
fn the_sixty_second_table_is_the_recipe_library() {
    let root = workspace();
    let guide = std::fs::read_to_string(root.join("CONTRIBUTING.md")).unwrap();
    assert!(
        guide.lines().count() < 120,
        "the guide stopped being one screen"
    );
    assert_eq!(
        guide
            .matches("| Recipe | When to use it | What it seats | Rough cost |")
            .count(),
        1,
        "the short guide has one recipe table"
    );
    let documented = guide_rows(&guide);

    let output = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(["recipes", "list"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "recipe listing failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let listing = String::from_utf8(output.stdout).unwrap();
    let listed: BTreeMap<String, (String, String, String)> = listing
        .lines()
        .filter(|line| !line.starts_with("warning:"))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            (fields.len() == 7).then(|| {
                (
                    fields[0].to_string(),
                    (
                        fields[5].to_string(),
                        fields[3].to_string(),
                        fields[4].to_string(),
                    ),
                )
            })
        })
        .collect();

    let mut library = Vec::new();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root.join("recipes"))
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join("bundle.json").is_file())
        .collect();
    dirs.sort();
    for dir in dirs {
        let raw = std::fs::read_to_string(dir.join("bundle.json")).unwrap();
        let bundle: Value = serde_json::from_str(&raw).unwrap();
        let name = bundle["name"].as_str().unwrap();
        let description = bundle["description"].as_str().unwrap();
        let cost = bundle["cost"].as_str().unwrap();
        assert!(
            !description.is_empty() && !description.contains(['\r', '\n']),
            "{name} needs a one-line description"
        );
        let (listed_description, seats, listed_cost) = listed
            .get(name)
            .unwrap_or_else(|| panic!("brokkr recipes list omitted {name}"));
        assert_eq!(listed_description, description, "{name} description");
        assert_eq!(listed_cost, cost, "{name} cost");
        library.push((
            name.to_string(),
            description.to_string(),
            seats.clone(),
            cost.to_string(),
        ));
    }
    assert_eq!(documented, library, "guide table drifted from recipe data");
}

#[test]
fn the_platform_gate_carries_every_part_of_the_ruling() {
    let root = workspace();
    let template = std::fs::read_to_string(root.join(".github/pull_request_template.md")).unwrap();
    assert!(template.lines().any(|line| line == "Brokkr-Run: <run id>"));

    let workflow = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    for binding in [
        "name: delivered by brokkr",
        "github.event.pull_request.base.sha",
        "fetch-depth: 0",
        "scripts/delivered-by-brokkr.sh",
        "join(github.event.pull_request.labels.*.name, ',')",
        // Decision 0038 ruling 4: the label re-runs the gate, no reopen.
        "types: [opened, synchronize, reopened, labeled, unlabeled]",
    ] {
        assert!(workflow.contains(binding), "CI lost binding: {binding}");
    }

    // The gate itself is repository-owned data too (0033 ruling 4, 0038
    // rulings 2, 3 and 6): every check the ruling names is in the script.
    let gate = std::fs::read_to_string(root.join("scripts/delivered-by-brokkr.sh")).unwrap();
    for binding in [
        "refs/heads/brokkr-runs/${run}",
        "verify-run \"$work/${run}.ndjson\"",
        ".payload.from == \"ship\" and .payload.result == \"shipped\"",
        ".seq == $seq and .journal_head_hash == $journal",
        "patch-id --verbatim",
        ".repo_head == $head",
        "Brokkr-Preflight",
        ".payload.from == \"review\" and .payload.next == \"done\"",
        ".classes.docs.paths",
        "by-hand label",
        "the tier would have been",
        // Decision 0046 ruling 3: the boundary is read off every
        // `effect/started` entry after the journal verifies, and the
        // adjective is the script's rendering of the plain word.
        "boundary_suffix()",
        ".payload.boundary",
        "IN(\"harness\", \"open\")",
        " · unboxed",
        " · boundary not recorded",
        "has(\"member\")",
    ] {
        assert!(gate.contains(binding), "the gate lost binding: {binding}");
    }

    // 0038 ruling 3: the docs class is data, not a pattern in a workflow.
    let classes: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".github/delivery-classes.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(classes["schema"], "forge.delivery-classes/v1");
    let docs = classes["classes"]["docs"]["paths"]
        .as_array()
        .expect("docs paths");
    assert!(!docs.is_empty());
    assert!(template.contains("Brokkr-Preflight: <run id>"));

    let manual = std::fs::read_to_string(root.join("docs/guides/contributing-by-hand.md")).unwrap();
    assert!(
        manual.lines().count() >= 600,
        "the old handbook was not preserved whole"
    );
    for preserved in [
        "## The nine checks",
        "## The coverage gate, practically",
        "## Commits, signing, and how your PR actually lands",
        "## The decision culture",
        "## What is frozen",
    ] {
        assert!(manual.contains(preserved), "manual guide lost {preserved}");
    }
}

/// Decision 0046's guides (boundary-guides / The guides document the
/// boundary and never lose a section): every page keeps the sections it
/// had and gained the rows the decision commissions, no page states
/// that the network was off under `harness` or `open`, and the decision
/// carries its one-line erratum under a heading of its own.
#[test]
fn the_boundary_guides_keep_every_section_and_gained_the_rows() {
    let root = workspace();
    // (page, kept, gained) — matched on whitespace-collapsed text so a
    // reflowed paragraph is not a lost row.
    let pages: [(&str, &[&str], &[&str]); 10] = [
        (
            "docs/guides/provider-adapters.md",
            &["A provider adapter is **data**", "## Hands"],
            &[
                "ok boundaries:",
                "### `hands.harness`",
                "| `gate` |",
                "| `work` |",
                "| `result` |",
                "`{result_path}`",
                "`{hands_mcp_json}` and `{hands_args_toml}`, are refused",
                "pending and the operator's**",
                "declares **no** `hands.harness` member yet",
            ],
        ),
        (
            "docs/guides/recipe-authoring.md",
            &["$ brokkr recipes list", "## The three files"],
            &[
                "| `driver.confine` | **Refused.**",
                "decision [0046](../decisions/0046-the-boundary-is-named.md) ruling 5",
                "| `hands` | Decision 0043:",
                "`mask` **declared and not enforced**",
                "Clearing the environment confines nothing on disk",
            ],
        ),
        (
            "docs/guides/quickstart.md",
            &["### Step 1 — install", "#### Re-run under another strategy"],
            &[
                "`\"boundary\": \"harness\"`",
                "renders such a run *unboxed*",
                "`bundles/self` and `recipes/panel-review`",
                "`recipes/triage` and `recipes/night-shift`",
                "crates/brokkr-runtime/src/bundle/model_policy_tests.rs",
                "The rerun compiles the new recipe in the discovered realm",
            ],
        ),
        (
            "docs/guides/journal-and-verification.md",
            &["$ brokkr verify-run", "`brokkr costs --run <id>`"],
            &[
                "## The boundary on the record, and the word *unboxed*",
                "`effect-boundary.v1.schema.json`",
                "`no boundary recorded`",
            ],
        ),
        (
            "docs/guides/repository-layout.md",
            &["| `crates/` | The engine:"],
            &[
                "`house`, `dialect` and `boundary`",
                "`realms.v4`",
                "`run-manifest.v9`",
                "`seat-record.v4`",
                "`effect-boundary.v1`",
            ],
        ),
        (
            "docs/guides/driver-authoring.md",
            &["## Wiring it into a bundle", "## See also"],
            &[
                "under the `namespace` boundary the engine runs the whole script through `brokkr hands exec`",
                "re-walks the script's declaring layer",
                "`driver.confine`",
                "decision 0046 ruling 5",
            ],
        ),
        (
            "ARCHITECTURE.md",
            &["## Drivers", "## Verification, in layers"],
            &["the realm's **boundary** (decision 0046)", "0046 ruling 5"],
        ),
        (
            "docs/extension-model.md",
            &["**Status**: partially accepted", "| `trust` |"],
            &["The wall itself is the realm's `boundary` (decision 0046"],
        ),
        (
            "docs/target-architecture.md",
            &[
                "**Status**: implementation blueprint, accepted 2026-08-22",
                "| `policy-confined` |",
                "| `public-evidence-only` |",
            ],
            &["Decision 0046's `container` boundary", "slice (iii)"],
        ),
        (
            "contracts/README.md",
            &["## Event vocabulary (v1)", "## Fold semantics"],
            &[
                "`realms.v4.schema.json`",
                "`run-manifest.v9.schema.json`",
                "`seat-record.v4.schema.json`",
                "`effect-boundary.v1.schema.json`",
                "`effect/started.boundary`",
            ],
        ),
    ];
    for (page, kept, gained) in pages {
        let raw = std::fs::read_to_string(root.join(page)).unwrap();
        let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        for section in kept {
            assert!(text.contains(section), "{page} lost: {section}");
        }
        for row in gained {
            assert!(text.contains(row), "{page} never gained: {row}");
        }
        // Task 12.8 (design DD15): the prefix is a narrowing the engine
        // attempts on Linux, and no page says the network was off.
        let lower = text.to_lowercase();
        for claim in ["network was off", "network is off", "network off"] {
            assert!(
                !lower.contains(claim),
                "{page} states the network was off: {claim}"
            );
        }
    }

    // The erratum: one heading, one line, the decision otherwise untouched.
    let decision =
        std::fs::read_to_string(root.join("docs/decisions/0046-the-boundary-is-named.md")).unwrap();
    assert!(decision.starts_with("# 0046 — The boundary is named"));
    assert!(decision.contains("\nStatus: accepted (operator ruled in chat, 2026-09-05)\n"));
    let erratum: Vec<&str> = decision
        .split("\n## Erratum\n")
        .nth(1)
        .expect("the decision carries its erratum heading")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(
        erratum.len(),
        1,
        "the erratum is exactly one line: {erratum:?}"
    );
    for named in [
        "`seat-record.v3`",
        "`seat-record.v4`",
        "#202",
        "decision 0034 rulings 6 and 7",
        "nothing else is renumbered",
    ] {
        assert!(
            erratum[0].contains(named),
            "the erratum does not name {named}"
        );
    }
}

/// Section and table-label inventory from the committed pre-0046 guides.
/// New sections are welcome; deleting existing guidance is not part of this slice.
#[test]
fn every_original_guide_section_and_table_row_remains_available() {
    let inventory = [
        ("docs/guides/provider-adapters.md", "# Provider adapters\n## Hands", "", ""),
        ("docs/guides/recipe-authoring.md", "# Recipe authoring — bundles, policy tables, and composition\n## Recipes and composition\n## The three files\n## `bundle.json` anatomy\n### A seat\n## Seat bodies: single, panel, sequence, select\n## Composition: `extends` and `override`\n## Digest identity\n## The policy table\n### A rule\n## The condition vocabulary\n## The reforging ladder\n## Compile it\n## See also", "Recipe\n---\n[`fast`](../../recipes/fast)\n[`triage`](../../recipes/triage/README.md)\n[`night-shift`](../../recipes/night-shift/README.md)\n[`wager-harness`](../../recipes/wager-harness/README.md)\n[`wager-harness-dsh`](../../recipes/wager-harness-dsh/README.md)\n[`wager-harness-muse`](../../recipes/wager-harness-muse/README.md)\n[`node`](../../recipes/node/README.md)\n[`panel-review`](../../recipes/panel-review)\n[`preflight`](../../recipes/preflight/README.md)\nKey\n`name`\n`description`\n`cost`\n`policy`\n`protected_phase`\n`egress_minimum`\n`seats`\n`extends`\n`override`, `remove`\n`results`\n`role` + `driver`\n`agent`\n`inputs`\n`limits`\n`secrets`\n`driver.confine`\n`hands`\nAggregate\n`unanimous-pass`\n`review-panel`\n`id`\n`from`\n`result`\n`reason`\n`next`\n`park`\n`severity`\n`when`\n`requires_artifacts`\nForm\nbare name\n`strategy_in`\n`<counter>_gte`\n`visits_<phase>_gte`\n`<axis>_above`\n`<axis>_at_most`\nRule\n`…-EXHAUSTED-ABOVE-MEDIUM`\n`…-EXHAUSTED-MEDIUM`\n`…-EXHAUSTED-DEBT`\n`…-EXHAUSTED-UNFIXED`", ""),
        ("docs/guides/quickstart.md", "# Quickstart — one spine, and everything else is a diff over it\n## The spine\n### Step 1 — install\n### Step 2 — `brokkr init .`\n### Step 3 — `brokkr run`\n### Step 4 — read the journal\n## Per-stack cards\n## Flow 2 — deliver\n## Flow 3 — adopt\n## After the spine\n### Where the run wrote things\n### The escape hatches\n#### Operator commands — `retry` and `stop`\n#### Resume\n#### Conclude — closing a run whose bundle no longer compiles\n#### Re-run under another strategy\n### What it cost\n### What the budgets do not cover\n### Limits worth knowing\n## Compact first-run tour\n## Next", "\n---\n1\n2\n3\n4\nChannel\ntarball\ncargo\nnix\napt\ndnf\nbrew\nscoop\nFlag\n`--repo <path>`\n`--db <path>`\n`--realms <file>`\n`--recipes-dir <path>`\n`--secrets-file <path>`\n`--dispatch <file>`\nCard\n[node](cards/node.md)\n[bun](cards/bun.md)\n[rust](cards/rust.md)\n[go](cards/go.md)\n[python](cards/python.md)", ""),
        ("docs/guides/journal-and-verification.md", "# The journal and verification\n#   <run>.redacted.ndjson — paths and usernames as stable placeholders, hashes\n#   verify only on the verbatim pair, and the manifest says so", "", ""),
        ("docs/guides/read-surfaces.md", "# The read surfaces\n### `brokkr realms` — the world\n### `brokkr runs` — the fleet\n### `brokkr inspect` — one run, explained\n### `brokkr watch` — the same, live\n### `brokkr tui` — the readouts made explorable\n### `brokkr ui` — the browser console\n### `brokkr muninn` — the fleet, read and advised on", "", ""),
        ("docs/guides/repository-layout.md", "# Repo layout", "Path\n---\n[`ARCHITECTURE.md`](../../ARCHITECTURE.md)\n[`CONTRIBUTING.md`](../../CONTRIBUTING.md)\n`crates/`\n`contracts/`\n`realms.json`\n`docs/house-rules.md`\n`bundles/`\n`recipes/`\n`agents/`\n`dialects/`\n`adapters/`\n`fixtures/`\n`policy/phase-machine.json`\n[`docs/decisions/`](../decisions/)\n[`docs/lore/`](../lore/)\n`assets/`\n`reference/`\n`scripts/coverage-exact.sh`", ""),
        ("docs/guides/driver-authoring.md", "# Driver authoring — the `forge-driver/v1` wire contract\n## Transport\n## The message family\n## The exchange, in order\n## What the engine actually sends\n## `resume` — rejoining the session you opened\n## `accepted` is the load-bearing message\n## Checkpoints\n## Results\n## The result-file contract\n## Deadlines and kills\n## A minimal driver, in prose\n## The conformance suite is the acceptance test\n## Wiring it into a bundle\n## See also", "Message\n---\n`hello`\n`capabilities`\n`start`\n`accepted`\n`checkpoint`\n`result`\n`resume`\n`cancel`\n`cancelled`\n`shutdown`\nWhat happened\nYour process exits **without** `accepted` and without a result\nYour process exits **after** `accepted` and without a result\nYou send `result` with `status: \"failed\"`\nYou violate the protocol\nField\n`input_tokens`\n`output_tokens`\n`cache_read_tokens`\n`reasoning_output_tokens`\n`cache_write_tokens`\nKey\n`inputs`\n`notes`\n`model`\n`effort`", ""),
        ("ARCHITECTURE.md", "# Architecture\n## The shape\n## The journal is the run\n## Every effect, in order\n## Policy is data\n## A bundle, resolved\n## Drivers\n## Verification, in layers\n## The operating surface", "Layer\n---\nDifferential corpus\nMachine proof\nSelf-delivery\nBrokkr verification", "**Status**: the system as implemented. The blueprint it grew from is"),
        ("docs/extension-model.md", "# Extension model — nodes, seats, and what may never be unplugged\n## Layer 1 — Phases (nodes of the outer machine)\n## Layer 2 — Seats (agents inside a phase)\n## Layer 3 — Profiles (the stack-specific bundle)\n## Resolved\n## Open questions for discussion", "Field\n---\n`role`\n`class`\n`trust`\n`result_schema`\n`driver`", "**Status**: partially accepted. Decisions 0002 and 0003 lock the outer-machine,"),
        ("docs/target-architecture.md", "# Target architecture\n## Product contract\n## System shape\n## Rust workspace and one shipped binary\n## State and control status\n## Event and effect protocol\n## SQLite and artifacts\n## Declarative bundles\n## Run manifest and versioning\n## Drivers and isolation\n## Cordis and other long-horizon harnesses\n## Local API and embedded UI\n## Audit and evaluation\n## Installation and operation\n## Delivery sequence\n## First-release acceptance criteria\n## Deferred choices\n## References", "Crate\n---\n`brokkr-core`\n`brokkr-store`\n`brokkr-runtime`\n`brokkr-protocol`\n`brokkr-api`\n`brokkr-cli`\nPrimitive\n`seat`\n`parallel`\n`join`\n`loop`\n`gate`\n`tool`\n`submachine`\n`emit-result`\nTrust\n`trusted`\n`policy-confined`\n`public-evidence-only`", "**Status**: implementation blueprint, accepted 2026-08-22 under"),
    ];
    for (path, headings, labels, statuses) in inventory {
        let text = std::fs::read_to_string(workspace().join(path)).unwrap();
        for heading in headings.lines() {
            assert!(
                text.lines().any(|line| line == heading),
                "{path} lost section {heading}"
            );
        }
        let current: Vec<&str> = text
            .lines()
            .filter(|line| line.starts_with('|'))
            .filter_map(|line| line.split('|').nth(1))
            .map(str::trim)
            .collect();
        for label in labels.lines() {
            assert!(current.contains(&label), "{path} lost row {label}");
        }
        for status in statuses.lines() {
            assert!(
                text.lines().any(|line| line == status),
                "{path} changed its status"
            );
        }
    }
}
