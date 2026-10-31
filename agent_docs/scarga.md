# Targa and Scarlion

The battle with Targa and Scarlion in Malaysia's Spooky World, after
Cosmic's `ScargaBattle`. The analysis is in v83ref's
`reports/b374-scarga.md`; the defaults shipped are under "b375" in
`lanes/QUESTIONS.md`.

All dialogue is placeholder text (`scarga.*` keys in
`game-data/placeholder_text.toml`): the licensed scripts have none for it.

## The way in

The battle is an expedition, `EventKind::Scarga` (`net/src/events.rs`,
`SCARGA`), gathered by the generic squad NPC (`crate::squads`,
`npc_scripts::squad`). Aldol (9270047, `SCARGA_RECRUITER`) stands at the
Spooky World's entrance (551030100, `SPOOKY_ENTRANCE`). One of level 100 or
more who holds a Spirit of Fantasy Theme Park (4032246,
`SPIRIT_OF_THEME_PARK`) may open one. Aldol checks for the spirit but does
not take it. The expedition has 6 to 30 members (`RUSTMS_EXPEDITION_MIN`
lowers the least in the harness) and five minutes to set out.

Starting it takes everyone into their own copy of the Spooky World
(551030200, `SPOOKY_WORLD`) for an hour. It lasts until its last member
leaves. A death sends the player to Kampung Village (the map's
`forcedReturn`), which counts as leaving. When the time is up, everyone
left goes back to the entrance.

There is no daily limit: Cosmic has one, but its default config turns it
off.

The spirit is quest 4576's reward, two at a time, from NPC 9270062 in
Kampung Village, at the end of the chain 4557 to 4560. Those quests ask for
the Etc items of Rodeo, Yabber Doo, Booper Scarlion, Vikerola and
Gallopera. These items, and those of the other Malaysian mobs 9420527 to
9420540, drop 3 times in 10 (`game-data/reward_overrides.toml`). The rest
of those mobs' loot is not there yet.

## The summons

The Spooky World has no mobs of its own. Its two reactors each take a
spirit dropped on them (Reactor.nx event type 100, the item and an area
around the reactor; `put_on_reactor` in `runtime/src/actor/field.rs`):

- 5511000 (`TARGA_REACTOR`) brings out Targa (9420542, `TARGA`) below
  (-527, 637);
- 5511001 (`SCARLION_REACTOR`) brings out Scarlion (9420547, `SCARLION`)
  below (-238, 636).

Each plays `Bgm09/TimeAttack` and sends its own pink line
(`scarga.targa.summoned`, `scarga.scarlion.summoned`). A reactor stays
struck once it takes its spirit, so each boss comes out once. Each first
form revives as an angrier one, and that as the last (Mob.nx's `revive`):
Furious Targa (9420544) and Furious Scarlion Boss (9420549), with 150
million HP each.

## The win

`SCARGA_BOSSES` holds the two last forms. The field actor keeps the ones
that have fallen in its copy (`scarga_fallen`, through
`events::scarga_falls`). The second to fall, in either order, is
`boss_falls`: the clear effect and jingle, and the event's one stage
cleared, which wins it and stops its clock. Every mob left in the copy goes
with it.

Aldol (9201134, `aldol_spooky` in `net/src/npc_scripts.rs`) in the Spooky
World asks before the win whether to leave, and sends those who say yes
back to the entrance. After the win Aldol gives each member one prize at
random from `SCARGA_REWARDS`, and sends them back to the entrance. Cosmic's
list has 2022306 twice, so we keep it twice. A player with no room in Equip,
Use or Etc is told to make some first.

## Drops

The content pack has no drops for either boss. Both last forms use Cosmic's
tables (`game-data/reward_overrides.toml`), including Targa's Foot
(4001241) or the Scarlion Boss's Foot (4001242) 3 times in 10. Cosmic drops
the four hats 1003023 to 1003026 three to five at a time; here an equip
drops one at a time. The first and angry forms drop nothing.

## Tests

`net` has unit tests for the event and the win rule (`events.rs`), the
reactors (`reactors.rs`) and Aldol's squad (`npc_scripts.rs`). The
harness's `tests/scarga_battle.rs` turns away a would-be leader with no
spirit. It takes an expedition of two in, and has Aldol offer only the way
out. It drops a spirit on each reactor and waits for each pink line. It
beats all three forms of Targa and checks that the battle is not won yet.
Then it beats Scarlion's three forms and sees the clear. Last, Aldol gives
each a prize and sends them out.
