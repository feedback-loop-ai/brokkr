//! `brokkr init <dir>` — scaffold a minimal reviewable bundle and prove it
//! compiles. The template carries the tightened ship taxonomy (`ready` →
//! `shipped` as the sole entry into `done`), the protected review phase,
//! one charter-defined seat per phase, per-seat limits (decision 0006),
//! the work/gate division of decision 0021 ruling 1, and the bundled
//! headless Claude Code driver. Everything written is ordinary text meant
//! to be reviewed and edited in git.
//!
//! The scaffold is a WORKSPACE, not only a bundle: it carries its own
//! `adapters/` tree, because since decision 0021 the tier that lets a
//! gate seat judge is adapter data, and a starter whose review seat
//! judged on nobody's authority would teach the wrong lesson on day one.
//! The operator's trust declarations are theirs to edit — which is why
//! they are scaffolded as a file in their tree rather than compiled into
//! this binary — and `brokkr` is run from inside the scaffold, where its
//! `adapters/` is the workspace's.

use std::path::Path;

use anyhow::{bail, Context, Result};
use brokkr_runtime::bundle::{DEFAULT_ADAPTERS_DIR, DEFAULT_AGENTS_DIR};
use brokkr_runtime::Bundle;

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
      "role": "roles/intake.md",
      "class": "work",
      "results": ["resolved"],
      "limits": {"max_attempts": 2, "timeout_seconds": 1800},
      "driver": {"command": ["{brokkr}", "driver", "claude", "--", "--permission-mode", "acceptEdits"]}
    },
    "implement": {
      "role": "roles/implementer.md",
      "class": "work",
      "results": ["complete", "broken", "blocked"],
      "limits": {"max_attempts": 2, "timeout_seconds": 5400},
      "driver": {"command": ["{brokkr}", "driver", "claude", "--", "--permission-mode", "acceptEdits"]}
    },
    "verify": {
      "role": "roles/verifier.md",
      "class": "gate",
      "results": ["pass", "fail"],
      "limits": {"max_attempts": 2, "timeout_seconds": 3600},
      "driver": {"command": ["{brokkr}", "driver", "claude", "--", "--permission-mode", "acceptEdits"]}
    },
    "review": {
      "role": "roles/reviewer.md",
      "class": "gate",
      "results": ["clean", "residual", "security-hold"],
      "limits": {"max_attempts": 2, "timeout_seconds": 3600},
      "driver": {"command": ["{brokkr}", "driver", "claude", "--", "--permission-mode", "acceptEdits"]}
    },
    "ship": {
      "role": "roles/shipper.md",
      "class": "gate",
      "results": ["ready", "shipped"],
      "limits": {"max_attempts": 2, "timeout_seconds": 1800},
      "driver": {"command": ["{brokkr}", "driver", "claude", "--", "--permission-mode", "acceptEdits"]}
    }
  }
}
"#;

/// The scaffold's own trust declaration (decision 0021 rulings 2 and 4),
/// for the one driver it seats. `trusted`, because the starter's verify,
/// review and ship seats are the gate roster of ruling 1 and would
/// otherwise refuse to compile — the operator inherits the incumbent's
/// journaled record and may demote it by editing this file. No binding
/// grant: no seat here declares `secrets`, and a grant nothing needs is
/// one more thing to take away later (ruling 4 — trust to judge and
/// clearance to receive are different grants).
const ADAPTER: &str = r#"{
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
  "tool_permissions": {"flag": "--allowedTools", "separator": ",", "names": {}},
  "mcp": {"flag": "--mcp-config", "servers": {}}
}
"#;

