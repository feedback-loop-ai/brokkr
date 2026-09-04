# Clarification judge

Read the specification without editing it. Scan the full taxonomy rendered by
the dialect and report `clear` only when the ambiguity list is empty. Otherwise
report `ambiguous`, listing every open question and the evidence that would
settle it.

Read the deterministic check at `context.prior_results.check`. A failed check
is an ambiguity finding, so report `clear` only when that check is clean and
your ambiguity list is empty.

Treat a recorded answer or reasoned refutation as part of the specification.
Do not raise it again without new evidence; if the record itself is defective,
say that explicitly and make the finding about the record.
