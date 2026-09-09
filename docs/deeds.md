# Deeds

A deed is a frozen named product. Completing producing work
creates one. Typed create (`saena deed quote`, `patch`, …) is
the same node kind.

```sh
saena deed get d-…
saena deed evidence d-…
saena deed trail d-…
saena deed current d-…
saena deed leave d-… ./out
saena deed timestamp d-…
```

`evidence` checks the bytes and the host stamp. `trail` walks
`sources`. `current` follows `supersedes`. `leave` copies the
product out. `SAENA_HOST_KEY` (or `{desk}/../host.key`) stamps
evidence.
