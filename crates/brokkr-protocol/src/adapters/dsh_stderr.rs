//! The dsh stderr a journal may quote: the child's own lines with its
//! reasoning redacted, and the one line the driver adds when the child
//! fails.

use super::{parse_dsh_model, split_dsh_model, Invocation};

/// The one dsh stderr stream the journal may not quote.
///
/// dsh 0.1.2-rc.1's headless profile streams the model's reasoning to
/// stderr under a `dsh: reasoning:` line (measured 2026-09-04: one
/// header, then the raw thinking text, until the harness's next `dsh: `
/// line or the end of the stream). The driver's stderr tail is what a
/// parked seat quotes into the journal, and a journal admits no
/// reasoning text (decisions 0032 and 0034). So a reasoning block is
/// replaced by one line that says it was there, and every harness line
/// survives, because those are what a park needs to be read.
pub(super) fn redact_dsh_reasoning(stderr: &str) -> String {
    const HEADER: &str = "dsh: reasoning:";
    const REDACTED: &str = "dsh: reasoning: [not journaled — decision 0034]";
    let mut kept = Vec::new();
    let mut inside = false;
    for line in stderr.lines() {
        if line.trim_end() == HEADER {
            inside = true;
            kept.push(REDACTED);
        } else if line.starts_with("dsh: ") {
            inside = false;
            kept.push(line);
        } else if !inside {
            kept.push(line);
        }
    }
    let mut text = kept.join("\n");
    if stderr.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// A failed dsh seat names the route and model it pinned (#532), as the
/// last line of its stderr, so the tail the engine journals beside `agent
/// CLI exited <code>` says which route the seat asked for. A host whose
/// dsh does not serve that route fails here, and the operator reads the
/// route from the failure rather than from the bundle.
///
/// The line states the pin and the exit code and nothing else: it reads
/// none of dsh's prose and decides nothing from it (decisions 0001 and
/// 0053). A clean exit, and a seat that pinned no model, add nothing.
/// The pin was validated before launch, so the parse here cannot refuse;
/// should it, the line names the pin whole as its route.
pub(super) fn name_the_pin(mut invocation: Invocation, extra: &[String]) -> Invocation {
    let pinned = split_dsh_model(extra).ok().and_then(|(model, _)| model);
    if let (true, Some(model)) = (invocation.exit_code != 0, pinned.as_deref()) {
        let route = parse_dsh_model(model).map_or(model, |pin| pin.provider);
        let stderr = &mut invocation.stderr;
        if !stderr.is_empty() && !stderr.ends_with('\n') {
            stderr.push('\n');
        }
        stderr.push_str(&format!(
            "dsh driver: dsh exited {} with the pinned model {model} on route {route}\n",
            invocation.exit_code
        ));
    }
    invocation
}

#[cfg(test)]
mod tests {
    use serde_json::Map;

    use super::super::LaunchTerminal;
    use super::*;

    fn failed(exit_code: i32, stderr: &str) -> Invocation {
        Invocation {
            exit_code,
            session_meta: Map::new(),
            stdout: String::new(),
            stderr: stderr.to_string(),
            state: None,
            refusal: None,
            launch: LaunchTerminal::Cold,
        }
    }

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_failed_seat_names_its_pinned_route_after_the_child_lines() {
        let glm = argv(&["--model", "spark-glm/GLM-5.3-Flash-EXL3"]);
        // A route-less host's dsh may say nothing at all: the line stands
        // alone.
        assert_eq!(
            name_the_pin(failed(1, ""), &glm).stderr,
            "dsh driver: dsh exited 1 with the pinned model \
             spark-glm/GLM-5.3-Flash-EXL3 on route spark-glm\n"
        );
        // The child's lines come first and keep their bytes, whether or not
        // the last one ended its line.
        assert_eq!(
            name_the_pin(failed(3, "Error: no route\n"), &glm).stderr,
            "Error: no route\ndsh driver: dsh exited 3 with the pinned model \
             spark-glm/GLM-5.3-Flash-EXL3 on route spark-glm\n"
        );
        assert_eq!(
            name_the_pin(failed(3, "Error: no route"), &glm).stderr,
            "Error: no route\ndsh driver: dsh exited 3 with the pinned model \
             spark-glm/GLM-5.3-Flash-EXL3 on route spark-glm\n"
        );
        // A bare id rides dsh's official route, and the line says so.
        assert_eq!(
            name_the_pin(failed(2, ""), &argv(&["--model", "deepseek-flash"])).stderr,
            "dsh driver: dsh exited 2 with the pinned model deepseek-flash on route \
             deepseek-official\n"
        );
        // A pin the launch grammar refuses (unreachable after pre-launch
        // validation, so bound here) names itself whole as its route.
        assert_eq!(
            name_the_pin(failed(2, ""), &argv(&["--model", "dashscope/deepseek@v4"])).stderr,
            "dsh driver: dsh exited 2 with the pinned model dashscope/deepseek@v4 on \
             route dashscope/deepseek@v4\n"
        );
    }

    #[test]
    fn a_clean_exit_or_an_unpinned_seat_adds_nothing() {
        let glm = argv(&["--model", "spark-glm/GLM-5.3-Flash-EXL3"]);
        assert_eq!(
            name_the_pin(failed(0, "dsh: done\n"), &glm).stderr,
            "dsh: done\n"
        );
        assert_eq!(
            name_the_pin(failed(1, "dsh: failed\n"), &[]).stderr,
            "dsh: failed\n"
        );
        // A pin the grammar cannot even split (never reaches launch) adds
        // nothing either: there is no pin to name.
        assert_eq!(
            name_the_pin(failed(1, "dsh: failed\n"), &argv(&["--model"])).stderr,
            "dsh: failed\n"
        );
    }
}
