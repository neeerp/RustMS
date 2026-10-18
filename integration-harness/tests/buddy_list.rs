//! Buddy lists: requests, accepting, channels of mutual buddies, buddy
//! chat, logging out and back in, and removing a buddy.

use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const NOT_FOUND: u8 = 0x0F;
const ALREADY_ADDED: u8 = 0x0D;

fn modify(mode: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::BuddyListModify as i16).unwrap();
    p.write_byte(mode).unwrap();
    p
}

fn add(name: &str) -> Packet {
    let mut p = modify(1);
    p.write_str_with_length(name).unwrap();
    p.write_str_with_length("Default Group").unwrap();
    p
}

fn with_id(mode: u8, id: i32) -> Packet {
    let mut p = modify(mode);
    p.write_int(id).unwrap();
    p
}

fn buddy_chat(recipient: i32, text: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MultiChat as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(1).unwrap();
    p.write_int(recipient).unwrap();
    p.write_str_with_length(text).unwrap();
    p
}

#[tokio::test]
async fn buddies_see_each_others_channel_once_both_agree() {
    let alice_config = load_harness_config_or_fail().await;
    let bobby_config = load_harness_config_or_fail().await;
    let mut alice = login_to_world_session(&alice_config).await.unwrap();
    let mut bobby = login_to_world_session(&bobby_config).await.unwrap();
    let (mut a, mut b) = (Trace::default(), Trace::default());
    let (alice_id, bobby_id) = (alice.character_id, bobby.character_id);
    let bobby_name = bobby.character_name.clone();

    // Unknown names and repeats are refused.
    send(&mut alice.connection, add("NoSuchName")).await;
    until(&mut alice.connection, &mut a, |t| {
        !t.buddy_errors.is_empty()
    })
    .await;
    assert_eq!(a.buddy_errors, [NOT_FOUND]);

    // Adding asks Bobby; until he agrees Alice can't see his channel.
    send(&mut alice.connection, add(&bobby_name.to_uppercase())).await;
    until(&mut alice.connection, &mut a, |t| {
        t.buddy_lists.last().is_some_and(|l| !l.is_empty())
    })
    .await;
    assert_eq!(
        a.buddy_lists.last().unwrap(),
        &[(bobby_id, bobby_name.clone(), -1)]
    );
    until(&mut bobby.connection, &mut b, |t| {
        !t.buddy_requests.is_empty()
    })
    .await;
    assert_eq!(b.buddy_requests, [(alice_id, alice.character_name.clone())]);
    send(&mut alice.connection, add(&bobby_name)).await;
    until(&mut alice.connection, &mut a, |t| t.buddy_errors.len() == 2).await;
    assert_eq!(a.buddy_errors[1], ALREADY_ADDED);

    // Logging in again carries the request in the list, for the client to
    // ask about once the map has loaded, rather than asking again.
    drop(bobby);
    let mut bobby = relogin(&bobby_config).await;
    let mut b = Trace::default();
    until(&mut bobby.connection, &mut b, |t| !t.buddy_lists.is_empty()).await;
    assert!(b.buddy_lists[0].is_empty());
    assert_eq!(b.buddy_list_requests[0], [alice_id]);
    read_for(&mut bobby.connection, &mut b, 300).await;
    assert!(b.buddy_requests.is_empty());

    // Accepting shows both of them the other's channel.
    send(&mut bobby.connection, with_id(2, alice_id)).await;
    until(&mut bobby.connection, &mut b, |t| {
        t.buddy_lists.last().is_some_and(|l| !l.is_empty())
    })
    .await;
    assert_eq!(
        b.buddy_lists.last().unwrap(),
        &[(alice_id, alice.character_name.clone(), 0)]
    );
    until(&mut alice.connection, &mut a, |t| {
        t.buddy_channels.contains(&(bobby_id, 0))
    })
    .await;

    // Buddy chat reaches listed buddies.
    send(&mut alice.connection, buddy_chat(bobby_id, "hey bobby")).await;
    until(&mut bobby.connection, &mut b, |t| !t.buddy_chat.is_empty()).await;
    assert_eq!(
        b.buddy_chat,
        [(alice.character_name.clone(), "hey bobby".to_string())]
    );

    // Logging out and back in shows up on the other's list.
    drop(bobby);
    until(&mut alice.connection, &mut a, |t| {
        t.buddy_channels.contains(&(bobby_id, -1))
    })
    .await;
    let mut bobby = relogin(&bobby_config).await;
    let mut b = Trace::default();
    until(&mut bobby.connection, &mut b, |t| !t.buddy_lists.is_empty()).await;
    assert_eq!(b.buddy_lists[0][0].2, 0, "Alice is online on channel 0");
    assert!(b.buddy_requests.is_empty());
    until(&mut alice.connection, &mut a, |t| {
        t.buddy_channels.ends_with(&[(bobby_id, 0)])
    })
    .await;

    // Removing Bobby hides Alice from him and empties her list.
    send(&mut alice.connection, with_id(3, bobby_id)).await;
    until(&mut alice.connection, &mut a, |t| {
        t.buddy_lists.last().is_some_and(|l| l.is_empty())
    })
    .await;
    until(&mut bobby.connection, &mut b, |t| {
        t.buddy_channels.contains(&(alice_id, -1))
    })
    .await;
    read_for(&mut bobby.connection, &mut b, 300).await;
    assert!(b.buddy_requests.is_empty());
}
