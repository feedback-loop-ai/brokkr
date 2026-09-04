# Panel review

This recipe adds independent correctness and security judges to the
delivery loop. Its verifier and shipper are the same boxed exec gates as
`fast`.

Cargo runs offline inside the box from the bound registry cache. If a
dependency is not cached, network remains refused, verification fails
closed, and the result notes quote Cargo's decisive error.
