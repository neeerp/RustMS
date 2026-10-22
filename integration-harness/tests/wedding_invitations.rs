//! Wedding invitations (report b066): a partner of a booked couple sends an
//! invitation from the Etc tab to a guest by name (RING_ACTION 5). A guest
//! online with room takes a received invitation; one offline, or without
//! room, finds it at Duey with a note. The invitation goes only once the
//! guest is on the list. A guest opening theirs (RING_ACTION 6) sees the
//! couple's names (MARRIAGE_RESULT 15), or "not valid" (34) uninvited.
//! An invitation that fits nowhere, with the guest's Duey inbox full, goes
//! back to its sender and the guest off the list.

use integration_harness::database::{
    book_wedding, engage, fill_parcels, grant_stack, parcels_and_notes, set_gender, set_job,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, step, until, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const MOONSTONE_BOX: i32 = 2240000;
const CATHEDRAL: i16 = 1;
const CATHEDRAL_INVITATION: i32 = 4031395;
const RECEIVED_INVITATION: i32 = 4031407;
const ETC: i16 = 4;
const ETC_SLOTS: i16 = 24;
const SNAIL_SHELL: i32 = 4000000;
const PINK: u8 = 5;

const INVITATION: u8 = 15;
const NOT_VALID: u8 = 34;

fn invite(guest: &str, marriage_id: i32, slot: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::RingAction as i16).unwrap();
    p.write_byte(5).unwrap();
    p.write_str_with_length(guest).unwrap();
    p.write_int(marriage_id).unwrap();
    p.write_int(slot).unwrap();
    p
}

fn open(slot: i32, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::RingAction as i16).unwrap();
    p.write_byte(6).unwrap();
    p.write_int(slot).unwrap();
    p.write_int(item_id).unwrap();
    p
}

/// MARRIAGE_RESULT 15 for the Cathedral wedding of `groom` and `bride`.
fn invitation(groom: &str, bride: &str) -> (i16, Vec<u8>) {
    let mut body = vec![INVITATION];
    for name in [groom, bride] {
        body.extend((name.len() as u16).to_le_bytes());
        body.extend(name.as_bytes());
    }
    body.extend(2u16.to_le_bytes());
    (SendOpcode::MarriageResult as i16, body)
}

fn result(mode: u8) -> (i16, Vec<u8>) {
    (SendOpcode::MarriageResult as i16, vec![mode])
}

/// Make the character behind `config` of `gender` at level 50 and log it
/// out; its id and name.
async fn new_character(config: &HarnessConfig, gender: i16) -> (i32, String) {
    let session = login_to_world_session(config).await.expect("first login");
    let (id, name) = (session.character_id, session.character_name.clone());
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 50, 50).await.expect("level");
    set_gender(id, gender).await.expect("gender");
    (id, name)
}

/// Send an invitation to `guest`; what the sender saw, once told.
async fn send_invitation(session: &mut WorldSession, guest: &str, marriage_id: i32) -> Trace {
    let c = &mut session.connection;
    let mut t = Trace::default();
    integration_harness::trace::send(c, invite(guest, marriage_id, 1)).await;
    until(c, &mut t, |t| !t.broadcasts.is_empty()).await;
    read_for(c, &mut t, 300).await;
    t
}

