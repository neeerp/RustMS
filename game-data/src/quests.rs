//! Quest definitions from `Quest.nx`, classified against what the server can
//! check and grant.
//!
//! In v83 the client renders quest conversations (`Say.img`) itself and only
//! asks the server to start or complete a quest, so the server reads the
//! requirements (`Check.img`) and actions (`Act.img`). A quest is supported
//! only if every requirement and action it declares is one the server
//! implements; anything else would be silently skipped, so such quests are
//! refused instead. HeavenClient offers only the supported quests, which it
//! reads from `supported_quests.txt`.

use crate::{GameDataError, NxDataFile};
use game::quest::INFO_QUESTS;
use game::QuestState;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Requirements to start a quest (`Check.img/<id>/0`) that the server checks.
pub const START_REQUIREMENTS: &[&str] = &[
    "npc",
    "job",
    "lvmin",
    "lvmax",
    "quest",
    "item",
    "skill",
    "pop",
    "interval",
    "dayByDay",
    "startscript",
    "equipAllNeed",
    "equipSelectNeed",
    "fieldEnter",
    "start",
    "end",
    "pet",
    "pettamenessmin",
    "pettamenessmax",
    "dayOfWeek",
    "buff",
    "exceptbuff",
    "petRecallLimit",
    "petAutoSpeakingLimit",
    "tamingmoblevelmin",
    "tamingmoblevelmax",
];
/// Keys in `Check.img/<id>/0` that only tell the client when to offer an
/// autoStart quest through its quest alert: `normalAutoStart` on stat
/// changes. v83's own start check (FUN_00721163) ignores it. (`fieldEnter`
/// also triggers the alert, on entering a listed map, but is a requirement.)
pub const START_TRIGGERS: &[&str] = &["normalAutoStart"];
/// Requirements to complete a quest (`Check.img/<id>/1`) that the server checks.
pub const END_REQUIREMENTS: &[&str] = &[
    "npc",
    "item",
    "mob",
    "quest",
    "endscript",
    "endmeso",
    "level",
    "pet",
    "pettamenessmin",
    "pettamenessmax",
    "buff",
    "exceptbuff",
    "mbmin",
    "mbmax",
    "mbcard",
    "questComplete",
    "partyQuest_S",
    "petRecallLimit",
    "petAutoSpeakingLimit",
];
/// Dates under `Check.img/<id>/1`, which nothing reads: v83's completion
/// check compares no dates (only the start check does), and the two quests
/// with them (4306, 4309) carry the placeholder 2000-01-01 to 01-02.
pub const INERT_END_REQUIREMENTS: &[&str] = &["start", "end"];
/// Quest scripts (`startscript`/`endscript`) the server implements. A quest
/// naming any other script is unsupported.
pub const SCRIPTS: &[&str] = &[
    "q1021s", "q1021e", "q1028s", "q2214e", "q2215e", "q2228s", "q2230e", "q2238s", "q2245s",
    "q2232e", "q2233e", "q2234e", "q2216s", "q2217s", "q2218s", "q2219s", "q3714s", "q4647e",
    "q4659e", "q6030e", "q6031e", "q6032e", "q6033e", "q6036e", "q8185e", "q8189e", "q2314s",
    "q2322s", "q2332s", "q2332e", "q2333s", "q2333e", "q2334s", "q2335s", "q2342s", "q3108s",
    "q3523s", "q3524s", "q3525s", "q3526s", "q3527s", "q3529s", "q3539s", "q2300s", "q2300e",
    "q2301s", "q2301e", "q2302s", "q2302e", "q2303s", "q2303e", "q2304s", "q2304e", "q2325e",
    "q2327s", "q2338s", "q2251e", "q2254s", "q2257e", "q2258s", "q2258e", "q2259s", "q2259e",
    "q2260s", "q2260e", "q3301e", "q3303e", "q3305s", "q3306s", "q3320s", "q3321s", "q3353s",
    "q3354s", "q3360s", "q3452e", "q3833e", "q3953e", "q3933s", "q3514e", "q3455s", "q2124e",
    "q2126e", "q2127e", "q2186e", "q2148s", "q2149s", "q2150s", "q2151s", "q2152s", "q2197e",
    "q2293s", "q8221s", "q8223s", "q8224s", "q8225s", "q8226s", "q8231s", "q8232s", "q8233s",
    "q8234s", "q8235s", "q8236s", "q8237s", "q8238s", "q8219s", "q8219e", "q8222s", "q8222e",
    "q8227s", "q8228s", "q8228e", "q8229s", "q8229e", "q8230s", "q8230e", "q29900s", "q29900e",
    "q29901s", "q29901e", "q29902s", "q29902e", "q29903s", "q29903e", "q29508e", "q29910s",
    "q29911s", "q29912s", "q29913s", "q29914s", "q29915s", "q29916s", "q29917s", "q29918s",
    "q29919s", "q29920s", "q29921s", "q29933s", "q21733s", "q21733e", "q21734s", "q21734e",
    "q21735s", "q21735e", "q21736s", "q21738s", "q21739e", "q21740s", "q21740e", "q21741s",
    "q21742s", "q21742e", "q21746s", "q21747s", "q21747e", "q21748e", "q21749s", "q21750e",
    "q21753s", "q21754s", "q21757e", "q3314e", "q3382e", "q3941s", "q3941e", "q8255s", "q2156e",
    "q2291e", "q21700s", "q21703s", "q21703e", "q21704s", "q21712s", "q21716s", "q21719s",
    "q21720e", "q21729s", "q21766s", "q21766e", "q21767s", "q21600s", "q21604s", "q21613s",
    "q21618s", "q21618e", "q3250s", "q3250e", "q29002s", "q29002e", "q29400s", "q29400e",
    "q29906s", "q29907s", "q29908s", "q29909s", "q29922s", "q29923s", "q29924s", "q29925s",
    "q29926s", "q29927s", "q29928s",
];
/// Quests with nothing to complete them by (an empty `Check/1`) whose start
/// script completes them as it starts them (`Effect::FinishQuest`): JM's
/// informants (2216-2219), each heard from once, Reef's thanks for Faust
/// (2228), Mu Young's way to Tristan's Tomb (2245), each time it opens, and
/// the Remnants of Horned Tail (3714), once for each Nine Spirit's Egg, and
/// Violetta's face (2334) and the Royal Seal she keeps (2342), and the
/// clue in the broken Spirit of Snow Statue (3108), and the lost memory
/// each job instructor gives back (3523-3527, 3529, 3539), and an Aran's Baby
/// Steps (21704), told to Lilin, and the titles of medals held (29922 to
/// 29928: Veteran Hunter, Legendary Hunter and the Aran titles).
pub const FINISHED_BY_START_SCRIPT: &[i16] = &[
    2216, 2217, 2218, 2219, 2228, 2245, 2334, 2342, 3108, 3523, 3524, 3525, 3526, 3527, 3529, 3539,
    3714, 21704, 29922, 29923, 29924, 29925, 29926, 29927, 29928,
];
/// The Knight titles (29906 to 29909): Cygnus (1101000) completes them, and
/// their medal (`QuestInfo` `viewMedalItem`) is the reward. The client's
/// data names no end script and an empty `Act/1`, so the medal is ours to
/// give; Cosmic's end scripts give it too.
pub const KNIGHT_TITLES: &[i16] = &[29906, 29907, 29908, 29909];
/// Quests no NPC completes through the quest window: `Check/1` is only an
/// `infoex` entry ("dummy") on an info string nothing sets, which v83's
/// completion check reads, so neither the client nor the server lets it
/// pass. An NPC's script completes them instead: Dollhouse (3230), which
/// Mark the Toy Soldier ends for the pendulum, and The Missing Mechanical
/// Parts (3239), which Cheng the Assistant ends for ten Machine Parts
/// (`net::npc_scripts`). Eliminating the Rest (2335) reads its own record
/// (`infoNumber` 2335), which nothing sets either; the Security Room's door
/// (`go_secretroom`, `net::scripts`) completes it for Violetta's key. The
/// Seed That Swallow Lost (3609) has an empty `Check/1`; Swallow's talk
/// completes it with a new seed.
pub const COMPLETED_BY_NPC_SCRIPT: &[i16] = &[2335, 3230, 3239, 3609];
/// Meren's Maker classes (quests 6030-6032) and the quest they make up
/// (6029) share 6029's quest-info string, one digit per class: `Act/0 info`
/// starts it at "000", a class starts only while its own digit is 0
/// (`Check/0 infoNumber`/`infoex`), its end script sets the digit, and 6029
/// completes at "111" (`Check/1 infoex`). 6029 also needs the three classes
/// completed, and each class needs 6029 started, so the server keeps no
/// string: it reads past these keys and makes the classes, which
/// `interval 0` would let be taken again, one-time quests, as their digit
/// does. Only a forfeited and restarted 6029, which resets the string,
/// differs: its classes stay done.
pub const MEREN_INFO_QUESTS: &[i16] = &[6029, 6030, 6031, 6032];
const INFO_KEYS: &[&str] = &["info", "infoNumber", "infoex"];
/// The explorer medals: each counts the maps of a region visited
/// (`map/onUserEnter/explorationPoint`, or for 29004 the `highposition`
/// portals) into its info record, and completes at Dalair once that count is
/// at least `Check/1/infoex` (cond 1, v83 FUN_00721d2c). The server keeps the
/// maps; see [`QuestTemplate::explored`].
pub const EXPLORER_QUESTS: &[i16] = &[
    29004, 29005, 29006, 29007, 29008, 29009, 29010, 29011, 29014,
];
/// Quests that complete once their own record reaches a count
/// (`Check/1/infoNumber` is the quest itself, one `infoex` entry, cond 1):
/// Exploring Mushroom Forest (1) (2314) and Over the Castle Walls (2)
/// (2322), whose `investigate1` and `investigate2` portals set it, "Over
/// the Castle Walls (4)" (2324), whose Thorn Remover sets it, and Where's
/// Violetta? (2332), which the Wedding Hall's `findvioleta` sets.
/// The server keeps the count as the record's only "kill"; see
/// [`QuestTemplate::end_record`].
pub const OWN_RECORD_QUESTS: &[i16] = &[2314, 2322, 2324, 2332];
/// Quests that complete once a record that is no quest reads a value
/// (`Check/1 infoNumber` names it, `infoex` the value), which a script sets
/// (`game::quest::RECORDS`): `(quest, record)`. See
/// [`QuestTemplate::end_info_record`].
pub const RECORD_QUESTS: &[(i16, i16)] = &[
    (6330, 6331),
    (6363, 6364),
    (6370, 6371),
    (6400, 6401),
    (6410, 6411),
    (3507, 7081),
    (29932, 7760),
    (20706, 20731),
    (20718, 20732),
    (21728, 21761),
    (21733, 21762),
    (3250, 7067),
    (21742, 21763),
    (21610, 21619),
    (21613, 21620),
    (3522, 7402),
    (3538, 7402),
];
/// Quests that start only once a record that is no quest reads a value
/// (`Check/0 infoNumber` names it, `infoex` the value), which a script sets
/// (`game::quest::RECORDS`): `(quest, record)`. The Test (2312) waits for
/// the Mushroom Kingdom's opening scene (`TD_MC_Openning`) to set 2311.
/// An Aran's "The Lost Seal Stone of Ellin Forest" (21753) waits for 21764,
/// set on entering Ellin's warehouse, and "Info on Another Seal Stone"
/// (21754) for 21765, set on entering the Bowman Instructional School, and
/// "The Puppeteer's Warning" (21720) for 21760, set by Francis's warning in
/// the Puppeteer's Cave. "Yulete's Request" (3365) and "Romeo's Present"
/// (3369) wait for 7072, set on winning a Romeo and Juliet run.
/// See [`StartRequirements::record`].
pub const START_RECORD_QUESTS: &[(i16, i16)] = &[
    (2312, 2311),
    (3365, 7072),
    (3369, 7072),
    (21720, 21760),
    (21753, 21764),
    (21754, 21765),
];
/// "What Meren Dropped" (6034) repeats its start requirements in `Act/0`,
/// with a `skill` 1007 entry (`acquire` 65535) that teaches nothing (Cosmic
/// has the same data); none of it is an action.
const REQUIREMENTS_IN_START_ACTIONS: i16 = 6034;
/// The `petSkill` bit of a pet that comes to its owner on a map change.
pub const PET_RECALL: u16 = 0x80;
/// The `petSkill` bit of a pet that speaks by itself.
pub const PET_AUTO_SPEAKING: u16 = 0x100;

