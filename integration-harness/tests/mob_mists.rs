//! Mob mists: a Commander Skeleton in the Excavation Site's tombs
//! (7130103, Poison Mist 131 level 5: x 200, fifteen seconds, 70% to land)
//! lays a poison mist over its box. A v83 client standing in it hurts
//! itself up to x a second and says so with the skill; the server takes
//! the HP, refuses more than x, and clears the mist when its time is up.

use integration_harness::connection::MapleTestConnection;
use integration_harness::hunting::move_mob;
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read_for, send, stat, until, warp, Trace};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::{sleep, Duration};

const MIST_MAP: i32 = 101_030_108;
const CASTER: i32 = 7_130_103;
const POISON_MIST: u8 = 131;
const LEVEL: u8 = 5;
/// Its `x`: the most it hurts a second.
const X: i32 = 200;

/// TAKE_DAMAGE as a v83 client sends it from a mist (FUN_00431783): source
/// -4, then the skill and level in place of a mob.
fn mist_damage(damage: i32, skill: u8, level: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(-4i8 as u8).unwrap();
    p.write_byte(4).unwrap();
    p.write_int(damage).unwrap();
    p.write_short(i16::from(skill) << 8 | i16::from(level))
        .unwrap();
    p.write_byte(2).unwrap();
    p
}

/// Grant the mob its one skill, then cast it from its first skill
/// animation.
async fn cast(c: &mut MapleTestConnection, t: &mut Trace, mob: i32, at: (i16, i16)) {
    let granted = t.field.grants.len();
    send(c, move_mob(mob, at, true, 0, 0)).await;
    until(c, t, |t| t.field.grants.len() > granted).await;
    assert_eq!(t.field.grants.last(), Some(&(mob, POISON_MIST, LEVEL)));
    send(
        c,
        move_mob(
            mob,
            at,
            false,
            21 << 1,
            i32::from(POISON_MIST) | i32::from(LEVEL) << 8,
        ),
    )
    .await;
}

#[tokio::test]
async fn a_mobs_poison_mist_hurts_the_players_in_it_until_it_clears() {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();
    let c = &mut player.connection;

    let mut t = Trace::default();
    assert_eq!(warp(c, MIST_MAP, "", &mut t).await, MIST_MAP);
    let controls = |t: &Trace| {
        t.field
            .controlled
            .iter()
            .copied()
            .find(|oid| t.field.mobs.contains(&(*oid, CASTER)))
    };
    until(c, &mut t, |t| controls(t).is_some()).await;
    let mob = controls(&t).unwrap();
    let at = t.field.mob_positions[&mob];

    // It lands 70% of the time, and the mob waits its 15 second interval
    // between tries.
    for _ in 0..8 {
        cast(c, &mut t, mob, at).await;
        read_for(c, &mut t, 2_000).await;
        if !t.field.mists.is_empty() {
            break;
        }
        sleep(Duration::from_secs(15)).await;
    }
    let mist = t
        .field
        .mists
        .first()
        .copied()
        .expect("the mist never landed");
    // A mob's mist (type 0) by its object id, starting at once over its
    // box, poison for the client's resistances.
    assert_eq!(
        (
            mist.kind,
            mist.owner_id,
            mist.skill_id,
            mist.level,
            mist.delay
        ),
        (0, mob, i32::from(POISON_MIST), LEVEL, 0)
    );
    let (x, y) = (i32::from(at.0), i32::from(at.1));
    assert_eq!(mist.area, (x - 135, y - 80, x + 135, y + 40));

    // Standing in it, the client takes up to x.
    send(c, mist_damage(1, POISON_MIST, LEVEL)).await;
    until(c, &mut t, |t| t.stat(stat::HP).is_some()).await;
    let hp = t.stat(stat::HP).unwrap();
    send(c, mist_damage(2, POISON_MIST, LEVEL)).await;
    until(c, &mut t, |t| t.stat(stat::HP) == Some(hp - 2)).await;
    // More than x, or from a skill that leaves no mist, is refused.
    let updates = t.stat_updates.len();
    send(c, mist_damage(X + 1, POISON_MIST, LEVEL)).await;
    send(c, mist_damage(1, 125, 1)).await;
    barrier(c, "refused", &mut t).await;
    assert_eq!(t.stat_updates.len(), updates);

    // It clears after its fifteen seconds.
    sleep(Duration::from_secs(13)).await;
    until(c, &mut t, |t| {
        t.field.mists_removed.contains(&mist.object_id)
    })
    .await;
}
