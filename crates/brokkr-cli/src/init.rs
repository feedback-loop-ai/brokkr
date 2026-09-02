//! `brokkr init <dir>` — scaffold a minimal reviewable bundle and prove it
//! compiles. The template carries the tightened ship taxonomy (`ready` →
//! `shipped` as the sole entry into `done`), the protected review phase,
//! one charter-defined seat per phase, per-seat limits (decision 0006),
//! the work/gate division of decision 0021 ruling 1, and the bundled
//! headless Claude Code driver. Everything written is ordinary text meant
//! to be reviewed and edited in git.
//!
//! The scaffold is a WORKSPACE, not only a bundle: it carries its own
//! `adapters/` and `agents/` trees, because since decision 0021 the tier
//! that lets a gate seat judge is adapter data, and since decision 0016
//! the tools a seat may run are the agent data its `tools.allow` names
//! expressed through the adapter's `tool_permissions.names`. A starter
//! whose review seat judged on nobody's authority, or whose seats could
//! not be granted the stack's own build and test commands, would teach
//! the wrong lesson on day one — the scaffold's seats reference the
//! scaffold's own agents, and those agents' allowances are exactly the
//! tools the stack that was detected needs. The operator's trust
//! declarations and tool grants are theirs to edit — which is why they
//! are scaffolded as files in their tree rather than compiled into this
//! binary — and `brokkr` is run from inside the scaffold, where its
//! `adapters/` and `agents/` are the workspace's.
//!
//! init LOOKS BEFORE IT SCAFFOLDS. The seats that build and prove a
//! change are told which commands to run, and a charter that told every
//! repository to "run the project's test suite" told none of them
//! anything. So the repository being delivered — the workspace `brokkr`
//! was invoked from, not the directory the recipe lands in — is read for
//! the manifests and lockfiles at its root, and the implementer's and
//! verifier's charters name that stack's own commands. Nothing is
//! executed to find out: file presence is the whole of the evidence, and
//! a guess that says which files it was read from is one an operator can
//! correct. Where no marker is recognized, the charters say so in those
//! words rather than dressing a placeholder as a choice.
//!
//! Detection reads two tables, both data, in this order:
//!
//! 1. `ORCHESTRATORS` — a monorepo's build tool (`turbo.json`, `nx.json`)
//!    outranks any single package's manifest, because in a monorepo the
//!    package's own script is the wrong command: it proves one workspace
//!    member and calls the repository green. The orchestrator's command
//!    runs through the repository's OWN package manager, which is a
//!    second axis: whichever lockfile is at the root picks the runner
//!    prefix (`RUNNERS`), and `npx` is what is left.
//! 2. `STACKS` — the per-manifest table. A language manifest outranks the
//!    `Makefile` catch-all; within one manifest the narrower marker set
//!    comes first, so `bun.lock` beside `package.json` out-votes the npm
//!    fallback and `uv.lock` beside `pyproject.toml` out-votes pip.
//!
//! A third table, `WORKSPACES`, adds no commands. `cargo build
//! --workspace` from a Cargo workspace root and `go build ./...` beside a
//! `go.work` already span every member; what was missing was the charter
//! SAYING it is a workspace, so a seat does not go hunting for a
//! per-crate or per-module command that nobody needed.
//!
//! The same stack decides the scaffold's TOOL GRANTS. The seats are
//! claude sessions, and a claude session that may run anything runs
//! nothing headless: it stops at every shell prompt it is not allowed to
//! answer. So the stack's own runners — the leading binaries of its
//! build, test and lint commands — are granted by name, as
//! `Bash(<bin>:*)` entries written into the scaffolded
//! `adapters/claude.json` `tool_permissions.names` map AND into the
//! scaffolded agents' `tools.allow` lists (decision 0016: an allowance
//! the adapter cannot express is a compile refusal, so the two files are
//! one grant, not two). The division is decision 0021 ruling 1's: the
//! WORK-class agents (intake, implement) get the full set — the stack's
//! runners plus `git`, `ls`, `rg` and `mkdir` — so a seat may run
//! exactly the commands its charter names and nothing broader; the
//! GATE-class agents (verify, review, ship) get the read-only subset —
//! the stack's runner, `git`, `ls` and `rg`, and never the write tools —
//! because nobody stands behind the judges. A stack no row of the two
//! tables recognizes grants nothing: the names map stays EMPTY and the
//! scaffold's README says so in those words, because a tool name this
//! file cannot back with a `Bash(...)` expression is a name invented,
//! and a charter that tells a seat to guess is the silence this feature
//! exists to end.

use std::path::Path;

use anyhow::{bail, Context, Result};
use brokkr_runtime::bundle::{DEFAULT_ADAPTERS_DIR, DEFAULT_AGENTS_DIR};
use brokkr_runtime::Bundle;
use serde_json::{json, Map};

// Drivers are built into the brokkr binary itself (decision 0009):
// scaffolds reference them as {brokkr} driver <kind>.

