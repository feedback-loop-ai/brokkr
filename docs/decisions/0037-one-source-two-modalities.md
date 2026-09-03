# 0037 — One source, two modalities: text is the record, the picture is the rendering

Status: proposed
Date: 2026-09-03

## Context

On 2026-09-03 the operator read `ARCHITECTURE.md` — 2,606 words and
not one diagram — and ruled the direction: more pictures, fewer words.
The page was rewritten the same day: nine diagrams, under 1,900 words.
The operator then asked for the principle behind the rewrite to be
captured, and stated it as: *humans and LLMs operate better in
different modalities — humans with visual artifacts, LLMs with text and
prose* — and asked to be grilled on it.

The claim was grilled. Where it holds:

- **Humans read topology spatially.** The `fast` table is twenty-two
  rules. As prose it is read in minutes and misremembered; as a state
  diagram the question "where can a run go from review?" is answered by
  adjacency in seconds. The machine already knew this: `brokkr inspect`
  renders the same topology as a `graph` block for exactly that reader
  (decisions 0013, 0014).
- **An LLM gets nothing from a raster.** A seat reads the tree with
  text tools. A fact that exists only in a PNG is a fact the machine
  cannot cite, check, diff or digest — which in this repository means
  it is not a fact at all.

Where it does not hold as stated:

- **"LLMs are better with prose" is wrong in the way that matters.**
  The 2,606 words were bad for the model too: the seven crates and
  their trust boundaries cost a seat thousands of tokens to establish,
  buried under qualifications, and the prose had already drifted from
  the code (it said six crates). What a model reads best is dense,
  structured, de-duplicated text: tables, schemas, rule lists — and
  diagram *source*. A mermaid state diagram is an edge list. It is
  closer to `policy.json` than to a paragraph, and a model reads it
  better than the paragraph it replaced.
- **Humans are not only visual.** The *why* — the context and the
  ruling's reason — is prose for both parties, and precision (which
  rule id, which condition, which threshold) is text. A diagram made
  to carry the precision stops being readable; the phase graph already
  strains at twenty-two rules, and the rule ids stay in the table.
- **The claim misses the danger.** A picture is trusted more than it is
  checked. An edge on a diagram that no test reads is an ungoverned
  claim on a living surface, in a repository whose whole culture is
  that every claim is journaled and checkable. Hand-drawn pictures
  drift exactly as the prose did, and drift in a picture is harder to
  see in a diff.

So the true split is not *text versus picture*. It is **source versus
rendering**: one text artifact in the repository, rendered as a picture
for the human and read as a graph by the model. Mermaid inside Markdown
is that artifact — GitHub, editors and the artifact pages render it,
`git diff` shows the edge that changed, and a test can parse it.

Alternatives weighed: committing rendered SVG or PNG beside the source
(two artifacts that drift from each other, and the raster wins in the
reader's eye); a diagram-only page with the prose moved elsewhere (the
reasons lose their home, and reasons are the part decisions are made
of); leaving the prose and adding pictures on top (the page grows, and
the same fact stated twice is the drift already described).

## Rulings

1. **One source, two renderings.** Every diagram on a living surface is
   text in the repository — mermaid inside the Markdown that explains
   it — rendered by whatever the reader is holding. No raster or
   hand-drawn binary is the source of a structural claim. The brand
   marks under `assets/` are not structural claims and are exempt.

   **Enforcement binding:** `crates/brokkr-cli/tests/diagrams.rs`
   walks the living prose surfaces and refuses a repository-hosted
   image embed — the Markdown image form or an HTML image tag — whose
   path is not under `assets/`.

2. **Pictures carry topology, tables carry precision, prose carries
   reasons.** A living page states each fact once, in the modality it
   belongs to: a structure with edges is a diagram, an enumeration is a
   table, a rationale is prose that cites its decision number. A page
   explaining a structure the machine owns leads with the picture.

   This ruling is judgment-guidance on which modality a fact belongs
   to; it has one determinable edge and that is bound: **enforcement
   binding:** the same test holds `ARCHITECTURE.md` under 2,000 words
   with at least six diagrams, and the front page with at least one.

3. **A diagram of a machine-owned structure is checked against the data
   it depicts.** A picture of a table, a dependency graph, a protocol or
   a schema is either generated from that data or asserted equal to it
   by a test, so a diagram cannot say what the code does not.

   **Enforcement binding:** the same test parses the phase-graph state
   diagram in `ARCHITECTURE.md` and asserts its edge set is exactly the
   `(from → next | park)` set of `recipes/fast/policy.json`, and parses
   the crate diagram and asserts every drawn edge is a declared
   `[dependencies]` entry and every workspace crate is drawn. A diagram
   of a structure that gains a check later is added to the same test.

4. **Rendering is the reader's, never the repository's.** No CI step
   renders diagrams to files, and no rendered file is committed. A
   page is complete when its source is; a reader whose surface does not
   render mermaid reads the edge list, which is the same fact.

   **Enforcement binding:** ruling 1's test — a committed rendering is
   a repository-hosted image outside `assets/`.

## Consequences

- `ARCHITECTURE.md` is the first page held to this: nine diagrams, the
  crate graph and the phase graph tested against the workspace and the
  recipe, prose under budget. The front page carries the bootstrap as a
  picture with the two measured budgets on it.
- Pages that explain a structure and carry no picture are now debt,
  not style: the quickstart's four-step spine, the driver protocol in
  the driver-authoring guide, the composition chain in recipe
  authoring. Each is a diff over this ruling, not a new one.
- A future generator that emits the phase diagram from `policy.json`
  (`brokkr compile --graph`, say) would satisfy ruling 3 by
  construction and retire that half of the test. Until it exists, the
  test is the generator's stand-in.
- The operator's stated claim is recorded above as stated, with the
  grilling beside it, so the ruling can be read back to the argument
  that produced it rather than to the slogan.
