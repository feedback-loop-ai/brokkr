# Controller evidence, 2026-09-10

These are partial installed-provider observations for task 10.6, not an
adapter enablement decision or completion of the full restriction proof.
The controller ran the probes outside the worker's read-only provider home.
The previous worker EROFS result therefore does not establish that the
controller cannot obtain Claude resume evidence.

All observations below used Claude Code 2.1.266, executable SHA-256
`19842705e989393fce936804df6d2ab034860e24b8f8880357981d87ffd83fac`.
The requested model was `sonnet`; the provider reported `claude-sonnet-5`.
The probes used only disposable controller fixtures. Raw local artifacts
are retained under `.forge/tasks/controller-claude-*-probe.json`; each
artifact records invocation arguments, stdout/stderr locations and results.

## Observations

- Root continuity: cold storage of a random token followed by explicit
  `--resume` recalled that token without resending it and returned the same
  session ID. Evidence: `controller-claude-root-probe.json`.
- Read grant replacement: a cold allowed Read succeeded; after same-root
  resume with a replacement grant, an actual Read of the old path was denied
  and an actual Read of the new path succeeded. Evidence:
  `controller-claude-grant-probe.json`.
- Native tool removal: the positive control actually wrote its fixture;
  same-root resume advertised Read only and left that file unchanged.
  Separate earlier trials received an actual unavailable-Write tool error,
  but lacked that successful cold write control. This is composite partial
  evidence, not a single complete enforcement experiment. Evidence:
  `controller-claude-tool-removal-probe.json`.
- MCP configuration removal: the cold invocation connected to a local
  fixture MCP server and actually called its ping tool. Same-root resume
  with `--strict-mcp-config --mcp-config '{"mcpServers":{}}'` advertised no
  tools or MCP servers, and the fixture server log received no new events.
  The resumed invocation produced an empty visible answer and did not attempt
  the removed tool. This proves the observed configuration change, not an
  explicit rejected invocation. Evidence: `controller-claude-mcp-probe.json`.

## Accounting comparison and limits

`controller-claude-accounting-analysis.json` compares each invocation's
stream message IDs against completed messages in its retained transcript.
The analysis deduplicates by provider message ID before summing usage;
stream fragments can repeat a message and carry provisional output counts.

The root, Read-grant and native-tool experiments had no historical message
ID overlap on resume. Their result input, cache creation, cache read and
output token totals exactly matched completed retained messages identified
in that current stream. The root experiment reported one turn per invocation.
These observations establish a current-message boundary for those samples.

The Read-grant resume reported three turns but emitted two distinct assistant
message IDs. The MCP resume reported two turns but emitted one assistant
message ID; its token totals did not equal that visible message's completed
usage. An empty-response retry was visible. These differences are not proof
of replay or double billing; they mean visible message counts cannot be
assumed to define all provider turns or independently reconcile every total.
Do not subtract a guessed baseline or convert absent evidence to zero.

The full effective restriction/precedence matrix, filesystem boundary,
accounting attribution for these exceptional cases, Brokkr adapter behavior,
and other providers remain to be established. Tasks 10.5–10.8 and provider
enablement remain unchecked. DSH's supported headless resume route remains
unestablished; these Claude results do not supply it.
