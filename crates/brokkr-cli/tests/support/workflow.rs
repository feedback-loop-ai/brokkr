//! A workflow's jobs, for the test files that hold the repository's
//! workflows to their rulings. `tests/it.rs` declares this once, so the
//! rule that splits a workflow into its jobs has one home.

/// The jobs of a workflow's text, keyed by job id, in file order. A job
/// starts at a key indented two spaces under `jobs:`; its body is every
/// line after that key up to the next one, a comment between two jobs
/// included in the first.
pub(crate) fn jobs(workflow: &str) -> Vec<(String, String)> {
    let (_, jobs) = workflow.split_once("\njobs:\n").expect("a jobs map");
    let mut parsed: Vec<(String, String)> = Vec::new();
    for line in jobs.lines() {
        let id = line
            .strip_prefix("  ")
            .filter(|rest| !rest.starts_with([' ', '#']))
            .and_then(|rest| rest.strip_suffix(':'));
        match (id, parsed.last_mut()) {
            (Some(id), _) => parsed.push((id.to_string(), String::new())),
            (None, Some((_, body))) => {
                body.push_str(line);
                body.push('\n');
            }
            (None, None) => {}
        }
    }
    parsed
}
