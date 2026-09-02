//! `brokkr init <dir>` — scaffold a minimal reviewable bundle and prove it
//! compiles. The template carries the tightened ship taxonomy (`ready` →
//! `shipped` as the sole entry into `done`), the protected review phase,
//! one agent-defined seat per phase, the work/gate division of decision
//! 0021 ruling 1, and the bundled headless Claude Code driver. Everything
//! written is ordinary text meant to be reviewed and edited in git.
//!
//! The scaffold is a WORKSPACE, not only a bundle: it carries its own
//! `adapters/` and `agents/` trees, because since decision 0021 the tier
//! that lets a gate seat judge is adapter data, and since decision 0016
//! what a seat may RUN is the agent data its `tools.allow` names,
//! expressed through the adapter's `tool_permissions.names`. A starter
//! whose review seat judged on nobody's authority would teach the wrong
//! lesson on day one; a starter whose seats could not be granted the
//! stack's own build and test commands would be a starter whose first run
//! stopped before a line was written — the looper adoption run (issue
//! #211, run 1) was IMPL-BLOCKED for exactly that. The operator's trust
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
//! The same stack decides the scaffold's TOOL GRANTS. A headless seat
//! runs under `--allowedTools`: a claude session that may run anything
//! stops at every shell prompt it is not allowed to answer. So the tools
//! the stack's own commands need — the binary each command leads with —
//! are written into the scaffolded `adapters/claude.json`
//! `tool_permissions.names` as `Bash(<bin>:*)` entries and granted, by
//! name, in the scaffolded agents' `tools.allow` lists: an allowance the
//! adapter cannot express is a compile refusal, so the two files are ONE
//! grant, not two. The split is decision 0021 ruling 1's: the WORK-class
//! seats (intake, implement) may run the full set — the stack's runners
//! plus `git`, `ls`, `rg` and `mkdir` — so a seat may run exactly the
//! commands its charter names and nothing broader; the GATE-class seats
//! (verify, review, ship) may run the test runner's tools plus the read
//! trio, and never `mkdir`. The grant is per BINARY, not per subcommand:
//! `Bash(cargo:*)` answers to `cargo build` as readily as to `cargo
//! test`, so it is each gate's charter — prove it, fix nothing — and not
//! the grant that keeps a gate from building, and the README says so
//! rather than promising a boundary the glob cannot draw. A stack no row
//! of the two tables recognizes earns NO tool name: the map is written
//! EMPTY, no agent declares a `tools` restriction, and the scaffold's
//! README says so in those words — a tool name is a permission, and one
//! guessed is one granted.

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

/// Every seat names an agent and nothing about what the agent IS: an
/// agent reference is total (the compiler refuses `role`, `limits` or a
/// `driver.command` beside it), so the seat carries only what is the
/// seat's own — its class, which is the seat's authority and never the
/// agent's (decision 0021 ruling 1), and its result vocabulary. The
/// charter, the model chain, the 0006 bounds and the tool grant are the
/// agent's, in `agents/`, where `brokkr agents show` can read them back.
/// This file is byte-identical for every repository: what varies by
/// stack lives in the adapter's tool map and the agents' allowances, not
/// here.
const BUNDLE: &str = r#"{
  "name": "starter",
  "policy": "policy.json",
  "protected_phase": "review",
  "seats": {
    "intake": {
      "agent": "intake",
      "class": "work",
      "results": ["resolved"]
    },
    "implement": {
      "agent": "implementer",
      "class": "work",
      "results": ["complete", "broken", "blocked"]
    },
    "verify": {
      "agent": "verifier",
      "class": "gate",
      "results": ["pass", "fail"]
    },
    "review": {
      "agent": "reviewer",
      "class": "gate",
      "results": ["clean", "residual", "security-hold"]
    },
    "ship": {
      "agent": "shipper",
      "class": "gate",
      "results": ["ready", "shipped"]
    }
  }
}
"#;

