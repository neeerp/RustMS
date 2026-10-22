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
/// The ferry docks a town's portal leads down to: Ellinia's (`inERShip`)
/// and Rien's (`enterPort`).
const ELLINIA_SKY_FERRY: i32 = 101000400;
const RIEN_DOCK: i32 = 140020300;
/// The Temple of Time's lanes (`timeQuest`): each lane's portal, the quest
/// that opens it, and where it leads (through its `out00`). Lane five of
/// each road leads on to the next road (Cosmic's quest ids).
const TIME_LANES: [(i32, i16, i32); 15] = [
    (270010100, 3501, 270010110),
    (270010200, 3502, 270010210),
    (270010300, 3503, 270010310),
    (270010400, 3504, 270010410),
    (270010500, 3507, 270020000),
    (270020100, 3508, 270020110),
    (270020200, 3509, 270020210),
    (270020300, 3510, 270020310),
    (270020400, 3511, 270020410),
    (270020500, 3514, 270030000),
    (270030100, 3515, 270030110),
    (270030200, 3516, 270030210),
    (270030300, 3517, 270030310),
    (270030400, 3518, 270030410),
    (270030500, 3519, 270040000),
];
/// The Broken Corridor's way into the Temple Ruins, open with a Marble of
/// Chaos or once Beyond the Ruins is done.
const BROKEN_CORRIDOR: i32 = 270040000;
const TEMPLE_RUINS: i32 = 270040100;
const MARBLE_OF_CHAOS: i32 = 4032002;
const BEYOND_THE_RUINS: i16 = 3522;
/// The halls behind Henesys Park (`enterAchter`), Ellinia (`enterMagiclibrar`)
/// and the Sleepywood dungeon (`balogTemple`).
const BOWMAN_SCHOOL: i32 = 100000201;
const MAGIC_LIBRARY: i32 = 101000003;
const UNDERGROUND_TEMPLE_STAIRS: i32 = 105100000;
/// Orbis's Neglected Strolling Path (`enterNepenthes`).
const NEGLECTED_STROLLING_PATH: i32 = 200060001;
/// Ariant's Castle Garden and the pass it takes (`ariant_castle`).
const CASTLE_GARDEN: i32 = 260000301;
const PALACE_ENTRY_PASS: i32 = 4031582;
/// Ereve's Training Forest, locked down during the Master of Disguise
/// quests (20301 to 20305) to all but holders of a permit (`enterDisguise0`).
const TRAINING_FOREST: i32 = 130010000;
const INVESTIGATION_PERMIT: i32 = 4032179;
/// Leafre's Cave of Black Witches (`enterWitch`): the cave The Stolen Egg
/// (20404) opens, then the one each later quest moves on to.
const STOLEN_EGG: i16 = 20404;
const WITCH_CAVES: [(i16, i32); 2] = [(20407, 924010200), (20406, 924010100)];
const WITCH_CAVE: i32 = 924010000;
/// The Mushroom Castle's Wedding Hall entrance (`TD_MC_enterboss1`).
const WEDDING_HALL_ENTRANCE: i32 = 106021401;
/// Magatia's secret basement (`secretDoor`), from Zenumist's hallway or
/// Alcadno's lab, once Verifying the Password (3360) is taken.
const SECRET_BASEMENT_PATH: i32 = 261030000;
const ZENUMIST_HALLWAY: i32 = 261010000;
const VERIFYING_THE_PASSWORD: i16 = 3360;
/// Rien's way out to the Dangerous Forest (`rienTutor8`). While an Aran
/// beginner is on Basic Fitness Training 1-3 (21015-21017), it keeps them in
/// town, saying which way their training lies.
const DANGEROUS_FOREST: i32 = 140010000;
const ARAN_BEGINNER: i16 = 2000;
const FITNESS_TRAINING: [(i16, &str); 3] = [
    (21015, keys::RIEN_TUTOR_FITNESS_1),
    (21016, keys::RIEN_TUTOR_FITNESS_2),
    (21017, keys::RIEN_TUTOR_FITNESS_3),
];
/// The Forest of Evil's hidden way into the Cursed Forest (`curseforest`),
/// open at night to players on Arwen's quests: Rage, Resentment, and
/// Revenge (2224) or Arwen's Apology (2226) taken, or Defeat Faust (2227)
/// done, after which the forest is the one Faust has left.
const CURSED_FOREST: i32 = 910100000;
const CURSED_FOREST_AFTER_FAUST: i32 = 910100001;
const DEFEAT_FAUST: i16 = 2227;
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
    /// Found an alliance of the player's guild and their party's other
    /// guild master's, and charge for it (Lenario). Lenario then says how
    /// it went, so nothing may follow it.
    CreateAlliance {
        name: String,
    },
    /// Grow the player's alliance a guild from `from`, the capacity
    /// Lenario quoted the price for, and charge it. As with `GrowGuild`,
    /// nothing may follow it.
    GrowAlliance {
        from: u8,
    },
    /// Disband the player's alliance, if they are its master. Nothing may
    /// follow it.
    DisbandAlliance,
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
    /// Teach a skill up to `level` and `master_level`, never lowering
    /// either (as a quest's `Act/1/skill` reward).
    TeachSkill {
        skill_id: i32,
        level: i32,
        master_level: i32,
    },
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
    /// A line in the chat log by SHOW_STATUS_INFO 9 (Cosmic's
    /// `showInfoText`), as Rien's tutorial portal tells an Aran where to go.
    InfoText(String),
    /// Take the player's party into its own copy of an event's maps
    /// (`events::Event::team`). Nothing is applied unless the party still
    /// may go; the script says its `refused` text instead.
    StartEvent(crate::events::EventKind),
    /// Tell the player's event something happened, once the effects before
    /// it have applied. Scripts end on it.
    EventCommand(crate::events::EventCommand),
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
        "inERShip" => through(Effect::WarpToPortal(
            ELLINIA_SKY_FERRY,
            named(ELLINIA_SKY_FERRY, "out00"),
        )),
        // Cosmic also starts an Aran quest's fight here, which isn't in.
        "enterPort" => through(Effect::WarpToPortal(RIEN_DOCK, named(RIEN_DOCK, "west00"))),
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
        "timeQuest" => {
            let map = player.stats.map_id;
            let to = if map == BROKEN_CORRIDOR {
                (player.inventory.count(MARBLE_OF_CHAOS) > 0
                    || quest(BEYOND_THE_RUINS) == QuestState::Completed)
                    .then_some(TEMPLE_RUINS)
            } else {
                let &(_, gate, to) = TIME_LANES.iter().find(|&&(lane, _, _)| lane == map)?;
                (quest(gate) == QuestState::Completed).then_some(to)
            };
            match to {
                Some(to) => through(Effect::WarpToPortal(to, named(to, "out00"))),
                // Back to the start of the lane's road.
                None => {
                    let start = match map / 10000 {
                        27001 => 270010000,
                        27002 => 270020000,
                        _ => 270030000,
                    };
                    ScriptOutcome {
                        effects: vec![
                            Effect::PortalSound,
                            Effect::Pink(content::text().get(keys::TIME_LANE_LOST, &[])),
                            Effect::WarpToPortal(start, named(start, "in00")),
                        ],
                        block: false,
                    }
                }
            }
        }
        "enterAchter" => through(Effect::WarpToPortal(
            BOWMAN_SCHOOL,
            named(BOWMAN_SCHOOL, "out02"),
        )),
        // Cosmic starts a Cygnus quest's instance here while Maybe it's
        // Grendel! (20718) is taken; that isn't in.
        "enterMagiclibrar" => through(Effect::WarpToPortal(
            MAGIC_LIBRARY,
            named(MAGIC_LIBRARY, "jobout00"),
        )),
        "balogTemple" => through(Effect::WarpToPortal(
            UNDERGROUND_TEMPLE_STAIRS,
            named(UNDERGROUND_TEMPLE_STAIRS, "top00"),
        )),
        // Cosmic sends an Aran on To the Sealed Garden (21739) to its fight
        // instead; that isn't in.
        "enterNepenthes" => through(Effect::WarpToPortal(
            NEGLECTED_STROLLING_PATH,
            named(NEGLECTED_STROLLING_PATH, "out00"),
        )),
        "ariant_castle" if player.inventory.count(PALACE_ENTRY_PASS) > 0 => through(
            Effect::WarpToPortal(CASTLE_GARDEN, named(CASTLE_GARDEN, "out00")),
        ),
        "ariant_castle" => refused(keys::ARIANT_PALACE_NO_PASS),
        "enterDisguise0"
            if (20301..=20305).any(|id| quest(id) == QuestState::Started)
                && player.inventory.count(INVESTIGATION_PERMIT) == 0 =>
        {
            refused(keys::DISGUISE_NO_PERMIT)
        }
        "enterDisguise0" => through(Effect::WarpToPortal(
            TRAINING_FOREST,
            named(TRAINING_FOREST, "east00"),
        )),
        "enterWitch" if quest(STOLEN_EGG) == QuestState::Completed => {
            let cave = WITCH_CAVES
                .iter()
                .find(|&&(id, _)| quest(id) == QuestState::Completed)
                .map_or(WITCH_CAVE, |&(_, cave)| cave);
            through(Effect::WarpToPortal(cave, named(cave, "out00")))
        }
        "enterWitch" => refused(keys::WITCH_CAVE_CREEPY),
        // Cosmic starts the fight with the Yeti brothers here while Stop the
        // Wedding (2330) is taken and they aren't all beaten; that isn't in,
        // so the way is open to everyone rather than shut for good.
        "TD_MC_enterboss1" => through(Effect::WarpToPortal(
            WEDDING_HALL_ENTRANCE,
            named(WEDDING_HALL_ENTRANCE, "out00"),
        )),
        // Cosmic asks for the password while the quest is taken; its start
        // script, which sets one, isn't run, so the door just opens.
        "secretDoor" if quest(VERIFYING_THE_PASSWORD) != QuestState::NotStarted => {
            let side = if player.stats.map_id == ZENUMIST_HALLWAY {
                "sp_jenu"
            } else {
                "sp_alca"
            };
            through(Effect::WarpToPortal(
                SECRET_BASEMENT_PATH,
                named(SECRET_BASEMENT_PATH, side),
            ))
        }
        "secretDoor" => refused(keys::SECRET_DOOR_LOCKED),
        "rienTutor8" => {
            let training = FITNESS_TRAINING
                .iter()
                .find(|&&(id, _)| quest(id) == QuestState::Started)
                .filter(|_| player.stats.job == ARAN_BEGINNER);
            match training {
                Some(&(_, key)) => ScriptOutcome {
                    effects: vec![Effect::InfoText(content::text().get(key, &[]))],
                    block: false,
                },
                // Cosmic's portal 2, the forest's `east00`.
                None => through(Effect::WarpToPortal(
                    DANGEROUS_FOREST,
                    named(DANGEROUS_FOREST, "east00"),
                )),
            }
        }
        // The hour of the route clock, which RUSTMS_CONTI_SCALE speeds up
        // as it does the stations' clocks.
        "curseforest" => {
            let hour = crate::conti::time_of_day(crate::conti::now()).0;
            cursed_forest(player, hour)
        }
        "elevator" => elevator(player.stats.map_id, crate::conti::now()),
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