/// Actions on starting a quest (`Act.img/<id>/0`) that the server applies.
pub const START_ACTIONS: &[&str] = &["item", "exp", "money", "buffItemID"];
/// Actions on completing a quest (`Act.img/<id>/1`) that the server applies.
/// `npcAct` names a special action of the completing NPC, which the server
/// has it play (11 quests: "quest", "action" or an `act<quest>`).
pub const END_ACTIONS: &[&str] = &[
    "exp",
    "money",
    "pop",
    "pettameness",
    "nextQuest",
    "item",
    "skill",
    "npcAct",
    "buffItemID",
    "petskill",
    "petspeed",
];
/// `Act` keys that copy a requirement from `Check` (`npc`, `lvmin`, `job`,
/// `quest`, a date or an `interval`). v83's client reads only `item`,
/// `skill`, `exp`, `money`, `pop`, `pettameness` and `nextQuest` under
/// `Act`, and Cosmic applies none of these, so they do nothing here either;
/// the `Check` copy is what counts. `map` (a list of maps, only 3706) and
/// `message` (an untranslated line: 3706, 2282-2284) are read by neither
/// the client (report b200) nor Cosmic, so they are ignored too.
pub const INERT_ACTIONS: &[&str] = &[
    "npc", "lvmin", "lvmax", "job", "quest", "interval", "start", "end", "map", "message",
];
/// Keys of an `Act/<n>/item/<k>` entry the server applies.
/// `var` (1 or 2, on equipment only) is read by neither v83's client nor
/// Cosmic, which give the base stats; so do we.
pub const ITEM_ACTION_KEYS: &[&str] = &["id", "count", "job", "gender", "prop", "period", "var"];
/// Keys of an `Act/1/skill/<k>` entry the server applies.
/// `onlyMasterLevel` marks entries without a `skillLevel`, which already
/// raise only the master level.
pub const SKILL_ACTION_KEYS: &[&str] =
    &["id", "skillLevel", "masterLevel", "job", "onlyMasterLevel"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuestPrerequisite {
    pub quest_id: i16,
    pub state: QuestState,
}

/// A card the Monster Book must hold between `min` and `max` of (either
/// unset means no bound) to complete a quest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BookCard {
    pub card: i32,
    pub min: Option<i32>,
    pub max: Option<i32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartRequirements {
    pub npc: i32,
    /// Allowed job ids; empty means any job.
    pub jobs: Vec<i16>,
    pub min_level: Option<i16>,
    pub max_level: Option<i16>,
    pub quests: Vec<QuestPrerequisite>,
    /// Items to hold, as `(item id, count)`: at least `count` if positive,
    /// none if 0, and at most `-count` if negative.
    pub items: Vec<(i32, i32)>,
    /// Skills to have (`true`, from `acquire`) or not have (`false`).
    pub skills: Vec<(i32, bool)>,
    /// Least fame (`pop`); 0 means no requirement.
    pub min_fame: i16,
    /// Minutes after completing before the quest can be started again
    /// (`interval`); `None` means it can't be repeated.
    pub repeat_interval: Option<u32>,
    /// Without an interval, repeatable once a day (`dayByDay`): from the
    /// first day (UTC) after the one it was completed on.
    pub daily: bool,
    /// Items that must all be worn (`equipAllNeed`).
    pub worn_all: Vec<i32>,
    /// Items of which one must be worn (`equipSelectNeed`), if any.
    pub worn_any: Vec<i32>,
    /// Maps the player must be on (`fieldEnter`); empty means any.
    pub maps: Vec<i32>,
    /// The hour (UTC) before which the quest can't be started (`start`,
    /// "yyyymmddhh").
    pub not_before: Option<SystemTime>,
    /// The hour (UTC) after which it can't (`end`). v83 offers a quest from
    /// its start to its end hour inclusive (FUN_00721163), so most event
    /// quests, which ended in 2005-2010, are never offered.
    pub not_after: Option<SystemTime>,
    /// The weekdays the quest can be started on (`dayOfWeek`), a bit each
    /// from Sunday (1) to Saturday (64); `None` means any day.
    pub weekdays: Option<u8>,
    /// Pet items of which the first pet out must be one (`pet`); empty means
    /// no pet is needed.
    pub pets: Vec<i32>,
    /// The closeness the first pet out must have, from `pettamenessmin` to
    /// `pettamenessmax` (30000 when only the least is given).
    pub pet_closeness: Option<(i32, i32)>,
    /// `petSkill` bits the first pet out must not have yet
    /// (`petRecallLimit`, `petAutoSpeakingLimit`); 0 means no pet is needed.
    pub pet_skill_limit: u16,
    /// The mount's level, from `tamingmoblevelmin` to `tamingmoblevelmax`
    /// (0 and 30 when unset); v83 (FUN_00721163) tests it on start only.
    /// Only Silver Mane (6006) has one.
    pub mount_level: Option<(i32, i32)>,
    /// An item whose effect, as if used, must be on the player (`buff`), or
    /// must not be (`exceptbuff`): Kerning's scents (2282 to 2284).
    pub buff: Option<i32>,
    pub except_buff: Option<i32>,
    /// A record that is no quest, and what its info string must read
    /// (`Check/0 infoNumber`, `infoex`); see [`START_RECORD_QUESTS`].
    pub record: Option<(i16, Vec<InfoCheck>)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestTemplate {
    pub id: i16,
    pub name: String,
    pub start: StartRequirements,
    pub end_npc: i32,
    /// Items needed to complete, as `(item id, count)`. A count of 0 means
    /// the player must have none left (e.g. Roger's apples, all eaten).
    pub end_items: Vec<(i32, i32)>,
    /// Mobs to kill before completing, as `(mob id, count)`.
    pub end_mobs: Vec<(i32, u16)>,
    /// Quests that must be in a given state before completing.
    pub end_quests: Vec<QuestPrerequisite>,
    /// Mesos to hold before completing (`endmeso`); 0 means none.
    pub end_meso: i32,
    /// Least level to complete (`Check/1/level`).
    pub end_min_level: Option<i16>,
    /// Pet items of which the first pet out must be one to complete (`pet`).
    pub end_pets: Vec<i32>,
    /// The closeness the first pet out must have to complete.
    pub end_pet_closeness: Option<(i32, i32)>,
    pub end_pet_skill_limit: u16,
    /// An item whose effect must be on the player to complete, or must not.
    pub end_buff: Option<i32>,
    pub end_except_buff: Option<i32>,
    /// The fewest and most distinct cards the Monster Book may hold to
    /// complete (`mbmin`, `mbmax`).
    pub end_book_min: Option<i32>,
    pub end_book_max: Option<i32>,
    /// Cards whose count in the Monster Book must lie within bounds to
    /// complete (`mbcard`), as `(card id, min, max)`.
    pub end_cards: Vec<BookCard>,
    /// The fewest quests completed before completing this one
    /// (`questComplete`), counted as the client counts them (0x7225ab):
    /// see [`QuestCatalog::counts_as_completed`]. Only Quest Specialist
    /// (29001) has one, 800.
    pub end_completed_quests: Option<i32>,
    /// The fewest party quests whose record is at rank S
    /// (`partyQuest_S`, 0x722525). Only PQ Mania (29000) has one, 5.
    pub end_party_quest_s: Option<i32>,
    /// Items given or taken on start and on completion.
    pub start_items: Vec<QuestItem>,
    pub end_rewards: Vec<QuestItem>,
    /// EXP given and mesos given (or, if negative, taken, which needs them
    /// held) on starting.
    pub start_exp: i32,
    pub start_money: i32,
    /// Items whose effect, as if used, the player gets on starting and on
    /// completing (`buffItemID`).
    pub start_buff_item: Option<i32>,
    pub end_buff_item: Option<i32>,
    /// The completing NPC's special action to play (`Act/1/npcAct`); the
    /// client ignores the key, so the server sends it (0x106).
    pub end_npc_act: Option<String>,
    /// Skills taught on completion.
    pub end_skills: Vec<QuestSkill>,
    /// Scripts that run the start or completion instead of the quest data.
    pub start_script: Option<String>,
    pub end_script: Option<String>,
    pub exp: i32,
    /// Mesos given on completion; negative takes them (and needs them held).
    pub money: i32,
    /// Fame given on completion (`pop`); negative takes it.
    pub fame: i16,
    /// Closeness given on completion to the first pet out (`pettameness`),
    /// as Cosmic does; v83 lists it last in the rewards (QuestIcon 9).
    pub pet_closeness: i16,
    /// `petSkill` bits given on completion to the first pet out
    /// (`petskill`): 0x80 recall (4660), 0x100 auto speaking (4661).
    pub pet_skill: u16,
    /// Whether completion lets the first pet out keep the owner's speed
    /// (`petspeed`, attribute bit 1: 3083, 3096).
    pub pet_speed: bool,
    pub next_quest: Option<i16>,
    /// `QuestInfo` `autoStart` and `autoComplete`: the client's quest alert
    /// offers the quest wherever the player is.
    pub auto_start: bool,
    pub auto_complete: bool,
    /// `QuestInfo` `autoPreComplete`: once started, the quest alert lists it
    /// in progress and completes it wherever the player is.
    pub auto_pre_complete: bool,
    /// An explorer medal's count: its info record and the maps it needs.
    pub explored: Option<Explored>,
    /// The count its own record must reach (`OWN_RECORD_QUESTS`).
    pub end_record: Option<u16>,
    /// For `game::quest::INFO_QUESTS`: the info string the start sets
    /// (`Act/0/info`) and what it must read to complete (`Check/1/infoex`).
    pub start_info: Option<String>,
    pub end_info: Vec<InfoCheck>,
    /// The record `end_info` reads when it is not the quest's own
    /// (`RECORD_QUESTS`).
    pub end_info_record: Option<i16>,
    /// `QuestInfo` `timeLimit`: the seconds the player has once started,
    /// shown as the client's timed-quest clock; then the quest is forfeited.
    pub time_limit: Option<u32>,
    /// `QuestInfo` `timeLimit2`: a timed title's seconds once started. Its
    /// started record is the end date, which the Quest Log counts down to
    /// (`game::quest::DATED_QUESTS`).
    pub time_limit2: Option<u32>,
    /// `QuestInfo` `dailyPlayTime`: the seconds of play a day needs to count
    /// toward the title (Diligent Explorer's three hours).
    pub daily_play_time: Option<u32>,
}

/// One `Check/1/infoex` entry on the quest's own info string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfoCheck {
    /// 0 the string equals `value`, 1 it is at least `value`, 2 at most,
    /// both read as numbers.
    pub cond: u8,
    pub value: String,
}

/// Whether an info string passes `checks`, as v83's completion check reads
/// them: a missing string fails, and any one entry passing is enough.
pub fn info_met(info: Option<&str>, checks: &[InfoCheck]) -> bool {
    let Some(info) = info else {
        return false;
    };
    checks.iter().any(|check| match check.cond {
        1 => atoi(&check.value) <= atoi(info),
        2 => atoi(info) <= atoi(&check.value),
        _ => info == check.value,
    })
}

/// C's `atoi`: the leading number, or 0.
fn atoi(text: &str) -> i64 {
    let text = text.trim_start();
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, text.strip_prefix('+').unwrap_or(text)),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    sign * digits[..end].parse::<i64>().unwrap_or(0)
}

