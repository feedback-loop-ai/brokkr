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

`brokkr costs --run <id>` reports per-seat attempts, turns and USD — the
LaneTally join surface, computed from journal checkpoints with stable
seat ids.

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
