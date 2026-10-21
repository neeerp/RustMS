//! Item templates from `Item.nx` (use, setup, etc, cash items) and
//! `Character.nx` (equipment). Looked up on demand: both files are memory
//! mapped and each lookup is a few binary searches.

use crate::{GameDataError, NxDataFile};
use game::buff::BuffStat;
use game::pet::PetCommand;
use game::scroll::{ScrollInfo, ScrollKind};
use game::EquipStats;
use std::path::Path;

/// Stack size when `info/slotMax` is absent (v83: 1 for equipment, else 100).
pub const DEFAULT_SLOT_MAX: i16 = 100;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemInfo {
    pub item_id: i32,
    pub slot_max: i16,
    /// Quest items (`info/quest`).
    pub quest: bool,
    /// One-of-a-kind items (`info/only`).
    pub only: bool,
    /// Items that can't change hands (`info/tradeBlock`).
    pub trade_block: bool,
    /// Equipment that can't change hands once worn (`info/equipTradeBlock`).
    pub equip_trade_block: bool,
    /// Items nobody may pick up (`info/pickUpBlock`): the invisible ones
    /// party quests drop.
    pub pickup_block: bool,
    /// Items that move only between an account's characters, through
    /// storage (`info/accountSharable`).
    pub account_sharable: bool,
    /// Party quest items (`info/pquest`).
    pub pquest: bool,
    /// Items gone once their owner logs out (`info/expireOnLogout`).
    pub expire_on_logout: bool,
    /// Time-limited items no sandglass extends (`info/notExtend`).
    pub not_extend: bool,
    /// What a meso sack holds (`info/meso`).
    pub meso: i32,
    pub price: i32,
    /// What one throwing star or bullet costs to recharge (`unitPrice`).
    pub unit_price: f64,
    /// What a star, arrow, or bullet adds to the attack of a shot that
    /// throws it (`incPAD`).
    pub pad: i32,
    /// What a portable chair heals every 10 seconds (`recoveryHP`/`recoveryMP`).
    pub recovery_hp: i32,
    pub recovery_mp: i32,
    /// The highest level that may use it (`info/maxLevel`), 0 for any.
    pub max_level: i32,
    /// `spec` of consumable items.
    pub spec: Option<ConsumeSpec>,
    /// What a skill or mastery book teaches (`info/skill`).
    pub book: Option<BookInfo>,
    /// What an equipment scroll (204xxxx) does.
    pub scroll: Option<ScrollInfo>,
    /// What a summoning sack (210xxxx) brings out (`mob`): each mob and its
    /// percent chance, rolled for each.
    pub sack: Vec<(i32, i32)>,
    /// What a catch item (227xxxx) catches.
    pub catch: Option<CatchInfo>,
    pub equip: Option<EquipInfo>,
    /// A pet's commands (`interact`), by the index the client sends.
    pub pet_commands: Vec<PetCommand>,
    /// The fullness a pet loses each time it grows hungry (`info/hungry`).
    pub pet_hunger: i32,
    /// The days a pet lives once bought, or once a Water of Life revives
    /// it (`info/life`); 0 for a pet that never dies (`info/permanent`).
    pub pet_life: i32,
    /// Whether a pet has a name tag style (`info/nameTag`), which lets it
    /// wear a label ring.
    pub pet_name_tag: bool,
    /// Whether a pet has a chat balloon style (`info/chatBalloon`), which
    /// lets it wear a quote ring.
    pub pet_chat_balloon: bool,
    /// What a 2x EXP (521xxxx) or drop (536xxxx) card multiplies, and when.
    pub rate_card: Option<RateCard>,
    /// The cloud a cash mist item (528xxxx) leaves.
    pub mist: Option<ItemMist>,
    /// What a Magical Sandglass (550xxxx) adds.
    pub sandglass: Option<Sandglass>,
}

/// A Magical Sandglass's `info`: the seconds it adds to a time-limited
/// equip (`addTime`), and how many days from now that may reach
/// (`maxDays`, 0 for any).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sandglass {
    pub add_seconds: i32,
    pub max_days: i32,
}

/// A cash mist item's cloud: its box around the user (`info/lt`, `info/rb`)
/// and how long it lasts (`info/time`, in seconds).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemMist {
    /// Left, top, right, bottom, from where the user stands.
    pub area: (i32, i32, i32, i32),
    pub seconds: i32,
    /// The emotes its user may pull (its `emotion` keys), one bit each by
    /// v83's emote numbers.
    pub faces: u32,
}

impl ItemMist {
    pub fn has_face(&self, emote: i32) -> bool {
        (0..32).contains(&emote) && self.faces & 1 << emote != 0
    }
}

/// v83's emote numbers by name, F1's "hit" being 1 (FUN_00416563).
const EMOTES: [&str; 23] = [
    "blink",
    "hit",
    "smile",
    "troubled",
    "cry",
    "angry",
    "bewildered",
    "stunned",
    "vomit",
    "oops",
    "cheers",
    "chu",
    "wink",
    "pain",
    "glitter",
    "blaze",
    "shine",
    "love",
    "despair",
    "hum",
    "bowing",
    "hot",
    "dam",
];

/// A rate card's multiplier (`info/rate`) and the hours it works in
/// (`info/time`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RateCard {
    pub rate: f64,
    /// Each `"DAY:HH-HH"` entry as (day, first hour, end hour), with Monday
    /// as day 0 and the end hour left out. `HOL` (holiday) entries are
    /// dropped: which days count is unknown.
    pub windows: Vec<(u8, u8, u8)>,
}