#[tokio::test]
async fn a_partner_invites_guests_online_and_offline() {
    let mut configs = Vec::new();
    for _ in 0..6 {
        configs.push(load_harness_config_or_fail().await);
    }
    let (groom, _) = new_character(&configs[0], 0).await;
    let (bride, bride_name) = new_character(&configs[1], 1).await;
    let (online, online_name) = new_character(&configs[2], 0).await;
    let (away, away_name) = new_character(&configs[3], 1).await;
    let (full, full_name) = new_character(&configs[4], 0).await;
    let (stuffed, stuffed_name) = new_character(&configs[5], 1).await;
    engage(groom, bride, MOONSTONE_BOX).await.expect("engage");
    let marriage_id = book_wedding(groom, CATHEDRAL, 3).await.expect("book");
    grant_stack(groom, ETC, 1, CATHEDRAL_INVITATION, 15)
        .await
        .expect("invitations");
    // An invitation the guest holds before anyone invited them.
    grant_stack(online, ETC, 1, RECEIVED_INVITATION, 1)
        .await
        .expect("stray invitation");
    for slot in 1..=ETC_SLOTS {
        grant_stack(full, ETC, slot, SNAIL_SHELL, 1)
            .await
            .expect("shell");
    }
    fill_parcels(stuffed, 10).await.expect("parcels");
    let mut a = relogin(&configs[0]).await;
    let groom_name = a.character_name.clone();
    let mut c = relogin(&configs[2]).await;
    let mut e = relogin(&configs[4]).await;

    // Not invited yet: not valid.
    let t = step(
        &mut c.connection,
        "open uninvited",
        open(1, RECEIVED_INVITATION),
    )
    .await;
    assert_eq!(t.marriage, [result(NOT_VALID)]);

    // A guest online takes theirs; the sender's goes.
    let t = send_invitation(&mut a, &online_name, marriage_id).await;
    assert_eq!(t.quantities, [(ETC as u8, 1, 14)]);
    assert_eq!(t.broadcasts.len(), 1);
    let mut ct = Trace::default();
    until(&mut c.connection, &mut ct, |t| !t.items.is_empty()).await;
    read_for(&mut c.connection, &mut ct, 300).await;
    assert_eq!(ct.items, [(RECEIVED_INVITATION, 1)]);
    assert_eq!(ct.added, [(ETC as u8, 2, RECEIVED_INVITATION)]);
    assert_eq!(ct.broadcasts.len(), 1);
    assert_eq!(ct.broadcasts[0].0, PINK);
    assert!(
        ct.broadcasts[0].1.contains(&groom_name),
        "{:?}",
        ct.broadcasts
    );

    // Opening either now shows the couple.
    let t = step(&mut c.connection, "open", open(2, RECEIVED_INVITATION)).await;
    assert_eq!(t.marriage, [invitation(&groom_name, &bride_name)]);

    // Twice, or to the couple, or to no one: the invitation stays.
    let already = send_invitation(&mut a, &online_name.to_uppercase(), marriage_id).await;
    let couple = send_invitation(&mut a, &bride_name, marriage_id).await;
    let nobody = send_invitation(&mut a, "nobodyhere", marriage_id).await;
    let texts: Vec<_> = [&already, &couple, &nobody]
        .iter()
        .map(|t| {
            assert_eq!(t.quantities, []);
            assert_eq!(t.broadcasts.len(), 1);
            t.broadcasts[0].1.clone()
        })
        .collect();
    assert!(texts[0] != texts[1] && texts[1] != texts[2] && texts[0] != texts[2]);
    // The wrong couple: nothing at all.
    let t = step(
        &mut a.connection,
        "other couple",
        invite(&away_name, marriage_id + 1, 1),
    )
    .await;
    assert_eq!(t.quantities, []);
    assert_eq!(t.broadcasts, []);

    // A guest offline, and one without room, find it at Duey.
    let t = send_invitation(&mut a, &away_name, marriage_id).await;
    assert_eq!(t.quantities, [(ETC as u8, 1, 13)]);
    let t = send_invitation(&mut a, &full_name, marriage_id).await;
    assert_eq!(t.quantities, [(ETC as u8, 1, 12)]);
    let mut et = Trace::default();
    read_for(&mut e.connection, &mut et, 1000).await;
    assert_eq!(et.items, []);
    assert_eq!(et.broadcasts.len(), 1, "told it waits at Duey");
    for guest in [away, full] {
        let (parcels, notes) = parcels_and_notes(guest).await.expect("waiting");
        assert_eq!(parcels, [(groom_name.clone(), Some(RECEIVED_INVITATION))]);
        assert_eq!(notes, [groom_name.clone()]);
    }

    // Nowhere to put it: it comes back, and the guest may be sent another.
    for left in [11, 10] {
        let c = &mut a.connection;
        let mut t = Trace::default();
        integration_harness::trace::send(c, invite(&stuffed_name, marriage_id, 1)).await;
        until(c, &mut t, |t| t.broadcasts.len() >= 2).await;
        read_for(c, &mut t, 300).await;
        assert_eq!(t.quantities, [(ETC as u8, 1, left)]);
        assert!(
            t.added
                .iter()
                .any(|&(tab, _, item)| tab == ETC as u8 && item == CATHEDRAL_INVITATION),
            "{:?}",
            t.added
        );
        assert!(
            t.broadcasts[1].1.contains(&stuffed_name),
            "{:?}",
            t.broadcasts
        );
        assert_ne!(t.broadcasts[0].1, t.broadcasts[1].1);
    }
    let (parcels, notes) = parcels_and_notes(stuffed).await.expect("full inbox");
    assert_eq!(parcels.len(), 10);
    assert_eq!(notes, Vec::<String>::new());
}
