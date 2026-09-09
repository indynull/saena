# Across projects

Project `alpha` needs a product from project `beta`. Each has
agents running. Alpha does not edit beta. Both sittings talk to
the same home desk.

## File in beta

From alpha, write the spec into a ticket on beta:

```sh
saena plan --project beta "need a list id"
```

The body is what beta will work from: need, ask, a pin or path
when the consumer is locked. File in the project that must
implement it.

## Beta runs the farm

Beta's inbox shows that ticket. Mint work, claim, spawn
children, complete. The parent deed cites the children.

```sh
saena plan list
saena work mint p-…
saena work claim w-… --as bob
saena work complete w-… --file list.rs
```

## Alpha waits on the work

Alpha's continuation depends on beta's work. Claim refuses
until that work is terminal. `wait` sleeps until then.

```sh
saena work "continue after list id"
saena work depend w-alpha w-beta
saena wait w-beta
saena deed get d-…
```

`wait` prints `done` and the deed id. `cancelled` is a bounce:
read the note and pick up the ticket filed back in alpha. Do
not treat cancel as a product.

The deed is already on the tree. Alpha cites `d-…` as a source
when it completes.
