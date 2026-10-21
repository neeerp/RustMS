//! Native NPC and quest scripts. Each is a list of pages with effects on its
//! answers; the runtime applies the effects (and persists them with any
//! quest change) before showing the next page. Text comes from the content
//! pack (`crate::content`), with committed placeholders. Lua is deferred;
//! these keep the effect-returning shape a scripting runtime would have.

use crate::content;
use crate::conti::{self, Route, State};
use crate::dialogue::{DialoguePage, Prompt};
use crate::events::{self, Cloto, Entry, EventCommand, EventKind};
use crate::scripts::Effect;
use game::guild::{self, Guild};
use game::{InventoryType, Player, QuestState};
use game_data::server_content::keys;
use rand::seq::SliceRandom;
use rand::Rng;

/// Roger's Apple, the tutorial's lesson on items.
pub const ROGERS_APPLE_QUEST: i16 = 1021;
const ROGERS_APPLE: i32 = 2010007;
const APPLE: i32 = 2010000;
const GREEN_APPLE: i32 = 2010009;
const ROGER: i32 = 2000;
const ROBIN: i32 = 2003;
const MAPLE_ADMINISTRATOR: i32 = 2007;
const SHANKS: i32 = 22000;
/// Heracle, at the guild headquarters. The runtime loads the player's
/// guild for his script (`NpcScript::heracle`).
pub const HERACLE: i32 = 2010007;
/// Lea, beside him, who makes guild emblems; her script needs the guild
/// too (`NpcScript::lea`).
pub const LEA: i32 = 2010008;
/// Lakelis, who sends parties into Kerning City's party quest; her script
/// needs to know whether the party may go (`NpcScript::lakelis`).
pub const LAKELIS: i32 = 9020000;
/// Nella, who lets players out of the party quest and back to town.
const NELLA: i32 = 9020002;
/// Cloto, who sets each stage of Kerning City's party quest.
pub const CLOTO: i32 = 9020001;
/// Jeff, at the way down from Ice Valley II.
const JEFF: i32 = 2030000;
/// Sharp Cliff I and its `under00` portal, the top of the dungeon below.
const SHARP_CLIFF: (i32, u8) = (211040300, 5);
const JEFF_LEVEL: u8 = 50;
/// The Orihalcon Hammer, which takes Jeff's holder straight to Ice Valley.
const ORIHALCON_HAMMER: i32 = 4031450;
const ICE_VALLEY: (i32, u8) = (921100100, 1);
/// The Time Gate at the edge of the Tera Forest.
const TIME_GATE: i32 = 2083006;
/// Andy the Time Traveler, which wakes the Time Gate.
const TIME_GATE_QUEST: i16 = 3718;
/// Each era's map (arriving at its `left00`, id 1) and the Guardian Nex
/// quest that opens it, once those before it are done.
const ERAS: [(i32, i16); 6] = [
    (240070100, 3719),
    (240070200, 3724),
    (240070300, 3730),
    (240070400, 3736),
    (240070500, 3742),
    (240070600, 3748),
];
/// The Mysterious Statue in the Sleepywood forest and the Crumbling Statue
/// at each of the Deep Forest's steps.
const MYSTERIOUS_STATUE: i32 = 1061006;
const CRUMBLING_STATUE: i32 = 1061007;
const STATUE_FOREST: i32 = 105040300;
/// The Deep Forest's steps a John quest opens: each quest, taken or done,
/// opens one more.
const DEEP_FOREST: [(i32, i16); 3] = [(105040310, 2052), (105040312, 2053), (105040314, 2054)];
/// Shane in Ellinia, who lets players into the Forest of Patience, and
/// Louis inside, who sees them back.
const SHANE: i32 = 1032003;
const LOUIS: i32 = 1032004;
const SHANE_LEVEL: u8 = 25;
const SHANE_FARE: i32 = 5000;
const ELLINIA: i32 = 101000000;
/// The Forest of Patience's first and third steps, each the start of one
/// of Sabitrama's herb quests (2050, 2051).
const PATIENCE_STEP_1: i32 = 101000100;
const PATIENCE_STEP_3: i32 = 101000102;
const LITH_HARBOR: i32 = 104_000_000;
/// Lucas's recommendation letter, which pays for Shanks's ship.
const RECOMMENDATION_LETTER: i32 = 4031801;
const SHIP_FARE: i32 = 150;
const SHIP_LEVEL: u8 = 7;
/// The stations' crews (report 183-routes): ticket sellers, ushers, and
/// the crew who let passengers off before departure.
const JOEL: i32 = 1032007;
const CHERRY: i32 = 1032008;
const PURIN: i32 = 1032009;
const AGATHA: i32 = 2012000;
const RINI: i32 = 2012001;
const ERIN: i32 = 2012002;
/// Each town's booth sells the ticket back to Orbis; Agatha, in Orbis,
/// sells one for each ship.
const SELLERS: [(i32, &Route); 4] = [
    (JOEL, &conti::TO_ORBIS),
    (2040000, &conti::LUDIBRIUM_TO_ORBIS),
    (2082000, &conti::LEAFRE_TO_ORBIS),
    (2102002, &conti::ARIANT_TO_ORBIS),
];
/// Agatha's tickets, with the destination her menu names.
const AGATHA_ROUTES: [(&Route, &str); 4] = [
    (&conti::TO_ELLINIA, "Ellinia"),
    (&conti::TO_LUDIBRIUM, "Ludibrium"),
    (&conti::TO_LEAFRE, "Leafre"),
    (&conti::TO_ARIANT, "Ariant"),
];
const USHERS: [(i32, &Route); 8] = [
    (CHERRY, &conti::TO_ORBIS),
    (RINI, &conti::TO_ELLINIA),
    (2012013, &conti::TO_LUDIBRIUM),
    (2041000, &conti::LUDIBRIUM_TO_ORBIS),
    (2012021, &conti::TO_LEAFRE),
    (2082001, &conti::LEAFRE_TO_ORBIS),
    (2012025, &conti::TO_ARIANT),
    (2102000, &conti::ARIANT_TO_ORBIS),
];
/// The waiting rooms' guides. Rosey (2041001) has both of Ludibrium's
/// train's, so the player's map picks the route.
const WAITING_ROOM_GUIDES: [i32; 7] = [PURIN, ERIN, 2041001, 2012022, 2012024, 2082002, 2102001];
/// Orbis Station's guide, and the passages to the platforms: (vehicle,
/// destination, passage), as `station_in` names them. The passages to Mu
/// Lung's crane and Ereve's ferry are left out until those run.
const ISA: i32 = 2012006;
const ORBIS_TUNNEL: i32 = 200000110;
const ISA_PLATFORMS: [(&str, &str, i32); 4] = [
    ("Ship", "Victoria Island", ORBIS_TUNNEL),
    ("Ship", "Ludibrium", 200000120),
    ("Ship", "Leafre", 200000130),
    ("Genie", "Ariant", 200000150),
];
/// Victoria Island's Regular Cabs (`taxi1`-`taxi6`), each with its fares
/// to the other towns (Cosmic's v83 fares). Beginners pay a tenth.
const REGULAR_CABS: [(i32, [(i32, i32); 5]); 6] = [
    (
        1002007,
        [
            (100000000, 1000),
            (102000000, 1000),
            (101000000, 800),
            (103000000, 1000),
            (120000000, 800),
        ],
    ),
    (
        1012000,
        [
            (104000000, 1000),
            (102000000, 1000),
            (101000000, 800),
            (103000000, 1000),
            (120000000, 800),
        ],
    ),
    (
        1022001,
        [
            (104000000, 1000),
            (100000000, 1000),
            (101000000, 800),
            (103000000, 1000),
            (120000000, 800),
        ],
    ),
    (
        1032000,
        [
            (104000000, 1000),
            (102000000, 1000),
            (100000000, 1000),
            (103000000, 1000),
            (120000000, 800),
        ],
    ),
    (
        1052016,
        [
            (104000000, 1000),
            (102000000, 1000),
            (100000000, 1000),
            (101000000, 800),
            (120000000, 800),
        ],
    ),
    (
        1092014,
        [
            (104000000, 1000),
            (102000000, 1000),
            (100000000, 1000),
            (101000000, 800),
            (103000000, 1000),
        ],
    ),
];
/// The VIP Cabs at Lith Harbor and Ellinia (`mTaxi`), to Ant Tunnel Park.
const VIP_CABS: [i32; 2] = [1002004, 1032005];
const ANT_TUNNEL_PARK: i32 = 105070001;
const VIP_FARE: i32 = 10000;
/// Ossyria's Danger Zone Taxi (`ossyria_taxi`): from each town it stands
/// in, (town, destination, its portal, fare).
const DANGER_ZONE_TAXI: i32 = 2023000;
const DANGER_ZONE_ROUTES: [(i32, i32, u8, i32); 4] = [
    (211000000, 211040200, 0, 10000),
    (220000000, 220050300, 1, 25000),
    (221000000, 220000000, 0, 25000),
    (240000000, 240030000, 0, 65000),
];
/// The Camel Cab (`nihal_taxi`) between Ariant's north gate and Magatia's
/// Sahel 1, each way to the town at the other end.
const CAMEL_CAB: i32 = 2110005;
const CAMEL_CAB_FARE: i32 = 1500;
const OUTSIDE_ARIANT: i32 = 260020000;
const MAGATIA: i32 = 261000000;
const ARIANT: i32 = 260000000;
/// The subway's ticket gates: Kerning City's (`subway_in`, whose menu also
/// offers Kerning Square and the Construction Site, left out until those
/// run) and New Leaf City's (`NLC_Move`).
const SUBWAY_GATES: [(i32, &Route); 2] = [
    (1052007, &conti::TO_NLC),
    (9201068, &conti::TO_KERNING_CITY),
];
/// Bell (`NLC_ticketing`) stands on both platforms and in both waiting
/// rooms.
const BELL: i32 = 9201057;
/// What Bell says about each line: her menu, its one choice, the ask, and
/// the way back out of the line's waiting room.
struct SubwayLine {
    route: &'static Route,
    menu: &'static str,
    choice: &'static str,
    ask: &'static str,
    leave: &'static str,
}
const SUBWAY_LINES: [SubwayLine; 2] = [
    SubwayLine {
        route: &conti::TO_NLC,
        menu: keys::SUBWAY_TO_NLC_MENU,
        choice: keys::SUBWAY_TO_NLC_CHOICE,
        ask: keys::SUBWAY_TO_NLC_ASK,
        leave: keys::SUBWAY_TO_NLC_LEAVE,
    },
    SubwayLine {
        route: &conti::TO_KERNING_CITY,
        menu: keys::SUBWAY_TO_KERNING_CITY_MENU,
        choice: keys::SUBWAY_TO_KERNING_CITY_CHOICE,
        ask: keys::SUBWAY_TO_KERNING_CITY_ASK,
        leave: keys::SUBWAY_TO_KERNING_CITY_LEAVE,
    },
];
/// The NLC Taxi between New Leaf City and the Haunted House's grounds,
/// either way (Cosmic's fare).
const NLC_TAXI: i32 = 9201056;
const NLC_TAXI_FARE: i32 = 15000;
const NEW_LEAF_CITY: i32 = 600000000;
const HAUNTED_HOUSE_GROUNDS: i32 = 682000000;
/// The Dolphin Taxi (`aqua_taxi`) at the Aquarium and Herb Town's pier:
/// down to the Sharp Unknown for a fare or a coupon, or across to the
/// other end.
const DOLPHIN: i32 = 2060009;
const DOLPHIN_TAXI_COUPON: i32 = 4031242;
const AQUARIUM: i32 = 230000000;
const HERB_TOWN_PIER: i32 = 251000100;
const SHARP_UNKNOWN: i32 = 230030200;
const SHARP_UNKNOWN_FARE: i32 = 1000;
const DOLPHIN_FARE: i32 = 10000;
/// Henesys's hair salon (`hair_henesys1`): a cut or a dye of the player's
/// choosing, for a coupon each.
const NATALIE: i32 = 1012103;
const HAIRCUT_COUPON: i32 = 5150001;
const DYE_COUPON: i32 = 5151001;
/// Mar the Fairy, who pours the Water of Life on dolls.
const MAR_THE_FAIRY: i32 = 1032102;
const NATALIE_HAIRS: [[i32; 7]; 2] = [
    [30060, 30140, 30200, 30210, 30310, 33040, 33100],
    [31150, 31300, 31350, 31700, 31740, 34050, 34110],
];

/// The second job's test: the examiner's proctor gives this for the marbles,
/// and the instructor takes it for the job.
const PROOF_OF_A_HERO: i32 = 4031012;
const DARK_MARBLE: i32 = 4031013;
const MARBLES_NEEDED: i32 = 30;

/// A job instructor: the first job, its text key, and the weapons,
/// ammunition and bag rows that come with it (upstream `fighter`,
/// `magician`, `bowman`, `rogue`, `kairinT`); then the second jobs and the
/// examiner's test that earns them.
struct Instructor {
    npc_id: i32,
    job: i16,
    text: &'static str,
    items: &'static [(i32, i16)],
    rows: &'static [InventoryType],
    second_jobs: &'static [i16],
    /// `None` for Kyrin, whose quests hold the test (`PIRATE_TESTS`).
    test: Option<Test>,
}

/// The second job's test: the instructor's letter, the examiner (and the
/// examiner's map and the portal back to them), the test map and its
/// proctor.
struct Test {
    letter: i32,
    examiner: i32,
    examiner_map: i32,
    examiner_portal: u8,
    test_map: i32,
    proctor: i32,
}

const INSTRUCTORS: [Instructor; 5] = [
    // Dances with Balrog: a sword and a row in every bag.
    Instructor {
        npc_id: 1022000,
        job: 100,
        text: "warrior",
        items: &[(1302077, 1)],
        rows: &[
            InventoryType::Equip,
            InventoryType::Use,
            InventoryType::Setup,
            InventoryType::Etc,
        ],
        second_jobs: &[110, 120, 130],
        test: Some(Test {
            letter: 4031008,
            examiner: 1072000,
            examiner_map: 102020300,
            examiner_portal: 2,
            test_map: 108000300,
            proctor: 1072004,
        }),
    },
    // Grendel the Really Old: a wand.
    Instructor {
        npc_id: 1032001,
        job: 200,
        text: "magician",
        items: &[(1372043, 1)],
        rows: &[],
        second_jobs: &[210, 220, 230],
        test: Some(Test {
            letter: 4031009,
            examiner: 1072001,
            examiner_map: 101020000,
            examiner_portal: 1,
            test_map: 108000200,
            proctor: 1072005,
        }),
    },
    // Athena Pierce: a bow and arrows.
    Instructor {
        npc_id: 1012100,
        job: 300,
        text: "bowman",
        items: &[(1452051, 1), (2060000, 6000)],
        rows: &[InventoryType::Equip, InventoryType::Use],
        second_jobs: &[310, 320],
        test: Some(Test {
            letter: 4031010,
            examiner: 1072002,
            examiner_map: 106010000,
            examiner_portal: 9,
            test_map: 108000100,
            proctor: 1072006,
        }),
    },
    // The Dark Lord: a claw, a dagger and two sets of throwing stars.
    Instructor {
        npc_id: 1052001,
        job: 400,
        text: "thief",
        items: &[(1472061, 1), (1332063, 1), (2070015, 1000), (2070015, 1000)],
        rows: &[InventoryType::Equip, InventoryType::Etc],
        second_jobs: &[410, 420],
        test: Some(Test {
            letter: 4031011,
            examiner: 1072003,
            examiner_map: 102040000,
            examiner_portal: 9,
            test_map: 108000400,
            proctor: 1072007,
        }),
    },
    // Kyrin: a knuckle, a gun and two sets of bullets.
    Instructor {
        npc_id: KYRIN,
        job: 500,
        text: "pirate",
        items: &[(1482014, 1), (1492014, 1), (2330006, 600), (2330006, 600)],
        rows: &[InventoryType::Equip, InventoryType::Etc],
        second_jobs: &[510, 520],
        test: None,
    },
];

const KYRIN: i32 = 1090000;
const KYRIN_MAP: i32 = 120000101;
/// Inside Kyrin's tests (`inside_pirate`).
const PIRATE_PROCTOR: i32 = 1072008;
const CRYSTALS_NEEDED: i32 = 15;

/// Kyrin's second-job tests: a quest for each job, started and completed
/// with her as the client runs quests, the test map she sends its holder
/// to, and the crystals its Octopirates drop.
struct PirateTest {
    job: i16,
    quest: i16,
    map: i32,
    crystal: i32,
}

const PIRATE_TESTS: [PirateTest; 2] = [
    // Brawler: Octopirates only Flash Fist hurts.
    PirateTest {
        job: 510,
        quest: 2191,
        map: 108000502,
        crystal: 4031856,
    },
    // Gunslinger: Octopirates only Double Shot hurts.
    PirateTest {
        job: 520,
        quest: 2192,
        map: 108000501,
        crystal: 4031857,
    },
];

/// The Explorer third job: a chief in El Nath sends a level-70 second job
/// to their instructor, who sends them through the Door of Dimension to
/// beat their clone; the clone's charm earns the Necklace of Strength, and
/// the Holy Stone's quiz the Necklace of Wisdom, which the chief takes for
/// the job. The steps between are script markers, as upstream servers keep
/// them: the chief's assignment, the instructor's, the Holy Stone's.
struct ThirdJob {
    /// The line's first job.
    job: i16,
    text: &'static str,
    chief: i32,
    /// The hunting map with the line's Door of Dimension.
    door_map: i32,
    /// Behind the door, and the clone's room after it.
    entry_map: i32,
    boss_map: i32,
}

const THIRD_JOBS: [ThirdJob; 5] = [
    // Tylus.
    ThirdJob {
        job: 100,
        text: "warrior",
        chief: 2020008,
        door_map: 105070001,
        entry_map: 108010300,
        boss_map: 108010301,
    },
    // Robeira.
    ThirdJob {
        job: 200,
        text: "magician",
        chief: 2020009,
        door_map: 100040106,
        entry_map: 108010200,
        boss_map: 108010201,
    },
    // Rene.
    ThirdJob {
        job: 300,
        text: "bowman",
        chief: 2020010,
        door_map: 105040305,
        entry_map: 108010100,
        boss_map: 108010101,
    },
    // Arec.
    ThirdJob {
        job: 400,
        text: "thief",
        chief: 2020011,
        door_map: 107000402,
        entry_map: 108010400,
        boss_map: 108010401,
    },
    // Pedro.
    ThirdJob {
        job: 500,
        text: "pirate",
        chief: 2020013,
        door_map: 105070200,
        entry_map: 108010500,
        boss_map: 108010501,
    },
];

/// The Explorer fourth job: a level-120 third job who has finished their
/// line's last quest in Leafre (bringing the Pentagon and the Star) asks
/// the line's instructor there to advance them.
struct FourthJob {
    /// The line's first job.
    job: i16,
    text: &'static str,
    instructor: i32,
    quest: i16,
}

const FOURTH_JOBS: [FourthJob; 5] = [
    // Harmonia; "Hero's Quality".
    FourthJob {
        job: 100,
        text: "warrior",
        instructor: 2081100,
        quest: 6904,
    },
    // Gritto.
    FourthJob {
        job: 200,
        text: "magician",
        instructor: 2081200,
        quest: 6914,
    },
    // Legor.
    FourthJob {
        job: 300,
        text: "bowman",
        instructor: 2081300,
        quest: 6924,
    },
    // Hellin.
    FourthJob {
        job: 400,
        text: "thief",
        instructor: 2081400,
        quest: 6934,
    },
    // Samuel.
    FourthJob {
        job: 500,
        text: "pirate",
        instructor: 2081500,
        quest: 6944,
    },
];

