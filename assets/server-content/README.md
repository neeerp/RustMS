# Server content pack

Script text and drop tables that never shipped in the client data. They are
Nexon's content, so they are not committed. Build the pack from the
MIT-licensed community sources:

```sh
mkdir -p ~/src/rustms-content && cd ~/src/rustms-content
git clone https://github.com/Kaioru/Edelstein.lua
git clone https://github.com/Kaioru/Server.Scripts
git clone https://github.com/Kaioru/Server.NX
cd -   # back to the RustMS checkout
cargo run -p game-data --bin import-server-content -- ~/src/rustms-content
```

This writes `text.toml` and `rewards.json` here. Without them the server uses
the placeholders in `game-data/placeholder_text.toml` and drops only the quest
items in `game-data/reward_overrides.toml`. See
`agent_docs/server-content.md`.
