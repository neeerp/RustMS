//! The Ariant Coliseum's Battle Arena. In its lobby Cesar (2101014) lets a
//! player of levels 20 to 30 open one of three arenas for the number they
//! type, or join one still gathering, and sends them to its entrance.
//! There Cesar (2101017) lets the host see who is in it, ban one, or take
//! them all into its battle map, where he gives each player bombs and
//! rocks once. Once fewer than two are left in a battle, it ends and the
//! rest go back to the lobby, whose way out leads back by the mirror.
//! Otherwise its players see each other's Spirit Jewels as they change, and
//! at its results earn a Battle Arena point for every ten (one only for a
//! winner far ahead), then go to the King's Room, where Areda (2101016)
//! gives them with EXP for the jewels, and Abdullah VIII (2101015) trades a
//! hundred points for a chair.

use game_data::server_content::keys;
use integration_harness::database::{grant_stack, set_ariant_points, set_job};
use integration_harness::events::{armed_in, armed_with, said, talk, told};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::protocol::WorldSession;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::ariant::{
    battle, entrance, ARIANT_FALLBACK, ARIANT_LOBBY, BOMB, CHAIR, ELEMENT_ROCK, KINGS_ROOM,
    SPIRIT_JEWEL,
};
use net::npc_scripts::{ARIANT_AREDA, ARIANT_CESAR, ARIANT_HOST, ARIANT_KING};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const HENESYS: i32 = 100_000_000;
const SAY: u8 = 0;
const ASK: u8 = 1;
const TEXT: u8 = 2;
const MENU: u8 = 4;
const START: i32 = 2;
const BAN: i32 = 1;
const ETC: u8 = 4;
const EXP: i32 = 0x10000;

/// NPC_TALK_MORE: answer a text prompt with `typed`.
fn typed(typed: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::NpcTalkMore as i16).unwrap();
    p.write_byte(TEXT).unwrap();
    p.write_byte(1).unwrap();
    p.write_str_with_length(typed).unwrap();
    p
}

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

fn lobby_cesar() -> Packet {
    talk(ARIANT_LOBBY, ARIANT_CESAR)
}

fn host_cesar(arena: u8) -> Packet {
    talk(entrance(arena), ARIANT_HOST)
}

/// Where arena `arena` is on the lobby's menu in `t`, if offered.
fn offered(t: &Trace, arena: u8) -> Option<i32> {
    let mark = format!("Arena ({arena})");
    let menu = t.prompt_texts.last()?;
    menu.split("#L").skip(1).find_map(|choice| {
        let (index, label) = choice.split_once('#')?;
        label.contains(&mark).then(|| index.parse().ok())?
    })
}

/// `player` picks arena `arena` from the lobby's Cesar.
async fn pick(player: &mut WorldSession, arena: u8) -> Trace {
    let t = told(player, lobby_cesar()).await;
    assert_eq!(t.prompts, [(ARIANT_CESAR, MENU)], "{t:?}");
    let index = offered(&t, arena).unwrap_or_else(|| panic!("{t:?}"));
    told(player, answer(MENU, 1, Some(index))).await
}

/// `player` asks the lobby's Cesar for arena `arena` and types `cap`.
async fn open(player: &mut WorldSession, arena: u8, cap: &str) -> Trace {
    let t = pick(player, arena).await;
    assert_eq!(t.prompts, [(ARIANT_CESAR, TEXT)], "{t:?}");
    assert!(said(&t, keys::ARIANT_ASK_CAP), "{t:?}");
    told(player, typed(cap)).await
}

/// `player` joins arena `arena` from the lobby and lands at its entrance.
async fn join(player: &mut WorldSession, arena: u8) {
    let mut t = pick(player, arena).await;
    until(&mut player.connection, &mut t, |t| {
        t.warps.contains(&entrance(arena))
    })
    .await;
}

/// The host picks `choice` from Cesar's menu at arena `arena`'s entrance.
async fn host_picks(host: &mut WorldSession, arena: u8, choice: i32) -> Trace {
    let t = told(host, host_cesar(arena)).await;
    assert_eq!(t.prompts, [(ARIANT_HOST, MENU)], "{t:?}");
    told(host, answer(MENU, 1, Some(choice))).await
}

/// Close the NPC's last line.
async fn close(player: &mut WorldSession) {
    told(player, answer(SAY, 1, None)).await;
}