const POLICY: &str = r#"{
  "schema": "forge.phase-machine/v1",
  "description": "Starter table scaffolded by `brokkr init`. Linear machine, one seat per phase. The review gate is constitutionally protected: every path to done passes review, security-hold is a hard stop, security residuals never take the tracked-debt path.",
  "phases": ["intake", "implement", "verify", "review", "ship", "done", "stop"],
  "initial": "intake",
  "terminal": ["done", "stop"],
  "shippable_from": ["review"],
  "rules": [
    {"id": "INTAKE-OK", "from": "intake", "result": "resolved", "next": "implement",
     "reason": "Task framed and recorded."},
    {"id": "IMPL-BROKEN-TWICE", "from": "implement", "result": "broken",
     "when": {"consecutive_failures_gte": 2}, "next": "stop", "severity": "hard",
     "reason": "Two consecutive broken implement runs; stop rather than thrash."},
    {"id": "IMPL-BROKEN-RETRY", "from": "implement", "result": "broken",
     "next": "implement", "reason": "First broken run; one re-run permitted."},
    {"id": "IMPL-BLOCKED", "from": "implement", "result": "blocked", "next": "stop",
     "severity": "hard", "reason": "Implementer blocked; report, never silently continue."},
    {"id": "IMPL-OK", "from": "implement", "result": "complete", "next": "verify",
     "reason": "Implementation complete and committed."},
    {"id": "VERIFY-FAIL", "from": "verify", "result": "fail", "next": "stop",
     "severity": "hard", "reason": "Verification failed; not shippable."},
    {"id": "VERIFY-PASS", "from": "verify", "result": "pass", "next": "review",
     "reason": "Suite green; reviewers read verified code."},
    {"id": "REVIEW-SECURITY-HOLD", "from": "review", "result": "security-hold",
     "next": "stop", "severity": "hard",
     "reason": "Unresolved security findings. NEVER ship; risk acceptance is the operator's."},
    {"id": "REVIEW-RESIDUAL-ABOVE-MEDIUM", "from": "review", "result": "residual",
     "when": {"max_residual_severity_above": "medium"}, "next": "stop",
     "severity": "hard", "reason": "Residual severity above medium; not shippable."},
    {"id": "REVIEW-RESIDUAL-SECURITY", "from": "review", "result": "residual",
     "when": {"has_security_residual": true}, "next": "stop", "severity": "hard",
     "reason": "Security residuals never take the tracked-debt path."},
    {"id": "REVIEW-RESIDUAL-OK", "from": "review", "result": "residual", "next": "ship",
     "severity": "flagged",
     "reason": "Non-security residuals at or below medium proceed as tracked debt."},
    {"id": "REVIEW-CLEAN-NO-FIXES", "from": "review", "result": "clean",
     "when": {"fixes_applied": false}, "next": "ship",
     "reason": "Clean with no code changed; verification evidence stands."},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "verify",
     "reason": "Clean but fixes applied; re-verify before shipping."},
    {"id": "SHIP-DRIFT", "from": "ship", "result": "ready",
     "when": {"drift_detected": true}, "next": "review", "severity": "flagged",
     "reason": "HEAD moved after review; re-arm a scoped review."},
    {"id": "SHIP-DIRTY", "from": "ship", "result": "ready",
     "when": {"dirty_worktrees": true}, "next": "stop", "severity": "hard",
     "reason": "Dirty tree at ship time is a defect."},
    {"id": "SHIP-READY", "from": "ship", "result": "ready", "next": "ship",
     "reason": "Gates passed and ledger written; confirm close-out and report shipped."},
    {"id": "SHIPPED-DRIFT", "from": "ship", "result": "shipped",
     "when": {"drift_detected": true}, "next": "review", "severity": "flagged",
     "reason": "HEAD moved between ready and close-out; re-arm a scoped review."},
    {"id": "SHIPPED-DIRTY", "from": "ship", "result": "shipped",
     "when": {"dirty_worktrees": true}, "next": "stop", "severity": "hard",
     "reason": "A dirty worktree at close-out is a defect."},
    {"id": "SHIP-COMPLETE", "from": "ship", "result": "shipped", "next": "done",
     "reason": "Close-out confirmed. The operator pushes and merges."}
  ]
}
"#;

/// The invariant seat roster, as a bundle that seats the scaffold's own
/// AGENTS (decision 0016). What varies by stack — the commands a seat may
/// run — lives in the agent files' `tools.allow` and the adapter's
/// `tool_permissions.names`, not here, so this file is byte-identical for
/// every repository: `intake` and `implement` declare the WORK class of
/// decision 0021 ruling 1, `verify`, `review` and `ship` declare the GATE
/// class, and each seat references the agent that carries its charter and
/// its allowance. The 0006 bounds the starter used to declare inline now
/// ride on the agents, exactly as adoption moved them.
const BUNDLE: &str = r#"{
  "name": "starter",
  "policy": "policy.json",
  "protected_phase": "review",
  "seats": {
    "intake": {
      "class": "work",
      "results": ["resolved"],
      "agent": "intake"
    },
    "implement": {
      "class": "work",
      "results": ["complete", "broken", "blocked"],
      "agent": "implementer"
    },
    "verify": {
      "class": "gate",
      "results": ["pass", "fail"],
      "agent": "verifier"
    },
    "review": {
      "class": "gate",
      "results": ["clean", "residual", "security-hold"],
      "agent": "reviewer"
    },
    "ship": {
      "class": "gate",
      "results": ["ready", "shipped"],
      "agent": "shipper"
    }
  }
}
"#;

