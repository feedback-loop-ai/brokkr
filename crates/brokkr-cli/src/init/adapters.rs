//! The adapter declarations `brokkr init` writes under the scaffold's
//! `adapters/`: one per agent CLI a seat is hired from, then exec's.
//!
//! Each model declaration carries its shipped adapter's MCP facts as U0
//! measured them (decision 0065 slice two, SI1 and SI2): the carriage
//! verdict and every measured shape the generated declaration can admit.
//! A shape is admitted only under hands and a resume shape the declaration
//! itself declares, so the scaffold's claude, which declares no hands,
//! carries Claude's cold shape without hands and not its boxed one, and the
//! scaffold's codex, which declares no `resume`, carries Codex's two cold
//! shapes and not its resumed one. Generation grants nothing and qualifies
//! nothing: the facts are copies, and `init_doctor` and `init_stacks` hold
//! each one equal to `adapters/*.json`. Exec serves no model, so it carries
//! the shipped declaration that it has no model MCP surface, never a
//! strictness of its own.

use serde_json::{json, Map, Value};

use super::{Cli, Grants};

/// The declaration written for `cli`'s seats.
pub(super) fn declaration(cli: Cli, grants: &Grants) -> String {
    match cli {
        Cli::Claude => claude(grants),
        Cli::Codex => CODEX.to_string(),
        Cli::Dsh => DSH.to_string(),
    }
}

/// The scaffold's exec adapter: the deterministic verify and ship offices.
pub(super) const EXEC: &str = r#"{
  "provider": "exec",
  "trust_tier": "untrusted",
  "egress": "contracted",
  "binary": "sh",
  "driver": ["{brokkr}", "driver", "exec", "--"],
  "models": {},
  "judges": [],
  "model_flag": "unsupported",
  "efforts": [],
  "effort_flag": "unsupported",
  "tool_permissions": "unsupported",
  "mcp": {
    "inapplicable": "exec runs the script its bundle names and serves no model, so it has no model MCP surface; this is neither a measured strictness nor a missing measurement (U0, exec: docs/evidence/adapters/slice-two-mcp-isolation.md)"
  },
  "hands": {"workspace": []}
}
"#;

/// Claude Code's native network tools, as `adapters/claude.json` declares
/// them (decision 0065 ruling 4): each switched ON by admitting it to the
/// seat's own tool lists and OFF by denying it by name. Adapter data and
/// argv composition only — no live denial or enablement was measured, and
/// the limitations say so. `init_stacks` holds this equal to the shipped
/// declaration, so a scaffold cannot drift into a weaker one.
fn claude_native_capabilities() -> Value {
    let native = |capability: &str, tool: &str, restriction: &str, extra: &[&str]| {
        let mut limitations = vec![
            "that a boxed seat's empty --tools list under --strict-mcp-config leaves no native \
             tool is adapter data, not a live measurement"
                .to_string(),
            format!(
                "{tool} ON beside the hands tool, and {tool} OFF by --disallowedTools on an \
                 unboxed seat, are both unmeasured live; the checks are owed to the controller"
            ),
        ];
        limitations.extend(extra.iter().map(|gap| gap.to_string()));
        json!({
            "capability": capability,
            "tools": [tool],
            "on": {"selection": {"include": [tool], "allow": [tool], "deny": []}},
            "off": {"selection": {"include": [], "allow": [], "deny": [tool]}},
            "restrictions": {"unsupported": restriction},
            "evidence": {
                "source": "adapters/claude.json and the installed 2.1.266 help: --tools, \
                           --allowedTools and --disallowedTools each take tool names",
                "scope": format!(
                    "adapter data and argv composition only; no live denial or enablement of \
                     {tool} has been measured"
                ),
                "limitations": limitations,
            },
            "authored": {
                "list_flags": ["--tools", "--allowedTools", "--allowed-tools"],
                "value_flags": ["--model", "--effort", "--permission-mode", "--mcp-config"],
            },
        })
    };
    json!({
        "known": {
            "web-search": native(
                "web-search",
                "WebSearch",
                "no native transport for a restriction on Claude Code's WebSearch has been \
                 established",
                &["this names the two native network tools that were known, not an exhaustive \
                   inventory of what Claude Code can reach on its own"],
            ),
            "web-fetch": native(
                "web-fetch",
                "WebFetch",
                "no native transport for a restriction on Claude Code's WebFetch has been \
                 established; a WebFetch(domain:…) permission pattern was not measured as a \
                 host allowlist",
                &[],
            ),
        },
        "selection": {
            "include": {"flag": "--tools", "separator": ","},
            "allow": {"flag": "--allowedTools", "separator": ","},
            "deny": {"flag": "--disallowedTools", "separator": ","},
        },
    })
}

