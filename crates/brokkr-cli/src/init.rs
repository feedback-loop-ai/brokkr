//! `brokkr init <dir>` — scaffold a minimal reviewable bundle and prove it
//! compiles. The template carries the tightened ship taxonomy (`ready` →
//! `shipped` as the sole entry into `done`), the protected review phase,
//! one agent-defined seat per phase, per-seat limits (decision 0006),
//! the work/gate division of decision 0021 ruling 1, and the bundled
//! headless Claude Code driver. Everything written is ordinary text meant
//! to be reviewed and edited in git.
//!
//! The scaffold is a WORKSPACE, not only a bundle: it carries its own
//! `agents/` and `adapters/` trees (proposed decision 0030), because an
//! agent's least-privilege tool grant and the tier that lets a gate seat
//! judge are workspace data. A starter whose seats could neither run
//! their charter nor show their authority would teach the wrong lesson
//! on day one.
//! The operator's trust declarations are theirs to edit — which is why
//! they are scaffolded as a file in their tree rather than compiled into
//! this binary — and `brokkr` is run from inside the scaffold, where its
//! `adapters/` is the workspace's.
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

use std::path::Path;

use anyhow::{bail, Context, Result};
use brokkr_runtime::bundle::{DEFAULT_ADAPTERS_DIR, DEFAULT_AGENTS_DIR};
use brokkr_runtime::Bundle;
use serde_json::{json, Map, Value};

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
/// charters call it by, the runner Claude may invoke, and the three
/// commands a seat would actually run there. Data, walked in order — the
/// only thing the code matches on is whether the named files are present.
struct Stack {
    /// The stack's own vocabulary, not a label invented here.
    name: &'static str,
    /// Every marker must be present at the repository root. No recursion:
    /// a scaffold is a starting point, not a monorepo analyzer.
    markers: &'static [&'static str],
    /// The executable at the front of all three commands. This lives
    /// beside the commands so detection and permission scaffolding cannot
    /// acquire separate stack vocabularies.
    tool: &'static str,
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
        tool: "cargo",
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
        tool: "bun",
        build: "bun install --frozen-lockfile",
        test: "bun run test",
        lint: "bun run typecheck",
    },
    Stack {
        name: "node/pnpm",
        markers: &["package.json", "pnpm-lock.yaml"],
        tool: "pnpm",
        build: "pnpm build",
        test: "pnpm test",
        lint: "pnpm lint",
    },
    Stack {
        name: "node/yarn",
        markers: &["package.json", "yarn.lock"],
        tool: "yarn",
        build: "yarn build",
        test: "yarn test",
        lint: "yarn lint",
    },
    Stack {
        name: "node/npm",
        markers: &["package.json"],
        tool: "npm",
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
        tool: "uv",
        build: "uv sync",
        test: "uv run pytest",
        lint: "uv run ruff check .",
    },
    Stack {
        name: "python",
        markers: &["pyproject.toml"],
        tool: "python",
        build: "python -m build",
        test: "python -m pytest",
        lint: "python -m ruff check .",
    },
    Stack {
        name: "go",
        markers: &["go.mod"],
        tool: "go",
        build: "go build ./...",
        test: "go test ./...",
        lint: "go vet ./...",
    },
    Stack {
        name: "make",
        markers: &["Makefile"],
        tool: "make",
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
const RUNNERS: &[(&str, &str, &str)] = &[
    ("bun.lock", "bunx", "bunx"),
    ("pnpm-lock.yaml", "pnpm exec", "pnpm"),
    ("yarn.lock", "yarn exec", "yarn"),
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
    tool: String,
    build: String,
    test: String,
    lint: String,
    note: Option<&'static str>,
}

/// The two permission envelopes scaffolded into the agent library
/// (proposed decision 0030). A work seat gets the detected runner plus
/// the repository-local hands its charters need; a gate gets the
/// read-only repository hands and the runner needed to execute the
/// stack's proof commands. An unknown stack gets neither: inventing a
/// runner would be broader than admitting that init does not know.
struct Grants {
    adapter: Vec<String>,
    work: Vec<String>,
    gate: Vec<String>,
}

fn grants(stack: Option<&Detected>) -> Grants {
    let Some(stack) = stack else {
        return Grants {
            adapter: Vec::new(),
            work: Vec::new(),
            gate: Vec::new(),
        };
    };
    let work = [stack.tool.as_str(), "git", "ls", "rg", "mkdir"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let gate = ["git", "ls", "rg", stack.tool.as_str()]
        .into_iter()
        .map(str::to_string)
        .collect();
    Grants {
        adapter: work.clone(),
        work,
        gate,
    }
}

/// The scaffold's own trust and capability declaration (decision 0021
/// rulings 2 and 4), for the one driver it seats. `trusted`, because the
/// starter's verify, review and ship seats are the gate roster of ruling
/// 1 and would otherwise refuse to compile — the operator inherits the
/// incumbent's journaled record and may demote it by editing this file.
/// No binding grant: no seat here declares `secrets`, and a grant nothing
/// needs is one more thing to take away later.
fn adapter(grants: &Grants) -> Value {
    let names = grants
        .adapter
        .iter()
        .map(|tool| (tool.clone(), json!(format!("Bash({tool}:*)"))))
        .collect::<Map<_, _>>();
    json!({
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
    })
}

/// One definition in the scaffold-local agent library. Class remains a
/// property of the invocation site in `bundle.json`; `gate` selects the
/// narrower tool grant written into this definition.
struct AgentTemplate {
    name: &'static str,
    description: &'static str,
    charter: &'static str,
    models: &'static [&'static str],
    max_attempts: u64,
    timeout_seconds: u64,
    gate: bool,
}

const AGENTS: &[AgentTemplate] = &[
    AgentTemplate {
        name: "intake",
        description:
            "Frames a raw request into a recorded, actionable task before code is written.",
        charter: "intake.md",
        models: &["sonnet", "opus"],
        max_attempts: 2,
        timeout_seconds: 1800,
        gate: false,
    },
    AgentTemplate {
        name: "implementer",
        description:
            "Builds the framed task to the repository's conventions and commits it with tests.",
        charter: "implementer.md",
        models: &["opus", "sonnet"],
        max_attempts: 2,
        timeout_seconds: 5400,
        gate: false,
    },
    AgentTemplate {
        name: "verifier",
        description:
            "Runs the repository's proof commands and reports pass or fail without fixing code.",
        charter: "verifier.md",
        models: &["sonnet", "opus"],
        max_attempts: 2,
        timeout_seconds: 3600,
        gate: true,
    },
    AgentTemplate {
        name: "reviewer",
        description: "Reviews correctness and security and records every residual finding.",
        charter: "reviewer.md",
        models: &["opus", "sonnet"],
        max_attempts: 2,
        timeout_seconds: 3600,
        gate: true,
    },
    AgentTemplate {
        name: "shipper",
        description: "Closes delivery out for the operator without pushing or merging.",
        charter: "shipper.md",
        models: &["sonnet", "opus"],
        max_attempts: 2,
        timeout_seconds: 1800,
        gate: true,
    },
];

fn agent(template: &AgentTemplate, grants: &Grants) -> Value {
    let allow = if template.gate {
        &grants.gate
    } else {
        &grants.work
    };
    let mut tools = Map::new();
    if !allow.is_empty() {
        tools.insert("allow".to_string(), json!(allow));
    }
    tools.insert("mcp".to_string(), json!([]));
    json!({
      "description": template.description,
      "charter": format!("charters/{}", template.charter),
      "models": template.models,
      "tools": tools,
      "limits": {
        "max_attempts": template.max_attempts,
        "timeout_seconds": template.timeout_seconds
      }
    })
}

fn readme(stack: Option<&Detected>, grants: &Grants) -> String {
    match stack {
        Some(stack) => format!(
            "# Brokkr starter\n\n\
             Detected `{}` from {}. The Claude adapter grants only the command\n\
             runner and repository-local tools the scaffolded seats name.\n\n\
             - Work agents: `{}`\n\
             - Gate agents: `{}`\n\n\
             Edit the adapter and each agent's `tools.allow` together when you\n\
             correct a charter command.\n",
            stack.name,
            stack.evidence,
            grants.work.join("`, `"),
            grants.gate.join("`, `"),
        ),
        None => "# Brokkr starter\n\n\
                 NO TOOL GRANTS WERE SCAFFOLDED. No stack was recognized, so\n\
                 `adapters/claude.json` has an empty `tool_permissions.names` map\n\
                 rather than invented command names. The agents therefore carry\n\
                 no `tools.allow` narrowing. Fill in the real charter commands,\n\
                 adapter mappings, and agent grants together before the first run.\n"
            .to_string(),
    }
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
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
        .find(|(lockfile, _, _)| repo.join(lockfile).is_file());
    let prefix = runner.map_or("npx", |(_, runner, _)| *runner);
    let tool = runner.map_or("npx", |(_, _, tool)| *tool);
    let mut markers: Vec<&str> = found.markers.to_vec();
    markers.extend(runner.map(|(lockfile, _, _)| *lockfile));
    Some(Detected {
        name: found.name.to_string(),
        evidence: evidence(&markers),
        tool: tool.to_string(),
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
        tool: found.tool.to_string(),
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
    // The scaffold now writes a trust declaration too, and that is
    // WORKSPACE data rather than this bundle's own text: an `adapters/`
    // already standing here is an operator's ruling (decision 0021
    // ruling 3), and a scaffolder that wrote over it would silently
    // re-promote a tier the operator had demoted — the one move this
    // vocabulary exists to make impossible by accident. Refused on the
    // same terms, and before anything is written.
    let declaration = dir.join(DEFAULT_ADAPTERS_DIR).join("claude.json");
    if declaration.exists() {
        bail!(
            "{} already declares a trust tier; refusing to overwrite — a tier \
             is an operator's ruling, not a scaffold's",
            declaration.display()
        );
    }

    let detected = detect(repo);
    let grants = grants(detected.as_ref());
    let library = dir.join(DEFAULT_AGENTS_DIR);
    let charter_root = library.join("charters");
    let occupied = std::iter::once(dir.join("README.md"))
        .chain(AGENTS.iter().flat_map(|template| {
            [
                library.join(format!("{}.json", template.name)),
                charter_root.join(template.charter),
            ]
        }))
        .find(|path| path.exists());
    if let Some(path) = occupied {
        bail!(
            "{} is already present; refusing to overwrite scaffold-owned text",
            path.display()
        );
    }

    std::fs::create_dir_all(&charter_root)?;
    std::fs::create_dir_all(dir.join(DEFAULT_ADAPTERS_DIR))?;
    std::fs::write(dir.join("README.md"), readme(detected.as_ref(), &grants))?;
    std::fs::write(dir.join("policy.json"), POLICY)?;
    std::fs::write(dir.join("bundle.json"), BUNDLE)?;
    let adapter_path = dir.join(DEFAULT_ADAPTERS_DIR).join("claude.json");
    write_json(&adapter_path, &adapter(&grants))?;
    for (name, content) in charters(detected.as_ref()) {
        std::fs::write(charter_root.join(name), content)?;
    }
    for template in AGENTS {
        let definition = library.join(format!("{}.json", template.name));
        write_json(&definition, &agent(template, &grants))?;
    }
    // init proves its own output: the scaffold must compile under the
    // constitutional lint before we call it a bundle. Against the
    // SCAFFOLD's own roots, not the process's: what init proves must be
    // a property of what it wrote, and a starter that compiled only
    // because the caller happened to stand in a tree with `agents/` or
    // `adapters/` would be a proof about the caller.
    let bundle = Bundle::compile_with(
        dir,
        &dir.join(DEFAULT_AGENTS_DIR),
        &dir.join(DEFAULT_ADAPTERS_DIR),
    )
    .context("scaffolded bundle failed to compile")?;
    Ok(bundle.manifest_digest())
}
