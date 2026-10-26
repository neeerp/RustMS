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
storage, treasure room and jails the win opens
(`v83ref/reports/a077-orbis-pq.md`, `a087-opq-garden-win-re.md`,
`a091-opq-bonus-jail-re.md`, `b185-opq-jails.md`). Zakum's Dead Mine (ZPQ), the first of the
trials before Zakum ([zakum](./zakum.md)), is played through too
(`v83ref/reports/a105-zakum-dead-mine.md`). Horntail's Trial Grounds
(HTPQ) has its entry, its first hand-in and its end rules; its rooms, lamp
and win come in later slices (`v83ref/reports/a116-htpq.md`). Zakum's own altar is an event
too, but of an expedition squad gathered across parties (`Event::squad`,
`net/src/squads.rs`): its team comes from the squad's roster rather than a
party, may come back in while it runs, and ends only when its last member
leaves (see the expedition in [zakum](./zakum.md)); Horntail's cave is
another ([horntail](./horntail.md)). The fight with
Papulatus is a party's event that ends that way too (`Event::until_empty`),
started by a portal rather than an NPC ([papulatus](./papulatus.md)).
Delli's rescue, for the Corsairs' 6410, is a party's event won by holding
out ([Delli's rescue](#dellis-rescue)). The Ludibrium Maze is played through
([the Ludibrium Maze](#the-ludibrium-maze)). Protecting Tylus, for the
warriors' 6192, is a party's event with nothing to win
([Protecting Tylus](#protecting-tylus)). Ellin Forest's Forest of Poison
Haze is played through
([the Forest of Poison Haze](#the-forest-of-poison-haze)). Herb Town's
Pirate Ship is played through ([the Pirate Ship](#the-pirate-ship)).
Magatia's Romeo and Juliet party quest has its entry, its way out, its
end rules and its first four stages; its other stages come in later slices
([Romeo and Juliet](#romeo-and-juliet)). Jr. Balrog's room, for quest 2245,
is taken alone ([Jr. Balrog's room](#jr-balrogs-room)), and so are King
Pepe's room in the Mushroom Castle, for quest 2330 ([King Pepe's
room](#king-pepes-room)), and its Wedding Hall, for quests 2332 and 2333
([the Wedding Hall](#the-wedding-hall)).

No event NPC offers Cosmic's "enable/disable Party Search" line. That
opt-out is Cosmic's own: in v83 the opt-out is the client's Party Search
Game Option, which the server never sees
([parties](./parties.md#party-search)).

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
2. Every member gets `ServerMessage::StageCleared{id, stage, exp}`
   (`exp` replaces the stage's EXP when given: the maze's coupons). Those on
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
Everyone starts on 922010100.

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
everyone starts on 920010000. Cosmic's Wristband exchange is left out.
On 920010000 Wonky asks before letting a player
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

## Orbis's jails

The sealed room under the hub (920010900, through the hub's hidden
portals) has three doors, `in01` to `in03` (Cosmic `party3_jail1`-`3`,
`scripts::OPQ_JAILS_IN`), to the jails 920010910, 920010920 and
920010930 (`events::OPQ_JAILS`, portal 0). They open with the storage, in
`opq_portal`, and say the storage's line until the quest is won.

Each jail holds 8 to 10 Lucidas (9300044: 3000 HP, no EXP, no drops,
never back) that players can't hurt, and 7, 6 and 6 levers (reactor
2008007, `lever1`.., which go up and down for good). A lever's hit drops
the spears of the trap of its number, `trapN` (Cosmic `2008007`,
`reactors::Action::Lever`: FIELD_OBSTACLE_ONOFF `trapN` 1, see
[combat](./combat.md)); the spears hurt players and Lucidas for about
1000. The second and third jails' extra traps have no lever. The tall
room's levers, named `1` to `5`, drop nothing.

The jail's way on, `in00` (Cosmic `party3_jailin`, `OPQ_JAIL_ON`), asks
the field (`FieldMessage::Jail`) how the levers stand against its draw,
made the first time anyone tries it (`events::draw_jail`: each lever up
or down by a coin, so none at all, an open way, is 1 in 128 or 64;
`RUSTMS_OPQ_JAIL1`-`3` pin it as the names of the levers up). With any
wrong, the whole jail sees `quest/party/wrong_kor` and hears
`Party1/Failed`, and the player reads `opq.notice.jail_levers` with the
count in pink. With none (`JailCheck::Solved`), the jail sees
`quest/party/clear` and hears `Party1/Clear`, and the draw is forgotten so
that the way stays open: through an open way (`JailCheck::Open`) no one
cheers. Either way the player goes, with the portal's sound, to the jail's
map + 2, portal 0.
There the critical box (2002018) starts at state 1 (`first_state`): its
state 0 has no event, and nothing moves it in Cosmic. It breaks on the
fourth hit and sprays as the treasure room's boxes do; so would the plain
box (2002017) of the jail's map + 1, which no way leads to. Their ways out
lead back into the jail, by `in00`.

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

## Horntail's Trial Grounds

At the Cave Entrance (240050000) the gatekeeper (2083001) takes a party of
exactly six in, all there at level 120 and up (`events::HTPQ`,
`EventKind::Htpq`): a menu of going in and what the quest is
(`talk_about_event`). The quest runs for 25 minutes on 240050100 to
240050310 and starts on 240050100. If any member leaves, the leader
included, the party is below six and the quest ends for everyone, as in
Cosmic. Leaving takes the quest's keys: the five Crystal Keys
(4001087-4001091), the Blue (4001092) and the Red (4001093). Its clears
give no EXP and no prize (Cosmic gives none either); the win sends the
party to the Entrance to Horntail's Cave (240050400). The Crystal of Roots
(2083002, see [horntail](./horntail.md)) takes a member out of it.

On 240050100 the way into the rooms (`in00`, script `hontale_BtoB1`,
`htpq_portal`) leads to the first room (240050101). It keeps back with a
pink line whoever holds the first Crystal Key, so that someone keeps it
for the hand-in, and whoever is alone on the map, so that one stays
behind. Talking to the gatekeeper there (`in_event`, `htpq_keeper`), the
leader hands in the five Crystal Keys and the party moves on to the lamp's
map (240050200) with its clock; a member who isn't the leader, or a
leader short of a key, is told so. On the light or dark map (240050300,
240050310) the same hand-in takes the Blue Key and six Red Keys and wins
the quest (`ClearStage` of the last stage).

The rooms 240050101-240050104 each have a seal (the `passKey` reactor
2408002, `Action::PassKey`) that takes only the room's own Crystal Key,
the 2nd to the 5th (`reactors::takes`, `events::htpq_room_key`); any other
item thrown on it stays on the ground. Taking it clears the room's stage
(`member_clears`): the room sees the clear effect and a pink line, and the
event drops the key free-for-all on 240050100 at keyDrop1 (`HTPQ_KEY_AT`,
`send_room_key`) with a pink line to that map, for the hand-in. Each
room's door (`hontale_Bopen`) refuses with a line until its stage is
cleared, then leads on to the next room. The last room's (240050105,
`htpq_last_seal`) holds for anyone but the leader holding the Blue Crystal
Key, who lifts it (`ClearStage(5)`, pink lines) and goes back to
240050100's `st00`, keeping the key; after that anyone goes through.

On the lamp's map (240050200) the lamp (`reactors::HTPQ_LAMP` 2408001,
named `light`) moves on with every hit, 0 to 1 to 2 to 3 and back to 0,
and never breaks. The way on (`east00`, `hontale_C`, `htpq_lamp_door`)
takes only the leader's word: lit (state 1) it moves the whole team to the
light map (240050300), dark (state 3) to the dark one (240050310), with
the clock; at 0 or 2 it tells the leader in pink to hit the lamp, and
anyone else that only the leader may choose. On the light map the
T-Skelegons (9300077) drop the Red Keys; on the dark one the six crates
(2402006) do, each breaking in four hits (both from the content pack).

Not yet: the stump (2083003).

## Delli's rescue

For the Corsairs' 6410 (`v83ref/reports/a118-delli.md`). Shulynch (1092008,
in the Nautilus's Training Room 120000104) asks a player on 6410 whether to
go after Delli and sends them to Looking for Delli 1 (925010000); without
the quest it says only its NX lines. The three Looking for Delli maps are
plain hunting maps leading on one to the next; Shulynch (1095000) on the
first takes anyone back to 120000104. On the third (925010200) Delli
(2095000) takes a party of one or two, all there at level 120 and up, into
Save Delli! (925010300) for six minutes (`events::DELLI`,
`EventKind::Delli`, a menu of going in and what to do). Delli doesn't ask
for the quest, as in Cosmic; only a player on it gets the record.

On Save Delli! the party protects Willi (9300162), a friendly mob
(`damagedByMob`) the other mobs hurt by running into it (`MobDamageMobFriendly`,
as the Moon Bunny). Its going loses the event at once and without a word
(`Event::survive`, `FieldActor::fail_event`): everyone goes back to
925010200. The time running out with it still there wins it instead
(`EventActor::time_up`): the clear effect, no EXP, and the other mobs leave
and stop coming (`FieldMessage::ClearMobs`, `MobSet::clear_hostile`, Cosmic
`killAllMonstersNotFriendly`). The way out (`s4mpt`, script `s4mind_end`,
`delli_portal`) holds with a pink line until the win, then sets 6411 to
"p2" for a player on 6410 and leads to the Nautilus Hideout Room
(925010400), where Delli completes 6410 and asks whether to go home to
120000104. Its lines are placeholders (`delli.*`).

The mobs come back on the field's usual seven seconds (Cosmic forces
fifteen).

## The Ludibrium Maze

Cosmic `LudiMazePQ` (`v83ref/reports/a119-ludimaze.md`; defaults in
`v83ref/lanes/QUESTIONS.md` a119). Rolly (9103001) in Ludibrium (220000000)
takes a party of three to six, all there at levels 51 to 70, into the maze
for 15 minutes (`events::LUDIMAZE`, `EventKind::LudiMaze`, a menu of going
in and the rules). Each member starts on a room of their own drawn from the
first 15 (809050000-014, `Event::entries`, `Event::draw_entry`), not one
for the whole party. The rooms' portals are plain ones leading from room to
room; only room 8 (809050008) leads to the goal room 809050015. Dying, or
the time running out, lands on the way out (809050017, the rooms'
returnMap and forcedReturn, and the event's exit), where Rolly (9103003) sends everyone
back to Ludibrium. Its lines are placeholders (`ludimaze.*`).

Each of the first 15 rooms has three boxes, which never come back and
change places each time the maze starts (`reactors::shuffled`, Cosmic
`shuffleReactors`). Four hits break one (`Action::MazeBox`): two lots of
mobs come out where it was, with a pink line to everyone in the room
(`ludimaze.summoned`). Boxes 8091000-4 bring out two each of their own
pair; box 8098000 brings out by the room it is in (`reactors::maze_box`,
Cosmic `8098000`). They drop nothing themselves; the mobs drop the coupons
(4001106) from the content pack's drops (`rewards.json`).

Pietri (9103000) in the goal room (`NpcScript::pietri`, routed by
`talk_to_balloon`) serves only the leader, and only once the whole team is
there (Cosmic `isEventTeamTogether`: as many on the copy's goal room as the
team has, `EventTicket::team_size`). Below 30 coupons he refuses (Cosmic's
hard floor; `events::MAZE_COUPONS_WANTED`). From 30 on he asks, takes all
the leader's coupons, and sends `EventCommand::WinWithCoupons(n)`: the
event wins with `MAZE_COUPON_EXP` (50) times `n` for every member, carried
in `ServerMessage::StageCleared`'s `exp` in place of the stage's, and takes
the team to the clear room 809050016 off the clock. The maze's win has no
clear effect or sound (Cosmic `clearPQ`). The maze has no `hand_ins`.

In the clear room, Rolly (9103002, `NpcScript::in_event` once won) asks
each member whether to take their present, gives one of
`events::MAZE_REWARDS` (Cosmic `setEventRewards`, all equally likely) and
sends them to the way out, which leaves the event. A member without a free
Equip, Use and Etc slot (`has_reward_slot`) is told so and stays.

## Protecting Tylus

El Nath's party quest, for the warriors' 6192
(`v83ref/reports/a123-elnath-pq.md`; defaults in
`v83ref/lanes/QUESTIONS.md` a123). Tylus (2020008) in the Chief's
Residence (211000001) answers a player on 6192 with a menu of going in and
what to do (`dialogue.rs`; otherwise the third-job chief's script). The
leader takes a party of one to four, all there at level 80 and up, into his
room (921100300) for ten minutes (`events::TYLUS`, `EventKind::Tylus`).

There the party protects Tylus (9300093), a friendly mob the other mobs
hurt by running into him, as Willi. Every 100 seconds he drops his Trust
(4031495; `friendly_drop_period`, Cosmic's `dropItemPeriod 10`, which our
Mob.nx lacks; the item is a quest drop for 6192 from
`reward_overrides.toml`, so `friendly_from` keeps quest items now). His
going says a line to the room (`tylus.fallen`) and nothing more: the event
goes on, and it never wins (`survive` is unset). The time running out
sends everyone to 211040100, as does the room's way out (`out01`, script
`s4common1_clear`, `tylus_portal`) for one without the Trust; with it, the
way leads to 921100301, where Tylus (2022004) thanks them and sends them to
El Nath (211000000, portal `in01`). The leader leaving the room, by either
way, ends the event for everyone (`Team::leave`), as in Cosmic. Tylus in
the Residence completes 6192 on the Trust; while 6192 is started he
doesn't offer the chief's other choices. Its lines are placeholders (`tylus.*`).

## The Forest of Poison Haze

Ellin Forest's party quest (`v83ref/reports/a125-ellin-pq.md`; defaults in
`v83ref/lanes/QUESTIONS.md` a125). Ellin (2133000) in the Deep Fairy
Forest (300030100) answers with a menu of going in and what the forest
holds. The leader takes a party of four to six, all there at levels 44 to
55, into its own copy of the forest for 30 minutes (`events::ELLIN`,
`EventKind::Ellin`), onto its entrance, 930000000. Its six stages are
930000100 to 930000600 (`stage_maps`); no stage gives EXP.

The entrance's way on (`east00`, script `party6_stage`, `ellin_portal`,
`scripts::ellin_way_on`) leads to the first stage. A stage's way on stays
shut, with a pink line (`ellin.stage1.shut`, `ellin.stage2.shut`), until
the party has cleared it. The first stage holds 50 Poisoned Lord Trees
(9300172, spawned once: `mobTime` -1, and a field's first spawn pass
fills every such point whatever its capacity); the death of the last in the
party's copy clears it (`events::cleared_by_killing`, the field's
`killed`, Cosmic `EllinPQ.monsterKilled`), with the clear effect.

On the second stage (930000200) the Poisoned Stone Bugs (9300173) drop a
Concentrated Poison (4001161) one time in five (`reward_overrides.toml`).
Nobody can pick it up, and one a player throws would vanish, so only a
bug's drop that lands on the pond (`reactors::ELLIN_POND`) works: the pond
takes it after 5 seconds, gives back a Diluted Poison (4001162) and fills
again 2 seconds on (see "timeOut" in `combat.md`). The spine
(`reactors::ELLIN_SPINE`, named `spine`) takes four Diluted Poisons, one
at a time, each 5 seconds after it lands on it; on the fourth it stays,
in its last state, and clears the stage for whoever threw it.

The third stage (930000300) is a maze of sixteen rooms, the party
arriving in the first. Each of the first fifteen has one script for all
its ways out (`party6_stage501` to `party6_stage515`,
`scripts::ellin_maze_way`): one time in ten it leads to the sixteenth
room's spot (`16st`, portal 16), and otherwise to the next room's (the
fifteenth's to the first's), with the portal sound. Ellin (2133001) in the
sixteenth room clears the maze for whichever member talks to her
(`ellin.guide.maze_end`, `npc_scripts::ellin_maze_end`), and her next
page moves the whole team to the fourth stage: the event lets any member
move the team to a stage that follows one any member clears, once it is
and only if the team hasn't been taken there yet, so a second member's
talk left open doesn't take it back (`member_moves_to`). The sixteen mileposts (2133002) let a
member out (`ellin.milepost.leave`).

The fourth stage (930000400) holds 20 Poison Flowers (9300175, spawned
once; one killed comes back, as its `revive` is itself). Ellin there
(`ClientActor::ellin_flowers`, `NpcScript::ellin_flowers`) first tells
each member what to do (`ellin.guide.flowers`, once per member: the
ticket's `hint_heard`, Cosmic `gridInsert`), then, while flowers are left,
gives a member with no Purification Marbles (2270004) ten of them
(`ellin.guide.no_room` if they don't fit), and asks one who has some
whether to leave (`ellin.guide.flowers.leave`), as she asks everyone once
all are caught (`ellin.guide.all_caught`). A marble catches a flower
below 40% of its HP within its reach and makes a Monster Marble (4001169,
the item's `info`, see the catch items in `inventory.md`); the last flower
caught, none being left to come back, clears the stage
(`events::cleared_by_catching`, Cosmic `monsterKilled` without a killer).
A member holding 20 Monster Marbles is sent to the leader
(`ellin.guide.to_leader`); the leader hands them in
(`ellin.guide.hand_in`), which moves the team to the fifth stage.

The fifth stage (930000500) holds nine stone boxes, which change places
each time the party's copy starts (`reactors::shuffled`): eight empty ones
(3002002) and one (`reactors::ELLIN_STONE_BOX`, 3002001) that breaks on
the fourth hit and drops the Purple Stone of Magic (4001163,
`reward_overrides.toml`), clearing the stage for whoever broke it. The
Sprite (2133004, `npc_scripts::ellin_sprite`) takes a leader holding the
stone, with the whole team, on to the sixth stage
(`ellin.sprite.to_altar`), and asks anyone else whether to leave
(`ellin.sprite.leave`).

On the sixth stage (930000600) the stone thrown on the altar
(`reactors::ELLIN_ALTAR`, an item reactor) brings out the Poison Golem
(9300180) at the altar, with a pink line (`ellin.altar.golem`). It dies
into its Charged form (9300181), and that into its Super-Charged one
(9300182), by their `revive`; that one's death clears the stage and wins
the forest (`events::cleared_by_boss`). The stage's way out (`out00`,
script `party6_out`, `scripts::ellin_way_out`) stays shut until then
(`ellin.stage6.shut`), then tells a member to wait for the leader
(`ellin.stage6.wait`) and takes the leader, with the team, to the prize
map.

The prize map, 930000800, is the event's exit and every party's own map
alike, not a copy: the time running out, leaving, and dying (the maps'
`returnMap`) all land there. The Super-Charged golem drops the Golem's
Marble (4001164) three times in five (`reward_overrides.toml`); thrown on
the map's fountain (`reactors::ELLIN_FOUNTAIN`, Reactor.nx 3008000, an item
reactor that stays used for about five minutes), it gives everyone on the
map 52,000 EXP (`reactors::Action::ExpToAll`, the field sending
`ServerMessage::GainExp`; no EXP rate applies). The way home
(`party6_stage800`, `scripts::ellin_home`) takes the forest's items and the
marble, gives an Altaire Fragment (4001198) while the fountain is used, or
refuses without room for one (`ellin.home.no_room`), and leads to
300030100. There Ellin (2133000) trades ten fragments for the Altaire
Earrings (1032060), and ten more each for the Glittering (1032061) and then
the Brilliant (1032072) pair, taking the pair before; only the bag counts,
not what is worn (`ellin.prize.*`). The numbers and the trades are
Cosmic's (QUESTIONS a125 4-5). Ellin (2133001) welcomes the party at the entrance
(`ellin.guide.welcome`) and, on the stages other than the third and
fourth, lets a member out to it (`ellin.guide.leave`); a member leaving, if fewer than four stay, or the leader
leaving, ends the event for everyone (`Team::leave`), as in Cosmic. The
event's items (4001162, 4001163, 4001169, 2270004) are taken from whoever
leaves. Its lines are placeholders (`ellin.*`).

## The Pirate Ship

Herb Town's party quest against the Lord Pirate
(`v83ref/reports/a128-pirate-pq.md`; defaults in
`v83ref/lanes/QUESTIONS.md` a128). Guon (2094000) Over the Pirate Ship
(251010404) answers with a menu of boarding and what waits aboard. The
leader takes those of the party there at levels 55 to 100, three to six of
them, the leader among them, into their own copy of the ship
(925100000-600; `events::PIRATE`, `EventKind::Pirate`); a member of
another level stays behind, as in Cosmic. Its six stages are 925100000,
100, 200, 300, 400 and 500, the last the Lord Pirate's; 201/202 and
301/302 are rooms off the third and fourth; beating him will take the team
to 925100600 (`clear_map`).

The clock starts at 4 minutes. The first time any member reaches the
second, third, fourth or fifth stage, it starts over at 6, and at the Lord
Pirate's at 8 (`events::clock_on`, Cosmic `changedMapInside`): the client
tells the event on the way there (`leaving_for`, `EventCommand::Reached`),
and the event restarts it once per map (`restart_clock`). Time running
out, the leader leaving, or fewer than three staying ends it for
everyone, who land on 925100700 (`exit`, every map's `returnMap`).

Guon (2094002) aboard lets anyone but the leader give up and go back to
251010404; the leader hears what the map asks (`pirate.guon.*`). On
925100700 he sends anyone back to 251010404 at once. The event's items
(the Old Metal Keys 4001117 and the Marks 4001120-4001122) are taken from
whoever leaves. Its lines are placeholders (`pirate.*`).

The first stage holds 26 monsters (Map.nx, mobTime -1, so none come back)
and thirteen boxes, whose places are shuffled for each copy
(`reactors::shuffled`, also on the third and fourth stages): six invisible
ones (2518000) only hold places. Each of the five mob boxes (2511000)
breaks on the fourth hit into three Ginseng Jars (9300109) and five
Bellflowers (9300110) (`Action::BringOutLots`); each of the two item boxes
(2512000, which links to it) into a spray of at least fifteen drops, its
loot topped up with piles of 30 to 59 mesos (`Action::Spray`). The last
monster aboard clears the stage for everyone (`cleared_by_killing`,
`member_clears`), with its effect and sound; stage clears give no EXP.
The way on (`next00`, script `davy_next0`) stays shut with a pink line
(`pirate.portal_shut`) while any monster is left, so the boxes' too, and
then leads to the second stage.

On the second stage the leader hands Guon twenty of each pirates' Mark in
turn: the Rookie's (4001120), then the Rising's (4001121), then the
Veteran's (4001122) (Cosmic `2094002`; `events::PIRATE_MARKS`). Only the
pirates of the Mark in turn come out, the Lord Pirate's Enraged Mr. Allis
(9300114), then Krus (9300115), then Captains (9300116), each kind at all
31 or 33 of its own points, kept filled (`events::only_mobs`,
`MobSet::only`, as Cosmic's `respawnStages`); their drops are the content
pack's (the Marks at 20%). Guon checks the bag for the Mark in turn
(`guon_marks`, with the team's count from `EventTicket::marks`), takes
twenty and tells the event (`EventCommand::HandInMarks`), which checks the
turn again, counts it (`Cleared::marks`) and has the copy swap its
pirates for the next Mark's (`FieldMessage::SwapMobs`,
`MobSet::swap_hostile`): those out go, giving nothing, and the others
come on the next spawn pass. The third clears the stage, with its effect,
and leaves it empty. The way on (`davy_next1`) opens only then
(`Effect::RequireStage`, with no item line of its own), saying so in pink
until it does (`pirate.marks_shut`).

The third and fourth stages are decks, with no monsters of their own in
Map.nx: each copy opens with twenty of the Lord Pirate's crew, five Krus
(9300124) and five Captains (9300125) at each end of each of its two
floors (`events::opening_mobs`, boarded as its field starts, as Cosmic's
`setup`), and none come back. Their boxes are shuffled as the first
stage's; each of the six mob boxes (2511001, which links to 2511000)
breaks into six Krus and six Captains more. The last monster aboard
clears the stage, and the way on (`davy_next2`, `davy_next3`) waits for
it as the first stage's does. Each deck's hidden portal (`in01`, type 8,
scripts `davy2_hd1` and `davy3_hd1`) leads down to its crew's room
(925100202, 925100302) whatever is left aboard; that room's Devoted Krus
and Captains are its own Map.nx spawns, kept coming while anyone is there
(Cosmic brings them out once). The bellflower rooms (925100201, 925100301)
are plain links from each deck's `in00`, and their `out00` leads back.
Each copy opens with ten Bellflowers (9300112) at its left end and ten
Bloody Bellflowers (9300113) at its right, on top of its own Map.nx
Bellflowers, which come out once. Its chest (2512001, `treasure1` and
`treasure2`, `reactors::PIRATE_CHEST`) starts hidden in state 0; once
no monster is left, the leader talking to Guon shows it
(`pirate.guon.chest`, an effect setting the named reactor to state 1,
which does nothing more if it is already shown). The Lord Pirate's key
(4031437, from the bosses only) dropped on it opens it into a spray of
at least fifteen drops, 50-100 mesos each besides the pack's list
(Cosmic `2512001`), and counts it for the run, two at most
(`EventCommand::ChestOpened`, `Cleared::chests`), whoever opened it. The
count will pick the Lord Pirate and the clear's EXP.

The fifth stage (925100400) is the doors: its Map.nx spawns come back as
any field's, and its four seals (2519000-3, named `sMob1`-`sMob4`,
`reactors::PIRATE_SEAL`) each take an Old Metal Key (4001117, 6% from
the pirates in the pack) dropped on them and stay set in state 1
(`reactors::Action::Seal`, `acts_staying`). A set seal stops the spawn
points in its box, 160 across and 75 down either way from it, as Cosmic's
`setAllowSpawnPointInBox` (`MobSet::deny_box`, kept until the field
renews); the monsters already out stay. The stage clears when all four
are set and no monster is left, whichever comes last (the seal's arm in
`show_reactor_hit`, and `killed` through `Field::sealed`). The way on
(`davy_next4`) waits for both, saying `pirate.portal_shut` in pink
(`Effect::RequireReactorsSet`, `RequireNoMobs`), and leads to the last
stage, where the first member through brings out the Lord Pirate once
(`EventCommand::PastTheDoors`, `FieldMessage::BringOut`) at (777, 140):
9300119 with no chest opened, 9300105 with one, 9300106 with two
(`events::lord_pirate`). Cosmic's `changeDifficulty` is not followed.

The Lord Pirate (420,000 HP as 9300119, 480,000 and 540,000 as the others)
keeps Mob.nx's stats at every party level. His death on a copy's last
stage frees Wu Yang (2094001): a pink `pirate.wu.freed` to the map, and Wu
Yang comes out where the Lord Pirate did (`Field::lord_pirate_falls`, from
`killed`; Cosmic `PiratePQ.monsterKilled`), once. The stage's reactor
2516000 is left unmapped (`v83ref/reports/a132-pirate-boss.md`; defaults
in `v83ref/lanes/QUESTIONS.md` a132). The monsters he called stay; Guon
says `pirate.guon.saved` once the map is bare. Wu Yang tells a member to
send the leader (`pirate.wu.wait`); the leader is thanked
(`pirate.wu.thanks`), and the OK clears the last stage
(`EventCommand::ClearStage`), which wins the ship: every member, wherever
they are aboard, earns 28,000 EXP with no chest opened, 35,000 with one
and 42,000 with two (`events::pirate_win_exp`, set by `EventActor::win`
and riding `StageCleared`), with no clear effect or sound (`quiet`, as
Cosmic's `clearPQ` there), and the team goes on to 925100600.

There, Wu Yang (Map.nx's) asks what they want (`pirate.wu.menu`, with
`pirate.wu.out` and `pirate.wu.hat`). Going out gives a Feather of Goddess
(4001158, `events::HPQ_REWARD`) and sends them to 251010404; with no Etc
room he says `pirate.wu.room_etc` and keeps them. The hat takes twenty
feathers and the hat before for the next of the Lord Pirate's Hats
(`events::PIRATE_HATS`: 1002572, then 1002573; Cosmic's third, 1003267, is
not in our Character.nx): `pirate.wu.hat_given` as it is made,
`pirate.wu.need_feathers` when short, `pirate.wu.room_equip` with no Equip
room, and `pirate.wu.best` to whoever holds the last one.

## Romeo and Juliet

Magatia's party quest against Yulete
(`v83ref/reports/b193-rj-pq.md`), from either side, each its own event
(`EventKind::Magatia(MagatiaSide)`, `events::MAGATIA`; Cosmic
`MagatiaPQ_Z` and `MagatiaPQ_A`, which differ only in their ids). Romeo
(2112004) in his room on the Zenumist side (261000011) and Juliet
(2112003) in hers on the Alcadno side (261000021) answer with a menu of
going in and what happened. The leader takes exactly four of the party,
the leader among them, all of levels 71 to 85 and in the room, into the
side's own copy of its maps (926100000-700 and 926110000-700) for 45
minutes. With only four, anyone leaving, by the exit, a warp, the party or
logging out, ends it for all, as Cosmic's `isEventTeamLackingNow` with a
minimum of four does.

Its seven stages, as Cosmic counts them for their EXP (0, 10,000,
20,000, 0, 20,000, 20,000, 0), are played on the library (x00000), the
hall below it (x00001), the beakers (x00100), the hub of files (x00200),
Yulete's room (x00203), the rope rooms' landing (x00300) and the boss's
room (x00401), `ZENUMIST_STAGES` and `ALCADNO_STAGES`.

The library's 26 books (Zenumist 2112007, Alcadno 2112013; Cosmic's
scripts of the same ids) hold, drawn once per copy when one is first
opened (`events::draw_books`, kept by the field as `books`), 7 bags of
500 mesos, 7 lessons worth 500 EXP, the side's letter (Zenumist 4001131,
Alcadno 4001130) and the switch; the other 10 hold nothing. Each gives
up what it held once (`ServerMessage::BookOpened`,
`NpcScript::magatia_book`). Cosmic multiplies the mesos and EXP by the
server's rates; ours don't move. A letter that doesn't fit is lost, as in
Cosmic. The switch lights the library's reactor `d00` (2618005, state 1)
for everyone there and clears stage 1, which gives no EXP. The 10 books
in the Alcadno hall hold nothing.

The library's way on (`rnj1_pt00`, `jnr1_pt00`) and the hall's
(`rnj1_out`, `jnr1_out`, to x00100) stay shut, in pink
(`magatia.portal_shut`), until their stage is cleared. The hall's 42
mobs (9300145 and 9300146, which don't come back) clear stage 2 with the
last kill (`cleared_by_killing`), for 10,000 EXP each.

The beaker room (x00100) has three beakers (2618000), each taking a Magic
Stone (4001132) per state up to state 6; a state left alone goes back
one after 9.5 seconds. Its mobs (9300147, which come back) drop the
stones at 40% (`reward_overrides.toml`; the pack has none). The third
beaker to reach state 6 (`net::reactors::Action::Beaker`, the field's
`beakers`) clears stage 3 for 20,000 EXP each, kills the room's mobs
and opens its door (`rnj2_door`, `jnr2_door`, 2618006, state 1); only
then does its way on (`rnj2_out`, `jnr2_out`, to x00200) open. A beaker
counts once, the first time it fills.

The hub of files (x00200) has 23 mobs (9300148, which come back) that
drop the Card Key (4001133) at 10% (`reward_overrides.toml`, Cosmic's
rate). A Card Key thrown at either of its side doors (`rnj3_out1` west,
`rnj3_out2` east; Reactor.nx 2618002 and 2618001) opens it
(`net::reactors::Action::SideDoor`), and with it the way back out of the
room behind it (x00201 `rnj31_out`, x00202 `rnj32_out`; 2618003 and
2618004), through the event (`ServerMessage::OpenEventReactor`,
`FieldMessage::OpenReactor`). Each portal (`rnj3_in0`, `rnj3_in1`,
`rnj_201_0`, `rnj_202` and their `jnr` twins) is shut, in pink, unless its
reactor has left state 0, and the doors shut again after 2 seconds, as
their state 1 times out in Reactor.nx. Cosmic's side doors stay open, as
its `hitReactor` never times out a state that only leads back. The rooms'
boxes trade places in each copy (`ReactorSet::shuffle`), but their ways
back stay put. The west room's box 2612002 holds the first file
(4001134) and the east room's 2612001 the second (4001135); the others
(2612003) hold potions and mesos, and the east room's 24 decoys are
2518000. Each box takes four hits.

Romeo (2112006) or Juliet (2112005) in the hub (`NpcScript::magatia_hub`,
the client's `magatia_hub`) takes the side's letter from anyone who
brings it, and everyone in the run hears, as a light-blue line, that they
were shaken (`EventCommand::Shocked`, once a run). From the leader with
both files, they take them, clear stage 4 (`talk_clears`; no EXP), send
the hub's mobs away (`Effect::ClearMobs`) and open its way on
(`rnj3_out3`, `jnr3_out3`, 2618007, `Effect::OpenReactor`) for 5 seconds;
`rnj3_out`/`jnr3_out` lead to x00203 only while it is open. After the
clear they open it again whenever it has shut. A member holding the files
is told to let the leader hand them in; anyone else is asked whether they
would rather retreat.

Yulete (Zenumist 2112000, Alcadno 2112010) comes out at (252, 243) in his
office (x00203) when the first member arrives there, and the field
keeps his mood (`net::events::Yulete`, the field's `yulete`). Talked to
(`FieldMessage::Yulete`, `events::yulete_talk`, Cosmic `2112000`), he
says he is leaving if he already means to; with anyone within 310 pixels
of him he says he is being watched and means to leave; if Romeo or Juliet
never read the letter (`EventTicket::shocked`) he calls the party sneaky
and means to leave; otherwise he is caught out. Ten seconds after the
first arrival, not scaled with the event, and only while someone is in the
office, he leaves (`build_uncontrol_npc`, then REMOVE_NPC 0x102), and
everyone in the run hears why in pink (`ServerMessage::YuleteLeft`,
`EventCommand::YuleteLeft`, `magatia.yulete.disgusted` if he was caught,
`magatia.yulete.fooled` if not); the event keeps which
(`EventTicket::caught_yulete`) for the boss's room. He leaves five of each
of his guards (9300143, 9300144) at each of (-455, 135), (0, 135) and
(360, 135). The last kill clears stage 5 (`cleared_by_killing`) for
20,000 EXP each and opens the office's door (`rnj6_out`, `jnr6_out`,
2618005, state 1); until then its way on (the same names) is shut, in
pink, and then leads to the landing (x00300).

On the landing, its four ways (`rnj4_r1`-`rnj4_r4`, `jnr4_r1`-`jnr4_r4`)
each lead into one rope room (x00301-x00304) the first time, and only
then (`EventTicket::take_room`): anyone else is told, in pink, that the
room is taken (`magatia.room_taken`). In a rope room each of ten rows
has four ropes, `pt<row><rope>` (`rnj5_rp`, `jnr5_rp`), of which one is
right; it leads to the row above (portal id + 4), and from the top row to
the top (`success`); any other leads back to the bottom (`fail`). Each
room draws its right ropes the first time anyone climbs
(`events::draw_climb`, `events::climb`; the harness pins them with
`RUSTMS_MAGATIA_ROPES`). Cosmic's Zenumist script draws them so that each
row's right rope differs in each of the four rooms; ours draws each room
on its own, as Cosmic's Alcadno script does. The first to reach the top
clears stage 6 for 20,000 EXP each, with no clear effect. The rooms' way
on (`out00`, a plain portal at the top) leads to the corridor (x00400),
whose way in (`rnj12_in`, `jnr12_in`, never shut) leads to the boss's
room (x00401).

There Yulete (2112000, 2112010) comes out at (200, 100) the first time
anyone enters (`npc_scripts::magatia_waiting`), beside the side's escort
(Zenumist 9300137, Alcadno 9300138) and two guards (9300150), whom the
map's spawner brings back as Cosmic's `respawnStages` does every 15 s.
The first time he is talked to (`FieldMessage::Yulete`, the field's
`frankenroid`, `events::yulete_in_lab`) he brings out his Frankenroid at
(250, 100): the mighty one (Zenumist 9300140, Alcadno 9300152) if he was
caught out in his office, otherwise the plain one (9300139, 9300151);
everyone in the run hears of it in pink (`EventCommand::FrankenroidOut`).
After that he bids them behold it, and once the run is won says he is
beaten. A mob walked into the escort hurts it, and its fainting is told
to the room (`magatia.juliet.fainted`, `magatia.romeo.fainted`) and kept
by the run (`EventCommand::EscortFailed`). The Frankenroid's fall clears
stage 7, which wins the run (`events::magatia_win_exp`): 90,000 EXP each
if the escort fainted, otherwise 105,000 for the plain one and 140,000
for the mighty one. Cosmic's Alcadno script pays 140,000 either way and
never sends anyone straight on (below); both sides do as its Zenumist
script does. The guards go, and Romeo (2112006) or Juliet (2112005)
comes out at (-370, -150). If the escort fainted they send each member
on to the last room (x00600); otherwise they give the side's gift
(Zenumist 4001159, Alcadno 4001160, which needs room) and send the member
on: to the last room, or, if the Frankenroid was the mighty one, first to
Yulete's traces (2112001, 2112011) in x00500, who send them on in turn.
In the last room Romeo & Juliet (2112018) thank each member and give
one of 23 rewards (`events::MAGATIA_REWARDS`, Cosmic's
`setEventRewards`), which needs a free slot in each of the Equip and Use
inventories, on the way back to the side's room. The traces also open
the dark pipe into Yulete's lab to the member (marker 32600, for Cosmic's
quest 7770, which `Quest.nx` lacks; see `map-travel.md`, `magatia_dark0`).

Inside, Romeo or Juliet asks whether a member would rather retreat
(`magatia.romeo.retreat`, `magatia.juliet.retreat`) and sends them to the
side's x00700, where everyone lands when the run ends (its `forcedReturn`
too). Its portal (`rnj_exit` on both sides' exits; Cosmic has a `jnr_exit`
our Map.nx never names) leads back to the side's room. The event's items
(4001130-4001135) are taken from whoever leaves. The maps'
`onFirstUserEnter` scripts (`StageMsg_romio`, `StageMsg_juliet`) do
nothing yet. Its lines are placeholders (`magatia.*`).

## Jr. Balrog's room

For "To Tristan's Tomb" (2245; Cosmic `BalrogQuest`). Mu Young's start
script `q2245s` (see [quests](./quests.md)) takes a player on 2238, at
level 50 and up, alone into their own copy of 910520000 for ten minutes
(`events::TRISTAN`, `EventKind::Tristan`), party or not. Only one may be in
it on a channel at a time (`runs: 1`, Cosmic's "someone in this map"
refusal): another is told so (`tristan.room_taken`, a placeholder) and
nothing is applied. The room's `life` holds the unknown Jr. Balrog
(9300326), spawned once. Its fall brings out Tristan's Spirit (1061015) at
(0, 115) (`FieldActor::tristan_balrog_falls`), where 2238 ends. The event
is never won: it ends when the player leaves by `out00` or the time runs
out, back to Mu Young's map 105100100, the room's `forcedReturn`.

## King Pepe's room

For "Stop the Wedding" (2330), which wants one each of the three Yeti
brothers King Pepe rides (3300005-3300007; Cosmic `KingPepeAndYetis`). The
East Castle Tower's (106021400) `TD_MC_enterboss1` takes a player on 2330
who has yet to beat one of them alone into their own copy of 106021500 for
twenty minutes (`events::KING_PEPE`, `EventKind::KingPepe`). Only one may be
in it on a channel at a time; another is told so in pink
(`king_pepe.busy`, a placeholder). Anyone else goes on to the Wedding Hall's
entrance (106021401). Each copy opens with one brother, drawn at random, at
(-28, -67) (Cosmic `pepeking_effect`), so a player may have to go in more
than three times. The event is never won: it ends when the player leaves by
`out01` (`out_pepeking`) or the time runs out, back to the tower. The way
out gives the Wedding Hall key (4032388), which "Where's Violetta?" (2332)
starts with, once all three are beaten and none is held, or stays shut with
no ETC slot for it (`king_pepe.key`, `king_pepe.key_no_room`).

## The Wedding Hall

The key starts "Where's Violetta?" (2332, `autoStart`, `autoAccept`): the
quest alert opens it and the client sends its start script `q2332s`, which
starts it without a word. The way to the hall's (106021402) door,
`TD_MC_enterboss2`, is locked (`wedding_hall.locked`) before then. From
2332 on, until "The Story of Betrayal" (2333) is done, it takes the player
alone into their own copy of the hall (106021600) for ten minutes
(`events::WEDDING_HALL_EVENT`, `EventKind::WeddingHall`; Cosmic
`MK_PrimeMinister`, which also lets a party of three in, but the quest says
"Must be alone"). Only one may be in it on a channel at a time; another is
told so in pink (`wedding_hall.busy`). It is never won: it ends when the
player leaves by `left00` or the time runs out, back to the door.

- Entering the hall (`findvioleta`) sets 2332's own record to 1, which its
  `Check/1 infoex` asks for (`OWN_RECORD_QUESTS`), so Violetta (1300002)
  ends it through `q2332e` for the 4,400 EXP of its `rewardSummary`.
- `q2333s` has her ask for help and name the Prime Minister (3300008), who
  comes out at (292, 143) (`Effect::BringOutMob`). Entering with 2333 taken
  and him not yet beaten brings him out too. His kill counts for 2333, and
  `q2333e` ends it for 15,000 EXP. He drops the Royal Seal (4001318) for
  those on "The Seal of the Empire" (2331; `reward_overrides.toml`).
- Once 2333 is done, the door leads straight into the hall itself, where
  `q2334s` (Violetta's face, Cosmic's 1,000 EXP) and `q2342s` (the seal, for
  one still on 2331 who left it behind) complete their quests as they start
  them (`FINISHED_BY_START_SCRIPT`). Anyone else asking for the seal is
  only told that she keeps it.
- Cosmic's refights through the Blocked Entrance (1300013) once 2331 is
  done are not there.

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
| Shulynch 1092008 | 120000104 | On 6410, to 925010000 |
| Shulynch 1095000 | 925010000 | Home to 120000104 |
| Delli 2095000 | 925010200 | Runtime (`talk_about_event`): a menu of going in and what to do |
| Delli 2095000 | 925010400 | Completes 6410; home to 120000104 |
| Rolly 9103001 | 220000000 | Runtime (`talk_about_event`): a menu of going into the maze and its rules |
| Rolly 9103003 | 809050017 | Home to 220000000 |
| Adobis 2030008 | 211042300 | Runtime (`talk_about_event`): the Dead Mine, the Breath of Lava, the Eyes of Fire |
| Aura 2032002 | 280010000 | Runtime (`zpq_aura`): what it is, the leader's hand-in, out to 211042300; the prize once won |
| Ali 2030011 | 280090000 | Home to 211042300 |
| Gatekeeper 2083001 | 240050000 | Runtime (`talk_about_event`): a menu of going in and what the quest is |
| Gatekeeper 2083001 | 240050100, 240050300, 240050310 | Runtime (`htpq_keeper`): the leader's hand-in of the keys |
| Tylus 2020008 | 211000001 | On 6192, runtime (`talk_about_event`): a menu of going in and what to do |
| Tylus 2022004 | 921100301 | Thanks, and on to El Nath 211000000 |
| Ellin 2133000 | 300030100 | Runtime (`talk_about_event`): a menu of going into the forest, what it holds, and the earrings for Altaire Fragments |
| Ellin 2133001 | 930000000 | Welcome |
| Ellin 2133001 | 930000300 | Clears the maze and moves the team on |
| Ellin 2133001 | 930000400 | Runtime (`ellin_flowers`): Purification Marbles, the leader's hand-in, or out |
| Ellin 2133001 | 930000100-600, others | Out to 930000800 |
| Milepost 2133002 | 930000300 | Out to 930000800 |
| Sprite 2133004 | 930000500 | Runtime (`ellin_sprite`): the leader with the stone on to the altar, or out |
| Guon 2094000 | 251010404 | Runtime (`talk_about_event`): a menu of boarding and what waits aboard |
| Guon 2094002 | 925100000-500 | The leader hears what the map asks, on 925100100 hands in the Marks, and in a bare bellflower room shows its chest; anyone else may go back to 251010404 |
| Wu Yang 2094001 | 925100500 | Out at the Lord Pirate's death; a member is sent to the leader, whose OK wins the ship |
| Wu Yang 2094001 | 925100600 | A menu: a feather and the way to 251010404, or the next hat for twenty feathers |
| Guon 2094002 | 925100700 | At once to 251010404 |
| Romeo 2112004 | 261000011 | Runtime (`talk_about_event`): a menu of going in and what happened |
| Romeo 2112004 | 926100000-400 | Asks whether to retreat, to 926100700 |
| Romeo 2112006 | 926100200 | Runtime (`magatia_hub`): the letter, the leader's files, the way on, or whether to retreat |
| Romeo 2112006 | 926100401 | Runtime (`magatia_saved`): once won, the gift and on to 926100600, or past 926100500 |
| Juliet 2112003 | 261000021 | Runtime (`talk_about_event`): a menu of going in and what happened |
| Juliet 2112003 | 926110000-400 | Asks whether to retreat, to 926110700 |
| Juliet 2112005 | 926110200 | Runtime (`magatia_hub`): the same on the Alcadno side |
| Yulete 2112000 | 926100203 | Runtime (`FieldMessage::Yulete`): caught out, watched, sneaky or leaving |
| Yulete 2112010 | 926110203 | The same on the Alcadno side |
| Yulete 2112000 | 926100401 | Runtime (`FieldMessage::Yulete`): brings out the Frankenroid, then bids them behold it, then is beaten |
| Yulete 2112010 | 926110401 | The same on the Alcadno side |
| Juliet 2112005 | 926110401 | Runtime (`magatia_saved`): the same on the Alcadno side |
| Yulete's traces 2112001 | 926100500 | Runtime (`yulete_traces`): on to 926100600 |
| Yulete's traces 2112011 | 926110500 | The same on the Alcadno side |
| Romeo & Juliet 2112018 | 926100600, 926110600 | Runtime (`magatia_thanks`): one of the rewards, and home to the side's room |
| Books 2112007 | 926100000 | Runtime (`BookOpened`): mesos, EXP, the letter, the switch or nothing |
| Books 2112013 | 926110000 | The same; in the hall (926110001) they hold nothing |

Their lines are unsourced placeholder keys `kpq.lakelis.*`,
`kpq.cloto.*`, `kpq.nella.*`, `hpq.tory.*`, `hpq.growlie.*`,
`hpq.tommy.*`, `lpq.sign.*`, `lpq.balloon.*`, `lpq.pink.*`,
`lpq.arturo.*`, `lpq.anderson.*`, `opq.wonky.*`, `opq.eak.*`,
`opq.minerva.*`, `zakum.adobis.mine.*`, `zakum.aura.*`, `zakum.mine.*`,
`zakum.ali.*`, `htpq.keeper.*`, `htpq.rooms.*`, `htpq.seal.*`,
`htpq.key.*`, `htpq.lamp.*`, `delli.*`, `ludimaze.*`, `tylus.*`,
`ellin.*` and `pirate.*`.
