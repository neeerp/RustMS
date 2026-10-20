//! Server content that never shipped in the client data: script dialogue,
//! mob and reactor drop tables, and NPC shops.
//!
//! Like the NX files, this content is Nexon's and stays out of the
//! repository. It is imported into an ignored content pack
//! (`assets/server-content/`, see `import-server-content`). Behavior is code;
//! only text and data come from the pack. Every text key has a committed
//! placeholder (`placeholder_text.toml`), so the server runs, and its tests
//! pass, without the pack.

use crate::GameDataError;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

const PLACEHOLDER_TEXT: &str = include_str!("../placeholder_text.toml");
const REWARD_OVERRIDES: &str = include_str!("../reward_overrides.toml");

/// Text keys used by server scripts. `placeholder_text.toml` lists them all.
pub mod keys {
    /// `glTutoMsg0`: the instruction balloon at Maple Island's one-way exits.
    pub const LEAVE_AREA_WARNING: &str = "glTutoMsg0.message";

    /// Roger's Apple (quest 1021): `q1021s` pages, then `q1021e` pages.
    pub const ROGER_TITLE_MALE: &str = "roger.title.male";
    pub const ROGER_TITLE_FEMALE: &str = "roger.title.female";
    pub const ROGER_START: [&str; 5] = [
        "roger.start.0",
        "roger.start.1",
        "roger.start.2",
        "roger.start.3",
        "roger.start.4",
    ];
    pub const ROGER_INVENTORY_FULL: &str = "roger.inventory_full";
    pub const ROGER_NOT_RECOVERED: &str = "roger.end.not_recovered";
    pub const ROGER_END: [&str; 3] = ["roger.end.0", "roger.end.1", "roger.end.2"];
    pub const ROGER_END_INVENTORY_FULL: &str = "roger.end.inventory_full";

    /// Maple Administrator's offer to skip the tutorial (`tutorialSkip`).
    pub const SKIP_ASK: &str = "skip.ask";
    pub const SKIP_DECLINE: &str = "skip.decline";

    /// Shanks's ship from Southperry to Victoria Island (`begin7`): the
    /// offer, then the letter's pages or the fare's.
    pub const SHANKS_ASK: &str = "shanks.ask";
    pub const SHANKS_DECLINE: &str = "shanks.decline";
    pub const SHANKS_LETTER: [&str; 2] = ["shanks.letter.0", "shanks.letter.1"];
    pub const SHANKS_FARE: &str = "shanks.fare";
    pub const SHANKS_TOO_WEAK: &str = "shanks.too_weak";
    pub const SHANKS_NO_MESOS: &str = "shanks.no_mesos";
    pub const SHANKS_PAID: &str = "shanks.paid";

    /// Heracle, who founds guilds in Orbis. No licensed script has his
    /// text, so these keep their placeholders.
    pub const HERACLE_ASK: &str = "heracle.ask";
    pub const HERACLE_DECLINE: &str = "heracle.decline";
    pub const HERACLE_IN_GUILD: &str = "heracle.in_guild";
    pub const HERACLE_TOO_WEAK: &str = "heracle.too_weak";
    pub const HERACLE_NO_MESOS: &str = "heracle.no_mesos";

    /// The continent ships' station staff: the booths that sell tickets
    /// (Joel, Agatha), the ushers that take them aboard (Cherry, Rini), and
    /// the crew that lets passengers off before take-off (Purin, Erin). No
    /// licensed script sells tickets or takes them, so those keep their
    /// placeholders.
    pub const SHIPS_SELL_ASK: &str = "ships.sell.ask";
    pub const SHIPS_SELL_DECLINE: &str = "ships.sell.decline";
    pub const SHIPS_SELL_SHORT: &str = "ships.sell.short";
    pub const SHIPS_SELL_MENU: &str = "ships.sell.menu";
    pub const SHIPS_SELL_CHOICE: &str = "ships.sell.choice";
    pub const SHIPS_SELL_FULL: &str = "ships.sell.full";
    pub const SHIPS_BOARD_ASK: &str = "ships.board.ask";
    pub const SHIPS_BOARD_DECLINE: &str = "ships.board.decline";
    pub const SHIPS_BOARD_NO_TICKET: &str = "ships.board.no_ticket";
    pub const SHIPS_BOARD_CLOSING: &str = "ships.board.closing";
    pub const SHIPS_BOARD_FULL: &str = "ships.board.full";
    pub const SHIPS_LEAVE_ASK: &str = "ships.leave.ask";
    pub const SHIPS_LEAVE_DECLINE: &str = "ships.leave.decline";

