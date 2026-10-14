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
  anyone else is ignored.

Packets follow the v83 layout (16-byte temporary status mask after the mob
id); HeavenClient used to skip 22 bytes there and now skips 16.

## Attacks

CLOSE_RANGE_ATTACK (`net::combat::parse_close_attack`) is checked before it
reaches the field (`validate`): alive, no skill (skills come later), one mob,
one hit, and each hit at most twice the character's basic maximum
(`max_base_damage`, v83's weapon multiplier × main stat + secondary stat,
× weapon attack / 100). The field shows the swing to the others, applies the
damage, sends the attacker SHOW_MONSTER_HP, and on death KILL_MONSTER to
everyone.

A dead mob's EXP is shared by damage dealt (the top damager gets any
rounding remainder) and each share goes to that player's client actor
(`ServerMessage::MobKilled`), which adds it with `Player::gain_exp`. A level
gained sends the new level, AP, SP, max HP/MP (+14/+11 per beginner level,
the middle of v83's ranges) and shows SHOW_FOREIGN_EFFECT 0 to the others. As
in v83, current HP and MP are not refilled. Each level grants 5 AP, and 3 SP
once past beginner.

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
entering see lying drops (mode 2). Quest drops are shown only to their owner.
For 15 s only the killer may pick a drop up (quest drops: always only the
owner); drops vanish after 180 s (REMOVE_ITEM_FROM_MAP 0). Drops players
throw down are anyone's at once; see "Client moves" in `inventory.md`.

ITEM_PICKUP (`0xCA`) needs the player within 800×600 of the drop. The field
removes it (REMOVE_ITEM_FROM_MAP 2 with the looter) and hands it to the
client actor, which adds mesos (MESO stat update, "You have gained mesos")
or the item (INVENTORY_OPERATION, "You have gained an item"). If the item
doesn't fit, the player sees "You can't get anymore items." and the drop goes
back on the ground.

## Getting hurt

TAKE_DAMAGE from a touch (-1) is accepted up to the touching mob's
`PADamage`, only for mobs with `bodyAttack` and without `notAttack`
(HeavenClient's `Mob::deals_touch_damage` applies the same rule, so other
mobs neither hurt nor knock back on touch). HP
drops, others see DAMAGE_PLAYER, and at 0 HP the player is dead:
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

New characters get Ctrl → attack (53) and Z → pick up (51) in their default
keymap, as HeavenClient numbers its actions.

## Tests

`field_mobs` unit tests cover spawning, control hand-off, movement, damage,
EXP sharing, and respawn; `net::combat` covers parsing, the damage cap, and
the broadcast; `mob_combat` (integration) runs two players in Dangerous
Forest through control, relayed movement, hits, kills, EXP, refused damage,
touch damage, a level-up seen by the other player, and a respawn;
`drops_and_pickup` checks ownership, reach, pickup, replays, meso
persistence, and that the tutorial sentinel's quest drop needs its quest;
`spend_points` levels a beginner and checks AP, the beginner pool, refusals,
and that both survive logging out.
