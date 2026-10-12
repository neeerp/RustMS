# Decision: how NPC, quest, portal, and event behaviour is expressed

Status: **proposed** (needs an owner decision before more content is added)

## Problem

v83 behaviour beyond movement and chat is mostly content: thousands of NPC
conversations, quests, portal scripts, reactors, and events. Today each piece
is hand-written Rust: Rain's quiz and the Heena/Nina contact quests are
special-cased by quest and NPC id (`game-data/src/quests.rs`,
`net/src/quests.rs`, `runtime/src/actor/client/dialogue.rs`), and portal
`script` fields are parsed and ignored. That does not scale past a handful of
quests.

## What the data already covers

`Quest.wz` describes most quests declaratively: `Check` (requirements: level,
job, items, prerequisite quests, mob kills) and `Act` (rewards: EXP, mesos,
items, fame, next quest), plus `Say` text. Shops, drops, and mob stats are
data too. Only a minority of behaviour (job advancement NPCs, events, PQs,
odd portals) needs real code.

## Options

| Option | For | Against |
| --- | --- | --- |
| **A. Data-driven engine + Lua (mlua) for exceptions** | Most content needs no code. Lua coroutines let a dialogue read top to bottom (`local yes = npc:ask_yes_no(...)`) while the server stays event-driven. Small, sandboxable, well-trodden in game servers. | Exception scripts are written fresh (or ported by hand). Adds a C dependency (or use Luau). |
| B. Embed JS (rquickjs) and reuse HeavenMS/Cosmic scripts | Thousands of existing scripts | They target HeavenMS's Java object model (`cm.getPlayer().getMap()...`), so the shim is enormous and inherits its semantics. HeavenMS is AGPLv3, which constrains this repository's licensing. |
| C. Rhai | Pure Rust, safe | No coroutines (dialogue becomes manual state machines again), slower, small ecosystem |
| D. Rust only | No new runtime | Every NPC is a code change and a rebuild; already showing strain |

## Recommendation: A

1. **Generic quest engine first.** Replace the quiz/contact special cases with
   one loader for `Check`/`Act`/`Say` and a quest state machine driven by the
   existing `QuestLog`, with rewards applied through `game::Player` and
   committed through `db::quest`. Rain and Heena/Nina become data.
2. **Lua for the rest.** Run scripts inside the owning `ClientActor` (it
   already owns the conversation). Expose a narrow host API that *returns
   effects* (say/ask/warp/give item/start quest) rather than mutating state,
   mirroring how world handlers return `WorldAction`s. One coroutine per
   conversation; the client's `NPC_TALK_MORE` resumes it.
3. **Scripts live in the repository** (`scripts/npc/<id>.lua`,
   `scripts/portal/<name>.lua`), looked up by the ids/names in the NX data, so
   content review is a normal code review.

## Consequences

- `net::quests` and `net::dialogue` move into a content module (likely a new
  crate) once the generic engine lands; `db::quest` keeps only persistence.
- Portal `script` fields get a dispatcher instead of being ignored.
- Integration tests keep driving behaviour end to end; scripts get unit tests
  through a fake host.
