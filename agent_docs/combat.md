# Mobs and combat

## Mobs

Map `life` entries of type `m` are mob spawn points (`x`, `cy` as the ground
height, `fh`, `f`, `mobTime`, `hide`); `game_data::mobs::MobData` reads their
templates from `Mob.nx` (`maxHP`, `exp`, `PADamage`, `bodyAttack`,
`notAttack`, ...). Each `FieldActor` owns a `field_mobs::MobSet`:

- **Spawning** follows the v83 server's CField::TryCreateMob, as
  OrionAlpha (a replica of Nexon's server) has it:
  - A field holds a number of mobs set by its size
    (`field_mobs::mob_capacity`): one per 128,000 square pixels of its
    footholds' bounds (`Footholds::bounds`, the client's: 30 px in from the
    sides, 300 px above the top, 10 px below the bottom; the server adds 10
    px a side). On a map with `info/VRLimit` (only Orbis Tower<B2>) the
    view range pulls those bounds in as the client does
    (`Footholds::bounds_within`). The area is counted as at least 800 wide and 600 high after taking 450
    off the height, times `info/mobRate`, from 1 to 40. With more than half
    that many users on the field it grows, up to twice as many at twice as
    many users.
  - A map with `info/fixedMobCapacity` (Nett's Pyramid's stages and the
    Kerning City event rooms, 365 maps) holds that many, past 40 and
    whoever is on it (`MobSet::with_fixed_capacity`). Its
    `info/createMobInterval` (3 s on all of them) replaces the 7 s below
    (`MobSet::with_interval`). Neither OrionAlpha nor Kinoko reads these
    keys; this follows Cosmic, without its scaling by users.
  - At most every 7 s, while the field holds fewer mobs than that, a pass
    fills spawn points picked at random from those with room, as many as
    are missing. A pass with nothing missing doesn't count. A point without
    a `mobTime` has room when no mob, nor a point filled in the same pass,
    is within 100 px in both x and y. A point with a positive `mobTime`
    holds one mob: it first fills 0.1 to 0.7 times its `mobTime` after the
    field loads, and again 1.3 to 2 times it after its mob goes (Mushmom's
    1800 s: 39 to 60 minutes). A point with a negative `mobTime` (party
    quest and job maps) fills on the first pass only. `hide` points never
    do.
  - The first pass runs as the field loads, with no users, so a field
    starts at its plain capacity. Fields live on once loaded and keep
    spawning with nobody on them.
  - New mobs get new object ids (mobs and drops each have a range in
    `actor::object_ids`). Joining players get SPAWN_MONSTER for every live
    mob.
  - `RUSTMS_FULL_SPAWN=1` (set by the integration harness) gives every
    spawn point a mob of its own instead: no capacity, no room check and no
    first wait, and an emptied point fills again after its `mobTime`, at
    least 7 s.
  - **Area bosses** (Mano, Stumpy, Faust, Timer, ..., 20 spots in
    `net::mobs::AREA_BOSSES`) are in none of the world's map data, so
    each gets a spawn point of its own as its field loads
    (`area_boss_spawns`, on the ground below its spot; event copies of a
    map get none), with `mobTime` `AREA_BOSS_TIME` (1800 s, as
    Mushmom's): from then on it is a point with a `mobTime` like any
    other, so the first boss comes out 3-21 min after the field loads,
    not at server start as in Cosmic. Maps and spots are Cosmic's
    `AreaBoss*` events', the middle of the line where Cosmic picks one at
    random; the Snack Bar keeps only its first spot. Bamboo Warrior and
    the six Door bosses are left out. Each time one comes out on a pass
    after the load, its map gets a type-6 line (`area_boss.appeared`, a
    placeholder naming it). The Snack Bar's point is emptied when the
    stand dies, not the real boss it lets out, so a second stand can come
    out while that boss lives. Defaults: QUESTIONS a117.
- **Recovery:** every 8 s from its spawn a live mob regains its
  `hpRecovery` HP and `mpRecovery` MP, up to its maxima, as OrionAlpha's
  `Mob.update` does, hit or not (`MobSet::recover`, from the field tick).
  Nothing is sent; the next HP percent or boss bar shows it. MP is what its
  attacks and skills spend, so a mob that ran dry casts again.
- **Revive:** a mob with `info/revive` (the Blue Perfume bottle, Yeti and
  Pepe, ...) lets those mobs out where it died once its `die1` animation is
  over (`MobInfo::dying_ms`, 500 ms without one), as Cosmic does. They fade
  in, facing its way, under its last controller, and never respawn. v83's
  SPAWN_MONSTER also has a -3 appearance that ties the new mob to the dying
  one, but by then the server has removed it, so they use -2.
- **Shielded:** Zakum's body (8800000) takes no harm and has no controller
  while any of its arms is out (`MobSet::shielded`); the field drops hits
  on it whole. The last arm's death makes it real (see
  [zakum](./zakum.md)).
