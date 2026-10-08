# Panel review

This recipe adds independent correctness and security judges to the
delivery loop. It extends `fast` (#359): an intake phase entered first,
the library's implementer, and the review panel; its verifier and
shipper are `fast`'s own exec gates, inherited, so it runs every check
`fast`'s verifier runs. Seven of `fast`'s rules are restated to carry
this table's own reason wording until the operator rules which wording
stands.

The verifier tells Cargo to stay offline, so it reads only the bound
registry cache; an uncached dependency fails verification closed, and the
result notes quote Cargo's decisive error. Both gates are boxed with no
network under a `namespace` boundary. A `harness` realm refuses this
recipe: the `review:correctness` seat's second link resolves to claude,
whose adapter declares no `hands.harness.gate` fragment, and compilation
refuses it naming that seat.
