# Spec-driven delivery

This recipe adds a design sequence and spec checks before implementation.
Verification and shipping remain deterministic boxed exec gates.

Cargo runs offline inside the box from the bound registry cache. If a
dependency is not cached, network remains refused, verification fails
closed, and the result notes quote Cargo's decisive error.