impl RateCard {
    /// Whether it works at `hour` of `day` (Monday is 0).
    pub fn active(&self, day: u8, hour: u8) -> bool {
        self.windows
            .iter()
            .any(|&(on, from, to)| on == day && from <= hour && hour < to)
    }
}

/// One `info/time` entry, such as `"SAT:19-23"`.
fn rate_window(entry: &str) -> Option<(u8, u8, u8)> {
    const DAYS: [&str; 7] = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];
    let (day, hours) = entry.split_once(':')?;
    let (from, to) = hours.split_once('-')?;
    let day = DAYS.iter().position(|d| *d == day)?;
    Some((day as u8, from.parse().ok()?, to.parse().ok()?))
}

/// A skill book (228xxxx) or mastery book (229xxxx).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BookInfo {
    /// The skills it can raise, one per job (`info/skill`).
    pub skills: Vec<i32>,
    /// The master level it grants (`masterLevel`).
    pub master_level: i32,
    /// The skill level the reader needs first (`reqSkillLevel`).
    pub req_skill_level: i32,
    /// Percent chance it works (`success`).
    pub success: i32,
}

/// What a catch item (227xxxx) catches, and what for (`info`). v83 reads
/// only the mob, the area and `create`; the rest is the server's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CatchInfo {
    /// The mob it catches (`mob`).
    pub mob: i32,
    /// The item a catch gives (`create`).
    pub create: i32,
    /// Where it reaches from the player's feet, whichever way they face:
    /// left, top, right, bottom.
    pub area: (i16, i16, i16, i16),
    /// The percent of its HP the mob must be below (`mobHP`); 100, the
    /// default, for any.
    pub mob_hp: i32,
    /// The percent chance a catch works (`bridleProp`, default 100).
    pub prop: f64,
    /// The ms before it may be used again (`useDelay`).
    pub use_delay: i32,
}

/// What using a consumable does (`spec`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConsumeSpec {
    pub hp: i32,
    pub mp: i32,
    /// Percent of max HP/MP.
    pub hp_r: i32,
    pub mp_r: i32,
    /// How long `stats` last, in ms (`time`).
    pub time: i32,
    /// The stats it raises for `time` (`pad`, `pdd`, `mad`, `mdd`, `acc`,
    /// `eva`, `speed`, `jump`, `thaw`, `expBuff`), and the template it
    /// morphs the player into (`morph`).
    pub stats: Vec<(BuffStat, i16)>,
    /// The diseases it cures (`poison`, `darkness`, `weakness`, `seal`,
    /// `curse`).
    pub cures: Vec<BuffStat>,
    /// What a pet food gives (`inc`): fullness for 212xxxx, closeness for
    /// cash food (524xxxx), which also fills the pet up.
    pub inc: i32,
    /// The pets a cash food suits (`0`, `1`, ...).
    pub pets: Vec<i32>,
    /// Where a return scroll (203xxxx) goes (`moveTo`): a map, or
    /// `NO_DESTINATION_MAP` for the map's return map.
    pub move_to: Option<i32>,
    /// Whether a return scroll may leave the player's area (`ignoreContinent`).
    pub ignore_continent: bool,
    /// The quest record an event scroll keeps the way back in (`returnMapQR`).
    pub return_map_qr: i32,
    /// What a Writ of Solomon (237xxxx) adds to the player's gacha EXP
    /// (`exp`).
    pub exp: i32,
    /// How much a mount food (226xxxx) changes a mount's tiredness
    /// (`incFatigue`): negative takes it off.
    pub inc_fatigue: i32,
}

impl ConsumeSpec {
    /// Whether USE_ITEM can apply any of it: HP or MP, a cure, or stats for
    /// a time.
    pub fn has_effect(&self) -> bool {
        self.hp != 0
            || self.mp != 0
            || self.hp_r != 0
            || self.mp_r != 0
            || !self.cures.is_empty()
            || (self.time > 0 && !self.stats.is_empty())
    }
}

/// The `spec` keys of the stats a consumable raises.
const SPEC_STATS: [(&str, BuffStat); 11] = [
    ("pad", BuffStat::Pad),
    ("pdd", BuffStat::Pdd),
    ("mad", BuffStat::Mad),
    ("mdd", BuffStat::Mdd),
    ("acc", BuffStat::Acc),
    ("eva", BuffStat::Eva),
    ("speed", BuffStat::Speed),
    ("jump", BuffStat::Jump),
    ("thaw", BuffStat::Thaw),
    ("expBuff", BuffStat::ExpBuff),
    ("morph", BuffStat::Morph),
];

/// The `spec` keys of the diseases a consumable cures.
const SPEC_CURES: [(&str, BuffStat); 5] = [
    ("poison", BuffStat::Poison),
    ("darkness", BuffStat::Darkness),
    ("weakness", BuffStat::Weakness),
    ("seal", BuffStat::Seal),
    ("curse", BuffStat::Curse),
];

