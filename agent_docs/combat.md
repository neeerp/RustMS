# Mobs and combat

## Mobs

Map `life` entries of type `m` are mob spawn points (`x`, `cy` as the ground
height, `fh`, `f`, `mobTime`, `hide`); `game_data::mobs::MobData` reads their
templates from `Mob.nx` (`maxHP`, `exp`, `PADamage`, `bodyAttack`,
`notAttack`, ...). Each `FieldActor` owns a `field_mobs::MobSet`:

- **Spawning:** every visible spawn point (not `hide`, `mobTime` ≥ 0) spawns
  when the field starts. A killed mob comes back after `mobTime` seconds, at
  least 7 s, on the field's 100 ms tick, with a new object id (mobs and drops
  each have a range in `actor::object_ids`). Joining players get SPAWN_MONSTER for every live mob.
- **Control:** one player moves each mob (SPAWN_MONSTER_CONTROL 1). New
  mobs and joiners pick up uncontrolled mobs; leaving hands them to another
  occupant. MOVE_LIFE from the controller updates the position, answers
  MOVE_MONSTER_RESPONSE, and relays MOVE_MONSTER to the others; movement from
  anyone else is ignored. The last absolute fragment's foothold (0 while
  airborne) becomes the mob's current foothold. In HeavenClient, a controlled
  mob that can walk but neither jump nor fly turns 20 px before any foothold
  end that does not lead on to another floor, as v83 does
  (`FootholdTree::get_walk_edge`).
- **Aggro:** a hit hands the mob to the attacker with aggro
  (SPAWN_MONSTER_CONTROL 2) for `AGGRO_DURATION` (180 s, v83's minimum chase
  time). The previous controller gets SPAWN_MONSTER_CONTROL 0, and later hits
  from the same player within that time send nothing.
- **Client AI:** HeavenClient decides each mob's movement in v83's 90 ms
  steps. Each decision holds for a random time: 1 to 3 s when idle (walk
  left, right, or stand), and 0.8 to 1.2 s when chasing. A mob given aggro
  heads for the local player, keeping its direction until it is 100 px past
  them, and gives up after 180 to 240 s. A mob that cannot move turns to
  face the player and forgets them once they leave a 2000 x 1200 box.