/// Claude's MCP facts that the scaffold's claude can admit: carriage, and
/// the cold shape without hands, because the scaffold gives no claude seat
/// hands.
const CLAUDE_MCP: &str = r#"{
  "carriage": {
    "measured": "U0 C05 and C06 (claude 2.1.287, Linux, 2026-10-03): an engine-written --mcp-config under --strict-mcp-config started the engine sentinel, listed it in system/init and answered its tools/call, without and with the hands server (docs/evidence/adapters/slice-two-mcp-isolation.md)"
  },
  "shapes": [
    {
      "invocation": "cold",
      "hands": "none",
      "measured_on": {"harness": "claude", "binary": "claude", "version": "2.1.287", "host": "linux"},
      "ambient": {
        "measured": "U0 C03 and C05: under --strict-mcp-config with an explicit empty or engine --mcp-config no planted user, project or plugin sentinel started and no claude.ai connector loaded while the engine sentinel answered; C07's managed-file refusal applies"
      },
      "native_write": {"unsupported": "U0 C09b: native Write was allowed outside the cwd"},
      "store_read": {"unsupported": "U0 C09b: native Read read the 0600 store canary"},
      "process_read": {
        "unmeasured": "U0 C09b: no mechanism excludes it; Claude's permission check blocked the one Bash probe of /proc/<pid>/environ, and native Read on /proc was not probed"
      }
    }
  ]
}"#;

/// The scaffold's claude adapter: decision 0021's trust declaration
/// (ruling 2's trusted tier — the starter's gate seats compile against
/// it — and an `uncontracted` egress) plus the tool map. `names`
/// is the union of every allowance the scaffold wrote — the work set,
/// which carries the gate set inside it — because a name any agent's
/// `tools.allow` lists must be expressible here or the scaffold's own
/// compile refuses. Where nothing was recognized the map stays EMPTY,
/// and the README carries the sentence that says which of the two it is.
fn claude(grants: &Grants) -> String {
    let mut names = Map::new();
    for tool in &grants.work {
        names.insert(tool.name.to_string(), json!(tool.permission));
    }
    let mcp: Value = serde_json::from_str(CLAUDE_MCP).expect("claude's MCP facts are JSON");
    let adapter = json!({
        "provider": "claude",
        "trust_tier": "trusted",
        "egress": "uncontracted",
        "binary": "claude",
        "driver": ["{brokkr}", "driver", "claude", "--", "--permission-mode", "acceptEdits"],
        "models": {
            "fable": "claude-fable-5-1",
            "opus": "claude-opus-5-5",
            "sonnet": "claude-sonnet-5-5",
            "haiku": "claude-haiku-4-5-20251001"
        },
        "judges": ["fable", "opus"],
        "model_flag": "--model",
        // The levels the installed CLI names, measured rather than
        // assumed — `claude --help` spells them out beside `--effort`.
        "efforts": ["low", "medium", "high", "xhigh", "max"],
        "effort_flag": "--effort",
        "tool_permissions": {"flag": "--allowedTools", "separator": ",", "names": names},
        "mcp": mcp,
        // Decision 0065 ruling 4: what this harness can already reach on
        // its own, and how each such power is switched on and off. A
        // scaffold grants nothing, so a stranger's first seats are
        // launched with both denied by name — the same assessment the
        // shipped adapter carries, word for word, evidence limits included.
        "native_capabilities": claude_native_capabilities(),
        // What has been MEASURED about resuming this provider here, in
        // this workspace, on this machine: nothing (proposed decision
        // 0056 ruling 5). The scaffold could omit the key — absence
        // reads as unmeasured and enables nothing — and writes it
        // anyway, because a stranger's first adapter should show the
        // shape a measurement would go into, and because "nobody has
        // looked" is a different statement from "we looked and it
        // cannot". Filling it in is the operator's own measurement, and
        // only a `supported` entry naming a measured version and all
        // four evidence references ever enables a rejoin.
        "resume": {
            "boxed-workspace": {
                "status": "unmeasured",
                "identity": {"unknown": "this workspace has measured no resumed invocation"},
                "classes": ["work"],
                "boundaries": ["namespace", "seatbelt", "container"],
                "hands": "boxed",
                "reason": "scaffolded, never measured: whether a resumed session enforces \
                           this seat's permission mode, tool list and MCP config is exactly \
                           what has to be observed before a retry may rejoin one"
            }
        }
    });
    format!(
        "{}\n",
        serde_json::to_string_pretty(&adapter).expect("the claude adapter serializes")
    )
}