/// An equipment template (`Character.nx/<category>/<id>.img/info`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EquipInfo {
    pub upgrade_slots: u8,
    pub str: i16,
    pub dex: i16,
    pub int: i16,
    pub luk: i16,
    pub hp: i16,
    pub mp: i16,
    pub watk: i16,
    pub matk: i16,
    pub wdef: i16,
    pub mdef: i16,
    pub acc: i16,
    pub avoid: i16,
    pub hands: i16,
    pub speed: i16,
    pub jump: i16,
    pub req_level: i16,
    /// Bit flags of job branches (1 warrior, 2 magician, 4 bowman, 8 thief,
    /// 16 pirate); 0 means anyone and -1 beginners only.
    pub req_job: i16,
    pub req_str: i16,
    pub req_dex: i16,
    pub req_int: i16,
    pub req_luk: i16,
    pub req_fame: i16,
    pub cash: bool,
    /// `info/recovery`: how much faster the wearer recovers on a map whose
    /// own recovery is above 1 (the sauna robes, 1.5).
    pub recovery: Option<f64>,
    /// The `TamingMob.nx` template a mount (190xxxx) rides with
    /// (`tamingMob`); 0 for one that can't be ridden.
    pub taming_mob: i32,
    /// The pets that pet wear (180xxxx) suits: the image's children named
    /// for pet item ids.
    pub pets: Vec<i32>,
}

/// Top-level `Item.nx` directory for a non-equipment item id.
fn item_directory(item_id: i32) -> Option<&'static str> {
    match item_id / 1_000_000 {
        2 => Some("Consume"),
        3 => Some("Install"),
        4 => Some("Etc"),
        5 => Some("Cash"),
        _ => None,
    }
}

/// `Character.nx` directories that hold equipment.
const EQUIP_DIRECTORIES: &[&str] = &[
    "Accessory",
    "Cap",
    "Cape",
    "Coat",
    "Glove",
    "Longcoat",
    "Pants",
    "PetEquip",
    "Ring",
    "Shield",
    "Shoes",
    "TamingMob",
    "Weapon",
];

pub struct ItemData {
    items: NxDataFile,
    equipment: NxDataFile,
}

impl ItemData {
    pub fn load(
        items: impl AsRef<Path>,
        equipment: impl AsRef<Path>,
    ) -> Result<Self, GameDataError> {
        Ok(Self {
            items: NxDataFile::open(items.as_ref())?,
            equipment: NxDataFile::open(equipment.as_ref())?,
        })
    }

    /// Every Monster Book card (`Consume/0238.img`) as `(card, mob)`, in
    /// card order.
    pub fn monster_cards(&self) -> Result<Vec<(i32, i32)>, GameDataError> {
        let nx = &self.items;
        let Some(cards) = path_index(nx, "Consume/0238.img")? else {
            return Ok(Vec::new());
        };
        let mut found = Vec::new();
        for node in nx.child_indices(cards)? {
            let Ok(card) = nx.node_name(node)?.parse::<i32>() else {
                continue;
            };
            let Some(info) = nx.child_by_name(node, "info")? else {
                continue;
            };
            if let Some(mob) = nx.int_child(info, "mob")? {
                found.push((card, mob));
            }
        }
        Ok(found)
    }

    /// Whether `Character.nx` has the hair (3xxxx) or face (2xxxx) `style`.
    pub fn has_style(&self, style: i32) -> Result<bool, GameDataError> {
        let directory = match style / 10_000 {
            2 => "Face",
            3 => "Hair",
            _ => return Ok(false),
        };
        Ok(path_index(&self.equipment, &format!("{directory}/{style:08}.img"))?.is_some())
    }

