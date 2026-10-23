# Party quests and instanced events

A party event is played in a copy of its maps of its own: an instance.
Kerning City's party quest (KPQ) is the first, and is played through: its
entry, exit and timer, stage 1 (Cloto's quiz), stages 2 to 4 (her puzzles
of areas to stand on), with their clears and the gates on, stage 5 (ten
passes), which wins it, and Cloto's reward and the bonus stage (see
`v83ref/reports/a005-instancing-kpq.md`, section 3, `a011-kpq-stage1.md`
and `a016-kpq-stages-2-5.md`). Henesys's party quest (HPQ) is played
through: its moonflowers, Moon Bunny, rice cakes, win, reward and Pig Town
bonus (`v83ref/reports/a058-henesys-pq.md`). Ludibrium's party quest (LPQ)
is played through too: its nine stages, bonus and reward
(`v83ref/reports/a066-ludibrium-pq.md`). Orbis's party quest (OPQ) has
its entry and exit, its cloud stage, the hub's ways into its rooms, the
six rooms, the garden, Minerva's statue and the win, her prizes, and the
storage and treasure room the win opens; its jails come in a later slice
(`v83ref/reports/a077-orbis-pq.md`, `a087-opq-garden-win-re.md`,
`a091-opq-bonus-jail-re.md`). Zakum's Dead Mine (ZPQ), the first of the
trials before Zakum ([zakum](./zakum.md)), is played through too
(`v83ref/reports/a105-zakum-dead-mine.md`). Zakum's own altar is an event
too, but of an expedition squad gathered across parties (`Event::squad`,
`net/src/squads.rs`): its team comes from the squad's roster rather than a
party, may come back in while it runs, and ends only when its last member
leaves (see the expedition in [zakum](./zakum.md)); Horntail's cave is
another ([horntail](./horntail.md)). The fight with
Papulatus is a party's event that ends that way too (`Event::until_empty`),
started by a portal rather than an NPC ([papulatus](./papulatus.md)).

## Rules

`net/src/events.rs` holds what doesn't move:

- An `Event` gives the lobby where the party gathers, the maps it plays
  in its copy, the entry and exit maps, the levels, the team size and the
  duration.
- `KPQ` has lobby 103000000, maps 103000800-805, exit 103000890, levels
  21-30, 3-4 players and 30 minutes (Cosmic `KerningPQ`).
- `HPQ` has lobby 100000200 (Henesys Park), maps 910010000-400, exit
  910010300, levels 10 and up, 3-6 players and 10 minutes (Cosmic
  `HenesysPQ`). Its exit lies inside its range of maps: `Event::holds`
  never counts the exit as one of the event's maps.