/// One recognized stack: the marker files that identify it, the name the
/// charters call it by, and the three commands a seat would actually run
/// there. Data, walked in order — the only thing the code matches on is
/// whether the named files are present.
struct Stack {
    /// The stack's own vocabulary, not a label invented here.
    name: &'static str,
    /// Every marker must be present at the repository root. No recursion:
    /// a scaffold is a starting point, not a monorepo analyzer.
    markers: &'static [&'static str],
    build: &'static str,
    test: &'static str,
    lint: &'static str,
}

/// The detection table, in precedence order. A language manifest outranks
/// the `Makefile` catch-all, because a repository carrying both usually
/// wraps the one in the other and the manifest is the more specific
/// truth. Within one manifest, the narrower marker set comes first: the
/// lockfiles have their say, and plain `package.json` is what is left.
const STACKS: &[Stack] = &[
    Stack {
        name: "rust",
        markers: &["Cargo.toml"],
        build: "cargo build --workspace",
        test: "cargo test --workspace",
        lint: "cargo clippy --workspace --all-targets -- -D warnings",
    },
    // Bun ahead of the three older node arms, and ahead of them all
    // because `bun.lock` is the narrower evidence: a bun-managed
    // repository carries only `package.json` as far as the npm fallback
    // can see, and the fallback then writes `npm test` into the charter
    // of a repository that has no npm lockfile to install from.
    //
    // This arm names an INSTALL step where no other node arm does, and
    // that is deliberate: `bun install --frozen-lockfile` is the
    // command that makes `bun run test` mean anything, and bun's install
    // is fast enough that a charter can honestly ask for it. The three
    // commands are the request's own — "bun install --frozen-lockfile,
    // bun run test / typecheck" — mapped onto the seats that get them:
    // build+test to the implementer, test+lint to the verifier, so the
    // verifier is handed `bun run test` and `bun run typecheck` and the
    // implementer is handed the install and the test.
    Stack {
        name: "node/bun",
        markers: &["package.json", "bun.lock"],
        build: "bun install --frozen-lockfile",
        test: "bun run test",
        lint: "bun run typecheck",
    },
    Stack {
        name: "node/pnpm",
        markers: &["package.json", "pnpm-lock.yaml"],
        build: "pnpm build",
        test: "pnpm test",
        lint: "pnpm lint",
    },
    Stack {
        name: "node/yarn",
        markers: &["package.json", "yarn.lock"],
        build: "yarn build",
        test: "yarn test",
        lint: "yarn lint",
    },
    Stack {
        name: "node/npm",
        markers: &["package.json"],
        build: "npm run build",
        test: "npm test",
        lint: "npm run lint",
    },
    // Same rule again, one language over: `uv.lock` is the narrower
    // evidence and uv is the environment the repository actually
    // resolves in, so `uv run pytest` is the honest spelling of "run the
    // suite" there and a bare `python -m pytest` would run against
    // whatever interpreter the seat happened to inherit.
    Stack {
        name: "python/uv",
        markers: &["pyproject.toml", "uv.lock"],
        build: "uv sync",
        test: "uv run pytest",
        lint: "uv run ruff check .",
    },
    // The pip fallback, one language over from uv. The commands name
    // `python3`, not `python`, because the tool grants a scaffold can
    // express are Bash(prefix:*) rules and the interpreter a seat
    // actually runs in a fresh project is python3 — a charter that said
    // `python` would hand the seat a command its own allowance cannot
    // answer. pytest is the venv's suite binary and is granted beside
    // the interpreter for the same reason.
    Stack {
        name: "python",
        markers: &["pyproject.toml"],
        build: "python3 -m build",
        test: "python3 -m pytest",
        lint: "python3 -m ruff check .",
    },
    Stack {
        name: "go",
        markers: &["go.mod"],
        build: "go build ./...",
        test: "go test ./...",
        lint: "go vet ./...",
    },
    Stack {
        name: "make",
        markers: &["Makefile"],
        build: "make build",
        test: "make test",
        lint: "make lint",
    },
];

/// How a repository runs a locally-installed JavaScript binary, keyed by
/// the lockfile that says which package manager put it there. Walked in
/// order; the first lockfile present wins, and when none is `npx` is
/// what is left — npm ships with node, and `npx` resolves a local
/// install before it reaches for the registry.
const RUNNERS: &[(&str, &str)] = &[
    ("bun.lock", "bunx"),
    ("pnpm-lock.yaml", "pnpm exec"),
    ("yarn.lock", "yarn exec"),
];

/// One monorepo build orchestrator. Same shape as `Stack`, except the
/// commands carry a `{runner}` placeholder: WHICH package manager runs
/// turbo or nx is the repository's business, read from `RUNNERS`, and a
/// table that hard-coded `npx` would be the same guess this arm exists
/// to stop making.
struct Orchestrator {
    name: &'static str,
    markers: &'static [&'static str],
    build: &'static str,
    test: &'static str,
    lint: &'static str,
    note: &'static str,
}

/// The orchestrator table, matched BEFORE `STACKS`. A repository with a
/// `turbo.json` at its root is a monorepo whose `package.json` scripts
/// belong to one member; naming a member's script there would prove one
/// package and call the whole repository green.
const ORCHESTRATORS: &[Orchestrator] = &[
    Orchestrator {
        name: "node/turbo",
        markers: &["package.json", "turbo.json"],
        build: "{runner} turbo run build",
        test: "{runner} turbo run test",
        lint: "{runner} turbo run lint",
        note: "This is a MONOREPO: `turbo.json` names an orchestrator, and the\n\
               commands above are the orchestrator's own — they span every\n\
               workspace package. Do not substitute a single package's script.",
    },
    Orchestrator {
        name: "node/nx",
        markers: &["package.json", "nx.json"],
        build: "{runner} nx run-many -t build",
        test: "{runner} nx run-many -t test",
        lint: "{runner} nx run-many -t lint",
        note: "This is a MONOREPO: `nx.json` names an orchestrator, and the\n\
               commands above are the orchestrator's own — they span every\n\
               workspace project. Do not substitute a single project's target.",
    },
];

