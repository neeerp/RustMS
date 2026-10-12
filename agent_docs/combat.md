# Mobs and combat

## Mobs

Map `life` entries of type `m` are mob spawn points (`x`, `cy` as the ground
height, `fh`, `f`, `mobTime`, `hide`); `game_data::mobs::MobData` reads their
templates from `Mob.nx` (`maxHP`, `exp`, `PADamage`, `bodyAttack`,
`notAttack`, ...). Each `FieldActor` owns a `field_mobs::MobSet`:

- **Spawning:** every visible spawn point (not `hide`, `mobTime` ≥ 0) spawns
  when the field starts. A killed mob comes back after `mobTime` seconds, at
  least 7 s, on the field's 100 ms tick, with a new object id (from
  2,000,000,000). Joining players get SPAWN_MONSTER for every live mob.
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
gained sends the new level, AP, max HP/MP (+14/+11 per beginner level, the
middle of v83's ranges), restores HP/MP, and shows SHOW_FOREIGN_EFFECT 0 to
the others.

## Getting hurt

TAKE_DAMAGE from a touch (-1) is accepted up to the touching mob's
`PADamage`, only for mobs with `bodyAttack` and without `notAttack`. HP
drops, others see DAMAGE_PLAYER, and at 0 HP the player is dead:
non-beginners lose 10% of their level's EXP; the client's respawn
(CHANGE_MAP with a target while dead) returns them to the map's return map.

## Keys

New characters get Ctrl → attack (53) and Z → pick up (51) in their default
keymap, as HeavenClient numbers its actions.

## Tests

`field_mobs` unit tests cover spawning, control hand-off, movement, damage,
EXP sharing, and respawn; `net::combat` covers parsing, the damage cap, and
the broadcast; `mob_combat` (integration) runs two players in Dangerous
Forest through control, relayed movement, hits, kills, EXP, refused damage,
touch damage, a level-up seen by the other player, and a respawn.