    /// Isa, who shows the way to Orbis Station's platforms.
    pub const ISA_MENU: &str = "isa.menu";
    pub const ISA_PLATFORM: &str = "isa.platform";
    pub const ISA_ASK: &str = "isa.ask";
    pub const ISA_DECLINE: &str = "isa.decline";

    /// Natalie, Henesys's hair salon: a menu of a cut or a dye, then for
    /// each the choice of styles, the line after, and the line to a player
    /// without the coupon.
    pub const NATALIE_MENU: &str = "natalie.menu";
    pub const NATALIE_CUT: &str = "natalie.cut";
    pub const NATALIE_DYE: &str = "natalie.dye";
    pub const NATALIE_CUT_PAGES: [&str; 3] = [
        "natalie.cut.ask",
        "natalie.cut.done",
        "natalie.cut.no_coupon",
    ];
    pub const NATALIE_DYE_PAGES: [&str; 3] = [
        "natalie.dye.ask",
        "natalie.dye.done",
        "natalie.dye.no_coupon",
    ];

    /// Robin's beginner Q&A (`begin5`): a menu, then each topic's pages.
    /// The placeholders decide how many topics and pages there are.
    pub const ROBIN_MENU: &str = "robin.menu";

    pub fn robin_choice(topic: usize) -> String {
        format!("robin.choice.{topic}")
    }

    pub fn robin_answer(topic: usize, page: usize) -> String {
        format!("robin.answer.{topic}.{page}")
    }

    /// A first-job instructor's lines, by job (`warrior`, `magician`,
    /// `bowman`, `thief`, `pirate`): `intro`, `ask`, `accept`, `after.0`, ... (the
    /// placeholders decide how many), `too_weak`, and `other` for everyone
    /// past beginner.
    pub fn first_job(job: &str, line: &str) -> String {
        format!("first_job.{job}.{line}")
    }

    pub const FIRST_JOB_INVENTORY_FULL: &str = "first_job.inventory_full";

    /// The Explorer second job. By first job (`warrior`, ...): the
    /// instructor's `letter`, `go` (to the examiner) and `choose` (a path,
    /// with the proof), the examiner's `examiner`, `examiner.ask`,
    /// `examiner.decline` and `examiner.other`, and the proctor's
    /// `proctor.pass` and `proctor.ask`. By second job (`110`, ...): its
    /// `name`, `about`, `ask` and `accept`. Kyrin has no letter or
    /// examiner: `pirate.quest` (take a test quest first), `pirate.test`
    /// (enter its test), the proctor's two lines, and no `name` for 510 and
    /// 520.
    pub fn second_job(job: &str, line: &str) -> String {
        format!("second_job.{job}.{line}")
    }

    pub const SECOND_JOB_INVENTORY_FULL: &str = "second_job.inventory_full";

    /// The Explorer third job. By first job (`warrior`, ...): the chief's
    /// `chief.ask`, `chief.decline`, `chief.go`, `chief.remind`,
    /// `chief.strength` (the Necklace of Strength handed in, the Holy Stone
    /// next), `chief.wisdom`, `chief.sp` (too much SP left), `chief.advance`
    /// and `chief.other`, and the instructor's `instructor.assign`,
    /// `instructor.remind` and `instructor.reward`. By third job (`111`,
    /// ...): its `accept`.
    pub fn third_job(job: &str, line: &str) -> String {
        format!("third_job.{job}.{line}")
    }

