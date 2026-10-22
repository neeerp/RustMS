//! Party potions, the Carnival and Dojo items with `party` and
//! `consumeOnPickup` (2022160-63, 2022430-33): picking one up works on
//! the picker and on every living party member on the map.

use integration_harness::database::{grant_stack, set_hp};
use integration_harness::hunting::pickup;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

/// A town no other test uses.
const FIELD: i32 = 104_000_000;
const USE: i16 = 2;
const SLOT: i16 = 5;
/// Party Mana Elixir: 300 MP.
const PARTY_ELIXIR: i32 = 2_022_160;
const OP_JOIN: u8 = 0x0F;

fn item_move(kind: i8, from: i16, to: i16, quantity: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(kind as u8).unwrap();
    p.write_short(from).unwrap();
    p.write_short(to).unwrap();
    p.write_short(quantity).unwrap();
    p
}

fn party_operation(op: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PartyOperation as i16).unwrap();
    p.write_byte(op).unwrap();
    p
}

#[tokio::test]
async fn a_party_potion_works_on_the_living_members_on_the_map() {
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let mut ids = Vec::new();
    for config in &configs {
        let id = login_to_world_session(config)
            .await
            .expect("first login failed")
            .character_id;
        drop(relogin(config).await);
        ids.push(id);
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(ids[0], USE, SLOT, PARTY_ELIXIR, 1)
        .await
        .expect("grant the potion");
    for config in &configs {
        let mut session = relogin(config).await;
        let mut t = Trace::default();
        assert_eq!(
            warp(&mut session.connection, FIELD, "", &mut t).await,
            FIELD
        );
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_hp(ids[2], 0, 100).await.expect("kill the third");

    let mut picker = relogin(&configs[0]).await;
    let mut member = relogin(&configs[1]).await;
    let mut dead = relogin(&configs[2]).await;
    let (p, m, d) = (
        &mut picker.connection,
        &mut member.connection,
        &mut dead.connection,
    );
    let (mut pt, mut mt, mut dt) = (Trace::default(), Trace::default(), Trace::default());
    read_for(p, &mut pt, 300).await;
    for (c, t, name) in [
        (&mut *m, &mut mt, &member.character_name),
        (&mut *d, &mut dt, &dead.character_name),
    ] {
        let mut invite = party_operation(4);
        invite.write_str_with_length(name).unwrap();
        send(p, invite).await;
        until(c, t, |t| !t.party_invites.is_empty()).await;
        let mut accept = party_operation(3);
        accept.write_int(t.party_invites[0].0).unwrap();
        send(c, accept).await;
        pt.party_ops.clear();
        until(p, &mut pt, |t| t.party_ops.contains(&OP_JOIN)).await;
    }
    read_for(m, &mut mt, 300).await;
    read_for(d, &mut dt, 300).await;
    for t in [&mut pt, &mut mt, &mut dt] {
        t.stat_updates.clear();
    }

    // Drop it from the bag, then pick it back up.
    send(p, item_move(USE as i8, SLOT, 0, 1)).await;
    until(p, &mut pt, |t| t.field.item_drop(PARTY_ELIXIR).is_some()).await;
    let loot = pt.field.item_drop(PARTY_ELIXIR).unwrap();
    let added = pt.added.len();
    send(p, pickup(loot.object_id, loot.x, loot.y)).await;
    until(p, &mut pt, |t| !t.stat_values(stat::MP).is_empty()).await;
    assert!(pt.added[added..]
        .iter()
        .all(|&(_, _, item)| item != PARTY_ELIXIR));

    // The living member gets its MP too; the dead one gets nothing.
    until(m, &mut mt, |t| !t.stat_values(stat::MP).is_empty()).await;
    read_for(d, &mut dt, 500).await;
    assert!(
        dt.stat_values(stat::MP).is_empty(),
        "a dead member drank a party potion"
    );
}
