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
- **Revive:** a mob with `info/revive` (the Blue Perfume bottle, Yeti and
  Pepe, ...) lets those mobs out where it died once its `die1` animation is
  over (`MobInfo::dying_ms`, 500 ms without one), as Cosmic does. They fade
  in, facing its way, under its last controller, and never respawn. v83's
  SPAWN_MONSTER also has a -3 appearance that ties the new mob to the dying
  one, but by then the server has removed it, so they use -2.
- **Leaving on their own:** a mob with `info/removeAfter` (the Kitties in
  Magatia's cheese storage, 926120400, ...) is taken away that many seconds
  after it spawned (`LiveMob::leaves_at`), as Cosmic does: KILL_MONSTER with
  animation 1, no EXP, drops or revive mobs, and its spawn point fills again
  as after a kill. The v83 client never reads `removeAfter`.
- **Destroying themselves:** a mob with `info/selfDestruction` goes once a
  hit leaves it at its `hp` or less (a Boomer, 9400547, at 350 of 700),
  counting as killed, or, without `hp`, `removeAfter` seconds after it came
  (a Bomb, 9300166, at once), as in Cosmic. Either way KILL_MONSTER carries
  its `action` as the leave type (`LiveMob::leave_type`), which the v83
  client plays: 1 dies, 3 plays `skill1` (it just goes without one), and 2
  or 4 and up blow it up with `attack1` (FUN_0067961d, FUN_00663e5b). The
  explosion hits whoever is in reach like any attack, so its TAKE_DAMAGE
  (attack 0) comes after the mob is gone; the server takes it, as it takes
  any mob's damage by the mob id. The client never uses `attack1` otherwise
  (FUN_0066a517). Darkstars (8500003, 8500004, action 3) also send MOB_BOMB
  (0xC1, the object id) once a player is in reach of their attack's box,
  unflipped (FUN_0066e636); the server blows them up with leave type 4 and
  no EXP, as Cosmic does. HeavenClient does all this in `Mob::kill` and
  `Mob::update`.
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
- **Chase animation:** a mob with a `chase` animation plays it instead of
  `move` while it walks with a target, and sends stance 32 (33 facing left),
  as v83's FUN_005cc410 does. Its shoe switches to `info/chaseSpeed` when the
  chase starts. When the chase runs out, v83 does not reset the shoe, so the
  mob keeps that speed until the next chase or control change. Other clients
  draw stance 32 with that animation.
- **First-attack mobs:** a mob with `info/firstAttack` and without
  `notAttack` starts the same chase without being hit once the local player
  comes within |dx| / 10 + |dy| / 3 <= 40 of it (v83's FUN_005ce8c6). Only
  its controller's client checks this, and the server is not told. v83 also
  starts the chase when one of the mob's attacks could reach the player;
  HeavenClient does not yet.
- **Distance reports:** so that the player a firstAttack mob goes for is the
  one whose client checks, every client, controller or not, sends MOB_DISTANCE
  (`0xBD`: object id, distance) once a second for each such mob, as v83's
  FUN_005ce6dc does. The distance is the one above, plus 100 while the player
  is dead. Unless someone has aggro on the mob, the server
  (`MobSet::report_distance`) hands it to a reporter nearer than the
  controller's last report, with SPAWN_MONSTER_CONTROL 1, or to anyone who
  reports before the controller has. v83's server rule isn't visible from the
  client; this one keeps the mob with the closest player. v83 also sends the
  report for mobs with `info/pickUp`, but no v83 mob has it.
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

Packets follow the v83 layout (the mob's statuses, a 16-byte mask and its
entries, after the mob id; see "Mob statuses"). After the
stance come two footholds: the one the mob stands on, then its spawn point's.
v83 puts the mob on the first, at the point along it nearest its x, so a mob
whose `cy` is a pixel off a slope does not fall; with 0 it falls from its
position. HeavenClient does the same (`FootholdTree::place_on_fh`).

## Attacks

CLOSE_RANGE_ATTACK, RANGED_ATTACK, and MAGIC_ATTACK (`net::combat::
parse_attack`, one `AttackHandler` per `AttackKind`) share a layout; a shot
has 5 more bytes before its targets (an int, the USE slot it fires from, the
cash ammo slot, and a byte), and 4 more for Hurricane, Piercing Arrow, and
Rapid Fire. Each is checked before it reaches the field (`validate`): the
attacker must be alive, and a basic attack (skill 0) hits one mob once, each
hit at most twice the character's basic maximum (`max_base_damage`, v83's
weapon multiplier × main stat + secondary stat, × weapon attack / 100). An
attack naming a skill needs that skill learned at some level; its `Skill.nx`
level (`game_data::skills::SkillLevel`) gives the mobs allowed, the hits (a
shot's `bulletCount`, otherwise `attackCount`), the MP spent, and the cap: a
`fixdamage` skill (Three Snails: 10/25/40) hits for at most that, a `mad`
spell for twice v83's magic maximum (`max_magic_damage`: ((M² / 1000 + M) / 30
+ INT / 200) × `mad`, where M is INT plus equipment magic attack), a `damage`
skill for that percentage of the basic cap, Shadow Meso for v83's highest
roll (ten times one and a half its `moneyCon`, raised by its `x` percent on a
critical), and a skill with none is not an attack. A spell must name a skill.
Too little MP refuses the attack, as do too few mesos for the skill's
`moneyCon` (Shadow Meso's, raised by Shadow Partner's `y` percent while that
buff is on, as the v83 client checks).
HeavenClient sends a skill as the attack v83's `CUserLocal::DoActiveSkill`
makes it, whatever the weapon: a swing, a shot, or a spell
(`SkillData::flags_of`). Any other skill goes out as SPECIAL_MOVE. Skills
that charge first (Big Bang, Hurricane, Corkscrew Blow) aren't supported. v83
casts Three Snails as a spell; RustMS and HeavenClient treat it as a swing.
The field shows
the attack under its own opcode (with the skill id and level, so others see
the skill's effect, and for a shot the star, arrow, or bullet it fired) to the
others, applies the damage, sends the attacker SHOW_MONSTER_HP, and on death
KILL_MONSTER to everyone; the attacker gets the MP and mesos left as
STAT_CHANGED.

A mob with an `info/damagedBySelectedSkill` list (Kyrin's test
Octopirates, 9001005 and 9001006) is hurt only by those skills. v83 rolls 0
(MISS) for any other attack (`FUN_0068146c`), so the field applies none of
its damage (`MobInfo::hurt_by`), and HeavenClient rolls MISS for it as well.

A shot fires from the USE slot the client names, as in v83 (`shot_ammo`). The
item there must fit the weapon (stars for claws, arrows for bows and the
Arrow claw 1472063, crossbow arrows, bullets for guns) and cover the cost:
the skill's `bulletConsume` if it has one (Avenger's 3), otherwise its
`bulletCount` (Lucky Seven's 2), or 1 for a basic shot. Anything else refuses
the attack. The cost is taken from that slot and the attacker gets
INVENTORY_OPERATION. Skills v83 fires without ammo (Shadow Meso, Taunt,
Energy Orb, and some Cygnus and Aran skills; `AMMO_FREE`) use none. The
server tracks neither Shadow Partner, which doubles the cost, nor Soul
Arrow's buff, nor the cash ammo slot. HeavenClient, like v83, fires from the
first fitting stack that covers the cost (`Inventory::find_bulletslot`).

HeavenClient refuses a skill before sending it, with v83's lines in the skill
chat colour (`Player::can_use`, `Skill::can_use`, `Combat::can_use_move`). The
checks run in v83's order. A skill cooling down comes first. Then come the
costs: HP, MP, mesos (`moneyCon`; Shadow Meso's grows by Shadow Partner's `y`
percent while that buff is on), and the item (`itemCon`, named in the line).
The four cost lines share one two-second throttle, so a second refusal within
two seconds prints nothing. An attack with no weapon says so next. Then the
weapon (`skill_weapon::check`). A shot needs its job's weapon whatever
`Skill.img` says: a bow or crossbow for archers, a claw for thieves, a gun for
pirates, nothing for anyone else, with eight exceptions such as Taunt and
Energy Orb. Brandish needs a sword or an axe, Corkscrew Blow a knuckle,
Grenade a gun, and Poison Bomb a claw. These say "You cannot use this skill
with the weapon you're equipped with." The held shots (Hurricane, Piercing
Arrow, Rapid Fire) refuse anything but a bow, crossbow, or gun without a word.
Any other mismatch with the weapon `Skill.img` names, or the knuckle, sword,
or wand rules a few pirate and Cygnus skills have instead, says "This skill
cannot be used with the equipped item." Ammo comes last. RustMS charges a
special move's `itemCon`, and Shadow Meso exactly the mesos the client checks
for. The skill's description gives each level a range around its
`moneyCon`, so v83's server may have charged a varying amount.