    /// The template for `item_id`, or `None` if the data has no such item.
    pub fn info(&self, item_id: i32) -> Result<Option<ItemInfo>, GameDataError> {
        if item_id / 1_000_000 == 1 {
            return self.equipment_info(item_id);
        }
        let Some(directory) = item_directory(item_id) else {
            return Ok(None);
        };
        let nx = &self.items;
        // Each pet has an image of its own.
        let pet = game::pet::is_pet(item_id);
        let path = if pet {
            format!("Pet/{item_id:07}.img")
        } else {
            format!("{directory}/{:04}.img/{:08}", item_id / 10_000, item_id)
        };
        let Some(node) = path_index(nx, &path)? else {
            return Ok(None);
        };
        let mut info = ItemInfo {
            item_id,
            slot_max: if pet { 1 } else { DEFAULT_SLOT_MAX },
            ..ItemInfo::default()
        };
        if let Some(meta) = nx.child_by_name(node, "info")? {
            if let Some(max) = nx.int_child(meta, "slotMax")? {
                info.slot_max = i16::try_from(max).unwrap_or(i16::MAX).max(1);
            }
            info.quest = nx.int_child(meta, "quest")? == Some(1);
            info.only = nx.int_child(meta, "only")? == Some(1);
            info.trade_block = nx.int_child(meta, "tradeBlock")? == Some(1);
            info.pickup_block = nx.int_child(meta, "pickUpBlock")? == Some(1);
            info.account_sharable = nx.int_child(meta, "accountSharable")? == Some(1);
            info.pquest = nx.int_child(meta, "pquest")? == Some(1);
            info.expire_on_logout = nx.int_child(meta, "expireOnLogout")? == Some(1);
            info.not_extend = nx.int_child(meta, "notExtend")? == Some(1);
            info.meso = nx.int_child(meta, "meso")?.unwrap_or(0);
            info.price = nx.int_child(meta, "price")?.unwrap_or(0);
            info.unit_price = nx.real_child(meta, "unitPrice")?.unwrap_or(0.0);
            info.pad = nx.int_child(meta, "incPAD")?.unwrap_or(0);
            info.recovery_hp = nx.int_child(meta, "recoveryHP")?.unwrap_or(0);
            info.recovery_mp = nx.int_child(meta, "recoveryMP")?.unwrap_or(0);
            info.max_level = nx.int_child(meta, "maxLevel")?.unwrap_or(0);
            if pet {
                info.pet_hunger = nx.int_child(meta, "hungry")?.unwrap_or(1);
                if nx.int_child(meta, "permanent")? != Some(1) {
                    info.pet_life = nx.int_child(meta, "life")?.unwrap_or(0);
                }
                info.pet_name_tag = nx.int_child(meta, "nameTag")?.unwrap_or(0) != 0;
                info.pet_chat_balloon = nx.int_child(meta, "chatBalloon")?.unwrap_or(0) != 0;
            }
            if game::pet::is_water_of_life(item_id) {
                info.pet_life = nx.int_child(meta, "life")?.unwrap_or(0);
            }
            if let Some(skills) = nx.child_by_name(meta, "skill")? {
                let mut book = BookInfo {
                    master_level: nx.int_child(meta, "masterLevel")?.unwrap_or(0),
                    req_skill_level: nx.int_child(meta, "reqSkillLevel")?.unwrap_or(0),
                    success: nx.int_child(meta, "success")?.unwrap_or(100),
                    ..BookInfo::default()
                };
                while let Some(skill) = nx.int_child(skills, &book.skills.len().to_string())? {
                    book.skills.push(skill);
                }
                info.book = Some(book);
            }
            if item_id / 10_000 == 204 {
                info.scroll = scroll_info(nx, node, meta)?;
            }
            if matches!(item_id / 10_000, 521 | 536) {
                let mut windows = Vec::new();
                if let Some(time) = nx.child_by_name(meta, "time")? {
                    for entry in nx.child_indices(time)? {
                        let name = nx.node_name(entry)?;
                        if let Some(window) = nx
                            .string_child(time, &name)?
                            .as_deref()
                            .and_then(rate_window)
                        {
                            windows.push(window);
                        }
                    }
                }
                info.rate_card = Some(RateCard {
                    rate: nx.real_child(meta, "rate")?.unwrap_or(1.0),
                    windows,
                });
            }
            if item_id / 10_000 == 528 {
                if let (Some((left, top)), Some((right, bottom))) =
                    (nx.vector_child(meta, "lt")?, nx.vector_child(meta, "rb")?)
                {
                    let mut faces = 0;
                    if let Some(emotion) = nx.child_by_name(node, "emotion")? {
                        for face in nx.child_indices(emotion)? {
                            let name = nx.node_name(face)?;
                            if let Some(emote) = EMOTES.iter().position(|e| *e == name) {
                                faces |= 1 << emote;
                            }
                        }
                    }
                    info.mist = Some(ItemMist {
                        area: (left, top, right, bottom),
                        seconds: nx.int_child(meta, "time")?.unwrap_or(0),
                        faces,
                    });
                }
            }
            // One that adds nothing is no sandglass.
            if let (550, Some(add_seconds @ 1..)) =
                (item_id / 10_000, nx.int_child(meta, "addTime")?)
            {
                info.sandglass = Some(Sandglass {
                    add_seconds,
                    max_days: nx.int_child(meta, "maxDays")?.unwrap_or(0),
                });
            }
        }
        if let Some(spec) = nx.child_by_name(node, "spec")? {
            let mut pets = Vec::new();
            if item_id / 10_000 == 524 {
                while let Some(pet) = nx.int_child(spec, &pets.len().to_string())? {
                    pets.push(pet);
                }
            }
            info.spec = Some(ConsumeSpec {
                hp: nx.int_child(spec, "hp")?.unwrap_or(0),
                mp: nx.int_child(spec, "mp")?.unwrap_or(0),
                hp_r: nx.int_child(spec, "hpR")?.unwrap_or(0),
                mp_r: nx.int_child(spec, "mpR")?.unwrap_or(0),
                time: nx.int_child(spec, "time")?.unwrap_or(0),
                stats: SPEC_STATS
                    .iter()
                    .filter_map(|&(key, stat)| match nx.int_child(spec, key) {
                        Ok(Some(0)) | Ok(None) => None,
                        Ok(Some(value)) => Some(Ok((stat, value as i16))),
                        Err(error) => Some(Err(error)),
                    })
                    .collect::<Result<_, _>>()?,
                cures: SPEC_CURES
                    .iter()
                    .filter_map(|&(key, stat)| match nx.int_child(spec, key) {
                        Ok(Some(0)) | Ok(None) => None,
                        Ok(Some(_)) => Some(Ok(stat)),
                        Err(error) => Some(Err(error)),
                    })
                    .collect::<Result<_, _>>()?,
                inc: nx.int_child(spec, "inc")?.unwrap_or(0),
                pets,
                move_to: nx.int_child(spec, "moveTo")?,
                ignore_continent: nx.int_child(spec, "ignoreContinent")?.unwrap_or(0) != 0,
                return_map_qr: nx.int_child(spec, "returnMapQR")?.unwrap_or(0),
                exp: nx.int_child(spec, "exp")?.unwrap_or(0),
                inc_fatigue: nx.int_child(spec, "incFatigue")?.unwrap_or(0),
            });
        }
        if item_id / 10_000 == 210 {
            if let Some(mobs) = nx.child_by_name(node, "mob")? {
                while let Some(mob) = nx.child_by_name(mobs, &info.sack.len().to_string())? {
                    let id = nx.int_child(mob, "id")?.unwrap_or(0);
                    let prob = nx.int_child(mob, "prob")?.unwrap_or(0);
                    info.sack.push((id, prob));
                }
            }
        }
        if item_id / 10_000 == 227 {
            if let Some(meta) = nx.child_by_name(node, "info")? {
                let side = |key: &str| -> Result<i16, GameDataError> {
                    Ok(nx
                        .int_child(meta, key)?
                        .map_or(0, |v| i16::try_from(v).unwrap_or(0)))
                };
                info.catch = Some(CatchInfo {
                    mob: nx.int_child(meta, "mob")?.unwrap_or(0),
                    create: nx.int_child(meta, "create")?.unwrap_or(0),
                    area: (side("left")?, side("top")?, side("right")?, side("bottom")?),
                    mob_hp: nx.int_child(meta, "mobHP")?.unwrap_or(100),
                    prop: nx.real_child(meta, "bridleProp")?.unwrap_or(100.0),
                    use_delay: nx.int_child(meta, "useDelay")?.unwrap_or(0),
                });
            }
        }
        if let Some(interact) = nx.child_by_name(node, "interact")? {
            while let Some(command) =
                nx.child_by_name(interact, &info.pet_commands.len().to_string())?
            {
                let level = |key: &str| -> Result<u8, GameDataError> {
                    Ok(nx
                        .int_child(command, key)?
                        .map_or(0, |v| u8::try_from(v).unwrap_or(u8::MAX)))
                };
                info.pet_commands.push(PetCommand {
                    levels: (level("l0")?, level("l1")?),
                    prob: nx.int_child(command, "prob")?.unwrap_or(0),
                    inc: nx
                        .int_child(command, "inc")?
                        .map_or(0, |v| i16::try_from(v).unwrap_or(0)),
                });
            }
        }
        Ok(Some(info))
    }