/// The scaffold's codex adapter, from the library's own: trusted, with
/// the two judges the review gate is hired from. It restricts by sandbox
/// CLASS, not by tool name, so no seat hired from it carries a tool
/// allowance; its seats carry hands instead, and under the `harness`
/// boundary the class fragments below are what hold them — read-only for
/// the gate, workspace-write for the work seats. Its MCP facts are Codex's
/// two cold shapes, the boxed and the harness hands it declares.
const CODEX: &str = r#"{
  "provider": "codex",
  "trust_tier": "trusted",
  "egress": "uncontracted",
  "binary": "codex",
  "driver": ["{brokkr}", "driver", "codex", "--"],
  "models": {
    "astra": "gpt-6-astra",
    "sol": "gpt-6.1-sol",
    "terra": "gpt-5.6-terra"
  },
  "judges": ["astra", "sol"],
  "model_flag": "--model",
  "efforts": ["none", "minimal", "low", "medium", "high", "xhigh", "max"],
  "effort_flag": "--effort",
  "tool_permissions": {
    "unsupported": "codex exec restricts by sandbox class (read-only, workspace-write), not by tool name; there is no per-tool allow-list flag to map a seat's tools onto"
  },
  "mcp": {
    "carriage": {
      "measured": "U0 X02b and X04 (codex-cli 0.160.0, Linux, 2026-10-03): the engine sentinel in a private CODEX_HOME config.toml answered cold and on exec resume; XA2 (2026-10-04, gpt-6-luna): the engine and hands tools answered through the code-mode tool catalogue"
    },
    "shapes": [
      {
        "invocation": "cold",
        "hands": "boxed",
        "measured_on": {"harness": "codex", "binary": "codex", "version": "0.160.0", "host": "linux"},
        "ambient": {
          "unsupported": "project, system and managed MCP configuration cannot be excluded; U0 X06 listed the project server beside the hands server, X12c the /etc system and managed servers, and X07's whole-table override loaded every planted source"
        },
        "native_write": {
          "measured": "U0 X06: a hands write outside the worktree failed; X09: codex's own read-only sandbox, run without a model, denied native writes outside the workspace"
        },
        "store_read": {
          "unsupported": "U0 X09: codex's native shell under the read-only sandbox this shape passes read the 0600 store canary; the hands tool excluded it in X06, where the model declined the native probe (X06n, X06n2)"
        },
        "process_read": {
          "measured": "U0 X06: the hands tool excluded the process canary; X08 and X09: codex's native shell ran in a private pid namespace of 4 pids with no canary"
        }
      },
      {
        "invocation": "cold",
        "hands": "harness",
        "measured_on": {"harness": "codex", "binary": "codex", "version": "0.160.0", "host": "linux"},
        "ambient": {
          "unsupported": "project, system and managed MCP configuration cannot be excluded; U0 X12 and X02b loaded the project server once codex wrote trust for the workdir into the engine-owned config, X02 the /etc system and managed servers, and X03's whole-table override every source"
        },
        "native_write": {
          "measured": "U0 X08 and X09: codex's sandbox denied native writes outside the workspace under both read-only and workspace-write; the workspace is writable only under workspace-write"
        },
        "store_read": {
          "unsupported": "U0 X08 and X09: codex's native shell read the 0600 store canary under both read-only and workspace-write"
        },
        "process_read": {
          "measured": "U0 X08 and X09: codex's native shell ran in a private pid namespace of 4 pids with no process canary"
        }
      }
    ]
  },
  "native_capabilities": {
    "known": {
      "web-search": {
        "capability": "web-search",
        "tools": [
          "web_search"
        ],
        "on": {
          "default": "codex-cli 0.154.0 cold `codex exec` has server-side web search ON with no flag: the default run of the 2026-09-21 controller measurement issued a web_search item and answered with a cited version. No explicit ON value is declared because none was measured."
        },
        "off": {
          "argv": [
            "-c",
            "web_search=\"disabled\""
          ]
        },
        "restrictions": {
          "unsupported": "no native transport for a restriction on codex's server-side search has been established; the measurement covers only the key's \"disabled\" value"
        },
        "evidence": {
          "source": ".forge/tasks/controller-codex-web-search-switch-2026-09-21.json",
          "scope": "codex-cli 0.154.0, cold `codex exec` only: with `-c web_search=\"disabled\"` the model answered NO SEARCH TOOL; without it the tool ran",
          "limitations": [
            "whether the OFF switch holds on a RESUMED codex session is unmeasured; the engine composes the pair on the `exec resume` argv too, and the live check is owed to the controller",
            "whether a resumed session that holds web-search has it ON is unmeasured",
            "values of web_search other than \"disabled\" were not measured, and the interactive CLI's --search flag was not exercised under `codex exec`",
            "codex-cli versions other than 0.154.0 were not measured",
            "this is one native capability that was measured, not an exhaustive inventory of what codex can reach on its own; profile and config.toml precedence over the -c override is unmeasured",
            "the hands fragment adds mcp_servers.brokkr and does not establish that an ambient MCP server in the operator's codex configuration is excluded"
          ]
        },
        "authored": {
          "flags": [
            "--search"
          ],
          "config_flags": [
            "-c",
            "--config"
          ],
          "config_keys": [
            "web_search",
            "web_search_mode",
            "tools.web_search",
            "features.web_search_request",
            "features.web_search_cached"
          ],
          "feature_flags": [
            "--enable",
            "--disable"
          ],
          "features": [
            "web_search_request",
            "web_search_cached"
          ],
          "value_flags": [
            "-m",
            "--model",
            "-i",
            "--image",
            "-o",
            "--output-last-message",
            "--output-schema",
            "-s",
            "--sandbox",
            "--effort",
            "-p",
            "--profile",
            "-C",
            "--cd",
            "--add-dir"
          ]
        }
      }
    }
  },
  "hands": {
    "workspace": [
      "--sandbox", "read-only",
      "-c", "mcp_servers.brokkr.command=\"{brokkr}\"",
      "-c", "mcp_servers.brokkr.args={hands_args_toml}",
      "-c", "mcp_servers.brokkr.default_tools_approval_mode=\"approve\""
    ],
    "notice": {"workspace_tool": "mcp__brokkr__workspace", "discovery_tool": "tool_search"},
    "harness": {
      "gate": ["--sandbox", "read-only", "--output-last-message", "{result_path}"],
      "work": ["--sandbox", "workspace-write"],
      "result": "last-message"
    }
  }
}
"#;

