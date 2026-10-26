//! Parties: inviting, joining, party HP bars, a mob's HP bar from a
//! member's hit, sharing a kill's EXP on Snail Hunting Ground I (40000),
//! and its drops, party chat, and disbanding.

use integration_harness::hunting::{attack, pickup};
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const FOREST: i32 = 40_000;
const SNAIL: i32 = 100100;

const OP_INVITE: u8 = 0x04;
const OP_CREATED: u8 = 0x08;
const OP_LEAVE: u8 = 0x0C;
const OP_JOIN: u8 = 0x0F;

fn party_operation(op: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PartyOperation as i16).unwrap();
    p.write_byte(op).unwrap();
    p
}

fn invite(name: &str) -> Packet {
    let mut p = party_operation(4);
    p.write_str_with_length(name).unwrap();
    p
}

fn accept(party_id: i32) -> Packet {
    let mut p = party_operation(3);
    p.write_int(party_id).unwrap();
    p
}

fn party_chat(recipient: i32, text: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MultiChat as i16).unwrap();
    p.write_byte(1).unwrap();
    p.write_byte(1).unwrap();
    p.write_int(recipient).unwrap();
    p.write_str_with_length(text).unwrap();
    p
}

#[tokio::test]
async fn a_party_shares_hp_bars_exp_and_chat() {
    let leader_config = load_harness_config_or_fail().await;
    let member_config = load_harness_config_or_fail().await;
    let mut leader = login_to_world_session(&leader_config).await.unwrap();
    let mut member = login_to_world_session(&member_config).await.unwrap();
    let (mut l, mut m) = (Trace::default(), Trace::default());

    // Inviting without a party creates one; the invitee joins it.
    send(&mut leader.connection, invite(&member.character_name)).await;
    until(&mut member.connection, &mut m, |t| {
        !t.party_invites.is_empty()
    })
    .await;
    let (party_id, inviter) = m.party_invites[0].clone();
    assert_eq!(inviter, leader.character_name);
    send(&mut member.connection, accept(party_id)).await;
    until(&mut member.connection, &mut m, |t| {
        t.party_ops.contains(&OP_JOIN)
    })
    .await;
    until(&mut leader.connection, &mut l, |t| {
        t.party_ops.contains(&OP_JOIN)
    })
    .await;
    assert_eq!(l.party_ops[0], OP_CREATED);
    assert_eq!(m.party_ops, [OP_INVITE, OP_JOIN]);

    // On the same map, each sees the other's HP.
    assert_eq!(
        warp(&mut leader.connection, FOREST, "", &mut l).await,
        FOREST
    );
    assert_eq!(
        warp(&mut member.connection, FOREST, "", &mut m).await,
        FOREST
    );
    until(&mut member.connection, &mut m, |t| {
        t.party_hp.iter().any(|hp| hp.0 == leader.character_id)
    })
    .await;
    until(&mut leader.connection, &mut l, |t| {
        t.party_hp.iter().any(|hp| hp.0 == member.character_id) && !t.field.mobs.is_empty()
    })
    .await;

    // The leader kills a snail (3 EXP) alone; both level 1, so the leader
    // gets 40% plus 20% for the damage, the member 40%.
    read_for(&mut member.connection, &mut m, 300).await;
    let old_drops = m.field.drops.len();
    let snail = l.field.alive(SNAIL)[0];
    send(&mut leader.connection, attack(snail, 4)).await;
    send(&mut leader.connection, attack(snail, 4)).await;
    until(&mut leader.connection, &mut l, |t| !t.gains.is_empty()).await;
    until(&mut member.connection, &mut m, |t| !t.gains.is_empty()).await;
    assert_eq!((l.gains[0], m.gains[0]), (2, 1));
    // The member saw the snail's HP bar from the leader's first hit.
    assert!(
        m.field.hp_bars.iter().any(|&(oid, _)| oid == snail),
        "{:?}",
        m.field.hp_bars
    );

    // Its drops (in the harness, at least a card) are the party's for now:
    // the member may take one at once.
    until(&mut member.connection, &mut m, |t| {
        t.field.drops.len() > old_drops
    })
    .await;
    let loot = m.field.drops[old_drops];
    assert_eq!((loot.owner, loot.ownership), (party_id, 1));
    send(
        &mut member.connection,
        pickup(loot.object_id, loot.x, loot.y),
    )
    .await;
    until(&mut member.connection, &mut m, |t| {
        !t.field.removals(loot.object_id).is_empty()
    })
    .await;
    assert_eq!(
        m.field.removals(loot.object_id),
        [Some(member.character_id)]
    );

    // Party chat reaches the others, not the speaker.
    send(
        &mut member.connection,
        party_chat(leader.character_id, "nice one"),
    )
    .await;
    until(&mut leader.connection, &mut l, |t| !t.party_chat.is_empty()).await;
    assert_eq!(
        l.party_chat,
        [(member.character_name.clone(), "nice one".to_string())]
    );
    read_for(&mut member.connection, &mut m, 300).await;
    assert!(m.party_chat.is_empty());

    // The leader leaving disbands the party for everyone.
    send(&mut leader.connection, party_operation(2)).await;
    until(&mut member.connection, &mut m, |t| {
        t.party_ops.contains(&OP_LEAVE)
    })
    .await;
    until(&mut leader.connection, &mut l, |t| {
        t.party_ops.contains(&OP_LEAVE)
    })
    .await;
}