/// One scaffolded agent's fixed identity: the name `bundle.json` seats it
/// under (which is also the file's stem), the class of the seat it backs,
/// the description and model chain its definition carries, and the 0006
/// bounds the starter used to declare inline. The model chains are the
/// repository's own library's: work leads with the stronger model where
/// the ship does, review reads adversarially, and the gate seats fall
/// back rather than pretend.
struct AgentSpec {
    agent: &'static str,
    class: Class,
    description: &'static str,
    models: [&'static str; 2],
    max_attempts: u64,
    timeout_seconds: u64,
}

/// The two classes of decision 0021 ruling 1, as the scaffold seats them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Work,
    Gate,
}

/// The scaffold's roster, in the order `bundle.json` seats them. The
/// class written here and the class each seat declares there must agree;
/// the scaffold's tests cross-check the two.
const SEATS: &[AgentSpec] = &[
    AgentSpec {
        agent: "intake",
        class: Class::Work,
        description: "Frames a raw request into a recorded, actionable task before any code is written.",
        models: ["sonnet", "opus"],
        max_attempts: 2,
        timeout_seconds: 1800,
    },
    AgentSpec {
        agent: "implementer",
        class: Class::Work,
        description: "Builds the framed task to the repository's conventions and commits the work with its tests.",
        models: ["opus", "sonnet"],
        max_attempts: 2,
        timeout_seconds: 5400,
    },
    AgentSpec {
        agent: "verifier",
        class: Class::Gate,
        description: "Runs the suites and gates and reports pass or fail on evidence, never on intent.",
        models: ["sonnet", "opus"],
        max_attempts: 2,
        timeout_seconds: 3600,
    },
    AgentSpec {
        agent: "reviewer",
        class: Class::Gate,
        description: "The single-seat reviewer: correctness and security in one pass, for recipes without a review panel.",
        models: ["opus", "sonnet"],
        max_attempts: 2,
        timeout_seconds: 3600,
    },
    AgentSpec {
        agent: "shipper",
        class: Class::Gate,
        description: "Closes a delivery out: ledger, gates, and the report the operator reads before merging.",
        models: ["sonnet", "opus"],
        max_attempts: 2,
        timeout_seconds: 1800,
    },
];

/// One granted tool: the name an agent's `tools.allow` lists and the
/// adapter's `tool_permissions.names` keys it by, and the `Bash(...)`
/// expression that name means on a claude command line. Data, never
/// derived: a scaffold invents no tool name, because a name the adapter
/// cannot back with an expression is one no seat can be granted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
/// The one WRITE tool: `mkdir`, held by the work seats that write
/// `.forge/` evidence. A gate seat judges; nothing a judge does needs to
/// create a directory.
const MKDIR: Tool = Tool {
    name: "mkdir",
    permission: "Bash(mkdir:*)",
};

/// The stack's own runners, one const per leading binary the two
/// detection tables can name. Each is the `Bash(<bin>:*)` expression the
/// shipped adapter vocabulary already carries for the same binary.
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
/// pytest is not a runner any command of the plain-python row leads
/// with — the suite runs as `python3 -m pytest` — but the venv's own
/// `pytest` binary is the honest spelling of "run the suite" in a fresh
/// project, and the shipped adapters grant it the same narrow way.
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

/// The tools one leading binary needs. Every first word the two tables
/// can put on a command has an arm here; the wildcard refuses rather
/// than invents, because the tables are closed data in this file and an
/// unbacked runner is a coding error, not a repository to scaffold. One
/// word carries two tools: the plain-python row's `python3` — its
/// interpreter, and the venv's suite binary beside it.
fn tools_for(token: &str) -> Vec<Tool> {
    match token {
        "cargo" => vec![CARGO],
        "bun" => vec![BUN],
        "bunx" => vec![BUNX],
        "pnpm" => vec![PNPM],
        "yarn" => vec![YARN],
        "npm" => vec![NPM],
        "npx" => vec![NPX],
        "uv" => vec![UV],
        "python3" => vec![PYTHON3, PYTEST],
        "go" => vec![GO],
        "make" => vec![MAKE],
        other => panic!(
            "the '{other}' runner of a recognized stack has no tool grant; \
             add an arm beside the table row that names it"
        ),
    }
}