/// [Skill Book] Maple Hero, which teaches every fourth job Maple Warrior;
/// the instructor gives one with the advancement (upstream).
const MAPLE_HERO: i32 = 2280003;

/// The chief has assigned the test (upstream `JB3`).
const CHIEF_ASSIGNED: i16 = 32100;
/// The instructor has sent the player to the door (`JBP`).
const INSTRUCTOR_ASSIGNED: i16 = 32101;
/// The chief has sent the player to the Holy Stone (`JBQ`).
const HOLY_STONE_ASSIGNED: i16 = 32102;
const DOOR_OF_DIMENSION: i32 = 1061009;
const SPARKLING_CRYSTAL: i32 = 1061010;
const HOLY_STONE: i32 = 2030006;
const SPIRUNA: i32 = 2032001;
/// Dropped by the clone, for the player sent to fight it.
const BLACK_CHARM: i32 = 4031059;
const NECKLACE_OF_STRENGTH: i32 = 4031057;
const NECKLACE_OF_WISDOM: i32 = 4031058;
const DARK_CRYSTAL: i32 = 4005004;
const DARK_CRYSTAL_ORE: i32 = 4004004;
/// Spiruna refines for those who finished her quest (upstream).
const SPIRUNA_QUEST: i16 = 3034;
const ORE_PER_CRYSTAL: i16 = 10;
const REFINING_FEE: i32 = 500_000;
/// Questions asked of each challenger, any wrong answer failing.
const QUIZ_LENGTH: usize = 5;
/// How long a challenger has with the clone (upstream).
const CLONE_TIME_LIMIT: u32 = 20 * 60;

/// The player's answer to the current prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptInput {
    Next,
    Previous,
    Yes,
    No,
    Select(i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Say(String),
    /// Yes applies `on_yes` and moves on; no ends the conversation, saying
    /// `on_no` first if there is one.
    Ask {
        text: String,
        on_yes: Vec<Effect>,
        on_no: Option<String>,
    },
    /// Each choice, a label and its pages, replaces the menu.
    Menu {
        text: String,
        choices: Vec<(String, Vec<Page>)>,
    },
    /// A choice of styles (`Prompt::Style`), each with the effects picking
    /// it applies before moving on.
    Style {
        text: String,
        choices: Vec<(i32, Vec<Effect>)>,
    },
    /// The last page of a menu choice that fails: said, and the script's
    /// end effects are dropped.
    End(String),
    /// The conversation ends here without another page, as when a menu
    /// choice warps at once: `effects`, then the script's end effects.
    Done(Vec<Effect>),
    /// Said as `effects` apply, as an NPC who hands something over while
    /// speaking. If they are refused, the script's `refused` text is said
    /// instead. Previous never goes back to it (`has_previous`), so they
    /// apply once.
    Act {
        text: String,
        effects: Vec<Effect>,
    },
}

/// What to do after an answer: apply `effects`, then show `prompt` (or end
/// the conversation if there is none).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Step {
    pub prompt: Option<Prompt>,
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcScript {
    npc_id: i32,
    pages: Vec<Page>,
    page: usize,
    /// Applied when the player reads past the last page.
    on_end: Vec<Effect>,
    /// Said instead of moving on when the items an answer gives don't fit.
    inventory_full: Option<String>,
    /// Said when the player no longer may have what an answer applies.
    refused: Option<String>,
}

impl NpcScript {
    fn new(npc_id: i32, pages: Vec<Page>) -> Self {
        Self {
            npc_id,
            pages,
            page: 0,
            on_end: Vec::new(),
            inventory_full: None,
            refused: None,
        }
    }

    /// Heracle's script as it plays for `player`, who is in `guild`, if it
    /// still exists.
    pub fn heracle(player: &Player, guild: Option<&Guild>) -> Self {
        heracle(player, guild)
    }

    /// Lea's script as it plays for `player`, who is in `guild`, if it
    /// still exists.
    pub fn lea(player: &Player, guild: Option<&Guild>) -> Self {
        lea(player, guild)
    }

    /// Lakelis's script, for a player whose party may or may not go into
    /// the party quest.
    pub fn lakelis(entry: Entry) -> Self {
        lakelis(entry)
    }

    /// Cloto's script on the party quest's stages, for `player`, who
    /// stands with her as `view`. A leader answering a puzzle gets
    /// `cloto_answer` instead.
    pub fn cloto(view: Cloto, player: &Player) -> Self {
        cloto(view, player)
    }

    /// Cloto to the leader answering puzzle stage `stage`: whether the team
    /// stands on the `right` areas, which clears the stage.
    pub fn cloto_answer(stage: u8, right: bool) -> Self {
        let page = if right {
            Page::Act {
                text: text(keys::KPQ_CLOTO_AREAS_RIGHT),
                effects: vec![Effect::EventCommand(EventCommand::ClearStage(stage))],
            }
        } else {
            Page::Say(text(keys::KPQ_CLOTO_AREAS_WRONG))
        };
        NpcScript::new(CLOTO, vec![page])
    }

    /// The script an NPC runs when clicked (`Npc.nx info/script`), as it
    /// plays for `player`. Heracle, Lea and Lakelis are not among them.
    pub fn for_npc(npc_id: i32, player: &Player) -> Option<Self> {
        if let Some(instructor) = INSTRUCTORS.iter().find(|i| i.npc_id == npc_id) {
            let stats = &player.stats;
            let sent = marked(player, CHIEF_ASSIGNED) || marked(player, INSTRUCTOR_ASSIGNED);
            if sent && third_job_line(player).is_some_and(|line| line.job == instructor.job) {
                return Some(third_job_instructor(instructor, player));
            }
            return Some(match &instructor.test {
                _ if stats.job != instructor.job || stats.level < 30 => {
                    first_job(instructor, player)
                }
                Some(test) => second_job(instructor, test, player),
                None => pirate_second_job(player),
            });
        }
        for instructor in &INSTRUCTORS {
            let Some(test) = &instructor.test else {
                continue;
            };
            if test.examiner == npc_id {
                return Some(examiner(instructor, test, player));
            }
            if test.proctor == npc_id {
                return Some(proctor(instructor, test, player));
            }
        }
        if let Some(line) = THIRD_JOBS.iter().find(|line| line.chief == npc_id) {
            return Some(chief(line, player));
        }
        if let Some(line) = FOURTH_JOBS.iter().find(|line| line.instructor == npc_id) {
            return Some(fourth_job_instructor(line, player));
        }
        if let Some((_, route)) = SELLERS.iter().find(|(seller, _)| *seller == npc_id) {
            return Some(ticket_seller(npc_id, route, player));
        }
        if let Some((_, route)) = SUBWAY_GATES.iter().find(|(gate, _)| *gate == npc_id) {
            return Some(subway_gate(npc_id, route, player, conti::now()));
        }
        if let Some((_, route)) = USHERS.iter().find(|(usher, _)| *usher == npc_id) {
            return Some(usher(npc_id, route, player, conti::now()));
        }
        if WAITING_ROOM_GUIDES.contains(&npc_id) {
            return conti::waiting_in(player.stats.map_id).map(|route| leave_ship(npc_id, route));
        }
        if let Some((_, fares)) = REGULAR_CABS.iter().find(|(cab, _)| *cab == npc_id) {
            return Some(regular_cab(npc_id, fares, player));
        }
        if VIP_CABS.contains(&npc_id) {
            return Some(vip_cab(npc_id, player));
        }
        match npc_id {
            DOOR_OF_DIMENSION => Some(door_of_dimension(player)),
            SPARKLING_CRYSTAL => sparkling_crystal(player),
            HOLY_STONE => Some(holy_stone(player, &mut rand::thread_rng())),
            SPIRUNA => Some(spiruna(player)),
            PIRATE_PROCTOR => Some(pirate_proctor(player)),
            ROBIN => Some(robin()),
            SHANKS => Some(shanks(player)),
            AGATHA => Some(agatha(player)),
            ISA => Some(isa()),
            DANGER_ZONE_TAXI => danger_zone_taxi(player),
            CAMEL_CAB => Some(camel_cab(player)),
            BELL => bell(player),
            NLC_TAXI => Some(nlc_taxi(player)),
            DOLPHIN => Some(dolphin(player)),
            NATALIE => Some(natalie(player)),
            NELLA => nella(player),
            JEFF => Some(jeff(player)),
            TIME_GATE => Some(time_gate(player)),
            SHANE => Some(shane(player)),
            LOUIS => Some(Self::new(
                LOUIS,
                vec![Page::Ask {
                    text: text(keys::LOUIS_ASK),
                    on_yes: vec![Effect::WarpToPortal(ELLINIA, 0)],
                    on_no: None,
                }],
            )),
            MYSTERIOUS_STATUE => Some(mysterious_statue(player)),
            CRUMBLING_STATUE => Some(Self::new(
                CRUMBLING_STATUE,
                vec![Page::Ask {
                    text: text(keys::CRUMBLING_ASK),
                    on_yes: vec![Effect::WarpToPortal(STATUE_FOREST, 0)],
                    on_no: None,
                }],
            )),
            MAPLE_ADMINISTRATOR => Some(Self::new(
                MAPLE_ADMINISTRATOR,
                vec![Page::Ask {
                    text: text(keys::SKIP_ASK),
                    on_yes: vec![Effect::Warp(LITH_HARBOR)],
                    on_no: Some(text(keys::SKIP_DECLINE)),
                }],
            )),
            _ => None,
        }
    }

    /// Mar the Fairy's offer when a Water of Life is used: pick a doll,
    /// and the water wakes it for the days the item gives (`info/life`).
    /// `None` if the player holds no Water of Life.
    pub fn water_of_life(player: &Player) -> Option<Self> {
        let water = player
            .inventory
            .items(InventoryType::Cash)
            .map(|(_, item)| item.item_id)
            .find(|&item_id| game::pet::is_water_of_life(item_id))?;
        let days = crate::items::info(water).map_or(0, |info| info.pet_life);
        let choices: Vec<_> = player
            .inventory
            .items(InventoryType::Cash)
            .filter(|(_, item)| game::pet::is_doll(item))
            .filter_map(|(_, item)| Some((item.cash_serial?, item.pet.as_ref()?)))
            .map(|(serial, pet)| {
                let name = pet.name.as_str();
                let says =
                    |key| content::text().get(key, &[("name", name), ("days", &days.to_string())]);
                let pages = vec![
                    Page::Ask {
                        text: says(keys::WATER_OF_LIFE_ASK),
                        on_yes: vec![
                            Effect::TakeItem {
                                item_id: water,
                                quantity: 1,
                            },
                            Effect::RevivePet { serial, days },
                        ],
                        on_no: None,
                    },
                    Page::Say(says(keys::WATER_OF_LIFE_DONE)),
                ];
                (format!("{name} (Lv. {})", pet.level), pages)
            })
            .collect();
        let page = if choices.is_empty() {
            Page::Say(text(keys::WATER_OF_LIFE_NO_DOLL))
        } else {
            Page::Menu {
                text: text(keys::WATER_OF_LIFE_PICK),
                choices,
            }
        };
        Some(Self::new(MAR_THE_FAIRY, vec![page]))
    }

    /// A quest's start or end script (`Check.img startscript`/`endscript`),
    /// as it plays for `player`.
    pub fn for_quest(script: &str, player: &Player) -> Option<Self> {
        match script {
            "q1021s" => Some(roger_start(player)),
            "q1021e" => Some(roger_end(player)),
            _ => None,
        }
    }

    pub fn npc_id(&self) -> i32 {
        self.npc_id
    }

    /// The first prompt.
    pub fn begin(&self) -> Step {
        self.step(Vec::new())
    }

    /// Advance with the player's answer. `None` means the answer did not fit
    /// the prompt (the runtime ends the conversation).
    pub fn respond(&mut self, input: ScriptInput) -> Option<Step> {
        match (&self.pages[self.page], input) {
            (Page::Say(_) | Page::Act { .. } | Page::End(_), ScriptInput::Next) => {
                Some(self.advance(Vec::new()))
            }
            (Page::Say(_), ScriptInput::Previous) if self.has_previous() => {
                self.page -= 1;
                Some(self.begin())
            }
            (Page::Ask { on_yes, .. }, ScriptInput::Yes) => {
                let effects = on_yes.clone();
                Some(self.advance(effects))
            }
            (Page::Ask { on_no, .. }, ScriptInput::No) => Some(match on_no.clone() {
                Some(text) => self.replace(text),
                None => Step::default(),
            }),
            (Page::Menu { choices, .. }, ScriptInput::Select(choice)) => {
                let (_, pages) = choices.get(usize::try_from(choice).ok()?)?;
                self.pages = pages.clone();
                self.page = 0;
                if matches!(self.pages.last(), Some(Page::End(_))) {
                    self.on_end.clear();
                }
                Some(self.begin())
            }
            (Page::Style { choices, .. }, ScriptInput::Select(choice)) => {
                let (_, effects) = choices.get(usize::try_from(choice).ok()?)?;
                let effects = effects.clone();
                Some(self.advance(effects))
            }
            _ => None,
        }
    }

    /// The items the last step gave don't fit: say so instead, if the
    /// script has words for it.
    pub fn inventory_full(&mut self) -> Option<Step> {
        let text = self.inventory_full.take()?;
        Some(self.replace(text))
    }

    /// An answer's effects were refused: say so instead, if the script has
    /// words for it.
    pub fn refused(&mut self) -> Option<Step> {
        let text = self.refused.take()?;
        Some(self.replace(text))
    }

    /// End with a single page and no further effects.
    fn replace(&mut self, text: String) -> Step {
        self.pages = vec![Page::Say(text)];
        self.page = 0;
        self.on_end.clear();
        self.begin()
    }

    fn advance(&mut self, mut effects: Vec<Effect>) -> Step {
        self.page += 1;
        if self.page < self.pages.len() {
            return self.step(effects);
        }
        effects.append(&mut self.on_end);
        Step {
            prompt: None,
            effects,
        }
    }

    fn has_previous(&self) -> bool {
        self.page > 0 && matches!(self.pages[self.page - 1], Page::Say(_))
    }

    fn step(&self, mut effects: Vec<Effect>) -> Step {
        let npc_id = self.npc_id;
        let prompt = match &self.pages[self.page] {
            Page::Done(done) => {
                effects.extend(done.iter().cloned());
                effects.extend(self.on_end.iter().cloned());
                return Step {
                    prompt: None,
                    effects,
                };
            }
            Page::Say(text) => Prompt::Text(DialoguePage {
                npc_id,
                text: text.clone(),
                has_previous: self.has_previous(),
                has_next: self.page + 1 < self.pages.len() || !self.on_end.is_empty(),
            }),
            Page::Act { text, effects: own } => {
                effects.extend(own.iter().cloned());
                Prompt::Text(DialoguePage {
                    npc_id,
                    text: text.clone(),
                    has_previous: false,
                    has_next: self.page + 1 < self.pages.len() || !self.on_end.is_empty(),
                })
            }
            Page::End(text) => Prompt::Text(DialoguePage {
                npc_id,
                text: text.clone(),
                has_previous: false,
                has_next: false,
            }),
            Page::Ask { text, .. } => Prompt::YesNo {
                npc_id,
                text: text.clone(),
            },
            Page::Menu { text, choices } => {
                let mut menu = text.clone();
                for (choice, (label, _)) in choices.iter().enumerate() {
                    if !menu.is_empty() {
                        menu.push('\n');
                    }
                    menu.push_str(&format!("#L{choice}##b{label}#k#l"));
                }
                Prompt::Selection { npc_id, text: menu }
            }
            Page::Style { text, choices } => Prompt::Style {
                npc_id,
                text: text.clone(),
                styles: choices.iter().map(|&(style, _)| style).collect(),
            },
        };
        Step {
            prompt: Some(prompt),
            effects,
        }
    }
}

/// `q1021s`: Roger hurts the player and hands over an apple.
fn roger_start(player: &Player) -> NpcScript {
    let title = text(if player.stats.gender == 0 {
        keys::ROGER_TITLE_MALE
    } else {
        keys::ROGER_TITLE_FEMALE
    });
    let mut on_yes = Vec::new();
    if player.stats.hp >= 50 {
        on_yes.push(Effect::SetHp(25));
    }
    if player.inventory.count(ROGERS_APPLE) == 0 {
        on_yes.push(Effect::GiveItem {
            item_id: ROGERS_APPLE,
            quantity: 1,
        });
    }
    on_yes.push(Effect::StartQuest(ROGERS_APPLE_QUEST));
    let [greeting, explain, ask, after, last] = keys::ROGER_START;
    let mut script = NpcScript::new(
        ROGER,
        vec![
            Page::Say(content::text().get(greeting, &[("title", &title)])),
            Page::Say(text(explain)),
            Page::Ask {
                text: text(ask),
                on_yes,
                on_no: None,
            },
            Page::Say(text(after)),
            Page::Say(text(last)),
        ],
    );
    script.on_end = vec![Effect::AvatarEffect("UI/tutorial.img/28".into())];
    script.inventory_full = Some(text(keys::ROGER_INVENTORY_FULL));
    script
}

/// `q1021e`: Roger checks the HP and pays the reward.
fn roger_end(player: &Player) -> NpcScript {
    if i32::from(player.stats.hp) < player.effective_max_hp() {
        // Not recovered: say so and stop without completing.
        return NpcScript::new(ROGER, vec![Page::Say(text(keys::ROGER_NOT_RECOVERED))]);
    }
    let mut script = NpcScript::new(
        ROGER,
        keys::ROGER_END.map(|key| Page::Say(text(key))).to_vec(),
    );
    script.on_end = vec![
        Effect::GiveItem {
            item_id: APPLE,
            quantity: 3,
        },
        Effect::GiveItem {
            item_id: GREEN_APPLE,
            quantity: 3,
        },
        Effect::GainExp(10),
        Effect::CompleteQuest(ROGERS_APPLE_QUEST),
    ];
    script.inventory_full = Some(text(keys::ROGER_END_INVENTORY_FULL));
    script
}

/// `begin7`: Shanks sails to Victoria Island for Lucas's letter or the fare.
fn shanks(player: &Player) -> NpcScript {
    let mut pages = vec![Page::Ask {
        text: text(keys::SHANKS_ASK),
        on_yes: Vec::new(),
        on_no: Some(text(keys::SHANKS_DECLINE)),
    }];
    let mut on_end = Vec::new();
    if player.inventory.count(RECOMMENDATION_LETTER) > 0 {
        pages.extend(keys::SHANKS_LETTER.map(|key| Page::Say(text(key))));
        on_end.push(Effect::TakeItem {
            item_id: RECOMMENDATION_LETTER,
            quantity: 1,
        });
    } else {
        pages.push(Page::Say(text(keys::SHANKS_FARE)));
        pages.push(Page::Say(text(if player.stats.level < SHIP_LEVEL {
            keys::SHANKS_TOO_WEAK
        } else if player.meso < SHIP_FARE {
            keys::SHANKS_NO_MESOS
        } else {
            on_end.push(Effect::SpendMeso(SHIP_FARE));
            keys::SHANKS_PAID
        })));
    }
    if !on_end.is_empty() {
        on_end.push(Effect::Warp(LITH_HARBOR));
    }
    let mut script = NpcScript::new(SHANKS, pages);
    script.on_end = on_end;
    script
}

