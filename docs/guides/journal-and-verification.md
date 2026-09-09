# The journal and verification

The journal is an append-only, hash-chained SQLite table. State is the
fold of it; nothing else is authoritative.

```
$ brokkr anchor --run latest                     # record the head in refs/forge/<run>
anchored prefix-selectors-for-the-read-su-8bf6d692 at 94c5bd9dff99bef4d4b9d224d8cc1661681fd194

$ brokkr anchor --run latest --check             # tamper evidence, re-checked
$ brokkr export --run latest --out ./out --redact  # plus a marked publishable derivative:
#   <run>.redacted.ndjson — paths and usernames as stable placeholders, hashes
#   verify only on the verbatim pair, and the manifest says so
{
  "chain_length": 2,
  "journal_head_hash": "74d186fb254b62fb486b3f5f3fe1e1ad7c91fa5deef2d850d60c16e5478be918",
  "repo_head": "c9f1…",
  "ref": "refs/forge/prefix-selectors-for-the-read-su-8bf6d692",
  "seq": 38,
  …
}

$ brokkr export --run latest --out ./out         # canonical NDJSON + pinned manifest
exported ./out/prefix-selectors-for-the-read-su-8bf6d692.ndjson

$ brokkr verify-run ./out/prefix-selectors-for-the-read-su-8bf6d692.ndjson
{
  "chain": "verified",
  …                                              # envelopes and fold, offline
}

$ brokkr replay --run latest                     # rebuild twice, compare
{
  "chain": "verified",
  "events": 38,
  "replay": "deterministic",
  …
}

$ brokkr import --from ./out/prefix-selectors-for-the-read-su-8bf6d692.ndjson \
      --db ./canonical/forge.db                  # adopt that run into another journal
{
  "adopted": "byte-identical",
  "chain": "verified",
  "events": 38,
  "imported_from": "./out/prefix-selectors-for-the-read-su-8bf6d692.ndjson",
  …
}
```

The anchor commit's tree carries `<run>.ndjson`; publishing it as
`refs/heads/brokkr-runs/<run>` gives pull-request CI a canonical export
to verify offline and a `repo_head` to compare with the proposed head.

`import` is the verb paired with `export`: **journals never merge, but
one run can relocate.** Nothing lands until the whole chain re-verifies
and the events fold — one broken link refuses the entire import, never a
good prefix of it — and a run_id that already exists in the destination
is refused outright. There is no rename and no overwrite: the run_id is
hashed into every envelope of its chain, so a collision is structurally
not a rename-and-retry, and it is the operator's to rule on. A second
import of the same export refuses the same way, because adoption is once
and not idempotent. A `--redact`ed derivative is refused by name, under
no flag: redaction rewrites payload bytes and leaves the recorded hashes
behind, so importing one could only ever adopt unverifiable content. And
the run_id itself must be one this journal would have minted — ASCII
letters, digits, `-` and `_`: event hashes are unkeyed, so a verified
chain proves its bytes were not altered and never that whoever sealed
them was entitled to the name, and the name goes on to be a path
component the next `export` writes.

The adopted events keep their exact bytes, hashes, seqs and `recorded_at`,
so `brokkr runs`, `brokkr tui` and `brokkr inspect` render the run
indistinguishably from one driven there natively. Where it arrived from
and when is store bookkeeping *beside* the chain — two columns on the
`runs` table, invisible to `fold` — so `state = fold(events)` holds
identically for a native run and an adopted one
([decision 0027](../decisions/0027-import.md)).

A journal cites git SHAs — the head it reviewed, the head it shipped —
and squash-merge, branch delete and `git gc` would collect them. Every
run plants **keep-refs** at conclusion: `refs/forge/keep/<run>/<sha>`,
one per cited object, so the exhibits outlive the branch that carried
them ([decision 0028](../decisions/0028-keep-refs.md)).

```
$ brokkr keep-refs list                          # which runs hold which exhibits
{
  "keep": {
    "keep-refs-the-journal-s-exhibits-c78fb73e": ["c9f1…", "3ad0…"]
  }
}

$ brokkr keep-refs plant --run latest            # idempotent; for runs that predate the mechanism
$ brokkr keep-refs delete --run <id>             # the exhibits may go — the operator's call alone
```

A run parked on **`GATE-MOVED-HEAD`** is saying the repository head moved
across a gate's own span: a gate reads and reports, it never writes
([decision 0041](../decisions/0041-one-office-per-seat.md)). The check is
per gate step, not per effect — the head is observed when a gate step
starts, or when a gate seat that has no steps of its own starts, and it
is compared at that same step's end. So an author that lawfully commits
before or after a gate in one sequence is never charged to the gate, and
only the step that moved the tree parks. The park's `evidence` carries
both raw observations, `head_at_start` and `head_at_end`, and the effect
ends `indeterminate`: what a gate did after it wrote is exactly what the
engine will not guess at.

`brokkr costs --run <id>` reports per-seat attempts, turns and USD — the
LaneTally join surface, computed from journal checkpoints with stable
seat ids.

## The boundary on the record, and the word *unboxed*

Every seat record that names a `model` carries `boundary` beside it
(`seat-record.v4`, decision
[0046](../decisions/0046-the-boundary-is-named.md) ruling 3): one of
`namespace`, `seatbelt`, `container`, `harness` or `open` — the realm's
word for what stood between that seat's hands and the machine — or
decision 0031's sentinel `not applicable` for a site that declares no
hands. The engine stamps it, never the driver: it is the only party
that knows which boundary it built, and a value a driver wrote is
replaced. `effect/started` carries the same fact for the whole attempt
as an additive extension field, `boundary`
(`effect-boundary.v1.schema.json`): a list with one entry per
invocation site — `member`, the site's tag as checkpoints and
provenance already use it; `boundary`, the word; `gate`, whether the
site is gate class — present only when at least one site of the attempt
declares hands, and never read by `fold`, so a run over a bundle that
boxes nothing journals byte-identical payloads.

The data carries the plain word; the screen carries the adjective. A
run in which any gate-class site stood under `harness` or `open` is
rendered **unboxed** wherever the run is summarised: the run header
prints the word and the adjective together (`harness · unboxed`), the
seats table of `brokkr inspect` and the `brokkr seats` verb show a
`boundary` column beside `model`, the TUI, the web console and
`costs`/`compare` show the boundary beside the model, and the delivery
gate's check summary on the pull request (decision 0038) appends
` · unboxed`. What *unboxed* states is exactly what the record can
prove: nothing of Brokkr's stood there. It makes no claim about the
network — on Linux the engine attempts a narrowing around an unboxed
exec dispatch and reports nothing about whether it held — and no claim
that the harness's own sandbox held, which is the harness's fact. A journal written before the word existed renders an
explicit absence, `no boundary recorded`, on every model row and
` · boundary not recorded` in the check summary, never a default: an
old run is not retroactively declared boxed.

Four verification layers back all of this, each mechanical: the 97-case
differential corpus pins the evaluator; the machine-proof suite drives
the real binary through every failure mode (30+ scenarios, three OSes,
coverage-gated CI); self-forge runs deliver changes under the full
constitution; and the verify agents adversarially review every landed
slice — their verdicts are journaled runs like any other.

Release admission additionally requires canonical formatting, warning-free
Clippy across all targets and features, a RustSec dependency audit, literal
nonzero 100% source-line/branch/function coverage, frozen/additive contract
compilation, producer-bridge conformance, and a checksum-verified platform
matrix. Release archives and `SHA256SUMS` carry GitHub Sigstore build-provenance
attestations; verify an asset with
`gh attestation verify <asset> -R feedback-loop-ai/brokkr`.