/// The leading word of one command — the binary that actually runs it.
fn leading_word(command: &str) -> &str {
    command.split_whitespace().next().unwrap_or(command)
}

/// The tools one command needs: those of its leading binary.
fn command_tools(command: &str) -> Vec<Tool> {
    tools_for(leading_word(command))
}

/// The tools a detected stack's commands run through: the leading binary
/// of each of its build, test and lint commands, deduplicated and in that
/// order. A scaffold grants exactly these runners — never a binary the
/// charters do not name.
fn runner_tools(detected: &Detected) -> Vec<Tool> {
    let mut tools: Vec<Tool> = Vec::new();
    for command in [&detected.build, &detected.test, &detected.lint] {
        for granted in command_tools(command) {
            if !tools.contains(&granted) {
                tools.push(granted);
            }
        }
    }
    tools
}

/// The tools one detected stack's seats may run, split by decision 0021
/// ruling 1's two classes:
///
/// - `work` — the whole set: every runner above plus `git`, `ls`, `rg`
///   and `mkdir`, so a work seat may run exactly the commands its
///   charter names and nothing broader;
/// - `gate` — the read-only subset: the test command's tools (which, for
///   every row in the tables today, are the same binary the build and
///   lint lines also lead with — the grant is per binary, and the README
///   says so) plus `git`, `ls` and `rg`, and never `mkdir`.
///
/// Both empty when no stack was recognized: the map is then empty too,
/// and an agent that named a key the map lacks would refuse to compile —
/// the right refusal, but not one a scaffold should hand over.
struct Grants {
    work: Vec<Tool>,
    gate: Vec<Tool>,
}

fn grants(detected: Option<&Detected>) -> Grants {
    let Some(detected) = detected else {
        return Grants {
            work: Vec::new(),
            gate: Vec::new(),
        };
    };
    let mut work = runner_tools(detected);
    let mut gate = command_tools(&detected.test);
    for read in [GIT, LS, RG] {
        work.push(read);
        gate.push(read);
    }
    work.push(MKDIR);
    Grants { work, gate }
}

/// The scaffold's claude adapter: decision 0021's trust declaration
/// (ruling 2's trusted tier — the starter's gate seats compile against
/// it — and ruling 4's absent binding grant) plus the tool map. `names`
/// is the union of every allowance the scaffold wrote — the work set,
/// which carries the gate set inside it — because a name any agent's
/// `tools.allow` lists must be expressible here or the scaffold's own
/// compile refuses. Where nothing was recognized the map stays EMPTY,
/// and the README carries the sentence that says which of the two it is.
fn adapter_json(grants: &Grants) -> String {
    let mut names = Map::new();
    for tool in &grants.work {
        names.insert(tool.name.to_string(), json!(tool.permission));
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

/// One agent definition, in the repository's own library format. The
/// `tools` key is omitted — not written empty — when there is nothing to
/// grant: the loader rejects an empty `allow` as ambiguous between "no
/// restriction" and "restrict to nothing", and the README says which of
/// the two an absent key means.
fn agent_json(spec: &AgentSpec, allowance: Option<&[Tool]>) -> String {
    let mut definition = json!({
        "description": spec.description,
        "charter": format!("charters/{}.md", spec.agent),
        "models": spec.models,
        "limits": {
            "max_attempts": spec.max_attempts,
            "timeout_seconds": spec.timeout_seconds,
        },
    });
    if let Some(allowance) = allowance {
        definition["tools"] = json!({
            "allow": allowance.iter().map(|tool| tool.name).collect::<Vec<_>>(),
            "mcp": [],
        });
    }
    format!(
        "{}\n",
        serde_json::to_string_pretty(&definition).expect("an agent definition serializes")
    )
}

/// The allowance one seat's agent is written with: the whole set for the
/// work-class seats, the read-only subset for the gate-class seats —
/// the class the seat declares in `bundle.json`, applied here to the
/// grant the agent may express.
fn allowance<'a>(spec: &AgentSpec, grants: &'a Grants) -> Option<&'a [Tool]> {
    match (spec.class, grants.work.is_empty()) {
        (Class::Work, false) => Some(&grants.work),
        (Class::Gate, false) => Some(&grants.gate),
        // No stack was recognized: no tool was granted, and an agent must
        // not name one — omit the restriction and let the README say why.
        (_, true) => None,
    }
}