/// What an explorer medal (`EXPLORER_QUESTS`) counts toward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Explored {
    /// The quest whose record holds the count (`Check/1/infoNumber`).
    pub record: i16,
    /// Maps needed to complete (`Check/1/infoex/0/value`).
    pub needed: u32,
}

/// One `Act/<n>/item` entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestItem {
    pub item_id: i32,
    /// Positive gives, negative takes, and 0 (no count in the data) takes
    /// all the player holds. The data leaves the count out only on the
    /// quest items a completion needs (Kyrin's test crystals), and v83's
    /// reward list doesn't show those; taking them all is our reading.
    pub count: i32,
    /// Job families allowed (v83's bit encoding: 1 beginner, 2 warrior, 4
    /// magician, 8 bowman, 16 thief, 32 pirate, ...); 0 means any.
    pub job: i32,
    /// 0 male, 1 female, 2 (or absent) either.
    pub gender: u8,
    /// Weight among the entries that have one; one of them is picked at
    /// random. `None` always applies. -1 marks a reward the player picks
    /// one of.
    pub prop: Option<i32>,
    /// Minutes a gift lasts. v83's client doesn't read it: the item comes
    /// dated.
    pub period: Option<i32>,
}

impl QuestItem {
    /// Whether this entry applies to a player of `job` and `gender`.
    pub fn applies_to(&self, job: i16, gender: u8) -> bool {
        if self.gender != 2 && self.gender != gender {
            return false;
        }
        if self.job == 0 {
            return true;
        }
        const FAMILIES: [(i32, i16); 12] = [
            (0x1, 0),
            (0x2, 100),
            (0x4, 200),
            (0x8, 300),
            (0x10, 400),
            (0x20, 500),
            (0x400, 1000),
            (0x800, 1100),
            (0x1000, 1200),
            (0x2000, 1300),
            (0x4000, 1400),
            (0x8000, 1500),
        ];
        FAMILIES
            .iter()
            .any(|&(bit, family)| self.job & bit != 0 && family / 100 == job / 100)
    }
}

/// One `Act/1/skill` entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestSkill {
    pub skill_id: i32,
    /// The level taught; 0 (no `skillLevel`) teaches only the master level.
    pub level: i32,
    pub master_level: i32,
    /// The jobs taught, matched exactly as v83's reward list does
    /// (FUN_0071808a); a 0 in the data is beginner, not "any".
    pub jobs: Vec<i16>,
}

/// The keys a quest declares in each `Check`/`Act` node, used to decide
/// whether the server supports it.
#[derive(Clone, Debug, Default)]
pub struct QuestKeys {
    pub start_check: Option<Vec<String>>,
    pub end_check: Option<Vec<String>>,
    pub start_act: Vec<String>,
    pub end_act: Vec<String>,
    /// Keys used by any `Act/<n>/item` entry.
    pub item_keys: Vec<String>,
    /// Keys used by any `Act/1/skill` entry.
    pub skill_keys: Vec<String>,
    /// Its start script completes it ([`FINISHED_BY_START_SCRIPT`]).
    pub finished_by_start_script: bool,
    /// An NPC's script completes it ([`COMPLETED_BY_NPC_SCRIPT`]).
    pub completed_by_npc_script: bool,
    /// `QuestInfo` `autoPreComplete`: the quest alert completes it.
    pub pre_complete: bool,
}

/// `Ok` if every declared requirement and action is supported, otherwise the
/// first unsupported key.
pub fn classify(keys: &QuestKeys) -> Result<(), String> {
    let start = keys.start_check.as_deref().ok_or("no start requirements")?;
    let end = keys
        .end_check
        .as_deref()
        .ok_or("no completion requirements")?;
    if !start.iter().any(|k| k == "npc") {
        return Err("no start NPC".into());
    }
    // Without a completion NPC, a quest that asks for items or kills is
    // handed in to the NPC that gave it (Pio's 1008). Without either, it is
    // a record kept by some event, not something an NPC completes.
    // Unless the start script completes it, or an end script does, at the
    // start NPC (Captain Al's 2232-2234), or another NPC's script does, or
    // the quest alert does (autoPreComplete, Maria's Map Reading 1030).
    if !keys.finished_by_start_script
        && !keys.completed_by_npc_script
        && !keys.pre_complete
        && !end
            .iter()
            .any(|k| k == "npc" || k == "item" || k == "mob" || k == "endscript")
    {
        return Err("no completion NPC".into());
    }
    unsupported(
        start,
        &[START_REQUIREMENTS, START_TRIGGERS].concat(),
        "Check/0",
    )?;
    unsupported(
        end,
        &[END_REQUIREMENTS, INERT_END_REQUIREMENTS].concat(),
        "Check/1",
    )?;
    unsupported(
        &keys.start_act,
        &[START_ACTIONS, INERT_ACTIONS].concat(),
        "Act/0",
    )?;
    unsupported(
        &keys.end_act,
        &[END_ACTIONS, INERT_ACTIONS].concat(),
        "Act/1",
    )?;
    unsupported(&keys.item_keys, ITEM_ACTION_KEYS, "Act/item")?;
    unsupported(&keys.skill_keys, SKILL_ACTION_KEYS, "Act/skill")?;
    Ok(())
}

fn unsupported(keys: &[String], allowed: &[&str], node: &str) -> Result<(), String> {
    match keys
        .iter()
        .find(|k| !allowed.contains(&k.as_str()) && !is_speech(k))
    {
        Some(key) => Err(format!("{node}/{key}")),
        None => Ok(()),
    }
}

