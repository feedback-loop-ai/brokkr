# Panel review

This recipe adds independent correctness and security judges to the
delivery loop. Its shipper is the same boxed exec gate as `fast`'s.

Its verifier is its own copy of `fast`'s from before #427: it runs
`cargo test --workspace` and the `bundles/self` compile only, not
`cargo fmt`, clippy with `-D warnings` or `scripts/lint-non-rust.sh`.
The recipe does not extend `fast`, so it did not inherit them, and its
judges are the change here, not the verifier; the pull request's
required checks still run them before a merge.

Cargo runs offline inside the box from the bound registry cache. If a
dependency is not cached, network remains refused, verification fails
closed, and the result notes quote Cargo's decisive error.