- **Flying mobs:** a flier (`Mob::think_fly`, v83's FUN_007f8875) heads for
  a waypoint every 90 ms step. Idle, it roams a random foothold group for
  (the group's width + 40 to 239) / 8 waypoints, each from 50 px above to
  9 px below a random spot on one of the group's footholds. Chasing, each
  waypoint is up to 120 px across and 40 px up or down from the player, and
  it gives up after 59 to 118 of them. It steers across until within 10 px
  of the waypoint, beats up while the waypoint is higher, and glides once
  within 10 px of its height. It moves on once within 11 px, or once the
  player is 150 px from the waypoint. It sends its movement at each new
  waypoint and at least once a second, and between those updates another
  client's flier glides the way it faces.
- **Jumping mobs:** a mob that can walk and jump stays within the reach of
  its current foothold group and the group it spawned on, less 20 px a side
  (`FootholdTree::limit_jumper`). At a loose foothold end or a wall it stops
  and jumps on the next tick. It also jumps at a rising wall up to 40 px
  ahead (`FootholdTree::get_wall_ahead`), and on a fifth of its idle
  decisions. When chasing, it jumps toward a target on a higher group whose
  reach covers it, walks down steps toward a target on its own group, and
  jumps off an end toward a lower target only half the time.

Packets follow the v83 layout (16-byte temporary status mask after the mob
id); HeavenClient used to skip 22 bytes there and now skips 16. After the
stance come two footholds: the one the mob stands on, then its spawn point's.
v83 puts the mob on the first, at the point along it nearest its x, so a mob
whose `cy` is a pixel off a slope does not fall; with 0 it falls from its
position. HeavenClient does the same (`FootholdTree::place_on_fh`).

## Attacks

CLOSE_RANGE_ATTACK (`net::combat::parse_close_attack`) is checked before it
reaches the field (`validate`): the attacker must be alive, and a basic
attack (skill 0) hits one mob once, each hit at most twice the character's
basic maximum (`max_base_damage`, v83's weapon multiplier × main stat +
secondary stat, × weapon attack / 100). An attack naming a skill needs that
skill learned at some level; its `Skill.nx` level (`game_data::skills::
SkillLevel`) gives the mobs and hits allowed, the MP spent, and the cap: a
`fixdamage` skill (Three Snails: 10/25/40) hits for at most that, a `damage`
skill for that percentage of the basic cap, and a skill with neither is not an
attack. Too little MP refuses the attack. HeavenClient sends Three Snails here
for melee weapons. The field shows the swing (with the skill id and level, so
others see the skill's effect) to the others, applies the damage, sends the
attacker SHOW_MONSTER_HP, and on death KILL_MONSTER to everyone; the attacker
gets the MP left as STAT_CHANGED.

In HeavenClient a hit of at least the mob's `pushed` value puts it in its
`hit1` stance for about a quarter second, with a three-tick push that moves it
a few pixels (`Mob::knockback_ticks`), as in period footage.

KILL_MONSTER usually arrives before the swing connects on screen. In
HeavenClient each mob counts the hits queued against it (`Mob::expect_hits`,
from the player's own attacks and from other players' attacks as they
arrive), and a killed mob starts its death animation when the last of them
lands, or after 2 s if one never does.

A dead mob's EXP is shared by damage dealt (the top damager gets any
rounding remainder) and each share goes to that player's client actor
(`ServerMessage::MobKilled`), which adds it with `Player::gain_exp`. A level
gained sends the new level, AP, SP, max HP/MP (+14/+11 per beginner level,
the middle of v83's ranges) and shows SHOW_FOREIGN_EFFECT 0 to the others. As
in v83, current HP and MP are not refilled. Each level grants 5 AP, and 3 SP
once past beginner.

## Buff skills

USE_SKILL (`0x5B`, `handle::world::use_skill`) casts a skill that doesn't
attack. Only the beginner buffs do anything yet (Recovery and Nimble Feet, by
`skill_id % 10_000_000`, so Noblesse and Legend copies work too); others are
ignored. The level the client names must be the learned one, the skill must
be off cooldown, and the player alive with enough MP. The caster gets the MP
left, GIVE_BUFF (the temporary-stat masks match HeavenMS: Speed and Recovery
are both second-mask bits), and COOLDOWN when the level has a `cooltime`;
the map sees SHOW_FOREIGN_EFFECT 1 with the skill.

`game::Buffs` on the `Player` holds the active buffs and cooldowns; neither is
saved. A new buff replaces one on the same stat. The client actor sleeps until
the next deadline (`actor::client::buff`): Recovery heals its `x` every 5 s
while the player is hurt (HP stat update), and an expired buff sends
CANCEL_BUFF, since the client never ends buffs itself. Dying and changing
channel end every buff; cooldowns keep running.

## Spending AP and SP

The client locks its UI after SPEND_AP or SPEND_SP until the server answers,
so every request is answered, refusals with an empty STAT_CHANGED.

- **AP** (`Player::spend_ap`): SPEND_AP names the stat by its STAT_CHANGED
  bit: STR, DEX, INT, LUK, MaxHP (0x800), or MaxMP (0x2000). Base stats stop
  at 999 and max HP/MP at 30,000. One AP adds the middle of v83's per-job
  HP/MP range (`ap_growth`; a beginner gets +10/+7).
- **SP** (`Player::raise_skill`): the skill must belong to the character's
  job or one it advanced through (`game::skill::in_job_tree`). Its `Skill.nx`
  template (`game_data::skills`) gives the max level and the `req` skills it
  needs; skills marked `invisible` must be granted first. Fourth-job skills
  also stop at the character's master level. Three Snails, Recovery, and
  Nimble Feet share a beginner pool of one per level from 2, up to 6, rather
  than SP. The reply is STAT_CHANGED with the SP left, then UPDATE_SKILLS.

## Drops and pickup

When a mob dies, each player who earned EXP from it rolls their own drops
(`net::loot::roll`) from the content pack's table plus the committed v83
overrides (`game-data/reward_overrides.toml`): each entry rolls its `prob`,
mesos vary by ±25%, items take `min`..`max`. Quest-only entries roll only
while that quest is started and the player doesn't hold the item yet. Without
a content pack, mobs drop only the overrides. The client actor rolls (it has
the player's quests) and asks the field to place the loot.

The field's `field_drops::DropSet` spreads a kill's drops around where the
mob died and shows them falling (DROP_ITEM_FROM_MAPOBJECT 1); players
entering see lying drops (mode 2). Each drop lands on the map's footholds
(`game_data::footholds`, loaded with the field template): on the highest
ground below its column, searched from 50 px above the mob as v83 servers do.
A wall between the mob and the drop's column, or no ground under it, keeps
the drop on the ground below the mob, so drops don't end up inside a cliff or
in the air. Quest drops are shown only to their owner.
For 15 s only the killer may pick a drop up (quest drops: always only the
owner); drops vanish after 180 s (REMOVE_ITEM_FROM_MAP 0). Drops players
throw down are anyone's at once; see "Client moves" in `inventory.md`.

HeavenClient's `Drop` gives a falling drop its own, lighter gravity instead
of the player's: it launches at -2.9 px per tick and gains 0.042 px per tick,
so it rises about 100 px and is in the air about 1.1 s, as measured from 2008
GMS footage. Its horizontal speed is set from that air time so it lands on
its destination, and it turns about twice on the way down. A picked up drop
(REMOVE_ITEM_FROM_MAP 2) follows the looter and rises about 50 px, slowing
down, then disappears after about 0.37 s; it stays opaque except for the last
six ticks. An expiring drop (mode 0) fades out where it is.

ITEM_PICKUP (`0xCA`) needs the player within 800×600 of the drop. The field
removes it (REMOVE_ITEM_FROM_MAP 2 with the looter) and hands it to the
client actor, which adds mesos (MESO stat update, "You have gained mesos")
or the item (INVENTORY_OPERATION, "You have gained an item"). If the item
doesn't fit, the player sees "You can't get anymore items." and the drop goes
back on the ground.

## Monster Book cards

Every mob with a card (`Item.nx/Consume/0238.img`, matched by each card's
`info/mob`) also rolls it on each kill: 1% as in v83, or
`RUSTMS_CARD_DROP_RATE` (0 to 1; the harness sets 1). The card drops like any
item, but picking it up puts it in the Monster Book (`game::MonsterBook`)
instead of the bags: MONSTER_BOOK_SET_CARD (`0x53`: added byte, card, level)
and the card effect (user effect 13), which the map sees too. A page holds
five copies; a sixth is reported as not added and has no effect. The cover
(MONSTER_BOOK_COVER `0x39` in, MONSTER_BOOK_SET_COVER `0x54` out) only takes a
collected card, or 0 to clear it. Cards and cover persist
(`monster_book_cards`, `characters.monster_book_cover`) and travel in
SET_FIELD's character data (`net::codec::character`): the cover int, a 0 byte, a
short count of cards, each its number within `0238.img` (a short) and level
(a byte).

HeavenClient's `UIMonsterBook` (B, or the status bar's shortcut menu) is
the v83 book from `UIClassic.nx`, laid out from 2008 footage:

- The "!" tab is the summary: book level (Cosmic's formula: level 1, then
  each level needs 10 × level more distinct cards), total, special
  (`card / 1000 % 10 == 8`) and basic counts, and the cover card.
- The nine coloured tabs list each category's cards, 25 to a page: collected
  ones in colour with their count (or the full mark at five), the rest in
  grey (`Color::grayscale`, which the fragment shader turns to luminance).
  The search box finds a monster by name and jumps to its card.
- The right page shows the selected card's monster. More copies show more:
  its picture from one, HP and MP from two, then Episode, Dropping (item
  tooltips on hover) and Found In (maps grouped under bold street names) at
  three, four and five. These come from `String.nx/MonsterBook.img`.
- Right-clicking a collected card offers SET AS COVER / REMOVE COVER.

A new card also prints "[Card name] has been successfully registered in the
Monster Book." in the chat log in pink (`#FFA194`, from footage).
## Reactors

A map's `reactor` nodes (`game_data::MapReactorTemplate`) place reactors such
as Amherst's scrap boxes; their states come from `Reactor.nx`
(`game_data::reactors`, following `info/link`). In each state, the first
event of type 0–2 is what an attack does: it moves the reactor to that
event's `state`, optionally only for the skills in `activeSkillID`. A state
without events is the end.

The field's `field_reactors::ReactorSet` spawns every reactor on load
(SPAWN_REACTOR `0x117`, also sent to players entering). DAMAGE_REACTOR
(`0xCD`) hits one if the player is within 250×200 of it, by the field's idea
of where they are. A hit that leaves it short of its last state is shown to
everyone (HIT_REACTOR `0x115`); the hit into the last state removes it
(REMOVE_REACTOR `0x118` with the state it broke from, which HeavenClient uses
to play the breaking animation) and the hitter rolls its drops
(`net::loot::roll_reactor`) where it stood. A broken reactor comes back after
the map's `reactorTime` seconds, under a new object id since HeavenClient
keeps a broken reactor under its old one; one without a `reactorTime` stays
broken. With each state's hit animation, including the break, HeavenClient
plays `Sound/Reactor.img/<id>/<state>/Hit`, with the id unpadded as v83
formats it.

Reactor drops come from the content pack's table, keyed by reactor id, plus
`[reactors]` in `reward_overrides.toml`; see `server-content.md`. Reactor
`action` scripts (`mBoxItem0` and others) are not run.

## Getting hurt

TAKE_DAMAGE from a touch (-1) is accepted up to the touching mob's
`PADamage`, only for mobs with `bodyAttack` and without `notAttack`
(HeavenClient's `Mob::deals_touch_damage` applies the same rule, so other
mobs neither hurt nor knock back on touch). The client works out the
damage (`Character/TouchDamage.h`) by a pre-Big Bang procedure
([ayumilove](https://ayumilove.net/maplestory-formula/)). The mob's base
damage is a roll from 85% of `PADamage` up to it. The player's defense is
`1.2 STR + 0.5 DEX + 0.4 INT + 0.5 LUK` plus W.DEF from equipment, and 75%
of it, rounded down, comes off the base. The minimum is 1. The stat part is
hidden: the stat window shows only the equipment W.DEF. Period footage agrees.
Snails hit level 2–4 beginners for 1, and Shrooms and Blue Snails hit a
level 5 beginner for 1–2 (#125). The procedure has no level-difference
term and no avoid check. HP drops, others see DAMAGE_PLAYER, and at 0 HP the player is dead:
non-beginners lose 10% of their level's EXP; the client's respawn
(CHANGE_MAP with a target while dead) returns them to the map's return map
with 50 HP.

HeavenClient's `Player::update_death` notices HP reaching 0: the player
becomes a ghost (`Char::State::DIED`), the tombstone from `Effect.nx/Tomb.img`
drops behind them (drawn by `Char`) as `Sound.nx/Game.img/Tombstone` plays,
keys are ignored, and a "You have died."
notice opens. The ghost is the body's `dead` frame, which fades in. Head, hair
and equips have no `dead` frames, so `CharLook` puts the standing ones on the
ghost's neck, as the original client does. The ghost and its name tag circle
the tombstone clockwise, 9 pixels out, once every two seconds; the data has no
motion for this, so `Char::draw` offsets them. The radius and period were
measured from 2008 GMS footage. The notice's OK sends that respawn CHANGE_MAP. The SET_FIELD that follows
carries the new HP, which brings the player back up.

## Keys

New characters get v83's default keymap (`game/src/keymap.rs`), with actions as
HeavenClient numbers them: Ctrl → attack (53), Z → pick up (51), X → sit (52)
and B → Monster Book (59), among others. The client's DEFAULT button in Keyboard
Setting restores the same table.

## Tests

`field_mobs` unit tests cover spawning, control hand-off, movement, damage,
EXP sharing, and respawn; `net::combat` covers parsing, the damage cap, and
the broadcast; `mob_combat` (integration) runs two players in Dangerous
Forest through control, relayed movement, hits, kills, EXP, refused damage,
touch damage, a level-up seen by the other player, and a respawn;
`reactors` (integration) breaks two of Amherst's boxes, before and during
Pio's quest;
`drops_and_pickup` checks ownership, reach, pickup, replays, meso
persistence, and that the tutorial sentinel's quest drop needs its quest;
`spend_points` levels a beginner and checks AP, the beginner pool, refusals,
and that both survive logging out; `monster_book` fills a card's page from
six snails, checks the effect another player sees and the cover, and that
both survive logging out.
and that both survive logging out; `beginner_skills` has three players learn
Three Snails, Nimble Feet, and Recovery and checks their damage caps, MP,
buffs, cooldowns, Recovery's heal, what the others see, and that the buffs
end on time and on death.