/// `Act` nodes may repeat conversation text under numbered keys and `yes`/
/// `no`, and an offer's questions (`ask`) with their right answers
/// (`stop`), as an old copy of `Say.img`; that is speech, not an action.
fn is_speech(key: &str) -> bool {
    key.bytes().all(|b| b.is_ascii_digit()) || ["yes", "no", "ask", "stop"].contains(&key)
}

#[derive(Debug, Default)]
pub struct QuestCatalog {
    supported: BTreeMap<i16, QuestTemplate>,
    unsupported: BTreeMap<i16, String>,
    /// The titles (medals): `QuestInfo` area 51.
    titles: BTreeSet<i16>,
}

/// `QuestInfo`'s area for the titles (medals).
pub const TITLE_AREA: i32 = 51;

impl QuestCatalog {
    pub fn load_from_nx(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        let nx = NxDataFile::open(path.as_ref())?;
        let mut catalog = Self::default();
        let checks = node(&nx, "Check.img")?;
        for quest in nx.child_indices(checks)? {
            let Ok(id) = nx.node_name(quest)?.parse::<i16>() else {
                continue;
            };
            let mut keys = QuestKeys {
                start_check: keys_at(&nx, &format!("Check.img/{id}/0"))?,
                end_check: keys_at(&nx, &format!("Check.img/{id}/1"))?,
                start_act: keys_at(&nx, &format!("Act.img/{id}/0"))?.unwrap_or_default(),
                end_act: keys_at(&nx, &format!("Act.img/{id}/1"))?.unwrap_or_default(),
                ..QuestKeys::default()
            };
            for stage in ["0", "1"] {
                let Some(items) = path_index(&nx, &format!("Act.img/{id}/{stage}/item"))? else {
                    continue;
                };
                for item in nx.child_indices(items)? {
                    keys.item_keys.extend(child_names(&nx, item)?);
                }
            }
            if let Some(skills) = path_index(&nx, &format!("Act.img/{id}/1/skill"))? {
                for skill in nx.child_indices(skills)? {
                    keys.skill_keys.extend(child_names(&nx, skill)?);
                }
            }
            if EXPLORER_QUESTS.contains(&id)
                || OWN_RECORD_QUESTS.contains(&id)
                || RECORD_QUESTS.iter().any(|&(quest, _)| quest == id)
            {
                if let Some(keys) = &mut keys.end_check {
                    keys.retain(|key| !INFO_KEYS.contains(&key.as_str()));
                }
            }
            if START_RECORD_QUESTS.iter().any(|&(quest, _)| quest == id) {
                if let Some(keys) = &mut keys.start_check {
                    keys.retain(|key| !INFO_KEYS.contains(&key.as_str()));
                }
            }
            if MEREN_INFO_QUESTS.contains(&id) {
                for keys in [&mut keys.start_check, &mut keys.end_check]
                    .into_iter()
                    .flatten()
                    .chain([&mut keys.start_act])
                {
                    keys.retain(|key| !INFO_KEYS.contains(&key.as_str()));
                }
            }
            // The server keeps these quests' own info string (not one named
            // by another quest's `infoNumber`, which stays unsupported).
            if INFO_QUESTS.contains(&id) {
                keys.start_act.retain(|key| key != "info");
                let own = match path_index(&nx, &format!("Check.img/{id}/1"))? {
                    Some(end) => nx.int_child(end, "infoNumber")? == Some(i32::from(id)),
                    None => false,
                };
                if let Some(keys) = &mut keys.end_check {
                    keys.retain(|key| key != "infoex" && !(own && key == "infoNumber"));
                }
            }
            if COMPLETED_BY_NPC_SCRIPT.contains(&id) {
                if let Some(keys) = &mut keys.end_check {
                    keys.retain(|key| key != "infoex" && key != "infoNumber");
                }
                keys.completed_by_npc_script = true;
            }
            keys.finished_by_start_script = FINISHED_BY_START_SCRIPT.contains(&id)
                && keys
                    .start_check
                    .iter()
                    .flatten()
                    .any(|k| k == "startscript");
            keys.pre_complete = match path_index(&nx, &format!("QuestInfo.img/{id}"))? {
                Some(info) => nx.int_child(info, "autoPreComplete")?.unwrap_or(0) != 0,
                None => false,
            };
            if id == REQUIREMENTS_IN_START_ACTIONS {
                keys.start_act.clear();
            }
            match classify(&keys).and_then(|()| template(&nx, id).map_err(|e| e.to_string())) {
                Ok(mut template) => {
                    if MEREN_INFO_QUESTS.contains(&id) {
                        template.start.repeat_interval = None;
                    }
                    catalog.supported.insert(id, template);
                }
                Err(reason) => {
                    catalog.unsupported.insert(id, reason);
                }
            }
        }
        let infos = node(&nx, "QuestInfo.img")?;
        for info in nx.child_indices(infos)? {
            let Ok(id) = nx.node_name(info)?.parse::<i16>() else {
                continue;
            };
            if nx.int_child(info, "area")? == Some(TITLE_AREA) {
                catalog.titles.insert(id);
            }
        }
        Ok(catalog)
    }

    /// Whether a completed `id` counts toward `questComplete`. The client
    /// (0x7210c2) leaves out ids 9000 to 10999 and the titles.
    pub fn counts_as_completed(&self, id: i16) -> bool {
        !(9000..=10999).contains(&id) && !self.titles.contains(&id)
    }

    pub fn quest(&self, id: i16) -> Option<&QuestTemplate> {
        self.supported.get(&id)
    }

    pub fn supported(&self) -> impl Iterator<Item = &QuestTemplate> {
        self.supported.values()
    }

    /// Why a quest in the data is not supported, if it is not.
    pub fn unsupported_reason(&self, id: i16) -> Option<&str> {
        self.unsupported.get(&id).map(String::as_str)
    }
}