/// A workspace is a fact ABOUT a stack, not another stack. `cargo build
/// --workspace` from a workspace root already builds every member crate
/// and `go build ./...` beside a `go.work` already builds every module
/// it lists — the commands were right, the charter was silent. So this
/// table adds a sentence and no syntax.
///
/// `needle` is matched against whole trimmed LINES, not as a substring:
/// a `Cargo.toml` whose comments discuss `[workspace]` is not one, and a
/// charter that told a lone crate it was a workspace would be the same
/// dishonesty this file exists to stop. The empty needle is the honest
/// spelling of "the file's presence is the whole of the evidence": a
/// `go.work` is a workspace by existing.
struct Workspace {
    stack: &'static str,
    file: &'static str,
    needle: &'static str,
    note: &'static str,
}

const WORKSPACES: &[Workspace] = &[
    Workspace {
        stack: "rust",
        file: "Cargo.toml",
        needle: "[workspace]",
        note: "This is a CARGO WORKSPACE (`[workspace]` in `Cargo.toml`). The\n\
               `--workspace` flag above already spans every member crate — there\n\
               is no per-crate command to go looking for.",
    },
    Workspace {
        stack: "go",
        file: "go.work",
        needle: "",
        note: "This is a GO WORKSPACE (`go.work` at the root). The `./...` above\n\
               already spans every module the workspace lists — there is no\n\
               per-module command to go looking for.",
    },
];

/// What detection concluded: the name the charters call the stack, the
/// evidence quoted back so the guess can be checked, the three commands,
/// and — where the root said so — the sentence that names this a
/// monorepo or a workspace.
struct Detected {
    name: String,
    evidence: String,
    build: String,
    test: String,
    lint: String,
    note: Option<&'static str>,
}

/// One granted tool: the name the adapter's `tool_permissions.names` map
/// and an agent's `tools.allow` list share, and the `Bash(<bin>:*)`
/// expression the claude CLI understands for it. Static data, in the
/// vocabulary the shipped adapters already carry — a scaffold never
/// invents a tool name, because an invented name is one no `Bash(...)`
/// expression can back and no seat can be granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tool {
    name: &'static str,
    permission: &'static str,
}

/// The read trio every seat — judges included — needs: `git` to read the
/// tree and its history, `ls` and `rg` to find and to search.
const GIT: Tool = Tool {
    name: "git",
    permission: "Bash(git:*)",
};
const LS: Tool = Tool {
    name: "ls",
    permission: "Bash(ls:*)",
};
const RG: Tool = Tool {
    name: "rg",
    permission: "Bash(rg:*)",
};
/// The one WRITE tool: `mkdir`, held only by work seats. A gate seat
/// judges; nothing a judge does needs to create a directory.
const MKDIR: Tool = Tool {
    name: "mkdir",
    permission: "Bash(mkdir:*)",
};

/// The per-stack runner tools, each the leading binary of one command in
/// the two detection tables.
const CARGO: Tool = Tool {
    name: "cargo",
    permission: "Bash(cargo:*)",
};
const BUN: Tool = Tool {
    name: "bun",
    permission: "Bash(bun:*)",
};
const BUNX: Tool = Tool {
    name: "bunx",
    permission: "Bash(bunx:*)",
};
const PNPM: Tool = Tool {
    name: "pnpm",
    permission: "Bash(pnpm:*)",
};
const YARN: Tool = Tool {
    name: "yarn",
    permission: "Bash(yarn:*)",
};
const NPM: Tool = Tool {
    name: "npm",
    permission: "Bash(npm:*)",
};
const NPX: Tool = Tool {
    name: "npx",
    permission: "Bash(npx:*)",
};
const UV: Tool = Tool {
    name: "uv",
    permission: "Bash(uv:*)",
};
const PYTHON3: Tool = Tool {
    name: "python3",
    permission: "Bash(python3:*)",
};
/// pytest is not a runner any python command leads with — it is the
/// venv's suite binary, and its allowance is narrower than the
/// interpreter's: a seat may run the suite through it without holding
/// the binary that could also build with it.
const PYTEST: Tool = Tool {
    name: "pytest",
    permission: "Bash(.venv/bin/pytest:*)",
};
const GO: Tool = Tool {
    name: "go",
    permission: "Bash(go:*)",
};
const MAKE: Tool = Tool {
    name: "make",
    permission: "Bash(make:*)",
};

/// The tool grants one scaffold carries, sized by decision 0021 ruling
/// 1's two classes:
///
/// - `work` — the allowance of the WORK-class agents (intake, implement):
///   the stack's runners, the read trio, and `mkdir`, so a seat may run
///   exactly the commands its charter names and nothing broader;
/// - `gate` — the allowance of the GATE-class agents (verify, review,
///   ship): the read-only subset — the stack's runner (which for every
///   row of the two tables also runs its test and lint commands), `git`,
///   `ls` and `rg` — and never the write tools.
#[derive(Debug, Clone)]
struct Toolset {
    work: Vec<Tool>,
    gate: Vec<Tool>,
}

