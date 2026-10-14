# Server game data

The world server reads static game data straight from the client's v83 NX
files, so server and client cannot disagree about maps, NPCs, or quests. The
files are converted Nexon WZ data: they are not in git and must not be
redistributed.

| File | Contents used by the server | Override |
| --- | --- | --- |
| `Map.nx` | Fields, portals, return maps, map NPCs | `RUSTMS_MAP_NX_PATH` |
| `Npc.nx` | NPC speech keys | `RUSTMS_NPC_NX_PATH` |
| `String.nx` | NPC speech text | `RUSTMS_STRING_NX_PATH` |
| `Quest.nx` | Quest templates | `RUSTMS_QUEST_NX_PATH` |
| `Item.nx` | Item templates (stack sizes, quest flags, consumable effects) | `RUSTMS_ITEM_NX_PATH` |
| `Character.nx` | Equipment stats and requirements | `RUSTMS_CHARACTER_NX_PATH` |
| `Mob.nx` | Mob stats (HP, EXP, attack, touch damage) | `RUSTMS_MOB_NX_PATH` |
| `Skill.nx` | Skill max levels, master levels, and requirements | `RUSTMS_SKILL_NX_PATH` |
| `Reactor.nx` | Reactor states and what hits them (boxes) | `RUSTMS_REACTOR_NX_PATH` |

All of these are the v83 GMS files that HeavenClient also loads (see
`agent_docs/client.md`); the client's `nx/` directory can symlink to these.

`SHA256SUMS` pins the exact files the server and its tests were written
against. After copying files in, check them with:

```sh
tools/verify-assets.sh
```

The world server loads all of them at startup and refuses to start if any is
missing or unreadable.

Unit tests that need these files skip themselves (with a note on stderr) when
they are absent, so `cargo test` passes on a fresh clone. That also means a
run without the files does not exercise them. When the files are present, run
the asset lane strictly so a missing file fails instead of being skipped:

```sh
RUSTMS_REQUIRE_ASSETS=1 cargo test -p game-data
```
