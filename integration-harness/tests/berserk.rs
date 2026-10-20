//! A Dark Knight's Berserk: the client says when it turned on or off
//! (CALC_DAMAGE_STAT, empty), and the server shows the aura to the knight
//! and the map, and to those who see the knight arrive.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_skill, grant_stack, set_hp, set_job};
use integration_harness::hunting::use_item;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;

const BERSERK: i32 = 1_320_006;
const WHITE_POTION: i32 = 2_000_002;
const USE: i16 = 2;
const ELLINIA: i32 = 101_000_000;
const HENESYS: i32 = 100_000_000;

fn calc_damage_stat() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::CalcDamageStat as i16).unwrap();
    p
}

/// Read until the skill effect `op` shows Berserk, and return whether it
/// turned on.
async fn aura(c: &mut MapleTestConnection, t: &mut Trace, op: SendOpcode, prefix: &[u8]) -> bool {
    let mut effect = prefix.to_vec();
    effect.push(1);
    effect.extend(BERSERK.to_le_bytes());
    let op = op as i16;
    loop {
        let p = read(c, t).await;
        let body = &p.bytes[2..];
        if p.opcode() == op && body.starts_with(&effect) {
            assert_eq!(body[effect.len()..effect.len() + 2], [120, 30]);
            return body[effect.len() + 2] != 0;
        }
    }
}

/// Watch the knight `id` arrive on the watcher's map, and return whether
/// their aura shows.
async fn arrives(w: &mut MapleTestConnection, wt: &mut Trace, id: i32) -> bool {
    wt.players.clear();
    until(w, wt, |t| t.players.iter().any(|p| p.character_id == id)).await;
    wt.players
        .iter()
        .find(|p| p.character_id == id)
        .unwrap()
        .berserk
}

#[tokio::test]
async fn berserk_shows_its_aura_on_and_off() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("knight login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    set_job(id, 132, 120, 1000).await.expect("dark knight");
    grant_skill(id, BERSERK, 30).await.expect("Berserk");
    set_hp(id, 400, 1000).await.expect("hurt");
    grant_stack(id, USE, 1, WHITE_POTION, 1)
        .await
        .expect("a potion");
    let mut knight = relogin(&config).await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (k, mut kt) = (&mut knight.connection, Trace::default());
    let (w, mut wt) = (&mut watcher.connection, Trace::default());
    assert_eq!(warp(w, ELLINIA, "", &mut wt).await, ELLINIA);
    assert_eq!(warp(k, ELLINIA, "", &mut kt).await, ELLINIA);
    assert!(!arrives(w, &mut wt, id).await);

    // 400 of 1000 HP is under Berserk's 50%: on, for both.
    let foreign = id.to_le_bytes();
    send(k, calc_damage_stat()).await;
    assert!(aura(k, &mut kt, SendOpcode::ShowItemGainInChat, &[]).await);
    assert!(aura(w, &mut wt, SendOpcode::ShowForeignEffect, &foreign).await);

    // It comes back with the knight.
    assert_eq!(warp(k, HENESYS, "", &mut kt).await, HENESYS);
    assert_eq!(warp(k, ELLINIA, "", &mut kt).await, ELLINIA);
    assert!(arrives(w, &mut wt, id).await);

    // A White Potion's 300 HP lifts the knight over 50%: off.
    send(k, use_item(1, WHITE_POTION)).await;
    send(k, calc_damage_stat()).await;
    assert!(!aura(k, &mut kt, SendOpcode::ShowItemGainInChat, &[]).await);
    assert!(!aura(w, &mut wt, SendOpcode::ShowForeignEffect, &foreign).await);
}
