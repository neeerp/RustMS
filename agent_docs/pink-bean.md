# Pink Bean

The battle with Pink Bean in the Temple of Time, after Cosmic's
`PinkBeanBattle`. The analysis and its open questions are in v83ref's
`reports/b274-pink-bean.md`; the defaults shipped are under "b283" in
`lanes/QUESTIONS.md`.

All dialogue is placeholder text (`pink_bean.*` keys in
`game-data/placeholder_text.toml`): no licensed script has any.

So far only the way in and out is done (slice 1). The statue, its guardians'
waves, Pink Bean itself, the win, the deaths count and the drops are still
to come.

## The expedition

The battle is a squad event, `EventKind::PinkBean` (`net/src/events.rs`,
`PINK_BEAN`), gathered like Zakum's (see [zakum](./zakum.md) and
`net/src/squads.rs`). The gatherer (2141001, `npc_scripts::PINK_BEAN_RECRUITER`,
no name in String.nx) at the temple's last gate (270050000,
`PINK_BEAN_GATE`) opens it for a leader of level 120 or more, with no item.
Others of level 120 join, and the leader sets out with 6 to 30 (two in the
harness, `RUSTMS_EXPEDITION_MIN`). One expedition a channel fights at a
time. A member who left may go back in while it lasts.

It goes into its own copy of the Twilight of Gods (270050100,
`PINK_BEAN_ARENA`) for two hours and twenty minutes, and lasts until its
last member leaves. When its time is up, everyone left goes to Between
Twilight and Daybreak (270050300, `PINK_BEAN_OUT`), Cosmic's exit map and
every map's `forcedReturn`.

## Death and the ways out

The copy also holds the Forgotten Twilight (270050200, `PINK_BEAN_REVIVE`).
A member who dies in the copy comes back there (`EventTicket::revive_to` in
`runtime/src/actor/event.rs`), which is Cosmic's wz `returnMap`. Royals's
Map.nx gives 270050300 instead, which would leave the dead no way back.
The Forgotten Twilight's `in00` (portal script `Pinkin`) leads back to the
fight, in the same copy.

The Forgotten Temple Keeper (2141002, `npc_scripts::TEMPLE_KEEPER`) stands
in both of the copy's maps. Before the win and after it, the keeper asks
whether to leave and takes those who say yes to the gate. As in Cosmic,
there is no reward from the keeper.

Between Twilight and Daybreak's `in00` (`PPinkOut`) leads to the gate.

## Tests

- `net`: the event's maps and terms (`events.rs`), the gatherer's level and
  ask (`npc_scripts.rs`), the keeper, and the two portals (`scripts.rs`).
- Harness `pink_bean_entry`: two gather and set out. The keeper asks first,
  and a dead member comes back in the Forgotten Twilight and goes back to
  the leader through `Pinkin`. The member leaves and may rejoin, and the
  last out ends the expedition. `PPinkOut` leads to the gate.