fn template(nx: &NxDataFile, id: i16) -> Result<QuestTemplate, GameDataError> {
    let start = node(nx, &format!("Check.img/{id}/0"))?;
    let end = node(nx, &format!("Check.img/{id}/1"))?;
    let mut requirements = StartRequirements {
        npc: nx
            .int_child(start, "npc")?
            .ok_or_else(|| invalid("missing start NPC"))?,
        min_level: nx
            .int_child(start, "lvmin")?
            .map(|v| small(v, "level"))
            .transpose()?,
        max_level: nx
            .int_child(start, "lvmax")?
            .map(|v| small(v, "level"))
            .transpose()?,
        min_fame: small(nx.int_child(start, "pop")?.unwrap_or(0), "fame")?,
        // v83 treats a negative interval, like none, as not repeatable.
        repeat_interval: nx
            .int_child(start, "interval")?
            .and_then(|v| u32::try_from(v).ok()),
        daily: nx.int_child(start, "dayByDay")?.unwrap_or(0) != 0,
        ..StartRequirements::default()
    };
    if let Some(jobs) = nx.child_by_name(start, "job")? {
        for job in nx.child_indices(jobs)? {
            let name = nx.node_name(job)?;
            let value = nx
                .int_child(jobs, &name)?
                .ok_or_else(|| invalid("non-integer job requirement"))?;
            requirements.jobs.push(small(value, "job")?);
        }
    }
    requirements.quests = prerequisites(nx, start)?;
    requirements.worn_all = int_list(nx, start, "equipAllNeed")?;
    requirements.worn_any = int_list(nx, start, "equipSelectNeed")?;
    requirements.maps = int_list(nx, start, "fieldEnter")?;
    requirements.not_before = nx.int_child(start, "start")?.map(date_hour).transpose()?;
    requirements.not_after = nx.int_child(start, "end")?.map(date_hour).transpose()?;
    requirements.weekdays = weekdays(nx, start)?;
    requirements.items = item_counts(nx, start)?;
    requirements.pets = pet_ids(nx, start)?;
    requirements.pet_closeness = pet_closeness(nx, start)?;
    requirements.pet_skill_limit = pet_skill_limit(nx, start)?;
    requirements.mount_level = mount_level(nx, start)?;
    requirements.buff = nx.int_child(start, "buff")?;
    requirements.except_buff = nx.int_child(start, "exceptbuff")?;
    if let Some(&(_, record)) = START_RECORD_QUESTS.iter().find(|&&(quest, _)| quest == id) {
        if nx.int_child(start, "infoNumber")? != Some(i32::from(record)) {
            return Err(invalid("start record quest with another record"));
        }
        requirements.record = Some((record, info_checks(nx, start)?));
    }
    if let Some(skills) = nx.child_by_name(start, "skill")? {
        for skill in nx.child_indices(skills)? {
            let skill_id = nx
                .int_child(skill, "id")?
                .ok_or_else(|| invalid("skill requirement without id"))?;
            let acquire = nx.int_child(skill, "acquire")?.unwrap_or(0) != 0;
            requirements.skills.push((skill_id, acquire));
        }
    }
    let end_items = item_counts(nx, end)?;
    let mut end_mobs = Vec::new();
    if let Some(mobs) = nx.child_by_name(end, "mob")? {
        for mob in nx.child_indices(mobs)? {
            let mob_id = nx
                .int_child(mob, "id")?
                .ok_or_else(|| invalid("mob requirement without id"))?;
            let count = nx.int_child(mob, "count")?.unwrap_or(1);
            end_mobs.push((
                mob_id,
                u16::try_from(count).map_err(|_| invalid("mob count out of range"))?,
            ));
        }
    }
    let actions = path_index(nx, &format!("Act.img/{id}/1"))?;
    let action_int = |key: &str| match actions {
        Some(index) => nx.int_child(index, key),
        None => Ok(None),
    };
    let exp = action_int("exp")?.unwrap_or(0);
    let start_actions = path_index(nx, &format!("Act.img/{id}/0"))?;
    let start_action_int = |key: &str| match start_actions {
        Some(index) => nx.int_child(index, key),
        None => Ok(None),
    };
    let start_exp = start_action_int("exp")?.unwrap_or(0);
    if exp < 0 || start_exp < 0 {
        return Err(invalid("negative EXP reward"));
    }
    let info = path_index(nx, &format!("QuestInfo.img/{id}"))?;
    let name = match info {
        Some(info) => nx.string_child(info, "name")?.unwrap_or_default(),
        None => String::new(),
    };
    let flag = |key: &str| -> Result<bool, GameDataError> {
        Ok(match info {
            Some(info) => nx.int_child(info, key)?.unwrap_or(0) != 0,
            None => false,
        })
    };
    let end_record = if OWN_RECORD_QUESTS.contains(&id) {
        let record = explored(nx, end)?;
        if record.record != id || !end_mobs.is_empty() {
            return Err(invalid("own-record quest with another record or mobs"));
        }
        Some(u16::try_from(record.needed).map_err(|_| invalid("record count too large"))?)
    } else {
        None
    };
    let explored = if EXPLORER_QUESTS.contains(&id) {
        Some(explored(nx, end)?)
    } else {
        None
    };
    let end_info_record = RECORD_QUESTS
        .iter()
        .find(|&&(quest, _)| quest == id)
        .map(|&(_, record)| record);
    let (start_info, end_info) = if INFO_QUESTS.contains(&id) {
        // One whose start sets no string (Shining Stone, 2166) starts with
        // an empty one, which a script sets whole.
        let start_info = match start_actions {
            Some(index) => text_child(nx, index, "info")?,
            None => None,
        };
        (Some(start_info.unwrap_or_default()), info_checks(nx, end)?)
    } else if let Some(record) = end_info_record {
        if nx.int_child(end, "infoNumber")? != Some(i32::from(record)) {
            return Err(invalid("record quest with another record"));
        }
        (None, info_checks(nx, end)?)
    } else if COMPLETED_BY_NPC_SCRIPT.contains(&id) {
        // Kept so the quest window can't complete it either.
        (None, info_checks(nx, end)?)
    } else {
        (None, Vec::new())
    };
    // See `classify`: quests without a completion NPC go back to the start NPC.
    let start_npc = requirements.npc;
    Ok(QuestTemplate {
        id,
        name,
        start: requirements,
        end_npc: nx.int_child(end, "npc")?.unwrap_or(start_npc),
        end_items,
        end_mobs,
        end_quests: prerequisites(nx, end)?,
        end_meso: nx.int_child(end, "endmeso")?.unwrap_or(0),
        end_min_level: nx
            .int_child(end, "level")?
            .map(|v| small(v, "level"))
            .transpose()?,
        end_pets: pet_ids(nx, end)?,
        end_pet_closeness: pet_closeness(nx, end)?,
        end_pet_skill_limit: pet_skill_limit(nx, end)?,
        end_buff: nx.int_child(end, "buff")?,
        end_except_buff: nx.int_child(end, "exceptbuff")?,
        end_book_min: nx.int_child(end, "mbmin")?,
        end_book_max: nx.int_child(end, "mbmax")?,
        end_cards: book_cards(nx, end)?,
        end_completed_quests: nx.int_child(end, "questComplete")?,
        end_party_quest_s: nx.int_child(end, "partyQuest_S")?,
        start_items: quest_items(nx, &format!("Act.img/{id}/0"))?,
        end_rewards: {
            let mut rewards = quest_items(nx, &format!("Act.img/{id}/1"))?;
            if KNIGHT_TITLES.contains(&id) {
                let medal = info
                    .map(|info| nx.int_child(info, "viewMedalItem"))
                    .transpose()?
                    .flatten()
                    .ok_or_else(|| invalid("Knight title without a medal"))?;
                rewards.push(QuestItem {
                    item_id: medal,
                    count: 1,
                    job: 0,
                    gender: 2,
                    prop: None,
                    period: None,
                });
            }
            rewards
        },
        start_exp,
        start_money: start_action_int("money")?.unwrap_or(0),
        start_buff_item: start_action_int("buffItemID")?,
        end_buff_item: action_int("buffItemID")?,
        end_npc_act: match actions {
            Some(index) => nx.string_child(index, "npcAct")?,
            None => None,
        },
        end_skills: quest_skills(nx, &format!("Act.img/{id}/1"))?,
        start_script: script(nx, start, "startscript")?,
        end_script: script(nx, end, "endscript")?,
        exp,
        money: action_int("money")?.unwrap_or(0),
        fame: small(action_int("pop")?.unwrap_or(0), "fame")?,
        pet_closeness: small(action_int("pettameness")?.unwrap_or(0), "pet closeness")?,
        pet_skill: u16::try_from(action_int("petskill")?.unwrap_or(0))
            .map_err(|_| invalid("pet skill out of range"))?,
        pet_speed: action_int("petspeed")?.is_some_and(|speed| speed != 0),
        next_quest: action_int("nextQuest")?
            .map(|id| small(id, "quest id"))
            .transpose()?,
        auto_start: flag("autoStart")?,
        auto_complete: flag("autoComplete")?,
        auto_pre_complete: flag("autoPreComplete")?,
        explored,
        end_record,
        start_info,
        end_info,
        end_info_record,
        time_limit: seconds(nx, info, "timeLimit")?,
        time_limit2: seconds(nx, info, "timeLimit2")?,
        daily_play_time: seconds(nx, info, "dailyPlayTime")?,
    })
}

/// A positive count of seconds under the quest's `QuestInfo` node, if any.
fn seconds(nx: &NxDataFile, info: Option<u32>, key: &str) -> Result<Option<u32>, GameDataError> {
    let Some(info) = info else {
        return Ok(None);
    };
    nx.int_child(info, key)?
        .filter(|&seconds| seconds > 0)
        .map(|seconds| u32::try_from(seconds).map_err(|_| invalid(key)))
        .transpose()
}

/// A string child, or an integer one written out ("0" may be stored as 0).
fn text_child(nx: &NxDataFile, parent: u32, key: &str) -> Result<Option<String>, GameDataError> {
    Ok(match nx.string_child(parent, key)? {
        Some(text) => Some(text),
        None => nx.int_child(parent, key)?.map(|value| value.to_string()),
    })
}

/// Every `infoex` entry of a `Check/0` or `Check/1` node.
fn info_checks(nx: &NxDataFile, end: u32) -> Result<Vec<InfoCheck>, GameDataError> {
    let Some(infoex) = nx.child_by_name(end, "infoex")? else {
        return Ok(Vec::new());
    };
    let mut checks = Vec::new();
    for entry in nx.child_indices(infoex)? {
        let value =
            text_child(nx, entry, "value")?.ok_or_else(|| invalid("infoex entry without value"))?;
        let cond = nx.int_child(entry, "cond")?.unwrap_or(0);
        checks.push(InfoCheck {
            cond: u8::try_from(cond).map_err(|_| invalid("infoex cond out of range"))?,
            value,
        });
    }
    Ok(checks)
}

/// An explorer medal's `infoNumber` and its one `infoex` entry, a count to
/// reach (cond 1), kept as a string.
fn explored(nx: &NxDataFile, end: u32) -> Result<Explored, GameDataError> {
    let record = nx
        .int_child(end, "infoNumber")?
        .ok_or_else(|| invalid("explorer medal without infoNumber"))?;
    let entry = match nx.child_by_name(end, "infoex")? {
        Some(infoex) => nx.child_by_name(infoex, "0")?,
        None => None,
    }
    .ok_or_else(|| invalid("explorer medal without infoex"))?;
    if nx.int_child(entry, "cond")? != Some(1) {
        return Err(invalid("explorer medal infoex isn't a count"));
    }
    // A numeric string, which reads as a number.
    let needed = nx
        .int_child(entry, "value")?
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| invalid("explorer medal infoex value isn't a count"))?;
    Ok(Explored {
        record: small(record, "quest id")?,
        needed,
    })
}

/// The integers listed under `key` of a `Check` node.
fn int_list(nx: &NxDataFile, check: u32, key: &str) -> Result<Vec<i32>, GameDataError> {
    let Some(list) = nx.child_by_name(check, key)? else {
        return Ok(Vec::new());
    };
    let mut result = Vec::new();
    for entry in nx.child_indices(list)? {
        let name = nx.node_name(entry)?;
        result.push(
            nx.int_child(list, &name)?
                .ok_or_else(|| invalid("non-integer list entry"))?,
        );
    }
    Ok(result)
}

/// The cards listed under `mbcard` of a `Check` node, each with the least
/// and most of it the Monster Book may hold.
fn book_cards(nx: &NxDataFile, check: u32) -> Result<Vec<BookCard>, GameDataError> {
    let Some(cards) = nx.child_by_name(check, "mbcard")? else {
        return Ok(Vec::new());
    };
    let mut result = Vec::new();
    for card in nx.child_indices(cards)? {
        result.push(BookCard {
            card: nx
                .int_child(card, "id")?
                .ok_or_else(|| invalid("mbcard entry without id"))?,
            min: nx.int_child(card, "min")?,
            max: nx.int_child(card, "max")?,
        });
    }
    Ok(result)
}

/// The pet items listed under `pet` of a `Check` node (`pet/<n>/id`).
fn pet_ids(nx: &NxDataFile, check: u32) -> Result<Vec<i32>, GameDataError> {
    let Some(pets) = nx.child_by_name(check, "pet")? else {
        return Ok(Vec::new());
    };
    let mut result = Vec::new();
    for pet in nx.child_indices(pets)? {
        result.push(
            nx.int_child(pet, "id")?
                .ok_or_else(|| invalid("pet requirement without id"))?,
        );
    }
    Ok(result)
}

