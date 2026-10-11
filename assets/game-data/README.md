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

All four are the v83 GMS files that HeavenClient also loads (see
`agent_docs/client.md`); the client's `nx/` directory can symlink to these.

`SHA256SUMS` pins the exact files the server and its tests were written
against. After copying files in, check them with:

```sh
tools/verify-assets.sh
```

The world server loads all of them at startup and refuses to start if any is
missing or unreadable. Unit tests that need them skip themselves (with a note
on stderr) when they are absent.