/// Heracle founds guilds, disbands them, and grows them, as the player
/// chooses (the HeavenMS script's menu). Only a guild's master may do the
/// last two.
fn heracle(player: &Player, guild: Option<&Guild>) -> NpcScript {
    let master = guild.filter(|guild| guild.is_master(player.stats.id));
    let choices = vec![
        (text(keys::HERACLE_CREATE), vec![found_guild(player)]),
        (text(keys::HERACLE_DISBAND), vec![disband_guild(master)]),
        (text(keys::HERACLE_GROW), grow_guild(player, master)),
    ];
    NpcScript::new(
        HERACLE,
        vec![Page::Menu {
            text: text(keys::HERACLE_MENU),
            choices,
        }],
    )
}

/// The client asks for the new guild's name.
fn found_guild(player: &Player) -> Page {
    if player.guild.is_some() {
        Page::Say(text(keys::HERACLE_IN_GUILD))
    } else if player.stats.level < guild::MIN_LEVEL {
        Page::Say(text(keys::HERACLE_TOO_WEAK))
    } else if player.meso < guild::CREATE_COST {
        Page::Say(text(keys::HERACLE_NO_MESOS))
    } else {
        Page::Ask {
            text: text(keys::HERACLE_ASK),
            on_yes: vec![Effect::OpenGuildNamePrompt],
            on_no: Some(text(keys::HERACLE_DECLINE)),
        }
    }
}

/// The client has Heracle say how it went.
fn disband_guild(master: Option<&Guild>) -> Page {
    match master {
        Some(_) => Page::Ask {
            text: text(keys::HERACLE_DISBAND_ASK),
            on_yes: vec![Effect::DisbandGuild],
            on_no: Some(text(keys::HERACLE_DECLINE)),
        },
        None => Page::Say(text(keys::HERACLE_DISBAND_NOT_MASTER)),
    }
}

/// A step of `guild::CAPACITY_STEP` members. Like the HeavenMS script it
/// asks first and only then says the player can't pay; the client has
/// Heracle say how a paid step went.
fn grow_guild(player: &Player, master: Option<&Guild>) -> Vec<Page> {
    let Some(master) = master else {
        return vec![Page::Say(text(keys::HERACLE_GROW_NOT_MASTER))];
    };
    if master.capacity >= guild::MAX_CAPACITY {
        return vec![Page::Say(text(keys::HERACLE_GROW_FULL))];
    }
    let price = guild::capacity_cost(master.capacity);
    let price_text = price.to_string();
    let capacity = (master.capacity + guild::CAPACITY_STEP).to_string();
    let vars = [
        ("price", price_text.as_str()),
        ("capacity", capacity.as_str()),
    ];
    let can_pay = player.meso >= price;
    let on_yes = if can_pay {
        vec![Effect::GrowGuild {
            from: master.capacity,
        }]
    } else {
        Vec::new()
    };
    let mut pages = vec![Page::Ask {
        text: content::text().get(keys::HERACLE_GROW_ASK, &vars),
        on_yes,
        on_no: Some(text(keys::HERACLE_DECLINE)),
    }];
    if !can_pay {
        pages.push(Page::Say(
            content::text().get(keys::HERACLE_GROW_NO_MESOS, &vars),
        ));
    }
    pages
}

/// Lea opens the client's emblem maker for a guild's master (the HeavenMS
/// script's menu). Like that script she doesn't check the price, which is
/// charged when the client sends the emblem.
fn lea(player: &Player, guild: Option<&Guild>) -> NpcScript {
    let emblem = if guild.is_some_and(|guild| guild.may_set_emblem(player.stats.id)) {
        let price = guild::EMBLEM_COST.to_string();
        Page::Ask {
            text: content::text().get(keys::LEA_ASK, &[("price", price.as_str())]),
            on_yes: vec![Effect::OpenEmblemMaker],
            on_no: Some(text(keys::LEA_DECLINE)),
        }
    } else {
        Page::Say(text(keys::LEA_NOT_MASTER))
    };
    NpcScript::new(
        LEA,
        vec![Page::Menu {
            text: text(keys::LEA_MENU),
            choices: vec![(text(keys::LEA_EMBLEM), vec![emblem])],
        }],
    )
}

/// A station's ticket booth: one ticket for the route's price. Like the
/// original, it asks first and only then says the player can't pay.
fn ticket_seller(npc_id: i32, route: &Route, player: &Player) -> NpcScript {
    let mut script = NpcScript::new(npc_id, sell_ticket(route, player));
    script.inventory_full = Some(ticket_text(keys::SHIPS_SELL_SHORT, route));
    script
}

/// Agatha's booth: a ticket for any of Orbis's ships.
fn agatha(player: &Player) -> NpcScript {
    let choices = AGATHA_ROUTES
        .iter()
        .map(|(route, place)| {
            let choice = content::text().get(keys::SHIPS_SELL_CHOICE, &[("place", place)]);
            (choice, sell_ticket(route, player))
        })
        .collect();
    let mut script = NpcScript::new(
        AGATHA,
        vec![Page::Menu {
            text: text(keys::SHIPS_SELL_MENU),
            choices,
        }],
    );
    script.inventory_full = Some(text(keys::SHIPS_SELL_FULL));
    script
}

fn ticket_text(key: &str, route: &Route) -> String {
    content::text().get(key, &[("price", &route.price.to_string())])
}

fn sell_ticket(route: &Route, player: &Player) -> Vec<Page> {
    let ask = ticket_text(keys::SHIPS_SELL_ASK, route);
    sell(
        route,
        player,
        ask,
        keys::SHIPS_SELL_DECLINE,
        keys::SHIPS_SELL_SHORT,
    )
}

/// One of `route`'s tickets, asked about with `ask`: no says `decline`,
/// and a player short of the price hears `short` after yes.
fn sell(route: &Route, player: &Player, ask: String, decline: &str, short: &str) -> Vec<Page> {
    let can_pay = player.meso >= route.price;
    let on_yes = if can_pay {
        vec![
            Effect::SpendMeso(route.price),
            Effect::GiveItem {
                item_id: route.ticket,
                quantity: 1,
            },
        ]
    } else {
        Vec::new()
    };
    let mut pages = vec![Page::Ask {
        text: ask,
        on_yes,
        on_no: Some(text(decline)),
    }];
    if !can_pay {
        pages.push(Page::Say(ticket_text(short, route)));
    }
    pages
}

/// `get_ticket`: the platform's usher takes a ticket and shows the player
/// to the waiting room while the ship boards, as of route time `t`.
fn usher(npc_id: i32, route: &Route, player: &Player, t: i64) -> NpcScript {
    let page = match route.timetable.state(t) {
        State::Wait if player.inventory.count(route.ticket) == 0 => {
            Page::Say(text(keys::SHIPS_BOARD_NO_TICKET))
        }
        State::Wait => Page::Ask {
            text: text(keys::SHIPS_BOARD_ASK),
            on_yes: vec![
                Effect::RequireBoarding(route.platform),
                Effect::TakeItem {
                    item_id: route.ticket,
                    quantity: 1,
                },
                Effect::Warp(route.waiting_room),
            ],
            on_no: Some(text(keys::SHIPS_BOARD_DECLINE)),
        },
        State::Start => Page::Say(text(keys::SHIPS_BOARD_CLOSING)),
        _ => Page::Say(text(keys::SHIPS_BOARD_FULL)),
    };
    NpcScript::new(npc_id, vec![page])
}

/// `goOutWaitingRoom`: back to the platform, without the ticket.
fn leave_ship(npc_id: i32, route: &Route) -> NpcScript {
    NpcScript::new(
        npc_id,
        vec![Page::Ask {
            text: text(keys::SHIPS_LEAVE_ASK),
            on_yes: vec![Effect::Warp(route.platform)],
            on_no: Some(text(keys::SHIPS_LEAVE_DECLINE)),
        }],
    )
}

/// `NLC_Move`: a subway gate asks first and only then looks for the
/// ticket, then shows the player to the waiting room while the train
/// boards, as of route time `t`.
fn subway_gate(npc_id: i32, route: &Route, player: &Player, t: i64) -> NpcScript {
    let pages = match route.timetable.state(t) {
        State::Wait => {
            let has_ticket = player.inventory.count(route.ticket) > 0;
            let on_yes = if has_ticket {
                vec![
                    Effect::RequireBoarding(route.platform),
                    Effect::TakeItem {
                        item_id: route.ticket,
                        quantity: 1,
                    },
                    Effect::Warp(route.waiting_room),
                ]
            } else {
                Vec::new()
            };
            let mut pages = vec![Page::Ask {
                text: text(keys::SUBWAY_BOARD_ASK),
                on_yes,
                on_no: Some(text(keys::SUBWAY_BOARD_DECLINE)),
            }];
            if !has_ticket {
                pages.push(Page::Say(text(keys::SUBWAY_BOARD_NO_TICKET)));
            }
            pages
        }
        State::Start => vec![Page::Say(text(keys::SUBWAY_BOARD_CLOSING))],
        _ => vec![Page::Say(text(keys::SUBWAY_BOARD_FULL))],
    };
    NpcScript::new(npc_id, pages)
}

/// `NLC_ticketing`: on a platform Bell sells the ticket for the line that
/// leaves it; in a waiting room she lets the player back out to the
/// platform.
fn bell(player: &Player) -> Option<NpcScript> {
    let map_id = player.stats.map_id;
    if let Some(line) = SUBWAY_LINES
        .iter()
        .find(|line| line.route.platform == map_id)
    {
        let route = line.route;
        let (term, price, ticket) = (
            route.timetable.term.to_string(),
            route.price.to_string(),
            route.ticket.to_string(),
        );
        let vars = [
            ("term", term.as_str()),
            ("price", &price),
            ("ticket", &ticket),
        ];
        let ask = content::text().get(line.ask, &vars);
        let pages = sell(
            route,
            player,
            ask,
            keys::SUBWAY_SELL_DECLINE,
            keys::SUBWAY_SELL_SHORT,
        );
        let mut script = NpcScript::new(
            BELL,
            vec![Page::Menu {
                text: text(line.menu),
                choices: vec![(text(line.choice), pages)],
            }],
        );
        script.inventory_full = Some(ticket_text(keys::SUBWAY_SELL_SHORT, route));
        return Some(script);
    }
    let line = SUBWAY_LINES
        .iter()
        .find(|line| line.route.waiting_room == map_id)?;
    Some(NpcScript::new(
        BELL,
        vec![Page::Ask {
            text: text(line.leave),
            on_yes: vec![Effect::Warp(line.route.platform)],
            on_no: Some(text(keys::SUBWAY_LEAVE_DECLINE)),
        }],
    ))
}

/// The NLC Taxi, to the Haunted House's grounds from New Leaf City and
/// back.
fn nlc_taxi(player: &Player) -> NpcScript {
    let to = if player.stats.map_id == HAUNTED_HOUSE_GROUNDS {
        NEW_LEAF_CITY
    } else {
        HAUNTED_HOUSE_GROUNDS
    };
    let (map, price) = (to.to_string(), NLC_TAXI_FARE.to_string());
    let ask = content::text().get(keys::NLC_TAXI_ASK, &[("map", &map), ("price", &price)]);
    let mut script = NpcScript::new(
        NLC_TAXI,
        ride(
            ask,
            Some(keys::NLC_TAXI_DECLINE),
            keys::NLC_TAXI_SHORT,
            NLC_TAXI_FARE,
            Effect::WarpToPortal(to, 0),
            player,
        ),
    );
    script.refused = Some(text(keys::NLC_TAXI_SHORT));
    script
}

/// Lakelis (Cosmic `9020000`): the party's leader may take the party in.
fn lakelis(entry: Entry) -> NpcScript {
    let page = match entry {
        Entry::NoParty => Page::Say(text(keys::KPQ_LAKELIS_NO_PARTY)),
        Entry::NotLeader => Page::Say(text(keys::KPQ_LAKELIS_NOT_LEADER)),
        Entry::NotEligible => Page::Say(kpq_terms()),
        Entry::Busy => Page::Say(text(keys::KPQ_LAKELIS_BUSY)),
        Entry::Ready => Page::Ask {
            text: text(keys::KPQ_LAKELIS_ASK),
            on_yes: vec![Effect::StartEvent(EventKind::Kpq)],
            on_no: Some(text(keys::KPQ_LAKELIS_DECLINE)),
        },
    };
    let mut script = NpcScript::new(LAKELIS, vec![page]);
    // Someone left or moved since she checked, or another party went in.
    script.refused = Some(kpq_terms());
    script
}

/// Who may play Kerning City's party quest.
fn kpq_terms() -> String {
    let (team, levels) = (&events::KPQ.team, &events::KPQ.levels);
    content::text().get(
        keys::KPQ_LAKELIS_NOT_ELIGIBLE,
        &[
            ("least", &team.start().to_string()),
            ("most", &team.end().to_string()),
            ("low", &levels.start().to_string()),
            ("high", &levels.end().to_string()),
        ],
    )
}

/// Cloto (Cosmic `9020001`). On stage 1 each member but the leader is
/// asked to hold a number of coupons, and holding exactly that many earns
/// a pass; the leader clears the stage by handing in a pass from each of
/// them. A wrong number costs nothing. On stages 2 to 4 she sets the
/// leader a puzzle (`events::Puzzles`).
fn cloto(view: Cloto, player: &Player) -> NpcScript {
    let held = |item_id| player.inventory.count(item_id);
    let page = match view {
        Cloto::Cleared => Page::Say(text(keys::KPQ_CLOTO_CLEARED)),
        Cloto::Won => Page::Say(text(keys::KPQ_CLOTO_S5_WON)),
        Cloto::Follow => Page::Say(text(keys::KPQ_CLOTO_FOLLOW)),
        Cloto::Gather => Page::Say(text(keys::KPQ_CLOTO_S5_GATHER)),
        // A puzzle unanswered (`cloto_answer`) is set out again.
        Cloto::Intro { stage } | Cloto::Puzzle { stage, .. } => {
            Page::Say(text(PUZZLES[usize::from(stage.clamp(2, 4) - 2)]))
        }
        Cloto::Member { passed: true, .. } => Page::Say(text(keys::KPQ_CLOTO_THANKS)),
        Cloto::Member { question, new, .. } => {
            let asked = text(QUESTIONS[question]);
            let answer = events::ANSWERS[question];
            if new {
                Page::Say(asked)
            } else if held(events::COUPON) == answer {
                Page::Act {
                    text: text(keys::KPQ_CLOTO_RIGHT),
                    effects: vec![
                        Effect::TakeItem {
                            item_id: events::COUPON,
                            quantity: answer as i16,
                        },
                        Effect::GiveItem {
                            item_id: events::PASS,
                            quantity: 1,
                        },
                        Effect::EventCommand(EventCommand::PassGiven),
                    ],
                }
            } else {
                Page::Say(content::text().get(keys::KPQ_CLOTO_WRONG, &[("question", &asked)]))
            }
        }
        Cloto::Leader { stage, needed } => {
            let says = |key| content::text().get(key, &[("n", &needed.to_string())]);
            let (clear, short) = if stage == 1 {
                (keys::KPQ_CLOTO_LEADER_CLEAR, keys::KPQ_CLOTO_LEADER_SHORT)
            } else {
                (keys::KPQ_CLOTO_S5_CLEAR, keys::KPQ_CLOTO_S5_SHORT)
            };
            match i16::try_from(needed) {
                Ok(quantity) if held(events::PASS) >= needed => Page::Act {
                    text: says(clear),
                    effects: vec![
                        Effect::TakeItem {
                            item_id: events::PASS,
                            quantity,
                        },
                        Effect::EventCommand(EventCommand::ClearStage(stage)),
                    ],
                },
                _ => Page::Say(says(short)),
            }
        }
    };
    let mut script = NpcScript::new(CLOTO, vec![page]);
    script.inventory_full = Some(text(keys::KPQ_CLOTO_FULL));
    script.refused = Some(text(keys::KPQ_CLOTO_REFUSED));
    script
}

/// Cloto's puzzles on stages 2 to 4: the ropes, platforms and barrels.
const PUZZLES: [&str; 3] = [
    keys::KPQ_CLOTO_S2_INTRO,
    keys::KPQ_CLOTO_S3_INTRO,
    keys::KPQ_CLOTO_S4_INTRO,
];

/// Cloto's questions, in the order of `events::ANSWERS`.
const QUESTIONS: [&str; 7] = [
    keys::KPQ_CLOTO_Q1,
    keys::KPQ_CLOTO_Q2,
    keys::KPQ_CLOTO_Q3,
    keys::KPQ_CLOTO_Q4,
    keys::KPQ_CLOTO_Q5,
    keys::KPQ_CLOTO_Q6,
    keys::KPQ_CLOTO_Q7,
];

/// Nella (Cosmic `9020002`): out of the party quest, then back to town.
fn nella(player: &Player) -> Option<NpcScript> {
    let map_id = player.stats.map_id;
    let (key, to) = if map_id == events::KPQ.exit {
        (keys::KPQ_NELLA_HOME, events::KPQ.lobby)
    } else if events::KPQ.holds(map_id) {
        (keys::KPQ_NELLA_LEAVE, events::KPQ.exit)
    } else {
        return None;
    };
    Some(NpcScript::new(
        NELLA,
        vec![Page::Ask {
            text: text(key),
            on_yes: vec![Effect::Warp(to)],
            on_no: None,
        }],
    ))
}

/// Jeff (`goDungeon`): players from level 50 may go down to Sharp Cliff,
/// and an Orihalcon Hammer's holder goes to Ice Valley without a word.
fn jeff(player: &Player) -> NpcScript {
    if player.inventory.count(ORIHALCON_HAMMER) > 0 {
        let (map, portal) = ICE_VALLEY;
        return NpcScript::new(
            JEFF,
            vec![Page::Done(vec![Effect::WarpToPortal(map, portal)])],
        );
    }
    let (map, portal) = SHARP_CLIFF;
    let next = if player.stats.level >= JEFF_LEVEL {
        Page::Ask {
            text: text(keys::JEFF_ASK),
            on_yes: vec![Effect::WarpToPortal(map, portal)],
            on_no: Some(text(keys::JEFF_DECLINE)),
        }
    } else {
        Page::Say(text(keys::JEFF_TOO_WEAK))
    };
    NpcScript::new(JEFF, vec![Page::Say(text(keys::JEFF_INTRO)), next])
}

/// The Time Gate (`TD_neoCity_enter`): once Andy's quest wakes it, it
/// takes the player to any era their Guardian Nex quests have opened.
fn time_gate(player: &Player) -> NpcScript {
    let done = |id| player.quests.state(id) == QuestState::Completed;
    if !done(TIME_GATE_QUEST) {
        return NpcScript::new(TIME_GATE, vec![Page::Say(text(keys::TIME_GATE_INACTIVE))]);
    }
    let open = ERAS.iter().take_while(|&&(_, quest)| done(quest)).count();
    if open == 0 {
        return NpcScript::new(TIME_GATE, vec![Page::Say(text(keys::TIME_GATE_NO_ERA))]);
    }
    let choices = ERAS[..open]
        .iter()
        .map(|&(map, _)| {
            (
                format!("#m{map}#"),
                vec![Page::Done(vec![Effect::WarpToPortal(map, 1)])],
            )
        })
        .collect();
    NpcScript::new(
        TIME_GATE,
        vec![Page::Menu {
            text: String::new(),
            choices,
        }],
    )
}