HeavenClient rolls Shadow Meso's hits as v83 does (`shot_path::meso_damage`,
FUN_0078df87): ten times a whole number of mesos between half and one and a
half its `moneyCon`, with its own `prop` and `x` for a critical in place of
the user's, then a percent less for each level the mob is above the user. It
misses as a blow does but ignores the mob's defense.

HeavenClient picks targets as v83 does (`Combat::find_targets`). Melee and
magic attacks take the mobs whose body overlaps the attack range in the order
they entered the map (`MapObjects::entry_order`), not the closest ones, so a
basic attack can pass the mob in front for one that spawned earlier. A mob
that is spawned again under the same oid keeps its place. Ranged attacks take
the closest mobs, which is what v83's outward sweep finds.

`AttackResult::damagelines` keeps the targets in that order, and the attack
packet lists them in it. Without a bullet, every hit on a mob lands at once,
at the attack's first hit frame (`Char::get_attackdelay(0)`), and each later
target lands a little after (`hit_timing::target_delay`): melee 70 ms per
target up to 280 ms, the multi-target spells 50 ms, and Chain Lightning
100 ms.

In HeavenClient a hit of at least the mob's `pushed` value puts it in its
`hit1` stance for about a quarter second, with a three-tick push that moves it
a few pixels (`Mob::knockback_ticks`), as in period footage.

A regular attack's stance is picked at random from v83's list for the
weapon's `info/attack` (`Character/Look/AttackStances.h`). Spears and polearms
swing T2, P1, and P2 and stab T1 and T2. Wands and staves swing O2. Knuckles,
and claws without stars, stab O1 and O2. Guns without bullets swing T1, or T2
two times in three. Claws throw with swing O1, O2, or O3.

A bow, crossbow, claw, or gun with no ammo still attacks, as in v83: the
regular attack becomes that weak melee hit at a tenth of the damage, with no
notice (`RegularAttack::can_use`, `Player::prepare_attack`).
With ammo, a regular attack still whacks instead of shooting when a mob is
right in front: v83 picks the melee stance first and looks for a mob in its
afterimage range, cut to reach at most 65 px forward
(`Gameplay/Combat/PointBlank.h`, `Combat::find_whack`). Only when there is
none does it shoot.

A shot leaves the user 28 px up and, for a regular attack, as far forward as
the whack would reach (skills shoot from the user). Each bullet flies straight
to where the shot's row crosses the target's body, or to the end of the range,
taking 1.5 ms per pixel. Bullets after the first leave 120 ms apart for stars
and some skills, and several bullets spread 14 px apart around the target.
Every hit on a target lands at the attack's hit frame plus 1.5 ms per pixel
between the shot's start and the target, not when a bullet arrives
(`Gameplay/Combat/ShotPath.h`, `Combat::extract_effects`).

A shot at one mob hits the first mob in its path, not the closest one in a
box. v83 searches 20 px steps from the shot's start out to its range. Each step
reaches 1 px above and below the shot per 4 px it is from the user, so a mob
well below a raised user is missed. The nearest step with any mob in it gives
the one that entered the map first. The range is the skill's own `range` if it
has one. Otherwise it is 300 px plus Eye of Amazon's range for bows and
crossbows, 200 px plus Keen Eyes' for claws, and 200 px for guns
(`shot_path::range`, `Combat::find_shot_target`). Other players' shots leave
out their range skill, whose level the client does not know.

Platforms stop such a shot. v83 draws a line from 28 px above the user's feet
to where the shot hits its target, or to the end of its range, and stops it at
the first platform the line enters from the open side. That means a floor from
above, or a wall or ceiling from the side it faces. Walls and ceilings only
count when they have no group or the group of the platform the user last stood
on. A line through the end of a platform only stops if it ends up behind the
platform joined there too. A stopped shot loses its target, and its bullet
flies from where it started to the platform (`shot_path::blocked`,
`Combat::find_platforms`). The line of sight in v83's other attack paths, such
as shots at several mobs, is not copied yet, and other players' shots never
stop.

Most single-target spells, such as Energy Bolt and Magic Claw, are aimed the
same way (`shot_path::aimed_spell`). The search starts 50 px in front of the
user and reaches 300 px, whatever the skill's `lt`/`rb`. A spell with a ball
flies it from that start to the hit, as one ball however many times it hits,
and its damage lands 1.5 ms per pixel of that straight path after the hit
frame. Platforms stop it as they stop a shot. A spell without a ball is never
stopped. Area spells such as Thunder Bolt, Big Bang, Heal, and the Demons
still take every mob in their range. Chain Lightning picks its first mob like
an aimed spell in v83, then chains from it; neither is copied yet.