/// The leading binary of one command token. The one rewrite is the
/// plain-python row: its commands run through `python3`, and `pytest` is
/// granted beside it for the reason [`PYTEST`] states.
fn tools_for_token(detected: &Detected, token: &str) -> Vec<Tool> {
    if detected.name == "python" && token == "python3" {
        return vec![PYTHON3, PYTEST];
    }
    vec![match token {
        "cargo" => CARGO,
        "bun" => BUN,
        "bunx" => BUNX,
        "pnpm" => PNPM,
        "yarn" => YARN,
        "npm" => NPM,
        "npx" => NPX,
        "uv" => UV,
        "go" => GO,
        "make" => MAKE,
        other => panic!(
            "the '{other}' runner of a recognized stack has no Bash grant; \
             add a Tool beside the stack row that names it"
        ),
    }]
}

/// The runner tools one detected stack's commands go through: the leading
/// binary of each of its build, test and lint commands, deduplicated and
/// in that order. A scaffold grants exactly these runners, never a binary
/// the charters do not name.
fn runner_tools(detected: &Detected) -> Vec<Tool> {
    let mut tools: Vec<Tool> = Vec::new();
    for command in [&detected.build, &detected.test, &detected.lint] {
        let token = command.split_whitespace().next().unwrap_or_default();
        for granted in tools_for_token(detected, token) {
            if !tools.iter().any(|have| have.name == granted.name) {
                tools.push(granted);
            }
        }
    }
    tools
}

/// The two class-sized allowances for one detected stack.
fn toolset(detected: &Detected) -> Toolset {
    let runners = runner_tools(detected);
    let mut gate = runners;
    gate.push(GIT);
    gate.push(LS);
    gate.push(RG);
    let mut work = gate.clone();
    work.push(MKDIR);
    Toolset { work, gate }
}

