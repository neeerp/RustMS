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
const LEAFRE_STATION_ENTRANCE: i32 = 240000100;
/// The Haunted House's first hall, past the grounds' gate.
const HAUNTED_HOUSE: i32 = 682000100;
/// The Mushroom Castle's way in and through (Cosmic's `td_mushcastle`).
const MUSHROOM_FOREST_FIELD: i32 = 106020000;
const SPLIT_ROAD: i32 = 106020400;
const CASTLE_WALL_EDGE: i32 = 106020501;
const CASTLE_EAST_GATE: i32 = 106021300;
const POISON_SPORE: i32 = 4000507;
/// Kerning City's subway: Line 1's first area, and its second, where the
/// construction site's tunnel comes out.
const LINE_1_AREA_1: i32 = 103000101;
const LINE_1_AREA_2: i32 = 103000103;
/// `rankRoom`: each job's hall (and Ereve's and the Dangerous Forest's),
/// the room it leads into, and the room's portal back.
const RANK_ROOMS: [(i32, i32, &str); 8] = [
    (100000201, 100000204, "out00"),
    (101000003, 101000004, "out00"),
    (102000003, 102000004, "out00"),
    (103000003, 103000008, "out00"),
    (120000101, 120000105, "out00"),
    (130000000, 130000100, "east00"),
    (130000200, 130000100, "west00"),
    (140010100, 140010110, "out00"),
];

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
    /// Open the client's emblem maker (Lea); the emblem is checked, and
    /// paid for, when the client sends it.
    OpenEmblemMaker,
    /// Grow the player's guild a step from `from` members, the capacity
    /// Heracle quoted the price for, and charge it. The client hears how it
    /// went from the guild, so nothing may follow it in the script.
    GrowGuild {
        from: u8,
    },
    /// Disband the player's guild, if they are its master. As with
    /// `GrowGuild`, nothing may follow it.
    DisbandGuild,
    /// Make a beginner a first-job Explorer (`Player::take_first_job`).
    /// Nothing is applied unless the player still qualifies.
    TakeFirstJob(i16),
    /// Make a first-job Explorer a second job (`Player::take_second_job`),
    /// on the same terms.
    TakeSecondJob(i16),
    /// Make a second-job Explorer a third job (`Player::take_third_job`),
    /// on the same terms.
    TakeThirdJob(i16),
    /// Make a third-job Explorer a fourth job (`Player::take_fourth_job`),
    /// on the same terms, and teach the job's first skills
    /// (`skills::fourth_job_skills`).
    TakeFourthJob(i16),
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
    /// A line in pink chat, as a portal that won't let the player through
    /// says why.
    Pink(String),
    /// Take the player's party into its own copy of an event's maps
    /// (`events::Event::team`). Nothing is applied unless the party still
    /// may go; the script says its `refused` text instead.
    StartEvent(crate::events::EventKind),
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
        // Leafre Station's way back into town (Cosmic's `dracoout`).
        "dracoout" => ScriptOutcome {
            effects: vec![
                Effect::PortalSound,
                Effect::WarpToPortal(
                    LEAFRE_STATION_ENTRANCE,
                    portal_id(LEAFRE_STATION_ENTRANCE, |portal| portal.name == "east00"),
                ),
            ],
            block: false,
        },
        // The Haunted House's gate, from its grounds (Cosmic's
        // `halloween_enter`).
        "halloween_enter" => ScriptOutcome {
            effects: vec![
                Effect::PortalSound,
                Effect::WarpToPortal(
                    HAUNTED_HOUSE,
                    portal_id(HAUNTED_HOUSE, |portal| portal.name == "st00"),
                ),
            ],
            block: false,
        },
        // An Empty House's way to the Mushroom Kingdom, open once the
        // quest that leads there is done or its first quests are taken.
        "TD_MC_first"
            if quest(2260) == QuestState::Completed
                || (2300..=2310).any(|id| quest(id) != QuestState::NotStarted) =>
        {
            through(Effect::Warp(MUSHROOM_FOREST_FIELD))
        }
        "TD_MC_first" => refused(keys::MC_FIRST_BLOCKED),
        // A Poison Spore clears the poisonous mushrooms in the way.
        "obstacle" if player.inventory.count(POISON_SPORE) > 0 => ScriptOutcome {
            effects: vec![
                Effect::Pink(content::text().get(keys::MC_OBSTACLE_USED, &[])),
                Effect::PortalSound,
                Effect::TakeItem {
                    item_id: POISON_SPORE,
                    quantity: 1,
                },
                Effect::WarpToPortal(SPLIT_ROAD, named(SPLIT_ROAD, "left00")),
            ],
            block: false,
        },
        "obstacle" => refused(keys::MC_OBSTACLE_BLOCKED),
        // Over the castle wall, once the Thorn Remover quest is done.
        "gotocastle" if quest(2324) == QuestState::Completed => {
            through(Effect::Warp(CASTLE_WALL_EDGE))
        }
        "gotocastle" => refused(keys::MC_CASTLE_BLOCKED),
        "TD_MC_jump" => through(Effect::Warp(CASTLE_WALL_EDGE)),
        "TD_MC_Egate" => through(Effect::WarpToPortal(
            CASTLE_EAST_GATE,
            named(CASTLE_EAST_GATE, "right00"),
        )),
        // Into the subway's tunnels, from its ticketing booth and from the
        // construction site north of town.
        "subway_in2" => through(Effect::WarpToPortal(
            LINE_1_AREA_1,
            named(LINE_1_AREA_1, "out00"),
        )),
        // The second of the area's spawn points, as in Cosmic.
        "met_in" => through(Effect::WarpToPortal(LINE_1_AREA_2, 1)),
        "rankRoom" => {
            let &(_, room, back) = RANK_ROOMS
                .iter()
                .find(|&&(hall, _, _)| hall == player.stats.map_id)?;
            through(Effect::WarpToPortal(room, named(room, back)))
        }
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

