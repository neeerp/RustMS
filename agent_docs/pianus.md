# Pianus

The two Pianus of Aqua Road's Cave of Pianus (230040420). The survey is
v83ref's `reports/a114-pianus.md`; the defaults shipped are under "a114" in
`lanes/QUESTIONS.md`.

## The cave

Pianus is no event: a player of any level, alone or not, takes the scripted
portal `boss00` (script `Pianus`, `PLAIN_WARPS`) from The Dangerous Cave
(230040410) into the cave, and leaves by `out00`. The map has no script, no
clock and no NPC; its `forcedReturn` is Aqua Road (230000000).

Its two spawn points are the fight:

| Mob | Where | HP | `mobTime` | Bar colours |
| --- | --- | --- | --- | --- |
| Pianus 8510000 | x 568, the right end | 30,000,000 | 86,400 s | 1, 5 |
| Pianus 8520000 | x -459, the left end | 24,000,000 | 129,600 s | 2, 5 |

They come and come back as any spawn point with a `mobTime` does
([combat](./combat.md)): the first a while after the field loads, then
1.3 to 2 times `mobTime` after a kill. The harness fills every spawn point
at once (`RUSTMS_FULL_SPAWN`). 8520000 also leaves on its own after 36
hours (`removeAfter`).

## Never turning

Both are `noFlip` (`MobInfo::no_flip`): the client never draws them
flipped, and the server treats them as facing left whatever stance their
controller reports (`LiveMob::facing_left`). Their skill areas are written
that way: the right one's seal, dispel, heal and Booms reach left, over
the cave, and the left one's reach right. Mirrored, they would miss the
cave.

## Skills

Heal (114), seal (120), dispel (127), the two immunities (140, 141), and
Bloody Booms (200 levels 41-44): five 8510100 at random points in the
level's area, on the ground below (`summon_spot`). A Boom has 10,000 HP and
blows itself up at half (`selfDestruction`); it drops nothing. All of it is
the generic mob skill machinery ([combat](./combat.md)).

## Drops

The content pack's `rewards.json` has both. Their drops are anyone's at
once (`publicReward`). Each drops the Pianus Certificate (4001085), one
of the Dragon Squad badges for quest 7302, every time; Pianus's Scream
(4031253) is a quest item, dropped only for those on quest 3076.

## Tests

`game-data` reads `noFlip` (`mobs.rs`); `field_mobs.rs` checks a `noFlip`
caster's area. The harness's `tests/pianus_boss.rs` finds both Pianus with
their bars, has the left one, turned right by its controller, seal the
player in front of him and call up his Booms there, and kills the right
one for his certificate and potions.