Damage numbers follow v83 (`Gameplay/Combat/DamageLayout.h`). The first digit
comes from the larger `NoRed1` set and the rest from `NoRed0`. Each digit
overlaps the previous one by 3/5 of its part right of the origin, and digits
alternate 3 px up and down, with 1 px of random drop. Criticals use `NoCri`,
start 30 px in, and sit on the `NoCri1/effect` banner. Hits on the player use
`NoViolet`, and a 0 shows `Miss`. A number is centred on the mob's head, 15 px
above it plus 30 px per hit (15 px for Demolition). It stays opaque for
400 ms, then fades over 600 ms, rising 30 px over the whole second.

KILL_MONSTER usually arrives before the swing connects on screen. In
HeavenClient each mob counts the hits queued against it (`Mob::expect_hits`,
from the player's own attacks and from other players' attacks as they
arrive), and a killed mob starts its death animation when the last of them
lands, or after 2 s if one never does.

SHOW_MONSTER_HP shows the mob's HP bar and name tag in HeavenClient
(`Mob::show_hp`) for 5 s, as v83 hides them 5 s after the player's last hit.
The bar is v83's 52x10 box (`MobHpBar`), centred over the frame's body rect
with its top 30 px above the rect's top. Party members' bars use the same box
filled red. As in v83, the name is orange for a mob 20 or more levels above
the player, yellow for one 20 or more below, and white otherwise. A "Lv. N"
tag (`Basic.img/ShowLevel`) sits left of the name, except for bosses, mobs
whose `info` hides their level, HP or name, and 9xxxxxx mobs outside 94xxxxx
and 96xxxxx.

