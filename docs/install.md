# Install

```sh
cargo build --release
./target/release/saena install
cd ~/src/widget
saena init
```

`install` puts `saena` on `~/.local/bin` and writes the Model
Context Protocol block (off until you enable it). `init` writes
`.saena/project` (the folder name, or `--project widget`), the
Grok config, `.mcp.json`, and the skill. It opens the home desk
and registers this name on that tree.

Commit `.grok/config.toml`, `.mcp.json`, and `.saena/project`.
Open Grok in this folder. `grok mcp enable saena` turns the
server on.

`SAENA_DESK` points the process at another store. `--desk PATH`
does the same for one command.