#[tokio::test]
async fn cesar_opens_an_arena_its_host_takes_into_battle_and_it_ends_below_two() {
    let mut host = armed_in(ARIANT_LOBBY, 25).await;
    let mut member = armed_in(ARIANT_LOBBY, 25).await;
    let mut banned = armed_in(ARIANT_LOBBY, 25).await;
    let mut high = armed_in(ARIANT_LOBBY, 31).await;

    // Over level 30 is too strong for the arenas.
    let t = told(&mut high, lobby_cesar()).await;
    assert!(said(&t, keys::ARIANT_TOO_HIGH), "{t:?}");
    close(&mut high).await;

    // The number typed must be one, and at least two.
    let t = open(&mut host, 1, "two").await;
    assert!(said(&t, keys::ARIANT_NOT_A_NUMBER), "{t:?}");
    close(&mut host).await;
    let t = open(&mut host, 1, "1").await;
    assert!(said(&t, keys::ARIANT_TOO_FEW), "{t:?}");
    close(&mut host).await;

    // Three opens the arena, the host at its entrance with a clock.
    let mut t = open(&mut host, 1, "3").await;
    let created = net::content::text().get(keys::ARIANT_CREATED, &[]);
    until(&mut host.connection, &mut t, |t| {
        t.warps.contains(&entrance(1))
            && !t.countdowns.is_empty()
            && t.broadcasts.iter().any(|(_, line)| *line == created)
    })
    .await;

    // Alone, the host may not start it.
    let t = host_picks(&mut host, 1, START).await;
    let count = net::content::text().get(keys::ARIANT_COUNT, &[("least", "2"), ("most", "3")]);
    assert!(
        t.prompt_texts.iter().any(|said| said.contains(&count)),
        "{t:?}"
    );
    close(&mut host).await;

    // Two join; one is banned, goes back and may not come again.
    join(&mut member, 1).await;
    join(&mut banned, 1).await;
    let t = host_picks(&mut host, 1, BAN).await;
    assert_eq!(t.prompts, [(ARIANT_HOST, MENU)], "{t:?}");
    let t = told(&mut host, answer(MENU, 1, Some(1))).await;
    let name = banned.character_name.clone();
    let line = net::content::text().get(keys::ARIANT_BANNED_NAME, &[("name", &name)]);
    assert!(
        t.prompt_texts.iter().any(|said| said.contains(&line)),
        "{t:?}"
    );
    close(&mut host).await;
    let mut bt = Trace::default();
    until(&mut banned.connection, &mut bt, |t| {
        t.warps.contains(&ARIANT_LOBBY)
    })
    .await;
    let t = pick(&mut banned, 1).await;
    assert!(said(&t, keys::ARIANT_BANNED), "{t:?}");
    close(&mut banned).await;

    // A member sees who is in it.
    let t = told(&mut member, host_cesar(1)).await;
    let names = format!("{}, {}", host.character_name, member.character_name);
    let line = net::content::text().get(keys::ARIANT_MEMBERS, &[("names", &names)]);
    assert!(
        t.prompt_texts.iter().any(|said| said.contains(&line)),
        "{t:?}"
    );
    close(&mut member).await;

    // The host takes both into the battle.
    let mut ht = host_picks(&mut host, 1, START).await;
    until(&mut host.connection, &mut ht, |t| {
        t.warps.contains(&battle(1))
    })
    .await;
    let mut mt = Trace::default();
    until(&mut member.connection, &mut mt, |t| {
        t.warps.contains(&battle(1))
    })
    .await;

    // A battling arena isn't offered in the lobby. The other tests here
    // hold arenas 2 and 3.
    let t = told(&mut banned, lobby_cesar()).await;
    assert_eq!(t.prompts, [(ARIANT_CESAR, MENU)], "{t:?}");
    assert!(offered(&t, 1).is_none(), "{t:?}");
    told(&mut banned, answer(MENU, 0, None)).await;

    // Cesar gives the bombs and rocks once.
    let t = told(&mut member, talk(battle(1), ARIANT_HOST)).await;
    assert!(said(&t, keys::ARIANT_KIT_GIVEN), "{t:?}");
    let given: Vec<_> = t.added.iter().map(|&(_, _, item)| item).collect();
    assert!(
        given.contains(&ELEMENT_ROCK) && given.contains(&BOMB),
        "{t:?}"
    );
    close(&mut member).await;
    let t = told(&mut member, talk(battle(1), ARIANT_HOST)).await;
    assert!(said(&t, keys::ARIANT_KIT_TAKEN), "{t:?}");
    close(&mut member).await;

    // One leaves, and the other goes back to the lobby.
    let mut mt = Trace::default();
    warp(&mut member.connection, HENESYS, "", &mut mt).await;
    let mut ht = Trace::default();
    until(&mut host.connection, &mut ht, |t| {
        t.warps.contains(&ARIANT_LOBBY)
    })
    .await;

    // The lobby's way out leads back by the mirror, or to Ariant.
    let maps = integration_harness::events::maps();
    let out = maps
        .field(ARIANT_LOBBY)
        .unwrap()
        .portal_by_name("out00")
        .unwrap();
    send(&mut host.connection, walk_to(out.x as i16, out.y as i16)).await;
    let mut ht = Trace::default();
    send(&mut host.connection, enter("out00")).await;
    until(&mut host.connection, &mut ht, |t| {
        t.warps.contains(&ARIANT_FALLBACK)
    })
    .await;
    read_for(&mut host.connection, &mut ht, 100).await;
}