/// `` `Cargo.toml` `` — or `` `package.json` + `pnpm-lock.yaml` ``: the
/// evidence, quoted back so the operator can check the guess.
fn evidence(markers: &[&str]) -> String {
    markers
        .iter()
        .map(|m| format!("`{m}`"))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Every named marker is a file at the repository root.
fn present(repo: &Path, markers: &[&str]) -> bool {
    markers.iter().all(|m| repo.join(m).is_file())
}

/// Read the repository, decide nothing else. The orchestrator table has
/// the first say — a monorepo's build tool outranks any one package's
/// manifest — then the per-manifest table. `None` is the honest answer
/// when neither matched, and the charters say so.
fn detect(repo: &Path) -> Option<Detected> {
    orchestrator(repo).or_else(|| stack(repo))
}

/// The monorepo arm. Two axes crossed: the orchestrator says WHAT to run,
/// whichever lockfile is at the root says what runs it.
fn orchestrator(repo: &Path) -> Option<Detected> {
    let found = ORCHESTRATORS.iter().find(|o| present(repo, o.markers))?;
    let runner = RUNNERS
        .iter()
        .find(|(lockfile, _)| repo.join(lockfile).is_file());
    let prefix = runner.map_or("npx", |(_, runner)| *runner);
    let mut markers: Vec<&str> = found.markers.to_vec();
    markers.extend(runner.map(|(lockfile, _)| *lockfile));
    Some(Detected {
        name: found.name.to_string(),
        evidence: evidence(&markers),
        build: found.build.replace("{runner}", prefix),
        test: found.test.replace("{runner}", prefix),
        lint: found.lint.replace("{runner}", prefix),
        note: Some(found.note),
    })
}

/// The per-manifest arm. First entry whose every marker is a file at the
/// root wins, and the workspace table then says whether the wildcards it
/// named were spanning a whole workspace all along.
fn stack(repo: &Path) -> Option<Detected> {
    let found = STACKS.iter().find(|s| present(repo, s.markers))?;
    let note = WORKSPACES
        .iter()
        .filter(|w| w.stack == found.name)
        .find(|w| {
            std::fs::read_to_string(repo.join(w.file)).is_ok_and(|contents| {
                w.needle.is_empty() || contents.lines().any(|line| line.trim() == w.needle)
            })
        })
        .map(|w| w.note);
    Some(Detected {
        name: found.name.to_string(),
        evidence: evidence(found.markers),
        build: found.build.to_string(),
        test: found.test.to_string(),
        lint: found.lint.to_string(),
        note,
    })
}

/// The two commands a seat is asked to run, and the sentence that says
/// where they came from. Introspection guesses; a guess that names its
/// source is one an operator can correct, so both branches end by saying
/// the charter is ordinary text.
fn commands(chosen: Option<(&Detected, &str, &str)>, placeholders: [&str; 2]) -> String {
    match chosen {
        Some((detected, first, second)) => format!(
            "This repository reads as a {} project ({}), so use its own\n\
             tooling:\n\
             \n    {first}\n    {second}\n\n\
             {}\
             `brokkr init` chose those from the files at the repository root —\n\
             it ran nothing to find out. Correct them here if they are wrong.\n",
            detected.name,
            detected.evidence,
            detected
                .note
                .map_or(String::new(), |note| format!("{note}\n\n")),
        ),
        None => format!(
            "NO STACK WAS RECOGNIZED at the repository root: none of the\n\
             manifests or lockfiles `brokkr init` looks for were there, so the\n\
             commands below are GENERIC placeholders and are NOT this\n\
             project's. Fill them in before the first run:\n\
             \n    {}\n    {}\n\n\
             Until you do, find the project's own suite and run it yourself;\n\
             one honest run is the whole of the signal.\n",
            placeholders[0], placeholders[1]
        ),
    }
}

fn implementer(stack: Option<&Detected>) -> String {
    format!(
        "# Implementer seat — build it\n\n\
         Implement the framed task (see `.forge/tasks/`). Match the project's\n\
         idiom. Tests are part of the change.\n\n\
         {}\n\
         Build and test before declaring anything. Commit your work; never push.\n\n\
         Result: `complete` (implemented, tests green, committed) · `broken`\n\
         (could not get it working — name the specific gap in `notes`) ·\n\
         `blocked` (something outside your control — name it precisely). Never\n\
         report `complete` with failing tests or uncommitted changes.\n",
        commands(
            stack.map(|s| (s, s.build.as_str(), s.test.as_str())),
            [
                "<this project's build command>",
                "<this project's test command>"
            ],
        )
    )
}

fn verifier(stack: Option<&Detected>) -> String {
    format!(
        "# Verifier seat — prove it, fix nothing\n\n\
         Run the project's full test and lint suites from the repository root.\n\n\
         {}\n\
         You change no code, fix nothing, commit nothing: one honest run is the\n\
         signal. Result: `pass` (everything green; `notes` lists commands and\n\
         counts) or `fail` (`notes` quotes the failing output's decisive lines\n\
         exactly — never soften a failure).\n",
        commands(
            stack.map(|s| (s, s.test.as_str(), s.lint.as_str())),
            [
                "<this project's test command>",
                "<this project's lint command>"
            ],
        )
    )
}

/// The charters, three of them fixed and two of them written to the
/// stack that was found. Intake, review and ship name no build tooling:
/// framing a task, reading a diff and closing out read the same in every
/// repository, and a charter that pretended otherwise would be padding.
///
/// Each charter is the agent of the same name's charter text, so the five
/// land under `agents/charters/` — INSIDE the library root the compiler
/// resolves agent references against (decision 0016), where the
/// `contained` rule can prove them safe.
fn charters(stack: Option<&Detected>) -> [(&'static str, String); 5] {
    [
        ("implementer.md", implementer(stack)),
        ("verifier.md", verifier(stack)),
        (
            "intake.md",
            "# Intake seat — frame the task\n\nRead the feature description and the repository until you can state\nthe task precisely. Write a short framing (goal, files you expect to\nchange, the tests that must prove it, non-goals) to\n`.forge/tasks/<slug>.md` in the working directory — run-local evidence,\nnot committed. Result: `resolved`, with `notes` naming the framing file.\nYou never decide the run's fate; the policy table does.\n".to_string(),
        ),
        (
            "reviewer.md",
            "# Reviewer seat — adversarial review, security riding along\n\nReview everything changed since the run began (`git log`/`git diff`).\nDimensions: correctness, simplicity, and SECURITY (non-removable;\nseverity vocabulary `none|info|low|medium|high|critical`). You may\napply small safe fixes — commit them and set `fixes_applied: true`\n(the machine then re-verifies; that is correct).\n\nResult: `clean` with `inputs: {\"fixes_applied\": <bool>}` · `residual`\nwith `inputs: {\"max_residual_severity\": \"<severity>\",\n\"has_security_residual\": <bool>}` (list every finding in `notes`;\nnever understate severity — the table decides what ships) ·\n`security-hold` for any unresolved high/critical security finding.\n".to_string(),
        ),
        (
            "shipper.md",
            "# Shipper seat — close out, hand to the operator\n\nYou do NOT push, merge, or open PRs — the operator holds that\nauthority. Make no commits in this phase: the drift gate compares HEADs\nagainst review time.\n\nStep 2 (`context.last_decision.rule_id == \"SHIP-READY\"`): confirm the\ntree is still clean and HEAD unchanged, then report `shipped` with a\nclose-out summary for the operator.\n\nStep 1 (anything else): confirm the tree is clean, write the delivery\nledger to `.forge/ledger/<run-id>.md` (what shipped, commits, test\nevidence, residual debt, operator next steps — run-local, not\ncommitted), and report `ready`.\n".to_string(),
        ),
    ]
}

/// Which allowance the agent of one seat carries: `Work` for the seats
/// that produce (intake, implement), `Gate` for the seats that judge
/// (verify, review, ship). The division is decision 0021 ruling 1's, and
/// it is applied HERE to the tool grant each agent may express; the seat
/// itself carries the class in `bundle.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Allowance {
    Work,
    Gate,
}

/// One agent of the starter's roster: the name `bundle.json` seats it
/// under, the description and model chain its file carries, the 0006
/// bounds that adoption moved from the seats, and the allowance class of
/// the seat it backs.
struct SeatAgent {
    name: &'static str,
    description: &'static str,
    models: &'static [&'static str],
    max_attempts: u64,
    timeout_seconds: u64,
    allowance: Allowance,
}