/// The Cursed Forest's way in at `hour` of the world's (UTC) day, the hour
/// the stations' clocks show: shut from 7:00 until 17:00.
fn cursed_forest(player: &Player, hour: u8) -> ScriptOutcome {
    let quest = |id: i16| player.quests.state(id);
    let after_faust = quest(DEFEAT_FAUST) == QuestState::Completed;
    if !(after_faust
        || [2224, 2226]
            .iter()
            .any(|&id| quest(id) == QuestState::Started))
    {
        return refused(keys::CURSED_FOREST_CLOSED);
    }
    if (7..17).contains(&hour) {
        return refused(keys::CURSED_FOREST_DAYTIME);
    }
    let forest = if after_faust {
        CURSED_FOREST_AFTER_FAUST
    } else {
        CURSED_FOREST
    };
    through(Effect::WarpToPortal(forest, named(forest, "out00")))
}

/// The Helios Tower elevator's door on `map_id` at route time `t`: into the
/// floor's waiting room while the car is there, which the car takes up or
/// down when it leaves (`net::conti`).
fn elevator(map_id: i32, t: i64) -> ScriptOutcome {
    match crate::conti::departing_from(map_id) {
        Some(route)
            if route.car.is_some() && route.timetable.state(t) == crate::conti::State::Wait =>
        {
            through(Effect::WarpToPortal(route.waiting_room, 0))
        }
        _ => refused(keys::ELEVATOR_AWAY),
    }
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
        assert_eq!(warp("inERShip", &player), (101000400, 2));
        assert_eq!(warp("enterPort", &player), (140020300, 1));
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

    /// Where an outcome that lets the player through warps them.
    fn warp_of(outcome: &ScriptOutcome) -> Option<(i32, u8)> {
        match outcome.effects.as_slice() {
            [Effect::PortalSound, Effect::WarpToPortal(map, portal)] => Some((*map, *portal)),
            _ => None,
        }
    }

    #[test]
    fn the_temple_of_time_lanes_open_quest_by_quest() {
        let at = |map, quests: &[(i16, QuestState)]| {
            let mut player = player_with(quests);
            player.stats.map_id = map;
            portal("timeQuest", &player).unwrap()
        };
        for (lane, gate, to) in TIME_LANES {
            assert_eq!(
                warp_of(&at(lane, &[(gate, QuestState::Completed)])),
                Some((to, named(to, "out00")))
            );
            let lost = at(lane, &[(gate, QuestState::Started)]);
            assert!(matches!(lost.effects[1], Effect::Pink(_)));
            let start = lane / 10000 * 10000;
            assert_eq!(
                lost.effects.last(),
                Some(&Effect::WarpToPortal(start, named(start, "in00")))
            );
        }
        // A lane's rest spot is through its `out00` (3); a road's last lane
        // leads to the next road's square (4).
        assert_eq!(
            warp_of(&at(270010100, &[(3501, QuestState::Completed)])),
            Some((270010110, 3))
        );
        assert_eq!(
            warp_of(&at(270010500, &[(3507, QuestState::Completed)])),
            Some((270020000, 4))
        );
        // The Broken Corridor sends the lost back to the Road to Oblivion.
        let lost = at(BROKEN_CORRIDOR, &[]);
        assert_eq!(
            lost.effects.last(),
            Some(&Effect::WarpToPortal(270030000, 3))
        );
        let ruins = Some((TEMPLE_RUINS, 3));
        assert_eq!(
            warp_of(&at(
                BROKEN_CORRIDOR,
                &[(BEYOND_THE_RUINS, QuestState::Completed)]
            )),
            ruins
        );
        let mut player = player_with(&[]);
        player.stats.map_id = BROKEN_CORRIDOR;
        player
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(MARBLE_OF_CHAOS, 1));
        assert_eq!(warp_of(&portal("timeQuest", &player).unwrap()), ruins);
        // Off the lanes, the script does nothing.
        player.stats.map_id = HENESYS;
        assert_eq!(portal("timeQuest", &player), None);
    }

    #[test]
    fn halls_and_gated_doors_of_section_three_let_the_right_players_through() {
        let fresh = player_with(&[]);
        let go = |script, player: &Player| warp_of(&portal(script, player).unwrap());
        assert_eq!(go("enterAchter", &fresh), Some((BOWMAN_SCHOOL, 9)));
        assert_eq!(go("enterMagiclibrar", &fresh), Some((MAGIC_LIBRARY, 8)));
        assert_eq!(
            go("balogTemple", &fresh),
            Some((UNDERGROUND_TEMPLE_STAIRS, 2))
        );
        assert_eq!(
            go("enterNepenthes", &fresh),
            Some((NEGLECTED_STROLLING_PATH, 2))
        );

        let with = |quests, item: Option<i32>| {
            let mut player = player_with(quests);
            if let Some(item) = item {
                player
                    .inventory
                    .insert(InventoryType::Etc, 1, game::Item::stack(item, 1));
            }
            player
        };
        assert!(pink(&portal("ariant_castle", &fresh).unwrap()));
        assert_eq!(
            go("ariant_castle", &with(&[], Some(PALACE_ENTRY_PASS))),
            Some((CASTLE_GARDEN, 5))
        );

        // Ereve's forest is open, except to Master of Disguise players
        // without a permit.
        assert_eq!(go("enterDisguise0", &fresh), Some((TRAINING_FOREST, 3)));
        let disguise = [(20303, QuestState::Started)];
        assert!(pink(
            &portal("enterDisguise0", &with(&disguise, None)).unwrap()
        ));
        assert_eq!(
            go(
                "enterDisguise0",
                &with(&disguise, Some(INVESTIGATION_PERMIT))
            ),
            Some((TRAINING_FOREST, 3))
        );

        assert!(pink(&portal("enterWitch", &fresh).unwrap()));
        let done = QuestState::Completed;
        for (quests, cave) in [
            (&[(STOLEN_EGG, done)][..], 924010000),
            (&[(STOLEN_EGG, done), (20406, done)][..], 924010100),
            (
                &[(STOLEN_EGG, done), (20406, done), (20407, done)][..],
                924010200,
            ),
        ] {
            assert_eq!(go("enterWitch", &with(quests, None)), Some((cave, 1)));
        }

        assert_eq!(
            go(
                "TD_MC_enterboss1",
                &player_with(&[(2330, QuestState::Started)])
            ),
            Some((WEDDING_HALL_ENTRANCE, 1))
        );

        // Magatia's secret doors open to each lab's side of the basement.
        assert!(pink(&portal("secretDoor", &fresh).unwrap()));
        let mut player = player_with(&[(VERIFYING_THE_PASSWORD, QuestState::Started)]);
        player.stats.map_id = ZENUMIST_HALLWAY;
        assert_eq!(go("secretDoor", &player), Some((SECRET_BASEMENT_PATH, 2)));
        player.stats.map_id = 261020200;
        assert_eq!(go("secretDoor", &player), Some((SECRET_BASEMENT_PATH, 1)));
    }

    #[test]
    fn rien_keeps_an_aran_in_training_in_town() {
        let forest = || Some(through(Effect::WarpToPortal(DANGEROUS_FOREST, 2)));
        assert_eq!(portal("rienTutor8", &player_with(&[])), forest());
        let mut lines = std::collections::HashSet::new();
        for (quest, _) in FITNESS_TRAINING {
            let mut player = player_with(&[(quest, QuestState::Started)]);
            // Only an Aran beginner is held back.
            assert_eq!(portal("rienTutor8", &player), forest());
            player.stats.job = ARAN_BEGINNER;
            let held = portal("rienTutor8", &player).unwrap();
            let [Effect::InfoText(text)] = held.effects.as_slice() else {
                panic!("{held:?}");
            };
            assert!(!text.is_empty());
            lines.insert(text.clone());
        }
        // Each training names its own quarry.
        assert_eq!(lines.len(), 3);
        let mut done = player_with(&[(21015, QuestState::Completed)]);
        done.stats.job = ARAN_BEGINNER;
        assert_eq!(portal("rienTutor8", &done), forest());
    }

    #[test]
    fn the_cursed_forest_opens_at_night_to_arwens_helpers() {
        let at = |quests: &[(i16, QuestState)], hour| cursed_forest(&player_with(quests), hour);
        let started = QuestState::Started;
        let forest = |forest| through(Effect::WarpToPortal(forest, named(forest, "out00")));
        assert!(pink(&at(&[], 20)));
        assert!(pink(&at(&[(2225, started)], 20)));
        assert!(pink(&at(&[(DEFEAT_FAUST, started)], 20)));
        for quests in [&[(2224, started)][..], &[(2226, started)][..]] {
            for hour in [0, 6, 17, 23] {
                assert_eq!(at(quests, hour), forest(CURSED_FOREST), "{hour}");
            }
            for hour in [7, 12, 16] {
                assert!(pink(&at(quests, hour)), "{hour}");
            }
        }
        let done = &[(DEFEAT_FAUST, QuestState::Completed)];
        assert_eq!(at(done, 3), forest(CURSED_FOREST_AFTER_FAUST));
        assert!(pink(&at(done, 9)));
        assert_eq!(named(CURSED_FOREST, "out00"), 1);
        assert_eq!(named(CURSED_FOREST_AFTER_FAUST, "out00"), 1);
        // The closed and the daytime lines differ.
        assert_ne!(at(&[], 20), at(&[(2224, started)], 12));
    }

    #[test]
    fn the_elevator_door_opens_onto_the_waiting_room_while_the_car_is_there() {
        const MINUTE: i64 = 60_000;
        // The car leaves the 99th floor on the hour and the 2nd floor two
        // minutes later; each boards for the minute before.
        let hour = 5 * 60 * MINUTE;
        let door = |map_id, at| elevator(map_id, at);
        let waiting = |room| through(Effect::WarpToPortal(room, 0));
        assert_eq!(door(222020100, hour + 90_000), waiting(222020110));
        assert_eq!(door(222020200, hour - 30_000), waiting(222020210));
        for minute in [0, 2, 3] {
            assert!(pink(&door(222020100, hour + minute * MINUTE + 30_000)));
        }
        for minute in [0, 1, 2] {
            assert!(pink(&door(222020200, hour + minute * MINUTE + 30_000)));
        }
        assert!(pink(&door(103000100, hour + 90_000)));
        assert_eq!(
            portal_id(222020100, |portal| portal.script.as_deref()
                == Some("elevator")),
            5
        );
        assert_eq!(
            portal_id(222020200, |portal| portal.script.as_deref()
                == Some("elevator")),
            5
        );
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
