# Plan

A ticket is a durable request. It outlives the sitting. Agents
do not claim tickets. The host mints work from a ready ticket.

```sh
saena plan "catalog"
saena plan --parent p-catalog --type schema --tag keys --deadline 2026-09-20 "keys.toml"
saena plan block p-schema p-catalog
saena work mint p-schema
saena plan list
saena plan agenda
saena plan close p-catalog
```

`block` means this ticket waits until the other closes. Ready
is that walk. `note` is a one-line ping. `append` is the written
result. `reject` / `resolve` point at a replacement id.

`plan export` prints tickets as JSON lines. `roadmap`, `graph`,
and `digest` are views of the same list.