/// The five agents the starter seats, in roster order — the same five
/// names `bundle.json` references, with the same bounds the inline
/// starter declared on its seats.
const SEAT_AGENTS: &[SeatAgent] = &[
    SeatAgent {
        name: "intake",
        description: "Frames a raw request into a recorded, actionable task before any code is written.",
        models: &["sonnet", "opus"],
        max_attempts: 2,
        timeout_seconds: 1800,
        allowance: Allowance::Work,
    },
    SeatAgent {
        name: "implementer",
        description: "Builds the framed task to the repository's conventions and commits the work with its tests.",
        models: &["opus", "sonnet"],
        max_attempts: 2,
        timeout_seconds: 5400,
        allowance: Allowance::Work,
    },
    SeatAgent {
        name: "verifier",
        description: "Runs the suites and gates and reports pass or fail on evidence, never on intent.",
        models: &["sonnet", "opus"],
        max_attempts: 2,
        timeout_seconds: 3600,
        allowance: Allowance::Gate,
    },
    SeatAgent {
        name: "reviewer",
        description: "The single-seat reviewer: correctness and security in one pass, for recipes without a review panel.",
        models: &["opus", "sonnet"],
        max_attempts: 2,
        timeout_seconds: 3600,
        allowance: Allowance::Gate,
    },
    SeatAgent {
        name: "shipper",
        description: "Closes a delivery out: ledger, gates, and the report the operator reads before merging.",
        models: &["sonnet", "opus"],
        max_attempts: 2,
        timeout_seconds: 1800,
        allowance: Allowance::Gate,
    },
];

/// One agent file of the starter. `allowance: None` — the no-stack
/// case, where the names map is empty — writes NO `tools` key at all:
/// the loader reads an absent `tools` as "no restriction", the only
/// reading an empty map can honestly serve, and the README says why.
fn agent_json(agent: &SeatAgent, allowance: Option<&[Tool]>) -> String {
    let mut definition = json!({
        "description": agent.description,
        "charter": format!("charters/{}.md", agent.name),
        "models": agent.models,
        "limits": {
            "max_attempts": agent.max_attempts,
            "timeout_seconds": agent.timeout_seconds,
        },
    });
    if let Some(tools) = allowance {
        definition["tools"] = json!({
            "allow": tools.iter().map(|tool| tool.name).collect::<Vec<_>>(),
            "mcp": [],
        });
    }
    format!(
        "{}\n",
        serde_json::to_string_pretty(&definition).expect("an agent definition serializes")
    )
}

/// The scaffold's claude adapter: decision 0021's trust declaration
/// (ruling 2's trusted tier — the starter's gate seats compile against
/// it — and ruling 4's absent binding grant) plus the per-stack tool
/// map. `names` is the union of every allowance the scaffold wrote: a
/// name any agent's `tools.allow` lists must be expressible here, or the
/// scaffold's own compile refuses — an empty map is exactly the silence
/// that left a fresh scaffold's seats without a grant.
fn adapter_json(toolset: Option<&Toolset>) -> String {
    let mut names = Map::new();
    if let Some(toolset) = toolset {
        for tool in &toolset.work {
            names.insert(tool.name.to_string(), json!(tool.permission));
        }
    }
    let adapter = json!({
        "provider": "claude",
        "trust_tier": "trusted",
        "binding_grant": false,
        "binary": "claude",
        "driver": ["{brokkr}", "driver", "claude", "--", "--permission-mode", "acceptEdits"],
        "models": {
            "fable": "claude-fable-5",
            "opus": "claude-opus-5",
            "sonnet": "claude-sonnet-5",
            "haiku": "claude-haiku-4-5-20251001"
        },
        "model_flag": "--model",
        "tool_permissions": {"flag": "--allowedTools", "separator": ",", "names": names},
        "mcp": {"flag": "--mcp-config", "servers": {}}
    });
    format!(
        "{}\n",
        serde_json::to_string_pretty(&adapter).expect("the claude adapter serializes")
    )
}