/// The closeness range of a `Check` node (`pettamenessmin`,
/// `pettamenessmax`), if it has one. v83 (FUN_00721163, FUN_00721d2c)
/// tests the first pet out's closeness against it, with 30000 for a
/// missing most.
fn pet_closeness(nx: &NxDataFile, check: u32) -> Result<Option<(i32, i32)>, GameDataError> {
    let min = nx.int_child(check, "pettamenessmin")?;
    let max = nx.int_child(check, "pettamenessmax")?;
    if min.is_none() && max.is_none() {
        return Ok(None);
    }
    Ok(Some((min.unwrap_or(0), max.unwrap_or(30_000))))
}

/// The mount level range of a start `Check` node (`tamingmoblevelmin`,
/// `tamingmoblevelmax`), if it has one.
fn mount_level(nx: &NxDataFile, check: u32) -> Result<Option<(i32, i32)>, GameDataError> {
    let min = nx.int_child(check, "tamingmoblevelmin")?;
    let max = nx.int_child(check, "tamingmoblevelmax")?;
    if min.is_none() && max.is_none() {
        return Ok(None);
    }
    Ok(Some((min.unwrap_or(0), max.unwrap_or(30))))
}

/// The `petSkill` bits a `Check` node's first pet out must lack:
/// `petRecallLimit` rules out recall (0x80) and `petAutoSpeakingLimit` auto
/// speaking (0x100), as v83 tests them (FUN_00721163, FUN_00721d2c).
fn pet_skill_limit(nx: &NxDataFile, check: u32) -> Result<u16, GameDataError> {
    let on = |key: &str| Ok::<_, GameDataError>(nx.int_child(check, key)?.is_some_and(|v| v != 0));
    let mut bits = 0;
    if on("petRecallLimit")? {
        bits |= PET_RECALL;
    }
    if on("petAutoSpeakingLimit")? {
        bits |= PET_AUTO_SPEAKING;
    }
    Ok(bits)
}

/// The `quest` requirements of a `Check` node: other quests' states.
fn prerequisites(nx: &NxDataFile, check: u32) -> Result<Vec<QuestPrerequisite>, GameDataError> {
    let Some(quests) = nx.child_by_name(check, "quest")? else {
        return Ok(Vec::new());
    };
    let mut result = Vec::new();
    for prerequisite in nx.child_indices(quests)? {
        let quest_id = nx
            .int_child(prerequisite, "id")?
            .ok_or_else(|| invalid("prerequisite without id"))?;
        // A prerequisite without a state must not have been started; the
        // data uses this for mutually exclusive quests.
        let state = nx
            .int_child(prerequisite, "state")?
            .unwrap_or(QuestState::NotStarted as i32);
        result.push(QuestPrerequisite {
            quest_id: small(quest_id, "quest id")?,
            state: QuestState::from_number(state)
                .ok_or_else(|| invalid("invalid prerequisite state"))?,
        });
    }
    Ok(result)
}

fn quest_items(nx: &NxDataFile, act: &str) -> Result<Vec<QuestItem>, GameDataError> {
    let Some(items) = path_index(nx, &format!("{act}/item"))? else {
        return Ok(Vec::new());
    };
    // The files list entries by name as text ("10" before "2"); v83 goes by
    // the number, which numbers the rewards a player picks from.
    let mut entries = Vec::new();
    for item in nx.child_indices(items)? {
        let name = nx.node_name(item)?;
        entries.push((name.parse::<u32>().unwrap_or(u32::MAX), item));
    }
    entries.sort_by_key(|&(order, _)| order);
    let mut result = Vec::new();
    for (_, item) in entries {
        result.push(QuestItem {
            item_id: nx
                .int_child(item, "id")?
                .ok_or_else(|| invalid("item action without id"))?,
            count: nx.int_child(item, "count")?.unwrap_or(0),
            job: nx.int_child(item, "job")?.unwrap_or(0),
            gender: u8::try_from(nx.int_child(item, "gender")?.unwrap_or(2)).unwrap_or(2),
            prop: nx.int_child(item, "prop")?,
            period: nx.int_child(item, "period")?,
        });
    }
    Ok(result)
}

fn quest_skills(nx: &NxDataFile, act: &str) -> Result<Vec<QuestSkill>, GameDataError> {
    let Some(skills) = path_index(nx, &format!("{act}/skill"))? else {
        return Ok(Vec::new());
    };
    let mut result = Vec::new();
    for skill in nx.child_indices(skills)? {
        let mut jobs = Vec::new();
        if let Some(list) = nx.child_by_name(skill, "job")? {
            for job in nx.child_indices(list)? {
                let name = nx.node_name(job)?;
                let value = nx
                    .int_child(list, &name)?
                    .ok_or_else(|| invalid("non-integer skill job"))?;
                jobs.push(small(value, "job")?);
            }
        }
        result.push(QuestSkill {
            skill_id: nx
                .int_child(skill, "id")?
                .ok_or_else(|| invalid("skill action without id"))?,
            level: nx.int_child(skill, "skillLevel")?.unwrap_or(0),
            master_level: nx.int_child(skill, "masterLevel")?.unwrap_or(0),
            jobs,
        });
    }
    Ok(result)
}

/// `dayOfWeek`: the days, `sun` to `sat`, whose value isn't 0, read as v83
/// does (FUN_0071e1d6). A day left out counts as 0, so a key with no day set
/// opens on none.
fn weekdays(nx: &NxDataFile, start: u32) -> Result<Option<u8>, GameDataError> {
    let Some(days) = nx.child_by_name(start, "dayOfWeek")? else {
        return Ok(None);
    };
    let mut mask = 0;
    for (bit, day) in ["sun", "mon", "tue", "wed", "thu", "fri", "sat"]
        .iter()
        .enumerate()
    {
        if nx.int_child(days, day)?.unwrap_or(0) != 0 {
            mask |= 1 << bit;
        }
    }
    Ok(Some(mask))
}

