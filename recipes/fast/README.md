# Fast

The default delivery recipe: implement, verify, review and ship. Its
verify and ship are deterministic exec gates, the folder's own (below).
Its shipper renders the journal with `brokkr ledger`. Brokkr boxes both,
with no network, where the realm's boundary is `namespace`; under
`harness`, which `brokkr init` writes on macOS, each runs unboxed in a
rebuilt environment and no network denial is reported. Brokkr's own
verifier, which every Brokkr-only recipe composed from `fast` runs, is
[`brokkr-fast`](../brokkr-fast/README.md)'s.

Its two model seats hire library offices (the operator's ruling of
2026-10-07, #360). `implement` seats `fast-implementer`, an overlay of
`implementer` on the intakeless implementer charter, and `review` seats
`fast-reviewer`, an overlay of `reviewer` that declares the review's
inputs. Neither writes a chain, tools or hands, so each hires its base's
(decision 0087 rulings 2 and 3): in a folder `brokkr init` made, what
`init` wrote for the agent CLI it found. Brokkr's own runs hire fast's
Brokkr picks through [`brokkr-fast`](../brokkr-fast/README.md), which
`landing` and the wager harnesses extend.

A `blocked` implementer says what blocked it in `inputs.blocked_by`
(decision 0092). `IMPL-BLOCKED-ENVIRONMENT` parks an `environment` block
(a signing key, a tool, a refused capability) with the seat's notes as the
reason, so the operator fixes it and `brokkr operator retry` re-runs that
seat with the earlier results intact. A `work` block, and a block that
names neither, stop hard at `IMPL-BLOCKED` as before. Every recipe that
extends `fast` inherits both rules.

**Verify and ship are the folder's own** (decision 0091). `fast`
installs onto any folder's shelf (`brokkr recipes add
<checkout>/recipes/fast --name fast`), and there its verify and ship
seats say `"from": "folder"`: each runs the seat the folder declares in
`.brokkr/bundle.json`, the script and box `brokkr init` wrote for the
stack it found, pinned with the folder's layer and walked at every spawn.
With no folder, fast's own declarations stand. Its verify then fails
closed, having run no check, and names `brokkr init` or a recipe that
declares its own verify seat as the remedy; its shipper is its own,
`scripts/ship-seat.sh`.