    /// The Explorer fourth job. By first job (`warrior`, ...): the Leafre
    /// instructor's `ask`, `decline`, `trials` (level 120 and the line's
    /// last quest first), `done` (already a fourth job) and `other`. By
    /// fourth job (`112`, ...): its `accept`.
    pub fn fourth_job(job: &str, line: &str) -> String {
        format!("fourth_job.{job}.{line}")
    }

    pub const FOURTH_JOB_INVENTORY_FULL: &str = "fourth_job.inventory_full";

    /// The Door of Dimension, the Sparkling Crystal back out of the clone's
    /// room, and items that don't fit.
    pub const THIRD_JOB_DOOR_ASK: &str = "third_job.door.ask";
    pub const THIRD_JOB_DOOR_BUSY: &str = "third_job.door.busy";
    pub const THIRD_JOB_DOOR_OTHER: &str = "third_job.door.other";
    pub const THIRD_JOB_CRYSTAL_ASK: &str = "third_job.crystal.ask";
    pub const THIRD_JOB_INVENTORY_FULL: &str = "third_job.inventory_full";

    /// The Holy Stone's quiz: its lines, then questions `0`, `1`, ... (the
    /// placeholders decide how many), each with one right answer and its
    /// wrong ones.
    pub const HOLY_STONE_OTHER: &str = "holy_stone.other";
    pub const HOLY_STONE_DONE: &str = "holy_stone.done";
    pub const HOLY_STONE_NO_CRYSTAL: &str = "holy_stone.no_crystal";
    pub const HOLY_STONE_ETC_FULL: &str = "holy_stone.etc_full";
    pub const HOLY_STONE_ASK: &str = "holy_stone.ask";
    pub const HOLY_STONE_PASS: &str = "holy_stone.pass";
    pub const HOLY_STONE_FAIL: &str = "holy_stone.fail";

    pub fn holy_stone_question(question: usize) -> String {
        format!("holy_stone.q.{question}")
    }

    pub fn holy_stone_right(question: usize) -> String {
        format!("holy_stone.q.{question}.right")
    }

    pub fn holy_stone_wrong(question: usize, answer: usize) -> String {
        format!("holy_stone.q.{question}.wrong.{answer}")
    }

    /// Spiruna refining Dark Crystals: for those who haven't done her
    /// quest, the offer, and the result or what is short.
    pub const SPIRUNA_OTHER: &str = "spiruna.other";
    pub const SPIRUNA_ASK: &str = "spiruna.ask";
    pub const SPIRUNA_DONE: &str = "spiruna.done";
    pub const SPIRUNA_SHORT: &str = "spiruna.short";

    /// Mar the Fairy and the Water of Life: the dolls to pick from, then for
    /// the one picked (`{name}`, revived for `{days}`) the offer and the
    /// pet awake; or no doll at all.
    pub const WATER_OF_LIFE_PICK: &str = "water_of_life.pick";
    pub const WATER_OF_LIFE_ASK: &str = "water_of_life.ask";
    pub const WATER_OF_LIFE_DONE: &str = "water_of_life.done";
    pub const WATER_OF_LIFE_NO_DOLL: &str = "water_of_life.no_doll";

    /// Every key: one per placeholder.
    pub fn all() -> Vec<String> {
        super::parse_text(super::PLACEHOLDER_TEXT, "placeholder_text.toml")
            .expect("placeholder_text.toml parses")
            .into_keys()
            .collect()
    }
}

/// Script text, resolved from the content pack with placeholder fallbacks.
#[derive(Debug)]
pub struct ServerText {
    text: BTreeMap<String, String>,
    from_pack: usize,
    unknown_in_pack: Vec<String>,
}

