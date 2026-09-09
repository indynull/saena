# Farm

One parent, several children, several agents on the same desk.

```sh
saena work "ship the app"
saena work claim w-parent --as alice
saena work spawn w-parent "store"
saena work spawn w-parent "ui"
saena work spawn w-parent "persist"
```

Each child claims one ready node and completes it. `work link`
makes a hard dependency: the child is not claimable until the
parent is terminal. `work verify` walks those links.

[DEMO.md](../DEMO.md) is this sitting as two Grok pastes: three
agents in one folder, then a second sitting on the tip deed.
