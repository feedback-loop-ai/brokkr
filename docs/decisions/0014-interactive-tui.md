# 0014 — `forge tui`: an interactive, read-only console in the terminal

Status: accepted (operator ruling in chat, 2026-08-29)

## Context

Decision 0013 gave the terminal two static readouts and a live one, and
explicitly put panes and keyboard navigation out of scope pending their
own decision. Using them proved the gap: `forge watch` answers "what is
happening now" but cannot be *explored*. The operator wants what Claude
Code offers for subagents — a navigable table of the fleet, arrow keys,
drill into one participant, come back — in the terminal, where the
operator sits while a forge runs.

The models already exist: `forge-view` derives them and 0013 promised
that "a future TUI, if ever wanted, renders the same models". This
decision spends that promise.

## Decision

**`forge tui` is a third renderer over the same `forge-view` models** —
never a fourth derivation. It is **read-only**, exactly as decision
0003 rules for every viewing surface: it issues no operator commands,
starts no runs, and writes nothing to the journal. `forge operator
retry|stop` stay CLI verbs precisely because they are consequential.

### Shape

Three levels, each a pane over models that already exist:

1. **Runs** — the run list: id, status, phase, seq, age, clamped
   feature. Selecting one descends.
2. **Run** — the phase graph as the tree from 0013 (`⑂` parallel, `→`
   sequential), the seats table (participant, status, attempts, turns,
   cost, activity), and the decision trail. Scoping by phase or by
   participant is selection, mirroring the console's exclusive scopes.
3. **Participant** — the seat's stream: its checkpoints, and its
   session transcript where one exists locally, with the
   `claude --resume <id>` line for the full session.

### Keys

Arrow keys and `j`/`k` move; `Enter` descends; `Esc`/`Backspace`
ascends; `Tab` cycles panes; `g`/`G` jump to top/bottom; `/` filters
the focused list; `r` forces a refresh; `?` opens help; `q` quits.
The exact map is the implementation's to finalize, but it MUST be
discoverable — a persistent footer naming the keys in context, and a
help overlay.

### Liveness

The view refreshes when the journal head moves, using the same polling
`forge watch` established. A live participant shows its current tool
and target; a concluded one shows result and duration. Selection
survives refreshes, and clears itself when its subject disappears —
the rule the console already follows.

### Dependency ruling

**`ratatui` and `crossterm` are adopted**, an explicit and narrow
exception to the no-new-dependencies default that governed 0012 and
0013. Raw-mode entry, key decoding, resize handling and the Windows
console API cannot be hand-rolled responsibly — CI tests Windows, and
a `stty`-shelling hack would be both fragile and unportable. They are
the de-facto standard for the task, permissively licensed, and the
RustSec audit job in CI covers the added tree. They may be used ONLY
on the TUI path: no other crate gains a terminal dependency, and
`forge-view` stays free of rendering concerns entirely.

## Constraints

- Key handling is a **pure state machine** over view models plus a key
  event, unit-tested headlessly — no terminal required to prove
  navigation. Rendering is tested through `ratatui`'s `TestBackend`
  into a buffer, so the TUI lands under the coverage gate like every
  other surface.
- Terminal safety from 0013 is inherited: journal text is
  seat-authored, so every string is sanitized before it reaches a
  cell, and the widths used for layout are computed on the sanitized
  text.
- Not a tty, or a terminal too small: exit with a clear message
  naming `forge inspect` and `forge watch`, never a corrupted frame.
- Raw mode is always restored — on quit, on error, and on panic.

## Consequences

- Three surfaces (console, CLI readouts, TUI) and one derivation; the
  invariant of 0013 holds by construction.
- The forge gains a dependency tree it did not have. That is the price
  of the capability, ruled deliberately here rather than smuggled in.
- Operator actions remain deliberate CLI verbs, so the read-only
  boundary of decision 0003 survives the arrival of interactivity.
