//! The guild bulletin board (GUILD_BBS): members post threads, which are
//! listed newest first, ten to a page, and reply to them; the master and
//! jr. masters post the one notice, and edit or delete anyone's post or
//! reply, where a member edits or deletes only their own. A guild sees only
//! its own board, and a player with no guild gets no answer.

use integration_harness::database::{found_guild, grant_stack, join_guild, set_guild_rank};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::step;
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use tokio::time::Duration;

const LIST: u8 = 6;
const THREAD: u8 = 7;
const MISSING: u8 = 8;
const CASH: i16 = 5;
/// The first cash emoticon, which icon 100 needs.
const EMOTICON: i32 = 5_290_000;

async fn new_character(config: &HarnessConfig) -> i32 {
    let session = login_to_world_session(config).await.expect("first login");
    let id = session.character_id;
    drop(session);
    drop(relogin(config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    id
}

fn bbs(mode: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::GuildBbs as i16).unwrap();
    p.write_byte(mode).unwrap();
    p
}

fn write(edit: Option<i32>, notice: bool, title: &str, icon: i32) -> Packet {
    write_body(edit, notice, title, &format!("{title} body"), icon)
}

fn write_body(edit: Option<i32>, notice: bool, title: &str, body: &str, icon: i32) -> Packet {
    let mut p = bbs(0);
    p.write_byte(u8::from(edit.is_some())).unwrap();
    if let Some(id) = edit {
        p.write_int(id).unwrap();
    }
    p.write_byte(u8::from(notice)).unwrap();
    p.write_str_with_length(title).unwrap();
    p.write_str_with_length(body).unwrap();
    p.write_int(icon).unwrap();
    p
}

fn with_int(mode: u8, value: i32) -> Packet {
    let mut p = bbs(mode);
    p.write_int(value).unwrap();
    p
}

fn reply(thread: i32, text: &str) -> Packet {
    let mut p = with_int(4, thread);
    p.write_str_with_length(text).unwrap();
    p
}

fn delete_reply(thread: i32, reply: i32) -> Packet {
    let mut p = with_int(5, thread);
    p.write_int(reply).unwrap();
    p
}

/// What a request got back: every GUILD_BBS body.
async fn ask(session: &mut WorldSession, what: &str, packet: Packet) -> Vec<Vec<u8>> {
    let marker = format!("{} {what}", session.character_name);
    step(&mut session.connection, &marker, packet).await.bbs
}

#[derive(Debug, PartialEq)]
struct Row {
    id: i32,
    poster: i32,
    title: String,
    icon: i32,
    replies: i32,
}

fn row(r: &mut Cursor<&[u8]>) -> Row {
    let (id, poster, title) = (
        r.read_int().unwrap(),
        r.read_int().unwrap(),
        r.read_str_with_length().unwrap(),
    );
    r.read_long().unwrap();
    Row {
        id,
        poster,
        title,
        icon: r.read_int().unwrap(),
        replies: r.read_int().unwrap(),
    }
}

/// A mode 6 body: the notice, the other threads' count, and the page.
fn list(body: &[u8]) -> (Option<Row>, i32, Vec<Row>) {
    assert_eq!(body[0], LIST);
    let mut r = Cursor::new(&body[1..]);
    let notice = (r.read_byte().unwrap() != 0).then(|| row(&mut r));
    let total = r.read_int().unwrap();
    let mut page = Vec::new();
    if total > 0 {
        for _ in 0..r.read_int().unwrap() {
            page.push(row(&mut r));
        }
    }
    assert_eq!(r.position() as usize, body.len() - 1);
    (notice, total, page)
}

#[derive(Debug, PartialEq)]
struct Shown {
    id: i32,
    poster: i32,
    title: String,
    body: String,
    icon: i32,
    /// (reply, poster, text), oldest first.
    replies: Vec<(i32, i32, String)>,
}

/// A mode 7 body.
fn thread(body: &[u8]) -> Shown {
    assert_eq!(body[0], THREAD);
    let mut r = Cursor::new(&body[1..]);
    let (id, poster) = (r.read_int().unwrap(), r.read_int().unwrap());
    r.read_long().unwrap();
    let (title, text, icon) = (
        r.read_str_with_length().unwrap(),
        r.read_str_with_length().unwrap(),
        r.read_int().unwrap(),
    );
    let mut replies = Vec::new();
    for _ in 0..r.read_int().unwrap() {
        let (reply, poster) = (r.read_int().unwrap(), r.read_int().unwrap());
        r.read_long().unwrap();
        replies.push((reply, poster, r.read_str_with_length().unwrap()));
    }
    assert_eq!(r.position() as usize, body.len() - 1);
    Shown {
        id,
        poster,
        title,
        body: text,
        icon,
        replies,
    }
}

/// The one thread a request got back.
async fn shown(session: &mut WorldSession, what: &str, packet: Packet) -> Shown {
    let got = ask(session, what, packet).await;
    assert_eq!(got.len(), 1, "{what}: {got:?}");
    thread(&got[0])
}

async fn page(session: &mut WorldSession, page: i32) -> (Option<Row>, i32, Vec<Row>) {
    let got = ask(session, &format!("page {page}"), with_int(2, page)).await;
    assert_eq!(got.len(), 1, "page {page}: {got:?}");
    list(&got[0])
}

#[tokio::test]
async fn members_post_reply_and_moderate_their_guilds_board() {
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let mut ids = Vec::new();
    for config in &configs {
        ids.push(new_character(config).await);
    }
    let [a_id, b_id, c_id, _] = ids[..] else {
        unreachable!()
    };
    // A masters a guild B is a member of, C masters another, D has none.
    let guild = configs[0].character_name.clone();
    found_guild(a_id, &guild).await.expect("A's guild");
    join_guild(b_id, &guild).await.expect("B joins");
    found_guild(c_id, &configs[2].character_name)
        .await
        .expect("C's guild");
    grant_stack(a_id, CASH, 1, EMOTICON, 1)
        .await
        .expect("emoticon");
    let mut a = relogin(&configs[0]).await;
    let mut b = relogin(&configs[1]).await;
    let mut c = relogin(&configs[2]).await;
    let mut d = relogin(&configs[3]).await;

    // An empty board, then a post, which is shown.
    assert_eq!(page(&mut a, 0).await, (None, 0, vec![]));
    let first = shown(&mut a, "post", write(None, false, "First", 0)).await;
    assert_eq!(
        first,
        Shown {
            id: 1,
            poster: a_id,
            title: "First".into(),
            body: "First body".into(),
            icon: 0,
            replies: vec![],
        }
    );

    // Twelve threads make two pages, newest first; a page past the end is
    // the first. A long title is cut to 25 characters.
    for n in 2..=11 {
        assert_eq!(
            shown(&mut b, "post", write(None, false, &format!("T{n}"), 1))
                .await
                .id,
            n
        );
    }
    let long = "x".repeat(30);
    let cut = shown(&mut a, "long", write(None, false, &long, 2)).await;
    assert_eq!((cut.id, cut.title.len()), (12, 25));
    let (notice, total, threads) = page(&mut b, 0).await;
    assert_eq!((notice, total), (None, 12));
    assert_eq!(
        threads.iter().map(|row| row.id).collect::<Vec<_>>(),
        (3..=12).rev().collect::<Vec<_>>()
    );
    assert_eq!(
        threads[1],
        Row {
            id: 11,
            poster: b_id,
            title: "T11".into(),
            icon: 1,
            replies: 0,
        }
    );
    let (_, _, threads) = page(&mut b, 1).await;
    assert_eq!(threads.iter().map(|row| row.id).collect::<Vec<_>>(), [2, 1]);
    let (_, _, threads) = page(&mut b, 5).await;
    assert_eq!(threads[0].id, 12);

    // Icons: three free ones, and a cash one only with its emoticon.
    assert!(ask(&mut a, "icon 3", write(None, false, "Bad", 3))
        .await
        .is_empty());
    assert!(ask(&mut b, "icon 100", write(None, false, "Bad", 100))
        .await
        .is_empty());
    assert!(ask(&mut a, "icon 101", write(None, false, "Bad", 101))
        .await
        .is_empty());
    let cash = shown(&mut a, "icon 100", write(None, false, "Cash", 100)).await;
    assert_eq!((cash.id, cash.icon), (13, 100));
    assert!(ask(&mut a, "delete 13", with_int(1, 13)).await.is_empty());

    // Only the master posts the notice, once; it is thread 0.
    assert!(ask(&mut b, "notice", write(None, true, "Mine", 0))
        .await
        .is_empty());
    assert_eq!(
        shown(&mut a, "notice", write(None, true, "Rules", 0))
            .await
            .id,
        0
    );
    assert!(ask(&mut a, "notice again", write(None, true, "Again", 0))
        .await
        .is_empty());
    let (notice, total, threads) = page(&mut b, 0).await;
    assert_eq!(
        notice,
        Some(Row {
            id: 0,
            poster: a_id,
            title: "Rules".into(),
            icon: 0,
            replies: 0,
        })
    );
    assert_eq!((total, threads.len()), (12, 10));

    // Replies, oldest first, and counted in the list.
    let b_reply = shown(&mut b, "reply", reply(1, "hi")).await;
    assert_eq!(b_reply.replies.len(), 1);
    let both = shown(&mut a, "reply", reply(1, "yo")).await;
    let texts: Vec<_> = both
        .replies
        .iter()
        .map(|(_, poster, text)| (*poster, text.as_str()))
        .collect();
    assert_eq!(texts, [(b_id, "hi"), (a_id, "yo")]);
    let (b_reply_id, a_reply_id) = (both.replies[0].0, both.replies[1].0);
    assert!(ask(&mut b, "empty reply", reply(1, "")).await.is_empty());
    let (_, _, threads) = page(&mut b, 1).await;
    assert_eq!((threads[1].id, threads[1].replies), (1, 2));

    // A member doesn't edit or delete the master's thread or reply.
    assert!(ask(&mut b, "edit 1", write(Some(1), false, "Hacked", 0))
        .await
        .is_empty());
    assert!(ask(&mut b, "delete 1", with_int(1, 1)).await.is_empty());
    assert!(ask(&mut b, "delete A's reply", delete_reply(1, a_reply_id))
        .await
        .is_empty());
    assert_eq!(shown(&mut b, "show 1", with_int(3, 1)).await.title, "First");

    // Another guild sees its own board and can't touch this one.
    assert_eq!(page(&mut c, 0).await, (None, 0, vec![]));
    assert_eq!(ask(&mut c, "show 1", with_int(3, 1)).await, [vec![MISSING]]);
    assert!(ask(&mut c, "delete reply", delete_reply(1, b_reply_id))
        .await
        .is_empty());
    assert!(ask(&mut c, "delete 1", with_int(1, 1)).await.is_empty());

    // A player with no guild gets no answer.
    assert!(ask(&mut d, "list", with_int(2, 0)).await.is_empty());
    assert!(ask(&mut d, "post", write(None, false, "Hi", 0))
        .await
        .is_empty());

    // A member edits and deletes their own; the master anyone's.
    let edited = shown(&mut b, "edit 2", write(Some(2), false, "T2 again", 1)).await;
    assert_eq!((edited.id, edited.title.as_str()), (2, "T2 again"));
    let left = shown(&mut a, "delete B's reply", delete_reply(1, b_reply_id)).await;
    assert_eq!(left.replies.len(), 1);
    let edited = shown(&mut a, "edit B's", write(Some(3), false, "Moderated", 0)).await;
    assert_eq!((edited.poster, edited.title.as_str()), (b_id, "Moderated"));
    assert!(ask(&mut a, "delete 1", with_int(1, 1)).await.is_empty());
    assert_eq!(ask(&mut a, "show 1", with_int(3, 1)).await, [vec![MISSING]]);
    assert_eq!(
        ask(&mut b, "reply to 1", reply(1, "late")).await,
        [vec![MISSING]]
    );
    assert!(ask(&mut b, "delete 2", with_int(1, 2)).await.is_empty());
    let (notice, total, threads) = page(&mut a, 0).await;
    assert!(notice.is_some());
    assert_eq!(total, 10);
    assert_eq!(
        threads.iter().map(|row| row.id).collect::<Vec<_>>(),
        (3..=12).rev().collect::<Vec<_>>()
    );

    // A deleted thread's number is free again only if it was the newest.
    assert_eq!(
        shown(&mut a, "post", write(None, false, "Next", 0))
            .await
            .id,
        13
    );
}

#[tokio::test]
async fn jr_masters_moderate_and_long_or_empty_text_is_cut_or_refused() {
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let a_id = new_character(&configs[0]).await;
    let b_id = new_character(&configs[1]).await;
    let guild = configs[0].character_name.clone();
    found_guild(a_id, &guild).await.expect("A's guild");
    join_guild(b_id, &guild).await.expect("B joins");
    let mut a = relogin(&configs[0]).await;
    let mut b = relogin(&configs[1]).await;

    // An empty title or body is refused and takes no number; a long body
    // is cut to 600 characters, a title to 25 characters (not bytes).
    assert!(ask(&mut a, "no title", write_body(None, false, "", "b", 0))
        .await
        .is_empty());
    assert!(ask(&mut a, "no body", write_body(None, false, "t", "", 0))
        .await
        .is_empty());
    let long = shown(
        &mut a,
        "long",
        write_body(None, false, &"é".repeat(30), &"y".repeat(700), 0),
    )
    .await;
    assert_eq!(long.id, 1);
    assert_eq!(long.title, "é".repeat(25));
    assert_eq!(long.body, "y".repeat(600));
    let replied = shown(&mut b, "long reply", reply(1, &"r".repeat(30))).await;
    assert_eq!(replied.replies[0].2, "r".repeat(25));
    assert_eq!(page(&mut b, -1).await.2[0].id, 1);

    // A reply to a thread that isn't there is told so; a reply that isn't
    // there is not deleted.
    assert_eq!(
        ask(&mut b, "reply to 9", reply(9, "hi")).await,
        [vec![MISSING]]
    );
    assert!(ask(&mut a, "delete reply 0", delete_reply(1, 0))
        .await
        .is_empty());

    // A jr. master posts the notice and edits anyone's thread; the rank is
    // read live, so once demoted they no longer do.
    assert!(ask(&mut b, "notice", write(None, true, "Mine", 0))
        .await
        .is_empty());
    set_guild_rank(b_id, 2).await.expect("B jr. master");
    assert_eq!(
        shown(&mut b, "notice", write(None, true, "Rules", 0))
            .await
            .id,
        0
    );
    assert_eq!(shown(&mut a, "show 0", with_int(3, 0)).await.title, "Rules");
    let edited = shown(&mut b, "edit 1", write(Some(1), false, "Tidied", 1)).await;
    assert_eq!((edited.poster, edited.title.as_str()), (a_id, "Tidied"));
    set_guild_rank(b_id, 3).await.expect("B member");
    assert!(
        ask(&mut b, "edit 1 again", write(Some(1), false, "Mine", 0))
            .await
            .is_empty()
    );

    // The master edits and deletes the notice; then there can be a new one.
    let edited = shown(&mut a, "edit 0", write(Some(0), false, "Rules 2", 0)).await;
    assert_eq!((edited.id, edited.poster), (0, b_id));
    assert!(ask(&mut a, "delete 0", with_int(1, 0)).await.is_empty());
    assert_eq!(page(&mut a, 0).await.0, None);
    assert_eq!(
        shown(&mut a, "notice", write(None, true, "New", 0))
            .await
            .id,
        0
    );
}