/// The hour a quest date names, read as v83 does (FUN_00720d26): the
/// digits "yyyymmddhh" as year, month, day and hour, in UTC.
fn date_hour(date: i32) -> Result<SystemTime, GameDataError> {
    let (year, month, day, hour) = (
        date / 1_000_000,
        date / 10_000 % 100,
        date / 100 % 100,
        date % 100,
    );
    if !(1601..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
    {
        return Err(invalid("bad quest date"));
    }
    // Howard Hinnant's days_from_civil.
    let (y, m) = if month <= 2 {
        (i64::from(year) - 1, i64::from(month) + 9)
    } else {
        (i64::from(year), i64::from(month) - 3)
    };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let day_of_year = (153 * m + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    let seconds = days * 86_400 + i64::from(hour) * 3_600;
    Ok(match u64::try_from(seconds) {
        Ok(after) => SystemTime::UNIX_EPOCH + Duration::from_secs(after),
        Err(_) => SystemTime::UNIX_EPOCH - Duration::from_secs(seconds.unsigned_abs()),
    })
}

/// A supported script name, if the node names one.
fn script(nx: &NxDataFile, parent: u32, key: &str) -> Result<Option<String>, GameDataError> {
    match nx.string_child(parent, key)? {
        None => Ok(None),
        Some(name) if SCRIPTS.contains(&name.as_str()) => Ok(Some(name)),
        Some(name) => Err(invalid(&format!("unsupported script {name}"))),
    }
}

fn keys_at(nx: &NxDataFile, path: &str) -> Result<Option<Vec<String>>, GameDataError> {
    match path_index(nx, path)? {
        Some(index) => child_names(nx, index).map(Some),
        None => Ok(None),
    }
}

fn child_names(nx: &NxDataFile, index: u32) -> Result<Vec<String>, GameDataError> {
    nx.child_indices(index)?
        .into_iter()
        .map(|child| nx.node_name(child))
        .collect()
}

fn path_index(nx: &NxDataFile, path: &str) -> Result<Option<u32>, GameDataError> {
    let mut index = 0;
    for key in path.split('/') {
        match nx.child_by_name(index, key)? {
            Some(child) => index = child,
            None => return Ok(None),
        }
    }
    Ok(Some(index))
}

fn node(nx: &NxDataFile, path: &str) -> Result<u32, GameDataError> {
    path_index(nx, path)?.ok_or_else(|| invalid(&format!("missing quest node {path}")))
}

/// The `item` list of a `Check.img` stage, as `(item id, count)`.
fn item_counts(nx: &NxDataFile, stage: u32) -> Result<Vec<(i32, i32)>, GameDataError> {
    let mut counts = Vec::new();
    if let Some(items) = nx.child_by_name(stage, "item")? {
        for item in nx.child_indices(items)? {
            let item_id = nx
                .int_child(item, "id")?
                .ok_or_else(|| invalid("item requirement without id"))?;
            counts.push((item_id, nx.int_child(item, "count")?.unwrap_or(0)));
        }
    }
    Ok(counts)
}

fn small(value: i32, what: &str) -> Result<i16, GameDataError> {
    i16::try_from(value).map_err(|_| invalid(&format!("{what} out of range")))
}

fn invalid(message: &str) -> GameDataError {
    GameDataError::InvalidData(message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(start: &[&str], end: &[&str], start_act: &[&str], end_act: &[&str]) -> QuestKeys {
        let owned = |k: &[&str]| k.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        QuestKeys {
            start_check: Some(owned(start)),
            end_check: Some(owned(end)),
            start_act: owned(start_act),
            end_act: owned(end_act),
            ..QuestKeys::default()
        }
    }

    #[test]
    fn talk_and_exp_quests_are_supported() {
        let quiz = keys(
            &["npc", "job", "quest"],
            &["npc"],
            &[],
            &["exp", "nextQuest"],
        );
        assert_eq!(classify(&quiz), Ok(()));
        // Act/0 sometimes repeats the offer text under numbered keys.
        let contact = keys(
            &["npc", "job", "lvmax"],
            &["npc"],
            &["0", "1", "yes", "no"],
            &["exp"],
        );
        assert_eq!(classify(&contact), Ok(()));
        // Or its questions, and the answers that are right.
        let quiz = keys(&["npc"], &["npc"], &["0", "1", "ask", "stop"], &["exp"]);
        assert_eq!(classify(&quiz), Ok(()));
        // Completion dates are not checked.
        let dated = keys(&["npc"], &["npc", "start", "end"], &[], &["exp"]);
        assert_eq!(classify(&dated), Ok(()));
        // Or copies of its requirements, which nothing reads.
        let copied = keys(
            &["npc", "lvmin", "quest"],
            &["npc"],
            &[
                "npc", "lvmin", "lvmax", "job", "quest", "interval", "start", "end",
            ],
            &["exp", "npc"],
        );
        assert_eq!(classify(&copied), Ok(()));
    }

    #[test]
    fn any_unsupported_key_marks_the_quest_unsupported() {
        let hunting = keys(&["npc"], &["npc", "mob"], &[], &["exp", "item"]);
        assert_eq!(classify(&hunting), Ok(()));
        let mut expiring = hunting.clone();
        expiring.item_keys = vec!["id".into(), "count".into(), "dateExpire".into()];
        assert_eq!(classify(&expiring), Err("Act/item/dateExpire".into()));
        let pet = keys(&["npc", "pet"], &["npc", "pet"], &[], &[]);
        assert_eq!(classify(&pet), Ok(()));
        let tame = keys(&["npc", "pet", "pettamenessmin"], &["npc"], &[], &[]);
        assert_eq!(classify(&tame), Ok(()));
        let money = keys(&["npc"], &["npc"], &[], &["money"]);
        assert_eq!(classify(&money), Ok(()));
        let start_money = keys(&["npc"], &["npc"], &["money", "exp"], &[]);
        assert_eq!(classify(&start_money), Ok(()));
        let start_fame = keys(&["npc"], &["npc"], &["pop"], &[]);
        assert_eq!(classify(&start_fame), Err("Act/0/pop".into()));
        let mut skill = keys(&["npc"], &["npc"], &[], &["skill"]);
        skill.skill_keys = vec!["id".into(), "masterLevel".into(), "job".into()];
        assert_eq!(classify(&skill), Ok(()));
        skill.skill_keys.push("onlyMasterLevel".into());
        assert_eq!(classify(&skill), Ok(()));
    }

    #[test]
    fn quests_need_a_start_and_a_completion_npc() {
        let auto_start = keys(&["job"], &["npc"], &[], &[]);
        assert!(classify(&auto_start).is_err());
        let auto_complete = keys(&["npc"], &[], &[], &[]);
        assert!(classify(&auto_complete).is_err());
        let hand_in = keys(&["npc"], &["item"], &[], &[]);
        assert_eq!(classify(&hand_in), Ok(()));
        let mut no_end = keys(&["npc"], &[], &[], &[]);
        no_end.end_check = None;
        assert!(classify(&no_end).is_err());
        let mut informant = keys(&["npc", "startscript"], &[], &[], &[]);
        assert!(classify(&informant).is_err());
        informant.finished_by_start_script = true;
        assert_eq!(classify(&informant), Ok(()));
        let mut scripted = keys(&["npc"], &[], &[], &[]);
        assert!(classify(&scripted).is_err());
        scripted.completed_by_npc_script = true;
        assert_eq!(classify(&scripted), Ok(()));
        // The quest alert completes an autoPreComplete quest.
        let mut alert = keys(&["npc", "fieldEnter", "item", "job"], &[], &[], &["exp"]);
        assert!(classify(&alert).is_err());
        alert.pre_complete = true;
        assert_eq!(classify(&alert), Ok(()));
    }

    /// The supported set must match `supported_quests.txt`, which HeavenClient
    /// reads, so the two can't disagree about what is offered.
    #[test]
    fn supported_quests_match_the_golden_list() {
        let Some(path) = crate::test_assets::path("Quest.nx") else {
            return;
        };
        let catalog = QuestCatalog::load_from_nx(path).unwrap();
        let actual: Vec<i16> = catalog.supported().map(|q| q.id).collect();
        let golden: Vec<i16> = include_str!("../supported_quests.txt")
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .map(|l| l.trim().parse().unwrap())
            .collect();
        assert_eq!(actual, golden, "update game-data/supported_quests.txt");
    }

    #[test]
    fn knight_titles_end_with_their_medal_and_held_medal_titles_are_offered() {
        let Some(path) = crate::test_assets::path("Quest.nx") else {
            return;
        };
        let catalog = QuestCatalog::load_from_nx(path).unwrap();
        let supported: Vec<i16> = catalog.supported().map(|q| q.id).collect();
        for (quest, medal) in [
            (29906, 1142066),
            (29907, 1142067),
            (29908, 1142068),
            (29909, 1142069),
        ] {
            assert!(supported.contains(&quest), "{quest}");
            let rewards = &catalog.quest(quest).unwrap().end_rewards;
            assert_eq!(rewards.len(), 1, "{quest}");
            assert_eq!(
                (rewards[0].item_id, rewards[0].count),
                (medal, 1),
                "{quest}"
            );
        }
        for quest in 29922..=29928 {
            assert!(supported.contains(&quest), "{quest}");
        }
    }

    #[test]
    fn titles_and_ids_from_9000_to_10999_are_not_counted_as_completed() {
        let Some(path) = crate::test_assets::path("Quest.nx") else {
            return;
        };
        let catalog = QuestCatalog::load_from_nx(path).unwrap();
        for id in [29000, 29001, 9000, 10999] {
            assert!(!catalog.counts_as_completed(id), "{id}");
        }
        for id in [1000, 8999, 11000] {
            assert!(catalog.counts_as_completed(id), "{id}");
        }
    }

    #[test]
    fn info_checks_read_as_v83_reads_them() {
        let check = |cond, value: &str| InfoCheck {
            cond,
            value: value.into(),
        };
        let exact = [check(0, "3333")];
        assert!(info_met(Some("3333"), &exact));
        assert!(!info_met(Some("3323"), &exact));
        assert!(!info_met(None, &exact));
        assert!(info_met(Some("12x"), &[check(1, "12")]));
        assert!(!info_met(Some("11"), &[check(1, "12")]));
        assert!(info_met(Some("abc"), &[check(2, "0")]));
        assert!(info_met(Some("7"), &[check(0, "5"), check(2, "9")]));
    }

    #[test]
    fn info_quests_load_their_strings() {
        let Some(path) = crate::test_assets::path("Quest.nx") else {
            return;
        };
        let catalog = QuestCatalog::load_from_nx(path).unwrap();
        let depots = catalog.quest(3926).unwrap();
        assert_eq!(depots.start_info.as_deref(), Some("2222"));
        assert_eq!(
            depots.end_info,
            [InfoCheck {
                cond: 0,
                value: "3333".into()
            }]
        );
        let oasis = catalog.quest(3900).unwrap();
        assert_eq!(oasis.start_info.as_deref(), Some("4"));
        // Shining Stone starts empty; the Shiny Stone sets the "5" Sharyl
        // reads.
        let stone = catalog.quest(2166).unwrap();
        assert_eq!(
            (stone.start_info.as_deref(), stone.end_npc, stone.exp),
            (Some(""), 1092003, 400)
        );
        assert_eq!(
            stone.end_info,
            [InfoCheck {
                cond: 0,
                value: "5".into()
            }]
        );
        // The Test starts once the opening scene has set record 2311 to 1.
        let test = catalog.quest(2312).unwrap();
        assert_eq!(
            test.start.record,
            Some((
                2311,
                vec![InfoCheck {
                    cond: 1,
                    value: "1".into()
                }]
            ))
        );
        assert_eq!((test.start.npc, test.exp), (1300005, 11500));
        // Every info quest starts with a string and counts no mobs, whose
        // counts would share the record with it.
        for &id in INFO_QUESTS {
            let quest = catalog.quest(id).unwrap_or_else(|| panic!("{id}"));
            assert!(quest.start_info.is_some(), "{id}");
            assert!(quest.end_mobs.is_empty(), "{id}");
        }
        // Quests outside the list keep infoex unsupported.
        assert!(catalog.quest(10052).is_none());
        // "Jonathan's Court" reads Lord Jonathan's test's record.
        let court = catalog.quest(6400).unwrap();
        assert_eq!(court.end_info_record, Some(6401));
        assert_eq!(
            court.end_info,
            [InfoCheck {
                cond: 0,
                value: "q3".into()
            }]
        );
        assert!(court.start_info.is_none());
        // Kyrin's tests read theirs, and "A New Revelation" its own.
        for (id, record) in [(6330, 6331), (6370, 6371), (6363, 6364)] {
            let test = catalog.quest(id).unwrap();
            assert_eq!(test.end_info_record, Some(record));
            assert_eq!(test.end_info[0].value, "2");
        }
        let delli = catalog.quest(6410).unwrap();
        assert_eq!(delli.end_info_record, Some(6411));
        assert_eq!(delli.end_info[0].value, "p2");
        // Protector of Pharaoh's count of Pyramid kills, at least 50,000.
        let pharaoh = catalog.quest(29932).unwrap();
        assert_eq!(pharaoh.end_info_record, Some(7760));
        assert_eq!(
            pharaoh.end_info,
            [InfoCheck {
                cond: 1,
                value: "50000".into()
            }]
        );
    }

    #[test]
    fn loads_held_item_and_skill_start_requirements() {
        let Some(path) = crate::test_assets::path("Quest.nx") else {
            return;
        };
        let catalog = QuestCatalog::load_from_nx(path).unwrap();
        // Borrowing Sera's Mirror asks for the GM shirt, held or worn.
        let mirror = catalog.quest(1000).unwrap();
        assert_eq!(mirror.start.items, [(1042003, 1)]);
        let mastery = catalog.quest(2415).unwrap();
        assert_eq!(mastery.start.skills, [(1001004, true)]);
        // Huckle's daily ingredients, and a quest done once.
        assert_eq!(
            catalog.quest(3038).unwrap().start.repeat_interval,
            Some(1440)
        );
        assert_eq!(mirror.start.repeat_interval, None);
        assert!(!mirror.start.daily);
        // Maria's Map Reading has no completion requirements; the quest
        // alert completes it.
        let maria = catalog.quest(1030).unwrap();
        assert!(maria.auto_pre_complete && !maria.auto_complete);
        assert_eq!((maria.end_npc, maria.exp), (2103, 11));
        assert!(catalog.quest(8245).unwrap().start.daily);
        // An event quest open from 00:00 to 01:00 on 31 December 2030.
        assert_eq!(
            catalog.quest(9625).unwrap().start.not_after,
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_924_909_200))
        );
        // The Magatia cape worn; Maple Administrator's hat, in his map.
        assert_eq!(catalog.quest(3307).unwrap().start.worn_all, [1102135]);
        let gm = &catalog.quest(9999).unwrap().start;
        assert_eq!(
            (&gm.worn_any[..], &gm.maps[..]),
            (&[1002140][..], &[180000000][..])
        );
        assert_eq!(catalog.quest(3300).unwrap().end_meso, 10000);
        // Jr. Officer Medin's plan gives half an hour; Sera's Mirror has no clock.
        assert_eq!(catalog.quest(3458).unwrap().time_limit, Some(1800));
        assert_eq!(mirror.time_limit, None);
        // Diligent Explorer has 30 days; every timed title keeps its end date.
        assert_eq!(catalog.quest(29003).unwrap().time_limit2, Some(2_592_000));
        assert_eq!(catalog.quest(29003).unwrap().daily_play_time, Some(10_800));
        assert_eq!(
            catalog
                .supported()
                .filter(|q| q.daily_play_time.is_some())
                .map(|q| q.id)
                .collect::<Vec<_>>(),
            [29003]
        );
        for quest in catalog.supported().filter(|q| q.time_limit2.is_some()) {
            assert!(
                game::quest::DATED_QUESTS.contains(&quest.id),
                "{}",
                quest.id
            );
        }
        assert_eq!(catalog.quest(29300).unwrap().end_min_level, Some(200));
        // Bartos's test: any of the 45 pets out, to start and to finish.
        let test = catalog.quest(4646).unwrap();
        assert_eq!(test.start.pets.len(), 45);
        assert!(test.start.pets.contains(&5000000));
        assert_eq!(test.end_pets, test.start.pets);
        assert_eq!(test.start.pet_closeness, None);
        // Garnox's first evolution quest wants a Baby Dragon at 1642.
        let evolution = catalog.quest(8184).unwrap();
        assert_eq!(evolution.start.pets, [5000029]);
        assert_eq!(evolution.start.pet_closeness, Some((1642, 30_000)));
        assert_eq!(
            catalog.quest(4647).unwrap().end_script.as_deref(),
            Some("q4647e")
        );
        // Rupi's paper lasts an hour.
        assert_eq!(catalog.quest(3085).unwrap().start_items[0].period, Some(60));
        // Not before Christmas Eve 2005; a fee, and EXP, to start; a
        // transformation for finishing.
        assert_eq!(
            catalog.quest(3600).unwrap().start.not_before,
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_135_382_400))
        );
        // Saturdays only, and weekdays only (Monday to Friday).
        assert_eq!(
            catalog.quest(8248).unwrap().start.weekdays,
            Some(0b100_0000)
        );
        assert_eq!(
            catalog.quest(8254).unwrap().start.weekdays,
            Some(0b011_1110)
        );
        assert_eq!(catalog.quest(3600).unwrap().start.weekdays, None);
        assert_eq!(catalog.quest(2029).unwrap().start_money, -1000);
        assert_eq!(catalog.quest(3406).unwrap().start_exp, 1600);
        // Mark's script completes Dollhouse; the quest window never can.
        let dollhouse = catalog.quest(3230).unwrap();
        assert_eq!(dollhouse.end_info[0].value, "dummy");
        assert_eq!(dollhouse.start_info, None);
        assert_eq!(catalog.quest(3239).unwrap().end_info[0].value, "dummy");
        // The Security Room's door completes Eliminating the Rest; the
        // quest window wants its own record at least 1, which nothing sets.
        let rest = catalog.quest(2335).unwrap();
        assert_eq!(
            (rest.end_info[0].cond, rest.end_info[0].value.as_str()),
            (1, "1")
        );
        assert_eq!(rest.end_info_record, None);
        assert_eq!(rest.start_script.as_deref(), Some("q2335s"));
        assert_eq!(catalog.quest(7301).unwrap().end_buff_item, Some(2210003));
        // The completing NPC's special action.
        let snow_sculpture = catalog.quest(3110).unwrap();
        assert_eq!(snow_sculpture.end_npc_act.as_deref(), Some("act3110"));
        assert_eq!(
            catalog.quest(2278).unwrap().end_npc_act.as_deref(),
            Some("action")
        );
        assert_eq!(catalog.quest(7301).unwrap().end_npc_act, None);
        // Its first step's offer asks questions (`Act/0/ask`, `stop`).
        assert_eq!(catalog.quest(7300).unwrap().start.min_level, Some(80));
        // The token taken, then scrolls to pick from, listed "1" to "16" in
        // numeric order.
        let rewards = &catalog.quest(3379).unwrap().end_rewards;
        assert_eq!(rewards.len(), 17);
        assert_eq!(rewards[0].prop, None);
        assert_eq!(rewards[2].item_id, 2043201);
        assert_eq!(rewards[10].item_id, 2044301);
        assert!(rewards[1..].iter().all(|item| item.prop == Some(-1)));
    }

    #[test]
    fn loads_beginner_quests_from_assets() {
        let Some(path) = crate::test_assets::path("Quest.nx") else {
            return;
        };
        let catalog = QuestCatalog::load_from_nx(path).unwrap();
        let heena = catalog.quest(1031).unwrap();
        assert_eq!((heena.start.npc, heena.end_npc, heena.exp), (2101, 2100, 5));
        assert_eq!(
            (heena.start.jobs.as_slice(), heena.start.max_level),
            (&[0][..], Some(10))
        );
        let nina = catalog.quest(1032).unwrap();
        assert_eq!(nina.next_quest, Some(1033));
        let walls = catalog.quest(2324).unwrap();
        assert_eq!(
            (walls.end_npc, walls.end_record, walls.exp),
            (1300004, Some(1), 3300)
        );
        for (id, script, exp) in [(2314, "q2314s", 8300), (2322, "q2322s", 11000)] {
            let investigation = catalog.quest(id).unwrap();
            assert_eq!(
                (
                    investigation.start_script.as_deref(),
                    investigation.end_npc,
                    investigation.end_record,
                    investigation.exp
                ),
                (Some(script), 1300003, Some(1), exp)
            );
        }
        let sen = catalog.quest(1033).unwrap();
        assert_eq!(
            sen.start.quests,
            [QuestPrerequisite {
                quest_id: 1032,
                state: QuestState::Completed
            }]
        );
        let rain: Vec<i32> = (1009..=1015)
            .map(|id| catalog.quest(id).unwrap().exp)
            .collect();
        assert_eq!(rain, [2, 4, 6, 10, 10, 10, 20]);
        assert_eq!(catalog.quest(1036).unwrap().exp, 40);
        // Roger's Apple runs through its scripts and needs every apple eaten.
        let roger = catalog.quest(1021).unwrap();
        assert_eq!(roger.start_script.as_deref(), Some("q1021s"));
        assert_eq!(roger.end_script.as_deref(), Some("q1021e"));
        assert_eq!(roger.end_items, [(2010007, 0)]);
        // Todd's hunt: Jr. Sentinels and their shellpieces.
        let todd = catalog.quest(1035).unwrap();
        assert_eq!(todd.end_mobs, [(9300018, 1)]);
        assert_eq!(todd.end_items, [(4031802, 1)]);
        // One of five beginner hats, and the shellpiece is taken.
        let hats = &todd.end_rewards[..5];
        assert!(hats.iter().all(|h| h.prop == Some(1) && h.applies_to(0, 0)));
        assert!(!hats[0].applies_to(100, 0));
        assert_eq!(
            todd.end_rewards[5],
            QuestItem {
                item_id: 4031802,
                count: -1,
                job: 0,
                gender: 2,
                prop: None,
                period: None
            }
        );
        // Kyrin's Brawler test takes every crystal: an item without a count.
        let brawler = catalog.quest(2191).unwrap();
        assert_eq!(brawler.end_items, [(4031856, 15)]);
        let crystals = &brawler.end_rewards[0];
        assert_eq!((crystals.item_id, crystals.count), (4031856, 0));
        // Ninja Ambush for either Night Lord or Shadower; 0 is in the job list.
        let ambush = catalog.quest(6141).unwrap();
        assert_eq!(
            ambush.end_skills,
            [
                QuestSkill {
                    skill_id: 4221004,
                    level: 0,
                    master_level: 10,
                    jobs: vec![422, 0]
                },
                QuestSkill {
                    skill_id: 4121004,
                    level: 0,
                    master_level: 10,
                    jobs: vec![412, 0]
                }
            ]
        );
        assert_eq!(catalog.quest(6800).unwrap().end_skills[0].level, 1);
        // Bahamut's master level only (`onlyMasterLevel`).
        let bahamut = &catalog.quest(6121).unwrap().end_skills[0];
        assert_eq!(
            (bahamut.skill_id, bahamut.level, bahamut.master_level),
            (2321003, 0, 15)
        );
    }
}
