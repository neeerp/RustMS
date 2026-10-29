# Server content pack

Some v83 content never shipped in the client data: the dialogue of scripted
NPCs and portals, mob drop tables, and what NPC shops sell. In v83 these lived on Nexon's servers.
Like the NX files they are Nexon's content, so they stay **out of the
repository**. Only behavior is code; text and data come from an ignored
content pack, with committed fallbacks so a fresh clone runs and passes its
tests.

## Layout

| Path | Committed | Contents |
| --- | --- | --- |
| `assets/server-content/text.toml` | no | Script text, `"key" = "text"` |
| `assets/server-content/rewards.json` | no | Drops per mob or reactor id (`item`/`money`, `prob`, `min`/`max`) |
| `assets/server-content/shops.json` | no | Shop rows per NPC id (`item`, `price`) |
| `assets/server-content/gachapon.json` | no | Gachapon prizes: `global` and `machines` (by NPC id), each with `common`, `uncommon` and `rare` lists of `{item, count}` |
| `game-data/placeholder_text.toml` | yes | A placeholder for **every** text key, written by us |
| `game-data/reward_overrides.toml` | yes | v83 corrections, e.g. quest drops |
| `game-data/gachapon_placeholder.toml` | yes | Our stand-in Gachapon prizes, in the same shape |
| `game-data/shop_additions.toml` | yes | Shops the pack lacks: rows of `[item, price]` per NPC id |
| `game-data/server_content.toml` | yes | Where the importer finds each key, plus its SHA-256 |

`RUSTMS_SERVER_CONTENT_PATH` overrides the pack directory (default
`assets/server-content`; the harness mounts `assets/` at `/app/assets`).

## Fallbacks

- **Text** (`game_data::server_content::ServerText`): each key resolves from
  the pack, otherwise from its placeholder. Placeholders start with
  `[placeholder]` and only describe what happens. Scripts reference keys
  (`server_content::keys`), never literal dialogue; a unit test fails if a key
  has no placeholder. `{name}` placeholders are filled at runtime.
- **Drops** (`RewardTable`): without `rewards.json` mobs drop nothing, except
  the committed overrides. `replace = [...]` replaces a mob's drops and
  `add = [...]` appends to them; both are facts from our own assets, e.g.
  Tutorial Jr. Sentinel → 4031802 for quest 1035 (`Check.img/1035/1/item`),
  since the imported v95 data names another item, and Manon/Griffey add the
  pirates' 4031860/4031861 for 6944, which the pack lacks.
  `quest = <id>` limits a drop to players with that quest started, and
  `quest = 0` makes a quest item an ordinary drop (MV's Lair's chests'
  Persimmons and Chestnuts, which only a dated-out quest asks for). Premium
  (cash) drops are filtered out.
- **Reactor drops** (`RewardTable::reactor_drops`): `rewards.json` mixes
  reactor ids in with the mobs. `[reactors.<id>]` overrides add to the
  pack's drops rather than replace them, unless `replace = [...]` names
  the whole table (the Orbis records and stone boxes): `like = <id>` borrows another
  reactor's drops (Amherst's box 2001 has none of its own, so it borrows
  2000's), and `quest_items` makes items quest drops that always drop during
  that quest (for a player who still needs them, or anyone in a party),
  adding them if the pack lacks them. So Pio's materials for quest 1008 drop
  from the boxes even without a pack.
- **Shops** (`ShopTable`): the pack (from Server.NX, a v95 dump) has no
  shop for 13 v83 shopkeepers, among them Morgan in Nautilus Harbor, Mayoren
  in Amoria, and the Singapore and Malaysia vendors. `shop_additions.toml`
  gives them their item ids and prices, in shop order, as facts taken from
  Cosmic's shop tables; a shop the pack has wins. Without `shops.json` only
  these 13 keep a shop, and other shopkeepers fall back to their normal
  speech.
- **Gachapon prizes** (`GachaponTable`): v83 kept them on the server, and
  the pack has none. Without `gachapon.json` (or if it doesn't parse) every
  machine draws from `gachapon_placeholder.toml`, a few potions, ores,
  scrolls and a sword we picked (a recommended default,
  `v83ref/lanes/QUESTIONS.md`). A machine draws from its own lists plus the
  global ones.

The world server logs what it found at startup:

```
Server content: assets/server-content: 63 text keys from the pack, 0 placeholders; drops for 646 mobs from the pack; 110 shops; Gachapon prizes from the stand-ins
```

## Importing

Sources (MIT, Copyright Kaioru): [Edelstein.lua](https://github.com/Kaioru/Edelstein.lua),
[Server.Scripts](https://github.com/Kaioru/Server.Scripts) (scripts), and
[Server.NX](https://github.com/Kaioru/Server.NX) (`Reward.img`, `NpcShop.img`). They target
v95; behavior differences are handled in code and overrides, never by
editing the text. Clone them into one directory and run:

```sh
cargo run -p game-data --bin import-server-content -- <sources dir> [pack dir]
```

For each key in `server_content.toml` the importer reads the Lua file, takes
its string literals in order (`game_data::lua_strings`), fills the key's
`template` (`{sN}` is the Nth literal), and checks the SHA-256. A mismatch
means the upstream text changed: review it, then update the hash. Drops are
copied from `Server.NX/src/Reward.img/*.json` into one `rewards.json`, and
shops from `Server.NX/src/NpcShop.img/*.json` into one `shops.json`. The
harness only has shops if its assets directory has the pack.

## Adding text

1. Add a constant to `server_content::keys` and to `keys::ALL`.
2. Add a placeholder line to `placeholder_text.toml`.
3. Add the source, template, and hash to `server_content.toml`.
4. Use the key from the script (`content::text().get(key, &vars)`).

## Tests

Unit tests cover placeholder coverage, pack overrides and unknown keys, the
quest-drop override, and Lua literal extraction. Behavior tests (unit and
integration) assert behavior and keys, not text, so they pass with or
without a pack.