/// A portal that lets the player through with its sound.
fn through(warp: Effect) -> ScriptOutcome {
    ScriptOutcome {
        effects: vec![Effect::PortalSound, warp],
        block: false,
    }
}

/// A portal that keeps the player back, saying why in pink chat.
fn refused(key: &str) -> ScriptOutcome {
    ScriptOutcome {
        effects: vec![Effect::Pink(content::text().get(key, &[]))],
        block: false,
    }
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

/// The id of `map_id`'s portal called `name`, else 0.
fn named(map_id: i32, name: &str) -> u8 {
    portal_id(map_id, |portal| portal.name == name)
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

    fn pink(outcome: &ScriptOutcome) -> bool {
        matches!(outcome.effects.as_slice(), [Effect::Pink(_)])
    }

    #[test]
    fn the_mushroom_castle_lets_players_in_once_their_quests_allow() {
        let through = |warp| vec![Effect::PortalSound, warp];
        let first = |quests| portal("TD_MC_first", &player_with(quests)).unwrap();
        assert!(pink(&first(&[])));
        assert!(pink(&first(&[(2299, QuestState::Started)])));
        for quests in [
            &[(2260, QuestState::Completed)][..],
            &[(2300, QuestState::Started)],
            &[(2310, QuestState::Completed)],
        ] {
            assert_eq!(first(quests).effects, through(Effect::Warp(106020000)));
        }

        let castle = |quests| portal("gotocastle", &player_with(quests)).unwrap();
        assert!(pink(&castle(&[(2324, QuestState::Started)])));
        assert_eq!(
            castle(&[(2324, QuestState::Completed)]).effects,
            through(Effect::Warp(106020501))
        );
        assert_eq!(
            portal("TD_MC_jump", &player_with(&[])).unwrap().effects,
            through(Effect::Warp(106020501))
        );
    }

    #[test]
    fn a_poison_spore_clears_the_way_through_the_mushroom_forest() {
        let mut player = player_with(&[]);
        assert!(pink(&portal("obstacle", &player).unwrap()));
        player
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(POISON_SPORE, 2));
        let effects = portal("obstacle", &player).unwrap().effects;
        assert!(matches!(effects[0], Effect::Pink(_)));
        assert!(effects.contains(&Effect::TakeItem {
            item_id: POISON_SPORE,
            quantity: 1,
        }));
        assert!(matches!(
            effects.last(),
            Some(Effect::WarpToPortal(106020400, _))
        ));
    }

    #[test]
    fn the_subways_tunnels_and_the_halls_rooms_open_with_the_portal_sound() {
        let mut player = player_with(&[]);
        let warp = |script, player: &Player| match portal(script, player).unwrap().effects[..] {
            [Effect::PortalSound, Effect::WarpToPortal(map, id)] => (map, id),
            ref effects => panic!("{effects:?}"),
        };
        assert_eq!(warp("subway_in2", &player), (103000101, 3));
        assert_eq!(warp("met_in", &player), (103000103, 1));
        for (hall, room, id) in [
            (100000201, 100000204, 2),
            (101000003, 101000004, 2),
            (102000003, 102000004, 1),
            (103000003, 103000008, 1),
            (120000101, 120000105, 1),
            (130000000, 130000100, 5),
            (130000200, 130000100, 4),
            (140010100, 140010110, 1),
        ] {
            player.stats.map_id = hall;
            assert_eq!(warp("rankRoom", &player), (room, id));
        }
        player.stats.map_id = HENESYS;
        assert_eq!(portal("rankRoom", &player), None);
    }

    #[test]
    fn leafre_station_lets_players_back_into_town() {
        let outcome = portal("dracoout", &player_with(&[])).unwrap();
        assert_eq!(
            outcome.effects,
            [
                Effect::PortalSound,
                Effect::WarpToPortal(LEAFRE_STATION_ENTRANCE, 2)
            ]
        );
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