A dead mob's EXP is shared by damage dealt (the top damager gets any
rounding remainder) and each share goes to that player's client actor
(`ServerMessage::MobKilled`), which adds it with `Player::gain_exp`. A level
gained sends the new level, AP, SP, max HP/MP (+14/+11 per beginner level,
the middle of v83's ranges) and shows SHOW_FOREIGN_EFFECT 0 to the others. As
in v83, current HP and MP are not refilled. Each level grants 5 AP, and 3 SP
once past beginner. A warrior's Improving MaxHP Increase (1000001), a
Brawler's Improve MaxHP (5100000) and a magician's Improving MaxMP Increase
(2000001) add their `x` to each level's growth and their `y` to an AP's
(`net::skills::growth_bonus`).

## Buff skills

USE_SKILL (`0x5B`, `handle::world::use_skill`) casts a skill that doesn't
attack. `net::skills::buffs` maps each buff skill to the stats it gives (the
level's attack, defense, accuracy, avoidability, speed and jump, plus the
skill's own stat); a skill missing from it is ignored. The level the client names must be the learned one, the skill must
be off cooldown, and the player alive with enough MP. The caster gets the MP
left, GIVE_BUFF (the temporary-stat masks match HeavenMS: Speed and Recovery
are both second-mask bits), and COOLDOWN when the level has a `cooltime`;
the map sees SHOW_FOREIGN_EFFECT 1 with the skill.

`game::Buffs` on the `Player` holds the active buffs and cooldowns; neither is
saved. A new buff replaces one on the same stat. The client actor sleeps until
the next deadline (`actor::client::buff`): Recovery heals its `x` every 5 s
while the player is hurt (HP stat update), and an expired buff sends
CANCEL_BUFF, since the client never ends buffs itself. Right-clicking a buff
icon sends CANCEL_BUFF (`0x5C`, the skill id), which ends that skill's buffs
and answers with CANCEL_BUFF. Dying and changing channel end every buff;
cooldowns keep running.

A skill that reaches the party (`skills::reaches_party`) carries a slot mask
after the level: 0x80 alone, else bit `1 << (5 - slot)` for each member the
client found in the skill's area, the caster included, with slots in joining
order. The handler passes the skill on as `WorldAction::PartySkill`; the
directory sends it to the members in those slots on the caster's map, and
each member's client actor applies it when they are alive, out of the Cash
Shop, and within the level's `lt`/`rb` around the caster plus 100 px
(`skills::in_area`). Members get the buffs without MP or cooldown, or for
Heal the caster's per-member share of HP, and SHOW_ITEM_GAIN_INCHAT /
SHOW_FOREIGN_EFFECT type 2 (`skill, level`), which plays the skill's
`affected` animation on them.

A few skills do more than buff:

- **Resurrection** reaches only dead members, raising them with full HP; the
  living, the caster included, are left alone. HeavenClient, like v83, plays
  the action and effect but sends nothing when a party member casts it with
  no dead member in range (`skill_use::sends`); alone it is sent with 0x80.
- **Time Leap** ends every cooldown but its own (COOLDOWN with 0 for each),
  for the caster and the members it reaches.
- **Chakra** heals only the caster, by a random amount between
  `luk * 2.3 * y / 100` and `luk * 3.5 * y / 100`, using LUK with equips
  (`skills::chakra_heal`). The client only lets it be cast below half HP;
  the server doesn't check, since its max HP leaves out equips.

These and Mystic Door have neither buff stats nor a mob debuff, so
`skills::acts_alone` lets them past the check that refuses such skills as
unsupported.

## Spending AP and SP

The client locks its UI after SPEND_AP or SPEND_SP until the server answers,
so every request is answered, refusals with an empty STAT_CHANGED.

- **AP** (`Player::spend_ap`): SPEND_AP names the stat by its STAT_CHANGED
  bit: STR, DEX, INT, LUK, MaxHP (0x800), or MaxMP (0x2000). Base stats stop
  at 999 and max HP/MP at 30,000. One AP adds the middle of v83's per-job
  HP/MP range (`ap_growth`; a beginner gets +10/+7). As in v83, the client
  only asks for MaxHP or MaxMP from level 20, and then only after the player
  confirms a warning that it may leave the character weaker.
- **SP** (`Player::raise_skill`): the skill must belong to the character's
  job or one it advanced through (`game::skill::in_job_tree`). Its `Skill.nx`
  template (`game_data::skills`) gives the max level and the `req` skills it
  needs; skills marked `invisible` must be granted first. Fourth-job skills
  also stop at the character's master level. Three Snails, Recovery, and
  Nimble Feet share a beginner pool of one per level from 2, up to 6, rather
  than SP. The reply is STAT_CHANGED with the SP left, then UPDATE_SKILLS.

## Drops and pickup

When a mob dies, its drops roll once (`net::loot::roll`), for whoever dealt
it the most damage of the players still on the field, as Cosmic does. They
come from the content pack's table plus the committed v83 overrides
(`game-data/reward_overrides.toml`): each entry rolls its `prob`, mesos
vary by ±25%, items take `min`..`max`. A mob worth no EXP still drops and
counts for quests. Quest drops are the entries
the overrides tag with a quest, and quest items (`info/quest`) that only one
quest's completion check (`Check.img/<quest>/1/item`) asks for. They roll
only while that quest is started and the player holds fewer than the check
asks for (any number if it doesn't name the item). Without a content pack,
mobs drop only the overrides. The client actor rolls (it has
the player's quests) and asks the field to place the loot.

The field's `field_drops::DropSet` spreads a kill's drops around where the
mob died and shows them falling (DROP_ITEM_FROM_MAPOBJECT 1); players
entering see lying drops (mode 2). Each drop lands on the map's footholds
(`game_data::footholds`, loaded with the field template): on the highest
ground below its column, searched from 50 px above the mob as v83 servers do.
A wall between the mob and the drop's column, or no ground under it, keeps
the drop on the ground below the mob, so drops don't end up inside a cliff or
in the air. Quest drops are shown only to their owner.

For 15 s only the owner may pick a drop up, or with a party, the owner's
party as it was at the drop (quest drops: always only the owner). Drops of
mobs with `publicReward` are anyone's at once, as are those of mobs with
`explosiveReward`, which fly out 40 px apart instead of 25. The packet's
ownership type tells the client which (v83 FUN_0050483a, whose pets follow
it): 0 with the owner's character id, 1 with the party id, 2 for anyone (and
for any drop once the 15 s are over), 3 for an explosive drop. Drops vanish
after 180 s (REMOVE_ITEM_FROM_MAP 0). Drops players throw down are anyone's
at once; see "Client moves" in `inventory.md`.

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
back on the ground. Equipment a mob or reactor dropped comes with its stats
varied (`net::items::give_dropped`, Cosmic's `randomizeStats`): each stat it
has moves up to a tenth of itself either way, at most 5, or 10 for defense,
HP and MP. Equipment a player threw down keeps its stats.

Nobody picks up an item with `info/pickUpBlock`, the invisible items party
quests drop (Transparent Item 4001074, ...). The v83 client leaves them out
of both the pickup key and pets' looting (FUN_005d5224 in FUN_0050463c and
FUN_0050483a), and so does HeavenClient (`Drop::blocked`); the server refuses
them too (`Loot::blocked`).

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

A new card also prints "[Card name] has been successfully recorded on the
Monster Book." in the chat log, and a card for a full page prints "This card
is already full in the Monster Book. This card will disappear.". Both use
chat type 0xc, as in v83 (`#FFAFAF`).
## Mob statuses

Players' skills put temporary statuses on mobs (`game::MobStat`, by their
bit in v83's 128-bit mask). MOB_STAT_SET (0xF2) shows one put on to the
whole map and MOB_STAT_RESET (0xF3) shows it wear off; both spawn packets
carry the statuses a mob has. Entries come in the client's decode order
(`MobStat::DECODE_ORDER`), not bit order.

- USE_SKILL carries the mobs the client found in the skill's area.
  `skills::mob_debuffs` says what such a skill puts on (Threaten, Slow,
  Seal, Doom, Shadow Web, Ninja Ambush), and the field rolls its `prop` for
  each mob still in the area (`FieldRequest::MobDebuff`). The Crashes and
  Dispel take mob buffs off instead.
- An attack's hits carry `skills::on_hit`: the attack skill's own status
  (stuns, freezes, poisons, Disorder's lower attack and defence, Taunt's
  higher defence) and a learned Venom's poison. Each mob hit for more than
  0 rolls each.
- Taunt also puts on Showdown (bit 11), which no v83 client reads: a mob
  that dies with it gives its value in percent more EXP and makes each of
  its drops that much likelier (`Kill::showdown`, `loot::roll`; the
  OdinMS-family rule).
- Bosses shrug off Stun, Freeze, Seal, Doom and Web. A mob immune to poison
  (`elemAttr` S1) or ice (I1) shrugs off Poison or Freeze.
- Poison, Venom and Ambush take HP each second on the field tick
  (`MobSet::hurt_by_statuses`), the mob's `fixedDamage` instead if it has
  one, never below 1 HP, and nothing if the status's skill can't hurt the
  mob (`damagedBySelectedSkill`). The damage counts toward the inflicter's
  EXP share, and they see the HP bar. Every client shows the numbers itself
  (HeavenClient `MobHurts`), MISS where nothing is taken.
- Statuses wear off on the field tick.

HeavenClient keeps a mob's statuses (`MobStatParser`): Slow feeds its speed,
Doom shows it as a Blue Snail, Stun, Freeze and Web hold it, and the
reason skill's `mob` effect plays. The formulas for poison, Venom and Ambush
come from OdinMS-family servers; `v83ref` report 157 has the detail.

### Mists

Poison Mist (2111003) leaves a cloud (an affected area) as well as
attacking. `skills::mist` says what a skill's cloud puts on, and the attack
asks the field for one (`FieldRequest::Mist`). The field keeps them
(`field_mists::MistSet`): a mist covers the skill's `lt`/`rb` box around the
caster, mirrored when they face right. It starts 800 ms after the cast,
reaches every 2.5 seconds for the live mobs in it that lack the status,
rolling `prop`, and lasts the skill's `time`. AFFECTED_AREA_CREATED (0x111)
shows it to the map and to players who join; AFFECTED_AREA_REMOVED (0x112)
lifts it. The reach interval and the 800 ms come from OdinMS-family servers.

HeavenClient's `MapMists` lays the skill's `tile` clouds over the area as
v83 does (`MistLayout.h`, a diamond of random tiles), draws them above the
map's objects, and fades them out over a second when the mist lifts.
`v83ref` report 160 has the client's detail.

### Mystic Door

A Priest's Mystic Door (2311002) opens a door in the field leading to the
field's return map. USE_SKILL carries where it stands. `skills::door_town`
refuses it in towns, in maps whose `fieldLimit` has bit 8 or whose id has 9
as its millions digit, within 50 px of a portal other than a start point,
and where there is no return map, as the v83 client does. The refusal
unlocks the client like any other.

The client actor keeps its one door (`runtime/src/actor/client/door.rs`)
and closes it after the skill's `time`, on a new cast, a channel change,
and logout. The field shows the door to everyone on the map
(SPAWN_DOOR 0x113, opening when just cast; REMOVE_DOOR 0x114). The
directory tells the owner (SPAWN_PORTAL 0x43, which v83 uses for a door
when alone) and the party members on the owner's channel (party op 0x25,
and the door section of the party block) where it leads.

USE_DOOR (0x85) names the door's owner and whether the player stands in
the town. The directory lets through the owner and members of the owner's
party on its channel, standing in the door's town or its field, and refuses
others with `BLOCKED_MAP` 6. From the field a player goes to the town's
door portal for the owner's party slot (`pt` 6, in id order); from town
they go back to the door, which SET_FIELD's chase position (`net::door_warp`)
puts them at.

In HeavenClient, `DoorRules.h` holds v83's rules without the map:

- Where a door may open (`door_rules::may_cast`).
- The box a player must stand in to use one: 20 px to either side, 50 px
  above and below.
- Which door stands at which of a town's door portals: alone, the
  player's at the first; in a party, each member's at the portal of their
  slot.

`Combat::request_door` refuses a cast with v83's messages.
`MapDoors` (`Gameplay/MapleMap`) draws the doors. A field door opens
(`cDoor`) and stands 1.8 s later (`mDoor` in its `Frame`); a removed one
fades over a second. A door is placed on the ground at the next update,
not when SPAWN_DOOR arrives, which can be before the map's footholds have
loaded. The town's doors are drawn at their portals. Up first tries the
map's portals. A door portal has no warp, so Up falls through to the doors
(`Stage::check_doors`): the town's if the map is a town, else the
field's. The chase position SET_FIELD carries lands on the ground. The
client looks for the ground from 5 px above it, since the rounded position
can sit just below a slope.

`v83ref` report 161 has the client's detail.

## Summons

Puppet, Silver Hawk, the Octopus, Elquines, the Knights' souls and the like
call up a summon (`net::summons`, the field's `field_summons::SummonSet`).
USE_SKILL for a skill in `summons::kind` carries where the caster's client
put it and which way it faces after the level. The client puts Puppets 200
px ahead on a foothold within 80 px, the Octopi 45 px ahead, and the rest at
the owner (v83 `FUN_0096b140`). The cast costs MP and any `itemCon`, for
example a Summoning Rock for Silver Hawk. It also gives a buff that shows
the summon's icon and lasts its `time`: Puppet's own stat for summons that
stay put, the combo counter for the rest. The field then takes the summon
(`FieldRequest::Summon`). If the spot is more than 250×150 px from the
owner, it uses the owner's position. The owner's earlier summon of that
kind is dismissed. SPAWN_SUMMON (`0xAF`) shows it to the map and to players
who join.

The owner's client moves it and decides when it attacks. The server checks
the rest:

- **MOVE_SUMMON** (`0xAF` in, `0xB1` out) is relayed to the others.
- **SUMMON_ATTACK** (`0xB0` in, `0xB2` out) is refused when any of these
  holds:
  - it hits more mobs than the skill's `mobCount`;
  - it hits one mob twice;
  - a hit exceeds `summons::max_damage`, which is twice what the owner's
    client can roll before multipliers, so it is the owner's main stat ×
    2.5 plus their other stats for a physical summon, or the v83 spell
    formula for a magic one, times the level's `pad` or `mad` percent.

  Otherwise the hits go through the field's usual damage and kill path.
  A hit that does damage can also put a status on its mob
  (`summons::inflict`):
  - Silver Hawk and Golden Eagle stun at the level's `prop`.
  - Elquines and Frostprey always freeze; their data has no `prop`.

  Each status lasts the level's `x` seconds. That is inferred: the client
  only shows what it's sent, and OdinMS-family servers hold every summon's
  status for 4 s.
- **DAMAGE_SUMMON** (`0xB1` in, `0xB4` out) hurts a Puppet. Puppets are the
  only summons that can be hurt, and their HP is the level's `x`. At 0 HP
  the Puppet breaks: REMOVE_SUMMON reason 2, and the owner's buff ends.
- **BEHOLDER** (`0xB2` in, SUMMON_SKILL `0xB3` out) is a Dark Knight's
  Beholder healing its owner with Aura of the Beholder (the level's `hp`)
  or buffing one stat with Hex of the Beholder (`summons::support`). Hex's
  stats are numbered in the order its levels add them: weapon defense,
  magic defense, accuracy, avoidability, weapon attack. Its buff lasts the
  level's `time`. The owner must know the skill, and Hex's level must give
  the stat. The field allows each no sooner than 10 s after the Beholder
  comes, then the level's `x` seconds apart, with 0.5 s of slack for the
  client's clock. The others see the Beholder play the action the owner
  sent.

REMOVE_SUMMON (`0xB0`) reasons:

- 0: the summon expired when its buff did.
- 1: silently, for example when the owner rejoins the same map.
- 2: a Puppet broke.
- 4: it was replaced.

When the owner changes maps, a summon that follows comes along: it spawns
again with no entrance. Stationary summons end there. The others drop a
player's summons when the player leaves, with no packet.

HeavenClient's `MapSummons` (`Gameplay/MapleMap`) draws them and plays their
`summon` animations. `SummonRules.h` holds v83's rules without the map:

- Followers glide 7 px a step to 70 px from the owner (42 for the souls).
- Fliers circle a random spot near the owner, or near their spawn if they
  stay put.
- Any summon jumps to an owner more than 300×200 px away.

`Combat::update_summons` makes the owner's summons attack every 3 s
(Gaviota and Wrath every 1.5 s):

- The mage summons hit the side of their `lt`/`rb` box with more mobs.
- The others hit the nearest mobs within their `range/r`.
- The spreading kinds hit one mob and then up to `mobCount`−1 more around
  it, 120 ms apart.
- Each hit lands `attackAfter` plus the projectile's flight time after the
  attack starts.

A Puppet draws the mobs already chasing the owner to it for 180–240 s.
A mob touching it hurts it by its attack squared, scaled by the Puppet's
level (`summon_rules::touch_damage`), and the owner sends DAMAGE_SUMMON.

A Beholder heals its owner when it may and they are hurt, else buffs
them:

- It first does so 10 s after it comes, then every Aura's or Hex's `x`
  seconds.
- In Berserk, it doesn't heal the owner out of it.
- Hex picks one of the stats its level gives. Later stats are likelier:
  the n-th of k is picked with weight n out of k(k+1)/2. If the owner's
  buffs already give at least that stat, Hex tries the lower stats from
  the top for one they lack (`summon_rules::hex_pick`).

Where HeavenClient differs from v83:

- Ball sprites aren't drawn.
- Followers send their paths the way fliers do.
- The fly speed (0.2 px/ms) is inferred.
- Range targeting is a ±`r` × ±100 px box.
- Whether the owner is in Berserk is inferred to be their HP at or under
  its `x` percent.

`v83ref` report 158 has the client's detail.

## Reactors

A map's `reactor` nodes (`game_data::MapReactorTemplate`) place reactors such
as Amherst's scrap boxes; their states come from `Reactor.nx`
(`game_data::reactors`, following `info/link`). Each state lists events
(`<state>/event/<n>`), each with a `type` and the `state` it moves to. A
state without events is the end.

A basic attack can swing at a reactor instead of attacking mobs. Event
types 0 to 4 take a swing: 0 any, 1 and 2 only facing right or left, and 3
and 4 the same but in the air. Of a state's events, a swing sets off the
first of the best rank (`reactors::swing_rank`, v83 0x736091): a side or
air event beats a plain one, and a plain one is better on the ground.

The field's `field_reactors::ReactorSet` spawns every reactor on load
(SPAWN_REACTOR `0x117`, also sent to players entering). DAMAGE_REACTOR
(`0xCD`) swings at one if the player is within 250×200 of it, by the
field's idea of where they are. A hit that leaves it short of its last
state is shown to everyone (HIT_REACTOR `0x115`, with the event and the
client's delay); the hit into the last state removes it (REMOVE_REACTOR
`0x118` with the state it reached) and the hitter rolls its drops
(`net::loot::roll_reactor`) where it stood. A broken reactor comes back after
the map's `reactorTime` seconds, under a new object id since HeavenClient
keeps a broken reactor under its old one; one without a `reactorTime` stays
broken.

| Op | Direction | Body |
| --- | --- | --- |
| 0xCD DAMAGE_REACTOR | client to server | `i32` object id, `i32` kind (0 a swing, 1 a skill), `i32` flags (bit 0 facing left, bit 1 on a foothold), `i16` delay in ms, `i32` skill |
| 0x115 HIT_REACTOR | server to client | `i32` object id, `u8` new state, `i16` x, y, `u16` delay, `i8` event of the old state (-1 its own hit), `i8` cooldown in 100 ms |
| 0x118 REMOVE_REACTOR | server to client | `i32` object id, `u8` state reached, `i16` x, y |

The server ignores skill swings (kind 1); only guild quest reactors take
them.

Reactor drops come from the content pack's table, keyed by reactor id, plus
`[reactors]` in `reward_overrides.toml`; see `server-content.md`. Reactor
`action` scripts (`mBoxItem0` and others) are not run.

### Client

HeavenClient's `Reactor` follows v83's FUN_00734688. A swing or a
HIT_REACTOR waits out its delay, then plays the hit animation of the state
the reactor left: `<old>/event/<n>/hit`, or `<old>/hit` without one. Then it
shows the new state. A swing whose event the server hasn't said yet (the
swinger's own, before HIT_REACTOR) picks it from the old state with the
swing's flags. REMOVE_REACTOR into a last state waits 400 ms, does the
same pick with the last swing's flags, plays the animation, and removes
the reactor when it ends; if no event fits, the reactor just goes. Removed
in a state it could still leave, a reactor plays no hit and goes when its
current animation ends. With each hit animation it plays
`Sound/Reactor.img/<id>/<state>/Hit`, with the id unpadded as v83 formats
it.

A basic attack swings at a reactor first (`Combat::swing_at_reactor`,
v83's FUN_0095719b): over the attack's range, it takes at most one reactor
that is idle, out of its cooldown, and in a state with a swing event. The
reactors are tried in the order they entered the map. The first whose
middle passes v83's "in front" test wins, else the last the range reaches.
Facing left, v83 negates the middle rather than the difference, so the
test there asks whether the middle is at or left of minus the user's x.
HeavenClient keeps that.
A swing sends the attack (`0x2C`, no mobs) and then `0xCD`, and the attack
hits no mobs. The rules are in `Gameplay/MapleMap/ReactorRules.h` and
tested in `tests/reactor_rules.cpp`.

One deviation: before every basic attack, v83 checks that no item, stat
or skill request is waiting on the server and 200 ms have passed since the
last (`ExclRequest::USE_WAIT`). HeavenClient checks this only before
swinging at a reactor, so a request the server leaves unanswered can't stop
a player attacking.

Not yet: skill hits (event type 5 and `0xCD` kind 1) and touching a
reactor (`0xCE`), which only the Guild Quest's reactors (`6109xxx`) and
`9222000` use, and events of types 100 and up (an item dropped on a
reactor, a timeout). v83 also skips a swing at a reactor whose
`info/quest` isn't in progress, but no v83 reactor has one.

## Mob attacks

`Mob.nx` lists a mob's attacks as `attack1`, `attack2`, ... with an `info`
each (`game_data::mobs::MobAttack`; HeavenClient's `Mob::AttackInfo`). The
`type` says how an attack reaches: 0 hits a box, 1 shoots the player it
aims at, 2 throws a ball, and 3 and 4 strike a row of cells (3 only cells
with ground under them). The geometry is in
`Gameplay/MapleMap/MobAttackRules.h`, from v83's FUN_0066a517 (the pick)
and FUN_0066d9c0 (DoAttack); details are in the v83 notes for L173.

- **The pick:** a controlled mob that is chasing the local player, not
  acting, standing or flying, and not held picks an attack on its 90 ms
  step (`Mob::pick_attack`). Each attack whose MP cost (`conMP`) the mob
  can pay and whose range covers the player, facing either way, is a
  candidate. Physical attacks skip a player in Dark Sight, and row attacks
  wait 2.5 s after the last. One candidate is taken at random.
- **Playing it:** the mob stops, turns, and plays `attackN` for its length.
  Its movement goes out with action 12 + the attack's index (shifted left
  one, plus 1 facing left) and the attack's data: the target's id << 2 for
  a shot, the ball's aim point as two shorts, or the row's cells as a
  bitmask. The server (`MobSet::act`) takes the MP, or shows the move
  without the attack if the mob lacks it. Its MOVE_MONSTER_RESPONSE carries
  the mob's MP and whether it may attack next: whenever the client asked
  (70% of chasing moves) and the mob has attacks. Only then does the client
  pick one. The server relays the action and data to the others in
  MOVE_MONSTER, and their clients play the same attack from the path's
  start.
- **Hits:** every client checks the attack against its own player, after
  `attackAfter` ms: the box or cell against the player's body (a
  `jumpAttack` box misses an airborne player), the shot if it was aimed at
  this player and in reach, and the ball's point every frame of its flight
  (`r` px at `bulletSpeed`). The `effect` plays after `effectAfter`, the
  `hit` animation plays on a player who takes damage, and a row attack
  shows its `areaWarning` on each cell.

HeavenClient leaves out `tremble` (the screen shake), `doFirst`, element
resistance, hits on summons and other mobs, and attacks on a Puppet.

### Mob skills

A mob's `info/skill` entries list the `MobSkill.img` skills it has, each with
a level, the animation it plays (`action` n means `skilln`) and when its
effect shows (`effectAfter`). The server decides when a mob casts; the
controller's client plays the cast and the server applies it
(`game::mob_skill` says what each skill does).

- **The grant:** when a controller's move asks to attack, the server
  (`MobSet::act`) may also let the mob cast one skill, picked at random
  from those it can pay for, that are off cooldown (`interval` seconds),
  whose `hp` limit the mob is at or under, and, for a summon, whose
  `limit` of live summons isn't reached. A sealed mob casts nothing. The
  skill and level end MOVE_MONSTER_RESPONSE; the client keeps a grant until
  another ack grants something else (FUN_0066c23b). An unused grant lapses
  on the server after 10 s.
- **The cast:** a mob let attack casts a granted skill it has before it
  picks an attack (FUN_0066e7af): it stops, plays `skilln`, queues the
  level's `effect` after `effectAfter`, and sends action 21 + n - 1 with
  data skill | level << 8 | the animation's length << 16. The server checks
  the skill and level against the grant, the action against the entry,
  and the MP, and applies the skill once the animation has played (3 s at
  most). Other clients replay the animation and effect from MOVE_MONSTER.
- **What's cast:** the buffs (100-103 and the area versions 110-113, Haste
  115 and 156, the immunities 140-142, and 150-157) give MOB_STAT_SET
  statuses whose reason is skill | level << 16, on the caster or on every
  mob in its `lt`/`rb` area, with chance `prop`, for `time` seconds. Power
  Up and Magic Up set the PowerUp and MagicUp stats to `x`, a percent the
  client scales the mob's physical or magic hits by. Heal (114) gives the
  mobs in its area `x` HP and `y` MP, up to their maxima, shown to everyone as a blue
  number (DAMAGE_MONSTER with a negative amount). Summon (200) spawns its
  listed mobs at the caster under the caster's controller, up to `limit`
  alive, with SPAWN_MONSTER's appearance set to `summonEffect`; clients
  play `Effect.nx` `Summon.img/<summonEffect>` on them. Summoned mobs don't
  respawn.
- **Diseases** (120-126, 128, 132) reach the players in the area; see
  below.
- Not cast yet: dispel (127), banish, poison mist, and the damage counters
  (143-145).

### Diseases

A disease is a buff stat the server puts on a player, whose skill (the
reason) is the mob skill | level << 16 and whose value is the level's `x`
(1 without one): Seal (120) 19, Darkness (121) 20, Weakness (122) 30, Stun
(123) 17, Curse (124) 31, Poison (125) 18, Slow (126) 32, Seduce (128) 39
and Confuse (132) 51 (`game::mob_skill::inflicted`).

- **Catching one:** a cast lands by its `prop`, rolled once, on every
  player in the caster's `lt`/`rb` box (mirrored when it faces right), or
  without a box on its controller. A mob attack with `disease` and `level`
  brings one by that level's `prop` when it does damage. A dead player, or
  one who has the stat already, catches nothing (`catch_disease`). The
  player gets GIVE_BUFF for `time` seconds; the map sees the shown ones
  (all but Slow) in GIVE_FOREIGN_BUFF with the reason as the value (Poison
  sends its `x` then the reason). The server ends it on time or on death,
  and the player can't cancel it.
- **Effects on the server:** Poison takes `x` HP each second, never the
  last one. Curse halves the EXP from kills (as Cosmic does). Darkness's
  lower accuracy isn't applied.
- **Dispel (2311001):** for each party member reached it rolls its `prop`,
  then cures Poison, Seal, Darkness, Weakness, Curse and Slow (not Stun,
  Seduce or Confuse) with CANCEL_BUFF and CANCEL_FOREIGN_BUFF. A sealed
  player can still cast it.
- **What the client does** (HeavenClient `Character/Diseases.h`, from v83
  CUserLocal): Stun stops all input, attacks and skills; Seduce moves the
  player by its `x` (1 left, 2 right, 3 jump, 4 down, 6 and 7 jump left and
  right) and stops jumps, attacks and skills; Confuse reverses the arrows
  (teleport too); Weakness stops jumping; Seal stops every skill but
  Dispel; Slow sets the walking speed to its `x` before the 80-140 clamp.
  Each disease shows `MobSkill.img/<skill>/level/<lvl>/affected` on the
  character, over the face when its `pos` is 1, otherwise at the feet; the
  player also gets a grey chat notice (none for Slow or Confuse) but no
  buff-bar icon.

## Getting hurt

TAKE_DAMAGE names the mob attack that hit (its index, or -1 for a touch).
A touch counts only for mobs with `bodyAttack` and without `notAttack`
(HeavenClient's `Mob::deals_touch_damage` applies the same rule, so other
mobs neither hurt nor knock back on touch); an index must name one of the
mob's attacks.

The client works out the damage as v83 does (`Character/MobDamage.h`, from
FUN_0079309f and FUN_0079345e):

- **Physical:** the attack's `PADamage` (the mob's, for a touch or an
  attack without one), P, rolls P² × 0.8–0.85 / 100. W.DEF and a stat
  factor take some off. Below the W.DEF expected of the player's job and
  level (Base.wz `StandardPDD`, baked into the client) the shortfall adds
  some back, more against a higher level mob. A miss roll weighs the
  player's avoid against the mob's accuracy first.
- **Magic:** the mob's `MADamage`, M, rolls M² × 0.75–0.8 / 100, less a
  share of M.DEF and stats (more for magicians); a separate miss roll.
- **Afterward:** the damage is at least 1 unless missed. A `deadlyAttack`
  takes all but 1 HP, and an `mpBurn` attack does 1. Invincible takes its
  `x` percent off a physical hit.

Period footage agrees: snails hit level 2–4 beginners for 1, and Shrooms
and Blue Snails hit a level 5 beginner for 1–2 (#125). The server
(`game::mob_damage`) refuses more than the best roll with no W.DEF, plus
the most a shortfall can add with doubled base stats.

v83 sends that damage before the other guards (`CUserLocal::SetDamaged`,
FUN_009581a9), and the server applies them once (`net::combat::taken`):

- **Achilles** keeps `x` per mille of the hit, for a Hero, Paladin, Dark
  Knight, or Aran who has learned it.
- **Magic Guard** pays its `x` percent in MP, at most the MP left; under
  Infinity the MP isn't spent. It leaves a `deadlyAttack` or `mpBurn`
  attack alone, as Cosmic does.
- **Meso Guard** (a mob's attack or touch only) saves half the hit at
  `x` percent of that in mesos. Short of mesos, it saves what they buy.
- **Power Guard** (a touch only, `net::combat::power_guard`, 0x9587c6)
  sends `x` percent of the hit back at the mob, at most a tenth of its
  max HP, halved for a boss, and at most a `fixedDamage` mob's fixed
  damage. An `invincible` mob takes none, and none goes if the mob's
  avoid beats the player's accuracy times 100 over (255 + 10 per level
  the mob is above them). What it sends back comes off the hit and the
  shown number; the server deals it to the mob as one of the player's
  hits (`FieldRequest::Reflect`). The client says it reflected with the
  block below; the server then uses its own buff and the mob's template.
- **Mana Reflection** (a mob's spell only, `net::combat::mana_reflection`,
  0x958d50) works the same way once the client's roll of its `prop`
  comes up, but sends back `x` percent, at most a twentieth of the mob's
  max HP, bosses alike, and spares the player nothing. Its accuracy is 5
  for each 10 of INT and of LUK, times 0.85. The buff's value is the
  skill level; v83 reads all three skills' data from 2121002's.

The rest comes off HP; a `deadlyAttack` also takes all but 1 MP, and an
`mpBurn` attack its `mpBurn` in MP. Poison ticks get Achilles too
(0x94b113). The client shows over the player's head the hit less Achilles
and Meso Guard, but not Magic Guard (0x9593b8); others see the same number
in DAMAGE_PLAYER.

A hit of -1 is a dodge. HeavenClient rolls them as v83 does: a Night
Lord's or Shadower's Shadow Shifter dodges any hit, a miss too, at its
`prop` percent (0x9584f6), and a Hero's or Paladin's Guardian blocks a hit
that lands while they wear a shield, at its `prop` per mille, after Power
Guard has sent its share back (0x958692). A dodge shows no number, only
the skill's `special` animation, and Shadow Shifter smiles for 1.5 s. The
server accepts -1 only from a Night Lord or Shadower who has Shadow
Shifter, or (with a shield worn and a guard byte) a Hero or Paladin with
Guardian. After the mob's id and object id, a mob's TAKE_DAMAGE carries
its direction, the reflect percent, the guard byte (1 for an attack
Guardian blocked, 2 for a touch), a 14-byte block when reflecting or for a
touch Guardian blocked (whether it was Power Guard, object id, hit action,
the mob's and the player's positions), and a stance byte, set when the Stance roll kept
the player from being knocked back. The server reads the tail by its length
(older clients send none) and passes the reflect block and stance on in
DAMAGE_PLAYER (`mob::Hurt`): `cid, from, damage`, then for a mob `mob_id,
left, reflect percent (and its 10 bytes), stance`, then the shown number,
or -1 and the dodging skill. A receiver shows the reflect percent of the
damage over the mob, at most a tenth of its max HP for a physical hit
(HeavenClient's `Mob::show_reflect`). Hits with no mob (-4) end with a byte: 1
poison (ignored, the server ticks poison itself), 2 a mist (Achilles
applies), or 3 Stop Motion (no guards, never fatal). At 0 HP the player is
dead (`death_penalty`): non-beginners lose 10% of their level's EXP. Holding a
charm keeps the EXP instead and uses up one charm. The charms are tried in
Cosmic's order: the Safety Charm (5130000), the Easter Basket (4031283), then
the Easter Charm (4140903). A grey chat line confirms it (SHOW_ITEM_GAIN_INCHAT
mode 6). For a Safety Charm it gives the charms left and the days the next one
lasts; any other charm is named. The server also keeps the spawn point nearest
where the player fell.

The client's respawn is a CHANGE_MAP with a target while dead. It returns the
player to the map's return map with 50 HP. When the player chose a Wheel of
Destiny (5510000) in the death dialog (the wheel byte after the portal name),
one wheel is used instead. The player then revives on the same map, at the
kept spawn point, and a grey line gives the wheels left (mode 0x15). After a
charm kept their EXP, the player revives with 30% of their max HP and MP added
rather than at 50 HP, as in Cosmic.

After a hit, HeavenClient ignores touches for 1500 ms, as v83 does. If the hit
did damage, the player blinks meanwhile: half brightness for 60 ms, then
normal for 60 ms. Other players don't blink.

HeavenClient's `Player::update_death` notices HP reaching 0: the player
becomes a ghost (`Char::State::DIED`), the tombstone from `Effect.nx/Tomb.img`
drops behind them (drawn by `Char`) as `Sound.nx/Game.img/Tombstone` plays,
keys are ignored, and the death window (`UIRevive`) opens. The ghost is the body's `dead` frame, which fades in. Head, hair
and equips have no `dead` frames, so `CharLook` puts the standing ones on the
ghost's neck, as the original client does. The ghost and its name tag circle
the tombstone clockwise, 9 pixels out, once every two seconds; the data has no
motion for this, so `Char::draw` offsets them. The radius and period were
measured from 2008 GMS footage. The window is "PRESS OK TO BE REVIVED."
(`UIWindow.img/Notice/0`); its OK sends that respawn CHANGE_MAP. A player
holding a Wheel of Destiny gets Notice/2 instead, which asks whether to revive
on this map, with YES and NO. Wheels are refused in some field types and maps
(v83 FUN_004a95bf, `field_limit::may_use_wheel`). Before showing Notice/2 the
client sends USE_DEATHITEM (0x35: the wheel's id and the tombstone's position),
which the server ignores. YES sets the wheel byte. The SET_FIELD that follows
carries the new HP, which brings the player back up.

## Keys

New characters get v83's default keymap (`game/src/keymap.rs`), with actions as
HeavenClient numbers them: Ctrl → attack (53), Z → pick up (51), X → sit (52)
and B → Monster Book (59), among others. The client's DEFAULT button in Keyboard
Setting restores the same table.

## Tests

`field_mobs` unit tests cover spawning, control hand-off (including by
distance report), movement, attacks' MP, damage, EXP sharing, and respawn;
`net::combat` covers parsing, the damage cap, Shadow Meso's mesos, the
broadcast, the guards' shares of a hit, both reflects, and the
TAKE_DAMAGE handler's bounds by attack index, its tail, and dodges; `game::mob_damage` covers the
StandardPDD table and the bound. HeavenClient's `mob_damage_tests` and
`mob_attack_tests` cover the damage and miss formulas, both reflects, the dodging skills, and the attacks'
ranges, aim, balls, and rows. `mob_combat` (integration) runs two players in Dangerous
Forest through control, relayed movement, hits, kills, EXP, refused damage,
touch damage, damage by attack index, a level-up seen by the other player, and a respawn, then has
two players report their distance from a firstAttack mob in Crimsonwood Keep
(610030011) and checks control follows the closer one until a hit;
`damage_reductions` (integration) has Magic Guard pay 80% of a touch in MP
and Power Guard send back a boss's share of one;
`reactors` (integration) breaks two of Amherst's boxes, before and during
Pio's quest;
`drops_and_pickup` checks ownership, reach, pickup, replays, meso
persistence, and that the tutorial sentinel's quest drop needs its quest;
`spend_points` levels a beginner and checks AP, the beginner pool, refusals,
and that both survive logging out; `monster_book` fills a card's page from
six snails, checks the effect another player sees and the cover, and that
both survive logging out; `beginner_skills` has three players learn
Three Snails, Nimble Feet, and Recovery and checks their damage caps, MP,
buffs, cooldowns, Recovery's heal, what the others see, and that the buffs
end on time and on death. `poison_mist` casts Poison Mist on a snail and
checks the cloud's packet, that it lifts on time, and that it poisons only
the snails in its box. `summons` has a ranger call up a Silver Hawk and a
Puppet, and checks what a watcher sees: the spawns, a refused and an accepted
Hawk attack, the Puppet breaking, and the Hawk following its owner to the
next map with no entrance. It also has a Dark Knight's Beholder heal them
and buff their weapon defense, refusing both too soon and a stat Hex's
level doesn't give, and an Arch Mage's Elquines freeze a mob for 3 s.
`mystic_door` has a Priest open a door, refused in town, use it to town
and back, keep a stranger out until they join the party, and close it on
logout.
`party_skills` has a Bishop resurrect a dead party member, and a Chief
Bandit heal themselves with Chakra.
`player_diseases` has a Hogul in Korean Folk Town cast Slow from far away
(it misses) and then on the player (it lands for 10 s, then ends).
HeavenClient's `diseases_tests` cover the input, jump, attack and skill
gates and the notices.
