# 0062 — Bound the transcript projector

Status: proposed
Date: 2026-09-17

## Context

Decision 0055 ruling 3 mapped one logical event to one displayed turn and
gave the display budget as a sum of emitted block text bytes. Issue #277
shows that mapping is not a memory bound: a bounded 32 MiB DSH session
whose streaming rows pack one member per token expands into millions of
retained projector events on the operator's own console, and a flood of
tiny ordinary events can spend the text-only budget without spending the
storage the console must hold. The operator's 2026-09-17 ruling authorizes
both a merge of consecutive packed members and a fixed structural charge.
This proposal supplements 0055 ruling 3's turn mapping and text-only
budget without editing its history, and leaves 0061's version, seeded and
recorded-token admission exactly as it is.

The adopted specification and its scenarios are
`openspec/changes/2026-09-17-bound-transcript-projector/`, whose reading
delta owns the merge rule once and whose command/TUI deltas consume it.
The design `design.md` sections D1-D9 record the complete-prefix fact pass,
the bounded projection pass, the shared charge, the blockless guard and the
two low repairs.

**Rulings.**

1. **Consecutive packed members coalesce into one displayed chunk.**
   After member-level citation suppression, each maximal consecutive run of
   unsuppressed nonempty members within one validated version-zero
   `text-chunks` or `reasoning-chunks` physical row is one assistant chunk
   turn, one block of that row's kind, and the member texts concatenated
   exactly in stored order. A suppressed readable member splits the run;
   empty members supply no text or turn and do not split it. The chunk's
   position and timestamp are its first surviving nonempty member's; later
   timestamps never reorder text or create turns. Coalescing never crosses
   a physical row, an ordinary event row, a block kind, or a recorded
   turn/step/index. Ordinary rows keep their individual turns, so packed
   and ordinary encodings can now differ in turn count while keeping the
   same content and citation outcomes.
2. **The display budget is display accounting bytes shared by all kinds.**
   The numeric `DISPLAY_CAP` stays 4,000,000. Each final content-bearing
   turn costs `DISPLAY_EVENT_COST = 512` structural accounting bytes plus
   the sum of its emitted blocks' UTF-8 text bytes; a blockless event costs
   zero and supplies no turn. A coalesced packed chunk pays once as its
   final turn, not once per token, and no block count, provider role or
   `--turn` selection waives the charge. The 512 constant roundly exceeds
   the approximately 330-byte per-event audit in issue #277; it is a
   portable accounting allowance, not a measured allocator layout or a
   whole-process RSS ceiling. Equality fits, checked arithmetic never wraps
   into admission, and the first over-budget turn seals the prefix without
   skipping forward to a smaller successor.
3. **DSH projection bounds retention before returning.** A read visits the
   complete admitted prefix for classification, validation, diagnostics
   and association facts, then projects the same snapshot into a bounded
   collector: accumulated content-bearing DSH turns stay within the shared
   budget, with at most one additional current candidate, throughout the
   read. A candidate that cannot fit is discarded whole. Packed coalescing
   constructs one candidate per surviving run and never materializes a
   payload-bearing event per member, even transiently. Compact identity,
   citation and dedicated-tool facts may scale with the actual bounded
   source; they hold no copied content and never scale with an unobserved
   sequence gap.
4. **A blockless DSH event is observed, not retained.** Every recorded
   sequence is observed before a blockless `tool/result` or `user/message`
   row is released, so duplicate identity, citation ambiguity, diagnostics
   and source order are unchanged while no empty payload slot survives the
   pass. The final empty-turn conversion stays as a defensive rule.
5. **The header bound is exact at true EOF.** Discovery admits at most
   65,536 opening-header bytes and refuses 65,537 with `discovery-limit`,
   including a newline-less header at true EOF; the delimiter is never an
   admitted header byte. The read's overflow probe alone is not the
   header-length predicate.
6. **0061 is untouched.** The admitted version set `{0, 3}`, the
   seeded-association gate, the per-version vocabulary, the recorded-token
   exactness rule and every version-three projection remain exactly as
   decision 0061 and the living reading specification state them.

Enforcement bindings: the packed-coalescing, indivisible-chunk,
structural-charge, complete-prefix-refusal, blockless-neutrality,
header-boundary, Codex association-sharing and full-boundary tests named
in the change's scenarios; `openspec validate --all --strict`; and the
literal exact-coverage gate. Each claimed bound has an independent
compiled removal that fails its intended assertion.

## Consequences

Displayed packed chunk counts and CLI/TUI one-based indices change
intentionally; no stored source, journal or document field migrates, and
refresh already invalidates a selection whose projected prefix changed.
The change is confined to `crates/brokkr-view/` and the named
`crates/brokkr-cli/src/ui.rs` header boundary; it adds no dependency,
public document field or reader API. It does not promise that process RSS,
arbitrary block arrays, JSON parsing transients or non-DSH intermediate
storage fit in four million bytes; those remain measured residuals. Only
the operator accepts this proposal, and filing it certifies no host
coverage, remote CI or publication.
