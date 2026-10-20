//! Native map and portal scripts, keyed by the script names in `Map.nx`
//! (`info/onUserEnter`, portal `script`), and the effects all scripts
//! return. Lua is deferred; returning effects keeps the shape a scripting
//! runtime would have. Unknown names do nothing.

use crate::content;
use game::character::FREE_MARKET_RETURN;
use game::{InventoryType, Player, QuestState};
use game_data::server_content::keys;
use game_data::PortalTemplate;

const FREE_MARKET: i32 = 910000000;
const HENESYS: i32 = 100000000;

/// What a map, portal, or NPC script does. Scripts only return effects; the
/// runtime applies them, committing player changes together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// A screen effect from `Map.nx/Effect.img` (map name banners).
    FieldEffect(String),
    /// A UI image anchored to the player (`UI/tutorial.img/<n>` hints).
    AvatarEffect(String),
    /// An instruction balloon above the player.
    Hint {
        text: String,
        width: i16,
        height: i16,
    },
    /// Offer to skip the tutorial (Maple Administrator), only to accounts
    /// that already have a level-30 character.
    OfferTutorialSkip,
    SetHp(i16),
    GiveItem {
        item_id: i32,
        quantity: i16,
    },
    /// Nothing is applied if the player no longer has them.
    TakeItem {
        item_id: i32,
        quantity: i16,
    },
    /// Bring the doll with cash serial `serial` back to life for `days`;
    /// nothing is applied if the player no longer has it.
    RevivePet {
        serial: i64,
        days: i32,
    },
    /// Nothing is applied if the player no longer has that much.
    SpendMeso(i32),
    GainExp(i32),
    StartQuest(i16),
    CompleteQuest(i16),
    Warp(i32),
    /// To a map's portal by id, rather than its first spawn point.
    WarpToPortal(i32, u8),
    /// Nothing is applied unless the ship from this platform is boarding
    /// (`conti::State::Wait`); the player may have answered too late.
    RequireBoarding(i32),
    /// Ask for a new guild's name (Heracle); the guild is founded, and paid
    /// for, when the client answers.
    OpenGuildNamePrompt,
    /// Make a beginner a first-job Explorer (`Player::take_first_job`).
    /// Nothing is applied unless the player still qualifies.
    TakeFirstJob(i16),
    /// Make a first-job Explorer a second job (`Player::take_second_job`),
    /// on the same terms.
    TakeSecondJob(i16),
    /// Make a second-job Explorer a third job (`Player::take_third_job`),
    /// on the same terms.
    TakeThirdJob(i16),
    /// Set or clear a script marker (`game::quest::SCRIPT_MARKERS`), which
    /// the client is not told about.
    Mark(i16),
    Unmark(i16),
    /// Nothing is applied while anyone is on this map in the player's
    /// channel; the script says its `refused` text instead.
    RequireEmptyMap(i32),
    /// Add a row of four slots to a bag, up to 96.
    AddSlotRow(InventoryType),
    /// Give the player a new hair (3xxxx), face (2xxxx) or skin (anything
    /// else), judged as the client judges a style's kind.
    SetStyle(i32),
    /// Set or clear where the player goes back to after a trip
    /// (`Player::saved_locations`).
    SaveLocation(i16, Option<i32>),
    /// Play the portal sound, which the client leaves to the server for a
    /// scripted portal.
    PortalSound,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ScriptOutcome {
    pub effects: Vec<Effect>,
    /// Don't run this portal's script again this session.
    pub block: bool,
}

/// A map's `onUserEnter` script. `None` if the script is unknown.
pub fn map_enter(script: &str) -> Option<Vec<Effect>> {
    // go<map id>: the map's name banner. HeavenMS also unlocks the UI here,
    // which HeavenClient never locks.
    let map = script.strip_prefix("go")?;
    if map.is_empty() || !map.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(vec![Effect::FieldEffect(format!("maplemap/enter/{map}"))])
}

