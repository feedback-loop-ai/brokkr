//! The scaffold's `bundle.json`: its seats, and the binds verify's hands
//! declare, the stack's own beside a node-family runner's install.

use serde_json::json;

use super::{node, stack_binds, Detected};

/// The scaffold follows the shipped roster: work and review are model
/// offices; verify and ship are exec scripts, boxed under `namespace`.
pub(super) fn bundle_json(detected: Option<&Detected>, reach: Option<&node::Reach>) -> String {
    let mut verify_binds = stack_binds(detected);
    verify_binds.extend(reach.map_or_else(Vec::new, node::Reach::binds));
    let bundle = json!({
        "name": "starter",
        "policy": "policy.json",
        "protected_phase": "review",
        "seats": {
            "intake": {"agent": "intake", "class": "work", "results": ["resolved"]},
            "implement": {"agent": "implementer", "class": "work", "results": ["complete", "broken", "blocked"]},
            "verify": {
                "class": "gate",
                "results": ["pass", "fail"],
                "limits": {"max_attempts": 2, "timeout_seconds": 3600},
                "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash", "./scripts/verify-seat.sh", "{prompt_file}"]},
                "hands": {"kind": "workspace", "network": false, "binds": verify_binds},
            },
            "review": {"agent": "reviewer", "class": "gate", "results": ["clean", "residual", "security-hold"]},
            "ship": {
                "class": "gate",
                "results": ["ready", "shipped"],
                "limits": {"max_attempts": 2, "timeout_seconds": 1800},
                "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash", "./scripts/ship-seat.sh", "{prompt_file}", "{brokkr}"]},
                "hands": {"kind": "workspace", "network": false, "binds": []},
            },
        }
    });
    format!(
        "{}\n",
        serde_json::to_string_pretty(&bundle).expect("bundle serializes")
    )
}