/// Shane (`herb_in`): from level 25, the Forest of Patience for a fare,
/// starting at the step of the herb quest the player is on, else the one
/// for their level.
fn shane(player: &Player) -> NpcScript {
    if player.stats.level < SHANE_LEVEL {
        return NpcScript::new(SHANE, vec![Page::Say(text(keys::SHANE_TOO_LOW))]);
    }
    let started = |id| player.quests.state(id) == QuestState::Started;
    let step = if started(2050) {
        PATIENCE_STEP_1
    } else if started(2051) || player.stats.level >= 50 {
        PATIENCE_STEP_3
    } else {
        PATIENCE_STEP_1
    };
    let price = SHANE_FARE.to_string();
    let ask = content::text().get(keys::SHANE_ASK, &[("price", &price)]);
    let mut script = NpcScript::new(
        SHANE,
        ride(
            ask,
            Some(keys::SHANE_DECLINE),
            keys::SHANE_SHORT,
            SHANE_FARE,
            Effect::WarpToPortal(step, 0),
            player,
        ),
    );
    script.refused = Some(text(keys::SHANE_SHORT));
    script
}

/// The Mysterious Statue (`flower_in`): past it, the Deep Forest's steps
/// that the player's John quests have opened.
fn mysterious_statue(player: &Player) -> NpcScript {
    let taken = |id| player.quests.state(id) != QuestState::NotStarted;
    let open = DEEP_FOREST
        .iter()
        .rposition(|&(_, quest)| taken(quest))
        .map_or(0, |last| last + 1);
    let mut pages = vec![Page::Say(text(keys::STATUE_INTRO))];
    if open > 0 {
        let choices = DEEP_FOREST[..open]
            .iter()
            .map(|&(map, _)| {
                (
                    format!("#m{map}#"),
                    vec![Page::Done(vec![Effect::WarpToPortal(map, 0)])],
                )
            })
            .collect();
        pages.push(Page::Menu {
            text: text(keys::STATUE_MENU),
            choices,
        });
    }
    NpcScript::new(MYSTERIOUS_STATUE, pages)
}

/// `station_in`: Isa shows the way to Orbis Station's platforms.
fn isa() -> NpcScript {
    let choices = ISA_PLATFORMS
        .iter()
        .map(|&(vehicle, place, passage)| {
            let vars = [("vehicle", vehicle), ("place", place)];
            let ask = Page::Ask {
                text: content::text().get(keys::ISA_ASK, &vars),
                on_yes: vec![Effect::Warp(passage)],
                on_no: Some(text(keys::ISA_DECLINE)),
            };
            (content::text().get(keys::ISA_PLATFORM, &vars), vec![ask])
        })
        .collect();
    NpcScript::new(
        ISA,
        vec![Page::Menu {
            text: text(keys::ISA_MENU),
            choices,
        }],
    )
}

/// `taxi1`-`taxi6`: a Regular Cab to another of Victoria's towns.
fn regular_cab(npc_id: i32, fares: &[(i32, i32)], player: &Player) -> NpcScript {
    let beginner = player.stats.job == 0;
    let mut menu = if beginner {
        text(keys::TAXI_DISCOUNT)
    } else {
        String::new()
    };
    menu.push_str(&text(keys::TAXI_MENU));
    let choices = fares
        .iter()
        .map(|&(town, fare)| {
            let fare = if beginner { fare / 10 } else { fare };
            let (map, price) = (town.to_string(), fare.to_string());
            let vars = [("map", map.as_str()), ("price", price.as_str())];
            let ride = ride(
                content::text().get(keys::TAXI_ASK, &vars),
                Some(keys::TAXI_DECLINE),
                keys::TAXI_SHORT,
                fare,
                Effect::Warp(town),
                player,
            );
            (content::text().get(keys::TAXI_CHOICE, &vars), ride)
        })
        .collect();
    let mut script = NpcScript::new(
        npc_id,
        vec![
            Page::Say(text(keys::TAXI_INTRO)),
            Page::Menu {
                text: menu,
                choices,
            },
        ],
    );
    script.refused = Some(text(keys::TAXI_SHORT));
    script
}

/// `mTaxi`: the VIP Cab to Ant Tunnel Park, a tenth of the fare for a
/// beginner.
fn vip_cab(npc_id: i32, player: &Player) -> NpcScript {
    let beginner = player.stats.job == 0;
    let fare = if beginner { VIP_FARE / 10 } else { VIP_FARE };
    let mut ask = if beginner {
        text(keys::TAXI_DISCOUNT)
    } else {
        String::new()
    };
    ask.push_str(&content::text().get(keys::VIP_CAB_ASK, &[("price", &fare.to_string())]));
    let mut pages = vec![Page::Say(text(keys::VIP_CAB_INTRO))];
    pages.extend(ride(
        ask,
        Some(keys::VIP_CAB_DECLINE),
        keys::VIP_CAB_SHORT,
        fare,
        Effect::Warp(ANT_TUNNEL_PARK),
        player,
    ));
    let mut script = NpcScript::new(npc_id, pages);
    script.refused = Some(text(keys::VIP_CAB_SHORT));
    script
}

/// `ossyria_taxi`: the Danger Zone Taxi from the town it stands in, past
/// the dangerous part of the way. `None` in a town it doesn't serve.
fn danger_zone_taxi(player: &Player) -> Option<NpcScript> {
    let &(town, to, portal, fare) = DANGER_ZONE_ROUTES
        .iter()
        .find(|(town, ..)| *town == player.stats.map_id)?;
    let (town, map, price) = (town.to_string(), to.to_string(), fare.to_string());
    let vars = [("town", town.as_str()), ("map", &map), ("price", &price)];
    let mut pages = vec![Page::Say(
        content::text().get(keys::DANGER_ZONE_TAXI_INTRO, &vars),
    )];
    pages.extend(ride(
        content::text().get(keys::DANGER_ZONE_TAXI_ASK, &vars),
        Some(keys::DANGER_ZONE_TAXI_DECLINE),
        keys::DANGER_ZONE_TAXI_SHORT,
        fare,
        Effect::WarpToPortal(to, portal),
        player,
    ));
    let mut script = NpcScript::new(DANGER_ZONE_TAXI, pages);
    script.refused = Some(text(keys::DANGER_ZONE_TAXI_SHORT));
    Some(script)
}

/// `nihal_taxi`: the Camel Cab to Magatia from outside Ariant's north
/// gate, and back to Ariant from Sahel 1.
fn camel_cab(player: &Player) -> NpcScript {
    let to = if player.stats.map_id == OUTSIDE_ARIANT {
        MAGATIA
    } else {
        ARIANT
    };
    let (map, price) = (to.to_string(), CAMEL_CAB_FARE.to_string());
    let ask = content::text().get(keys::CAMEL_CAB_ASK, &[("map", &map), ("price", &price)]);
    let mut script = NpcScript::new(
        CAMEL_CAB,
        ride(
            ask,
            Some(keys::CAMEL_CAB_DECLINE),
            keys::CAMEL_CAB_SHORT,
            CAMEL_CAB_FARE,
            Effect::WarpToPortal(to, 0),
            player,
        ),
    );
    script.refused = Some(text(keys::CAMEL_CAB_SHORT));
    script
}

/// `aqua_taxi`: the Dolphin Taxi down to the Sharp Unknown, for a coupon
/// if the player has one and a fare if not, or across to the other end
/// of the route.
fn dolphin(player: &Player) -> NpcScript {
    let across = if player.stats.map_id == HERB_TOWN_PIER {
        AQUARIUM
    } else {
        HERB_TOWN_PIER
    };
    let down = SHARP_UNKNOWN.to_string();
    let down = if player.inventory.count(DOLPHIN_TAXI_COUPON) > 0 {
        let vars = [("map", down.as_str())];
        let ask = Page::Ask {
            text: content::text().get(keys::DOLPHIN_COUPON_ASK, &vars),
            on_yes: vec![
                Effect::TakeItem {
                    item_id: DOLPHIN_TAXI_COUPON,
                    quantity: 1,
                },
                Effect::WarpToPortal(SHARP_UNKNOWN, 2),
            ],
            on_no: None,
        };
        (content::text().get(keys::DOLPHIN_COUPON, &vars), vec![ask])
    } else {
        dolphin_ride(
            SHARP_UNKNOWN,
            SHARP_UNKNOWN_FARE,
            Effect::WarpToPortal(SHARP_UNKNOWN, 2),
            player,
        )
    };
    let across = dolphin_ride(across, DOLPHIN_FARE, Effect::Warp(across), player);
    let mut script = NpcScript::new(
        DOLPHIN,
        vec![Page::Menu {
            text: text(keys::DOLPHIN_MENU),
            choices: vec![down, across],
        }],
    );
    script.refused = Some(text(keys::DOLPHIN_SHORT));
    script
}

/// A choice of the Dolphin Taxi's menu: to `to` for `fare`.
fn dolphin_ride(to: i32, fare: i32, warp: Effect, player: &Player) -> (String, Vec<Page>) {
    let (map, price) = (to.to_string(), fare.to_string());
    let vars = [("map", map.as_str()), ("price", price.as_str())];
    let ride = ride(
        content::text().get(keys::DOLPHIN_ASK, &vars),
        None,
        keys::DOLPHIN_SHORT,
        fare,
        warp,
        player,
    );
    (content::text().get(keys::DOLPHIN_CHOICE, &vars), ride)
}

/// A ride asked about with `ask`: yes pays `fare` and applies `warp`, no
/// says `decline` (if any), and a player short of the fare hears `short`
/// after yes instead.
fn ride(
    ask: String,
    decline: Option<&str>,
    short: &str,
    fare: i32,
    warp: Effect,
    player: &Player,
) -> Vec<Page> {
    let can_pay = player.meso >= fare;
    let on_yes = if can_pay {
        vec![Effect::SpendMeso(fare), warp]
    } else {
        Vec::new()
    };
    let mut pages = vec![Page::Ask {
        text: ask,
        on_yes,
        on_no: decline.map(text),
    }];
    if !can_pay {
        pages.push(Page::Say(text(short)));
    }
    pages
}

/// A first-job instructor. A beginner who qualifies may take the job, its
/// weapons and its bag rows; one who doesn't is told to train; everyone
/// else hears what the job is like.
fn first_job(instructor: &Instructor, player: &Player) -> NpcScript {
    let line = |line: &str| text(&keys::first_job(instructor.text, line));
    if player.stats.job != 0 {
        return NpcScript::new(instructor.npc_id, vec![Page::Say(line("other"))]);
    }
    let mut pages = vec![Page::Say(line("intro"))];
    if !player.can_take_first_job(instructor.job) {
        pages.push(Page::Say(line("too_weak")));
        return NpcScript::new(instructor.npc_id, pages);
    }
    let mut on_yes = vec![Effect::TakeFirstJob(instructor.job)];
    on_yes.extend(instructor.rows.iter().map(|&kind| Effect::AddSlotRow(kind)));
    on_yes.extend(
        instructor
            .items
            .iter()
            .map(|&(item_id, quantity)| Effect::GiveItem { item_id, quantity }),
    );
    pages.push(Page::Ask {
        text: line("ask"),
        on_yes,
        on_no: None,
    });
    pages.push(Page::Say(line("accept")));
    let content = content::text();
    pages.extend(
        (0..)
            .map(|page| keys::first_job(instructor.text, &format!("after.{page}")))
            .take_while(|key| content.contains(key))
            .map(|key| Page::Say(text(&key))),
    );
    let mut script = NpcScript::new(instructor.npc_id, pages);
    script.inventory_full = Some(text(keys::FIRST_JOB_INVENTORY_FULL));
    script
}

/// A first-job Explorer of level 30 or more: a letter for the examiner,
/// and for the proof from the examiner's test, the choice of second job.
fn second_job(instructor: &Instructor, test: &Test, player: &Player) -> NpcScript {
    let line = |line: &str| text(&keys::second_job(instructor.text, line));
    if player.inventory.count(PROOF_OF_A_HERO) > 0 {
        let choices = instructor
            .second_jobs
            .iter()
            .map(|&job| {
                let line = |line: &str| text(&keys::second_job(&job.to_string(), line));
                let on_yes = vec![
                    Effect::TakeItem {
                        item_id: PROOF_OF_A_HERO,
                        quantity: 1,
                    },
                    Effect::TakeSecondJob(job),
                    Effect::AddSlotRow(InventoryType::Etc),
                ];
                let pages = vec![
                    Page::Say(line("about")),
                    Page::Ask {
                        text: line("ask"),
                        on_yes,
                        on_no: None,
                    },
                    Page::Say(line("accept")),
                ];
                (line("name"), pages)
            })
            .collect();
        let menu = Page::Menu {
            text: line("choose"),
            choices,
        };
        return NpcScript::new(instructor.npc_id, vec![menu]);
    }
    if player.inventory.count(test.letter) > 0 {
        return NpcScript::new(instructor.npc_id, vec![Page::Say(line("go"))]);
    }
    let mut script = NpcScript::new(instructor.npc_id, vec![Page::Say(line("letter"))]);
    script.on_end = vec![Effect::GiveItem {
        item_id: test.letter,
        quantity: 1,
    }];
    script.inventory_full = Some(text(keys::SECOND_JOB_INVENTORY_FULL));
    script
}

/// The second job's examiner lets the instructor's letter into the test.
/// The player keeps the letter until passing, so leaving and coming back
/// needs no other record (upstream marks a quest the client doesn't have).
fn examiner(instructor: &Instructor, test: &Test, player: &Player) -> NpcScript {
    let line = |line: &str| text(&keys::second_job(instructor.text, line));
    let pages = if player.inventory.count(test.letter) == 0 {
        vec![Page::Say(line("examiner.other"))]
    } else {
        vec![
            Page::Say(line("examiner")),
            Page::Ask {
                text: line("examiner.ask"),
                on_yes: vec![Effect::Warp(test.test_map)],
                on_no: Some(line("examiner.decline")),
            },
        ]
    };
    NpcScript::new(test.examiner, pages)
}

/// Inside the test, the proctor trades all of the player's Dark Marbles,
/// once there are enough, and the letter for the proof, and sees anyone
/// out to the examiner.
fn proctor(instructor: &Instructor, test: &Test, player: &Player) -> NpcScript {
    let line = |line: &str| text(&keys::second_job(instructor.text, line));
    let out = Effect::WarpToPortal(test.examiner_map, test.examiner_portal);
    let marbles = player.inventory.count(DARK_MARBLE);
    if marbles < MARBLES_NEEDED {
        let leave = Page::Ask {
            text: line("proctor.ask"),
            on_yes: vec![out],
            on_no: None,
        };
        return NpcScript::new(test.proctor, vec![leave]);
    }
    let mut on_end = vec![Effect::TakeItem {
        item_id: DARK_MARBLE,
        quantity: i16::try_from(marbles).unwrap_or(i16::MAX),
    }];
    if player.inventory.count(test.letter) > 0 {
        on_end.push(Effect::TakeItem {
            item_id: test.letter,
            quantity: 1,
        });
    }
    on_end.push(Effect::GiveItem {
        item_id: PROOF_OF_A_HERO,
        quantity: 1,
    });
    on_end.push(out);
    let mut script = NpcScript::new(test.proctor, vec![Page::Say(line("proctor.pass"))]);
    script.on_end = on_end;
    script.inventory_full = Some(text(keys::SECOND_JOB_INVENTORY_FULL));
    script
}

/// Kyrin and a level-30 Pirate. With one of her quests completed, she
/// offers its job; with one started, the way into its test; otherwise she
/// points to the quests, which the client offers beside her script.
fn pirate_second_job(player: &Player) -> NpcScript {
    let line = |line: &str| text(&keys::second_job("pirate", line));
    let quest = |state| {
        PIRATE_TESTS
            .iter()
            .find(|test| player.quests.state(test.quest) == state)
    };
    let pages = if let Some(test) = quest(QuestState::Completed) {
        let line = |line: &str| text(&keys::second_job(&test.job.to_string(), line));
        vec![
            Page::Say(line("about")),
            Page::Ask {
                text: line("ask"),
                on_yes: vec![
                    Effect::TakeSecondJob(test.job),
                    Effect::AddSlotRow(InventoryType::Etc),
                ],
                on_no: None,
            },
            Page::Say(line("accept")),
        ]
    } else if let Some(test) = quest(QuestState::Started) {
        vec![Page::Ask {
            text: line("test"),
            on_yes: vec![Effect::Warp(test.map)],
            on_no: None,
        }]
    } else {
        vec![Page::Say(line("quest"))]
    };
    let mut script = NpcScript::new(KYRIN, pages);
    script.inventory_full = Some(text(keys::SECOND_JOB_INVENTORY_FULL));
    script
}

/// `inside_pirate`: in a test map, with the test's 15 crystals, the
/// proctor sees the player back to Kyrin; without, offers the way out. The
/// crystals stay for Kyrin's quest to take (upstream takes them only from
/// a player who leaves early).
fn pirate_proctor(player: &Player) -> NpcScript {
    let line = |line: &str| text(&keys::second_job("pirate", line));
    let out = Effect::WarpToPortal(KYRIN_MAP, 0);
    let passed = PIRATE_TESTS.iter().any(|test| {
        test.map == player.stats.map_id && player.inventory.count(test.crystal) >= CRYSTALS_NEEDED
    });
    if !passed {
        let leave = Page::Ask {
            text: line("proctor.ask"),
            on_yes: vec![out],
            on_no: None,
        };
        return NpcScript::new(PIRATE_PROCTOR, vec![leave]);
    }
    let mut script = NpcScript::new(PIRATE_PROCTOR, vec![Page::Say(line("proctor.pass"))]);
    script.on_end = vec![out];
    script
}

/// The third-job line of a level-70 second-job Explorer, if they are one.
fn third_job_line(player: &Player) -> Option<&'static ThirdJob> {
    let stats = &player.stats;
    if stats.level < 70 || stats.job % 10 != 0 || stats.job % 100 == 0 {
        return None;
    }
    THIRD_JOBS
        .iter()
        .find(|line| line.job == stats.job / 100 * 100)
}

fn marked(player: &Player, marker: i16) -> bool {
    player.quests.state(marker) == QuestState::Started
}

/// Whether the player holds any `item_id`.
fn holds(player: &Player, item_id: i32) -> bool {
    player.inventory.count(item_id) > 0
}

