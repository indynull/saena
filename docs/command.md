# Command

`saena surface` lists every sitting verb. Groups:

- `status` / `sit` / `search` / `open` / `wait`
- `work` — create, claim, spawn, complete, mint, depend, link,
  list, verify
- `deed` — get, list, typed create, evidence, trail
- `plan` — create, list, block, close, agenda, export
- `memory` — remember, prefer, pin, cards
- `init` / `install` / `mcp` / `surface`

`--desk PATH` opens that store and skips the project file.
Without it, the process opens the home desk and the nearest
`.saena/project`.

`--project NAME` on `work` or `plan` files into another inbox.
`work mint TICKET` copies that ticket's project onto the work.
`work depend ID OTHER` makes `ID` wait until `OTHER` is
terminal. `wait ID` sleeps; `--timeout-ms N` gives up.
