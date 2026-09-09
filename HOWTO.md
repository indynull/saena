# How to sit

A Rust toolchain.

```sh
cargo build --release
./target/release/saena install
cd ~/src/widget
saena init
```

That puts `saena` on `~/.local/bin` and writes `.saena/project`.
The tree is the home desk (`$XDG_DATA_HOME/saena/desk`). Commit
`.grok/config.toml`, `.mcp.json`, and `.saena/project`. Open Grok
here.

`grok mcp enable saena` turns the server on for every folder:

```json
{
  "mcpServers": {
    "saena": {
      "command": "saena",
      "args": ["mcp"]
    }
  }
}
```

The server sits at the home desk. `complete` snapshots this
folder. Ready work is this project's inbox.

## First path

```sh
saena work "name the note"
saena work claim w-… --as alice
printf 'a named note\n' > note.txt
saena work complete w-… --file note.txt
saena deed get d-…
saena memory remember "always open the deed, not the chat"
saena sit
```

The next sitting is `saena deed get d-…`.

## Farm

```sh
saena work spawn w-… "child work"
saena work claim w-child --as child
saena work complete w-child --file child.txt --source d-parent
saena work link w-parent w-child
saena work verify
saena work list --all
```

## Plan

```sh
saena plan "catalog"
saena plan --parent p-catalog --type schema --tag keys --deadline 2026-09-20 "keys.toml"
saena plan block p-schema p-catalog
saena plan close p-catalog
saena plan list
saena plan agenda
saena plan export
```

## Across projects

From project `alpha`, file the request in `beta`. Beta mints
work, farms it, and names a deed. Alpha waits, then reads the
deed.

```sh
# in alpha
saena plan --project beta "need a list id"
saena work "continue after list id"
saena work depend w-alpha w-beta
saena wait w-beta
saena deed get d-…

# in beta
saena plan list
saena work mint p-…
saena work claim w-… --as bob
saena work complete w-… --file list.rs
```

## Memory

```sh
saena memory pin review
saena memory remember "always open the deed, not the chat"
saena search "deed"
saena memory cards
```

`USER.md` and `MEMORY.md` live beside the desk. Search merge is
the host panel: `SAENA_FUSE`, `SAENA_DIVERSIFY`, `SAENA_DECAY`.
Default is Borda then maximal marginal relevance.

## Deeds

```sh
saena deed quote --name "NLL" --excerpt "lifetimes from the CFG"
saena deed list
saena deed evidence d-…
saena deed trail d-…
```

`SAENA_HOST_KEY` (or `{desk}/../host.key`) stamps evidence.

## Surface

`saena surface` lists every sitting verb.