/// A chief in El Nath and a second job of their line: the test, the
/// Necklace of Strength traded for the Holy Stone's assignment, and the
/// Necklace of Wisdom for the third job, once the player has spent all but
/// three SP for each level past 70.
fn chief(line: &ThirdJob, player: &Player) -> NpcScript {
    let say = |part: &str| text(&keys::third_job(line.text, part));
    let mut script = NpcScript::new(line.chief, Vec::new());
    if third_job_line(player).is_none_or(|own| own.job != line.job) {
        script.pages = vec![Page::Say(say("chief.other"))];
        return script;
    }
    let job = player.stats.job + 1;
    script.pages = if marked(player, HOLY_STONE_ASSIGNED) && holds(player, NECKLACE_OF_WISDOM) {
        if !player.can_take_third_job(job) {
            vec![Page::Say(say("chief.sp"))]
        } else {
            vec![
                Page::Ask {
                    text: say("chief.advance"),
                    on_yes: vec![
                        Effect::TakeItem {
                            item_id: NECKLACE_OF_WISDOM,
                            quantity: 1,
                        },
                        Effect::Unmark(HOLY_STONE_ASSIGNED),
                        Effect::TakeThirdJob(job),
                    ],
                    on_no: None,
                },
                Page::Say(text(&keys::third_job(&job.to_string(), "accept"))),
            ]
        }
    } else if marked(player, HOLY_STONE_ASSIGNED) {
        vec![Page::Say(say("chief.wisdom"))]
    } else if holds(player, NECKLACE_OF_STRENGTH) {
        script.on_end = vec![
            Effect::TakeItem {
                item_id: NECKLACE_OF_STRENGTH,
                quantity: 1,
            },
            Effect::Mark(HOLY_STONE_ASSIGNED),
        ];
        vec![Page::Say(say("chief.strength"))]
    } else if marked(player, CHIEF_ASSIGNED) || marked(player, INSTRUCTOR_ASSIGNED) {
        vec![Page::Say(say("chief.remind"))]
    } else {
        vec![
            Page::Ask {
                text: say("chief.ask"),
                on_yes: vec![Effect::Mark(CHIEF_ASSIGNED)],
                on_no: Some(say("chief.decline")),
            },
            Page::Say(say("chief.go")),
        ]
    };
    script
}

/// A fourth-job instructor in Leafre and a third job of their line: the
/// advancement once the player is level 120 and has finished the line's
/// last quest.
fn fourth_job_instructor(line: &FourthJob, player: &Player) -> NpcScript {
    let say = |part: &str| text(&keys::fourth_job(line.text, part));
    let stats = &player.stats;
    let mut script = NpcScript::new(line.instructor, Vec::new());
    script.pages = if stats.job / 100 * 100 != line.job || stats.job % 10 == 0 {
        vec![Page::Say(say("other"))]
    } else if stats.job % 10 == 2 {
        vec![Page::Say(say("done"))]
    } else if stats.level < 120 || player.quests.state(line.quest) != QuestState::Completed {
        vec![Page::Say(say("trials"))]
    } else {
        let job = stats.job + 1;
        vec![
            Page::Ask {
                text: say("ask"),
                on_yes: vec![
                    Effect::GiveItem {
                        item_id: MAPLE_HERO,
                        quantity: 1,
                    },
                    Effect::TakeFourthJob(job),
                ],
                on_no: Some(say("decline")),
            },
            Page::Say(text(&keys::fourth_job(&job.to_string(), "accept"))),
        ]
    };
    script.inventory_full = Some(text(keys::FOURTH_JOB_INVENTORY_FULL));
    script
}

/// The instructor of a player the chief sent: on to the Door of Dimension,
/// and for the clone's Black Charm, the Necklace of Strength.
fn third_job_instructor(instructor: &Instructor, player: &Player) -> NpcScript {
    let say = |part: &str| text(&keys::third_job(instructor.text, part));
    let mut script = NpcScript::new(instructor.npc_id, Vec::new());
    if marked(player, CHIEF_ASSIGNED) {
        script.pages = vec![Page::Say(say("instructor.assign"))];
        script.on_end = vec![
            Effect::Unmark(CHIEF_ASSIGNED),
            Effect::Mark(INSTRUCTOR_ASSIGNED),
        ];
    } else if holds(player, BLACK_CHARM) {
        script.pages = vec![Page::Say(say("instructor.reward"))];
        script.on_end = vec![
            Effect::TakeItem {
                item_id: BLACK_CHARM,
                quantity: 1,
            },
            Effect::GiveItem {
                item_id: NECKLACE_OF_STRENGTH,
                quantity: 1,
            },
            Effect::Unmark(INSTRUCTOR_ASSIGNED),
        ];
        script.inventory_full = Some(text(keys::THIRD_JOB_INVENTORY_FULL));
    } else {
        script.pages = vec![Page::Say(say("instructor.remind"))];
    }
    script
}

/// The Door of Dimension in a line's hunting map lets in the player their
/// instructor sent, one challenger at a time: there is a single clone
/// room per line.
fn door_of_dimension(player: &Player) -> NpcScript {
    let line = THIRD_JOBS
        .iter()
        .find(|line| line.door_map == player.stats.map_id)
        .filter(|line| third_job_line(player).is_some_and(|own| own.job == line.job))
        .filter(|_| marked(player, INSTRUCTOR_ASSIGNED) && !holds(player, BLACK_CHARM));
    let Some(line) = line else {
        return NpcScript::new(
            DOOR_OF_DIMENSION,
            vec![Page::Say(text(keys::THIRD_JOB_DOOR_OTHER))],
        );
    };
    let mut script = NpcScript::new(
        DOOR_OF_DIMENSION,
        vec![Page::Ask {
            text: text(keys::THIRD_JOB_DOOR_ASK),
            on_yes: vec![
                Effect::RequireEmptyMap(line.boss_map),
                Effect::RequireEmptyMap(line.entry_map),
                Effect::WarpToPortal(line.entry_map, 0),
            ],
            on_no: None,
        }],
    );
    script.refused = Some(text(keys::THIRD_JOB_DOOR_BUSY));
    script
}

/// The Sparkling Crystal in a clone's room leads back to the door. `None`
/// anywhere else.
fn sparkling_crystal(player: &Player) -> Option<NpcScript> {
    let line = THIRD_JOBS
        .iter()
        .find(|line| line.boss_map == player.stats.map_id)?;
    Some(NpcScript::new(
        SPARKLING_CRYSTAL,
        vec![Page::Ask {
            text: text(keys::THIRD_JOB_CRYSTAL_ASK),
            on_yes: vec![Effect::Warp(line.door_map)],
            on_no: None,
        }],
    ))
}

/// The clone's room: the time a challenger has in it, which the map itself
/// doesn't set.
pub fn time_limit(map_id: i32) -> Option<u32> {
    resets_when_empty(map_id).then_some(CLONE_TIME_LIMIT)
}

/// Whether the field puts its mobs back when the last player leaves, as a
/// clone's room does for the next challenger.
pub fn resets_when_empty(map_id: i32) -> bool {
    THIRD_JOBS.iter().any(|line| line.boss_map == map_id)
}

/// The Holy Stone takes a Dark Crystal from the player the chief sent and
/// asks them `QUIZ_LENGTH` questions picked from the text's; all right
/// earns the Necklace of Wisdom, any wrong ends the try (the crystal is
/// spent either way).
fn holy_stone(player: &Player, rng: &mut impl Rng) -> NpcScript {
    let say = |key| NpcScript::new(HOLY_STONE, vec![Page::Say(text(key))]);
    if !marked(player, HOLY_STONE_ASSIGNED) {
        return say(keys::HOLY_STONE_OTHER);
    }
    if holds(player, NECKLACE_OF_WISDOM) {
        return say(keys::HOLY_STONE_DONE);
    }
    if !holds(player, DARK_CRYSTAL) {
        return say(keys::HOLY_STONE_NO_CRYSTAL);
    }
    if crate::items::give(&mut player.inventory.clone(), NECKLACE_OF_WISDOM, 1).is_err() {
        return say(keys::HOLY_STONE_ETC_FULL);
    }
    let content = content::text();
    let questions: Vec<usize> = (0..)
        .take_while(|&n| content.contains(&keys::holy_stone_question(n)))
        .collect();
    let fail = vec![Page::End(text(keys::HOLY_STONE_FAIL))];
    let mut next = vec![Page::Say(text(keys::HOLY_STONE_PASS))];
    for &n in questions.choose_multiple(rng, QUIZ_LENGTH) {
        let mut choices = vec![(text(&keys::holy_stone_right(n)), next)];
        choices.extend(
            (0..)
                .map(|k| keys::holy_stone_wrong(n, k))
                .take_while(|key| content.contains(key))
                .map(|key| (text(&key), fail.clone())),
        );
        choices.shuffle(rng);
        next = vec![Page::Menu {
            text: text(&keys::holy_stone_question(n)),
            choices,
        }];
    }
    let mut pages = vec![Page::Ask {
        text: text(keys::HOLY_STONE_ASK),
        on_yes: vec![Effect::TakeItem {
            item_id: DARK_CRYSTAL,
            quantity: 1,
        }],
        on_no: None,
    }];
    pages.extend(next);
    let mut script = NpcScript::new(HOLY_STONE, pages);
    script.on_end = vec![Effect::GiveItem {
        item_id: NECKLACE_OF_WISDOM,
        quantity: 1,
    }];
    script.inventory_full = Some(text(keys::THIRD_JOB_INVENTORY_FULL));
    script
}

/// Spiruna refines Dark Crystal Ore into a Dark Crystal, one at a time,
/// for those who finished her quest.
fn spiruna(player: &Player) -> NpcScript {
    if player.quests.state(SPIRUNA_QUEST) != QuestState::Completed {
        return NpcScript::new(SPIRUNA, vec![Page::Say(text(keys::SPIRUNA_OTHER))]);
    }
    let can_pay = player.meso >= REFINING_FEE
        && player.inventory.count(DARK_CRYSTAL_ORE) >= i32::from(ORE_PER_CRYSTAL);
    let on_yes = if can_pay {
        vec![
            Effect::TakeItem {
                item_id: DARK_CRYSTAL_ORE,
                quantity: ORE_PER_CRYSTAL,
            },
            Effect::SpendMeso(REFINING_FEE),
            Effect::GiveItem {
                item_id: DARK_CRYSTAL,
                quantity: 1,
            },
        ]
    } else {
        Vec::new()
    };
    let pages = vec![
        Page::Ask {
            text: text(keys::SPIRUNA_ASK),
            on_yes,
            on_no: None,
        },
        Page::Say(text(if can_pay {
            keys::SPIRUNA_DONE
        } else {
            keys::SPIRUNA_SHORT
        })),
    ];
    let mut script = NpcScript::new(SPIRUNA, pages);
    script.inventory_full = Some(text(keys::THIRD_JOB_INVENTORY_FULL));
    script
}

/// `begin5`: Robin's questions and answers, as many topics and pages as the
/// text has.
fn robin() -> NpcScript {
    let content = content::text();
    let choices = (0..)
        .map_while(|topic| {
            let label = keys::robin_choice(topic);
            content.contains(&label).then(|| {
                let pages = (0..)
                    .map(|page| keys::robin_answer(topic, page))
                    .take_while(|key| content.contains(key))
                    .map(|key| Page::Say(text(&key)))
                    .collect();
                (text(&label), pages)
            })
        })
        .collect();
    NpcScript::new(
        ROBIN,
        vec![Page::Menu {
            text: text(keys::ROBIN_MENU),
            choices,
        }],
    )
}

/// `hair_henesys1`: Natalie cuts or dyes the player's hair for a coupon.
fn natalie(player: &Player) -> NpcScript {
    let hair = player.stats.hair;
    let gender = usize::from(player.stats.gender.min(1));
    let cuts = NATALIE_HAIRS[gender].map(|style| style + hair % 10);
    let dyes = (0..8).map(|colour| hair / 10 * 10 + colour);
    NpcScript::new(
        NATALIE,
        vec![Page::Menu {
            text: text(keys::NATALIE_MENU),
            // The client can't show an empty choice of styles.
            choices: [
                (
                    keys::NATALIE_CUT,
                    hair_salon(
                        player,
                        cuts.into_iter(),
                        HAIRCUT_COUPON,
                        keys::NATALIE_CUT_PAGES,
                    ),
                ),
                (
                    keys::NATALIE_DYE,
                    hair_salon(player, dyes, DYE_COUPON, keys::NATALIE_DYE_PAGES),
                ),
            ]
            .into_iter()
            .filter_map(|(label, pages)| Some((text(label), pages?)))
            .collect(),
        }],
    )
}

/// A choice of the hair `styles` that exist, other than the one worn, for a
/// `coupon`; then what the stylist says after, or to a player without one.
/// `None` if there are no such styles.
fn hair_salon(
    player: &Player,
    styles: impl Iterator<Item = i32>,
    coupon: i32,
    [ask, done, no_coupon]: [&str; 3],
) -> Option<Vec<Page>> {
    let has_coupon = player.inventory.count(coupon) > 0;
    let choices = styles
        .filter(|&style| style != player.stats.hair && crate::items::has_style(style))
        .map(|style| {
            let effects = if has_coupon {
                vec![
                    Effect::TakeItem {
                        item_id: coupon,
                        quantity: 1,
                    },
                    Effect::SetStyle(style),
                ]
            } else {
                Vec::new()
            };
            (style, effects)
        })
        .collect::<Vec<_>>();
    (!choices.is_empty()).then(|| {
        vec![
            Page::Style {
                text: text(ask),
                choices,
            },
            Page::Say(text(if has_coupon { done } else { no_coupon })),
        ]
    })
}

fn text(key: &str) -> String {
    content::text().get(key, &[])
}