/// The scaffold's dsh adapter: untrusted and judging nothing, so it holds
/// the work seats and never the review gate. It can restrict neither its
/// tools nor its hands from the command line, and says so. Its MCP facts
/// are dsh's one measured shape, cold without hands.
const DSH: &str = r#"{
  "provider": "dsh",
  "trust_tier": "untrusted",
  "egress": "uncontracted",
  "binary": "dsh",
  "driver": ["{brokkr}", "driver", "dsh", "--"],
  "models": {
    "flash": "deepseek-flash",
    "pro": "deepseek-v4-pro"
  },
  "judges": [],
  "model_flag": "--model",
  "efforts": ["low", "medium", "high", "xhigh"],
  "effort_flag": "--effort",
  "tool_permissions": "unsupported",
  "mcp": {
    "carriage": {
      "measured": "U0 D03 (dsh 0.1.5-rc.1, Linux, 2026-10-03, local Spark route): the engine's dsh-mcp-client row, inserted by the --patch overlay over an engine-only DSH_HOME, started the engine sentinel and answered its call; U0c K02, K05, K08 and K11t (2026-10-06) answered it on deepseek-official, dashscope, meta and meta-contributor (docs/evidence/adapters/slice-two-mcp-isolation.md)"
    },
    "shapes": [
      {
        "invocation": "cold",
        "hands": "none",
        "measured_on": {"harness": "dsh", "binary": "dsh", "version": "0.1.5-rc.1", "host": "linux"},
        "ambient": {
          "measured": "U0 D03 and D04 (spark, spark-glm) and U0c K01 to K12r (deepseek-official, dashscope, meta, meta-contributor): under an engine-only DSH_HOME no planted home- or profile-level sentinel started while the engine row loaded, and D01 and U0c's positive controls answered both; dsh's existing home fails (D02); no plugin bundle layer was planted"
        },
        "native_write": {
          "unmeasured": "U0 D04: dsh's default workspace-write sandbox denied one native write outside the workspace, but dsh offered an approval-gated escalation (policy ask) that was not exercised, so confinement is not established"
        },
        "store_read": {
          "unsupported": "U0 D04: dsh's native shell under its default workspace-write sandbox, in an engine-only DSH_HOME, read the 0600 store canary outside the workspace"
        },
        "process_read": {
          "measured": "U0 D04: dsh's native shell ran in a private pid namespace of 4 pids and found no process canary in /proc"
        }
      }
    ]
  },
  "native_capabilities": {
    "unmeasured": "dsh's tool_permissions and mcp declarations cover command-line narrowing, ambient MCP exclusion and engine-server carriage only, which does not establish that dsh has no native egress. U0 and U0c listed native web_fetch and web_search under an engine-only DSH_HOME, and the web_search row (web-search-deepseek) reads DEEPSEEK_API_KEY on every route family; no cell exercised either, and no OFF control has been declared or measured. Its native inventory, and any ON or OFF control for it, remain unmeasured: nothing is granted through dsh and no native denial is claimed (decision 0065 slice one; owed to the controller)"
  },
  "hands": {
    "unsupported": "dsh replaces its tool surface only through a profile plugin; no CLI flag disables its shell and file tools or adds an MCP server"
  }
}
"#;
