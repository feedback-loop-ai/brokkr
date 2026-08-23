# 0007 — Input provenance: declared, engine-owned, or dropped

**Status**: accepted (operator goal directive, 2026-08-23 — enumeration by
Claude under that directive)

## Ruling

Every evaluation input has exactly one declared provenance, closed at
compile time:

1. **Engine-owned** (`consecutive_failures`, `drift_detected`,
   `dirty_worktrees`, `reviewed_heads`): journal-computed, overlaid over
   anything a seat claims, and never declarable by a seat.
2. **Seat-declared**: a seat may supply only the typed facts it declares
   (`seats.<phase>.inputs`); the declaration may name only known,
   non-engine-owned evaluator inputs. The default declaration is the set
   of non-engine-owned inputs the phase's own rules reference.
3. **Everything else is dropped** before evaluation — an undeclared claim
   never reaches the table and never enters the `transition/decided`
   record. The journal states only facts something was entitled to assert.

`forge compile` rejects a declaration naming an engine-owned or unknown
input, and rejects a policy whose rule references an input its phase's
seat cannot supply — a rule that could never fire from seat data is dead
policy, and dead deny rules are the exact defect decision 0004's typo
incident recorded.

## Why

This closes the loop 0004 deferred ("noted for the port"): the evaluator
guarantees absence is never an *advantage*; provenance now guarantees
presence is never an *imposture*. Together with the closed condition
vocabulary, every fact the table can read is either computed by the
engine from the journal or explicitly granted to a named seat in the
reviewed, content-addressed bundle.

## Consequences

- Existing bundles keep working: the implied declaration (rule-referenced
  inputs) is what well-behaved seats already supplied.
- Machine proof grows: undeclared seat claims never reach the journal;
  the three provenance compile rejections.
- The self bundle adopts explicit declarations alongside its 0006 limits
  once the in-flight ship-taxonomy delivery lands.