#[tokio::test]
async fn the_host_leaving_sends_the_arena_back_to_the_lobby() {
    let mut host = armed_in(ARIANT_LOBBY, 22).await;
    let mut member = armed_in(ARIANT_LOBBY, 22).await;
    let mut t = open(&mut host, 2, "2").await;
    until(&mut host.connection, &mut t, |t| {
        t.warps.contains(&entrance(2))
    })
    .await;
    join(&mut member, 2).await;

    // An arena at its cap is full.
    let mut late = armed_in(ARIANT_LOBBY, 22).await;
    let t = pick(&mut late, 2).await;
    assert!(said(&t, keys::ARIANT_FULL), "{t:?}");
    close(&mut late).await;

    // The host leaves, and both go back, told so.
    told(&mut host, host_cesar(2)).await;
    let mut ht = told(&mut host, answer(MENU, 1, Some(3))).await;
    until(&mut host.connection, &mut ht, |t| {
        t.warps.contains(&ARIANT_LOBBY)
    })
    .await;
    let left = net::content::text().get(keys::ARIANT_HOST_LEFT, &[]);
    let mut mt = Trace::default();
    until(&mut member.connection, &mut mt, |t| {
        t.warps.contains(&ARIANT_LOBBY) && t.broadcasts.iter().any(|(_, line)| *line == left)
    })
    .await;

    // With no roster at the entrance, Cesar offers the way back.
    let mut lt = Trace::default();
    warp(&mut late.connection, entrance(2), "", &mut lt).await;
    let t = told(&mut late, host_cesar(2)).await;
    assert_eq!(t.prompts, [(ARIANT_HOST, ASK)], "{t:?}");
    assert!(said(&t, keys::ARIANT_NO_ROSTER), "{t:?}");
    let mut lt = told(&mut late, answer(ASK, 1, None)).await;
    until(&mut late.connection, &mut lt, |t| {
        t.warps.contains(&ARIANT_LOBBY)
    })
    .await;
}

/// Whether `t` saw a board giving `name` `jewels`.
fn scored(t: &Trace, name: &str, jewels: i32) -> bool {
    (t.ariant_scores.last()).is_some_and(|board| board.contains(&(name.to_owned(), jewels)))
}

