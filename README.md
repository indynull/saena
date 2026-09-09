# Saena

A desk. A person sits with one tree. Agents claim on the live
graph. A sitting ends by naming a deed. The next sitting opens
that id.

Every checkout talks to the same home desk. A project file names
the checkout. Work filed in another project is that project's
inbox. The requester waits on the deed.

## Install

```sh
cargo build --release
./target/release/saena install
saena init
```

`saena` lands on your path. `init` writes `.saena/project` and
opens the home desk (`$XDG_DATA_HOME/saena/desk`). Commit
`.grok/config.toml`, `.mcp.json`, and `.saena/project`. Open Grok
here.

## First path

```sh
saena work "name the note"
saena work claim w-… --as alice
printf 'a named note\n' > note.txt
saena work complete w-… --file note.txt
saena deed get d-…
saena sit
```

The next sitting is `saena deed get d-…`. Same id, same bytes.

## Sittings

| You want | Start here |
| --- | --- |
| Name a product in this folder | [One checkout](docs/one-checkout.md) |
| Several agents on one job | [Farm](docs/farm.md) |
| A plan that outlives a sitting | [Plan](docs/plan.md) |
| Keep a lesson | [Memory](docs/memory.md) |
| Many checkouts, one tree | [Projects](docs/projects.md) |
| Project A needs a deed from B | [Across projects](docs/across.md) |
| Open, check, or follow a deed | [Deeds](docs/deeds.md) |

The book: [docs/README.md](docs/README.md). How to sit:
[HOWTO.md](HOWTO.md). Two Grok pastes: [DEMO.md](DEMO.md). The
law: [VISION.md](VISION.md). Agents in this checkout:
[AGENTS.md](AGENTS.md).

## Check

```sh
just check
```

## License

[MIT](LICENSE). Copyright 2026 Ali-Akber Saifee.
