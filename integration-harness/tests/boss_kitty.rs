//! Boss Kitty in Showa Town quizzes one on Sakura, the Kitty, and the
//! Orange Marble (8012) for the marble, for 300 Fried Chicken a try.

use game_data::server_content::ServerText;
use integration_harness::database::{grant_stack, start_quest};
use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{BOSS_KITTY, FRIED_CHICKEN, ORANGE_MARBLE, ORANGE_MARBLE_QUEST};
use tokio::time::Duration;

const SHOWA_TOWN: i32 = 801000000;
const USE: i16 = 2;

/// The menu choice of a Boss Kitty question that is its right answer,
/// found by its question, since one question's answer can be a wrong
/// choice of another.
fn right_answer(menu: &str) -> i32 {
    let text = ServerText::load(None).unwrap();
    let (question, choices) = menu.split_once("\n#L").expect("a menu");
    let right = (0..)
        .map(|n| format!("boss_kitty.q.{n}"))
        .take_while(|key| text.contains(key))
        .find(|key| text.get(key, &[]) == question)
        .map(|key| text.get(&format!("{key}.right"), &[]))
        .expect("a known question");
    format!("#L{choices}")
        .split("#L")
        .skip(1)
        .find_map(|choice| {
            let (index, label) = choice.split_once("##b")?;
            (label.split_once("#k#l")?.0 == right).then(|| index.parse().unwrap())
        })
        .expect("the right answer among the choices")
}

#[tokio::test]
async fn boss_kitty_gives_the_orange_marble_for_five_right_answers() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, USE, 1, FRIED_CHICKEN, 300).await.unwrap();
    start_quest(id, ORANGE_MARBLE_QUEST).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, SHOWA_TOWN, "", &mut seen).await, SHOWA_TOWN);
    let asked = told(&mut session, talk(SHOWA_TOWN, BOSS_KITTY)).await;
    assert_eq!(asked.prompts, [(BOSS_KITTY, 1)], "{asked:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(FRIED_CHICKEN, -300))).await;
    assert_eq!(seen.prompts, [(BOSS_KITTY, 0)], "{seen:?}");
    let mut t = told(&mut session, answer(0, 1, None)).await;
    for _ in 0..5 {
        assert_eq!(t.prompts, [(BOSS_KITTY, 4)], "{t:?}");
        let choice = right_answer(&t.prompt_texts[0]);
        t = told(&mut session, answer(4, 1, Some(choice))).await;
    }
    assert_eq!(t.prompts, [(BOSS_KITTY, 0)], "{t:?}");
    let mut seen = told(&mut session, answer(0, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(ORANGE_MARBLE, 1))).await;
}