/// The scaffold's README: what was written, where each half of the grant
/// lives, and — the part an operator must be able to check — which tools
/// the seats were granted and why. Where no stack was recognized it says
/// the map is EMPTY, in those words, so an empty grant cannot be mistaken
/// for a considered one.
fn readme(detected: Option<&Detected>) -> String {
    match detected {
        Some(detected) => {
            let grants = grants(Some(detected));
            let rendered = |tools: &[Tool]| {
                tools
                    .iter()
                    .map(|tool| format!("{} → {}", tool.name, tool.permission))
                    .collect::<Vec<_>>()
                    .join("\n    ")
            };
            format!(
                "# starter — scaffolded by `brokkr init`\n\n\
                 This directory was scaffolded from inside a repository that reads\n\
                 as a {name} project ({evidence}). Everything here is ordinary text:\n\
                 read it, edit it, commit it.\n\n\
                 ## What is here\n\n\
                 - `bundle.json` — five seats, each naming an agent, with the class\n\
                   the seat declares (decision 0021 ruling 1: work or gate) and its\n\
                   results. This file does not vary by stack.\n\
                 - `policy.json` — the phase table; `review` is the protected phase.\n\
                 - `adapters/claude.json` — the one driver: its trust tier, and the\n\
                   tool map (`tool_permissions.names`) the allowances below are\n\
                   expressed through.\n\
                 - `agents/*.json` — one agent per seat: charter, model chain,\n\
                   tool allowance, limits. `brokkr agents show <name>` reads one back.\n\
                 - `agents/charters/*.md` — the charters; the implementer's and the\n\
                   verifier's name this repository's own commands.\n\n\
                 ## Tool grants\n\n\
                 `adapters/claude.json` maps each tool the seats are granted below\n\
                 to the `Bash(...)` expression the claude CLI reads — the stack's\n\
                 own runners, and nothing broader:\n\
                 \n    {work_rendered}\n\n\
                 Work-class seats (intake, implement) are granted the whole set, so\n\
                 a seat may run exactly the commands its charter names:\n\
                 {work_list}.\n\n\
                 Gate-class seats (verify, review, ship) are granted the read-only\n\
                 subset — the test runner's tools, and the tools that read — and\n\
                 never `mkdir`: {gate_list}.\n\n\
                 The grant is per BINARY, not per subcommand: the test runner's\n\
                 binary also answers to its build and install subcommands, so it is\n\
                 each gate's charter (prove it, fix nothing) and not the grant that\n\
                 keeps a gate from building.\n\n\
                 An allowance is ONE grant with the adapter's `tool_permissions.names`:\n\
                 an allowance whose name the map cannot express refuses this\n\
                 scaffold's own compile, so when you edit one, edit both.\n\n\
                 `brokkr init` chose all of this from the files at the repository\n\
                 root — it ran nothing to find out. Correct it here if it is wrong.\n",
                name = detected.name,
                evidence = detected.evidence,
                work_rendered = rendered(&grants.work),
                work_list = grants
                    .work
                    .iter()
                    .map(|tool| tool.name)
                    .collect::<Vec<_>>()
                    .join(", "),
                gate_list = grants
                    .gate
                    .iter()
                    .map(|tool| tool.name)
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        }
        None => "# starter — scaffolded by `brokkr init`\n\n\
                 NO STACK WAS RECOGNIZED at the repository `init` was run in:\n\
                 none of the manifests or lockfiles it looks for were at that root,\n\
                 so the implement and verify charters\n\
                 (`agents/charters/`) carry GENERIC placeholders rather than\n\
                 commands that would be guesses. Fill them in before the first run.\n\n\
                 The tool map was scaffolded EMPTY for the same reason:\n\
                 `adapters/claude.json` → `tool_permissions.names` names nothing,\n\
                 and no agent under `agents/` declares a `tools` restriction.\n\
                 `brokkr init` grants no tool it could not read from a manifest,\n\
                 because a tool name is a permission and one guessed is one granted.\n\n\
                 Before a headless run, find this repository's own build, test and\n\
                 lint commands and grant them by name: add each binary to the\n\
                 adapter's `tool_permissions.names` as `Bash(<bin>:*)`, then list\n\
                 the names in each agent's `tools.allow` — the work-class seats\n\
                 (intake, implement) get the whole set, the gate-class seats\n\
                 (verify, review, ship) the read-only subset (git, ls, rg and the\n\
                 test runner).\n"
            .to_string(),
    }
}

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
    // The pip fallback. The commands run through `python3`, not
    // `python`: the interpreter a fresh project actually resolves is
    // python3 (the shipped adapters grant it that name), and a charter
    // that said `python` would hand the seat a command its own allowance
    // cannot answer. pytest, the venv's suite binary, is granted beside
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
/// They live under `agents/charters/`, because an agent's charter must
/// stay inside the library that names it.
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
    // The agent library is workspace data on the same terms: a grant an
    // operator narrowed, or a chain they re-ordered, is theirs, and a
    // scaffolder that wrote over it would widen a permission by accident.
    // Each definition is refused by name, before anything is written.
    let library = dir.join(DEFAULT_AGENTS_DIR);
    for spec in SEATS {
        let definition = library.join(format!("{}.json", spec.agent));
        if definition.exists() {
            bail!(
                "{} already defines an agent; refusing to overwrite — its \
                 allowances are an operator's ruling, not a scaffold's",
                definition.display()
            );
        }
    }
    let detected = detect(repo);
    let detected = detected.as_ref();
    let grants = grants(detected);
    std::fs::create_dir_all(library.join("charters"))?;
    std::fs::create_dir_all(dir.join(DEFAULT_ADAPTERS_DIR))?;
    std::fs::write(dir.join("policy.json"), POLICY)?;
    std::fs::write(dir.join("bundle.json"), BUNDLE)?;
    std::fs::write(dir.join("README.md"), readme(detected))?;
    std::fs::write(&declaration, adapter_json(&grants))?;
    for spec in SEATS {
        let allowance = allowance(spec, &grants);
        let definition = library.join(format!("{}.json", spec.agent));
        std::fs::write(definition, agent_json(spec, allowance))?;
    }
    for (name, content) in charters(detected) {
        std::fs::write(library.join("charters").join(name), content)?;
    }
    // init proves its own output: the scaffold must compile under the
    // constitutional lint before we call it a bundle — and, because the
    // seats resolve through the scaffold's own agents, that compile
    // resolves every allowance through the scaffold's own `adapters/`, so
    // a tool name this file got wrong refuses right here. Against the
    // SCAFFOLD's own roots, not the process's: what init proves must be a
    // property of what it wrote, and a starter that compiled only because
    // the caller happened to stand in a tree with an `adapters/` or
    // `agents/` would be a proof about the caller.
    let bundle = Bundle::compile_with(dir, &library, &dir.join(DEFAULT_ADAPTERS_DIR))
        .context("scaffolded bundle failed to compile")?;
    Ok(bundle.manifest_digest())
}

#[cfg(test)]
mod tests;