    fn equipment_info(&self, item_id: i32) -> Result<Option<ItemInfo>, GameDataError> {
        let nx = &self.equipment;
        let image = format!("{item_id:08}.img");
        let mut found = None;
        for directory in EQUIP_DIRECTORIES {
            if let Some(index) = path_index(nx, &format!("{directory}/{image}"))? {
                found = Some(index);
                break;
            }
        }
        let Some(node) = found else {
            return Ok(None);
        };
        let Some(meta) = nx.child_by_name(node, "info")? else {
            return Ok(None);
        };
        let mut pets = Vec::new();
        if item_id / 10_000 == 180 {
            for child in nx.child_indices(node)? {
                if let Ok(pet) = nx.node_name(child)?.parse::<i32>() {
                    pets.push(pet);
                }
            }
        }
        let stat = |key: &str| -> Result<i16, GameDataError> {
            Ok(nx
                .int_child(meta, key)?
                .map_or(0, |v| i16::try_from(v).unwrap_or(0)))
        };
        let equip = EquipInfo {
            upgrade_slots: u8::try_from(nx.int_child(meta, "tuc")?.unwrap_or(0)).unwrap_or(0),
            str: stat("incSTR")?,
            dex: stat("incDEX")?,
            int: stat("incINT")?,
            luk: stat("incLUK")?,
            hp: stat("incMHP")?,
            mp: stat("incMMP")?,
            watk: stat("incPAD")?,
            matk: stat("incMAD")?,
            wdef: stat("incPDD")?,
            mdef: stat("incMDD")?,
            acc: stat("incACC")?,
            avoid: stat("incEVA")?,
            hands: stat("incCraft")?,
            speed: stat("incSpeed")?,
            jump: stat("incJump")?,
            req_level: stat("reqLevel")?,
            req_job: stat("reqJob")?,
            req_str: stat("reqSTR")?,
            req_dex: stat("reqDEX")?,
            req_int: stat("reqINT")?,
            req_luk: stat("reqLUK")?,
            req_fame: stat("reqPOP")?,
            cash: nx.int_child(meta, "cash")? == Some(1),
            recovery: nx.real_child(meta, "recovery")?,
            taming_mob: nx.int_child(meta, "tamingMob")?.unwrap_or(0),
            pets,
        };
        Ok(Some(ItemInfo {
            item_id,
            slot_max: 1,
            quest: nx.int_child(meta, "quest")? == Some(1),
            only: nx.int_child(meta, "only")? == Some(1),
            trade_block: nx.int_child(meta, "tradeBlock")? == Some(1),
            equip_trade_block: nx.int_child(meta, "equipTradeBlock")? == Some(1),
            pickup_block: nx.int_child(meta, "pickUpBlock")? == Some(1),
            account_sharable: nx.int_child(meta, "accountSharable")? == Some(1),
            pquest: nx.int_child(meta, "pquest")? == Some(1),
            expire_on_logout: nx.int_child(meta, "expireOnLogout")? == Some(1),
            not_extend: nx.int_child(meta, "notExtend")? == Some(1),
            meso: 0,
            price: nx.int_child(meta, "price")?.unwrap_or(0),
            unit_price: 0.0,
            pad: 0,
            recovery_hp: 0,
            recovery_mp: 0,
            max_level: 0,
            spec: None,
            book: None,
            scroll: None,
            sack: Vec::new(),
            catch: None,
            equip: Some(equip),
            pet_commands: Vec::new(),
            pet_hunger: 0,
            pet_life: 0,
            pet_name_tag: false,
            pet_chat_balloon: false,
            rate_card: None,
            mist: None,
            sandglass: None,
        }))
    }
}