const ROLES: [(&str, &str); 5] = [
    (
        "intake.md",
        "# Intake seat — frame the task\n\nRead the feature description and the repository until you can state\nthe task precisely. Write a short framing (goal, files you expect to\nchange, the tests that must prove it, non-goals) to\n`.forge/tasks/<slug>.md` in the working directory — run-local evidence,\nnot committed. Result: `resolved`, with `notes` naming the framing file.\nYou never decide the run's fate; the policy table does.\n",
    ),
    (
        "implementer.md",
        "# Implementer seat — build it\n\nImplement the framed task (see `.forge/tasks/`). Match the project's\nidiom. Tests are part of the change. Run the project's test suite\nyourself before declaring anything. Commit your work; never push.\n\nResult: `complete` (implemented, tests green, committed) · `broken`\n(could not get it working — name the specific gap in `notes`) ·\n`blocked` (something outside your control — name it precisely). Never\nreport `complete` with failing tests or uncommitted changes.\n",
    ),
    (
        "verifier.md",
        "# Verifier seat — prove it, fix nothing\n\nRun the project's full test and lint suites from the repository root.\nYou change no code, fix nothing, commit nothing: one honest run is the\nsignal. Result: `pass` (everything green; `notes` lists commands and\ncounts) or `fail` (`notes` quotes the failing output's decisive lines\nexactly — never soften a failure).\n",
    ),
    (
        "reviewer.md",
        "# Reviewer seat — adversarial review, security riding along\n\nReview everything changed since the run began (`git log`/`git diff`).\nDimensions: correctness, simplicity, and SECURITY (non-removable;\nseverity vocabulary `none|info|low|medium|high|critical`). You may\napply small safe fixes — commit them and set `fixes_applied: true`\n(the machine then re-verifies; that is correct).\n\nResult: `clean` with `inputs: {\"fixes_applied\": <bool>}` · `residual`\nwith `inputs: {\"max_residual_severity\": \"<severity>\",\n\"has_security_residual\": <bool>}` (list every finding in `notes`;\nnever understate severity — the table decides what ships) ·\n`security-hold` for any unresolved high/critical security finding.\n",
    ),
    (
        "shipper.md",
        "# Shipper seat — close out, hand to the operator\n\nYou do NOT push, merge, or open PRs — the operator holds that\nauthority. Make no commits in this phase: the drift gate compares HEADs\nagainst review time.\n\nStep 2 (`context.last_decision.rule_id == \"SHIP-READY\"`): confirm the\ntree is still clean and HEAD unchanged, then report `shipped` with a\nclose-out summary for the operator.\n\nStep 1 (anything else): confirm the tree is clean, write the delivery\nledger to `.forge/ledger/<run-id>.md` (what shipped, commits, test\nevidence, residual debt, operator next steps — run-local, not\ncommitted), and report `ready`.\n",
    ),
];

pub fn init(dir: &Path) -> Result<String> {
    if dir.join("bundle.json").exists() {
        bail!(
            "{} already contains a bundle.json; refusing to overwrite",
            dir.display()
        );
    }
    std::fs::create_dir_all(dir.join("roles"))?;
    std::fs::create_dir_all(dir.join(DEFAULT_ADAPTERS_DIR))?;
    std::fs::write(dir.join("policy.json"), POLICY)?;
    std::fs::write(dir.join("bundle.json"), BUNDLE)?;
    std::fs::write(dir.join(DEFAULT_ADAPTERS_DIR).join("claude.json"), ADAPTER)?;
    for (name, content) in ROLES {
        std::fs::write(dir.join("roles").join(name), content)?;
    }
    // init proves its own output: the scaffold must compile under the
    // constitutional lint before we call it a bundle. Against the
    // SCAFFOLD's own roots, not the process's: what init proves must be
    // a property of what it wrote, and a starter that compiled only
    // because the caller happened to stand in a tree with an `adapters/`
    // would be a proof about the caller.
    let bundle = Bundle::compile_with(
        dir,
        &dir.join(DEFAULT_AGENTS_DIR),
        &dir.join(DEFAULT_ADAPTERS_DIR),
    )
    .context("scaffolded bundle failed to compile")?;
    Ok(bundle.manifest_digest())
}
