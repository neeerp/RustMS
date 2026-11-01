//! The Small Street Light in Kerning City's subway weakens Shade when
//! turned on: its skills are sealed (mob skill 157 level 1) and its
//! avoidability cut (155 level 2), and the line reads so.

use integration_harness::events::{talk, told};
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use net::npc_scripts::BOSS_SWITCHES;

const SUBWAY: i32 = 103_000_202;
const SEAL_SKILL: i32 = 157 | 1 << 16;
const LESS_AVOIDABILITY: i32 = 155 | 2 << 16;

#[tokio::test]
async fn the_lamp_seals_shades_skills_and_cuts_its_avoidability() {
    let (lamp, shade, _, notice) = BOSS_SWITCHES[0];
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, SUBWAY, "", &mut seen).await, SUBWAY);
    until(c, &mut seen, |t| !t.field.alive(shade).is_empty()).await;
    let mob = seen.field.alive(shade)[0];

    let asked = told(&mut session, talk(SUBWAY, lamp)).await;
    assert_eq!(asked.prompts, [(lamp, 1)], "{asked:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let line = net::content::text().get(notice, &[]);
    let c = &mut session.connection;
    until(c, &mut seen, |t| {
        let set = |skill| t.field.stat_sets.iter().any(|s| (s.0, s.3) == (mob, skill));
        set(SEAL_SKILL) && set(LESS_AVOIDABILITY) && t.item_lines.contains(&(line.clone(), 0))
    })
    .await;
}