/// Item names (`String.nx`); pets' and the cash items' messages so far.
pub struct ItemNames {
    strings: NxDataFile,
}

impl ItemNames {
    pub fn load(strings: impl AsRef<Path>) -> Result<Self, GameDataError> {
        Ok(Self {
            strings: NxDataFile::open(strings.as_ref())?,
        })
    }

    /// A pet's species name (`Pet.img/<id>/name`), which new pets take.
    pub fn pet(&self, item_id: i32) -> Result<Option<String>, GameDataError> {
        let nx = &self.strings;
        let Some(node) = path_index(nx, &format!("Pet.img/{item_id}"))? else {
            return Ok(None);
        };
        nx.string_child(node, "name")
    }

    /// A cash item's message (`Cash.img/<id>/msg`), such as a weather
    /// item's `%s's snow : %s`.
    pub fn cash_message(&self, item_id: i32) -> Result<Option<String>, GameDataError> {
        let nx = &self.strings;
        let Some(node) = path_index(nx, &format!("Cash.img/{item_id}"))? else {
            return Ok(None);
        };
        nx.string_child(node, "msg")
    }
}

/// An equipment scroll's `info` and its `req` list of the only equipment it
/// accepts; `None` without `info/success`.
fn scroll_info(nx: &NxDataFile, node: u32, meta: u32) -> Result<Option<ScrollInfo>, GameDataError> {
    let Some(success) = nx.int_child(meta, "success")? else {
        return Ok(None);
    };
    let flag =
        |key: &str| -> Result<bool, GameDataError> { Ok(nx.int_child(meta, key)? == Some(1)) };
    let stat = |key: &str| -> Result<i16, GameDataError> {
        Ok(nx
            .int_child(meta, key)?
            .map_or(0, |v| i16::try_from(v).unwrap_or(0)))
    };
    let kind = if flag("preventslip")? {
        ScrollKind::Spikes
    } else if flag("warmsupport")? {
        ScrollKind::Cold
    } else if flag("recover")? {
        ScrollKind::CleanSlate
    } else if flag("randstat")? {
        ScrollKind::Chaos
    } else {
        ScrollKind::Normal
    };
    let mut req = Vec::new();
    if let Some(list) = nx.child_by_name(node, "req")? {
        while let Some(id) = nx.int_child(list, &req.len().to_string())? {
            req.push(id);
        }
    }
    Ok(Some(ScrollInfo {
        kind,
        success,
        cursed: nx.int_child(meta, "cursed")?.unwrap_or(0),
        stats: EquipStats {
            str: stat("incSTR")?,
            dex: stat("incDEX")?,
            int: stat("incINT")?,
            luk: stat("incLUK")?,
            hp: stat("incMHP")?,
            mp: stat("incMMP")?,
            watk: stat("incPAD")?,
            matk: stat("incMAD")?,
            wdef: stat("incPDD")?,
            mdef: stat("incMDD")?,
            acc: stat("incACC")?,
            avoid: stat("incEVA")?,
            hands: stat("incCraft")?,
            speed: stat("incSpeed")?,
            jump: stat("incJump")?,
            ..EquipStats::default()
        },
        req,
    }))
}

