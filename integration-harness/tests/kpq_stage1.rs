//! Kerning City's party quest, stage 1: Cloto asks each member but the
//! leader for a number of coupons, gives a pass for exactly that many, and
//! the leader clears the stage by handing in a pass from each of them,
//! which opens the way on to stage 2 and gives everyone its EXP. The
//! harness world pins her question (`RUSTMS_KPQ_QUESTION`), so the
//! members' coupons can be granted before they log in.

use game_data::GameData;
use integration_harness::database::{grant_stack, set_job};
use integration_harness::hunting::pickup;
use integration_harness::packets::build_change_map;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{ANSWERS, COUPON, KPQ, PASS};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const LAKELIS: i32 = 9020000;
const CLOTO: i32 = 9020001;
const OP_JOIN: u8 = 0x0F;
const ETC: u8 = 4;
/// The question the harness world pins, 1 to 7.
const QUESTION: usize = 7;

fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

fn talk(map: i32, npc: i32) -> Packet {
    let index = maps()
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

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

/// Throw `quantity` of the Etc stack in `slot` on the ground.
fn throw_down(slot: i16, quantity: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(ETC).unwrap();
    p.write_short(slot).unwrap();
    p.write_short(0).unwrap();
    p.write_short(quantity).unwrap();
    p
}

/// A new level-25 character holding `quantity` of `item_id` in Etc slot 1,
/// logged in and waiting in Kerning City.
async fn player(item_id: i32, quantity: i16) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 25, 50).await.unwrap();
    grant_stack(id, ETC as i16, 1, item_id, quantity)
        .await
        .unwrap();
    let mut session = relogin(&config).await;
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut session.connection, KPQ.lobby, "", &mut trace).await,
        KPQ.lobby
    );
    session
}

/// The first player invites the rest into a party.
async fn party(players: &mut [WorldSession]) {
    let (leader, members) = players.split_first_mut().unwrap();
    for member in members {
        send(&mut leader.connection, invite(&member.character_name)).await;
        let mut trace = Trace::default();
        until(&mut member.connection, &mut trace, |t| {
            !t.party_invites.is_empty()
        })
        .await;
        let party_id = trace.party_invites[0].0;
        send(&mut member.connection, accept(party_id)).await;
        until(&mut member.connection, &mut trace, |t| {
            t.party_ops.contains(&OP_JOIN)
        })
        .await;
    }
}

/// What `player` is shown on talking to Cloto: one text page, and the
/// trace that came with it.
async fn ask_cloto(player: &mut WorldSession) -> (String, Trace) {
    let name = player.character_name.clone();
    let trace = step(&mut player.connection, &name, talk(KPQ.entry, CLOTO)).await;
    assert_eq!(trace.prompts, [(CLOTO, 0)], "{trace:?}");
    (trace.prompt_texts[0].clone(), trace)
}

/// What a member on the stage's map is shown on its clear: the clear on
/// screen, its sound, and the gate opening.
fn clear_effects() -> Vec<(u8, String)> {
    vec![
        (3, "quest/party/clear".into()),
        (4, "Party1/Clear".into()),
        (2, "gate".into()),
    ]
}

fn nothing_changed_hands(trace: &Trace) {
    assert!(
        trace.added.is_empty() && trace.removed.is_empty() && trace.quantities.is_empty(),
        "{trace:?}"
    );
}

#[tokio::test]
async fn cloto_gives_passes_for_coupons_and_the_leader_clears_stage_one() {
    let answer_count = ANSWERS[QUESTION - 1] as i16;
    let mut team = vec![
        player(PASS, 1).await,
        player(COUPON, answer_count).await,
        player(COUPON, answer_count - 1).await,
    ];
    party(&mut team).await;

    // In they go, once the party before them is out (tests/kpq.rs).
    let name = team[0].character_name.clone();
    for _ in 0..5 {
        let trace = step(&mut team[0].connection, &name, talk(KPQ.lobby, LAKELIS)).await;
        if trace.prompts == [(LAKELIS, 1)] {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    let mut seen = Some(step(&mut team[0].connection, &name, answer(1, 1, None)).await);
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&KPQ.entry)
        })
        .await;
    }
    let [leader, right, wrong] = &mut team[..] else {
        unreachable!()
    };

    // The way on is shut until the stage is cleared, for everyone.
    for player in [&mut *leader, &mut *wrong] {
        send(
            &mut player.connection,
            build_change_map(-1, "next00").unwrap(),
        )
        .await;
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            !t.blocked.is_empty()
        })
        .await;
        assert_eq!(trace.blocked, [(SendOpcode::BlockedMap as i16, 1)]);
        assert!(trace.warps.is_empty(), "{trace:?}");
    }

    // Each member is asked first, whatever they hold.
    let (question, trace) = ask_cloto(right).await;
    nothing_changed_hands(&trace);
    assert_eq!(ask_cloto(wrong).await.0, question);

    // One coupon short is wrong, and costs nothing; she asks again.
    let (told, trace) = ask_cloto(wrong).await;
    nothing_changed_hands(&trace);
    assert!(told != question && told.contains(&question), "{told}");

    // The right number earns a pass, once.
    let (_, trace) = ask_cloto(right).await;
    assert_eq!(trace.removed, [(ETC, 1)]);
    let pass_slot = match trace.added.as_slice() {
        [(ETC, slot, PASS)] => *slot,
        other => panic!("{other:?}"),
    };
    nothing_changed_hands(&ask_cloto(right).await.1);

    // The leader needs a pass from each of the others.
    nothing_changed_hands(&ask_cloto(leader).await.1);

    // Passes change hands: dropped, and picked up.
    send(&mut right.connection, throw_down(pass_slot, 1)).await;
    let mut trace = Trace::default();
    until(&mut leader.connection, &mut trace, |t| {
        t.field.item_drop(PASS).is_some()
    })
    .await;
    let pass = trace.field.item_drop(PASS).unwrap();
    send(
        &mut leader.connection,
        pickup(pass.object_id, pass.x, pass.y),
    )
    .await;
    until(&mut leader.connection, &mut trace, |t| {
        t.quantities.contains(&(ETC, 1, 2))
    })
    .await;

    // Both passes clear the stage: everyone on it sees and hears it, the
    // gate opens, and each gets the stage's EXP.
    let (cleared, mut trace) = ask_cloto(leader).await;
    assert_eq!(trace.removed, [(ETC, 1)]);
    until(&mut leader.connection, &mut trace, |t| !t.gains.is_empty()).await;
    assert_eq!(trace.field_effects, clear_effects());
    assert_eq!(trace.gains, [KPQ.stage_exp(1)]);
    for member in [&mut *right, &mut *wrong] {
        let mut trace = Trace::default();
        until(&mut member.connection, &mut trace, |t| !t.gains.is_empty()).await;
        assert_eq!(trace.field_effects, clear_effects());
        assert_eq!(trace.gains, [KPQ.stage_exp(1)]);
    }

    // Cloto says so to everyone.
    let (told, trace) = ask_cloto(wrong).await;
    nothing_changed_hands(&trace);
    assert!(!told.contains(&question), "{told}");
    assert_ne!(told, cleared);

    // Through it the leader goes on to stage 2, and back on stage 1 the
    // gate is still open.
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut leader.connection, -1, "next00", &mut trace).await,
        KPQ.stage_map(2)
    );
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut leader.connection, KPQ.entry, "", &mut trace).await,
        KPQ.entry
    );
    until(&mut leader.connection, &mut trace, |t| {
        !t.field_effects.is_empty()
    })
    .await;
    assert_eq!(trace.field_effects, [(2, "gate".to_string())]);
}
