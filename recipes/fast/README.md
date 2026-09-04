# Fast

The default Rust delivery recipe. Its verifier runs `cargo test
--workspace` and compiles `bundles/self`; its shipper renders the journal
with `brokkr ledger`. Both are deterministic boxed exec gates.

Cargo runs offline inside the box from the bound registry cache. If a
dependency is not cached, network remains refused, the command fails
closed, and the verifier's `fail` notes quote Cargo's decisive cache or
offline error.