pub(crate) fn path_index(nx: &NxDataFile, path: &str) -> Result<Option<u32>, GameDataError> {
    let mut index = 0;
    for key in path.split('/') {
        match nx.child_by_name(index, key)? {
            Some(child) => index = child,
            None => return Ok(None),
        }
    }
    Ok(Some(index))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> Option<ItemData> {
        let items = crate::test_assets::path("Item.nx")?;
        let equipment = crate::test_assets::path("Character.nx")?;
        Some(ItemData::load(items, equipment).unwrap())
    }

    #[test]
    fn reads_rate_cards_and_their_hours() {
        assert_eq!(rate_window("SAT:19-23"), Some((5, 19, 23)));
        assert_eq!(rate_window("HOL:00-24"), None);
        let Some(data) = data() else {
            return;
        };
        // Weekend Type 5: Saturdays and Sundays, 7 to 11 pm.
        let weekend = data.info(5211013).unwrap().unwrap().rate_card.unwrap();
        assert_eq!(
            (weekend.rate, weekend.windows),
            (2.0, vec![(5, 19, 23), (6, 19, 23)])
        );
        // A drop card's rate is a real, and the 3x card also has a HOL entry.
        let drop = data.info(5360000).unwrap().unwrap().rate_card.unwrap();
        assert_eq!((drop.rate, drop.windows.len()), (2.0, 7));
        let triple = data.info(5211060).unwrap().unwrap().rate_card.unwrap();
        assert_eq!((triple.rate, triple.windows.len()), (3.0, 7));
        assert!(triple.active(3, 0) && triple.active(3, 23));
        assert!(data.info(5040000).unwrap().unwrap().rate_card.is_none());
    }

    #[test]
    fn reads_the_cash_mists_clouds() {
        let Some(data) = data() else {
            return;
        };
        for item_id in [5281000, 5281001] {
            let mist = data.info(item_id).unwrap().unwrap().mist.unwrap();
            assert_eq!(mist.area, (-110, -82, 110, 83));
            assert_eq!(mist.seconds, 60);
        }
        let gas = data.info(5281000).unwrap().unwrap().mist.unwrap();
        // vomit, angry, cry, oops
        assert!([8, 5, 4, 9].iter().all(|&emote| gas.has_face(emote)));
        assert!(!gas.has_face(17) && !gas.has_face(1));
        let scent = data.info(5281001).unwrap().unwrap().mist.unwrap();
        // love, cheers, glitter, chu
        assert_eq!(scent.faces, 1 << 17 | 1 << 10 | 1 << 14 | 1 << 11);
        assert!(data.info(5120000).unwrap().unwrap().mist.is_none());
    }

    #[test]
    fn reads_consumables_quest_items_and_stack_sizes() {
        let Some(data) = data() else {
            return;
        };
        let apple = data.info(2010007).unwrap().unwrap();
        assert!(apple.quest);
        assert_eq!(apple.spec.unwrap().hp, 30);
        assert_eq!(apple.slot_max, DEFAULT_SLOT_MAX);
        let shell = data.info(4000019).unwrap().unwrap();
        assert_eq!(shell.slot_max, 200);
        assert!(!shell.trade_block);
        // Stored as an integer in one and as a string in the other.
        assert!(data.info(2000012).unwrap().unwrap().trade_block);
        assert!(data.info(4031044).unwrap().unwrap().trade_block);
        assert!(data.info(4001007).unwrap().unwrap().pquest);
        assert!(!shell.pquest);
        assert!(data.info(4999999).unwrap().is_none());
        let subi = data.info(2070000).unwrap().unwrap();
        assert_eq!(
            (subi.price, subi.unit_price, subi.slot_max, subi.pad),
            (250, 0.3, 500, 15)
        );
        let all_cure = data.info(2050004).unwrap().unwrap().spec.unwrap();
        assert_eq!(all_cure.cures.len(), 5);
        let warrior_pill = data.info(2002004).unwrap().unwrap().spec.unwrap();
        assert_eq!(
            (warrior_pill.stats, warrior_pill.time),
            (vec![(BuffStat::Pad, 5)], 180_000)
        );
        let morph = data.info(2210000).unwrap().unwrap().spec.unwrap();
        assert!(morph.stats.contains(&(BuffStat::Morph, 1)));
        let air_bubble = data.info(2022040).unwrap().unwrap().spec.unwrap();
        assert_eq!(
            (air_bubble.stats, air_bubble.time),
            (vec![(BuffStat::Thaw, -6)], 900_000)
        );
        let relaxer = data.info(3010000).unwrap().unwrap();
        assert_eq!((relaxer.recovery_hp, relaxer.recovery_mp), (50, 0));
    }

    #[test]
    fn reads_what_a_writ_of_solomon_gives_and_to_whom() {
        let Some(data) = data() else {
            return;
        };
        let writ = |id| {
            let info = data.info(id).unwrap().unwrap();
            (info.max_level, info.spec.unwrap().exp)
        };
        assert_eq!(writ(2370000), (50, 100000));
        assert_eq!(writ(2370012), (50, 100));
        assert_eq!(data.info(2000000).unwrap().unwrap().max_level, 0);
    }

    #[test]
    fn reads_what_a_summoning_sack_brings_out() {
        let Some(data) = data() else {
            return;
        };
        assert_eq!(data.info(2100000).unwrap().unwrap().sack, [(8130100, 100)]);
        let sack = data.info(2100001).unwrap().unwrap().sack;
        assert_eq!(
            (sack.len(), sack[0], sack[15]),
            (16, (100100, 100), (1210102, 50))
        );
        assert!(data.info(2000000).unwrap().unwrap().sack.is_empty());
    }

    #[test]
    fn reads_what_a_catch_item_catches() {
        let Some(data) = data() else {
            return;
        };
        assert_eq!(
            data.info(2270000).unwrap().unwrap().catch,
            Some(CatchInfo {
                mob: 9300101,
                create: 1902000,
                area: (-100, -100, 100, 50),
                mob_hp: 100,
                prop: 100.0,
                use_delay: 0,
            })
        );
        let rock = data.info(2270002).unwrap().unwrap().catch.unwrap();
        assert_eq!((rock.mob_hp, rock.prop, rock.use_delay), (40, 50.0, 800));
        assert_eq!(data.info(2000000).unwrap().unwrap().catch, None);
    }

    #[test]
    fn knows_which_hairs_and_faces_exist() {
        let Some(data) = data() else {
            return;
        };
        assert!(data.has_style(30060).unwrap());
        assert!(data.has_style(20000).unwrap());
        assert!(!data.has_style(30069).unwrap());
        assert!(!data.has_style(1302000).unwrap());
    }

    #[test]
    fn reads_pets_and_their_names() {
        let Some(data) = data() else {
            return;
        };
        let tiger = data.info(5000066).unwrap().unwrap();
        assert_eq!(tiger.slot_max, 1);
        assert_eq!(tiger.pet_hunger, 3);
        assert!(tiger.pet_name_tag && tiger.pet_chat_balloon);
        let kitty = data.info(5000000).unwrap().unwrap();
        assert!(!kitty.pet_name_tag && !kitty.pet_chat_balloon);
        assert_eq!(data.info(5000000).unwrap().unwrap().pet_hunger, 2);
        assert_eq!(data.info(5000000).unwrap().unwrap().pet_life, 90);
        assert_eq!(data.info(5000060).unwrap().unwrap().pet_life, 0);
        assert_eq!(data.info(5180000).unwrap().unwrap().pet_life, 90);
        let food = data.info(2120000).unwrap().unwrap().spec.unwrap();
        assert_eq!(food.inc, 30);
        let drumstick = data.info(5240027).unwrap().unwrap().spec.unwrap();
        assert_eq!((drumstick.inc, drumstick.pets), (100, vec![5000066]));
        assert_eq!(tiger.pet_commands.len(), 36);
        assert_eq!(
            tiger.pet_commands[28],
            PetCommand {
                levels: (1, 9),
                prob: 85,
                inc: 3
            }
        );
        let Some(strings) = crate::test_assets::path("String.nx") else {
            return;
        };
        let names = ItemNames::load(strings).unwrap();
        assert_eq!(names.pet(5000066).unwrap().as_deref(), Some("Baby Tiger"));
        assert_eq!(names.pet(5000000).unwrap().as_deref(), Some("Brown Kitty"));
        assert_eq!(names.pet(5009999).unwrap(), None);
        assert_eq!(
            names.cash_message(5120000).unwrap().as_deref(),
            Some("%s's snow : %s")
        );
        assert_eq!(names.cash_message(5121011).unwrap(), None);
    }

    #[test]
    fn reads_skill_and_mastery_books() {
        let Some(data) = data() else {
            return;
        };
        let book = data.info(2290000).unwrap().unwrap().book.unwrap();
        assert_eq!(
            book,
            BookInfo {
                skills: vec![1121001, 1221001, 1321001],
                master_level: 20,
                req_skill_level: 5,
                success: 70,
            }
        );
        let book = data.info(2280000).unwrap().unwrap().book.unwrap();
        assert_eq!(
            (book.skills, book.master_level, book.req_skill_level),
            (vec![2121003], 10, 0)
        );
        assert!(data.info(2010007).unwrap().unwrap().book.is_none());
    }

    #[test]
    fn reads_equipment_scrolls() {
        let Some(data) = data() else {
            return;
        };
        let dark = data.info(2043004).unwrap().unwrap().scroll.unwrap();
        assert_eq!(
            (dark.kind, dark.success, dark.cursed),
            (ScrollKind::Normal, 70, 50)
        );
        assert_eq!((dark.stats.watk, dark.stats.str, dark.stats.dex), (2, 1, 0));
        assert!(dark.req.is_empty());
        let chaos = data.info(2049100).unwrap().unwrap().scroll.unwrap();
        assert_eq!(
            (chaos.kind, chaos.success, chaos.cursed),
            (ScrollKind::Chaos, 60, 0)
        );
        let clean_slate = data.info(2049003).unwrap().unwrap().scroll.unwrap();
        assert_eq!(
            (clean_slate.kind, clean_slate.success, clean_slate.cursed),
            (ScrollKind::CleanSlate, 20, 50)
        );
        let dragon_stone = data.info(2041200).unwrap().unwrap().scroll.unwrap();
        assert_eq!(dragon_stone.req, vec![1122000]);
        let spikes = data.info(2040727).unwrap().unwrap().scroll.unwrap();
        assert_eq!(spikes.kind, ScrollKind::Spikes);
        let cold = data.info(2041058).unwrap().unwrap().scroll.unwrap();
        assert_eq!(cold.kind, ScrollKind::Cold);
        assert!(data.info(2010007).unwrap().unwrap().scroll.is_none());
    }

    #[test]
    fn reads_monster_cards_and_their_mobs() {
        let Some(data) = data() else {
            return;
        };
        let cards = data.monster_cards().unwrap();
        assert!(cards.len() > 300);
        assert_eq!(cards[0], (2380000, 100100)); // Snail
        assert!(cards.contains(&(2380001, 100101))); // Blue Snail
    }

    #[test]
    fn reads_equipment_stats_and_requirements() {
        let Some(data) = data() else {
            return;
        };
        let hat = data.info(1002067).unwrap().unwrap();
        let equip = hat.equip.unwrap();
        assert_eq!(
            (equip.upgrade_slots, equip.wdef, equip.req_level),
            (7, 5, 5)
        );
        assert_eq!(hat.slot_max, 1);
        let sword = data.info(1302000).unwrap().unwrap().equip.unwrap();
        assert!(sword.watk > 0);
        assert_eq!(sword.recovery, None);
        let robe = data.info(1050018).unwrap().unwrap().equip.unwrap();
        assert_eq!(robe.recovery, Some(1.5));
        let earrings = data.info(1032031).unwrap().unwrap();
        assert!(earrings.equip_trade_block && !earrings.trade_block);
        assert!(data.info(1112405).unwrap().unwrap().account_sharable);
        assert!(!earrings.account_sharable);
    }

    #[test]
    fn reads_which_pets_pet_wear_suits() {
        let Some(data) = data() else {
            return;
        };
        let wings = data.info(1802063).unwrap().unwrap().equip.unwrap();
        assert_eq!((wings.pets, wings.cash), (vec![5000066], true));
        let hat = data.info(1802000).unwrap().unwrap().equip.unwrap();
        assert!(hat.pets.contains(&5000000) && !hat.pets.contains(&5000066));
        let magnet = data.info(1812000).unwrap().unwrap().equip.unwrap();
        assert!(magnet.pets.is_empty());
    }
}