/// Map an NPC_TALK_MORE answer to a script input, given the prompt it
/// answers. `None` closes the conversation.
pub fn input(
    prompt: &Prompt,
    message_type: u8,
    response: i8,
    selection: Option<i32>,
) -> Option<ScriptInput> {
    match (prompt, message_type, response) {
        (Prompt::Text(_), 0, 1) => Some(ScriptInput::Next),
        (Prompt::Text(page), 0, 0) if page.has_previous => Some(ScriptInput::Previous),
        (Prompt::YesNo { .. }, 1, 1) => Some(ScriptInput::Yes),
        (Prompt::YesNo { .. }, 1, 0) => Some(ScriptInput::No),
        (Prompt::Selection { .. }, 4, 1) => selection.map(ScriptInput::Select),
        (Prompt::Style { .. }, 7, 1) => selection.map(ScriptInput::Select),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::QuestStatus;

    fn player(hp: i16) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.max_hp = 50;
        player.stats.hp = hp;
        player
    }

    fn kind(step: &Step) -> Option<u8> {
        step.prompt.as_ref().map(|p| match p {
            Prompt::Text(_) => 0,
            Prompt::YesNo { .. } => 1,
            Prompt::Selection { .. } => 4,
            Prompt::Style { .. } => 7,
        })
    }

    fn page_text(step: &Step) -> String {
        match &step.prompt {
            Some(Prompt::Text(page)) => page.text.clone(),
            other => panic!("text page expected, got {other:?}"),
        }
    }

    fn buttons(step: &Step) -> (bool, bool) {
        match &step.prompt {
            Some(Prompt::Text(page)) => (page.has_previous, page.has_next),
            other => panic!("text page expected, got {other:?}"),
        }
    }

    #[test]
    fn roger_starts_the_quest_with_an_apple_and_a_scratch() {
        let mut script = NpcScript::for_quest("q1021s", &player(50)).unwrap();
        assert_eq!(buttons(&script.begin()), (false, true));
        assert_eq!(
            buttons(&script.respond(ScriptInput::Next).unwrap()),
            (true, true)
        );
        let ask = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(kind(&ask), Some(1));
        let accepted = script.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            accepted.effects,
            [
                Effect::SetHp(25),
                Effect::GiveItem {
                    item_id: ROGERS_APPLE,
                    quantity: 1
                },
                Effect::StartQuest(ROGERS_APPLE_QUEST),
            ]
        );
        // No going back to the question.
        assert_eq!(buttons(&accepted), (false, true));
        assert_eq!(
            buttons(&script.respond(ScriptInput::Next).unwrap()),
            (true, true)
        );
        let end = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(end.prompt, None);
        assert_eq!(
            end.effects,
            [Effect::AvatarEffect("UI/tutorial.img/28".into())]
        );
    }

    #[test]
    fn declining_roger_changes_nothing() {
        let mut script = NpcScript::for_quest("q1021s", &player(50)).unwrap();
        script.respond(ScriptInput::Next);
        script.respond(ScriptInput::Next);
        assert_eq!(script.respond(ScriptInput::No), Some(Step::default()));
        // Answers that don't fit the prompt end the conversation.
        let mut script = NpcScript::for_quest("q1021s", &player(50)).unwrap();
        assert_eq!(script.respond(ScriptInput::Yes), None);
    }

    #[test]
    fn roger_says_so_when_the_apple_does_not_fit() {
        let mut script = NpcScript::for_quest("q1021s", &player(50)).unwrap();
        script.respond(ScriptInput::Next);
        script.respond(ScriptInput::Next);
        script.respond(ScriptInput::Yes);
        let full = script.inventory_full().unwrap();
        assert!(full.effects.is_empty());
        assert_eq!(buttons(&full), (false, false));
        assert_eq!(script.respond(ScriptInput::Next), Some(Step::default()));
    }

    #[test]
    fn roger_pays_only_after_full_recovery() {
        let mut script = NpcScript::for_quest("q1021e", &player(25)).unwrap();
        let not_yet = script.begin();
        assert!(not_yet.effects.is_empty());
        assert_eq!(buttons(&not_yet), (false, false));
        assert_eq!(script.respond(ScriptInput::Next), Some(Step::default()));

        let mut script = NpcScript::for_quest("q1021e", &player(50)).unwrap();
        script.respond(ScriptInput::Next).unwrap();
        let reward = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(buttons(&reward), (true, true));
        let paid = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(paid.prompt, None);
        assert!(paid.effects.contains(&Effect::GainExp(10)));
        assert!(paid
            .effects
            .contains(&Effect::CompleteQuest(ROGERS_APPLE_QUEST)));
    }

    #[test]
    fn robin_answers_any_topic_page_by_page() {
        let mut robin = NpcScript::for_npc(ROBIN, &player(50)).unwrap();
        let Some(Prompt::Selection { text, .. }) = &robin.begin().prompt else {
            panic!("menu expected");
        };
        assert_eq!(text.matches("#L").count(), 18);
        // Maple Island has four pages.
        let first = robin.respond(ScriptInput::Select(5)).unwrap();
        assert_eq!(buttons(&first), (false, true));
        for _ in 0..2 {
            robin.respond(ScriptInput::Next).unwrap();
        }
        assert_eq!(
            buttons(&robin.respond(ScriptInput::Next).unwrap()),
            (true, false)
        );
        assert_eq!(robin.respond(ScriptInput::Next), Some(Step::default()));
        let mut robin = NpcScript::for_npc(ROBIN, &player(50)).unwrap();
        assert_eq!(robin.respond(ScriptInput::Select(18)), None);
    }

    #[test]
    fn the_administrator_warps_to_lith_harbor_on_yes() {
        let mut admin = NpcScript::for_npc(MAPLE_ADMINISTRATOR, &player(50)).unwrap();
        assert_eq!(kind(&admin.begin()), Some(1));
        assert_eq!(
            admin.respond(ScriptInput::Yes).unwrap(),
            Step {
                prompt: None,
                effects: vec![Effect::Warp(LITH_HARBOR)]
            }
        );
        let mut admin = NpcScript::for_npc(MAPLE_ADMINISTRATOR, &player(50)).unwrap();
        assert_eq!(
            buttons(&admin.respond(ScriptInput::No).unwrap()),
            (false, false)
        );
        assert_eq!(admin.respond(ScriptInput::Next), Some(Step::default()));
    }

    /// Shanks's pages after yes, and what reading past the last one does.
    fn sail(player: &Player) -> (Vec<(bool, bool)>, Vec<Effect>) {
        let mut shanks = NpcScript::for_npc(SHANKS, player).unwrap();
        assert_eq!(kind(&shanks.begin()), Some(1));
        let mut step = shanks.respond(ScriptInput::Yes).unwrap();
        assert!(step.effects.is_empty());
        let mut pages = Vec::new();
        while step.prompt.is_some() {
            pages.push(buttons(&step));
            step = shanks.respond(ScriptInput::Next).unwrap();
        }
        (pages, step.effects)
    }

    fn traveller(level: u8, meso: i32, letter: bool) -> Player {
        let mut player = player(50);
        player.stats.level = level;
        player.meso = meso;
        if letter {
            player.inventory.insert(
                game::InventoryType::Etc,
                1,
                game::Item::stack(RECOMMENDATION_LETTER, 1),
            );
        }
        player
    }

    #[test]
    fn shanks_sails_for_the_fare_from_level_7() {
        let warp = Effect::Warp(LITH_HARBOR);
        assert_eq!(
            sail(&traveller(7, 150, false)),
            (
                vec![(false, true), (true, true)],
                vec![Effect::SpendMeso(SHIP_FARE), warp.clone()]
            )
        );
        // Too weak or too poor: he says so and stays.
        assert_eq!(
            sail(&traveller(6, 1000, false)),
            (vec![(false, true), (true, false)], vec![])
        );
        assert_eq!(
            sail(&traveller(7, 149, false)),
            (vec![(false, true), (true, false)], vec![])
        );
        // Lucas's letter pays, at any level.
        assert_eq!(
            sail(&traveller(1, 0, true)),
            (
                vec![(false, true), (true, true)],
                vec![
                    Effect::TakeItem {
                        item_id: RECOMMENDATION_LETTER,
                        quantity: 1
                    },
                    warp
                ]
            )
        );
        let mut shanks = NpcScript::for_npc(SHANKS, &traveller(7, 150, false)).unwrap();
        assert_eq!(
            buttons(&shanks.respond(ScriptInput::No).unwrap()),
            (false, false)
        );
        assert_eq!(shanks.respond(ScriptInput::Next), Some(Step::default()));
    }

    fn guilder(level: u8, meso: i32, guilded: bool) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = level;
        player.meso = meso;
        if guilded {
            player.guild = Some(guild::GuildTag {
                id: 1,
                name: "Testers".into(),
                emblem: guild::Emblem::default(),
            });
        }
        player
    }

    /// A guild of two whose master is `master`.
    fn testers(master: i32, capacity: u8) -> Guild {
        let member = |id, rank| guild::GuildMember {
            id,
            name: format!("Member{id}"),
            job: 0,
            level: 10,
            rank,
            online: true,
        };
        Guild {
            id: 1,
            name: "Testers".into(),
            titles: guild::DEFAULT_TITLES.map(String::from),
            capacity,
            emblem: guild::Emblem::default(),
            notice: String::new(),
            points: 0,
            alliance_id: 0,
            members: vec![member(master, guild::MASTER_RANK), member(master + 1, 3)],
        }
    }

    /// Heracle, with the menu entry `choice` picked.
    fn heracle_picks(player: &Player, guild: Option<&Guild>, choice: i32) -> (NpcScript, Step) {
        let mut heracle = NpcScript::heracle(player, guild);
        assert_eq!(kind(&heracle.begin()), Some(4));
        let step = heracle.respond(ScriptInput::Select(choice)).unwrap();
        (heracle, step)
    }

    fn says_one_page(mut heracle: NpcScript, step: &Step) {
        assert_eq!(buttons(step), (false, false));
        assert_eq!(heracle.respond(ScriptInput::Next), Some(Step::default()));
    }

    #[test]
    fn heracle_asks_for_a_guild_name_from_level_10_with_the_fee() {
        assert_eq!(NpcScript::for_npc(HERACLE, &guilder(10, 0, false)), None);
        let founder = guilder(10, guild::CREATE_COST, false);
        let (mut heracle, step) = heracle_picks(&founder, None, 0);
        assert_eq!(kind(&step), Some(1));
        let yes = heracle.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.prompt, None);
        assert_eq!(yes.effects, [Effect::OpenGuildNamePrompt]);

        // Too weak, too poor, or already in a guild: he says so.
        for player in [
            guilder(9, guild::CREATE_COST, false),
            guilder(10, guild::CREATE_COST - 1, false),
            guilder(10, guild::CREATE_COST, true),
        ] {
            let (heracle, step) = heracle_picks(&player, None, 0);
            says_one_page(heracle, &step);
        }
    }

    #[test]
    fn heracle_disbands_only_a_masters_guild() {
        let master = guilder(10, 0, true);
        let id = master.stats.id;
        let guild = testers(id, 10);
        let (mut heracle, step) = heracle_picks(&master, Some(&guild), 1);
        assert_eq!(kind(&step), Some(1));
        let yes = heracle.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.prompt, None);
        assert_eq!(yes.effects, [Effect::DisbandGuild]);

        let (mut heracle, _) = heracle_picks(&master, Some(&guild), 1);
        let no = heracle.respond(ScriptInput::No).unwrap();
        assert!(no.effects.is_empty());
        says_one_page(heracle, &no);

        // A member who isn't the master, or someone without a guild.
        for guild in [Some(testers(id - 1, 10)), None] {
            let (heracle, step) = heracle_picks(&master, guild.as_ref(), 1);
            says_one_page(heracle, &step);
        }
    }

    #[test]
    fn heracle_grows_a_masters_guild_by_5_for_the_steps_price() {
        let master = guilder(10, 500_000, true);
        let id = master.stats.id;
        let guild = testers(id, 10);
        let (mut heracle, step) = heracle_picks(&master, Some(&guild), 2);
        let Some(Prompt::YesNo { text, .. }) = &step.prompt else {
            panic!("a question expected, got {:?}", step.prompt);
        };
        assert!(text.contains("500000") && text.contains("15"), "{text}");
        let yes = heracle.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.prompt, None);
        assert_eq!(yes.effects, [Effect::GrowGuild { from: 10 }]);

        // Short of the price: asked all the same, then told.
        let poor = guilder(10, 499_999, true);
        let (mut heracle, _) = heracle_picks(&poor, Some(&guild), 2);
        let yes = heracle.respond(ScriptInput::Yes).unwrap();
        assert!(yes.effects.is_empty());
        says_one_page(heracle, &yes);

        // At 100 members, not the master, or without a guild.
        for guild in [
            Some(testers(id, guild::MAX_CAPACITY)),
            Some(testers(id - 1, 10)),
            None,
        ] {
            let (heracle, step) = heracle_picks(&master, guild.as_ref(), 2);
            says_one_page(heracle, &step);
        }
    }

    #[test]
    fn lea_opens_the_emblem_maker_for_the_master() {
        assert_eq!(NpcScript::for_npc(LEA, &guilder(10, 0, true)), None);
        let master = guilder(10, 0, true);
        let id = master.stats.id;
        let guild = testers(id, 10);
        let lea_picks = |guild: Option<&Guild>| {
            let mut lea = NpcScript::lea(&master, guild);
            assert_eq!(kind(&lea.begin()), Some(4));
            let step = lea.respond(ScriptInput::Select(0)).unwrap();
            (lea, step)
        };
        // Asked without a word of the master's mesos, which are checked
        // when the emblem comes back.
        let (mut lea, step) = lea_picks(Some(&guild));
        let Some(Prompt::YesNo { text, .. }) = &step.prompt else {
            panic!("a question expected, got {:?}", step.prompt);
        };
        assert!(text.contains("5000000"), "{text}");
        let yes = lea.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.prompt, None);
        assert_eq!(yes.effects, [Effect::OpenEmblemMaker]);

        let (mut lea, _) = lea_picks(Some(&guild));
        let no = lea.respond(ScriptInput::No).unwrap();
        assert!(no.effects.is_empty());
        says_one_page(lea, &no);

        // A member who isn't the master, or someone without a guild.
        for guild in [Some(testers(id - 1, 10)), None] {
            let (lea, step) = lea_picks(guild.as_ref());
            says_one_page(lea, &step);
        }
    }

    #[test]
    fn the_booths_sell_a_ticket_for_5000_mesos() {
        let buyer = traveller(10, 5000, false);
        let mut joel = NpcScript::for_npc(JOEL, &buyer).unwrap();
        assert_eq!(kind(&joel.begin()), Some(1));
        assert_eq!(
            joel.respond(ScriptInput::Yes).unwrap(),
            Step {
                prompt: None,
                effects: vec![
                    Effect::SpendMeso(5000),
                    Effect::GiveItem {
                        item_id: conti::TO_ORBIS.ticket,
                        quantity: 1
                    }
                ]
            }
        );
        let mut agatha = NpcScript::for_npc(AGATHA, &traveller(10, 30000, false)).unwrap();
        assert_eq!(kind(&agatha.begin()), Some(4));
        assert_eq!(
            kind(&agatha.respond(ScriptInput::Select(2)).unwrap()),
            Some(1)
        );
        assert_eq!(
            agatha.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::SpendMeso(30000),
                Effect::GiveItem {
                    item_id: conti::TO_LEAFRE.ticket,
                    quantity: 1
                }
            ]
        );
        assert!(agatha.inventory_full().is_some(), "a full Etc tab is told");
        // The other towns' booths sell the ticket back to Orbis.
        let mut mel = NpcScript::for_npc(2040000, &buyer).unwrap();
        mel.begin();
        assert!(mel.respond(ScriptInput::Yes).unwrap().effects.is_empty());
        let mut mel = NpcScript::for_npc(2040000, &traveller(10, 6000, false)).unwrap();
        assert_eq!(
            mel.respond(ScriptInput::Yes).unwrap().effects[1],
            Effect::GiveItem {
                item_id: 4031045,
                quantity: 1
            }
        );

        // Short of mesos: yes is answered with why not.
        let mut joel = NpcScript::for_npc(JOEL, &traveller(10, 4999, false)).unwrap();
        let short = joel.respond(ScriptInput::Yes).unwrap();
        assert!(short.effects.is_empty());
        assert_eq!(buttons(&short), (false, false));
    }

    #[test]
    fn the_usher_boards_ticket_holders_only_while_the_ship_boards() {
        const HOUR: i64 = 3_600_000;
        let minute = |m: i64| 7 * HOUR + m * 60_000;
        let mut holder = traveller(10, 0, false);
        holder.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(conti::TO_ORBIS.ticket, 1),
        );
        let mut cherry = usher(CHERRY, &conti::TO_ORBIS, &holder, minute(12));
        assert_eq!(kind(&cherry.begin()), Some(1));
        assert_eq!(
            cherry.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::RequireBoarding(101000300),
                Effect::TakeItem {
                    item_id: conti::TO_ORBIS.ticket,
                    quantity: 1
                },
                Effect::Warp(101000301),
            ]
        );
        // No ticket, boarding closed, or the ship gone: she says so.
        for cherry in [
            usher(
                CHERRY,
                &conti::TO_ORBIS,
                &traveller(10, 0, false),
                minute(12),
            ),
            usher(CHERRY, &conti::TO_ORBIS, &holder, minute(14)),
            usher(CHERRY, &conti::TO_ORBIS, &holder, minute(5)),
        ] {
            assert_eq!(buttons(&cherry.begin()), (false, false));
        }
    }

    #[test]
    fn the_crew_let_passengers_off_and_isa_shows_the_way() {
        let waiting = |map_id| {
            let mut passenger = player(50);
            passenger.stats.map_id = map_id;
            passenger
        };
        // Back to the platform the waiting room boards from; Rosey has two.
        for (guide, room, platform) in [
            (PURIN, 101000301, 101000300),
            (ERIN, 200000112, 200000111),
            (2041001, 200000122, 200000121),
            (2041001, 220000111, 220000110),
            (2082002, 240000111, 240000110),
            (2102001, 260000110, 260000100),
        ] {
            let mut script = NpcScript::for_npc(guide, &waiting(room)).unwrap();
            assert_eq!(
                script.respond(ScriptInput::Yes).unwrap().effects,
                [Effect::Warp(platform)]
            );
        }
        assert!(NpcScript::for_npc(2041001, &waiting(220000100)).is_none());

        for (choice, passage) in [(0, ORBIS_TUNNEL), (2, 200000130), (3, 200000150)] {
            let mut isa = NpcScript::for_npc(ISA, &player(50)).unwrap();
            assert_eq!(kind(&isa.begin()), Some(4));
            assert_eq!(
                kind(&isa.respond(ScriptInput::Select(choice)).unwrap()),
                Some(1)
            );
            assert_eq!(
                isa.respond(ScriptInput::Yes).unwrap().effects,
                [Effect::Warp(passage)]
            );
        }
    }

    #[test]
    fn the_subway_gates_ask_first_and_bell_sells_tickets_and_lets_riders_out() {
        const HOUR: i64 = 3_600_000;
        // A train leaves on the hour and every 5 minutes; boarding is open
        // from 4 to 1 minutes before.
        let minute = |m: i64| 7 * HOUR + m * 60_000;
        let mut holder = traveller(10, 0, false);
        holder.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(conti::TO_NLC.ticket, 1),
        );
        let mut gate = subway_gate(1052007, &conti::TO_NLC, &holder, minute(2));
        assert_eq!(kind(&gate.begin()), Some(1));
        assert_eq!(
            gate.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::RequireBoarding(103000100),
                Effect::TakeItem {
                    item_id: 4031711,
                    quantity: 1
                },
                Effect::Warp(600010004),
            ]
        );
        // Without a ticket the gate still asks, then says why not.
        let mut gate = subway_gate(
            9201068,
            &conti::TO_KERNING_CITY,
            &traveller(10, 0, false),
            minute(2),
        );
        assert_eq!(kind(&gate.begin()), Some(1));
        let refused = gate.respond(ScriptInput::Yes).unwrap();
        assert!(refused.effects.is_empty());
        assert_eq!(buttons(&refused), (false, false));
        // Boarding closed, or the train gone.
        for t in [minute(4), minute(5)] {
            let gate = subway_gate(1052007, &conti::TO_NLC, &holder, t);
            assert_eq!(buttons(&gate.begin()), (false, false));
        }

        // Bell sells the ticket for the line leaving her platform.
        for (platform, ticket) in [(103000100, 4031711), (600010001, 4031713)] {
            let mut buyer = traveller(10, 5000, false);
            buyer.stats.map_id = platform;
            let mut bell = NpcScript::for_npc(BELL, &buyer).unwrap();
            assert_eq!(kind(&bell.begin()), Some(4));
            assert_eq!(
                kind(&bell.respond(ScriptInput::Select(0)).unwrap()),
                Some(1)
            );
            assert_eq!(
                bell.respond(ScriptInput::Yes).unwrap().effects,
                [
                    Effect::SpendMeso(5000),
                    Effect::GiveItem {
                        item_id: ticket,
                        quantity: 1
                    }
                ]
            );
            assert!(bell.inventory_full().is_some());
        }
        // ...and lets riders back out of a waiting room to its platform.
        for (room, platform) in [(600010004, 103000100), (600010002, 600010001)] {
            let mut rider = traveller(10, 0, false);
            rider.stats.map_id = room;
            let mut bell = NpcScript::for_npc(BELL, &rider).unwrap();
            assert_eq!(
                bell.respond(ScriptInput::Yes).unwrap().effects,
                [Effect::Warp(platform)]
            );
        }
        assert!(NpcScript::for_npc(BELL, &traveller(10, 0, false)).is_none());
    }

    #[test]
    fn the_nlc_taxi_runs_between_new_leaf_city_and_the_haunted_house() {
        for (from, to) in [(600000000, 682000000), (682000000, 600000000)] {
            let mut taxi = NpcScript::for_npc(NLC_TAXI, &rider(100, 15000, from)).unwrap();
            assert_eq!(kind(&taxi.begin()), Some(1));
            assert_eq!(
                taxi.respond(ScriptInput::Yes).unwrap().effects,
                [Effect::SpendMeso(15000), Effect::WarpToPortal(to, 0)]
            );
        }
        let mut taxi = NpcScript::for_npc(NLC_TAXI, &rider(100, 14999, 600000000)).unwrap();
        taxi.begin();
        assert!(taxi.respond(ScriptInput::Yes).unwrap().effects.is_empty());
    }

    #[test]
    fn cloto_gives_a_pass_for_the_right_number_of_coupons() {
        let member = |new, passed| Cloto::Member {
            question: 6,
            new,
            passed,
        };
        let holding = |item_id, quantity| hunter(&[(item_id, quantity)], &[]);
        let take = |item_id, quantity| Effect::TakeItem { item_id, quantity };

        // Asked first; then 8 coupons, no more and no fewer, earn a pass.
        let asked = NpcScript::cloto(member(true, false), &holding(events::COUPON, 8)).begin();
        assert_eq!(kind(&asked), Some(0));
        assert!(asked.effects.is_empty());
        for wrong in [7, 9] {
            let step = NpcScript::cloto(member(false, false), &holding(events::COUPON, wrong));
            assert!(step.begin().effects.is_empty());
        }
        let mut right = NpcScript::cloto(member(false, false), &holding(events::COUPON, 8));
        assert_eq!(
            right.begin().effects,
            [
                take(events::COUPON, 8),
                Effect::GiveItem {
                    item_id: events::PASS,
                    quantity: 1
                },
                Effect::EventCommand(EventCommand::PassGiven),
            ]
        );
        assert!(right.respond(ScriptInput::Next).unwrap().effects.is_empty());
        let thanks = NpcScript::cloto(member(false, true), &holding(events::COUPON, 8)).begin();
        assert!(thanks.effects.is_empty());

        // The leader hands in a pass from each of the others.
        let leader = Cloto::Leader {
            stage: 1,
            needed: 3,
        };
        let short = NpcScript::cloto(leader, &holding(events::PASS, 2)).begin();
        assert!(short.effects.is_empty());
        let clear = NpcScript::cloto(leader, &holding(events::PASS, 4)).begin();
        assert_eq!(
            clear.effects,
            [
                take(events::PASS, 3),
                Effect::EventCommand(EventCommand::ClearStage(1))
            ]
        );
        let cleared = NpcScript::cloto(Cloto::Cleared, &holding(events::PASS, 4)).begin();
        assert!(cleared.effects.is_empty());
    }

    #[test]
    fn cloto_sets_the_leader_a_puzzle_and_clears_it_when_answered() {
        let player = hunter(&[], &[]);
        let says = |view| {
            let page = NpcScript::cloto(view, &player).begin();
            assert!(page.effects.is_empty());
            page_text(&page)
        };
        let intros: Vec<String> = (2..=4).map(|stage| says(Cloto::Intro { stage })).collect();
        assert!(intros[0] != intros[1] && intros[1] != intros[2]);
        assert_eq!(says(Cloto::Puzzle { stage: 3, combo: 7 }), intros[1]);
        assert!(!intros.contains(&says(Cloto::Follow)));

        let wrong = NpcScript::cloto_answer(3, false).begin();
        assert!(wrong.effects.is_empty());
        let right = NpcScript::cloto_answer(3, true).begin();
        assert_eq!(
            right.effects,
            [Effect::EventCommand(EventCommand::ClearStage(3))]
        );
        assert_ne!(page_text(&wrong), page_text(&right));
    }

    #[test]
    fn the_leader_hands_cloto_the_last_stage_s_ten_passes() {
        let holding = |quantity| hunter(&[(events::PASS, quantity)], &[]);
        let says = |view, passes| {
            let page = NpcScript::cloto(view, &holding(passes)).begin();
            (page_text(&page), page.effects)
        };
        let leader = Cloto::Leader {
            stage: 5,
            needed: 10,
        };
        let (short, effects) = says(leader, 9);
        assert!(effects.is_empty());
        let (clear, effects) = says(leader, 11);
        assert_eq!(
            effects,
            [
                Effect::TakeItem {
                    item_id: events::PASS,
                    quantity: 10
                },
                Effect::EventCommand(EventCommand::ClearStage(5))
            ]
        );
        // Not stage 1's lines.
        let first = Cloto::Leader {
            stage: 1,
            needed: 10,
        };
        assert_ne!(short, says(first, 9).0);
        assert_ne!(clear, says(first, 11).0);

        let (gather, effects) = says(Cloto::Gather, 10);
        assert!(effects.is_empty());
        let (won, effects) = says(Cloto::Won, 10);
        assert!(effects.is_empty());
        let (cleared, _) = says(Cloto::Cleared, 10);
        assert!(gather != won && won != cleared && gather != short);
    }

    #[test]
    fn lakelis_takes_a_ready_party_in_and_tells_the_rest_why_not() {
        let mut ready = NpcScript::lakelis(Entry::Ready);
        assert_eq!(kind(&ready.begin()), Some(1));
        assert_eq!(
            ready.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::StartEvent(EventKind::Kpq)]
        );
        // Refused on the way in: she gives the terms.
        let refused = ready.refused().unwrap();
        assert_eq!(kind(&refused), Some(0));
        for entry in [
            Entry::NoParty,
            Entry::NotLeader,
            Entry::NotEligible,
            Entry::Busy,
        ] {
            let mut lakelis = NpcScript::lakelis(entry);
            assert_eq!(kind(&lakelis.begin()), Some(0));
            assert!(lakelis
                .respond(ScriptInput::Next)
                .unwrap()
                .effects
                .is_empty());
        }
    }

    #[test]
    fn nella_lets_players_out_of_the_party_quest_then_home() {
        for (from, to) in [(103000801, 103000890), (103000890, 103000000)] {
            let mut nella = NpcScript::for_npc(NELLA, &rider(100, 0, from)).unwrap();
            assert_eq!(kind(&nella.begin()), Some(1));
            assert_eq!(
                nella.respond(ScriptInput::Yes).unwrap().effects,
                [Effect::Warp(to)]
            );
        }
        assert_eq!(NpcScript::for_npc(NELLA, &rider(100, 0, 103000000)), None);
    }

    #[test]
    fn jeff_lets_level_50_players_down_to_sharp_cliff() {
        let mut jeff = NpcScript::for_npc(JEFF, &traveller(50, 0, false)).unwrap();
        assert_eq!(kind(&jeff.begin()), Some(0));
        assert_eq!(kind(&jeff.respond(ScriptInput::Next).unwrap()), Some(1));
        assert_eq!(
            jeff.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::WarpToPortal(211040300, 5)]
        );

        let mut jeff = NpcScript::for_npc(JEFF, &traveller(49, 0, false)).unwrap();
        jeff.begin();
        let refusal = jeff.respond(ScriptInput::Next).unwrap();
        assert_eq!(kind(&refusal), Some(0));
        assert!(jeff.respond(ScriptInput::Next).unwrap().effects.is_empty());
    }

    #[test]
    fn jeff_sends_an_orihalcon_hammers_holder_to_ice_valley_at_once() {
        let mut player = traveller(30, 0, false);
        player.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(ORIHALCON_HAMMER, 1),
        );
        let jeff = NpcScript::for_npc(JEFF, &player).unwrap();
        assert_eq!(
            jeff.begin(),
            Step {
                prompt: None,
                effects: vec![Effect::WarpToPortal(921100100, 1)],
            }
        );
    }

    fn with_quests(quests: &[(i16, bool)]) -> Player {
        let mut player = traveller(70, 0, false);
        for &(quest_id, completed) in quests {
            let status = if completed {
                QuestStatus::Completed {
                    at: std::time::SystemTime::now(),
                }
            } else {
                QuestStatus::Started { kills: Vec::new() }
            };
            player.quests.upsert(game::QuestRecord { quest_id, status });
        }
        player
    }

    #[test]
    fn the_time_gate_opens_the_eras_guardian_nex_was_beaten_in() {
        let says = |quests: &[(i16, bool)]| {
            let gate = NpcScript::for_npc(TIME_GATE, &with_quests(quests)).unwrap();
            kind(&gate.begin())
        };
        assert_eq!(says(&[]), Some(0));
        assert_eq!(says(&[(3718, false), (3719, true)]), Some(0));
        assert_eq!(says(&[(3718, true), (3719, false)]), Some(0));

        // Only eras in an unbroken line from the first are offered.
        let player = with_quests(&[(3718, true), (3719, true), (3724, true), (3736, true)]);
        let mut gate = NpcScript::for_npc(TIME_GATE, &player).unwrap();
        let menu = gate.begin();
        assert_eq!(
            menu_text(&menu),
            "#L0##b#m240070100##k#l\n#L1##b#m240070200##k#l"
        );
        assert_eq!(
            gate.respond(ScriptInput::Select(1)),
            Some(Step {
                prompt: None,
                effects: vec![Effect::WarpToPortal(240070200, 1)],
            })
        );
        let mut gate = NpcScript::for_npc(TIME_GATE, &player).unwrap();
        gate.begin();
        assert_eq!(gate.respond(ScriptInput::Select(2)), None);
    }

    #[test]
    fn shane_lets_players_into_the_forest_of_patience_for_a_fare() {
        let ask = |player: &Player| {
            let mut shane = NpcScript::for_npc(SHANE, player).unwrap();
            assert_eq!(kind(&shane.begin()), Some(1));
            shane.respond(ScriptInput::Yes).unwrap()
        };
        let mut player = with_quests(&[]);
        player.meso = SHANE_FARE;
        for (level, step) in [(25, 101000100), (49, 101000100), (50, 101000102)] {
            player.stats.level = level;
            assert_eq!(
                ask(&player).effects,
                [Effect::SpendMeso(SHANE_FARE), Effect::WarpToPortal(step, 0)]
            );
        }
        // The herb quest the player is on decides the step.
        let mut player = with_quests(&[(2050, false)]);
        player.meso = SHANE_FARE;
        assert_eq!(ask(&player).effects[1], Effect::WarpToPortal(101000100, 0));
        let mut player = with_quests(&[(2051, false)]);
        (player.meso, player.stats.level) = (SHANE_FARE, 30);
        assert_eq!(ask(&player).effects[1], Effect::WarpToPortal(101000102, 0));

        // Short of the fare: a refusal and no ride.
        player.meso = SHANE_FARE - 1;
        let short = ask(&player);
        assert!(short.effects.is_empty());
        assert_eq!(kind(&short), Some(0));

        player.stats.level = 24;
        let shane = NpcScript::for_npc(SHANE, &player).unwrap();
        assert_eq!(kind(&shane.begin()), Some(0));

        let mut louis = NpcScript::for_npc(LOUIS, &player).unwrap();
        assert_eq!(kind(&louis.begin()), Some(1));
        assert_eq!(
            louis.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::WarpToPortal(101000000, 0)]
        );
    }

    #[test]
    fn the_statues_lead_into_and_out_of_the_deep_forest() {
        let mut statue = NpcScript::for_npc(MYSTERIOUS_STATUE, &with_quests(&[])).unwrap();
        assert_eq!(kind(&statue.begin()), Some(0));
        assert_eq!(statue.respond(ScriptInput::Next), Some(Step::default()));

        let player = with_quests(&[(2052, true), (2053, false)]);
        let mut statue = NpcScript::for_npc(MYSTERIOUS_STATUE, &player).unwrap();
        statue.begin();
        let menu = statue.respond(ScriptInput::Next).unwrap();
        assert!(menu_text(&menu).ends_with("#L1##b#m105040312##k#l"));
        assert!(!menu_text(&menu).contains("#L2#"));
        assert_eq!(
            statue.respond(ScriptInput::Select(1)).unwrap().effects,
            [Effect::WarpToPortal(105040312, 0)]
        );

        let mut crumbling = NpcScript::for_npc(CRUMBLING_STATUE, &player).unwrap();
        assert_eq!(kind(&crumbling.begin()), Some(1));
        assert_eq!(
            crumbling.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::WarpToPortal(105040300, 0)]
        );
    }

    fn rider(job: i16, meso: i32, map_id: i32) -> Player {
        let mut player = traveller(10, meso, false);
        player.stats.job = job;
        player.stats.map_id = map_id;
        player
    }

    fn menu_text(step: &Step) -> &str {
        match &step.prompt {
            Some(Prompt::Selection { text, .. }) => text,
            other => panic!("menu expected, got {other:?}"),
        }
    }

    #[test]
    fn the_cabs_charge_beginners_a_tenth_of_the_fare() {
        // Henesys's Regular Cab: past the greeting, the other five towns.
        let mut cab = NpcScript::for_npc(1012000, &rider(100, 1000, 100000000)).unwrap();
        assert_eq!(buttons(&cab.begin()), (false, true));
        let menu = cab.respond(ScriptInput::Next).unwrap();
        assert_eq!(menu_text(&menu).matches("#L").count(), 5);
        assert!(!menu_text(&menu).contains("#m100000000#"));
        assert!(!menu_text(&menu).contains(&text(keys::TAXI_DISCOUNT)));
        assert_eq!(kind(&cab.respond(ScriptInput::Select(2)).unwrap()), Some(1));
        assert_eq!(
            cab.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::SpendMeso(800), Effect::Warp(101000000)]
        );

        // A beginner is told of the discount and pays a tenth.
        let mut cab = NpcScript::for_npc(1012000, &rider(0, 100, 100000000)).unwrap();
        cab.begin();
        let menu = cab.respond(ScriptInput::Next).unwrap();
        assert!(menu_text(&menu).starts_with(&text(keys::TAXI_DISCOUNT)));
        cab.respond(ScriptInput::Select(0));
        assert_eq!(
            cab.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::SpendMeso(100), Effect::Warp(104000000)]
        );

        // Short of the fare, yes is answered with why not; no is declined.
        let mut cab = NpcScript::for_npc(1022001, &rider(0, 79, 102000000)).unwrap();
        cab.begin();
        cab.respond(ScriptInput::Next);
        cab.respond(ScriptInput::Select(2));
        let short = cab.respond(ScriptInput::Yes).unwrap();
        assert!(short.effects.is_empty());
        assert_eq!(buttons(&short), (false, false));
        let mut cab = NpcScript::for_npc(1022001, &rider(0, 80, 102000000)).unwrap();
        cab.begin();
        cab.respond(ScriptInput::Next);
        cab.respond(ScriptInput::Select(2));
        let no = cab.respond(ScriptInput::No).unwrap();
        assert!(no.effects.is_empty());
        assert_eq!(buttons(&no), (false, false));
        assert!(cab.refused().is_some(), "a fare spent meanwhile is told");

        // The VIP Cab goes to Ant Tunnel Park.
        for (job, fare) in [(100, 10000), (0, 1000)] {
            let mut vip = NpcScript::for_npc(1032005, &rider(job, 10000, 101000000)).unwrap();
            assert_eq!(buttons(&vip.begin()), (false, true));
            assert_eq!(kind(&vip.respond(ScriptInput::Next).unwrap()), Some(1));
            assert_eq!(
                vip.respond(ScriptInput::Yes).unwrap().effects,
                [Effect::SpendMeso(fare), Effect::Warp(ANT_TUNNEL_PARK)]
            );
        }
    }

    #[test]
    fn ossyrias_taxis_take_the_fare_to_the_far_end() {
        // The Danger Zone Taxi from Ludibrium stops at the Path of Time's
        // second spawn point; it doesn't run from towns it doesn't serve.
        let mut taxi = NpcScript::for_npc(DANGER_ZONE_TAXI, &rider(200, 25000, 220000000)).unwrap();
        assert_eq!(buttons(&taxi.begin()), (false, true));
        assert_eq!(kind(&taxi.respond(ScriptInput::Next).unwrap()), Some(1));
        assert_eq!(
            taxi.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::SpendMeso(25000), Effect::WarpToPortal(220050300, 1)]
        );
        assert!(NpcScript::for_npc(DANGER_ZONE_TAXI, &rider(200, 25000, 104000000)).is_none());
        let mut taxi = NpcScript::for_npc(DANGER_ZONE_TAXI, &rider(200, 64999, 240000000)).unwrap();
        taxi.begin();
        taxi.respond(ScriptInput::Next);
        assert!(taxi.respond(ScriptInput::Yes).unwrap().effects.is_empty());

        // The Camel Cab goes to the town at the other end.
        for (from, to) in [(OUTSIDE_ARIANT, MAGATIA), (260020700, ARIANT)] {
            let mut cab = NpcScript::for_npc(CAMEL_CAB, &rider(200, 1500, from)).unwrap();
            assert_eq!(kind(&cab.begin()), Some(1));
            assert_eq!(
                cab.respond(ScriptInput::Yes).unwrap().effects,
                [Effect::SpendMeso(1500), Effect::WarpToPortal(to, 0)]
            );
        }

        // The Dolphin Taxi takes a coupon instead of the fare down, and
        // the fare across.
        let mut holder = rider(200, 0, AQUARIUM);
        holder.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(DOLPHIN_TAXI_COUPON, 1),
        );
        let mut dolphin = NpcScript::for_npc(DOLPHIN, &holder).unwrap();
        assert_eq!(kind(&dolphin.begin()), Some(4));
        dolphin.respond(ScriptInput::Select(0));
        assert_eq!(
            dolphin.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::TakeItem {
                    item_id: DOLPHIN_TAXI_COUPON,
                    quantity: 1
                },
                Effect::WarpToPortal(SHARP_UNKNOWN, 2)
            ]
        );
        let pier = rider(200, 11000, HERB_TOWN_PIER);
        for (choice, effects) in [
            (
                0,
                [
                    Effect::SpendMeso(1000),
                    Effect::WarpToPortal(SHARP_UNKNOWN, 2),
                ],
            ),
            (1, [Effect::SpendMeso(10000), Effect::Warp(AQUARIUM)]),
        ] {
            let mut dolphin = NpcScript::for_npc(DOLPHIN, &pier).unwrap();
            dolphin.begin();
            dolphin.respond(ScriptInput::Select(choice));
            assert_eq!(dolphin.respond(ScriptInput::Yes).unwrap().effects, effects);
        }
        let mut dolphin = NpcScript::for_npc(DOLPHIN, &rider(200, 9999, AQUARIUM)).unwrap();
        dolphin.begin();
        assert_eq!(
            kind(&dolphin.respond(ScriptInput::Select(1)).unwrap()),
            Some(1)
        );
        assert!(dolphin
            .respond(ScriptInput::Yes)
            .unwrap()
            .effects
            .is_empty());
    }

    #[test]
    fn instructors_advance_beginners_who_qualify() {
        let mut beginner = player(50);
        (beginner.stats.job, beginner.stats.str) = (0, 4);
        beginner.stats.dex = 25;
        let mut athena = NpcScript::for_npc(1012100, &beginner).unwrap();
        assert_eq!(buttons(&athena.begin()), (false, true));
        assert_eq!(kind(&athena.respond(ScriptInput::Next).unwrap()), Some(1));
        let accepted = athena.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            accepted.effects,
            [
                Effect::TakeFirstJob(300),
                Effect::AddSlotRow(InventoryType::Equip),
                Effect::AddSlotRow(InventoryType::Use),
                Effect::GiveItem {
                    item_id: 1452051,
                    quantity: 1
                },
                Effect::GiveItem {
                    item_id: 2060000,
                    quantity: 6000
                },
            ]
        );
        assert_eq!(buttons(&accepted), (false, true));
        for _ in 0..5 {
            athena.respond(ScriptInput::Next).unwrap();
        }
        assert_eq!(athena.respond(ScriptInput::Next), Some(Step::default()));
        assert!(athena.inventory_full().is_some());

        // Too little STR: the intro, then a word about training.
        let mut balrog = NpcScript::for_npc(1022000, &beginner).unwrap();
        assert_eq!(
            buttons(&balrog.respond(ScriptInput::Next).unwrap()),
            (true, false)
        );

        // Past beginner, a single page.
        beginner.stats.job = 300;
        let athena = NpcScript::for_npc(1012100, &beginner).unwrap();
        assert_eq!(buttons(&athena.begin()), (false, false));
    }

    #[test]
    fn first_jobs_pass_the_examiners_test_for_their_second_job() {
        const LETTER: i32 = 4031010;
        let bowman = |items: &[(i16, i32, i16)]| {
            let mut player = player(50);
            (player.stats.job, player.stats.level) = (300, 30);
            for &(slot, item_id, quantity) in items {
                let item = game::Item::stack(item_id, quantity);
                player.inventory.insert(InventoryType::Etc, slot, item);
            }
            player
        };
        let give = |item_id| Effect::GiveItem {
            item_id,
            quantity: 1,
        };
        let take = |item_id, quantity| Effect::TakeItem { item_id, quantity };

        // Athena gives a letter for the examiner, then sends the player on.
        let mut athena = NpcScript::for_npc(1012100, &bowman(&[])).unwrap();
        assert_eq!(buttons(&athena.begin()), (false, true));
        assert_eq!(
            athena.respond(ScriptInput::Next).unwrap().effects,
            [give(LETTER)]
        );
        assert!(athena.inventory_full().is_some());
        let with_letter = bowman(&[(1, LETTER, 1)]);
        let athena = NpcScript::for_npc(1012100, &with_letter).unwrap();
        assert_eq!(buttons(&athena.begin()), (false, false));

        // The examiner lets letter holders into the test, letter and all.
        let mut examiner = NpcScript::for_npc(1072002, &with_letter).unwrap();
        examiner.respond(ScriptInput::Next).unwrap();
        let entered = examiner.respond(ScriptInput::Yes).unwrap();
        assert_eq!(entered.effects, [Effect::Warp(108000100)]);
        let examiner = NpcScript::for_npc(1072002, &bowman(&[])).unwrap();
        assert_eq!(buttons(&examiner.begin()), (false, false));

        // The proctor sees anyone out, and trades every marble, once there
        // are 30, and the letter for the proof.
        let out = Effect::WarpToPortal(106010000, 9);
        let mut proctor = NpcScript::for_npc(1072006, &with_letter).unwrap();
        assert_eq!(kind(&proctor.begin()), Some(1));
        assert_eq!(
            proctor.respond(ScriptInput::Yes).unwrap().effects,
            [out.clone()]
        );
        let passed = bowman(&[(1, LETTER, 1), (2, DARK_MARBLE, 25), (3, DARK_MARBLE, 6)]);
        let mut proctor = NpcScript::for_npc(1072006, &passed).unwrap();
        assert_eq!(
            proctor.respond(ScriptInput::Next).unwrap().effects,
            [
                take(DARK_MARBLE, 31),
                take(LETTER, 1),
                give(PROOF_OF_A_HERO),
                out
            ]
        );

        // For the proof, Athena offers each path.
        let mut athena = NpcScript::for_npc(1012100, &bowman(&[(1, PROOF_OF_A_HERO, 1)])).unwrap();
        assert_eq!(kind(&athena.begin()), Some(4));
        assert_eq!(athena.respond(ScriptInput::Select(2)), None, "two paths");
        athena.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(kind(&athena.respond(ScriptInput::Next).unwrap()), Some(1));
        assert_eq!(
            athena.respond(ScriptInput::Yes).unwrap().effects,
            [
                take(PROOF_OF_A_HERO, 1),
                Effect::TakeSecondJob(320),
                Effect::AddSlotRow(InventoryType::Etc),
            ]
        );

        // Below level 30, the first job's single page.
        let mut young = bowman(&[]);
        young.stats.level = 29;
        let athena = NpcScript::for_npc(1012100, &young).unwrap();
        assert_eq!(buttons(&athena.begin()), (false, false));
    }

    #[test]
    fn kyrin_makes_pirates_and_sends_them_to_her_quests_tests() {
        let mut beginner = player(50);
        (beginner.stats.job, beginner.stats.dex) = (0, 20);
        let mut kyrin = NpcScript::for_npc(KYRIN, &beginner).unwrap();
        kyrin.respond(ScriptInput::Next).unwrap();
        let accepted = kyrin.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            accepted.effects[..3],
            [
                Effect::TakeFirstJob(500),
                Effect::AddSlotRow(InventoryType::Equip),
                Effect::AddSlotRow(InventoryType::Etc),
            ]
        );
        assert_eq!(
            accepted.effects.len(),
            7,
            "a knuckle, a gun, two sets of bullets"
        );
        for _ in 0..5 {
            kyrin.respond(ScriptInput::Next).unwrap();
        }
        assert_eq!(kyrin.respond(ScriptInput::Next), Some(Step::default()));

        let pirate = |quest: Option<(i16, QuestStatus)>, map, crystals| {
            let mut player = player(50);
            (player.stats.job, player.stats.level, player.stats.map_id) = (500, 30, map);
            if let Some((quest_id, status)) = quest {
                player.quests.upsert(game::QuestRecord { quest_id, status });
            }
            if crystals > 0 {
                let item = game::Item::stack(4031856, crystals);
                player.inventory.insert(InventoryType::Etc, 1, item);
            }
            player
        };
        let started = || QuestStatus::Started { kills: Vec::new() };

        // Without a quest, a word about them; with Brawler's started, the
        // way into its test.
        let kyrin = NpcScript::for_npc(KYRIN, &pirate(None, KYRIN_MAP, 0)).unwrap();
        assert_eq!(buttons(&kyrin.begin()), (false, false));
        let mut kyrin =
            NpcScript::for_npc(KYRIN, &pirate(Some((2191, started())), KYRIN_MAP, 0)).unwrap();
        assert_eq!(kind(&kyrin.begin()), Some(1));
        assert_eq!(
            kyrin.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::Warp(108000502)]
        );

        // The proctor sees anyone out to Kyrin, after a word of praise for
        // the crystals, which stay for the quest.
        let out = Effect::WarpToPortal(KYRIN_MAP, 0);
        let mut proctor = NpcScript::for_npc(PIRATE_PROCTOR, &pirate(None, 108000502, 14)).unwrap();
        assert_eq!(kind(&proctor.begin()), Some(1));
        assert_eq!(
            proctor.respond(ScriptInput::Yes).unwrap().effects,
            [out.clone()]
        );
        let elsewhere = pirate(None, 108000501, 15);
        let proctor = NpcScript::for_npc(PIRATE_PROCTOR, &elsewhere).unwrap();
        assert_eq!(kind(&proctor.begin()), Some(1), "the Gunslinger's test");
        let mut proctor = NpcScript::for_npc(PIRATE_PROCTOR, &pirate(None, 108000502, 15)).unwrap();
        assert_eq!(buttons(&proctor.begin()), (false, true));
        assert_eq!(proctor.respond(ScriptInput::Next).unwrap().effects, [out]);

        // With the quest completed, the job.
        let done = QuestStatus::Completed {
            at: std::time::SystemTime::UNIX_EPOCH,
        };
        let mut kyrin =
            NpcScript::for_npc(KYRIN, &pirate(Some((2191, done)), KYRIN_MAP, 0)).unwrap();
        assert_eq!(kind(&kyrin.respond(ScriptInput::Next).unwrap()), Some(1));
        assert_eq!(
            kyrin.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::TakeSecondJob(510),
                Effect::AddSlotRow(InventoryType::Etc),
            ]
        );
    }

    #[test]
    fn answers_must_fit_the_prompt() {
        let page = Prompt::Text(DialoguePage {
            npc_id: 1,
            text: String::new(),
            has_previous: false,
            has_next: true,
        });
        assert_eq!(input(&page, 0, 1, None), Some(ScriptInput::Next));
        assert_eq!(input(&page, 0, 0, None), None, "no previous page");
        assert_eq!(input(&page, 1, 1, None), None);
        let menu = Prompt::Selection {
            npc_id: 1,
            text: String::new(),
        };
        assert_eq!(input(&menu, 4, 1, Some(3)), Some(ScriptInput::Select(3)));
        assert_eq!(input(&menu, 4, -1, None), None);
        let styles = Prompt::Style {
            npc_id: 1,
            text: String::new(),
            styles: vec![30000],
        };
        assert_eq!(input(&styles, 7, 1, Some(0)), Some(ScriptInput::Select(0)));
        assert_eq!(input(&styles, 7, 0, None), None, "cancelled");
        assert_eq!(input(&styles, 4, 1, Some(0)), None);
    }

    fn styles(step: &Step) -> Vec<i32> {
        match &step.prompt {
            Some(Prompt::Style { styles, .. }) => styles.clone(),
            other => panic!("styles expected, got {other:?}"),
        }
    }

    #[test]
    fn natalie_cuts_hair_for_a_coupon() {
        // A man with black hair (30030) and a haircut coupon.
        let mut man = player(50);
        man.stats.gender = 0;
        man.inventory.insert(
            game::InventoryType::Cash,
            1,
            game::Item::stack(HAIRCUT_COUPON, 1),
        );
        let mut natalie = NpcScript::for_npc(NATALIE, &man).unwrap();
        assert_eq!(kind(&natalie.begin()), Some(4));
        let cuts = natalie.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(
            styles(&cuts),
            [30060, 30140, 30200, 30210, 30310, 33040, 33100]
        );
        let done = natalie.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(
            done.effects,
            [
                Effect::TakeItem {
                    item_id: HAIRCUT_COUPON,
                    quantity: 1
                },
                Effect::SetStyle(30140),
            ]
        );
        assert_eq!(buttons(&done), (false, false));
        assert_eq!(natalie.respond(ScriptInput::Next), Some(Step::default()));
    }

    #[test]
    fn natalie_dyes_to_other_colours_and_needs_the_coupon() {
        // A woman with hair 31142, and no dye coupon.
        let mut woman = player(50);
        woman.stats.hair = 31142;
        let mut natalie = NpcScript::for_npc(NATALIE, &woman).unwrap();
        let cuts = natalie.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(styles(&cuts)[0], 31152, "her colour, the women's styles");
        let mut natalie = NpcScript::for_npc(NATALIE, &woman).unwrap();
        let dyes = natalie.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(
            styles(&dyes),
            [31140, 31141, 31143, 31144, 31145, 31146, 31147]
        );
        assert_eq!(natalie.respond(ScriptInput::Select(7)), None);
        let refused = natalie.respond(ScriptInput::Select(0)).unwrap();
        assert!(refused.effects.is_empty());
        assert_eq!(kind(&refused), Some(0));
    }

    #[test]
    fn the_water_of_life_wakes_the_doll_picked() {
        if crate::items::data().is_err() {
            return;
        }
        const WATER: i32 = 5180000;
        let mut owner = player(50);
        assert_eq!(NpcScript::water_of_life(&owner), None, "no water");
        owner
            .inventory
            .insert(InventoryType::Cash, 1, game::Item::stack(WATER, 1));
        let mut none = NpcScript::water_of_life(&owner).unwrap();
        assert_eq!(kind(&none.begin()), Some(0), "no doll to wake");
        assert_eq!(none.respond(ScriptInput::Next), Some(Step::default()));

        let mut kitty = game::Item::stack(5000000, 1);
        kitty.cash_serial = Some(78);
        kitty.pet = Some(game::pet::PetData::new("Brown Kitty"));
        owner
            .inventory
            .insert(InventoryType::Cash, 2, kitty.clone());
        kitty.cash_serial = Some(79);
        kitty.expiration = Some(game::pet::doll_date());
        owner.inventory.insert(InventoryType::Cash, 3, kitty);
        let mut mar = NpcScript::water_of_life(&owner).unwrap();
        assert_eq!(mar.npc_id(), MAR_THE_FAIRY);
        let Some(Prompt::Selection { text, .. }) = mar.begin().prompt else {
            panic!("a choice of dolls");
        };
        assert_eq!(text.matches("#L").count(), 1, "only the doll");
        assert!(text.contains("#L0##bBrown Kitty (Lv. 1)#k#l"));
        assert_eq!(kind(&mar.respond(ScriptInput::Select(0)).unwrap()), Some(1));
        let woken = mar.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            woken.effects,
            [
                Effect::TakeItem {
                    item_id: WATER,
                    quantity: 1
                },
                Effect::RevivePet {
                    serial: 79,
                    days: 90
                },
            ]
        );
        assert_eq!(kind(&woken), Some(0));
    }

    /// A level-70 Hunter holding `items` in the Etc bag, with `markers`.
    fn hunter(items: &[(i32, i16)], markers: &[i16]) -> Player {
        let mut player = player(50);
        (player.stats.job, player.stats.level, player.stats.sp) = (310, 70, 0);
        for (slot, &(item_id, quantity)) in (1..).zip(items) {
            let item = game::Item::stack(item_id, quantity);
            player.inventory.insert(InventoryType::Etc, slot, item);
        }
        for &marker in markers {
            player.quests.mark(marker);
        }
        player
    }

    #[test]
    fn second_jobs_earn_the_third_job_from_their_chief() {
        const RENE: i32 = 2020010;
        const ATHENA: i32 = 1012100;
        let take = |item_id| Effect::TakeItem {
            item_id,
            quantity: 1,
        };

        // Rene only trains her own line, from level 70.
        let mut young = hunter(&[], &[]);
        young.stats.level = 69;
        assert_eq!(
            kind(&NpcScript::for_npc(RENE, &young).unwrap().begin()),
            Some(0)
        );
        let mut tylus = NpcScript::for_npc(2020008, &hunter(&[], &[])).unwrap();
        assert_eq!(kind(&tylus.begin()), Some(0));
        assert_eq!(tylus.respond(ScriptInput::Next).unwrap().effects, []);

        // She assigns the test, and Athena sends the player to the door.
        let mut rene = NpcScript::for_npc(RENE, &hunter(&[], &[])).unwrap();
        assert_eq!(
            rene.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::Mark(CHIEF_ASSIGNED)]
        );
        let mut athena = NpcScript::for_npc(ATHENA, &hunter(&[], &[CHIEF_ASSIGNED])).unwrap();
        assert_eq!(
            athena.respond(ScriptInput::Next).unwrap().effects,
            [
                Effect::Unmark(CHIEF_ASSIGNED),
                Effect::Mark(INSTRUCTOR_ASSIGNED)
            ]
        );
        let sent = hunter(&[], &[INSTRUCTOR_ASSIGNED]);
        let rene = NpcScript::for_npc(RENE, &sent).unwrap();
        assert_eq!(buttons(&rene.begin()), (false, false));

        // The door lets the player in from the line's map only, the clone's
        // room and the way in empty.
        let mut at_door = sent.clone();
        at_door.stats.map_id = 105040305;
        let mut door = NpcScript::for_npc(DOOR_OF_DIMENSION, &at_door).unwrap();
        assert_eq!(
            door.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::RequireEmptyMap(108010101),
                Effect::RequireEmptyMap(108010100),
                Effect::WarpToPortal(108010100, 0),
            ]
        );
        assert_eq!(kind(&door.refused().unwrap()), Some(0));
        let door = NpcScript::for_npc(DOOR_OF_DIMENSION, &sent).unwrap();
        assert_eq!(kind(&door.begin()), Some(0));
        let mut in_room = sent.clone();
        in_room.stats.map_id = 108010101;
        let mut crystal = NpcScript::for_npc(SPARKLING_CRYSTAL, &in_room).unwrap();
        assert_eq!(
            crystal.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::Warp(105040305)]
        );
        assert_eq!(time_limit(108010101), Some(20 * 60));
        assert_eq!(time_limit(108010100), None);

        // The clone's charm earns the Necklace of Strength, which Rene
        // trades for the Holy Stone's assignment.
        let charmed = hunter(&[(BLACK_CHARM, 1)], &[INSTRUCTOR_ASSIGNED]);
        let mut athena = NpcScript::for_npc(ATHENA, &charmed).unwrap();
        assert_eq!(
            athena.respond(ScriptInput::Next).unwrap().effects,
            [
                take(BLACK_CHARM),
                Effect::GiveItem {
                    item_id: NECKLACE_OF_STRENGTH,
                    quantity: 1
                },
                Effect::Unmark(INSTRUCTOR_ASSIGNED),
            ]
        );
        let mut rene =
            NpcScript::for_npc(RENE, &hunter(&[(NECKLACE_OF_STRENGTH, 1)], &[])).unwrap();
        assert_eq!(
            rene.respond(ScriptInput::Next).unwrap().effects,
            [
                take(NECKLACE_OF_STRENGTH),
                Effect::Mark(HOLY_STONE_ASSIGNED)
            ]
        );

        // For the Necklace of Wisdom, the job, once enough SP is spent.
        let mut wise = hunter(&[(NECKLACE_OF_WISDOM, 1)], &[HOLY_STONE_ASSIGNED]);
        wise.stats.level = 72;
        wise.stats.sp = 7;
        let rene = NpcScript::for_npc(RENE, &wise).unwrap();
        assert_eq!(kind(&rene.begin()), Some(0), "too much SP left");
        wise.stats.sp = 6;
        let mut rene = NpcScript::for_npc(RENE, &wise).unwrap();
        assert_eq!(
            rene.respond(ScriptInput::Yes).unwrap().effects,
            [
                take(NECKLACE_OF_WISDOM),
                Effect::Unmark(HOLY_STONE_ASSIGNED),
                Effect::TakeThirdJob(311),
            ]
        );
        // Unsent, Athena talks as she does to any second job.
        let athena = NpcScript::for_npc(ATHENA, &hunter(&[], &[])).unwrap();
        assert_eq!(athena, first_job(&INSTRUCTORS[2], &hunter(&[], &[])));
    }

    #[test]
    fn third_jobs_who_passed_the_trials_take_the_fourth_job_in_leafre() {
        const LEGOR: i32 = 2081300;
        let says = |player: &Player| kind(&NpcScript::for_npc(LEGOR, player).unwrap().begin());
        let mut ranger = hunter(&[], &[]);
        (ranger.stats.job, ranger.stats.level) = (311, 120);
        assert_eq!(says(&ranger), Some(0), "the trials first");
        ranger.quests.upsert(game::QuestRecord {
            quest_id: 6924,
            status: QuestStatus::Completed {
                at: std::time::SystemTime::UNIX_EPOCH,
            },
        });
        let mut legor = NpcScript::for_npc(LEGOR, &ranger).unwrap();
        assert_eq!(kind(&legor.begin()), Some(1));
        assert_eq!(
            legor.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::GiveItem {
                    item_id: MAPLE_HERO,
                    quantity: 1
                },
                Effect::TakeFourthJob(312),
            ]
        );
        assert_eq!(kind(&legor.inventory_full().unwrap()), Some(0));

        let mut young = ranger.clone();
        young.stats.level = 119;
        assert_eq!(says(&young), Some(0), "level 120");
        let mut other = ranger.clone();
        other.stats.job = 411;
        let hellin = NpcScript::for_npc(2081400, &ranger).unwrap();
        assert_eq!(kind(&hellin.begin()), Some(0), "another line");
        assert_eq!(says(&other), Some(0));
        ranger.stats.job = 312;
        assert_eq!(says(&ranger), Some(0), "already a fourth job");
    }

    /// The answer the quiz on the current page takes as right.
    fn right_answer(script: &NpcScript) -> i32 {
        let Page::Menu { choices, .. } = &script.pages[script.page] else {
            panic!("a question");
        };
        let right = choices
            .iter()
            .position(|(_, pages)| !matches!(pages.last(), Some(Page::End(_))));
        right.unwrap() as i32
    }

    #[test]
    fn the_holy_stone_asks_five_questions_for_the_necklace() {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        let ready = hunter(&[(DARK_CRYSTAL, 2)], &[HOLY_STONE_ASSIGNED]);
        let crystal = Effect::TakeItem {
            item_id: DARK_CRYSTAL,
            quantity: 1,
        };

        let mut stone = holy_stone(&ready, &mut rng);
        assert_eq!(kind(&stone.begin()), Some(1));
        assert_eq!(
            stone.respond(ScriptInput::Yes).unwrap().effects,
            [crystal.clone()]
        );
        let mut asked = Vec::new();
        for _ in 0..QUIZ_LENGTH {
            let Page::Menu { text, choices } = &stone.pages[stone.page] else {
                panic!("a question");
            };
            assert_eq!(choices.len(), 5);
            asked.push(text.clone());
            let right = right_answer(&stone);
            assert_eq!(
                stone.respond(ScriptInput::Select(right)).unwrap().effects,
                []
            );
        }
        asked.sort();
        asked.dedup();
        assert_eq!(asked.len(), QUIZ_LENGTH, "no question twice");
        assert_eq!(
            stone.respond(ScriptInput::Next).unwrap().effects,
            [Effect::GiveItem {
                item_id: NECKLACE_OF_WISDOM,
                quantity: 1
            }]
        );

        // A wrong answer ends it, the crystal spent.
        let mut stone = holy_stone(&ready, &mut rng);
        stone.respond(ScriptInput::Yes).unwrap();
        let wrong = (right_answer(&stone) + 1) % 5;
        let failed = stone.respond(ScriptInput::Select(wrong)).unwrap();
        assert_eq!(buttons(&failed), (false, false));
        assert_eq!(stone.respond(ScriptInput::Next).unwrap().effects, []);

        // Not without the chief's assignment, a crystal, or with the necklace.
        for player in [
            hunter(&[(DARK_CRYSTAL, 1)], &[]),
            hunter(&[], &[HOLY_STONE_ASSIGNED]),
            hunter(
                &[(DARK_CRYSTAL, 1), (NECKLACE_OF_WISDOM, 1)],
                &[HOLY_STONE_ASSIGNED],
            ),
        ] {
            let mut stone = holy_stone(&player, &mut rng);
            assert_eq!(kind(&stone.begin()), Some(0));
            assert_eq!(stone.respond(ScriptInput::Next).unwrap().effects, []);
        }
    }

    #[test]
    fn spiruna_refines_dark_crystals_after_her_quest() {
        let mut player = hunter(&[(DARK_CRYSTAL_ORE, 12)], &[]);
        player.meso = REFINING_FEE;
        let spiruna = NpcScript::for_npc(SPIRUNA, &player).unwrap();
        assert_eq!(kind(&spiruna.begin()), Some(0));
        player.quests.upsert(game::QuestRecord {
            quest_id: SPIRUNA_QUEST,
            status: QuestStatus::Completed {
                at: std::time::SystemTime::UNIX_EPOCH,
            },
        });
        let mut spiruna = NpcScript::for_npc(SPIRUNA, &player).unwrap();
        assert_eq!(
            spiruna.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::TakeItem {
                    item_id: DARK_CRYSTAL_ORE,
                    quantity: 10
                },
                Effect::SpendMeso(REFINING_FEE),
                Effect::GiveItem {
                    item_id: DARK_CRYSTAL,
                    quantity: 1
                },
            ]
        );
        player.meso -= 1;
        let mut spiruna = NpcScript::for_npc(SPIRUNA, &player).unwrap();
        assert_eq!(spiruna.respond(ScriptInput::Yes).unwrap().effects, []);
    }
}