/// A touch portal's script. `None` if the script is unknown.
pub fn portal(script: &str, player: &Player) -> Option<ScriptOutcome> {
    let quest = |id: i16| player.quests.state(id);
    // The tutorial hints show once a condition holds, then not again this
    // session (HeavenMS blocks the portal even when nothing was shown, so a
    // hint touched too early would never appear).
    let hint = |condition: bool, image: u8| {
        if condition {
            ScriptOutcome {
                effects: vec![Effect::AvatarEffect(format!("UI/tutorial.img/{image}"))],
                block: true,
            }
        } else {
            ScriptOutcome::default()
        }
    };
    // Where the player was saved to go back to, else Henesys.
    let saved = |kind: i16| {
        player
            .saved_locations
            .get(&kind)
            .copied()
            .unwrap_or(HENESYS)
    };
    let outcome = match script {
        "infoMinimap" => hint(quest(1031) == QuestState::Started, 25),
        "infoAttack" => hint(quest(1035) == QuestState::Started, 20),
        "infoPickup" => hint(quest(1035) == QuestState::Started, 21),
        "infoSkill" => hint(quest(1035) == QuestState::Completed, 23),
        "infoWorldmap" => hint(true, 26),
        "infoReactor" if quest(1008) == QuestState::Completed => hint(true, 22),
        "infoReactor" => hint(quest(1020) == QuestState::Completed, 27),
        "tutoChatNPC" => ScriptOutcome {
            effects: vec![Effect::OfferTutorialSkip],
            block: true,
        },
        // The Free Market's exit, back to the town the player came from.
        "market00" => {
            let town = saved(FREE_MARKET_RETURN);
            go_back(
                FREE_MARKET_RETURN,
                Effect::WarpToPortal(town, portal_id(town, is_market_portal)),
            )
        }
        // A town's Free Market entrance (market01 to market22).
        script if script.starts_with("market") => ScriptOutcome {
            effects: vec![
                Effect::SaveLocation(FREE_MARKET_RETURN, Some(player.stats.map_id)),
                Effect::PortalSound,
                Effect::WarpToPortal(
                    FREE_MARKET,
                    portal_id(FREE_MARKET, |portal| portal.name == "out00"),
                ),
            ],
            block: false,
        },
        // Event maps' exits, back to where the invitation (2031001, 2031006,
        // 2031003) was used, kept under its `returnMapQR`.
        "photoOut" => go_back(7686, Effect::Warp(saved(7686))),
        "StudioZone_Out" => go_back(7752, Effect::Warp(saved(7752))),
        "outBabyBird" => go_back(10061, Effect::Warp(saved(10061))),
        "glTutoMsg0" => ScriptOutcome {
            effects: vec![Effect::Hint {
                text: content::text().get(keys::LEAVE_AREA_WARNING, &[]),
                width: 150,
                height: 5,
            }],
            block: false,
        },
        _ => return None,
    };
    Some(outcome)
}

/// End a trip: forget the saved location `kind` and take the player back
/// with `warp`.
fn go_back(kind: i16, warp: Effect) -> ScriptOutcome {
    ScriptOutcome {
        effects: vec![Effect::SaveLocation(kind, None), Effect::PortalSound, warp],
        block: false,
    }
}

fn is_market_portal(portal: &PortalTemplate) -> bool {
    portal
        .script
        .as_deref()
        .is_some_and(|script| script.contains("market"))
}