impl ServerText {
    /// Load placeholders, then override them with `<pack>/text.toml` if the
    /// pack exists.
    pub fn load(pack: Option<&Path>) -> Result<Self, GameDataError> {
        let mut text = parse_text(PLACEHOLDER_TEXT, "placeholder_text.toml")?;
        let mut from_pack = 0;
        let mut unknown_in_pack = Vec::new();
        if let Some(path) = pack.map(|p| p.join("text.toml")).filter(|p| p.exists()) {
            let source = std::fs::read_to_string(&path)?;
            for (key, value) in parse_text(&source, &path.display().to_string())? {
                match text.get_mut(&key) {
                    Some(slot) => {
                        *slot = value;
                        from_pack += 1;
                    }
                    None => unknown_in_pack.push(key),
                }
            }
        }
        Ok(Self {
            text,
            from_pack,
            unknown_in_pack,
        })
    }

    /// The text for `key`, with `{name}` placeholders filled from `vars`.
    pub fn get(&self, key: &str, vars: &[(&str, &str)]) -> String {
        let mut text = self
            .text
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string());
        for (name, value) in vars {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }

    /// How many keys came from the content pack rather than placeholders.
    pub fn from_pack(&self) -> usize {
        self.from_pack
    }

    pub fn placeholders(&self) -> usize {
        self.text.len() - self.from_pack
    }

    /// Keys in the pack that no script uses (e.g. a newer pack).
    pub fn unknown_in_pack(&self) -> &[String] {
        &self.unknown_in_pack
    }

    pub fn contains(&self, key: &str) -> bool {
        self.text.contains_key(key)
    }
}

fn parse_text(source: &str, name: &str) -> Result<BTreeMap<String, String>, GameDataError> {
    toml::from_str(source).map_err(|e| GameDataError::InvalidData(format!("{name}: {e}")))
}

/// One possible drop, in the server data's `Reward.img` shape.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RewardEntry {
    #[serde(default)]
    pub item: Option<i32>,
    #[serde(default)]
    pub money: Option<i32>,
    pub prob: f64,
    #[serde(default)]
    pub min: Option<i32>,
    #[serde(default)]
    pub max: Option<i32>,
    /// Only drops while this quest is started (our v83 overrides).
    #[serde(default)]
    pub quest: Option<i16>,
    /// Cash-shop "premium" drops, which v83 did not have.
    #[serde(default)]
    pub premium: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct Overrides {
    #[serde(default)]
    mobs: BTreeMap<String, MobOverride>,
    #[serde(default)]
    reactors: BTreeMap<String, ReactorOverride>,
}

#[derive(Debug, Deserialize)]
struct MobOverride {
    /// Drops that replace whatever the pack says for this mob.
    replace: Vec<RewardEntry>,
}

#[derive(Debug, Deserialize)]
struct ReactorOverride {
    /// Use the pack's drops for this other reactor (the same box).
    #[serde(default)]
    like: Option<i32>,
    /// Quest items, by item id: they drop only while their quest is started,
    /// and always drop then, even without the pack.
    #[serde(default)]
    quest_items: BTreeMap<String, i16>,
}

/// Mob and reactor drops: the pack's `rewards.json` (both kinds share its id
/// space) plus committed v83 corrections.
#[derive(Debug, Default)]
pub struct RewardTable {
    pack: BTreeMap<i32, Vec<RewardEntry>>,
    mobs: BTreeMap<i32, Vec<RewardEntry>>,
    reactors: BTreeMap<i32, Vec<RewardEntry>>,
}