#[tokio::test]
async fn a_battles_jewels_earn_points_that_areda_gives_and_abdullah_trades_for_a_chair() {
    // The host has 98 points from battles before.
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 25, 50).await.unwrap();
    grant_stack(id, ETC.into(), 1, SPIRIT_JEWEL, 30)
        .await
        .unwrap();
    set_ariant_points(id, 98).await.unwrap();
    let mut host = relogin(&config).await;
    let mut ht = Trace::default();
    warp(&mut host.connection, ARIANT_LOBBY, "", &mut ht).await;
    let mut member = armed_with(ARIANT_LOBBY, 25, &[(SPIRIT_JEWEL, 25)]).await;
    let mut t = open(&mut host, 3, "2").await;
    until(&mut host.connection, &mut t, |t| {
        t.warps.contains(&entrance(3))
    })
    .await;
    join(&mut member, 3).await;
    let mut ht = host_picks(&mut host, 3, START).await;
    let mut mt = Trace::default();
    until(&mut member.connection, &mut mt, |t| {
        t.warps.contains(&battle(3))
    })
    .await;

    // Both see the board.
    let (hn, mn) = (host.character_name.clone(), member.character_name.clone());
    until(&mut host.connection, &mut ht, |t| {
        scored(t, &hn, 30) && scored(t, &mn, 25)
    })
    .await;
    until(&mut member.connection, &mut mt, |t| {
        scored(t, &hn, 30) && scored(t, &mn, 25)
    })
    .await;

    // Away from the battle, Areda and Abdullah have nothing for them.
    let mut far = armed_in(ARIANT_LOBBY, 25).await;
    let mut ft = Trace::default();
    warp(&mut far.connection, KINGS_ROOM, "", &mut ft).await;
    let t = told(&mut far, talk(KINGS_ROOM, ARIANT_AREDA)).await;
    assert!(said(&t, keys::ARIANT_ABSENT), "{t:?}");
    close(&mut far).await;

    // At the results, 30 and 25 jewels are 3 and 2 points, then both go to
    // the King's Room.
    until(&mut host.connection, &mut ht, |t| {
        t.ariant_results == 1 && t.warps.contains(&KINGS_ROOM)
    })
    .await;
    until(&mut member.connection, &mut mt, |t| {
        t.ariant_results == 1 && t.warps.contains(&KINGS_ROOM)
    })
    .await;
    // With 98 points and 3 to come, Abdullah sends the host to Areda.
    told(&mut host, talk(KINGS_ROOM, ARIANT_KING)).await;
    let t = told(&mut host, answer(MENU, 1, Some(0))).await;
    let nearly = net::content::text().get(keys::ARIANT_KING_NEARLY, &[("points", "98")]);
    assert!(
        t.prompt_texts.iter().any(|said| said.contains(&nearly)),
        "{t:?}"
    );
    close(&mut host).await;
    let t = told(&mut host, talk(KINGS_ROOM, ARIANT_AREDA)).await;
    let jewels =
        net::content::text().get(keys::ARIANT_JEWELS, &[("jewels", "30"), ("points", "3")]);
    assert!(
        t.prompt_texts.iter().any(|said| said.contains(&jewels)),
        "{t:?}"
    );
    let t = told(&mut host, answer(SAY, 1, None)).await;
    assert!(said(&t, keys::ARIANT_THANKS), "{t:?}");
    assert!(t.removed.contains(&(ETC, 1)), "{t:?}");
    assert!(
        (t.stat_updates.iter()).any(|update| update.get(&EXP).is_some_and(|&exp| exp > 0)),
        "{t:?}"
    );
    close(&mut host).await;
    let t = told(&mut host, talk(KINGS_ROOM, ARIANT_AREDA)).await;
    assert!(said(&t, keys::ARIANT_NO_JEWELS), "{t:?}");
    close(&mut host).await;

    // A hundred of the 101 buy the chair.
    told(&mut host, talk(KINGS_ROOM, ARIANT_KING)).await;
    let t = told(&mut host, answer(MENU, 1, Some(0))).await;
    let trade = net::content::text().get(keys::ARIANT_KING_TRADE, &[("price", "100")]);
    assert!(
        t.prompt_texts.iter().any(|said| said.contains(&trade)),
        "{t:?}"
    );
    let t = told(&mut host, answer(SAY, 1, None)).await;
    assert!(t.added.iter().any(|&(_, _, item)| item == CHAIR), "{t:?}");

    // The one left is far short of another.
    told(&mut host, talk(KINGS_ROOM, ARIANT_KING)).await;
    let t = told(&mut host, answer(MENU, 1, Some(0))).await;
    let short = net::content::text().get(
        keys::ARIANT_KING_SHORT,
        &[("points", "1"), ("price", "100")],
    );
    assert!(
        t.prompt_texts.iter().any(|said| said.contains(&short)),
        "{t:?}"
    );
    close(&mut host).await;

    // Leaving without seeing Areda takes the jewels.
    let mut mt = Trace::default();
    warp(&mut member.connection, HENESYS, "", &mut mt).await;
    read_for(&mut member.connection, &mut mt, 300).await;
    assert!(mt.removed.contains(&(ETC, 1)), "{mt:?}");
}