- `LPQ` has lobby 221024500 (Eos Tower's 101st floor), maps
  922010000-922011100, exit 922010000, levels 35-50, 5-6 players and 45
  minutes (Cosmic `LudiPQ`). Its exit also lies inside its range of maps.
- `OPQ` has lobby 200080101 (Orbis), maps 920010000-920011300, exit
  920011200, levels 51-70, 5-6 players and 60 minutes (Server.NX
  `FieldSet.img/Party5`; Cosmic has 45). Its exit lies inside its range of
  maps too. The win leaves the team where it is (no `clear_map`): Minerva
  sends each member to her prize room 920011300. Its nine stages are
  listed in `Event::stage_maps` (see "Orbis's cloud stage and hub").
- `Event::team` picks who goes: the party's members online on the
  leader's channel, standing in the lobby, at the right levels. Members
  in the Cash Shop are on no map and don't count: the client tells the
  directory (`EnteredCashShop`), which marks their entry until they leave
  the shop and register again (Cosmic `isLoggedinWorld`). The leader
  must be one of them, and there must be enough of them and not too many.
- `Team::leave` ends the event when the leader leaves or too few remain.
  Once the event is won (`Team::win`), only the last member's leaving
  ends it.
- `Team::hand_over`: when the leader passes the party's lead on, the new
  leader leads the event too, if they are in its maps; if not, the event
  ends, unless it is won (Cosmic `changedLeader`; a won event's lead may
  go to someone outside and back). The directory tells the old and the new
  leader's client actors (`ServerMessage::LeaderHandedOver`), and either
  tells the event (`EventCommand::HandOver`), which takes it from the
  leader or from the new leader. The leader is shared with the tickets
  (`EventTicket::leader`). Two handovers within moments of each other may
  reach the event out of order.
- `RUSTMS_EVENT_SCALE` (a whole number, default 1) shortens every event.
  The harness world sets it to 60, so KPQ lasts 30 seconds.

## Who owns what

| Owner | Holds |
| --- | --- |
| Directory | Which event each channel is playing, as `(channel, kind)` → id. There is one at a time per kind, like a GMS lobby. |
| Event actor (`runtime/src/actor/event.rs`) | The team, the deadline, the members' handles, and the stages' progress (`Stage1`, and `Puzzles` for stages 2-4). It alone writes the stages cleared (`Cleared`, one bit per stage), shared with the tickets. |
| Field registry | The open instance ids. The fields of a copy are keyed by `FieldKey{channel, map, instance_id}`. |
| Client actor (`client/events.rs`) | The member's `EventTicket`: the id, the kind, the end time, the event's handle, and the stages cleared. |

## Flow

1. **Asking.** Lakelis (9020000) asks the directory
   (`DirectoryMessage::EventEntry`) whether the party may go:
   - `NoParty`, `NotLeader` and `Busy` each get a Say page.
   - `NotEligible` gets a Say page that states the terms.
   - `Ready` gets a yes/no question (`NpcScript::lakelis`).
2. **Reserving.** Saying yes runs `Effect::StartEvent`, which must be the
   script's last effect. The leader's client asks the directory to reserve
   the event (`StartEvent`). The directory checks again and returns an
   `EventStart` with the team's handles, or `None`; then the script's
   `refused` text, the terms, is shown whatever the reason. A reservation
   whose reply the leader can't take is dropped at once.
3. **Starting.** The leader's client spawns the event actor
   (`event::spawn`). That opens the instance in the registry and sends each
   member `ServerMessage::EventStarted(ticket)`.
4. **Entering.** A member who gets a ticket warps to the entry map if they
   are still in the party and the lobby, and not in the Cash Shop;
   otherwise they drop it. While a
   member holds a ticket, `change_field` uses `field_key`, so the event's
   maps resolve to its copy. On arrival the client is sent the countdown
   (`show_event_clock`) while the ticket has a deadline (`EventTicket::clock`,
   none once the event is won until a bonus starts).
5. **Leaving.** A ticket hands itself back when dropped
   (`EventMessage::Left`), including one that never reached its member.
   Each of these ways out drops it and stops the clock:
   - going to a map outside the event, whether by Nella, by dying, or by a
     debug warp;
   - leaving or losing the party (`PartyChanged{None}`), which also warps
     the member to the exit, unless the event is won (Cosmic
     `playerLeft`);
   - logging out or changing channel, which come through `cleanup`. A
     member in the Cash Shop keeps the ticket until they come back out,
     which is a channel change.

   Whichever way they leave, the event's own items (`Event::items`, KPQ's
   coupons and passes, Cosmic's `setExclusiveItems`) are taken from them:
   `Event::take_items`, shown to the client unless they are logging out.
   A blessing given on arrival (Orbis's, below) ends too, told only to
   the player, and not at all on logging out, when buffs go anyway.
6. **Ending.** The event ends when its time is up, when its leader leaves
   or passes the lead to someone not in it, or when too few remain; once
   won, only when its last member leaves. It sends `EventEnded{id}`, and
   members still on its maps go to the exit. Two seconds later the actor
   exits. Its
   `on_exit`, which runs even after a panic, sends every member
   `EventEnded` again, closes the instance (`FieldRegistry::close_instance`
   sends each field `FieldMessage::Close`) and tells the directory
   (`EventClosed`), which frees the lobby, about two seconds after the
   event ended. A won event frees it at the win (below).

A player saved on an event map logs back in at its `forcedReturn`, the
event's exit (103000890, 910010300), outside the copy. Where that
`forcedReturn` is itself one of the event's maps, `leave_forced_return_map`
follows on by that map's own `forcedReturn`, so a player saved in Pig Town
(910010200) starts on 910010300 and one saved in Ludibrium's bonus map
(922011000) on 922010000. A player without a ticket who warps onto an
event map by a debug warp lands on the channel's own copy of it, which the
harness uses to check that copies are apart.

## Closed copies

A closed copy never comes back:

- `FieldRegistry::field` starts a field of a closed instance for its caller
  alone, without registering it. It stops when the caller leaves, so a
  stale ticket can't make a copy outlive its event.
- `send_to_field` doesn't rejoin a gone field of a closed instance
  (`FieldRegistry::is_open`). The event has already sent its members out.
- `field` and `close_instance` both lock the fields before the open
  instances, so no field of a copy is registered after it closes.

## Stage 1

On 103000800 Cloto (9020001) sets a quiz (`net::events::Stage1`, Cosmic
`9020001`):

- Each member but the leader is asked, on their first talk, for as many
  coupons (4001007) as one of seven job-advancement requirements
  (`ANSWERS`). The question is drawn then and stays theirs.
  `RUSTMS_KPQ_QUESTION` (1 to 7) pins it; the harness world pins 7, whose
  answer is 8.
- Holding exactly that many earns a pass (4001008) for the coupons. A
  wrong number costs nothing, and she asks again. A member who has a pass
  is told to give it to the leader.
- The leader clears the stage by handing in a pass from each of the other
  members (`members - 1`). Passes reach the leader by trade or by being
  dropped and picked up.
- After the clear, Cloto tells everyone to hurry on.

The client actor asks its ticket where the player stands
(`EventTicket::cloto`, a reply from the event actor), and builds
`NpcScript::cloto` from that and the player's bag. The items change hands
as her page is shown (`Page::Act`, as in Cosmic, where the page and the
items go in one call). The script's last effect, `Effect::EventCommand`,
tells the event (`PassGiven`, `ClearStage(1)`) once the items have changed
hands. Outside an event Cloto has nothing to say.

## Stages 2 to 4

On 103000801-803 the team must take three of the map's areas, one member
to each (Cosmic `9020001`'s rope, platform and barrel stages):

- The areas are the map's own (`Map.nx` `area`, `FieldTemplate::areas`,
  in node order; `Area::contains` is half-open): 4 ropes, 5 platforms and
  6 barrels.
- A puzzle is three of them (`combos(n, Event::areas_taken)`: the masks
  below `2^n` with that many bits, in order). It is drawn at the leader's
  first talk on the stage and kept (`net::events::Puzzles`).
  `RUSTMS_KPQ_COMBO` pins the index into `combos`, from 0, on every stage
  of every event; an index past a stage's last is ignored there. The
  harness world pins 1: in Kerning City areas 0, 1 and 3, on Ludibrium's
  stage 8 crates 0, 1, 2, 3 and 5.
- A member who isn't the leader is told to follow the leader
  (`Cloto::Follow`). The leader's first talk sets out the puzzle
  (`Cloto::Intro`). Each later talk by the leader is an answer.
- At the leader's later talks (`Cloto::Puzzle`) the client actor asks the
  field where everyone in it stands (`FieldMessage::Positions`) and checks
  them (`on_areas`): one occupant in each of the puzzle's areas and nobody
  in another, each counted in the first area holding them. Only the
  event's members can be in its copy.
- Wrong, everyone on the map is shown FIELD_EFFECT 3
  `quest/party/wrong_kor` and hears 4 `Party1/Failed`, and Cloto says so.
  Right, her page (`NpcScript::cloto_answer`) runs
  `EventCommand::ClearStage(stage)`, which the event takes only from the
  leader, for a stage whose puzzle has been set.
- Once the stage is cleared, Cloto tells everyone so.

Where a player stands is only what their client last sent in MOVE_PLAYER.
A client may hold back moves in the air or on a rope until it lands, so a
member hanging still on a rope may still count where they last stood
(`v83ref/reports/a018-kpq-stage-facts.md`).

## Stage 5

On 103000804 the monsters drop the passes, and the leader hands Cloto ten
of them (`Event::passes`, Cosmic `9020001`) to clear the stage
(`Event::last_stage_talk`):

- The leader holding fewer is told to bring ten; with ten or more, Cloto
  takes ten (`Cloto::Leader{stage: 5}`, `ClearStage(5)`).
- The other members are told to gather passes for the leader
  (`Cloto::Gather`). Only the leader may hand them in.
- Once cleared, everyone is told the party quest is won (`Cloto::Won`).

The clear is a stage clear like the others, and then wins the event
(`EventActor::win`, Cosmic `clearPQ`):

- The deadline goes (`ends` is `None`), and every member is sent
  STOP_CLOCK with the clear. Coming onto one of the event's maps after
  that shows no clock (the ticket's `clock` is `None`).
- `Team::win`: the event no longer ends when the leader leaves or too few
  remain, only when the last member leaves its maps or the game.
- The lobby is freed at once (`EventClosed`, said again when the event
  ends, which changes nothing): the next party may go in while the won
  one is still inside, in a copy of its own. Cosmic's `setEventCleared`
  frees it too, after `EVENT_LOBBY_DELAY` (10 seconds); we don't wait.
- Leaving the party, or the leader breaking it up, no longer sends anyone
  out (Cosmic `playerLeft` and `disbandParty` once cleared).

## Kerning City's reward and bonus stage

Once stage 5 is won, Cloto on 103000804 congratulates each member who
comes to her (`Cloto::Won`, `kpq.cloto.s5.won`), and on the next page
gives them one of `KPQ_REWARDS` (Cosmic `KerningPQ`'s 42: scrolls,
earrings, hats, potions and ores, all equally likely) and sends them to
the bonus stage, `KPQ_BONUS` (103000805, `st00`), so nobody gets it twice
(Cosmic `9020001`, `giveEventReward`). A member without a free slot in
each of the Equip, Use and Etc tabs, which the rewards come in, is told to
make room (`kpq.cloto.s5.full`) and stays. There is no EXP in the reward;
the stage EXP came with each clear.

The bonus stage has no time limit, as in Cosmic, where `clearPQ` stops the
event's timer (Cloto's text there speaks of one, of unsourced length).
Nella there asks only whether the player
is ready to go (`kpq.nella.bonus`) and sends them to 103000890 like on the
other stages, which takes the coupons and passes they still hold. The won
event ends when the last member has left its maps.

## Clearing a stage

Stage n is played on `entry + (n - 1) * stride` (`Event::stage_map`):
KPQ's and HPQ's stage maps follow one another (`stride` 1), and LPQ's are
100 apart (922010100 to the boss's 922010900), with rooms off a stage
between them, on which no stage is played (`Event::stage_on`). OPQ's
don't follow a stride and are listed instead (`Event::stage_maps`). `Cleared`
holds 16 stages. A clear, taken once and only from the
leader, goes like this (`EventActor::clear`):

1. The event sets the stage's bit in `Cleared` before telling anyone, so a
   member who sees the clear can go on.
2. Every member gets `ServerMessage::StageCleared{id, stage}`. Those on
   the stage's map are shown FIELD_EFFECT 3 `quest/party/clear`, 4
   `Party1/Clear` (the sound), and 2 `gate` for an event whose stage maps
   have one (`Event::gates`: KPQ's do, HPQ's doesn't).
   Everyone in the event, wherever they are in it, gets the stage's EXP
   as quest EXP (`Event::exp_per_stage`: 100, 200, 400, 800, 1500,
   Cosmic's, though not scaled by an EXP rate as Cosmic's is). A member in
   the Cash Shop gets none; leaving it takes them out of the event anyway.
3. A member who comes onto a cleared stage's map later is shown its gate
   again (`show_event_gate`, from `join_current_field`).

The client doesn't stop anyone at a closed gate: it sends CHANGE_MAP with
the portal's name either way. So `handle_packet` checks CHANGE_MAP before
the handler moves the player (`gate_closed`). For `next00` on 103000800-803
(`Event::gated`) before that stage's clear, it answers BLOCKED_MAP 1, the
client's own "The portal is closed for now." line, which also unlocks it
(`v83ref/reports/a012-kpq-gate.md`). Map-id warps aren't checked.

## Henesys's moonflowers

Primrose Hill (910010000) has six moonflowers (`9108000`-`9108005`, named
`moonflower1`-`6`), each taking one seed of its own colour (`4001095`-
`4001100`) put on it, as any reactor takes an item (`combat.md`). The seeds
drop from the hill's nut reactors (`9102002`-`9102007`). A moonflower stays
in bloom on its last state rather than breaking (`Action::Bloom`,
`reactors::stays`). Each bloom puts the full moon (`9101000`, named
`fullmoon`) in as many phases as there are flowers in bloom (HIT_REACTOR),
and as it turns full, the Moon Bunny (`9300061`) comes out on the ground
below the moon. Everyone on the map then gets a light-blue line
(BROADCAST_MSG 6, `hpq.moon.bunny_out`) and, for 7 seconds, the quest's own
weather item `5120016` with `hpq.moon.weather` (BLOW_WEATHER), which a
player arriving meanwhile sees too. Each copy starts with its flowers and
moon at state 0.

## Henesys's Moon Bunny

The Moon Bunny is a friendly mob (Mob.nx `damagedByMob`): players' attacks
pass through it (`handle_attack` drops their hits), and only other mobs
hurt it. Its client sends MOB_DAMAGE_MOB_FRIENDLY (recv 0xC0: attacker,
character, victim) when a mob's body touches it, with no damage, so the
server (`MobSet::friendly_hit`) rolls a tenth of a number under (maxHP / 13
+ PADamage * 10) * 2 + 500 (Cosmic's rule), for a mob that isn't friendly
within 600 x 300 px, at most once a second per victim (the v83 client sends
it once in 1.5 s; this only stops several clients doubling it). Everyone
gets DAMAGE_MONSTER (0xF6) with the number, HP and max HP, which the client
shows as the friendly's gauge. Every sixth hit tells the map
`hpq.bunny.sick` (BROADCAST_MSG 6).

In HeavenClient (`FriendlyRules.h`, `MapMobs::update_friendly`), a mob
whose Mob.wz `info/damagedByMob` is set is friendly. Its client's attacks
and skills skip it, and so do Hypnotize, touch damage and Mine. Mobs this
client controls that may hurt it (any mob, or those on its
`damagedBySelectedMob` list) chase it. The party's leader's client, or a
lone player's, sends 0xC0 for a mob whose body meets it, at most once in
1.5 s across the map; the victim flinches away from the attacker. Its
DAMAGE_MONSTER sets a red 52x10 gauge, which it shows from the start
(other mobs' bars are now light green), and makes it take its hit stance,
blinking grey 30 ms on and off, unless the hit is under its knockback.

Its `dropItemPeriod` (6, in tens of seconds) is a third as long for the
bunny, so every 20 seconds it drops what the content pack's drops give mob
`9300061` (a rice cake, `4001101`), without the quest items or the
drops every mob shares (`loot::roll_friendly`), owned by the party of a player there, and the map
hears `hpq.bunny.cake` with the count. 20 s is a recommended default in
`v83ref/lanes/QUESTIONS.md` (row 180).

When its HP runs out it goes home rather than dying (KILL_MONSTER 0, no
drops, no EXP) and the map hears `hpq.bunny.gone`. In an event's copy, 5
seconds later (shortened like events' time) everyone there gets a pink
`hpq.bunny.lost` (BROADCAST_MSG 5) and their clients tell the event it
failed (`ServerMessage::EventFailed`, `EventCommand::Fail`), which ends it
and sends everyone to the way out, unless it is already won. Players'
mists don't reach a friendly mob either.

## Henesys's win and Pig Town

On 910010000 the party's leader hands Growlie (1012114) ten rice cakes
(`RICE_CAKE`, his menu shows the choice to the leader only): he takes them
(`ClearStage(1)`), which is the quest's one stage, so it is a clear (1600
EXP, the clear effect and sound, no gate) and the win. An event with a
`clear_map` then moves everyone in it there (`EventActor::move_team`,
`ServerMessage::EventMoved`, Cosmic `warpEventTeam`): HPQ's is 910010100.
Cosmic also kills the mobs that aren't friendly; we don't, since everyone
leaves the map at once.

On 910010100 Tory (1012112) gives each member who comes to her the reward
(`HPQ_REWARD`, `4001158` x1) and sends them home, so nobody gets it twice;
with a full Etc tab she says `hpq.tory.reward.full` and keeps them. The
item and the once-per-member rule are recommended defaults in
`v83ref/lanes/QUESTIONS.md` (row 180, 2A and 3A). Tommy (1012113) tells
of Pig Town; the leader may take everyone still in the event there
(`EventCommand::StartBonus`, once): the event moves them to the bonus map
(`Event::bonus`, 910010200) with a new deadline of 5 minutes (shortened
like events' time), which their clients show. In Pig Town Tommy lets a
player out to its exit (910010400), where the bonus's clock still shows
until it runs out, as Cosmic shows an event's timer on any of its maps. When the bonus's time is up, those
still in Pig Town are moved to its exit with the clock stopped, and the
event carries on, won, until its last member leaves (`time_up`). On
910010400 Tory gives the reward and sends them home too. A copy's NPCs
talk this way only to a player holding its ticket
(`NpcScript::in_event`); without one, Tory and Tommy only send players
home.

## Ludibrium's entry

On 221024500 the Red Sign (2040034) has a menu (Cosmic `2040034`): taking
the party in, or what the quest is. As with Lakelis and Tory, the entry
check is the runtime's (`talk_about_event`); the party's leader takes the
party in at once, without being asked again, as in Cosmic. Those who go
are the party's members on 221024500 at levels 35 to 50, so a member of a
higher level, or elsewhere, stays behind, and there must still be five.
Everyone starts on 922010100. Cosmic's party-search toggle is left out,
as with Tory.

Sgt. Anderson (2040047) is on every stage map: he asks before letting a
player out to 922010000, as they can't come back. On 922010000 he sends
them down to 221024500 after one page. Leaving takes the quest's items
(`Event::items`): the passes 4001022, the key 4001023, and the pass
4001156 for stage 7's boxes, which Cosmic leaves with the player (the
default 5A in `v83ref/lanes/QUESTIONS.md`, LPQ). Five players are the
fewest, so the first to leave ends the quest for a party of five.

`922010100`-`500`, `700` and `800` keep `next00` shut until their stage is
cleared (`Event::gated`); `600`'s maze has no clear. Its hidden portals
(`h000`-`h044`) lead within the map, which the client does on its own:
HeavenClient moves the player and sends nothing, and the v83 client sends
only INNER_PORTAL (0x65), which needs no answer (`a069`). Its `next00`
leads on to stage 7 unshut, as stage 5's gate already held the party back.

## Orbis's entry

On 200080101 Wonky (2013000) has a menu like the Red Sign's: taking the
party in, or what the quest is (`talk_about_event`). Those who go are the
party's members on 200080101 at levels 51 to 70, five at least, and
everyone starts on 920010000. Cosmic's party search and Wristband
exchange are left out. On 920010000 Wonky asks before letting a player
out to 920011200, from where Eak (2013001) sends them back to 200080101.
Leaving takes the quest's items, 4001044 to 4001063 (Cosmic
`setExclusiveItems`), and five players are the fewest, as in Ludibrium.

A team with a warrior, a magician, a bowman, a thief and a pirate among
it, at any advancement and Cygnus Knights included, gets one of Wonky's
four blessings (2022090 to 2022093, two stats each for 30 minutes), drawn
once for the team (`Event::blessing`, Cosmic `afterSetup`). The directory
draws it from the members' `Presence::job` when it reserves the event, and
`EventTicket::blessing` carries it. Each member gets it as an item buff
(`take_item_effect`) once on 920010000, and loses it whichever way they
leave the tower (`drop_event_ticket`, Cosmic `playerUnregistered`).

## Orbis's cloud stage and hub

OPQ's stages are Cosmic's `statusStg0` to `statusStg8`, numbered from 1
(`OPQ.stage_maps`): 1 the cloud stage 920010000; 2 to 7 the six rooms off
the hub, 920010200 to 920010700, each hiding a piece of Minerva's statue;
8 the garden 920010800; and 9, the win, Minerva's statue on the hub
920010100 (`OPQ_HUB`). Every stage but the garden gives 3500 EXP (Cosmic
`giveEventPlayersExp`; for the win, see "Orbis's garden").

On the cloud stage the light (reactor 2006000) starts in state 20
(`reactors::first_state`): v83's Reactor.nx has its item event, 20 Cloud
Pieces 4001063, only there, where Cosmic's XML has it on state 0. The
pieces dropped on it break it (`Action::Appear`, Cosmic's `2006000.js`):
everyone on the map is shown a pink line (`opq.notice.light`), and Eak
(2013001) comes out on the ground below it. The field adds him to its NPCs
(`FieldActor::appear`, the next object id after the map's own and the
foothold's id), shows him to everyone, and hands him to the NPCs'
controller; whoever joins later sees him with the rest. Until then he is
only the light, and as each member arrives he tells them to bring him the
20 Cloud Pieces (`NpcScript::opq_arrival`, `opq.eak.arrival`, Cosmic
`playerEntry`).

Eak there (`NpcScript::opq_eak`, from `dialogue.rs` with the ticket's
leader and stage 1's clear) sends anyone but the leader up to the stage's
`st00`, and the leader too once it is cleared. The leader first hears of
Minerva (`opq.eak.cloud`), then sends `ClearStage(1)`, which the event
takes from the leader (`Event::talk_clears`). The clear takes everyone on
the cloud stage to `st00` before its effect and EXP (`Event::clear_portals`,
`stage_cleared`). Cosmic warps first and talks after; a warp closes the
dialog here, so Eak talks first. On the hub Eak tells the leader to
gather the six pieces (`opq.eak.gather`) and anyone else that he speaks
only with the leader (`opq.eak.leader_only`).

The hub's portals `party3_room1` to `party3_room6` lead into the rooms
(`PLAIN_WARPS`, Cosmic's portal ids), and each room's `party3_roomout`
leads back to its own portal on the hub (`OPQ_ROOMS_OUT`). The storage's
`party3_room8` (`in06`) stays shut with a pink line
(`opq.notice.storage_closed`) until the quest is won (see "Orbis's
storage and treasure room").

## Orbis's walkway and lounge

In two of the rooms Eak takes what the team gathered for a piece of the
statue (`Event::hand_ins`, `opq_hand_in`): on the walkway 920010200
(stage 2) 30 of 4001050 for 4001044, and in the lounge 920010600 (stage 6)
40 of 4001052 for 4001048 (Cosmic `2013001`, `statusStg1` and
`statusStg5`). They go through the balloons' path (`talk_to_balloon`,
`EventTicket::balloon`), but `HandIn::intro` is false: the leader need not
hear the stage out first, and `HandIn::prize` is the piece Eak gives with
the clear (`eak_hand_in`). A leader short of them hears what to bring
(`opq.eak.hand_in`), anyone else is sent to the leader
(`opq.eak.leader_only`), and once the room is cleared he only sends the
team on (`opq.eak.room_cleared`). Cosmic lets the walkway's be handed in
again after the clear; here they can't.

The leader sets each piece in its scar on the hub (reactors 2008000 to
2008005, `Action::Hold`): dropping the right piece on it takes it to its
last state, where it stays (`reactors::stays`) and keeps showing the piece.
2008003 takes 4001044, 2008004 4001048; the others are for the rooms still
to come.

## Orbis's Salion stage

On 920010300 (stage 3) a Cellion (9300040) stands at the first of fourteen
spots, reactors 2001002 to 2001015 (Cosmic `2001002`). Killed, it drops
4001074, which its spot takes (Reactor.nx: an item event for 4001074) like
any item landing on a reactor (see `combat.md`): the spot breaks. Each
break brings out another Cellion at the spot `reactors::salion_next` names,
11 spots on each time, until a number of them, drawn when the first spot
breaks (4 to 13, `reactors::salion_limit`, as Cosmic draws it), have gone.
The field keeps that count (`FieldActor::salion`; each team has its own
copy of the map). The spot that breaks after the last drops its loot, a
piece of the statue, 4001045, for the owner of the drop that broke it (or
anyone still on the map), whose client actor then tells the event the stage is cleared
(`Event::member_clears`; any member may). The piece goes in 2008005 on
the hub.

A Cellion can die away from any spot, and then its drop is never taken.
For that case Eak here (`ClientActor::opq_eak`, `NpcScript::opq_salion`;
Cosmic `2013001`, `statusStg2`) offers a backup. Once no monster
(`FieldMessage::MobCount`) and no drop of any kind (`DropCount`) is left
on the map, he gives whoever asks the piece himself, if they have room
for it. That also clears the stage. Nobody can pick up a Cellion's drop
(4001074 has `pickUpBlock`), so a stray one has to lie out its three
minutes (`field_drops::LIFETIME`) before the room is bare. Like the
hand-ins, two members asking at the same moment could each get a piece,
and so could one asking just after the last spot's piece was picked up,
before the clear reaches the ticket. Once the room is cleared, Eak only
sends the team on. Harness `opq_salion` checks that he only says to
search while a Cellion is out; the bare room is covered by
`npc_scripts`'s unit test.

## Orbis's music room and sealed room

On 920010400 (stage 4) the music box, reactor 2008006 named `music`,
plays only the day's record: 4001056 on Sunday to 4001062 on Saturday
(Cosmic `2008006`), so `reactors::takes` turns every other record away
and it stays where it lands. Every state of the box has the seven item
events in Reactor.nx; the day is the field's (`ReactorSet::day`, fixed
when the field actor starts, from `reactors::music_day`). Once the box plays,
Eak (`ClientActor::opq_eak`) brings out the stone box `stone3` (2002011)
by setting it to state 1 (`Effect::SetReactorState`, Cosmic
`forceHitReactor`, though never back to an earlier state) and clears
the stage; before then he asks for the
day's record, and after he only thanks them. The stone box, out of its
fade-in, takes four hits and drops 4001046.

On 920010500 (stage 5) Eak wants three of the team on the room's three
platforms (Map.nx `area` 1-3), so many on each. The field draws the split
the first time he is asked (`FieldActor::split`, `events::draw_split`,
Cosmic's rule) and keeps it; `events::head_count` counts who stands where,
each player on the first platform that holds them. Other than three on
the platforms, he only says so; with three but a wrong split, the room
shows the wrong effect and he says whether any platform is right; the
right split brings out `stone4` (2002012) as above and clears the stage.
Its box drops 4001047. Any member may clear either stage
(`Event::member_clears`). The pieces drop without the content pack
(`replace` in `reward_overrides.toml`).

The weekday is UTC plus `RUSTMS_RATE_CARD_UTC_OFFSET`.
`RUSTMS_OPQ_DAY` (0-6, Sunday first) pins the day and `RUSTMS_OPQ_SPLIT`
(three digits summing to 3, e.g. `210`) the split; the harness world sets
both (`tests/opq_music.rs`).

## Orbis's climbs and levers

The lounge (920010600) and the tall room (920010700) are climbed pad by
pad: each row's pads (`party3_r4pt`, `party3_r6pt`, scripted portals the
client asks about with CHANGE_MAP_SPECIAL) have one right pad, drawn the
first time anyone steps on one (`FieldActor::climb`, `events::draw_climb`).
`ClientActor::opq_portal` answers them from `events::climb`, with the
portal sound and a warp to a portal of the same map (a full map change,
as in Cosmic). On the lounge's two rows of three the right pad leads a row
up and then to `success` at the top, a wrong one to `fail` below the first
row. On the tall room's sixteen rows of four the right pad leads a row up,
or after every fourth row to that section's landing (`pt00`-`pt03`), and
a wrong one back to the start of its section (`r6fail` in the first).
Cosmic's script divides with a fraction there, so from rows 6-8, 10-12
and 14-16 it names no whole portal and row 5 sends a player to the very
bottom; here they fall to their section's start (`v83ref/lanes/QUESTIONS.md`).

At the tall room's top are five levers, reactors 2008007 named "1" to
"5", which flip between 0 and 1 on each hit and never break. Eak, at the
bottom, wants two of them up, drawn the first time he is asked
(`FieldActor::levers`, `events::draw_levers`), and judges them with
`events::lever_check`: other than two up, he asks for two; two others
show the wrong effect, and he says whether one of them is his; his two
bring out the stone box `stone6` (2002013, a link to 2002011) and clear
stage 7. The box drops 4001049. As in the music and sealed rooms any
member may ask him; Cosmic hears only the leader. `RUSTMS_OPQ_LOUNGE`
(two digits 1-3), `RUSTMS_OPQ_TOWER` (sixteen digits 1-4) and
`RUSTMS_OPQ_LEVERS` (two lever names, e.g. `14`) pin the draws; the
harness world sets them (`tests/opq_levers.rs`).

## Orbis's garden

Once the six scars on the hub hold their pieces, Eak there takes the
whole team to the garden 920010800 (stage 8) at the leader's word
(`Statue::Garden`, `EventCommand::MoveTeam`, which the event takes only
from the leader and only to one of its own maps, with its clock still
running; Cosmic `warpEventTeam`). Eak in the garden tells anyone of Papa
Pixie (`opq.eak.garden`).

The garden is a chain of seeds set in its reactors, each taking an item
that lands in its area (as the scars do):

- Nine Nependeaths (9300048) stand there, spawned once. Each drops a
  Strange Seed 4001053.
- A seed in one of the eight pots (2001000, 2001001, `Action::Pot`) breaks
  it and grows a mob on the ground below (`FieldActor::pots`,
  `reactors::pot_mob`): a Royal Nependeath (9300049) 40 times in a hundred
  for the first seven, then always; otherwise another Nependeath. A Royal
  drops 4001074.
- Under each pot spot is an invisible trigger (2001016,
  `Action::PapaPixie`). A Royal's seed on one takes every mob off the
  field, stops the pots growing anything (`FieldActor::papa_pixie`), brings
  out Papa Pixie (9300039) on the ground below (260, 490), and shows
  everyone there a pink line (`opq.notice.papa_pixie`). He drops 4001054.
- His seed in the Grass of Life's pot (2002003) and four blows break it
  (`Action::DropsAndClears`): it drops the Grass of Life 4001055 and clears
  stage 8 for whoever broke it (`Event::member_clears`). The clear gives no
  EXP but plays the clear effect and sound, as every stage here does; Cosmic
  plays none (`v83ref/lanes/QUESTIONS.md`). A Royal grown from a pot and
  killed where it stands drops its seed on the trigger under that pot.

The four drops are in `game-data/reward_overrides.toml` for a world without
the content pack. The garden's way out, `party3_gardenin`, takes the team
back to the hub's first portal with the portal sound only for the leader
holding the Grass (Cosmic keeps it); anyone else gets a pink line
(`opq.notice.garden_out`). The client asks about it with
CHANGE_MAP_SPECIAL, answered by `ClientActor::opq_portal`. Back on the hub
Eak then tells the leader to set the Grass at the statue's foot
(`opq.eak.grass`), and once the quest is won to talk to Minerva
(`opq.eak.minerva`).

Map.nx flags the pots for shuffling (`reactorShuffle`), but Cosmic doesn't
shuffle them, and neither does RustMS. `RUSTMS_OPQ_POTS` (a digit 0-7)
pins how many Nependeaths grow before the Royals; the harness world sets 1
(`tests/opq_garden.rs`).

Minerva's statue (reactor 2006001 at (-48, -916) on the hub,
`reactors::OPQ_STATUE`) starts in state 6 (`reactors::first_state`), the
one whose Reactor.nx event takes the Grass of Life; its states 0 to 5 take
nothing. The Grass dropped at its foot by anyone breaks it
(`Action::AppearAndClears`): Minerva (2013002) comes out on the ground
below it, and stage 9 clears for whoever set it (`OPQ_WIN_STAGE` in
`Event::member_clears`, whose last stage wins rather than only clears).
The win gives 3500 EXP and plays the clear effect and sound, and stops
the clock with no new one; the team stays on the hub. Cosmic's statue
script means to give that EXP and effect and start a five-minute clock,
but fails before them, so its team gets neither
(`v83ref/lanes/QUESTIONS.md`). Eak then tells the leader to talk to
Minerva.

Minerva on the hub asks each member, alone, whether to go
(`opq.minerva.ask`), and sends those who say yes to her prize room
920011300 (`OPQ_PRIZE_ROOM`, portal 0). There she thanks them
(`opq.minerva.thanks`) and gives each one of `events::OPQ_REWARDS` (94
items and counts, all equally likely, Cosmic `OrbisPQ`
`setEventRewards`), then sends them home to 200080101, portal 0. A member
without a free slot in each of the Equip, Use and Etc tabs gets
`opq.minerva.full` and nothing yet.

## Orbis's storage and treasure room

Once the quest is won the hub's `in06` (`scripts::OPQ_STORAGE_IN`,
Cosmic `party3_room8`) takes any member, with the portal's sound, to the
hidden storage 920011000 (`OPQ_STORAGE`, portal 0); the runtime opens it
in `opq_portal`, and outside a won event the pure script keeps it shut.
The storage's 19 guards (9300051-9300053, which never come back) drop
the pack's diary pages, and its `st00` leads back to the hub's portal 16.
There Eak (Cosmic `2013001`) answers only the leader
(`opq.eak.leader_only`): while a guard is left he says to clear them
(`opq.eak.storage`, through `Effect::RequireNoMobs` and the script's
refused line), and then he takes the leader alone, without a word, to
the treasure room 920011100 (`OPQ_BONUS`, `st00`), as Cosmic warps only
the player who asked. There is no clock, as the win stopped it.

The treasure room's 16 boxes (reactor 2002014, an ordinary box
`0002000` underneath) each break on the fourth hit and spray at least 15 drops with mesos of 100 to 399 (Cosmic
`dropItems(true, 1, 100, 400, 15)`, `Action::SprayAndAppear`). The
first to break also brings Minerva out at (46, 840)
(`OPQ_BONUS_MINERVA_AT`, the floor there, as Cosmic's `spawnNpc`); the
field brings no second one while she is out. She asks the leader whether
to leave (`opq.minerva.bonus_ask`) and sends them to her prize room,
920011300, portal 0, for their prize like everyone else.

A player who logs in on the treasure room would land on its
`forcedReturn`, the prize room, where nobody outside the event may be;
`leave_forced_return_map` goes on past any event's map by that map's own
`forcedReturn`, so they start on the tower's exit 920011200.

## Ludibrium's balloons

Stages 1 to 5, 7 and the boss's 9 are cleared by handing their balloon
NPC the stage's items (`Event::hand_ins`, Cosmic `2040036`-`2040044`):
25, 15, 32, 6, 24 and 3 passes 4001022, then Alishar's key 4001023. The
event keeps which stages' balloons the leader has heard
(`EventActor::balloon`, Cosmic's `statusStg`), and tells the runtime
where a member stands (`EventTicket::balloon`, `Balloon`):

- the stage is cleared: anyone is hurried on;
- a member who isn't the leader is told to fetch the leader;
- the leader's first talk only says what the stage asks, and clears
  nothing, with or without the items;
- after that, a leader holding the items hands them in and sends
  `ClearStage(stage)`, which the event takes only from a leader who has
  heard that stage out. The last stage's clear wins the quest.

`talk_to_balloon` routes the click; the Aqua Balloon (2040041) on the maze
says one line.

Stage 8 is a puzzle like Kerning City's (`Event::puzzles`, see "Stages 2
to 4"), but five of the map's nine crates (`areas_taken` 5), and the Blue
Balloon (2040043) sets it. The event answers its talks through the same
`Puzzles` (`EventActor::balloon` maps them to `Balloon`; the leader's first
talk draws the combo), and a later talk by the leader is an answer
(`Balloon::Answer`). The client actor counts who stands on any crate
(`standing`, Cosmic counts every member): other than five, the balloon asks
for five and shows nothing; five wrong, everyone on the map sees and hears
the wrong effects; five right, the leader sends `ClearStage(8)`, which the
event takes as it does a Kerning City puzzle's.

The passes come from the content pack's rewards: stage 1's Ratz, the
boxes 2202003 of stages 2 and 5, and the monsters of stages 3 to 5 each
drop one, and Alishar drops the key. Stage 7's three monsters drop the
other pass 4001156, which the pack lacks (`reward_overrides.toml`). One
of stage 2's boxes, 2200002, is a trap (`reactors::Action::WarpMap`, Cosmic
`go922010201`). Breaking it shows everyone on the map a pink line
(`lpq.notice.trap`) and sends them all to 922010201, still in the party's
copy. That room's four boxes make up stage 2's 15 passes, and its `out00`
leads back up.

Each of stage 3's eight boxes (2201001, Cosmic `ludiMob1`) lets out three
Bloctopuses (9300007) on the ground below it (`Action::BringOut`), which
drop passes like the four already there. Stage 5's portals `in01`-`in06`
lead to six rooms (922010501-506), each with four boxes 2202003 and an
`out00` back; they are in the party's copy like every map in the range.

Stage 7's three monsters each drop the other pass 4001156; put on one of
the stage's three reactors 2201002 (Cosmic `ludiMob2`), it brings out
Rombad (9300010) at (1, -211) with a pink line (`lpq.notice.rombad`), and
each Rombad drops one of the stage's three passes. On the boss's map a
pass put on reactor 2201003 (`ludiBoss0`) brings out Alishar (9300012) at
(941, 184) with `lpq.notice.alishar`; he drops the key. Both use
`Action::SpawnAt`.

## Ludibrium's bonus and reward

The key handed in wins the quest, and the win itself takes the team on
to the bonus stage, 922011000, for a minute (`Bonus::on_win`; Cosmic
`clearPQ` has no NPC step), after the clear's EXP and effects. Its 36
boxes 2202004 (`Action::Spray`, Cosmic `ludiquest4`) drop the pack's
rows with a pile of 30-59 mesos first, and more piles until there are 15
drops. The Pink Balloon (2040045) lets a player off early, to the
reward's `st00`; when the minute is up everyone still there is moved on
(`time_up`). On 922011100 Arturo (2040035) gives each member of the team
that won one of `LPQ_REWARDS` (Cosmic's 77, scrolls, potions, capes,
earrings, ores; the default 1A of `v83ref/lanes/QUESTIONS.md`) and sends
them back to 221024500, so nobody gets it twice. Without a free slot in
each of the Equip, Use and Etc tabs, which the rewards come in, he says
`lpq.arturo.full` and gives nothing yet (`giveEventReward`). There is no
EXP at the reward.

## Zakum's Dead Mine

At the Door to Zakum (211042300) Adobis (2030008) takes a party of one to
six in, the first choice of his menu for those a chief has approved
(`talk_about_event`): its members there at level 50 and up. Everyone starts
on the hub, 280010000, and the quest runs for 30 minutes on 280010000 to
280011006, a maze of rooms whose portals are Map.nx's own, in a fixed layout
(Cosmic's is fixed too). Leaving takes the quest's items, the documents
4001015, keys 4001016 and the Fire Ore 4001018. Dying or logging back in
lands on the Room of Tragedy (280090000, the maps' `forcedReturn`), outside
the event; Ali (2030011) there sends players back to 211042300.

In the maze, a trap box (2110000) sends its breaker to the hub's `sp` and a
Mimic box (2111000) lets out three Mimics (9300004); each tells the breaker
so in a pink line (`reactors::breaker_notice`, sent from the field to the
occupant who broke it, as Cosmic's reactor scripts tell only the
breaker). The documents and keys come from the mine's chests: the pack has
no rows for them, so `reward_overrides.toml` gives each its one item
(2112004 and 2112011 a key, 2112005 and 2112012 a document), and the giant
chest 2112014 in 280011005, which seven keys thrown near it open, the Fire
Ore.

Aura (2032002) on the hub has a menu: what the quest is, the hand-in, and
out to 211042300. Only the leader hands in, and only with the Fire Ore.
With 30 documents or more too, she takes the ore and 30 documents
(`EventCommand::WinWithDocuments`, which sets the event's stage bit
`ZPQ_DOCUMENTS` before the win); with fewer, the ore only
(`ClearStage(1)`, the one stage, which wins it). Each member then gets
20,000 EXP with the documents, or 12,000 without (`stage_cleared`; no
clear effect, sound or gate, as Cosmic's `clearPQ` shows none). The EXP
is not multiplied by the server's rate. Once won, Aura gives each member,
once, five Return to Dead Mine scrolls (2030007) if the documents went in,
and a Piece of Fire Ore (4031061); the claim is the member's own
(`EventTicket::prize_taken`, set by `EventCommand::PrizeTaken`, which stays
in the client), so it goes with leaving. Without the room in the Etc and
Use tabs she gives nothing yet.

The hub's way out, `ps01` (script `Zakum03`, `zpq_portal`), stays shut with
a pink line until the quest is won and, after, until the player has
claimed the prize. Aura's way out needs neither. Cosmic's warp of holders
of 4001109 to 921100000 is a custom left out.

## NPCs

| NPC | Where | Script |
| --- | --- | --- |
| Lakelis 9020000 | 103000000 | Runtime (`talk_about_event`), like Heracle |
| Cloto 9020001 | 103000800-804 | Runtime (`talk_to_cloto`): stage 1's quiz, stages 2-4's puzzles, stage 5's passes, then the reward and on to 103000805 |
| Nella 9020002 | 103000800-805 | Out to 103000890 (on 103000805 her own question) |
| Nella 9020002 | 103000890 | Home to 103000000 |
| Tory 1012112 | 100000200 | Runtime (`talk_about_event`): a menu of going up and how it is played |
| Growlie 1012114 | 910010000 | How it is played, the leader's rice cakes, or out to 910010300 |
| Tory 1012112 | 910010100, 910010400 | The reward once won, and home to 100000200 |
| Tommy 1012113 | 910010100 | Pig Town: the leader may take the party there once won |
| Tommy 1012113 | 910010200 | Out to 910010400 |
| Tommy 1012113 | 910010300 | Home to 100000200 |
| Red Sign 2040034 | 221024500 | Runtime (`talk_about_event`): a menu of going in and what the quest is |
| Balloons 2040036-2040044 | 922010100-900 | Runtime (`talk_to_balloon`): the stage's hand-in, stage 8's crates, the maze's line |
| Pink Balloon 2040045 | 922011000 | Off the bonus to 922011100 |
| Arturo 2040035 | 922011100 | The reward once won, and down to 221024500 |
| Sgt. Anderson 2040047 | 922010100-900 | Out to 922010000 |
| Sgt. Anderson 2040047 | 922010000 | Down to 221024500 |
| Wonky 2013000 | 200080101 | Runtime (`talk_about_event`): a menu of going in and what the quest is |
| Wonky 2013000 | 920010000 | Out to 920011200 |
| Eak 2013001 | 920010000 | Runtime (`opq_eak`): out of the light; the leader clears the cloud stage, the others go up to `st00` |
| Eak 2013001 | 920010100 | Runtime (`opq_eak`, `opq_hub`): the six pieces, then to the garden, the Grass and Minerva, to the leader only |
| Eak 2013001 | 920010200, 920010600 | Runtime (`eak_hand_in`): 30 of 4001050 / 40 of 4001052 for a piece of the statue, clearing the room |
| Eak 2013001 | 920010300 | Runtime (`opq_eak`, `opq_salion`): with no monster or drop left, the second piece of the statue, clearing the room |
| Eak 2013001 | 920010400, 920010500, 920010700 | Runtime (`opq_eak`): the music box, the head count, the levers; brings out the room's stone box and clears it |
| Eak 2013001 | 920010800 | Runtime (`opq_eak`): Papa Pixie |
| Minerva 2013002 | 920010100 | Once won, alone to 920011300 |
| Eak 2013001 | 920011000 | Runtime (`opq_eak`): to the leader only; once the guards are gone, the leader alone to 920011100 `st00` |
| Minerva 2013002 | 920011100 | Out of the first box broken; alone to 920011300 |
| Minerva 2013002 | 920011300 | The prize once won, and home to 200080101 |
| Eak 2013001 | 920011200 | Home to 200080101 |
| Adobis 2030008 | 211042300 | Runtime (`talk_about_event`): the Dead Mine, the Breath of Lava, the Eyes of Fire |
| Aura 2032002 | 280010000 | Runtime (`zpq_aura`): what it is, the leader's hand-in, out to 211042300; the prize once won |
| Ali 2030011 | 280090000 | Home to 211042300 |

Their lines are unsourced placeholder keys `kpq.lakelis.*`,
`kpq.cloto.*`, `kpq.nella.*`, `hpq.tory.*`, `hpq.growlie.*`,
`hpq.tommy.*`, `lpq.sign.*`, `lpq.balloon.*`, `lpq.pink.*`,
`lpq.arturo.*`, `lpq.anderson.*`, `opq.wonky.*`, `opq.eak.*`,
`opq.minerva.*`, `zakum.adobis.mine.*`, `zakum.aura.*`, `zakum.mine.*`
and `zakum.ali.*`.