impl RewardTable {
    pub fn load(pack: Option<&Path>) -> Result<Self, GameDataError> {
        let mut table = Self::default();
        if let Some(path) = pack.map(|p| p.join("rewards.json")).filter(|p| p.exists()) {
            let file = std::fs::File::open(&path)?;
            let raw: BTreeMap<String, Vec<RewardEntry>> =
                serde_json::from_reader(std::io::BufReader::new(file))
                    .map_err(|e| GameDataError::InvalidData(format!("{}: {e}", path.display())))?;
            for (id, entries) in raw {
                // v83 had no premium drops.
                let entries: Vec<_> = entries
                    .into_iter()
                    .filter(|e| e.premium.is_none())
                    .collect();
                table.pack.insert(parse_id(&id, "id")?, entries);
            }
        }
        let overrides: Overrides = toml::from_str(REWARD_OVERRIDES)
            .map_err(|e| GameDataError::InvalidData(format!("reward_overrides.toml: {e}")))?;
        for (mob, entry) in overrides.mobs {
            table
                .mobs
                .insert(parse_id(&mob, "override mob id")?, entry.replace);
        }
        for (reactor, entry) in overrides.reactors {
            let id = parse_id(&reactor, "override reactor id")?;
            let mut drops = table
                .pack
                .get(&entry.like.unwrap_or(id))
                .cloned()
                .unwrap_or_default();
            for (item, quest) in entry.quest_items {
                let item = parse_id(&item, "override quest item id")?;
                let mut found = false;
                for drop in drops.iter_mut().filter(|d| d.item == Some(item)) {
                    drop.quest = Some(quest);
                    drop.prob = 1.0;
                    found = true;
                }
                if !found {
                    drops.push(RewardEntry {
                        item: Some(item),
                        money: None,
                        prob: 1.0,
                        min: None,
                        max: None,
                        quest: Some(quest),
                        premium: None,
                    });
                }
            }
            table.reactors.insert(id, drops);
        }
        Ok(table)
    }

