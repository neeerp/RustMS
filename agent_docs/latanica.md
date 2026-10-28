# Capt. Latanica

The fight with Capt. Latanica in the Ghost Ship's Engine Room off Singapore,
after Cosmic's `LatanicaBattle`. The analysis and its open questions are in
v83ref's `reports/b276-latanica.md`; the defaults shipped are under "b279"
in `lanes/QUESTIONS.md`.

All dialogue is placeholder text (`latanica.*` keys in
`game-data/placeholder_text.toml`): the licensed scripts have none for it,
and Bob's script (9270033) is not in our Npc.nx.

## The way in

The fight is a party event, `EventKind::Latanica` (`net/src/events.rs`,
`LATANICA`). The way down from the Ghost Ship's last deck (541010060,
`GHOST_SHIP_DECK`) is the portal script `captinsg00`
(`scripts::LATANICA_IN`, `latanica_portal` in
`runtime/src/actor/client/events.rs`). It checks, in this order:

1. that the player holds a White Essence (4000381, `WHITE_ESSENCE`);
2. that they are in a party, and its leader;
3. that the party has one to six members on the deck, of any level;
4. that no other party is fighting on the channel.

Each failure gets a pink line (`latanica.portal.*`). The portal does not
take the essence.

The party goes into its own copy of the Engine Room (541010100,
`ENGINE_ROOM`) for 10 minutes, and the fight lasts until its last member
leaves. When the time is up everyone left goes to the Peaceful Ship
(541010110, `PEACEFUL_SHIP`).

## The summon

The Engine Room's reactor (5411000, `reactors::LATANICA_BOSS`) takes a
White Essence dropped on it (Reactor.nx event type 100, the item and an
area around the reactor), five seconds after it lands (`put_on_reactor` in
`runtime/src/actor/field.rs`). It then plays `Bgm09/TimeAttack`, sends the
pink line `latanica.summoned`, and lets out Capt. Latanica (9420513,
`mobs::LATANICA`) on the ground below (-146, 225) (`LATANICA_AT`).

## The win

Latanica's death in the copy is `boss_falls`: the clear effect and jingle,
and the event's one stage cleared, which wins it and stops its clock.

Bob (9270033) in the Engine Room asks before the win whether to leave, and
sends those who say yes to the Peaceful Ship. After the win, Bob says a
line and takes them up.

## Drops

The content pack has no drops for the Ghost Ship's mobs or the captain
(9420509 to 9420513). Theirs come from `game-data/reward_overrides.toml`:
Cosmic's tables, with each mob's mesos as one pile. Each drops its essence
3 times in 10:

- Slimy (9420510): 4000379;
- Pac Pinky (9420509): 4000380;
- Selkie Jr. (9420511): the Blue Essence 4000382;
- Mr. Anchor (9420512): the Red Essence 4000383, which Ralph's 4522 asks for;
- the captain (9420513): the Black Essence 4000384, which Fight for the
  future (4513) asks for, and a Soul Lantern (4000385).

No quest here asks for the others.

The captain's kill in the copy counts for 4513, like any kill.

## Tests

`net` has unit tests for the event (`events.rs`), the portal
(`scripts.rs`), the reactor (`reactors.rs`) and Bob (`npc_scripts.rs`). The
harness's `tests/latanica_battle.rs` turns away a player with no essence, a
player with no party and a member who is not the leader. It takes a party of
two in and drops the essence on the reactor. It turns away the next party
while the fight is on, and has Bob offer only the way out. It waits for the
pink line, the music and the captain, then beats the captain and sees the
clear. It checks that the clock stopped, and has Bob take both up. Last, it
lets the next party in and has its time run out. The hunter is on 4513 and
sees the kill count. `game-data` checks the drops load without the pack.
