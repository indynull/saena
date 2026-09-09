# Protocol

The Model Context Protocol server is `saena mcp`. The tools are
the same sitting verbs as the command: `status`, `work`,
`claim`, `complete`, `plan`, `mint`, `depend`, `wait`, `get`,
and the rest of `saena surface`.

Call `status` first. Do not ask for `SAENA_DESK` or file paths.
File work for another project with the `project` argument. Mint
work from that ticket. `depend` so this sitting waits. `wait`
until the node is terminal, then `get` the deed.

`complete` snapshots the checkout that holds `.saena/project`.
The next sitting opens the tip deed.