- **Buff:** a mob with `info/buff` (Horntail's 2022108, Pink Bean's
  2022449) gives that item's effect, when it dies, to each living player
  on its map not in the Cash Shop (`MobInfo::buff`, `ServerMessage::MobBuff`),
  as Cosmic's `killMonster` does. Cosmic also shows the buff's effect to
  the player and the map, naming the item id as a skill; that is left out.
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
  from the same player within that time send nothing. Hits by Disorder
  (4001002, 14001002) and Hypnotize (5221009) give no aggro
  (`skills::provokes`): v83's client has the mob they hit give up its chase
  instead (0x7a82a2 passes 0 to FUN_007f73d8), as HeavenClient's `Mob::calm`
  does for the local player's hits.
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
  comes within |dx| / 10 + |dy| / 3 <= 40 of it, or once one of its attacks
  could reach them: the attack pick, run without needing a chase or the
  ground under the mob (v83 FUN_0066e32d and FUN_0066a517, report c057;
  `Mob::attack_reaches`). Only its controller's client checks this, every
  update, and the server is not told. HeavenClient leaves out v83's Monster
  Carnival exception (same-team mobs) and, unlike v83, never finds a dead
  player in reach (the distance alone still starts a chase).
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
weapon formula on stats counting equipment, a thief's dagger and Lucky Seven
on LUK, × weapon attack / 100, where weapon attack also counts attack buffs,
an expert skill's `x` (Bow Expert, Marksman Boost, High Mastery), a shot's
star, arrow or bullet `incPAD`, and a weaponless pirate's 10 + 0.7 a level, at
most 31). Criticals raise every cap by their bonus over
100%: the weapon's critical skill (Critical Shot and Critical Throw with
their Cygnus twins, and Critical Punch), Sharp Eyes's low byte as sent, and
Stun Mastery. An attack naming a skill needs that skill learned at some level (a GM
skill, 900 or 910, also needs a GM account: Super Dragon Roar is ten times a
blow on up to 15 mobs); its `Skill.nx`
level (`game_data::skills::SkillLevel`) gives the mobs allowed, the hits (a
shot's `bulletCount`, otherwise `attackCount`), the MP spent, and the cap: a
`fixdamage` skill (Three Snails: 10/25/40) hits for at most that, a `mad`
spell for twice v83's magic maximum (`max_magic_damage`: ((M² / 1000 + M) / 30
+ INT / 200) × `mad`, where M is INT plus equipment magic attack), a `damage`
skill for that percentage of the basic cap (Barrage's fifth line twice that
and its sixth four times; Elemental Boost adds its `damage` to Flamethrower's
and Ice Splitter's, as v83 does at 0x79004c), Shadow Meso for v83's highest
roll (ten times one and a half its `moneyCon`, raised by its `x` percent on a
critical), Snipe's shot for 199,999, and a skill with none is not an attack.
A spell must name a skill.
Too little HP or MP refuses the attack, as do too few mesos for the skill's
`moneyCon` (Shadow Meso's, raised by Shadow Partner's `y` percent while that
buff is on, as the v83 client checks). An attack skill with a `cooltime`
(Heaven's Hammer, Snipe, Aerial Strike) is refused while it cools down;
otherwise it starts the cooldown and the attacker gets COOLDOWN.
A skill's HP cost (`skills::hp_cost`, as v83's `FUN_00764256`) is its
`hpCon`, which must stay under the player's HP; Slash Blast and the Boosters
have one. Dragon Roar instead takes its `x` percent of max HP and needs at
least half of it, and then stuns its user for its `y` seconds
(`skills::self_stun`): a Stun disease under Dragon Roar's own id, so the
client shows the skill's icon and won't let it be cancelled. The mobs it
hits are stunned for the same `y` seconds (it has no `time`). Sacrifice takes its `x` percent of its first blow, never
the last HP, which v83 leaves wholly to the server (Cosmic's rule).
Final Attack (`skills::FINAL_ATTACKS`) is a swing or shot the client sends
on its own after a skill attack whose skill lists it under `finalAttack`
(Power Strike, Slash Blast, Arrow Blow, Double Shot, …). It is an ordinary
attack packet with the Final Attack's id and level, costs no MP (the
Cygnus ones have an `mpCon` for their buff), and hits one mob, except that
after Slash Blast (bit 0 of the attack's flags) it reaches as many as the
player's Slash Blast does (`skills::slash_blast`).
Assassinate (4221001) goes only from Dark Sight, and in Dark Sight nothing
else goes. The client sends CANCEL_BUFF for Dark Sight, then a 3-line
strike; an attack delay later, if that found a mob, it strikes that mob
again with one line, bit 0x20 of the attack's flags set, and dashes 100 px
forward (a movement element 8 holds where it started). Every line of both
strikes is multiplied by one more for each 3 seconds since Dark Sight was
given, up to the skill's `time`, and only the second strike crits, with the
skill's `prop` and `criticalDamage`. The server takes MP once, for the
first strike, allows it within a second of cancelling Dark Sight, caps a
line at the most the time can multiply it by, and takes a second strike
only on the first's mob within 2.5 s (`net::combat::second_strike`).
HeavenClient sends a skill as the attack v83's `CUserLocal::DoActiveSkill`
makes it, whatever the weapon: a swing, a shot, or a spell
(`SkillData::flags_of`). Any other skill goes out as SPECIAL_MOVE. Skills
that charge first (Big Bang, Hurricane, Corkscrew Blow) aren't supported. v83
casts Three Snails as a spell; RustMS and HeavenClient treat it as a swing.
The field shows
the attack under its own opcode (with the skill id and level, so others see
the skill's effect, and for a shot the star, arrow, or bullet it fired) to the
others, applies the damage, sends the attacker SHOW_MONSTER_HP, and on death
KILL_MONSTER to everyone; the attacker gets the HP, MP and mesos left as
STAT_CHANGED.

A mob with an `info/damagedBySelectedSkill` list (Kyrin's test
Octopirates, 9001005 and 9001006) is hurt only by those skills. v83 rolls 0
(MISS) for any other attack (`FUN_0068146c`), so the field applies none of
its damage (`MobInfo::hurt_by`), and HeavenClient rolls MISS for it as well.

A shot fires from the USE slot the client names, as in v83 (`shot_ammo`). The
item there must fit the weapon (stars for claws, arrows for bows and the
Arrow claw 1472063, crossbow arrows, bullets for guns), need no more than
the player's level (`info/reqLevel`, which v83's picker also checks,
FUN_00a10579; so does Shadow Stars' star), and cover the cost:
the skill's `bulletConsume` if it has one (Avenger's 3), otherwise its
`bulletCount` (Lucky Seven's 2), or 1 for a basic shot. Anything else refuses
the attack. The cost is taken from that slot and the attacker gets
INVENTORY_OPERATION. Skills v83 fires without ammo (Shadow Meso, Taunt,
Energy Orb, and some Cygnus and Aran skills; `AMMO_FREE`) use none, and
neither does a bow or crossbow under Soul Arrow. The server tracks neither
Shadow Partner, which doubles the cost, nor the cash ammo slot. HeavenClient,
like v83, fires from the first fitting stack that covers the cost
(`Inventory::find_bulletslot`).

Shadow Stars (4121006) names a star in USE_SKILL and costs its
`bulletConsume` (200) of it, summed over that star's stacks, from a claw;
otherwise it is refused. Its buff (bit 40, the client's SpiritJavelin) carries
the star as `id - 2069999`, an i16 in GIVE_BUFF and an i32 for the others.
While it lasts, a claw throws that star from a slot holding one and spends
none; the shot sets display bit 0x40 and, unless the skill is ammo-free,
carries the star as an int after the cash slot byte (v83 0x95514b). The
others are shown the star in the attack's projectile. If that star is gone
from USE (dropped, sold or moved away mid-buff), v83 throws no other star: a
shot that takes ammo does nothing at all, with no throw, packet or message,
unless Soul Arrow is on; a whack at a mob in front still swings (0x949b5d,
0x9537d5). HeavenClient does the same (`Player::shadow_star_lost`,
`Combat::apply_move`), and the server refuses such a shot anyway.

HeavenClient refuses a skill before sending it, with v83's lines in the skill
chat colour (`Player::can_use`, `Skill::can_use`, `Combat::can_use_move`). The
checks run in v83's order. A skill cooling down comes first. Then come the
costs: HP (Dragon Roar's is its `x` percent of max HP, and needs half of
it), MP, mesos (`moneyCon`; Shadow Meso's grows by Shadow Partner's `y`
percent while that buff is on), and the item (`itemCon`, named in the line).
The four cost lines share one two-second throttle, so a second refusal within
two seconds prints nothing. An attack with no weapon says so next, and one
with a weapon that doesn't count toward the stats (see "Equipment
requirements" in `inventory.md`), or a cash weapon over none that does, says
"You cannot use this weapon with the current stats." on the same two-second
timer (0x95b85a). Then the
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

On a map whose `fieldLimit` has bit 0x2, the attacks that carry the player
along or push them back do nothing, without a word: the Rushes, Assaulter,
Assassinate, Ninja Storm, Backspin Blow, both Corkscrew Blows, Snatch, Final
Charge, Final Toss, Rolling Spin, Dragon's Breath, and Recoil Shot
(`field_limit::may_attack`, 0x950921 and 0x9537d5).

HeavenClient rolls Shadow Meso's hits as v83 does (`shot_path::meso_damage`,
FUN_0078df87): ten times a whole number of mesos between half and one and a
half its `moneyCon`, with its own `prop` and `x` for a critical in place of
the user's, then a percent less for each level the mob is above the user. It
misses as a blow does but ignores the mob's defense. Snipe's shot never
misses and meets neither defense nor guards: 199,999 less a roll from 0 to
10,000 (`shot_path::snipe_damage`, 0x78e4d6), always shown as a critical
(FUN_00668b83). Another player's attack shows a hit as a critical when its
damage has the top bit set (0x980620); RustMS sets it only for Snipe.

HeavenClient rolls every other blow as v83's PDamage does (`Mob::next_blow`,
`Gameplay/Combat/PhysicalDamage.h`, FUN_0078df87). The weapon, not the job,
picks the stats and the multiplier, and a swing and a stab differ by the
action's name; bow, crossbow, claw and gun whacks and a prone stab have
formulas of their own. The roll starts at the mastery's share of the main
stat (`(5m + 10)%`, 10% unlearned), and the mastery skill counts only for the
reach it suits. A hit lands when 70% to 130% of the accuracy (ten percent
lower for each level the mob is above) reaches the mob's avoid. Then come the
level penalty, the skill's element, a White Knight charge, a defense roll of
half to 0.6 of the mob's, the skill's percent, the Combo orbs, a critical
(the weapon's crit skill, Sharp Eyes, or Stun Mastery on a stunned mob),
Power Guard Up, Shadow Partner's share of the first set, Hard Skin, Berserk
and Barrage, at most 199,999. A shot adds the attack of the ammo it throws,
and the stat window shows the same range v83 does (FUN_008c2870).

The totals count what v83's SetFrom adds (`Character/SecondaryStat.h`,
FUN_0077f4c9): accuracy and avoid from DEX and LUK by the job, the `x`
accuracy and `y` avoid of Blessing of Amazon, Nimble Body, Bullet Time and
Quick Motion, a Thrust's speed, Shield Mastery's `x` percent of the shield's
own defence, and a weaponless pirate's attack. A mob's blow and the stat
window use the same avoid.

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
none does it shoot. Most skills that shoot do the same, and so does each tick
of a held Hurricane or Rapid Fire, which also lets the skill go; a few shots,
such as Arrow Rain or Arrow Eruption, never whack (`point_blank::always_shoots`).
A Ranger's or Sniper's whack, with Mortal Blow learned, first rolls its
`prop`: on success it shoots Mortal Blow at the mob instead, flagged 0x04;
on failure the whack goes out flagged 0x10 (`point_blank::mortal_blow`).

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

A mob whose `info` gives both `hpTagColor` and `hpTagBgcolor` (`MobInfo::hp_tags`)
has a boss HP bar along the top of the screen instead: FIELD_EFFECT mode 5,
with the mob's template id, HP, max HP and the two colours. The field sends
it to everyone on the field each time such a mob's HP changes, and once more
with HP 0, which takes the bar down, when the mob dies or leaves. Nothing is
sent when it spawns or when a player enters, as in Kinoko. A mob that also
sets `HPgaugeHide` gets no mode 5 while it lives (`MobInfo::boss_bar`): the
client shows the bar itself from SHOW_MONSTER_HP, to the attacker alone, as
v83 does. Bosses without `HPgaugeHide` show no small bar at all. In
HeavenClient the bar is `UIMobGage`, laid out from `UIWindow.img/MobGage` as
v83 does (`MobGageLayout.h`): it starts at the minimap's right edge, the
fill truncates, and hovering the icon shows the mob's name. As in v83, a
`HPgaugeHide` mob's bar comes down 20 s after the last hit while it lives.
v83 delays those bars 500 ms, which HeavenClient doesn't.

Some attacks take something back from the mobs they hit (`skills::takes`,
handled in the field's `handle_attack`):

- **Drain**, **Energy Drain** and **Vampire** give back their `x` percent of
  each hit's damage as HP, no more than the mob's max HP per mob, and no more
  than half the player's max HP per attack (`absorbed`).
- **MP Eater** (2100000, 2200000, 2300000, by the mage's branch) rolls its
  `prop` for each non-boss mob a spell hits and survives, and takes `x`
  percent of the mob's max MP, no more than it has, as MP. The player sees
  the skill on themselves (SHOW_ITEM_GAIN_INCHAT 1) and the map sees it on
  them (SHOW_FOREIGN_EFFECT 1), once per attack.
- **Mortal Blow** (3110001, 3210001) rolls its `y` percent per attack. On a
  hit, a non-boss mob with no more than `x` percent of its max HP left dies.
  The v83 client shoots it as a ranged attack when a whack's roll succeeds
  (FUN_00950921), as described above.
- **Steal** (4201004) rolls its `prop` for each mob it hits, misses too, and
  takes one item from a mob once (`LiveMob.stolen`). The client actor picks
  it from the mob's drops, quest items aside, weighted by their chances
  (`loot::steal`), and drops it as the attacker's own.
- **Pickpocket** (4211003), while its buff is on, rolls its `prop` for each
  line that hurts in a melee attack, Savage Blow, Assaulter, Boomerang Step,
  Band of Thieves and the like (`skills::pickpocket`), and drops
  `line * x / 20000` mesos, at least 1 and no more than `x`, near the mob,
  anyone's at once. They fall 100 ms apart, the DROP packet's delay. A
  killing blow still pays.

The client actor applies what came back as one STAT_CHANGED
(`ServerMessage::Absorbed`).

A dead mob's EXP is shared by damage dealt (the top damager gets any
rounding remainder) and each share goes to that player's client actor
(`ServerMessage::MobKilled`), which adds it with `Player::gain_exp`. A level
gained sends the new level, AP, SP, max HP/MP and shows SHOW_FOREIGN_EFFECT
0 to the others. As in v83, current HP and MP are not refilled. Each level
grants 5 AP, and 3 SP once past beginner.

Max HP/MP growth (`game::character::growth`) starts from the v83 client's
own per-job floors, the slopes of its minimum max HP/MP tables: per level
beginner 12/10, warrior 24/4, magician 10/22, bowman and thief 20/14,
pirate 22/18; per AP the AP Reset dialog's 8/6, 20/2, 6/18, 16/10 and
18/14. A roll (`net::skills::growth_roll`) adds 0-4 HP and 0-2 MP on top.
The client keys these by `job % 1000`, so Cygnus Knights and Aran use the
Explorer rows; GMs use the beginners'. Max MP also gains a tenth of total
INT per level and 3/40 of it per AP (the share the client's AP Reset takes
back), and a warrior's Improving MaxHP Increase (1000001), a Brawler's
Improve MaxHP (5100000) and a magician's Improving MaxMP Increase (2000001),
and the Knights' Max HP Enhancement (11000000, Dawn Warrior), Improve Max
HP (15100000, Thunder Breaker) and Increasing Max MP (12000000, Blaze
Wizard), add their `x` to each level's growth and their `y` to an AP's
(`net::skills::growth_bonus`). Level ups stop at 30,000 like AP.

A job advance adds max HP/MP so the player stays above the client's least
max HP/MP for the new job (`game::ap_reset::min_hp`, `min_mp`): the floor's
rise plus 25. Pages and Spearmen get 125 MP where Fighters get 325 HP, since
the client raises their least MP, not HP (`second_job_bonus`).

Reaching level 200 sends a notice to the whole world (`level.200`, Cosmic),
but not for a GM. It is sent from `commit_player`, so EXP from kills, quests
and `!exp` all trigger it; `!level` sets the level directly and doesn't.

### The same spot

The client limits how often a player may act without moving
(`HeavenClient Gameplay/Combat/SameSpot.h`, v83's FUN_009536d0, report
b377). A step of 6 px across or 150 px up or down makes a new spot.

- From one spot, the player gets 100 attacks after the first. This counts
  swings, shots, spells, the body attacks, Meso Explosion and a basic
  attack in a special field or at a reactor. Hurricane's and Rapid Fire's
  shots don't count.
- A skill that sends the player's position (`skill_use::sends_position`)
  gets 20 uses after the first.

Past either limit nothing is sent and no message appears:

- a swing doesn't happen at all;
- a shot, a spell or Meso Explosion plays its action but hits nothing;
- a skill plays its action without its effect.

A new map starts both counts over. The server has no such limit.

### Rates and rate cards

`net::rates` scales kills for each player. Kill EXP is multiplied by the
world's `RUSTMS_EXP_RATE`; quest and script EXP is not. Drop chances are
multiplied by `RUSTMS_DROP_RATE` and meso amounts by `RUSTMS_MESO_RATE`;
reactor drops are not. Each rate is 1 when unset.

Play can also earn NX Credit, which v83 never did (`net::rates::nx`). Both
rates are 0, off, when unset:
- `RUSTMS_NX_PER_KILL` is earned for each kill the player loots (one per
  kill, even in a party).
- `RUSTMS_NX_PER_HOUR` is earned for each hour connected to a channel,
  with a pink chat line outside the Cash Shop. A channel change starts the
  hour again.

The client actor keeps the NX it earns and puts it on the account
(`db::cash_shop::grant`) at each autosave, on entering the Cash Shop, and on
leaving, so kills cost no database write of their own. The harness world
gives 1 NX per kill (`tests/earned_nx.rs`).

A vote site can also give NX Credit through the world's vote hook
(`runtime::vote_hook`), which is off unless both `RUSTMS_VOTE_HOOK_ADDR` (the
address it listens on, such as `0.0.0.0:8090`) and `RUSTMS_VOTE_HOOK_SECRET`
are set. The site calls `GET` or `POST /vote` with `account` (the login name,
matched exactly) and `secret` in the query or a form body, or the secret in
an `X-Vote-Secret` header. A vote gives `RUSTMS_VOTE_NX` (1000 when unset)
once per `RUSTMS_VOTE_COOLDOWN_HOURS` (12) per account. Each vote is a row in
`votes` (`db::vote::record`), which keeps the cooldown and serves as a log. The
answer is plain text: 200 `counted`, 429 within the cooldown, 404 `no such
account` (or `not found` for another path), 403 for a wrong secret, 405 for
another method, 400 for a request it can't read, and 500 if the database
fails. A secret in the query or form must be percent-encoded. The hook answers
64 calls at once and closes any more unanswered. The
Cash Shop reads balances on entry, so a vote made while the player is in it
shows once they check again. The harness world's hook is on the port after its
channels' (`tests/vote_hook.rs`).

A 2x EXP card (521xxxx) or drop card (536xxxx) in the Cash inventory
multiplies the same numbers again while it works. A card works while it is
unexpired and the clock is inside one of its `info/time` windows: a
`"DAY:HH-HH"` entry, with the end hour left out.
- The clock is UTC, moved by `RUSTMS_RATE_CARD_UTC_OFFSET` hours. The cards
  were sold for Pacific time.
- `HOL` (holiday) entries never match.
- A drop card also doubles mesos, as 5360000's description says.
- Cards of one kind don't stack; the best one counts.
- A few cards last hours instead of days from purchase, as their tooltips
  and descriptions say: 5211060 2 h, 5211048 and 5360042 4 h, 5211049
  90 min.

Hunter's Luck (2450000) multiplies kill EXP by its `spec/expBuff` percent
(200) for its `spec/time` (30 minutes), on top of the rates and cards. It is
an item buff on TemporaryStat bit 74 (`BuffStat::ExpBuff`), which only the
player sees: the client shows the item's icon and reads nothing else.

v83's client never reads a card's rate or hours, and has no packet or line
for one. The extra EXP is folded into the base number of the "You have
gained experience" message; the "Bonus Event EXP" field would mislabel it.

### Meso Explosion

Meso Explosion (4211006) sets off mesos lying in front of a Chief Bandit,
and each hurts the mobs around it by what it is worth (v83 FUN_0096b3fb;
`v83ref` report 194). The client picks everything and sends it in its
CLOSE_RANGE_ATTACK, laid out differently from other attacks (v83 0x96bafe):
each target has 12 bytes, then a byte counting its lines, then the lines
and the 4-byte checksum. After the targets come where the player stands,
a byte counting the drops, each drop's object id and a byte of the targets
it hit (bit j for the j-th), and an i16 of when the hits land, in ms.

HeavenClient (`Gameplay/Combat/MesoExplosion.h`, `Combat::set_off_mesos`)
refuses it on a rope or ladder or in the air. It looks in the skill's box
(`lt`/`rb`, mirrored when facing right, without its right and bottom
edges) for mesos that have landed and that the player may take by their
ownership type. Unlike pickup, it ignores how long they have lain. It takes
the first `attackCount` (at most 20) in the order they entered the map,
then sorts them cheapest first. Each drop hits the live mobs whose body
meets a box 50 px around it. That is at most `mobCount` mobs, in the order
they entered the map, and at most 15 drops on each mob. Only the first 15
drops hurt anything. One line is 50·`x` × a roll from 0.5 up to 1 × a ratio
of the mesos: (0.82m + 28) / 5300 up to 1,000 mesos, then m / (m + 5250). A
Disabled mob takes 0, and a mob under PImmune takes 1. The packet carries
every line. A mob shows them added up into at most five numbers (the i-th
line to number i % 5), all at the attack's hit frame. Those numbers show the
mob's `fixedDamage` for a hit, or 0 if its `damagedBy` leaves the skill out.
With no mesos the skill still acts and costs MP. The skill's `hit`
animations are the drops going off, not hits on the mobs.

The server (`net::meso_explosion`, `blast` in `net::combat`) refuses more
drops than `attackCount` and a drop named twice, and checks that each
target's lines match the drops whose bits name it. Each line is capped at
50·`x`. The field (`set_off`) takes each drop the player may take that lies
within the skill's box (100 px of slack), and removes it at once. It sends
REMOVE_ITEM_FROM_MAP mode 4 (`u8 4, i32 oid, i16 delay`). The delay is the
client's, plus 100 ms for each of the next four drops, then again from the
start (Cosmic's spread), and never more than 1 s. A line counts only if
its drop went off and the line is no more than that drop could do
(`line_cap`). It counts as the mob's `fixedDamage` if it has one. On mode 4,
HeavenClient's drop stays where it is, and can't be picked up or chosen
again. When the delay ends, it disappears and plays one of the skill's
`hit` animations at random on the effect layer. The skill's `Hit` sound
plays no more than once every 90 ms (FUN_00504bff). Other players see each
line as its own number, all at once, with the same `fixedDamage` and
`damagedBy` rules (FUN_009803ab).

Harness `meso_explosion` throws mesos and sets them off. It checks the 30
MP, the mode-4 delays of 300 and 400 ms, and that a line of 1000 from 100
mesos kills while 400 from 10 does nothing. It also checks that a drop
already set off doesn't go off again. Finally, a drop named twice, a line
from no drop, a bit naming no target and 21 drops are refused and leave
the drop in place. Its builder is `hunting::meso_explosion`, and
`Field::exploded` records mode 4.

## Buff skills

USE_SKILL (`0x5B`, `handle::world::use_skill`) casts a skill that doesn't
attack. `net::skills::buffs` maps each buff skill to the stats it gives (the
level's attack, defense, accuracy, avoidability, speed and jump, plus the
skill's own stat); a skill missing from it is ignored. The level the client names must be the learned one, the skill must
be off cooldown, and the player alive with enough HP and MP. The caster gets
the MP left (and the HP, for a Booster's `hpCon`), GIVE_BUFF (the temporary-stat masks match HeavenMS: Speed and Recovery
are both second-mask bits), and COOLDOWN when the level has a `cooltime`;
the map sees SHOW_FOREIGN_EFFECT 1 with the skill.

`game::Buffs` on the `Player` holds the active buffs and cooldowns; neither is
saved. A new buff replaces one on the same stat. The client actor sleeps until
the next deadline (`actor::client::buff`): Recovery heals its `x` every 5 s
while the player is hurt (HP stat update). Dragon Blood drinks its `x` every
4 s, showing its `special` animation to everyone (user effect 5, an int
skill id); when the player has no more than that left it ends instead, as its
description says ("until 4 seconds before the remaining HP exhausts"). An
expired buff sends CANCEL_BUFF, since the client never ends buffs itself. Right-clicking a buff
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
- **MP Recovery** takes its `x` percent of the caster's max HP (without
  equips) and gives `y` percent of that as MP (`skills::recovered_mp`). It
  is refused unless the caster has more HP than it takes. v83 checks
  nothing for it, so the rule is OdinMS-family servers'.
- **Combo Attack** (`net::combo`) starts at value 1, which is one more than
  the orbs. After an accepted swing that reaches a mob (not Shout), the
  attack handler adds an orb up to Combo Attack's `x`, or Advanced Combo's
  once learned, and with Advanced Combo a second one on its `prop` chance,
  as Cosmic does. Panic and Coma reset the value to 1, and `validate`
  refuses them without an orb, as the client does. Each change goes to
  the player as GIVE_BUFF for the time left and to the map as
  GIVE_FOREIGN_BUFF. The orbs multiply a swing's or a shot's damage after
  its skill's, by the client's table (FUN_007653fa, `combo::damage_percent`).
  The damage cap and HeavenClient's `Player::combo_percent` both use it.
  A Dawn Warrior's skills follow the same rules, with Combo Attack's last
  level at 20. Enrage, a buff of its `pad`, wants a value of 10 (nine
  orbs: the client compares the value, 0x764379) and resets it to 1 the
  same way (`combo::enrage_refused`). Harness `enrage` charges a Hero's
  Combo on snails and checks the refusals below 10, the 40 MP, the 240 s
  buff, the value back to 1 and the 360 s cooldown.
  HeavenClient draws the orbs as v83 does (`combo::Orbs`, FUN_0093cdbc):
  the newest five on a turning ring 40 pixels over the feet, the newest
  popping in, all fading out when spent, and a finisher's
  `finish/<orbs>` burst at the attacker's feet.
- **Energy Charge** (`net::energy`, 5110001 and the Thunder Breaker's
  15100004) fills a gauge with each hit, up to 10000. A full gauge frees
  the Energy skills (`energy::refused`, chat 0xdd1 on the client) and
  lends the player the skill's own attack, accuracy and avoid where they
  beat the buffed stat. With a full gauge, a mob that touches the player
  takes a body attack first: the client sends TOUCH_MONSTER_ATTACK (0x2F),
  a close attack of Energy Charge on that one mob with one line of the
  skill's damage, at most once a second, off ropes and ladders and not
  on a tamed monster (FUN_0095f135). Neither side plays an action or an
  effect of the attacker's, only the skill's `hit` on the mob. The server
  takes it through the attack handler (`AttackKind::Body`) and sends the
  map ENERGY_ATTACK (0xBD); Energy Charge comes no other way. It drops a
  body attack, Energy Charge's or Body Pressure's, that comes within 800 ms
  of the last one it let through (`body_attack_due`). Harness
  `energy_charge` fills a Marauder's gauge 192 a swing to 10000 and checks
  that Energy Blast waits for it.
- **Weapon charges** (`net::charge`, 1211003 to 1211008 and the Paladin's
  1221003 and 1221004) raise every weapon hit by the charge's `damage`
  and its element; the client works the damage out and puts no status on
  the mob, so the server only widens its damage cap. A sword's charges
  need a sword and a mace's a mace (FUN_00966f7a, 0x1116 on the client).
  Charged Blow (1211002) is refused without a charge (`charge::refused`)
  and spends it when it reaches a mob. A Paladin's Advanced Charge
  (1220010) keeps the charge by its `x` percent, and its `damage` replaces
  Charged Blow's own on both sides. The Dawn Warrior's Soul Charge
  (11111007, holy), the Thunder Breaker's Lightning Charge (15101006) and
  the Aran's Snow Charge (21111005, ice) are charges too; the client reads
  each one's element from its skill (FUN_007908e7). Snow Charge also slows
  every mob it hits by its `x` for `y` seconds (`skills::on_hit`).
- **Cygnus and Aran buffs** share the Explorer arms in `skills::buffs`:
  Iron Body, Rage, Magic Armor, Meditation, Focus and Haste give their
  leveled stats; the boosters, Magic Guard, Dark Sight, Combo Attack and
  the Aran's Maple Warrior give the same stat as their Explorer twins; and
  Freeze Standing (21121003) is the Aran's Power Stance, which the client
  picks for job 2112 (0x958aa9).
- **The Aran's combo** (`net::aran_combo`, report b360) counts the hits a
  polearm lands while Combo Ability (21000000, or 20000017 for job 2000)
  is learned. The client counts nothing: it sends ARAN_COMBO_COUNTER
  (0xA3, no payload) for each landed line, and the server answers with
  SHOW_COMBO (0xE1, the count). It also gives the count as stat 68
  (`BuffStat::AranCombo`) under Combo Ability at the first hit and every
  tenth, where it adds to attack and criticals: Combo Ability's `y` attack
  for every ten up to `x` tens, and with Combo Critical (21110000) a
  critical's damage raised by its `damage` for every ten up to `x` tens,
  plus 100 (200 if nothing else made the hit a critical). The count is
  capped at 30000. It ends 3 seconds after the last hit on both sides: the
  client sends CANCEL_BUFF for Combo Ability, and the server's own buff
  ends with CANCEL_BUFF. Combo Smash and Combo Drain need 30, Combo Penril
  100, and Combo Tempest and Combo Barrier 200 (`aran_combo::refused`,
  `AttackRejection::ComboShort`, `Refusal::ComboShort`). The client says
  nothing about it, since its key picks a skill the combo allows. Any of
  them spends the whole count, with a CANCEL_BUFF of stat 68. Combo Drain
  (21100005, stat 69) heals its `x` percent of each weapon hit (`skills::takes`),
  and Combo Barrier (21120007, stat 70, reaching the party) keeps its `x`
  per mille of a hit after Achilles (`combat::taken`). Body Pressure
  (21101003, stat 71) lets the client hit one touching mob a second with
  TOUCH_MONSTER_ATTACK, which is refused without the buff. Smart Knockback
  (21111001, stat 72) is the client's alone. None of the five is shown to
  others. Harness `aran_combo` builds a combo to 30, checks that Combo
  Drain waits for it and spends it, and waits for the 3 s end.
  HeavenClient (`Gameplay/Combat/AranCombo.h`) sends 0xA3 from
  `Combat::apply_damage_effect` for each of the player's own landed lines
  with damage, Combo Ability and a polearm, keeps the SHOW_COMBO count on
  the `Player`, and ends it 3 seconds on with CANCEL_BUFF; a combo skill
  used ends it at once. `UIAranCombo` draws the count at the top right
  (report b361): digits from `Effect/BasicEff.img/Combo/<tier>`, the word
  under them, and the ComboCommand at 30, 100 and 200. A key bound to a
  combo skill uses it only once the combo allows it. Stat 68 adds Combo Ability's attack in `Player::recalc_stats`,
  so the stat window shows it, and Combo Critical's rate (its `y` percent
  for every ten up to `x` tens, plus 10) and damage come after Stun
  Mastery's in the mob's critical roll (`physical_damage::aran_critical`).
  `Combat::body_attack` sends Body Pressure's TOUCH_MONSTER_ATTACK on a
  touching mob, a second apart from Energy Charge's own, and shows others'
  as Energy Charge's are. A local hit with Smart Knockback makes a mob that
  isn't a boss flinch at its percent of the mob's `pushed`
  (`aran_combo::pushed`); the Aran's Achilles (21120004) and Combo Barrier
  count in the damage the client predicts over its head. The Lie Detector
  freezes the combo (FUN_00a265c2, which b360 took for the mini room's; see
  `lie-detector.md`).
  Still open: the GIVE_BUFF layout of stats 68 to 72 is inferred. The
  server takes 0xA3 on trust, Body Pressure's `prop` status isn't
  modelled, and Combo Drain
  shares Drain's cap of half max HP.
- **The Aran's key sequences** (HeavenClient `aran_combo::Commands`,
  reports b364 to b366) turn arrows and the attack key, pressed in order
  and in time, into commands. A command waits up to a second (two for
  Final Charge, Blow and Toss) and is tried each frame while the player is
  alive and not attacking. While one waits, the keys that act are dropped.
  - Attack, attack is Double Swing, and a third attack is Triple Swing.
    Left or Right then attack is Final Charge, Down then attack is Final
    Blow, and Up then attack is Final Toss. Each goes only on the swing
    before it, or after it while it waits: Double Swing on a plain polearm
    swing, Triple Swing on a Double Swing, and the three finishers on a
    Triple Swing. On their own, Final Blow and Toss take the attack key,
    which then does nothing.
  - Up, Up, attack is Rolling Spin. Down then Left or Right then attack,
    and Down, Down, attack, use the best combo skill the count allows
    (b362). Down, Up, attack is Rage of Pharaoh for any job, with a use
    left in the Pyramid (`pyramid.md`); the rest are the Aran's alone.
  - Left, Left or Right, Right is Combat Step (21001001). The client
    slides the player along the ground at the level's `x`
    (`Player::combat_step`), only on a foothold, where the map allows
    movement skills (or for a GM), not on a jump pad (a touch portal of
    type 12 or 13, `Stage::use_combat_step`), and a second apart. A jump down holds
    it off 200 ms, and no jump is allowed during the step. The path gets
    element 20, from which others see the step's effect
    (`OtherChar::send_movement`). The server announces no skill effect for
    it (`skills::COMBAT_STEP`).
  - An Aran's plain attack with a polearm plays one of `swingT2PoleArm`,
    `swingP1PoleArm` and `swingP2PoleArm` at random
    (`aran_combo::polearm_swing`). Double Swing waits on those. Others see
    the plain swing of the same name, since HeavenClient's wire stance is
    its own list.
  - Its plain polearm attack hits two more mobs than a twentieth of its
    level (`aran_combo::plain_mob_count` on both sides); anyone else's
    hits one.
  - Full Swing (21110002) and Over Swing (21120002), Over Swing first,
    make Double and Triple Swing cast their hidden skills (21110007/8,
    21120009/10) at their own level (`aran_combo::swing_cast` in
    `Combat::use_move`). The client's skill book and the server's
    `skills::learned` give a hidden swing its owner's level
    (`aran_combo::swing_owner`). The Legend's copies have no such skills.
  - That plain swing holds off the next attack with no skill for its
    action's length, every frame's delay whatever its sign and whatever
    the attack speed (540 ms for each of the three), and 400 ms more
    (`aran_combo::polearm_swing_wait`, `Combat::apply_move`).
- **Berserk** (`net::berserk`, 1320006) is a Dark Knight's passive. The
  client keeps the state: Berserk is on while HP is at most `x` percent
  of max HP, rechecked when HP changes and anew on entering the game. Each
  change sends an empty CALC_DAMAGE_STAT (0x6C). The server works the state
  out again, keeps it on the player, and shows the looping aura (the
  skill's `effect`) to the player as SHOW_ITEM_GAIN_INCHAT 1 and to the map
  as SHOW_FOREIGN_EFFECT, with a last byte for on or off. Players who see
  the knight arrive get it from SPAWN_PLAYER's berserk byte. While it is on,
  every weapon hit is raised by its `damage` percent, and the server's
  damage cap counts it when the server last saw it on or the HP is low
  enough (`berserk::damage_percent`). HeavenClient's Beholder heals by
  Berserk's `x` only while it is on.
- **Homing Beacon** (`net::beacon`, 5211006) and a Corsair's **Bullseye**
  (5220011) are plain shots: the client fires Bullseye in the beacon's
  place once it is learned. After the shot, the server marks the first mob
  hit with a self-only GIVE_BUFF of the two-state GuidedBullet (bit 87,
  17 bytes: on, skill, sign, time, mob). A new mark comes after a
  CANCEL_BUFF of the old one, since the client doesn't take an old mark
  off on its own, and leaving the map takes it off (`drop_beacon`). It has
  no time and no icon. HeavenClient loops the skill's `mob` animation on
  the marked mob, a mob arriving later too. Other shots and single-target
  spells hit the mark first if it is in their path. With Bullseye learned,
  every hit on the mark, a summon's too, is raised by its `x` percent, and
  the server's damage cap counts it (`beacon::damage_percent`).

Teleport and Flash Jump move the player on the client alone; the server
only spends the MP, with no buff or cooldown, and refuses a level not
learned or too little MP (harness `flash_jump`). HeavenClient's Flash Jump (4111006 and the Night
Walker's 14101004, `flash_jump`) follows v83 (FUN_0096be38). It works only
in the air, off any rope or ladder, and, out of water, not once the player
is falling. It works once until the player lands or grabs a rope, and maps
that bar movement skills refuse it ("You cannot use that skill on this
map."). It throws the player 350 px/s forward and 250 up, plus 40 and 20
for every four levels, records path element 6 before the push, and leaves
`BasicEff.img/Flying` (`Flying1` for a Night Walker) where the jump began.
Another player's client shows that effect, picked by the jumper's job, when
their path arrives with element 6 (FUN_0097fdf8).

A Gunslinger's Recoil Shot (5201006) is a plain gun shot to the server.
HeavenClient follows v83 (FUN_009537d5): as the shot goes, it records path
element 13 and throws the player 250 px/s back, away from where they
face, and 250 up, plus 40 and 20 for every four levels (`recoil_shot`). No
Flash Jump follows it before the player lands, and it may be shot again
only after 2 s. Another player's client shows its `effect` when their path
arrives with element 13, rather than with the attack.

These, Mystic Door, Teleport and Flash Jump have neither buff stats nor a
mob debuff, so `skills::acts_alone` lets them past the check that refuses
such skills as unsupported.

### Mounts

Monster Rider (1004, and 10001004, 20001004, 20011004 for the other
beginners) rides the 190xxxx mount worn at -18 with the 191xxxx saddle at
-19 (`game::mount`, report 201). USE_SKILL for it (`use_skill::ride`)
checks the player is alive and not Seduced, knows the skill at that level,
the map's `fieldLimit` lacks 0x200 (`game_data::FIELD_LIMIT_MOUNTS`), both
are worn and the mount has a `tamingMob`, and the MP. The ride is a
`BuffStat::Riding` buff (bit 85) with the rider skill's id and no end. It
is a two-state stat, so the generic buff packets never carry it.
GIVE_BUFF and GIVE_FOREIGN_BUFF for it carry its own block: the mount, the
skill, a byte and an int (`buff::build_give_ride`). SPAWN_PLAYER carries
the same block in its riding slot. The client locks every skill
until it hears bit 85 back, so a refusal and CANCEL_BUFF for the skill
both answer with CANCEL_BUFF for riding (`mount::dismount`). The map sees
CANCEL_FOREIGN_BUFF when the player gets off. HeavenClient plays the
mount's effect (the rider's Monster Rider, or the Battleship's) as a
player gets on or off, for themselves (`Player::change_ride`) and for
others (`OtherChar::show_mount_effect`, as v83's FUN_0098385d and
FUN_00983921 do), but not for one already riding as they come into view.
Another player boarding the ship also sounds its `Use`; the owner hears
it on the cast.

`Player.mount` holds the mount's level, EXP and tiredness, saved in
`character_mounts` (a row once it differs from a new mount's 1 / 0 / 0).
SET_TAMING_MOB_INFO (`0x30`) carries them: to the player on entering the
world wearing a mount, and to the map on each change. While riding, the
mount tires by one each minute (the buff's interval, `BuffTick.tire`);
past 99 it stays at 99, the ride ends, and the player is told so (a
BROADCAST_MSG type 6 line). USE_MOUNT_FOOD (`0x4D`, `MountFoodHandler`)
takes a 226xxxx food from USE while riding. The food takes up to its
`spec/incFatigue` (Revitalizer: 30) off for `ceil(taken / 30 * (2 * level
+ 6))` EXP, levelling up by Cosmic's table to 29. The food is used even
on a rested mount, as in Cosmic. CHAR_INFO shows the worn mount's state.

The ride ends on death, a mob's Dispel, logout, and entering a map with
field limit 0x200 (`keep_mounts_off`). The mount and saddle, and their
cash covers, can't be moved while riding. HeavenClient, as v83 does, gets
off instead (CANCEL_BUFF for the job's rider skill, `Player::ride_keeps`)
when a 190xxxx or 191xxxx item is worn, taken off, or dropped on the
field while riding a tamed monster (0x4f1c2d, 0x4f0e97, 0x4f11b4,
0x4f3287; report b150). A double-click in the Equip window never takes
off 1902040 to 1902042 (`mount::stays_on`). Not done: Cosmic's explorer and
Cygnus restrictions on who may equip which mount, and event mounts. The
skill comes with quest 6002 (its `skill` act, `agent_docs/quests.md`),
and the Hog from catching a Tamable Hog (`agent_docs/inventory.md`). Kenta
(2060005) trades a Hog for the Silver Mane (1902001) in quest 6006, which
starts only for a mount of level 3 or more (`tamingmoblevelmin`). HeavenClient draws the rider on the mount (`CharLook::set_vehicle`,
report 201).

Battleship (5221006, report 228) is a ride too, on the vehicle 1932000
whatever is worn (`Player::ride`), with no mount or saddle needed
(`use_skill::board`). Its GIVE_BUFF puts the skill's weapon and magic
defense before the ride's block (`build_give_ride` with stats), and
SKILL_COOLDOWN under the id 5221999 follows with the ship's HP, which the
client shows on the buff icon: `400 * slv + 200 * (level - 120)`
(`mount::battleship_max_hp`). Each hit the player takes in HP comes off
the ship too (`mount::hit_battleship`), and the ship keeps its damage
between rides (`Buffs::battleship_hp`, not saved). At 0 it sinks: the
skill's 90 s cooldown starts, which boarding doesn't, and the player is put
off it. The ship never tires. Not done: refusing the ship's own attacks
off it, and other attacks on it, which the client already does.

### Morphs

A morph item (2210xxx) turns its user into the Morph.nx template in its
`spec/morph` (2210000 is template 1, the Orange Mushroom) for its
`spec/time`, as a `BuffStat::Morph` buff (bit 33) from the item (report
225). Its value is the template, written as a short. Morph is a movement
stat, so GIVE_BUFF ends with the extra byte. The map sees it with
GIVE_FOREIGN_BUFF and in SPAWN_PLAYER, after Soul Arrow. Expiry,
CANCEL_ITEM_EFFECT (right-clicking the icon), death and logout end it like
any item buff. An item's morph (a value below 100) can't
attack: the attack handlers drop the attack (`AttackRejection::Morphed`,
Cosmic's `isMorphWithoutAttack`).

v83 sends no morph item, nor a ghost item (236xxxx), while the user is
already morphed, sits on a portable chair, rides the Battleship (193xxxx)
or has a trade or personal shop open, and says nothing (0xa092fb,
`morph::potion_refused`). To change morph, the user cancels the first.
The server refuses the first three too, and a ghost too, keeping the item
(`world/use_item.rs` `potion_refused`).

The Maplemas Party Potion (2212000, report c041) morphs someone else. Its
`spec/morphRandom` lists templates 23-30, each with a weight `prop`. v83
opens a dialog asking whom to transform (`UITransform`, from 0x4f03c1 and
FUN_00a1005a), with no morph, ghost or chair check, only none while a
request is pending or a mini room is open. OK sends USE_TARGET_ITEM (0xA0)
`i32 tick, i16 slot, i32 item id, str name` and locks the client until
RANDOM_MORPH_RES (0x77) comes: `[0]` unlocks, `[1, 2, str name]` also says
"Failed to find user %s." and `[1, 3]` "You can use random transform
potion only in the town." in the chat log. The server (`UseTargetItemHandler`)
answers `[1, 3]` off a town map, and `[1, 2]` for an empty name, the
user's own, or a character not on the channel, dead, a ghost or in the
Cash Shop (the directory looks the name up, `morph_by_another` applies the
potion to the target, and `morphed_another` tells the user). The target's
morph, one of the weighted templates for the potion's `spec/time`,
replaces any morph they had. One potion is used, and only when someone was
morphed. These rules beyond the client's are defaults (lanes/QUESTIONS.md).

A ghost item (2360000 and 2360001 `spec/ghost` 1, 2360002 `spec/ghost` 2,
each for an hour) makes its user a ghost: a `BuffStat::Ghost` buff (bit 49)
from the item, its value a short, after Morph in GIVE_FOREIGN_BUFF and
SPAWN_PLAYER (report c039). It is a movement stat, so GIVE_BUFF ends with
the extra byte. v83 lets a ghost neither attack nor use any skill (it can
still emote and sit), so the server refuses both as for a morph
(`morph::ghosted`, which `morph::refused` and the party quest skill check
use). Right-clicking the icon ends it, and so does dying. A map change
keeps it. v83 makes its user anew in each field and re-applies only a
morph, not a ghost, from the stats it kept (report c044). So after each
map join the server sends the owner the ghost's GIVE_BUFF again, with
the time left (`show_ghost`). The map sees it in SPAWN_PLAYER.

HeavenClient draws a ghost from the body's ghost actions in Character.nx
(`ghoststand`, `ghostwalk`, `ghostjump`, `ghostproneStab`, `ghostladder`,
`ghostrope`, `ghostfly`, `ghostsit`; `morph::ghost_action`), in the skin's
body file or else `00002000.img`. The value picks the variant child:
1 is the white sheet, 2 the flying saucer (`morph::ghost_variant`,
`GhostLook`). The ghost frame replaces the body, arms and clothes; the
head, hair, face, earrings, eye accessory and cap are drawn on its `neck`
as in the stand (or, for `ladder` and `rope`, whose frames have `face` 0,
from behind as on a ladder). A morph is drawn in its place. A ghost
refuses attacks, skills, map seats, item effects, a trade invite, a
personal shop and the morph and ghost items with the same lines as a
morph (`Player::ghost_refuses`, chosen by the ghost item's
`noCancelMouse`), but may emote and sit on a portable chair; its icon
always cancels it.

The cash morphs (Fungus Scroll 5300000, Oinker Delight 5300001 and Zeta
Nightmare 5300002, templates 1-3 for ten minutes, report c012) are the
same, used from the Cash tab: USE_CASH_ITEM `i16 slot, i32 item id, i32
tick` (v83 cash type 0x28). Unlike a morph potion, v83 sends one even
while morphed, and the new morph takes the old one's place. It sends none,
and says nothing, from a portable chair or a tamed monster (190xxxx;
`morph::cash_morph_refused`); the server refuses those too, keeping the
item (`world/use_item.rs` `cash_morph`). Otherwise it takes the item,
applies its spec (which also heals 50 HP) and answers as for a USE item.
Map seats aren't known to the server.

Oak Barrel (5101007) and Transformation (5111005) and Super
Transformation (5121003) give the skill level's `morph` (1002, 1000,
1001) as the Morph buff, beside their leveled defence, speed and jump; a
woman's superman is the template plus 100 (`net::morph::template`, as
Cosmic sends). `net::morph::refused` keeps the client's skill rules on
the server (reports 225b, 225c): nothing, not even a normal attack, from
inside the barrel (the client's attack key cancels it instead); no
Double Shot, Oak Barrel or Burst Fire as a superman; Shockwave only
transformed and Demolition and Snatch only after Super Transformation.
Use Skill refuses the transformations on maps that forbid movement
skills, like Dash.

HeavenClient draws the template's action for the stance (stand, walk,
jump, prone, ladder, rope, fly; stand when the template lacks it) in
place of the body and equipment (`MorphTemplate`, `morph::action`). The
item's morph moves at the template's speed and jump, clamped to 80-140 and
80-123, with its `fs` and `swim`. It refuses attacks, skills, chairs,
emotes and item effects with "Unavailable while transforming." for an item
with `noCancelMouse`, and the longer line about right-clicking the icon
otherwise. Right-clicking a `noCancelMouse` item's icon shows "You may not
cancel the transformation." and sends nothing.

Oak Barrel (a `hide` template) drops onto the player with the skill's
`effect` and replaces the body 840 ms later, and breaks with `affected`
and `Use2` as the body fades back in over 240 ms (`morph::Barrel`); on a
ladder or rope, or coming into view, it swaps at once. The barrel moves by
its template like an item's morph and is refused the same things, but the
attack key sends CANCEL_BUFF instead, and its icon can't cancel it while
it drops or breaks. Mobs never pick an attack against it, and touching one
does no damage: every 2 seconds of contact rolls `prop` and breaks the
barrel on a hit (`morph::spotted`). HeavenClient also refuses the attack
key while the barrel drops, where v83 attacks and the server would refuse.

A superman (a `superman` template) also shows its alert and sits, and
draws an attack or skill by the template's action of the same name
(`morph::superman_action`; alert5 is the alert), played through once by
its own frames. Every action but the walk plays at attack speed, standing
too. It keeps the player's own speed and jump. It refuses Double Shot, Oak
Barrel, Burst Fire and the beginners' rides with the right-click line
(`morph::superman_refuses`), and emotes and item effects, but may sit. A
normal attack needs a knuckle (or a bow for Albatross): a melee one is
dropped silently, a shot says "This skill cannot be used with the equipped
item." Without the transformation, Shockwave, Demolition and Snatch say
"This skill is available only during transformation."
(`morph::untransformed`). Any morph, a superman's too, also refuses a
trade invite and opening a personal shop with the same line
(`Player::shape_refuses`). A template that can't move the way the map
does isn't put on: in water only 0004 (2210003), which has no `fly`,
stays off, and the buff just runs (`MorphTemplate::moves`). Cliff's
Special Potion (2211000) heals 50 and morphs its user for an hour into one
of the eight templates 23-30 of its `spec/morphRandom` pool, picked by the
entries' `prop` weights (they add up to 100) in their order
(`use_item::random_morph`; no reference server picks one, report c041).
The Maplemas Party Potion (2212000) has the same pool but morphs another
player named in v83's Transform dialog (send 0xA0, reply 0x77; see the
Party Potion above). Any morph also refuses opening a mini-game
(`UIMiniGameCreate::open`) or a hired merchant (`UIPersonalShop::create`,
v83's FUN_00a1dd99) with the same check.

### Item auras

Some item buffs draw an aura over the player: Horntail and Pink Bean
Squad Victory, Breath of Nine Spirit, the ARPQ shield,
Pharaoh's Blessing 1-4 and Power Buff, the flat entries of
`Effect/ItemEff.img` (report c029 A). For the first four v83 shows it through
SHOW_CONSUME_EFFECT (`0xA6`: character id, item id, 0 for none). The client
keeps the id (user+0x1f64) and loops the item's frames at their `z` until
told 0, the owner's own client included; SPAWN_PLAYER doesn't carry it.
`net::consume_effect` derives the aura from the active buffs (an item buff is
kept under `-item_id`) and `refresh` sends the 166 to the player and the map
whenever it changes: when the item is used, and with 0 when its buff is
cancelled, expires or ends at death. The client actor checks again after
each packet it handles and each party skill that reaches the player, so a
skill's buff that takes over all the item buff's stats takes the aura with
them. The field sends it after SPAWN_PLAYER to
those who arrive later and when a hidden player shows, and the client actor
tells the player's own client again on each map join, as v83 makes the user
anew there. `Player.consume_effect` holds what the others were last shown
(not saved).

HeavenClient draws it in `Char::show_consume_effect` (layers clamped to -5
to 20, 5 without a `z`), not while morphed, and `Stage::clear` clears the
player's own at the map change, before the server's resend.

Pharaoh's Blessing and Power Buff draw theirs through their buff instead
(report b339), so they aren't in `consume_effect::ITEMS`: their `berserk`
spec is temporary stat 73 (`BuffStat::ItemAura`) and their `booster` is
Booster (11), its negative value sent as the item has it. 73's value
changes nothing; the client only draws the item's `ItemEff.img` frames on
a layer of their own (user+0x2adc), the item coming as the reason
(`-item_id`), which GIVE_FOREIGN_BUFF and SPAWN_PLAYER carry for the
others. While it's on, Power Explosion's loop is hidden. HeavenClient's
`Char::show_item_aura` draws it. Cosmic gives no damage for
`berserk` either.

Not done: the skills that show an item's aura (5110000, 4300000, 5100000);
and facing (the aura isn't flipped).

## Spending AP and SP

The client locks its UI after SPEND_AP or SPEND_SP until the server answers,
so every request is answered, refusals with an empty STAT_CHANGED.

- **AP** (`Player::spend_ap`): SPEND_AP names the stat by its STAT_CHANGED
  bit: STR, DEX, INT, LUK, MaxHP (0x800), or MaxMP (0x2000). Base stats stop
  at 999 and max HP/MP at 30,000. One AP adds the job's growth (see EXP
  above; a beginner gets +8-12 HP or +6-8 MP). As in v83, the client
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
take 4/5 of the table amount plus up to 2/5 more (OrionAlpha's
`Reward.create`, about ±20%), items take `min`..`max` (see "Rates and rate cards" for
what scales them). An entry with a `period` (minutes) is made as it drops,
expiring that long after, and the drop packet carries the time
(`net::items::dropped`; the wedding's Heart Keys and onyx chests, for
example). A mob worth no EXP still drops and
counts for quests. Quest drops are the entries
the overrides tag with a quest, and quest items (`info/quest`) that only one
quest's completion check (`Check.img/<quest>/1/item`) asks for, unless an
override says `quest = 0`. For a
player out of a party they roll only while that quest is started and the
player holds fewer than the check asks for (any number if it doesn't name the
item); in a party they roll anyway, as Cosmic's do, for the members who need
them. Without a content pack,
mobs drop only the overrides. The client actor rolls (it has
the player's quests) and asks the field to place the loot.

The field's `field_drops::DropSet` spreads a kill's drops around where the
mob died and shows them falling (DROP_ITEM_FROM_MAPOBJECT 1); players
entering see lying drops (mode 2). Each drop lands on the map's footholds
(`game_data::footholds`, loaded with the field template): on the highest
ground below its column, searched from 50 px above the mob as v83 servers do.
A wall between the mob and the drop's column, or no ground under it, keeps
the drop on the ground below the mob, so drops don't end up inside a cliff or
in the air. A quest drop is shown only to the players on the map who still
need it (`net::loot::wants`): the field hands its packet to each occupant's
client actor as `ServerMessage::QuestDrop`, which checks the player's quests
and inventory, as Cosmic's `spawnDrop` does. Removals go to everyone; clients
ignore drops they never saw.

For 15 s only the owner may pick a drop up, or with a party, the owner's
party as it was at the drop; a player outside it who needs a quest drop
sees it but waits like anyone else. A quest drop is also refused to anyone
who doesn't need it, the owner included: the pickup carries a `QuestNeeds`
snapshot of the taker's started quests and the quest items they already hold
enough of. Drops of
mobs with `publicReward` are anyone's at once, as are those of mobs with
`explosiveReward`, which fly out 40 px apart instead of 25. The packet's
ownership type tells the client which (v83 FUN_0050483a, whose pets follow
it): 0 with the owner's character id, 1 with the party id, 2 for anyone (and
for any drop once the 15 s are over), 3 for an explosive drop. Drops vanish
after 180 s (REMOVE_ITEM_FROM_MAP 0), except on the 24 maps with
`info/everlast` (the OPQ and GPQ storage rooms, the Hill of Christmas), where
they stay until picked up, as in Cosmic. Drops players throw down are anyone's
at once; see "Client moves" in `inventory.md`.

HeavenClient's `Drop` gives a falling drop its own, lighter gravity instead
of the player's: it launches at -2.9 px per tick and gains 0.042 px per tick,
so it rises about 100 px and is in the air about 1.1 s, as measured from 2008
GMS footage. Its horizontal speed is set from that air time so it lands on
its destination, and it turns about twice on the way down. A picked up drop
(REMOVE_ITEM_FROM_MAP 2) follows the looter and rises about 50 px, slowing
down, then disappears after about 0.37 s; it stays opaque except for the last
six ticks. An expiring drop (mode 0) fades out where it is. Mode 4 sets
mesos off for Meso Explosion (see "Meso Explosion" above).

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

A Use item with `spec/consumeOnPickup` works the moment it is picked up and
never reaches the bag (report c036; Cosmic's `applyConsumeOnPickup`). The
fruits 2022086-88, which reactors 2402000 and 2402001 drop, are the ones a
player can reach today. The client actor applies the spec as USE_ITEM would
(`apply_item_effect`); its HP and MP update unlocks the pickup, and GIVE_BUFF
`-item_id` follows. One whose spec the server doesn't support yet (the EXP
chests, Carnival points) goes to the bag as before.

A party potion (`spec/party`: the Carnival items 2022160-63 and the Dojo
items 2022430-33) works on every living party member on the picker's map
too, as in Cosmic. The picker's client actor tells the directory
(`DirectoryMessage::PartyItem`). The directory passes it on to the other
members on the same channel and map (`ServerMessage::PartyItem`). Each
member's actor then applies the spec as for the picker, unless they have
died, left the map or are in the Cash Shop, and sends the update without
the unlock. The harness test `party_potions` covers this.

An item's `specEx/0` names a mob skill and level (`ConsumeSpec::mob_skill`).
Without `target`, using the item puts that skill's disease on the player
(`combat::take_disease`, as a mob's skill would, for its `x` and `time`):
Russellon's Pills (2022198, Poison 125/7) and Russellon's Potions
(2022225-27, among 3314's rewards).
The give-buff names the mob skill | level << 16, and the map sees it as a
mob's disease. With `target` (`ConsumeSpec::mob_skill_on_others`), as the
Ariant Coliseum's Crazy Skull (2022268, Confuse 132/1) has, it works on
everyone else on the map instead, as in Cosmic: picking it up (its
`consumeOnPickup` is under `specEx`) sends the field
`FieldRequest::DiseaseOthers`, and each other player catches the disease as
from a mob (`ServerMessage::Diseased`). The picker gets nothing.

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
  higher defence, Hypnotize's Dazzle) and a learned Venom's poison. Each mob hit for more than
  0 rolls each. Elemental Boost makes Ice Splitter's freeze last its `y`
  seconds longer and Flamethrower's burn hit its `x` percent harder
  (`skills::elemental_boost`).
- Taunt also puts on Showdown (bit 11), which no v83 client reads: a mob
  that dies with it gives its value in percent more EXP and makes each of
  its drops that much likelier (`Kill::showdown`, `loot::roll`; the
  OdinMS-family rule).
- Hypnotize (5221009) has no `damage` and hits for the plain blow, like
  Disorder and Taunt. Its Dazzle (bit 28) stops the mob attacking players
  on the client. Other mobs' attacks that reach it then hurt it: each
  client that sees it computes the damage and sends MOB_DAMAGE_MOB (0xC2:
  attacker, character, victim, a byte, damage, then a flag and a point that
  go unread). The server (`MobSet::mob_hit`) takes it only for a live
  Dazzled victim of a live attacker that isn't, within 600 x 300 px, once
  in 2 s per victim (so the copies from several clients count once), and
  cuts it to the most the client's formula gives the attacker's best
  attack and statuses against the victim's defense, or its magic attack
  against none (`mob_hit_cap`); a hit that comes to nothing is dropped.
  The sender is credited as if they dealt it (Cosmic's rule), and the
  others get DAMAGE_MONSTER (0xF6) with the number and HP; a kill goes the
  usual way. The friendly-mob variant (0xC0, the Moon Bunny's) is in
  `party-quests.md`. In
  HeavenClient the client controlling an undazzled attacker collects its
  landed box hits and its balls in flight (`Mob::take_sweeps`), checks them
  against up to 15 other mobs' bodies (`MapMobs::update`), and the victim
  (`Mob::hurt_by`) rolls the damage with v83's formula (`mob_attack::mob_hit`),
  shows the hit and number and sends 0xC2. Unlike v83 it doesn't turn the
  victim on its attacker, and direct shots (type 1) hurt no mob.
- Bosses shrug off Stun, Freeze, Seal, Doom, Web and Dazzle. A mob immune to poison
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

A Night Walker's Poison Bomb (14111006) and a Blaze Wizard's Flame Gear
(12111005) leave the same poisoning cloud. Flame Gear's sits around the
caster like Poison Mist's. Poison Bomb's falls where the grenade went off:
its attack ends with that spot after where the player stands (v83
0x952b89, `Attack::grenade`), and `skills::mist_origin` centres the box
there unless it is more than 1000 px from the thrower.

A Shadower's Smokescreen (4221006) is a USE_SKILL that asks the field for
a type 2 (smoke) mist around the caster that puts nothing on mobs. The
v83 client shelters itself from mob attacks while it stands in its own or
a party member's smoke, so the server has nothing more to do.

Cash mists (Passed Gas 5281000, Floral Scent 5281001) are type 3 mists
that do nothing to anyone; see `inventory.md`.

HeavenClient's `MapMists` lays the skill's `tile` clouds over the area as
v83 does (`MistLayout.h`, a diamond of random tiles; Flame Gear's is its
one tile at the middle of the area's bottom), draws them above the
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
stay put, and `BuffStat::Summon` for the rest. The client shows that icon
on Combo Attack's bit, as Cosmic does, but a Dawn Warrior has both, so the
server keeps them apart as two stats. While Combo Attack is up, the
summon's icon is never sent, nor its end (`Buffs::heard`), so the orbs
stay. If Combo Attack ends with a summon out, its cancel takes the
summon's icon away too. The field then takes the summon
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
  it.
- Each hit lands `attackAfter` after the attack starts, plus 120 ms for
  every hit of the spreading kinds and 50 ms more for each hit after the
  first of the others (v83 0x7a55d4, report c028 section 8).
- Only the Octopus and Wrath of the Octopi fire a ball
  (`summon/attack1/info/ball`). It leaves the summon's `range/sp`, mirrored
  when the summon faces right, at `attackAfter`. It flies straight to where
  the muzzle's row meets the mob, taking 600 ms per `bulletSpeed` px
  (300 px in 600 ms), turned to point along its path. The hit lands as it
  arrives (`summon_rules::muzzle`, `ball_time`, `ball_angle`).
- Another player's summon fires no ball. All of its hits land at
  `attackAfter` (FUN_007a6882).

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
state without events is the end. A linked reactor keeps its own `action`.

A basic attack can swing at a reactor instead of attacking mobs. Event
types 0 to 4 take a swing: 0 any, 1 and 2 only facing right or left, and 3
and 4 the same but in the air. Of a state's events, a swing sets off the
first of the best rank (`reactors::swing_rank`, v83 0x736091): a side or
air event beats a plain one, and a plain one is better on the ground.

The field's `field_reactors::ReactorSet` spawns every reactor on load
(SPAWN_REACTOR `0x117`, also sent to players entering). DAMAGE_REACTOR
(`0xCD`) swings at one if the player is within 250×200 of it, by the
field's idea of where they are, or uses a skill by it (below). A hit that leaves it short of its last
state is shown to everyone (HIT_REACTOR `0x115`, with the event and the
client's delay); the hit into the last state removes it (REMOVE_REACTOR
`0x118` with the state it reached) and the hitter rolls its drops
(`net::loot::roll_reactor`) where it stood. A broken reactor comes back after
the map's `reactorTime` seconds, under a new object id since HeavenClient
keeps a broken reactor under its old one; one without a `reactorTime` stays
broken until the field renews: a field that starts afresh when its last
player leaves (`npc_scripts::resets_when_empty`) puts every reactor back with
its mobs (`ReactorSet::renew`), so the fourth job's nests have their eggs for
the next player.

| Op | Direction | Body |
| --- | --- | --- |
| 0xCD DAMAGE_REACTOR | client to server | `i32` object id, `i32` kind (0 a swing, 1 a skill), `i32` flags (bit 0 facing left, bit 1 on a foothold), `i16` delay in ms, `i32` skill |
| 0x115 HIT_REACTOR | server to client | `i32` object id, `u8` new state, `i16` x, y, `u16` delay, `i8` event of the old state (-1 its own hit), `i8` cooldown in 100 ms |
| 0x118 REMOVE_REACTOR | server to client | `i32` object id, `u8` state reached, `i16` x, y |

A skill used by a reactor (kind 1) moves it by an event of type 5, which
lists the skills it takes (`activeSkillID`) and where the user must stand,
`lt`-`rb` from the reactor (`ReactorTemplate::skill_used`). The handler
takes only a skill the player has. The field takes the first such event of
the reactor's state that lists the skill, if the player stands in its area
by the field's idea of where they are, give or take 100 px
(`ReactorSet::skill_used`, `SKILL_SLACK`), since the field's position
trails the client's. Of the reactors that take skills, only Crimsonwood
Keep's sigils do anything more on the server (`Action::Sigil`).

In v83 every active skill the player uses, an attack's or a buff's, right
after it readies the pet's MP potion, goes through the map's reactors
(FUN_00735ab7, from 0x9693ce). For each reactor not already playing a hit
and not cooling down, the first type-5 event of its state that lists the
skill and whose area, moved to the reactor, holds the player sets it off:
the client plays that event's hit 800 ms on and sends kind 1 with the
skill and that delay. It goes on to the other reactors, so one skill can
set off several. HeavenClient does the same, trying the reactors in the
order they entered the map (`Combat::use_skill_at_reactors`,
`Reactor::use_skill`, `reactor_rules::pick_skill_event`).

Nine reactors go off by touch (`info/activateByTouch`, `ReactorTemplate::touch`): Crimsonwood
Keep's fourth stage's stirges and flames (6109013, 6109014, 6109021 to
6109027). Once a second (v83 polls on its update, `CField` reactor pool) the
client checks which of them the player stands in, by each one's drawn
rectangle, and on each change sends TOUCH_REACTOR (`0xCE`): stepped on or
off. It keeps the set of reactors it stands in for the field and forgets
it on a new field. HeavenClient does the same (`Stage::update`,
`MapReactors::touch`, `reactor_rules::touch_changes`).

The field keeps who stands on each (`ReactorSet::touch`), taking a step on
only within 250×200 of the reactor by the field's idea of where the player
is. The first one on moves it by its state's event of type 6, and the last
one off by type 7, each shown with HIT_REACTOR and no delay
(`ReactorTemplate::touched`; all nine go 0 to 1 and back). A player who
leaves the field, or comes into it again, steps off everything
(`ReactorSet::step_off`). What the stage does with them is in
[party quests](./party-quests.md#crimsonwood-keep).

| Op | Direction | Body |
| --- | --- | --- |
| 0xCE TOUCH_REACTOR | client to server | `i32` object id, `u8` 1 stepped on, 0 off |

An event of type 100 takes an item instead: exactly `1` of item `0`
landing within `lt`-`rb` of the reactor (`ReactorTemplate::item_dropped`).
When a player drops such an item (not one that vanishes), or a mob or
reactor drops one that isn't a quest drop (`reactor_item`, as Cosmic's
`spawnDrop` does), and it lands, on the ground below, in such an area,
the field waits 5 seconds, as Cosmic's `ActivateItemReactor` does (`ITEM_TAKEN_AFTER`). If the
item still lies there and the reactor still wants it, the item goes
(REMOVE_ITEM_FROM_MAP, no one picking it up) and the reactor moves on as if
hit, shown with HIT_REACTOR event 0, or breaks. A reactor that then waits
for another item looks at what already lies on it. The resurrection mound
(`2302003`, `923000100`) takes a Cold Flame, then the Black Book; the
fourth job's eggs (`2401001`, `2401002`, `924000100`) take their egg, then
three hits.

A state whose events have a `timeOut` (in ms) goes back on its own that
long after the reactor reaches it, through its last event's `next` (Cosmic
`refreshReactorTimeout`): the field's tick (`ReactorSet::revert`) shows it
with HIT_REACTOR event 0 and looks again at what lies on it. A state
reached through an event whose index in the new state leads back, or is
missing, is an end state (Cosmic `hitReactor`, `ReactorTemplate::acts_in`):
there it waits out no timeout and only moves, as in Cosmic, unless
`reactors::acts_staying` names it. Those do what breaking would, yet stay
(`Hit::Acted`): the Forest of Poison Haze's pond (`3002000`) takes a
Concentrated Poison, drops a Diluted Poison, and two seconds on takes
poison again (Cosmic refreshes no timeout in its end state, so its pond
would stay full; the NX's 2-second `timeOut` back to state 0 is followed
here). The forest's spine (`3009000`) stays in its last state and clears
the stage there (`Action::DropsAndClears`), as Cosmic triggers an
item-triggered reactor's last step rather than destroying it.

What a broken reactor does is `net::reactors::action`, keyed by reactor id
as Cosmic's scripts are: by default the breaker rolls its drops; the eggs
bring out the Fire Hawk or Ice Eagle (`9300089`, `9300090`), the
Hitman's hideout boxes (`1021000`, `1021001`) its `9300091`, and
Ludibrium's stage 3 boxes (`2201001`) three Bloctopuses (`9300007`), on
the ground just below the reactor, without drops (`Action::BringOut`, a
mob and a count). Others send the breaker to a portal
(`Action::Warp`, looked up by name): the hideout's boxes `1020000` to
`1020002` lead up its floors (`pt00` to `pt02`, within the map, so the room
stays as it is). The Berserk room's false statues (`1050000`) drop their
loot (none) 30 times in a hundred, else throw the breaker out to
`105090200`'s first `sp` (`Action::DropsOrWarp`). The field tells the
breaker's client actor (ReactorBroken), which rolls the drops or warps.
Ludibrium's stage 2 trap box (`2200002`, `Action::WarpMap`) sends everyone
on the map away: the field shows them all its pink line, then tells each
of their client actors (see "Ludibrium's balloons" in `party-quests.md`).
Its stage 7 and boss reactors (`2201002`, `2201003`, `Action::SpawnAt`)
show everyone a pink line and bring out Rombad or Alishar at a fixed
point of the map; Showa's statue (`8001000`) brings out a bodyguard the
same way, with no line ([showa](./showa.md)). The boxes on its bonus stage (`2202004`, `Action::Spray`)
drop their loot with piles of mesos (`loot::spray`): one first, and more
until there are 15 drops.
Orbis's light (`2006000`, `Action::Appear`) shows everyone a pink line and
brings out an NPC, Eak, on the ground below it (`FieldActor::appear`); it
starts in state 20 (`reactors::first_state`, see "Orbis's cloud stage and
hub" in `party-quests.md`). Minerva's statue on Orbis's hub (`2006001`,
`Action::AppearAndClears`) starts in state 6 and brings out Minerva the
same way, without a line, then tells the breaker's client actor, which
clears (and so wins) the stage (see "Orbis's garden").
Primrose Hill's moonflowers (`9108000`-`9108005`, `Action::Bloom`) never
break: reaching their last state they stay (`reactors::stays`), and the
field waxes its full moon (see "Henesys's moonflowers" in
`party-quests.md`). The scars on Orbis's hub (`2008000`-`2008005`,
`Action::Hold`) stay too, holding the piece of the statue set in them (see
"Orbis's walkway and lounge" in `party-quests.md`). The spots of Orbis's
Salion stage (`2001002`-`2001015`, `Action::Salion`) take the Cellion's
drop and bring out the next Cellion, until the last drops its loot and
clears the stage (see "Orbis's Salion stage" in `party-quests.md`).

The Berserk room's (`910500200`) reactors change places among themselves
whenever the field starts afresh (`reactors::shuffled`,
`ReactorSet::shuffle`), so the true statue (`1052000`) stands somewhere new,
as Cosmic's `shuffleReactors` at the room's entry does. The map's
`info/reactorShuffle` flag isn't read.

Server logic can also set a reactor's state by its map name
(`ReactorSet::set_state`, shown with HIT_REACTOR): the Helios Tower
elevator's car docking and leaving (`ships.md`).

Reactor drops come from the content pack's table, keyed by reactor id, plus
`[reactors]` in `reward_overrides.toml`; see `server-content.md`. The
`action` script names (`mBoxItem0` and others) aren't used.

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
A skill sets off reactors as described above, with the event of the
reactor's current state.
A swing sends the attack (`0x2C`, no mobs) and then `0xCD`, and the attack
hits no mobs. The rules are in `Gameplay/MapleMap/ReactorRules.h` and
tested in `tests/reactor_rules.cpp`.

One deviation: before every basic attack, v83 checks that no item, stat
or skill request is waiting on the server and 200 ms have passed since the
last (`ExclRequest::USE_WAIT`). HeavenClient checks this only before
swinging at a reactor, so a request the server leaves unanswered can't stop
a player attacking.

Reactors touched rather than hit (`0xCE`) are covered above. v83 also
skips a swing at a reactor whose `info/quest` isn't in progress, but no v83
reactor has one.

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

A magic attack with an element, and a fire (130) or poison (125) mob
mist, lose the job's resistance skill's `x` percent (`Player::resists`:
Dragon Knights, Priests and Blaze Wizards resist every element, F/P
Wizards fire and poison, I/L Wizards ice and lightning). Poison Mist (131)
always hurts its full `x`. HeavenClient leaves out `tremble` (the screen
shake), `doFirst`, hits on summons and other mobs, and attacks on a
Puppet.

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
  mob in its `lt`/`rb` area, with chance `prop`, for `time` seconds. An
  area is written for a caster facing left and mirrored when it faces
  right, except for a mob that never turns (`noFlip`, as Pianus), whose
  areas stay as written whatever stance its controller reports. Power
  Up and Magic Up set the PowerUp and MagicUp stats to `x`, a percent the
  client scales the mob's physical or magic hits by. Heal (114) gives the
  mobs in its area `x` HP and `y` MP, up to their maxima, shown to everyone as a blue
  number (DAMAGE_MONSTER with a negative amount). Summon (200) spawns its
  listed mobs at the caster under the caster's controller, up to `limit`
  alive, with SPAWN_MONSTER's appearance set to `summonEffect`. A level
  with an `lt`/`rb` area (11 of 176: the Papulatus Clock's Darkstars,
  Pianus's Bloody Booms, a few event mobs) puts each at a random point in
  it, on the ground below; with no ground there, at the caster (the
  field's `summon_spot`). Clients
  play `Effect.nx` `Summon.img/<summonEffect>` on them. Summoned mobs don't
  respawn. A mob summons nothing on a field already holding 80 mobs
  (Cosmic's cap).
- **Diseases** (120-126, 128, 132) reach the players in the area; see
  below.
- **Counters** (143 physical, 144 magic, 145 both) put PCounter and
  MCounter at `x` on the mobs it reaches: each attack on one hurts the
  attacker `x`, give or take half of `y` (`MobSkillEffect::Counters`).
- **Immunities** (140 physical, 141 magic): a mob under PImmune takes at
  most 1 a damage line from a player's weapon attack, and under MImmune
  from a spell (`immunity_against` in `runtime/src/actor/field.rs`, as
  Cosmic's `AbstractDealDamageHandler`). The attack's header gives the
  lines; Meso Explosion caps each target by its own. A whole miss still
  deals 0, and summons and reflected hits aren't judged.
- **Dispel** (127) ends the skill buffs, and **Banish** (129) sends up to
  `count` players to the mob's `info/ban` map, of the players in its area,
  each cast landing by its `prop`. **Poison Mist** (131) lays a mob mist over
  its area by its `prop`; each client in it hurts itself (see above).

### Diseases

A disease is a buff stat the server puts on a player, whose skill (the
reason) is the mob skill | level << 16 and whose value is the level's `x`
(1 without one): Seal (120) 19, Darkness (121) 20, Weakness (122) 30, Stun
(123) 17, Curse (124) 31, Poison (125) 18, Slow (126) 32, Seduce (128) 39
and Confuse (132) 51 (`game::mob_skill::inflicted`).

- **Catching one:** a cast lands by its `prop`, rolled once, on every
  player in the caster's `lt`/`rb` box (mirrored when it faces right), or
  without a box on its controller. Seduce (128) takes at most its `count`
  of them (`diseased_most`, as Cosmic's `applyDisease`). A mob attack with `disease` and `level`
  brings one by that level's `prop` when it does damage. A dead player, or
  one who has the stat already, catches nothing (`catch_disease`). The
  player gets GIVE_BUFF for `time` seconds; the map sees the shown ones
  (all but Slow) in GIVE_FOREIGN_BUFF with the reason as the value (Poison
  sends its `x` then the reason). The server ends it on time or on death,
  and the player can't cancel it.
- **Effects on the server:** Poison takes `x` HP each second, never the
  last one, with HP_TAKEN before each stat update (v83 checks the pet's HP
  potion after each tick, 0x94b1b4). Curse halves the EXP from kills (as Cosmic does). Darkness's
  lower accuracy isn't applied.
- **Dispel (2311001):** for each party member reached it rolls its `prop`,
  then cures Poison, Seal, Darkness, Weakness, Curse and Slow (not Stun,
  Seduce or Confuse) with CANCEL_BUFF and CANCEL_FOREIGN_BUFF. A sealed
  player can still cast it.
- **Hero's Will** (the 4th job's 1121011 and its kin, Pirate's Rage
  5121008 and 5221010 among them; `skills::HEROS_WILL`, from v83
  FUN_00765e2d): cures Dispel's diseases and Seduce, Confuse and Undead
  (Cosmic's purgeDebuffs) the same way, then cools down for its `cooltime`.
- **What the client does** (HeavenClient `Character/Diseases.h`, from v83
  CUserLocal): Stun stops all input, attacks and skills; Seduce moves the
  player by its `x` (1 left, 2 right, 3 jump, 4 down, 6 and 7 jump left and
  right) and stops jumps, attacks and skills but Hero's Will; Confuse
  reverses the arrows (teleport too); Weakness stops jumping; Seal stops
  every skill but Dispel; Slow sets the walking speed to its `x` before the
  80-140 clamp. Each disease shows `MobSkill.img/<skill>/level/<lvl>/affected` on the
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
the skill's `special` animation, and Shadow Shifter smiles for 1.5 s. On a
map whose `fieldLimit` has bit 0x2, neither skill nor Power Stance takes
effect (0x9585a5, 0x958778, 0x958af6). The
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
(HeavenClient's `Mob::show_reflect`). A mob's attack, miss or not, plays its
`info/hit` on the player it hit (`Stage::show_mob_hit`, FUN_009832e3): at a
random spot on the body, flipped when the mob is on the right, or, for a
`hit/attach` one, on the player. Its attack1, attack2 and attack9 also play
the mob's `CharDam1`, `CharDam2` and `CharDamF` sound. The player's own hits
do the same, but only when they take damage. When Power Stance (or, with
a full gauge, Energy Charge) keeps the player in place, their own client
plays the skill's `special` at their feet, and Energy Charge's `Use2`
sound (`Combat::show_stance`, 0x958c52). Nobody else sees a player's
Power Stance or knockback. Hits with no mob (-4) end with a byte: 1
poison (ignored, the server ticks poison itself), 2 a mist (Achilles
applies), or 3 Stop Motion (no guards, never fatal). At 0 HP the player is
dead (`death_penalty`). Non-beginners lose part of their level's EXP: 1% in a
town (`info/town`), otherwise 10%, or 5% with 50 base LUK or more. Nobody
loses any on a map with field limit 0x80000, which covers the Dojo and most
event maps. These are Cosmic's rules; v83 never reads the bit. Holding a
charm keeps the EXP instead and uses up one charm. The charms are tried in
Cosmic's order: the Safety Charm (5130000), the Easter Basket (4031283), then
the Easter Charm (4140903). A grey chat line confirms it (SHOW_ITEM_GAIN_INCHAT
mode 6). For a Safety Charm it gives the charms left and the days the next one
lasts; any other charm is named. The server also keeps the spawn point nearest
where the player fell.

Some maps hurt whoever is on them (`info/decHP`): El Nath's snowfields, the
Aquarium's seabed, Sharenian, and others, 123 maps in all. v83 reads none of
`decHP`, `decInterval` or `protectItem`, so the server does it
(`runtime/src/actor/client/hazard.rs`, `net::hurt_by_hazard`). From a field
join, every `decInterval` ms (only the Balrog's maps have one, 15 s), or
otherwise every 10 s, the player loses `decHP` HP. HP_TAKEN (`0xD4`) with
the amount goes before the stat update, since a v83 client checks the pet's
HP potion on that packet but never on a stat change.
Nothing happens while they're dead, in the Cash Shop, or wearing the map's
`protectItem` (1102109 in the cold, 1102061 under water). The 10 s comes from
Cassandra's Magic (2022187), which protects "every 10 seconds underwater".
Cosmic and StrategyWiki both say 5 s, so the interval is a guess.

Two more things keep hazards off, both server-side too:

- A thaw item (`spec/thaw`) gives the Thaw buff (temporary stat 29), which
  takes its value off each hit of its own kind of hazard. Red Bean Porridge
  (2022001, 10) says "10 HP will be protected" in the cold. Air Bubble
  (2022040, -6) and Cassandra's Magic (-6) are for the water. So a negative
  thaw counts on `swim` maps, a positive one elsewhere. That reading is
  ours; Cosmic sends MAP_PROTECTION instead.
- An equip scrolled with ADD PREVENT COLDNESS (2041058, `FLAG_COLD`) keeps
  the cold off. A hazard is cold when it isn't under water and the map
  either returns to El Nath (211000000) or the internet cafe (193000000),
  as Cosmic has it, or wants the cold cape (1102109).

Traps hurt too: objs whose `Map/Obj` node has `obstacle`, such as the jump
quests' thorns and lasers, Adobis's fire and the Leafre animals (3514
placements on 276 maps). The client tests them itself, as v83 does
(FUN_00642264 and FUN_00642710; `ObstacleRules.h`). Each tick a trap's box is
its current frame's `lt`/`rb` on the obj, mirrored when the obj is flipped,
and frames without one, or under 5 px both ways, don't hurt. The client takes
the last-loaded trap the player's body meets, under the same gates as a mob's
touch, and rolls its `damage` within a tenth either way when it's over 10. A
`forceHP` trap (Test of Agility's menhirs) takes the player down to that
much HP, or hits for all of it when they're at or below it. The hit knocks
the player by the trap's `dir`, which v83 turns into a push with
`90 - 30 * dir` taken as radians, so only 3 points where it says. Without a
`dir` the push is 270 back against the player's step and up unless they're
rising. The hit then makes the player invincible and blink as a mob's does,
with no miss, guard or Power Stance, and goes out as TAKE_DAMAGE from -3:
`tick, -3, 0, damage, short disease << 8 | level, 0`. The server
(`obstacle_damage`) accepts up to the map's highest `damage` plus a tenth, or
the player's HP on a map with a `forceHP` trap (`game_data::Traps`), takes
Achilles' and Magic Guard's shares, and shows DAMAGE_PLAYER -3, which has no
mob. Like Cosmic and Kinoko, it doesn't give the disease. A trap with a
`targetField` (the Witch Tower's snacks, back to 980040000) sends a
CHANGE_MAP there on touch, hurting or not, which the server allows a living
player from that map.

Named traps can be switched between states by the server with
FIELD_OBSTACLE_ONOFF (139: name, state; `build_obstacle_state`), which the
field keeps (`FieldActor::obstacles`, through `move_obstacle`) and sends a
player who enters with its list (140, `build_obstacle_states`); nothing
resets them, as the event maps they're on are thrown away. Only the Orbis
jails' spears use them so far. A trap with `mobdamage` (only those spears)
also hurts mobs: as v83 does (FUN_006675a8), each client tests every live
mob once a second, with its body box swept back to where it stood the tick
before, against the traps with `mobdamage`, rolls it within a tenth, and
sends FIELD_DAMAGE_MOB (0xBF: `i32 oid, i32 damage`; HeavenClient's
`MapMobs::hit_traps`); the mob it controls flinches. Every client in the map
tells of the same hit, so the field (`handle_field_damage_mob`) takes it only
on a map whose traps have moved, up to the map's highest `mobdamage` plus a
tenth (`Traps::max_mob_hit`), and once in 900 ms for each mob, then shows
everyone the damage as a mob's hit would and kills as usual. Cosmic takes up
to 2000 from anyone, with no wait.

The client's respawn is a CHANGE_MAP with a target while dead. It returns the
player to the map's return map with 50 HP. When the player chose a Wheel of
Destiny (5510000) in the death dialog (the wheel byte after the portal name),
one wheel is used instead. The player then revives on the same map, at the
kept spawn point, and a grey line gives the wheels left (mode 0x15). After a
charm kept their EXP, the player revives with 30% of their max HP and MP added
rather than at 50 HP, as in Cosmic.

After a hit, HeavenClient ignores touches for 1500 ms, as v83 does. If the hit
did damage, the player blinks meanwhile: half brightness for 60 ms, then
normal for 60 ms. Other players don't blink. Dark Sight keeps touches off
too, but not on a map whose `fieldLimit` has bit 0x2, and never the Master
Guardian's (9400594; `field_limit::may_touch`, FUN_00678c77).

HeavenClient's `Player::update_death` notices HP reaching 0: the player
becomes a ghost (`Char::State::DIED`), the tombstone from `Effect.nx/Tomb.img`
drops behind them (drawn by `Char`) as `Sound.nx/Game.img/Tombstone` plays
(from `Char::set_state`, so other players' deaths are heard too, quieter the
farther they are; someone already dead when they appear lies under a landed
tombstone, unheard),
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
client sends USE_DEATHITEM (0x35: the wheel's id and the tombstone's position)
and plays `BasicEff.img/TombUpgrade` there once (FUN_0095af8e, report b334).
The server, for a dead player holding the wheel, sends the rest of the map
SHOW_UPGRADE_TOMB_EFFECT (0xC3: the player's id, the wheel's id, x and y), and
they play the same animation (`UseDeathItemHandler`); the wheel isn't used
then. YES sets the wheel byte. The SET_FIELD that follows
carries the new HP, which brings the player back up.

Some mobs' hits knock items out of the bag (`loseItem/<n>`: `id`, `prop`,
`x`; `game_data::mobs::LoseItem`), as the Ariant Coliseum's bomb 9300166
knocks out Spirit Jewels (4031868). Each hit that does damage rolls `x`
times at `prop` percent. As many as come up, but no more than are held,
leave the bag (`knocked_out`). They fall one at a time from the player
(`FieldRequest::Shed`), for anyone, as Cosmic's TakeDamageHandler drops
them: the first at the player's feet, then 37, -25, 62, -50, ... pixels
out, on the ground below. A Barrier (bit 50, the `barrier` of 2022269,
which works when picked up) keeps them all in.

## Keys

New characters get v83's default keymap (`game/src/keymap.rs`), with actions as
HeavenClient numbers them: Ctrl → attack (53), Z → pick up (51), X → sit (52)
and B → Monster Book (59), among others. The client's DEFAULT button in Keyboard
Setting restores the same table.

Which keys the status bar's eight quick slots stand for is stored per character
(`game/src/quick_slots.rs`, table `quick_slots`). QUICKSLOT_INIT (`0x9F` out)
follows SET_FIELD on world entry: a flag byte, then eight i32 scan codes when
the character has a layout, or just 0 for v83's defaults (Shift, Insert, Home,
Page Up over Ctrl, Delete, End, Page Down). QUICK_SLOT (`0xB7` in) carries
eight i32s; the server keeps them only if all eight are keys with a quick slot
label and none repeats.

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
`ariant_bomb` (integration) opens the Coliseum's Bomb only in its battle
map, and has its bomb knock Spirit Jewels out until a Barrier keeps them in;
`reactors` (integration) breaks two of Amherst's boxes, before and during
Pio's quest;
`drops_and_pickup` checks ownership, reach, pickup, replays, meso
persistence, and that the tutorial sentinel's quest drop needs its quest;
`spend_points` levels a beginner and checks AP, the beginner pool, refusals,
and that both survive logging out; `monster_book` fills a card's page from
six snails, checks the effect another player sees and the cover, and that
both survive logging out; `rate_cards` has a 3x EXP card triple a snail's
EXP and Hunter's Luck double it; `beginner_skills` has three players learn
Three Snails, Nimble Feet, and Recovery and checks their damage caps, MP,
buffs, cooldowns, Recovery's heal, what the others see, and that the buffs
end on time and on death. `hp_costs` has a Dragon Knight spend HP on Slash
Blast and Dragon Roar, checks Dragon Roar's stun and its end, refuses
Dragon Roar under half HP, and has Dragon Blood drink HP until it ends
early. `poison_mist` casts Poison Mist on a snail and
checks the cloud's packet, that it lifts on time, and that it poisons only
the snails in its box. `smokescreen` checks the smoke's packet and that
the 600 s cooldown refuses another. `snipe` has a Marksman Snipe snails: 200,000 is
refused, 199,999 kills and starts the 4 s cooldown, which refuses another
until it ends. `critical_shots` has an Assassin with Subis shot for 16,
its cap, but not 17, then for 32 but not 33 once it learns Critical Throw.
`summons` has a ranger call up a Silver Hawk and a
Puppet, and checks what a watcher sees: the spawns, a refused and an accepted
Hawk attack, the Puppet breaking, and the Hawk following its owner to the
next map with no entrance. It also has a Dark Knight's Beholder heal them
and buff their weapon defense, refusing both too soon and a stat Hex's
level doesn't give, and an Arch Mage's Elquines freeze a mob for 3 s.
`mounts` has a rider on a tired Hog get on in Henesys. A player coming
onto the map sees them riding. A Revitalizer levels the Hog up, for both
of them, and getting off shows on both. Food is refused once off, and the
Hog's state survives logging out.
`mystic_door` has a Priest open a door, refused in town, use it to town
and back, keep a stranger out until they join the party, and close it on
logout.
`party_skills` has a Bishop resurrect a dead party member, and a Chief
Bandit heal themselves with Chakra.
`combo_attack` has a Crusader refused Panic without an orb, gather five
orbs by swinging at a snail (a sixth swing adds none), and spend them with
Panic.
`player_diseases` has a Hogul in Korean Folk Town cast Slow from far away
(it misses) and then on the player (it lands for 10 s, then ends).
`boss_hp_bar` has a player hit Neo City's 7220003 and checks that another
player on the field sees its bar in its colours, and the bar's removal when
it dies.
`net::consume_effect` covers the aura following the item buff;
`consume_effects` has a player drink Horntail Squad Victory and checks that
they and a watcher see the aura, that it is shown again to the watcher
coming back and to the drinker on each new map, and that cancelling the
buff takes it away for both; that a skill's buff taking over its stats
takes it away; and that Pharaoh's Blessing gives Booster and 73, shows the
watcher 73 with the item, never a 166, and takes 73 away when cancelled.
`map_traps` has a player in the Forest of Patience take a thorn's 1 damage,
which another player sees as DAMAGE_PLAYER -3, and has a hit of 2 refused.
`map_hazards` warps to El Nath's Ice Valley in its cape, which keeps the
cold out, then takes it off and drinks an Air Bubble, which doesn't help: it
loses 10 HP at the next tick, after an HP_TAKEN of 10. After a Red Bean Porridge it loses nothing.
HeavenClient's `diseases_tests` cover the input, jump, attack and skill
gates and the notices; `combo_tests` cover the orbs' damage table and
`combo_orbs_tests` what they show; `flash_jump_tests` cover Flash Jump's
push, its gates and its effects.