    pub fn drops(&self, mob_id: i32) -> &[RewardEntry] {
        self.mobs
            .get(&mob_id)
            .or_else(|| self.pack.get(&mob_id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn reactor_drops(&self, reactor_id: i32) -> &[RewardEntry] {
        self.reactors
            .get(&reactor_id)
            .or_else(|| self.pack.get(&reactor_id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// How many mobs and reactors have drops from the content pack.
    pub fn from_pack(&self) -> usize {
        self.pack.len()
    }
}

fn parse_id(id: &str, what: &str) -> Result<i32, GameDataError> {
    id.parse()
        .map_err(|_| GameDataError::InvalidData(format!("bad {what} {id}")))
}

/// One thing an NPC shop lists, in the server data's `NpcShop.img` shape.
/// A throwing star or bullet without a price is only recharged there.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ShopEntry {
    pub item: i32,
    #[serde(default)]
    pub price: i32,
}

/// NPC shops: the pack's `shops.json`, by NPC id. Without the pack there are
/// no shops.
#[derive(Debug, Default)]
pub struct ShopTable {
    npcs: BTreeMap<i32, Vec<ShopEntry>>,
}

impl ShopTable {
    pub fn load(pack: Option<&Path>) -> Result<Self, GameDataError> {
        let mut npcs = BTreeMap::new();
        if let Some(path) = pack.map(|p| p.join("shops.json")).filter(|p| p.exists()) {
            let file = std::fs::File::open(&path)?;
            let raw: BTreeMap<String, Vec<ShopEntry>> =
                serde_json::from_reader(std::io::BufReader::new(file))
                    .map_err(|e| GameDataError::InvalidData(format!("{}: {e}", path.display())))?;
            for (npc, entries) in raw {
                let id = npc
                    .parse()
                    .map_err(|_| GameDataError::InvalidData(format!("bad shop npc id {npc}")))?;
                npcs.insert(id, entries);
            }
        }
        Ok(Self { npcs })
    }

    /// What `npc_id` sells, in the order it lists it, if it keeps a shop.
    pub fn shop(&self, npc_id: i32) -> Option<&[ShopEntry]> {
        self.npcs.get(&npc_id).map(Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.npcs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.npcs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_named_key_has_a_placeholder() {
        let text = ServerText::load(None).unwrap();
        let named = [
            keys::LEAVE_AREA_WARNING,
            keys::ROGER_TITLE_MALE,
            keys::ROGER_TITLE_FEMALE,
            keys::ROGER_INVENTORY_FULL,
            keys::ROGER_NOT_RECOVERED,
            keys::ROGER_END_INVENTORY_FULL,
            keys::SKIP_ASK,
            keys::SKIP_DECLINE,
            keys::SHANKS_ASK,
            keys::SHANKS_DECLINE,
            keys::SHANKS_FARE,
            keys::SHANKS_TOO_WEAK,
            keys::SHANKS_NO_MESOS,
            keys::SHANKS_PAID,
            keys::HERACLE_ASK,
            keys::HERACLE_DECLINE,
            keys::HERACLE_IN_GUILD,
            keys::HERACLE_TOO_WEAK,
            keys::HERACLE_NO_MESOS,
            keys::SHIPS_SELL_ASK,
            keys::SHIPS_SELL_DECLINE,
            keys::SHIPS_SELL_SHORT,
            keys::SHIPS_SELL_MENU,
            keys::SHIPS_SELL_CHOICE,
            keys::SHIPS_SELL_FULL,
            keys::SHIPS_BOARD_ASK,
            keys::SHIPS_BOARD_DECLINE,
            keys::SHIPS_BOARD_NO_TICKET,
            keys::SHIPS_BOARD_CLOSING,
            keys::SHIPS_BOARD_FULL,
            keys::SHIPS_LEAVE_ASK,
            keys::SHIPS_LEAVE_DECLINE,
            keys::ISA_MENU,
            keys::ISA_PLATFORM,
            keys::ISA_ASK,
            keys::ISA_DECLINE,
            keys::NATALIE_MENU,
            keys::NATALIE_CUT,
            keys::NATALIE_DYE,
            keys::ROBIN_MENU,
            keys::FIRST_JOB_INVENTORY_FULL,
            keys::SECOND_JOB_INVENTORY_FULL,
            keys::THIRD_JOB_DOOR_ASK,
            keys::THIRD_JOB_DOOR_BUSY,
            keys::THIRD_JOB_DOOR_OTHER,
            keys::THIRD_JOB_CRYSTAL_ASK,
            keys::THIRD_JOB_INVENTORY_FULL,
            keys::HOLY_STONE_OTHER,
            keys::HOLY_STONE_DONE,
            keys::HOLY_STONE_NO_CRYSTAL,
            keys::HOLY_STONE_ETC_FULL,
            keys::HOLY_STONE_ASK,
            keys::HOLY_STONE_PASS,
            keys::HOLY_STONE_FAIL,
            keys::SPIRUNA_OTHER,
            keys::SPIRUNA_ASK,
            keys::SPIRUNA_DONE,
            keys::SPIRUNA_SHORT,
        ];
        for key in named
            .iter()
            .chain(&keys::ROGER_START)
            .chain(&keys::ROGER_END)
            .chain(&keys::SHANKS_LETTER)
            .chain(&keys::NATALIE_CUT_PAGES)
            .chain(&keys::NATALIE_DYE_PAGES)
        {
            assert!(text.contains(key), "{key} has no placeholder");
        }
        assert!(text.contains(&keys::robin_answer(0, 0)));
        assert!(text.contains(&keys::first_job("thief", "after.4")));
        assert!(text.contains(&keys::second_job("thief", "proctor.ask")));
        assert!(text.contains(&keys::second_job("420", "accept")));
        assert!(text.contains(&keys::first_job("pirate", "after.4")));
        assert!(text.contains(&keys::second_job("pirate", "test")));
        assert!(text.contains(&keys::second_job("520", "accept")));
        assert!(text.contains(&keys::third_job("pirate", "instructor.reward")));
        assert!(text.contains(&keys::third_job("521", "accept")));
        assert!(text.contains(&keys::fourth_job("pirate", "trials")));
        assert!(text.contains(&keys::fourth_job("522", "accept")));
        assert!(text.contains(keys::FOURTH_JOB_INVENTORY_FULL));
        assert!(text.contains(&keys::holy_stone_wrong(4, 3)));
        assert!(text
            .get(keys::LEAVE_AREA_WARNING, &[])
            .starts_with("[placeholder]"));
        assert_eq!(text.placeholders(), keys::all().len());
        assert_eq!(text.from_pack(), 0);
    }

    #[test]
    fn the_pack_overrides_placeholders_and_reports_unknown_keys() {
        let dir = std::env::temp_dir().join(format!("rustms-pack-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("text.toml"),
            "\"glTutoMsg0.message\" = \"Real {who}\"\n\"stale.key\" = \"x\"\n",
        )
        .unwrap();
        let text = ServerText::load(Some(&dir)).unwrap();
        assert_eq!(
            text.get(keys::LEAVE_AREA_WARNING, &[("who", "text")]),
            "Real text"
        );
        assert_eq!(
            (text.from_pack(), text.placeholders()),
            (1, keys::all().len() - 1)
        );
        assert_eq!(text.unknown_in_pack(), ["stale.key"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn quest_drops_work_without_the_pack() {
        let table = RewardTable::load(None).unwrap();
        assert_eq!(table.from_pack(), 0);
        // Todd's Hunting Method (1035) needs this shellpiece from the
        // Tutorial Jr. Sentinel; the v95 data names a different item.
        assert_eq!(
            table.drops(9300018),
            [RewardEntry {
                item: Some(4031802),
                money: None,
                prob: 1.0,
                min: None,
                max: None,
                quest: Some(1035),
                premium: None,
            }]
        );
        assert!(table.drops(100100).is_empty());
        // Kyrin's test Octopirates drop the crystals her quests check for.
        let crystal = |item, quest| RewardEntry {
            prob: 0.7,
            ..quest_item(item, quest)
        };
        assert_eq!(table.drops(9001006), [crystal(4031856, 2191)]);
        assert_eq!(table.drops(9001005), [crystal(4031857, 2192)]);
    }

    fn quest_item(item: i32, quest: i16) -> RewardEntry {
        RewardEntry {
            item: Some(item),
            money: None,
            prob: 1.0,
            min: None,
            max: None,
            quest: Some(quest),
            premium: None,
        }
    }

    #[test]
    fn amherst_boxes_drop_pios_items_without_the_pack() {
        let table = RewardTable::load(None).unwrap();
        assert_eq!(
            table.reactor_drops(2001),
            [quest_item(4031161, 1008), quest_item(4031162, 1008)]
        );
        assert!(table.reactor_drops(9999998).is_empty());
    }

    #[test]
    fn amherst_boxes_borrow_the_packs_box_drops() {
        let dir = std::env::temp_dir().join(format!("rustms-rewards-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("rewards.json"),
            r#"{"2000": [{"money": 10, "prob": 0.7}, {"item": 4031161, "prob": 0.5}],
                "100100": [{"item": 4000019, "prob": 0.6}]}"#,
        )
        .unwrap();
        let table = RewardTable::load(Some(&dir)).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        assert_eq!(table.from_pack(), 2);
        let drops = table.reactor_drops(2001);
        assert_eq!(drops[0].money, Some(10));
        assert_eq!(
            drops[1..],
            [quest_item(4031161, 1008), quest_item(4031162, 1008)]
        );
        assert_eq!(table.drops(100100)[0].item, Some(4000019));
    }

    #[test]
    fn shops_come_from_the_pack() {
        assert!(ShopTable::load(None).unwrap().is_empty());

        let dir = std::env::temp_dir().join(format!("rustms-shops-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("shops.json"),
            r#"{"1011100": [{"item": 2000000, "price": 50, "stock": 100000},
                            {"item": 2070001, "stock": 100000, "unitPrice": 0.4}]}"#,
        )
        .unwrap();
        let shops = ShopTable::load(Some(&dir)).unwrap();
        assert_eq!(
            shops.shop(1011100).unwrap(),
            [
                ShopEntry {
                    item: 2000000,
                    price: 50
                },
                ShopEntry {
                    item: 2070001,
                    price: 0
                }
            ]
        );
        assert!(shops.shop(11000).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
