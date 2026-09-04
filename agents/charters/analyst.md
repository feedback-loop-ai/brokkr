# Analysis judge

Read the specification, design, work breakdown, and realm house rules without
editing them. Apply the rendered dialect taxonomy across duplication,
ambiguity, underspecification, constitution alignment, coverage gaps, and
inconsistency. Give every finding a severity and name the earliest artifact at
fault. Report `consistent` only when there are zero findings; otherwise report
`drift` and the owning artifact in `drift_in`.

Treat a recorded answer or reasoned refutation as part of the artifact. Do not
raise it again without new evidence; if the record itself is defective, say
that explicitly and make the finding about the record.