/// The scaffold's README. For a recognized stack it names the stack and
/// the two class-sized grants, and where each half of the grant lives;
/// for a repository no row recognizes it says so in those words — the
/// names map was scaffolded EMPTY because `brokkr init` does not invent
/// tool names, and a tool it cannot back with a `Bash(...)` expression
/// is one no seat can be granted.
fn readme(detected: Option<&Detected>) -> String {
    match detected {
        Some(detected) => {
            let toolset = toolset(detected);
            let work = rendered(&toolset.work);
            let gate = rendered(&toolset.gate);
            format!(
                "# starter — a brokkr workspace\n\n\
                 This directory was scaffolded by `brokkr init` from inside a\n\
                 repository that reads as a {name} project ({evidence}).\n\
                 Everything here is ordinary text: read it, edit it, commit it.\n\n\
                 ## The seats and their tools\n\n\
                 `bundle.json` seats one agent per phase. Each agent lives in\n\
                 `agents/`, its charter in `agents/charters/`, and its tool\n\
                 allowance in the agent's `tools.allow`:\n\n\
                 - the work-class seats (`intake`, `implement`) may run the full\n\
                   set — {work} — so a seat may run exactly the commands its\n\
                   charter names and nothing broader;\n\
                 - the gate-class seats (`verify`, `review`, `ship`) may run the\n\
                   read-only subset — {gate} — and never the write tools,\n\
                   because nobody stands behind the judges.\n\n\
                 The allowances are ONE grant with\n\
                 `adapters/claude.json` → `tool_permissions.names`, which maps\n\
                 every name to the `Bash(...)` expression the claude CLI\n\
                 understands. An allowance whose name the map cannot express\n\
                 refuses this scaffold's own compile, so when you edit one, edit\n\
                 both.\n\n\
                 ## Run it\n\n\
                 From inside this directory, drive the machine with\n\
                 `brokkr run --bundle . --repo <this repository> --feature \"…\"`\n\
                 (the quickstart's four-step spine); `--repo` is the repository\n\
                 `init` was run in — the charters name its own commands, and the\n\
                 allowance above is sized to them.\n",
                name = detected.name,
                evidence = detected.evidence,
                work = work,
                gate = gate,
            )
        }
        None => "# starter — a brokkr workspace\n\n\
             This directory was scaffolded by `brokkr init`. Everything here is\n\
             ordinary text: read it, edit it, commit it.\n\n\
             NO STACK WAS RECOGNIZED at the repository `init` was run in: none\n\
             of the manifests or lockfiles it looks for were at that root, so\n\
             the implement and verify charters (`agents/charters/`) carry\n\
             GENERIC placeholders rather than commands that would be guesses.\n\
             Fill them in before the first run.\n\n\
             The tool map was scaffolded EMPTY for the same reason:\n\
             `adapters/claude.json` → `tool_permissions.names` names nothing,\n\
             and each agent in `agents/` declares no `tools.allow`. `brokkr\n\
             init` does not invent tool names — a tool it cannot back with a\n\
             `Bash(...)` expression is one no seat can be granted. Before a\n\
             headless run, find this repository's own build, test and lint\n\
             commands and grant them by name: add each binary to the adapter's\n\
             `tool_permissions.names` as `Bash(<bin>:*)`, then list the names in\n\
             each agent's `tools.allow` — the work-class seats (intake,\n\
             implement) get the full set, the gate-class seats (verify, review,\n\
             ship) get the read-only subset (git, ls, rg and the test runner).\n"
            .to_string(),
    }
}

/// The `Bash(...)` expressions of one allowance, comma-joined for prose.
fn rendered(tools: &[Tool]) -> String {
    tools
        .iter()
        .map(|tool| format!("`{}`", tool.permission))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Scaffold into `dir`, having read `repo` — the repository being
/// delivered, which is the workspace `brokkr` was invoked from and not
/// the directory the recipe lands in. Two paths because they are two
/// things: `brokkr init my-bundle` is run from inside the project whose
/// seats the charters describe, and `my-bundle` is only where the text
/// is written.
pub fn init(dir: &Path, repo: &Path) -> Result<String> {
    if dir.join("bundle.json").exists() {
        bail!(
            "{} already contains a bundle.json; refusing to overwrite",
            dir.display()
        );
    }
    // The scaffold writes a trust declaration too, and that is WORKSPACE
    // data rather than this bundle's own text: an `adapters/` already
    // standing here is an operator's ruling (decision 0021 ruling 3), and
    // a scaffolder that wrote over it would silently re-promote a tier the
    // operator had demoted — the one move this vocabulary exists to make
    // impossible by accident. Refused on the same terms, and before
    // anything is written.
    let declaration = dir.join(DEFAULT_ADAPTERS_DIR).join("claude.json");
    if declaration.exists() {
        bail!(
            "{} already declares a trust tier; refusing to overwrite — a tier \
             is an operator's ruling, not a scaffold's",
            declaration.display()
        );
    }
    let detected = detect(repo);
    let toolset = detected.as_ref().map(toolset);
    let adapters = dir.join(DEFAULT_ADAPTERS_DIR);
    let agents = dir.join(DEFAULT_AGENTS_DIR);
    std::fs::create_dir_all(&adapters)?;
    std::fs::create_dir_all(agents.join("charters"))?;
    std::fs::write(dir.join("policy.json"), POLICY)?;
    std::fs::write(dir.join("bundle.json"), BUNDLE)?;
    std::fs::write(dir.join("README.md"), readme(detected.as_ref()))?;
    let adapter = adapter_json(toolset.as_ref());
    std::fs::write(adapters.join("claude.json"), adapter)?;
    for agent in SEAT_AGENTS {
        let allowance = toolset.as_ref().map(|set| match agent.allowance {
            Allowance::Work => set.work.as_slice(),
            Allowance::Gate => set.gate.as_slice(),
        });
        let definition = agent_json(agent, allowance);
        let path = agents.join(format!("{}.json", agent.name));
        std::fs::write(path, definition)?;
    }
    for (name, content) in charters(detected.as_ref()) {
        std::fs::write(agents.join("charters").join(name), content)?;
    }
    // init proves its own output: the scaffold must compile under the
    // constitutional lint before we call it a bundle — and, since the
    // seats reference the scaffold's own agents, that compile resolves
    // every allowance through the scaffold's own `adapters/`, so a tool
    // name this file got wrong refuses right here. Against the SCAFFOLD's
    // own roots, not the process's: what init proves must be a property
    // of what it wrote, and a starter that compiled only because the
    // caller happened to stand in a tree with an `adapters/` or `agents/`
    // would be a proof about the caller.
    let bundle = Bundle::compile_with(dir, &agents, &adapters)
        .context("scaffolded bundle failed to compile")?;
    Ok(bundle.manifest_digest())
}

#[cfg(test)]
mod tests;