/// The id of `map_id`'s first portal that `pick` accepts, else 0.
fn portal_id(map_id: i32, pick: impl Fn(&PortalTemplate) -> bool) -> u8 {
    crate::game_data::get()
        .ok()
        .and_then(|data| data.field(map_id))
        .and_then(|field| field.portals().find(|portal| pick(portal)))
        .map_or(0, |portal| u8::try_from(portal.id).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::{QuestRecord, QuestStatus};
    use std::time::SystemTime;

    fn player_with(quests: &[(i16, QuestState)]) -> Player {
        let mut player = crate::codec::stats::sample_player();
        for &(quest_id, state) in quests {
            let status = match state {
                QuestState::Started => QuestStatus::Started { kills: Vec::new() },
                _ => QuestStatus::Completed {
                    at: SystemTime::UNIX_EPOCH,
                },
            };
            player.quests.upsert(QuestRecord { quest_id, status });
        }
        player
    }

    fn avatar(path: &str) -> ScriptOutcome {
        ScriptOutcome {
            effects: vec![Effect::AvatarEffect(path.into())],
            block: true,
        }
    }

    #[test]
    fn the_free_market_sends_players_back_where_they_came_from() {
        let mut player = player_with(&[]);
        player.stats.map_id = 100000100;
        assert_eq!(
            portal("market01", &player).unwrap().effects,
            vec![
                Effect::SaveLocation(FREE_MARKET_RETURN, Some(100000100)),
                Effect::PortalSound,
                Effect::WarpToPortal(FREE_MARKET, 34),
            ]
        );
        player.saved_locations.insert(FREE_MARKET_RETURN, 220000000);
        assert_eq!(
            portal("market00", &player).unwrap().effects,
            vec![
                Effect::SaveLocation(FREE_MARKET_RETURN, None),
                Effect::PortalSound,
                Effect::WarpToPortal(220000000, 31),
            ]
        );
        player.saved_locations.clear();
        assert_eq!(
            portal("market00", &player).unwrap().effects[2],
            Effect::WarpToPortal(HENESYS, 0)
        );
    }

    #[test]
    fn event_exits_go_back_where_the_invitation_was_used() {
        let mut player = player_with(&[]);
        player.saved_locations.insert(7686, 101000000);
        assert_eq!(
            portal("photoOut", &player).unwrap().effects,
            vec![
                Effect::SaveLocation(7686, None),
                Effect::PortalSound,
                Effect::Warp(101000000),
            ]
        );
        assert_eq!(
            portal("outBabyBird", &player).unwrap().effects[2],
            Effect::Warp(HENESYS)
        );
    }

    #[test]
    fn map_entry_shows_the_maps_banner() {
        assert_eq!(
            map_enter("go10000"),
            Some(vec![Effect::FieldEffect("maplemap/enter/10000".into())])
        );
        assert_eq!(map_enter("goAdventure"), None);
        assert_eq!(map_enter("go"), None);
    }

    #[test]
    fn hints_wait_for_their_quest_condition() {
        let fresh = player_with(&[]);
        assert_eq!(
            portal("infoMinimap", &fresh),
            Some(ScriptOutcome::default())
        );
        assert_eq!(
            portal("infoMinimap", &player_with(&[(1031, QuestState::Started)])),
            Some(avatar("UI/tutorial.img/25"))
        );
        let hunting = player_with(&[(1035, QuestState::Started)]);
        assert_eq!(
            portal("infoAttack", &hunting),
            Some(avatar("UI/tutorial.img/20"))
        );
        assert_eq!(
            portal("infoPickup", &hunting),
            Some(avatar("UI/tutorial.img/21"))
        );
        assert_eq!(
            portal("infoSkill", &hunting),
            Some(ScriptOutcome::default())
        );
        assert_eq!(
            portal("infoSkill", &player_with(&[(1035, QuestState::Completed)])),
            Some(avatar("UI/tutorial.img/23"))
        );
        assert_eq!(
            portal("infoWorldmap", &fresh),
            Some(avatar("UI/tutorial.img/26"))
        );
        assert_eq!(
            portal(
                "infoReactor",
                &player_with(&[(1008, QuestState::Completed)])
            ),
            Some(avatar("UI/tutorial.img/22"))
        );
        assert_eq!(
            portal(
                "infoReactor",
                &player_with(&[(1020, QuestState::Completed)])
            ),
            Some(avatar("UI/tutorial.img/27"))
        );
        assert_eq!(
            portal("tutoChatNPC", &fresh),
            Some(ScriptOutcome {
                effects: vec![Effect::OfferTutorialSkip],
                block: true
            })
        );
        assert_eq!(portal("somethingElse", &fresh), None);
    }

    #[test]
    fn the_leave_area_warning_repeats_with_its_text() {
        let outcome = portal("glTutoMsg0", &player_with(&[])).unwrap();
        assert!(!outcome.block);
        let [Effect::Hint {
            text,
            width,
            height,
        }] = outcome.effects.as_slice()
        else {
            panic!("expected a hint");
        };
        assert!(!text.is_empty());
        assert_eq!((*width, *height), (150, 5));
    }
}
