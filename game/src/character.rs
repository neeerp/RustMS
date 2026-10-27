use crate::buff::{BuffStat, Buffs, ShownBuff};
use crate::dojo::DojoRecord;
use crate::guild::GuildTag;
use crate::inventory::{equip_slot, EquipStats, Inventory, InventoryType, Item};
use crate::keymap::Keymap;
use crate::monster_book::MonsterBook;
use crate::mount::Mount;
use crate::pet::{FieldPet, FOLLOW_THE_LEAD};
use crate::quest::QuestLog;
use crate::quick_slots::QuickSlots;
use crate::skill::{self, SkillBook, SkillRecord};
use crate::skill_macro::SkillMacros;
use crate::teleport_rock::TeleportRocks;
use std::collections::BTreeMap;

/// The most STR, DEX, INT, or LUK AP can raise a stat to.
pub const MAX_BASE_STAT: i16 = 999;
/// The most max HP or max MP AP can raise.
pub const MAX_HP_MP: i16 = 30_000;
/// The most SP the three beginner skills can share (one per level from 2).
pub const MAX_BEGINNER_SP: i32 = 6;
/// The `saved_locations` key of the town the player entered the Free Market
/// from. Script keys are below 1000; event invitations use their
/// `returnMapQR`, which starts at 1056.
pub const FREE_MARKET_RETURN: i16 = 0;
/// The `saved_locations` key of the map a world tour with Spinel left from.
pub const WORLD_TOUR_RETURN: i16 = 1;
/// The `saved_locations` key of the town a trip to Florina Beach left from.
pub const FLORINA_RETURN: i16 = 2;
/// The `saved_locations` key of the map the player entered Nett's Pyramid's
/// Pyramid Dunes from.
pub const PYRAMID_RETURN: i16 = 3;
/// The `saved_locations` key of the map the player took the Mu Lung
/// Dojo's notice board from (Cosmic `MIRROR`).
pub const DOJO_RETURN: i16 = 4;
/// The `saved_locations` key of the map the player went to the Monster
/// Carnival's Carnival Field from (Cosmic `MONSTER_CARNIVAL`).
pub const CARNIVAL_RETURN: i16 = 5;
/// The `saved_locations` key of the map an Event Assistant took the player
/// to a GM's event from (Cosmic `EVENT`).
pub const EVENT_RETURN: i16 = 6;

/// A stat `SPEND_AP` can raise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApStat {
    Str,
    Dex,
    Int,
    Luk,
    MaxHp,
    MaxMp,
}

/// The core stat block that v83 sends in character lists, SET_FIELD, and
/// stat updates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterStats {
    pub id: i32,
    pub name: String,
    pub gender: u8,
    pub skin: u8,
    pub face: i32,
    /// Hair style including its color digit, as the client expects it.
    pub hair: i32,
    pub level: u8,
    pub job: i16,
    pub str: i16,
    pub dex: i16,
    pub int: i16,
    pub luk: i16,
    pub hp: i16,
    pub max_hp: i16,
    pub mp: i16,
    pub max_mp: i16,
    pub ap: i16,
    pub sp: i16,
    pub exp: i32,
    /// EXP from Writs of Solomon, taken by double-clicking the EXP gauge.
    pub gacha_exp: i32,
    pub fame: i16,
    pub map_id: i32,
    /// Portal id the character respawns at when entering `map_id`.
    pub spawn_point: u8,
}

/// What other players see: body, face, hair, and visible equipment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AvatarLook {
    pub gender: u8,
    pub skin: u8,
    pub face: i32,
    pub hair: i32,
    /// Visible equipment keyed by positive body slot (1 = cap, 5 = top, ...).
    /// Cash items override regular items in the same slot.
    pub visible: BTreeMap<u8, i32>,
    /// Regular items hidden underneath a cash item, keyed the same way.
    pub masked: BTreeMap<u8, i32>,
    /// The cash weapon shown over the regular weapon, if any.
    pub cash_weapon: Option<i32>,
    /// The item ids of the summoned pets, in list order, 0 past the last.
    pub pets: [i32; 3],
}

impl AvatarLook {
    /// Build a look from equipped item ids keyed by their (negative) slots.
    pub fn from_equipped(
        stats: &CharacterStats,
        equipped: impl IntoIterator<Item = (i16, i32)>,
    ) -> Self {
        let mut regular = BTreeMap::new();
        let mut cash = BTreeMap::new();
        let mut cash_weapon = None;
        for (slot, item_id) in equipped {
            if slot == equip_slot::CASH_WEAPON {
                cash_weapon = Some(item_id);
            } else if let Some(body) = equip_slot::cash_body_slot(slot) {
                cash.insert(body, item_id);
            } else if let Some(body) = equip_slot::body_slot(slot) {
                regular.insert(body, item_id);
            }
        }

        let mut visible = regular.clone();
        let mut masked = BTreeMap::new();
        for (body, item_id) in cash {
            if let Some(hidden) = visible.insert(body, item_id) {
                masked.insert(body, hidden);
            }
        }

        Self {
            gender: stats.gender,
            skin: stats.skin,
            face: stats.face,
            hair: stats.hair,
            visible,
            masked,
            cash_weapon,
            pets: [0; 3],
        }
    }
}

/// What a field needs in order to show a player to everyone else on the map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldAvatar {
    pub id: i32,
    pub name: String,
    pub level: u8,
    pub job: i16,
    pub look: AvatarLook,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
    /// The portable chair (a 301xxxx setup item) the player sits on, or 0.
    pub chair: i32,
    /// The buffs others see, as `Buffs::shown`.
    pub buffs: Vec<ShownBuff>,
    pub guild: Option<GuildTag>,
    /// The balloon of the mini room the player keeps open, if any. The
    /// field sets it.
    pub mini_room: Option<MiniRoomBox>,
    /// The summoned pets, in list order. The field moves them.
    pub pets: Vec<FieldPet>,
    /// The note on the chalkboard the player holds up, if any.
    pub chalkboard: Option<String>,
    /// The effect item (501xxxx or 429xxxx) the player shows, or 0.
    pub item_effect: i32,
    /// The item whose aura (recv 166) the player's buffs show, or 0.
    pub consume_effect: i32,
    /// Whether the player's Berserk aura shows.
    pub berserk: bool,
    /// The pairs' ring effects, while the player wears a crush, friendship
    /// or wedding ring.
    pub rings: crate::ring::RingLooks,
    /// Energy Charge's gauge, as `Buffs::energy`.
    pub energy: Option<ShownBuff>,
    /// Dash's speed and jump, while the player dashes.
    pub dash: [Option<ShownBuff>; 2],
    /// The tamed monster the player rides, if any.
    pub ride: Option<Ride>,
    /// The player's mount's level, EXP and tiredness, which every player
    /// shows whether they ride or not.
    pub mount: Mount,
    /// Whether the player's account is a GM's: only GMs see a hidden GM.
    pub gm: bool,
    /// Whether the player is a GM hidden by Hide.
    pub hidden: bool,
    /// The player's side in an event field, Monster Carnival's 0 Maple Red
    /// or 1 Maple Blue. The client sets it.
    pub team: u8,
}

/// What a player rides (the client's RideVehicle data): the mount item and
/// the skill that rode it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ride {
    pub item_id: i32,
    pub skill_id: i32,
}

/// The balloon over a player who keeps a mini room open, which others
/// double-click to visit it (v83 UPDATE_CHAR_BOX).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MiniRoomBox {
    /// The room type: 1 Omok, 2 match cards, 4 a personal shop.
    pub kind: u8,
    pub sn: i32,
    pub title: String,
    /// The art: for a shop, its permit's skin; for Omok, the stones; for
    /// match cards, the size of the board.
    pub skin: u8,
    pub users: u8,
    pub max_users: u8,
    /// A game asks for a password to visit.
    pub private: bool,
    /// A game is being played.
    pub in_progress: bool,
}

/// A character that is logged in. The owning client actor holds the only
/// copy; everything else (fields, the world directory) receives projections.
#[derive(Clone, Debug)]
pub struct Player {
    pub account_id: i32,
    pub world_id: u8,
    pub stats: CharacterStats,
    pub meso: i32,
    pub buddy_capacity: u8,
    /// The account's highest other character in this world, whose name
    /// Blessing of the Fairy's tooltip shows; looked up at each entry, not
    /// saved.
    pub linked_name: Option<String>,
    pub inventory: Inventory,
    pub skills: SkillBook,
    pub keymap: Keymap,
    pub quests: QuestLog,
    pub monster_book: MonsterBook,
    /// Account GM level; 0 for regular players.
    pub gm_level: i16,
    /// Active buffs and cooldowns; not saved.
    pub buffs: Buffs,
    /// The portable chair the player sits on, or 0; not saved.
    pub chair: i32,
    /// The guild the character belongs to; saved by `db::guild`, not with
    /// the player.
    pub guild: Option<GuildTag>,
    /// The character's senior and juniors; saved by `db::family`, not with
    /// the player.
    pub family: crate::family::Relatives,
    /// The family rate entitlement working for the character; saved by
    /// `db::family`, not with the player.
    pub family_buff: Option<crate::family::FamilyBuff>,
    /// The character's engagement or marriage; saved by `db::marriage`,
    /// not with the player.
    pub marriage: Option<crate::marriage::MarriageRecord>,
    /// The crush and friendship rings the character carries, worn or not;
    /// saved by `db::ring`, not with the player.
    pub rings: Vec<crate::ring::CoupleRing>,
    /// The cash serials of the summoned pets, in the client's list order;
    /// saved with the pets.
    pub pets: Vec<i64>,
    /// The note on the chalkboard the player holds up, if any; not saved.
    pub chalkboard: Option<String>,
    /// The effect item (501xxxx or 429xxxx) the player shows, or 0; not
    /// saved.
    pub item_effect: i32,
    /// The item whose aura (recv 166) the others were last shown, or 0;
    /// not saved. `net::consume_effect` keeps it up to date.
    pub consume_effect: i32,
    /// A charm kept the player's EXP when they last died, so they revive
    /// with 30% of their max HP and MP; not saved.
    pub charmed: bool,
    /// Whether Berserk is on, as the client last told the server
    /// (`net::berserk`); not saved.
    pub berserk: bool,
    /// Whether the GM hid with Hide (9101004); not saved.
    pub hidden: bool,
    /// The catch item (227xxxx) last thrown and when, for its `useDelay`;
    /// not saved.
    pub last_catch: Option<(i32, std::time::Instant)>,
    /// The cash mist (528xxxx) last used, whose face the player may pull
    /// once; not saved.
    pub mist_face: Option<i32>,
    /// The field the scene shown on the player's map leads to, which the
    /// client asks for by id when the scene ends; not saved, and gone once
    /// the player leaves the map.
    pub scene_exit: Option<i32>,
    /// The maps kept for the teleport rocks.
    pub teleport_rocks: TeleportRocks,
    /// The skill macros, by place in the list.
    pub skill_macros: SkillMacros,
    /// The tamed monster's level, EXP and tiredness.
    pub mount: Mount,
    /// Training points and the score recorded at the Mu Lung Dojo.
    pub dojo: DojoRecord,
    /// The energy built on the Dojo's maps, up to `dojo::FULL_ENERGY`;
    /// not saved.
    pub dojo_energy: i32,
    /// The uses of Rage of Pharaoh the player's Nett's Pyramid run has
    /// left, as the run last told; not saved.
    pub pyramid_rage: i32,
    /// The mode (0 to 3) of the Nett's Pyramid run the player just went
    /// all the way through, whose gem for Pharaoh Yeti's Tomb Duarte offers
    /// in its room in the Shades; kept from its result map to that room,
    /// and gone on any other way out of either. Not saved.
    pub pyramid_gem: Option<i32>,
    /// How the Monster Carnival battle the player just fought ended for
    /// them: whether their side won, and its CP in all, which 2042002
    /// grades in their result room. Gone on the way out of it. Not saved.
    pub carnival_result: Option<(bool, i32)>,
    /// Where the player came from before a trip, to go back to when it
    /// ends: the town they entered the Free Market from
    /// (`FREE_MARKET_RETURN`), the map a world tour or a trip to Florina
    /// Beach left from (`WORLD_TOUR_RETURN`, `FLORINA_RETURN`), or the map
    /// they used an event invitation in, under its `returnMapQR`.
    pub saved_locations: BTreeMap<i16, i32>,
    /// The records kept beside the quest log, as the party quests'
    /// (`crate::quest_ex`).
    pub quest_ex: crate::quest_ex::QuestExRecords,
    /// The potions the first pet gives from the HP and MP pouches' slots,
    /// or 0.
    pub auto_potions: AutoPotions,
    /// The quick slot panel's keys, or None if never set, when the client
    /// is told to use its defaults.
    pub quick_slots: Option<QuickSlots>,
    /// Until when the second pendant slot (`equip_slot::PENDANT2`) is
    /// open, if 5550000 was ever bought for it.
    pub extra_pendant: Option<std::time::SystemTime>,
    dirty: bool,
}

/// The potions set in the pet panel's slots beside the HP and MP pouches
/// (v83 keeps them with the key map, at `+0x3c0` and `+0x3c4`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoPotions {
    pub hp: i32,
    pub mp: i32,
}

impl Player {
    pub fn new(
        account_id: i32,
        world_id: u8,
        stats: CharacterStats,
        meso: i32,
        inventory: Inventory,
        skills: SkillBook,
        keymap: Keymap,
        quests: QuestLog,
    ) -> Self {
        Self {
            account_id,
            world_id,
            stats,
            meso,
            buddy_capacity: 20,
            linked_name: None,
            inventory,
            skills,
            keymap,
            quests,
            monster_book: MonsterBook::default(),
            gm_level: 0,
            buffs: Buffs::default(),
            chair: 0,
            guild: None,
            family: Default::default(),
            family_buff: None,
            marriage: None,
            rings: Vec::new(),
            pets: Vec::new(),
            chalkboard: None,
            item_effect: 0,
            consume_effect: 0,
            charmed: false,
            berserk: false,
            hidden: false,
            last_catch: None,
            mist_face: None,
            scene_exit: None,
            teleport_rocks: TeleportRocks::default(),
            skill_macros: SkillMacros::default(),
            mount: Mount::default(),
            dojo: DojoRecord::default(),
            dojo_energy: 0,
            pyramid_rage: 0,
            pyramid_gem: None,
            carnival_result: None,
            saved_locations: BTreeMap::new(),
            quest_ex: Default::default(),
            auto_potions: AutoPotions::default(),
            quick_slots: None,
            extra_pendant: None,
            dirty: false,
        }
    }

    /// Whether a pendant may be worn in the second pendant slot at `now`.
    pub fn extra_pendant_open(&self, now: std::time::SystemTime) -> bool {
        self.extra_pendant.is_some_and(|until| until > now)
    }

    /// The stats of what the player wears that count. A pendant in the
    /// second pendant slot stops counting once the slot runs out, as v83's
    /// BasicStat::SetFrom (0x77ec9f) skips slot 51 past its expiry; it
    /// stays worn.
    pub fn worn_equips(&self) -> impl Iterator<Item = &EquipStats> {
        let shut = !self.extra_pendant_open(std::time::SystemTime::now());
        self.inventory
            .items(InventoryType::Equipped)
            .filter(move |(position, _)| !(shut && *position == equip_slot::PENDANT2))
            .filter_map(|(_, item)| item.equip.as_ref())
    }

    pub fn id(&self) -> i32 {
        self.stats.id
    }

    /// Link the player to the account's highest other character, `(name,
    /// level)`: Blessing of the Fairy takes one level for each 10 of its,
    /// master level 20. With none, the skill goes.
    pub fn link(&mut self, linked: Option<(String, i16)>) {
        let Some(skill_id) = skill::fairy_blessing(self.stats.job) else {
            return;
        };
        match linked {
            Some((name, level)) => {
                self.linked_name = Some(name);
                self.skills.set(SkillRecord {
                    skill_id,
                    level: skill::fairy_blessing_level(level),
                    master_level: 20,
                    expiration: None,
                });
            }
            None => {
                self.linked_name = None;
                self.skills.remove(skill_id);
            }
        }
    }

    pub fn name(&self) -> &str {
        &self.stats.name
    }

    pub fn look(&self) -> AvatarLook {
        let mut look = AvatarLook::from_equipped(&self.stats, self.inventory.equipped_ids());
        for (id, pet) in look.pets.iter_mut().zip(self.summoned_pets()) {
            *id = pet.item_id;
        }
        look
    }

    /// The summoned pets' items, in list order.
    pub fn summoned_pets(&self) -> impl Iterator<Item = &Item> + '_ {
        (0..self.pets.len()).filter_map(|index| self.summoned_pet(index))
    }

    /// The item of the pet at `index` in the list, if one is out there.
    pub fn summoned_pet(&self, index: usize) -> Option<&Item> {
        let serial = *self.pets.get(index)?;
        self.inventory
            .items(InventoryType::Cash)
            .map(|(_, item)| item)
            .find(|item| item.cash_serial == Some(serial) && item.pet.is_some())
    }

    /// The exception list of the pet `serial`, empty if it isn't carried.
    pub fn pet_ignored(&self, serial: i64) -> &[i32] {
        self.inventory
            .items(InventoryType::Cash)
            .find_map(|(_, item)| {
                item.pet
                    .as_ref()
                    .filter(|_| item.cash_serial == Some(serial))
            })
            .map_or(&[], |pet| &pet.ignored)
    }

    /// The summoned pets' cash serials, 0 past the last.
    pub fn pet_serials(&self) -> [i64; 3] {
        let mut serials = [0; 3];
        for (slot, &serial) in serials.iter_mut().zip(&self.pets) {
            *slot = serial;
        }
        serials
    }

    /// Whether the player may keep more than one pet out.
    pub fn follows_the_lead(&self) -> bool {
        FOLLOW_THE_LEAD
            .iter()
            .any(|&skill| self.skills.level(skill) > 0)
    }

    /// Mutable access to the summoned pets; marks the player as needing a
    /// save.
    pub fn pets_mut(&mut self) -> &mut Vec<i64> {
        self.dirty = true;
        &mut self.pets
    }

    /// The skill the player rides with, while they ride.
    pub fn ride_skill(&self) -> Option<i32> {
        let riding = self
            .buffs
            .active()
            .iter()
            .find(|buff| buff.stat == BuffStat::Riding)?;
        Some(riding.skill_id)
    }

    /// What the player rides: the Battleship, an event skill's vehicle, or
    /// else the mount worn, while they ride it.
    pub fn ride(&self) -> Option<Ride> {
        let skill_id = self.ride_skill()?;
        if skill_id == crate::mount::BATTLESHIP {
            let item_id = crate::mount::BATTLESHIP_VEHICLE;
            return Some(Ride { item_id, skill_id });
        }
        if let Some(item_id) = crate::mount::event_vehicle(skill_id, self.skills.level(skill_id)) {
            return Some(Ride { item_id, skill_id });
        }
        let item_id = self
            .inventory
            .get(InventoryType::Equipped, equip_slot::MOUNT)?
            .item_id;
        Some(Ride { item_id, skill_id })
    }

    /// What a pet wears at `slot` (one of `equip_slot`'s pet slots, such
    /// as `PET_CLOTHES`), worn as cash 100 below it or as regular equipment.
    pub fn pet_equip(&self, slot: i16) -> Option<&Item> {
        self.inventory
            .get(InventoryType::Equipped, slot - 100)
            .or_else(|| self.inventory.get(InventoryType::Equipped, slot))
    }

    /// Whether the pet out at `index` wears a quote ring, so that it speaks
    /// in its kind's chat balloon.
    pub fn pet_quoted(&self, index: usize) -> bool {
        equip_slot::PET_QUOTES
            .get(index)
            .is_some_and(|&slot| self.pet_equip(slot).is_some())
    }

    /// The marriage block of the player's look: while the player has a
    /// couple and wears a wedding ring. The ring named is the record's, as
    /// the client takes for the player's own (report b059).
    pub fn marriage_ring(&self) -> Option<crate::marriage::MarriageRing> {
        let record = self.marriage.as_ref()?;
        self.inventory
            .equipped_regular()
            .any(|(_, item)| crate::marriage::is_wedding_ring(item.item_id))
            .then(|| crate::marriage::MarriageRing {
                self_id: self.stats.id,
                partner_id: record.partner_of(self.stats.id),
                item_id: record.ring_id(),
            })
    }

    /// The crush or friendship block of the player's look: the first ring of
    /// `kind` worn, if one of the player's pairs has it (report b061).
    pub fn ring_look(&self, kind: crate::ring::RingKind) -> Option<crate::ring::RingLook> {
        crate::ring::cash_ring_slots()
            .filter_map(|slot| self.inventory.get(InventoryType::Equipped, slot))
            .filter(|item| crate::ring::kind(item.item_id) == Some(kind))
            .find_map(|item| {
                let serial = item.cash_serial?;
                let ring = self.rings.iter().find(|ring| ring.id == serial)?;
                Some(crate::ring::RingLook {
                    ring_id: ring.id,
                    partner_ring_id: ring.partner_ring_id,
                    item_id: item.item_id,
                })
            })
    }

    /// The ring blocks of the player's look.
    pub fn ring_looks(&self) -> crate::ring::RingLooks {
        crate::ring::RingLooks {
            crush: self.ring_look(crate::ring::RingKind::Crush),
            friendship: self.ring_look(crate::ring::RingKind::Friendship),
            marriage: self.marriage_ring(),
        }
    }

    /// The projection a field broadcasts for this player at a position.
    pub fn field_avatar(&self, x: i16, y: i16, stance: u8) -> FieldAvatar {
        FieldAvatar {
            id: self.stats.id,
            name: self.stats.name.clone(),
            level: self.stats.level,
            job: self.stats.job,
            look: self.look(),
            x,
            y,
            stance,
            chair: 0,
            buffs: self.buffs.shown(),
            guild: self.guild.clone(),
            mini_room: None,
            pets: (0..self.pets.len())
                .filter_map(|index| Some((index, self.summoned_pet(index)?)))
                .map(|(index, item)| FieldPet {
                    item_id: item.item_id,
                    name: item
                        .pet
                        .as_ref()
                        .map_or_else(String::new, |pet| pet.name.clone()),
                    serial: item.cash_serial.unwrap_or(0),
                    x,
                    y,
                    stance: 0,
                    foothold: 0,
                    name_tag: self.pet_equip(equip_slot::PET_LABELS[index]).is_some(),
                    chat_balloon: self.pet_quoted(index),
                })
                .collect(),
            chalkboard: self.chalkboard.clone(),
            item_effect: self.item_effect,
            consume_effect: self.consume_effect,
            berserk: self.berserk,
            rings: self.ring_looks(),
            energy: self.buffs.energy(),
            dash: [BuffStat::DashSpeed, BuffStat::DashJump].map(|stat| self.buffs.two_state(stat)),
            ride: self.ride(),
            mount: self.mount,
            gm: self.gm_level > 0,
            hidden: self.hidden,
            team: 0,
        }
    }

    /// Record that the character entered `map_id` at `spawn_point`.
    pub fn move_to(&mut self, map_id: i32, spawn_point: u8) {
        self.stats.map_id = map_id;
        self.stats.spawn_point = spawn_point;
        self.scene_exit = None;
        self.dirty = true;
    }

    /// Record where the character should appear when they next enter
    /// their current map (after logout or a channel change).
    pub fn set_spawn_point(&mut self, spawn_point: u8) {
        if self.stats.spawn_point != spawn_point {
            self.stats.spawn_point = spawn_point;
            self.dirty = true;
        }
    }

    /// Mutable access to the keymap; marks the player as needing a save.
    pub fn keymap_mut(&mut self) -> &mut Keymap {
        self.dirty = true;
        &mut self.keymap
    }

    /// Mutable access to the Monster Book; marks the player as needing a
    /// save.
    pub fn monster_book_mut(&mut self) -> &mut MonsterBook {
        self.dirty = true;
        &mut self.monster_book
    }

    /// Mutable access to the inventory; marks the player as needing a save.
    pub fn inventory_mut(&mut self) -> &mut Inventory {
        self.dirty = true;
        &mut self.inventory
    }

    /// Add EXP, levelling up as many times as it covers. Each level grants
    /// 5 AP, 3 SP past beginner, and the job family's max HP/MP growth plus
    /// `bonus`, what the player's skills and INT add to it, up to 30000;
    /// `roll(spread)` picks the random part, 0 to `spread`. Current HP and MP
    /// stay as they were, as in v83. Returns the number of levels gained.
    pub fn gain_exp(
        &mut self,
        amount: i32,
        bonus: (i16, i16),
        mut roll: impl FnMut(i16) -> i16,
    ) -> u8 {
        if amount <= 0 {
            return 0;
        }
        let mut exp = i64::from(self.stats.exp) + i64::from(amount);
        let mut gained = 0;
        while let Some(needed) = crate::exp::to_next_level(self.stats.level) {
            if exp < needed {
                break;
            }
            exp -= needed;
            self.stats.level += 1;
            self.stats.ap = self.stats.ap.saturating_add(5);
            if self.stats.job % 1000 != 0 {
                self.stats.sp = self.stats.sp.saturating_add(3);
            }
            let (hp, mp) = growth(self.stats.job).level_up(&mut roll);
            let stats = &mut self.stats;
            stats.max_hp = stats.max_hp.saturating_add(hp + bonus.0).min(MAX_HP_MP);
            stats.max_mp = stats.max_mp.saturating_add(mp + bonus.1).min(MAX_HP_MP);
            gained += 1;
        }
        if crate::exp::to_next_level(self.stats.level).is_none() {
            exp = 0;
        }
        self.stats.exp = i32::try_from(exp).unwrap_or(i32::MAX);
        self.dirty = true;
        gained
    }

    /// Whether this beginner may take the first job `job` (100, 200, 300,
    /// 400 or 500): level 10, or 8 for magicians, and 35 STR, 20 INT,
    /// 25 DEX, 25 DEX or 20 DEX.
    pub fn can_take_first_job(&self, job: i16) -> bool {
        let stats = &self.stats;
        let Some((level, _, _)) = first_job_rules(job) else {
            return false;
        };
        let (stat, needed) = match job {
            100 => (stats.str, 35),
            200 => (stats.int, 20),
            500 => (stats.dex, 20),
            _ => (stats.dex, 25),
        };
        stats.job == 0 && stats.level >= level && stat >= needed
    }

    /// Become a first-job `job`: 3 SP for each level past the job's
    /// minimum, plus 1, and the job's max HP/MP bonus. Base stats and
    /// current HP/MP stay as they are.
    pub fn take_first_job(&mut self, job: i16) {
        let Some((level, hp, mp)) = first_job_rules(job) else {
            return;
        };
        let stats = &mut self.stats;
        let sp = (i16::from(stats.level) - i16::from(level)).max(0) * 3 + 1;
        stats.job = job;
        stats.sp = stats.sp.saturating_add(sp);
        stats.max_hp = stats.max_hp.saturating_add(hp).min(MAX_HP_MP);
        stats.max_mp = stats.max_mp.saturating_add(mp).min(MAX_HP_MP);
        self.dirty = true;
    }

    /// Whether this first-job Explorer may take the second job `job` of its
    /// line (e.g. a 100 may become a 110, 120 or 130): level 30.
    pub fn can_take_second_job(&self, job: i16) -> bool {
        let stats = &self.stats;
        second_job_rules(job).is_some() && stats.job == job / 100 * 100 && stats.level >= 30
    }

    /// Become a second-job `job`: 1 SP and the job's max HP/MP bonus.
    pub fn take_second_job(&mut self, job: i16) {
        let Some((hp, mp)) = second_job_bonus(job) else {
            return;
        };
        let stats = &mut self.stats;
        stats.job = job;
        stats.sp = stats.sp.saturating_add(1);
        stats.max_hp = stats.max_hp.saturating_add(hp).min(MAX_HP_MP);
        stats.max_mp = stats.max_mp.saturating_add(mp).min(MAX_HP_MP);
        self.dirty = true;
    }

    /// Whether this second-job Explorer may take the third job `job` that
    /// follows it (e.g. a 110 may become a 111): level 70, with no more SP
    /// left than 3 for each level past 70.
    pub fn can_take_third_job(&self, job: i16) -> bool {
        let stats = &self.stats;
        job % 10 == 1
            && second_job_rules(job - 1).is_some()
            && stats.job == job - 1
            && stats.level >= 70
            && stats.sp <= (i16::from(stats.level) - 70) * 3
    }

    /// Become a third-job `job`: 1 SP, 5 AP and the second job's max HP/MP
    /// bonus again.
    pub fn take_third_job(&mut self, job: i16) {
        let Some((hp, mp)) = second_job_rules(job - 1) else {
            return;
        };
        let stats = &mut self.stats;
        stats.job = job;
        stats.sp = stats.sp.saturating_add(1);
        stats.ap = stats.ap.saturating_add(5);
        stats.max_hp = stats.max_hp.saturating_add(hp).min(MAX_HP_MP);
        stats.max_mp = stats.max_mp.saturating_add(mp).min(MAX_HP_MP);
        self.dirty = true;
    }

    /// Whether this third-job Explorer may take the fourth job `job` that
    /// follows it (e.g. a 111 may become a 112): level 120.
    pub fn can_take_fourth_job(&self, job: i16) -> bool {
        let stats = &self.stats;
        job % 10 == 2
            && second_job_rules(job - 2).is_some()
            && stats.job == job - 1
            && stats.level >= 120
    }

    /// Become a fourth-job `job`: 3 SP, 5 AP and the second job's max HP/MP
    /// bonus again. The job's skills are taught apart from this.
    pub fn take_fourth_job(&mut self, job: i16) {
        let Some((hp, mp)) = second_job_rules(job - 2) else {
            return;
        };
        let stats = &mut self.stats;
        stats.job = job;
        stats.sp = stats.sp.saturating_add(3);
        stats.ap = stats.ap.saturating_add(5);
        stats.max_hp = stats.max_hp.saturating_add(hp).min(MAX_HP_MP);
        stats.max_mp = stats.max_mp.saturating_add(mp).min(MAX_HP_MP);
        self.dirty = true;
    }

    /// Spend one AP on `stat`, which for max HP or MP adds the job's growth
    /// plus `bonus`, what the player's skills and INT add to it, with
    /// `roll` as in `gain_exp`. Returns false, changing nothing, without AP
    /// or once the stat is at its cap.
    pub fn spend_ap(
        &mut self,
        stat: ApStat,
        bonus: (i16, i16),
        mut roll: impl FnMut(i16) -> i16,
    ) -> bool {
        if self.stats.ap <= 0 {
            return false;
        }
        let (hp, mp) = growth(self.stats.job).ap(&mut roll);
        let (hp, mp) = (hp + bonus.0, mp + bonus.1);
        let stats = &mut self.stats;
        let (value, gain, cap) = match stat {
            ApStat::Str => (&mut stats.str, 1, MAX_BASE_STAT),
            ApStat::Dex => (&mut stats.dex, 1, MAX_BASE_STAT),
            ApStat::Int => (&mut stats.int, 1, MAX_BASE_STAT),
            ApStat::Luk => (&mut stats.luk, 1, MAX_BASE_STAT),
            ApStat::MaxHp => (&mut stats.max_hp, hp, MAX_HP_MP),
            ApStat::MaxMp => (&mut stats.max_mp, mp, MAX_HP_MP),
        };
        if *value >= cap {
            return false;
        }
        *value = value.saturating_add(gain).min(cap);
        stats.ap -= 1;
        self.dirty = true;
        true
    }

    /// SP left for the three beginner skills: one per level from 2 up to
    /// six, less what they already have.
    pub fn beginner_sp(&self) -> i32 {
        let base = i32::from(self.stats.job) / 1000 * 10_000_000;
        let used: i32 = (1000..=1002).map(|id| self.skills.level(base + id)).sum();
        (i32::from(self.stats.level) - 1).min(MAX_BEGINNER_SP) - used
    }

    /// Raise `skill_id` a level, up to `max_level`, paying one SP (or one
    /// beginner SP for the beginner skills). Fourth-job skills stop at the
    /// master level the character was granted instead. Returns the new
    /// record, or `None` if the skill can't go higher or there is no SP.
    /// Which skills the character may learn is the caller's to check.
    pub fn raise_skill(&mut self, skill_id: i32, max_level: i32) -> Option<SkillRecord> {
        let mut record = self.skills.get(skill_id).cloned().unwrap_or(SkillRecord {
            skill_id,
            level: 0,
            master_level: 0,
            expiration: None,
        });
        let cap = if skill::is_fourth_job(skill_id) {
            record.master_level.min(max_level)
        } else {
            max_level
        };
        if record.level >= cap {
            return None;
        }
        if skill::uses_beginner_sp(skill_id) {
            if self.beginner_sp() <= 0 {
                return None;
            }
        } else if self.stats.sp <= 0 {
            return None;
        } else {
            self.stats.sp -= 1;
        }
        record.level += 1;
        self.skills.set(record.clone());
        self.dirty = true;
        Some(record)
    }

    /// The max HP the v83 client shows and the player heals up to
    /// (BasicStat::SetFrom, 0x77ec9f): the base plus what the worn equipment
    /// adds, raised by Hyper Body's percent, at most 30000. `stats.max_hp`
    /// stays the base, which is what the MAXHP stat sends.
    pub fn effective_max_hp(&self) -> i32 {
        self.effective_max(self.stats.max_hp, |equip| equip.hp, BuffStat::MaxHp)
    }

    /// The max MP, as `effective_max_hp` works out max HP.
    pub fn effective_max_mp(&self) -> i32 {
        self.effective_max(self.stats.max_mp, |equip| equip.mp, BuffStat::MaxMp)
    }

    fn effective_max(&self, base: i16, gear: fn(&EquipStats) -> i16, buff: BuffStat) -> i32 {
        let gear: i32 = self.worn_equips().map(|equip| i32::from(gear(equip))).sum();
        let percent = self.buffs.value(buff).map_or(0, i32::from);
        ((i32::from(base) + gear) * (100 + percent) / 100).min(i32::from(MAX_HP_MP))
    }

    /// Bring HP and MP down to their max after it fell, e.g. when Hyper
    /// Body ends or equipment comes off (Cosmic's enforceMaxHpMp). Returns
    /// whether either changed.
    pub fn fit_hp_mp(&mut self) -> bool {
        let hp = i32::from(self.stats.hp).min(self.effective_max_hp());
        let mp = i32::from(self.stats.mp).min(self.effective_max_mp());
        if (hp, mp) == (i32::from(self.stats.hp), i32::from(self.stats.mp)) {
            return false;
        }
        (self.stats.hp, self.stats.mp) = (hp as i16, mp as i16);
        self.dirty = true;
        true
    }

    /// Change HP by `delta`, clamped to `0..=effective_max_hp()`. Returns
    /// the new HP.
    pub fn add_hp(&mut self, delta: i32) -> i16 {
        let hp = (i32::from(self.stats.hp) + delta).clamp(0, self.effective_max_hp());
        self.stats.hp = hp as i16;
        self.dirty = true;
        self.stats.hp
    }

    /// Recover `hp` from a potion or a skill, halved while Undead holds, as
    /// its notice says (and Cosmic's ZOMBIFY does). Returns the new HP.
    pub fn recover_hp(&mut self, hp: i32) -> i16 {
        if self.buffs.has(crate::buff::BuffStat::Undead) {
            self.add_hp(hp / 2)
        } else {
            self.add_hp(hp)
        }
    }

    /// Change MP by `delta`, clamped to `0..=effective_max_mp()`. Returns
    /// the new MP.
    pub fn add_mp(&mut self, delta: i32) -> i16 {
        let mp = (i32::from(self.stats.mp) + delta).clamp(0, self.effective_max_mp());
        self.stats.mp = mp as i16;
        self.dirty = true;
        self.stats.mp
    }

    /// Flag in-memory changes that have not been persisted yet. Mutators on
    /// this type call this themselves; use it after editing public fields.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Called after a successful save of a snapshot taken from this player.
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }
}

/// A job family's max HP/MP growth in v83: the least a level up and an AP
/// add, the slopes of the client's minimum max HP/MP tables (`FUN_008cd10a`,
/// `FUN_008cd369`) and its AP Reset dialog's gains (`0x8cc216`), plus a roll of up to
/// `HP_SPREAD` HP and `MP_SPREAD` MP on top, as servers of the time rolled.
struct Growth {
    level_up: (i16, i16),
    ap: (i16, i16),
}

const HP_SPREAD: i16 = 4;
const MP_SPREAD: i16 = 2;

impl Growth {
    fn level_up(&self, roll: &mut impl FnMut(i16) -> i16) -> (i16, i16) {
        Self::rolled(self.level_up, roll)
    }

    fn ap(&self, roll: &mut impl FnMut(i16) -> i16) -> (i16, i16) {
        Self::rolled(self.ap, roll)
    }

    fn rolled((hp, mp): (i16, i16), roll: &mut impl FnMut(i16) -> i16) -> (i16, i16) {
        let hp = hp + roll(HP_SPREAD).clamp(0, HP_SPREAD);
        (hp, mp + roll(MP_SPREAD).clamp(0, MP_SPREAD))
    }
}

/// `job`'s growth row. The client keys its tables by `job % 1000`, so
/// Cygnus Knights and Aran share the Explorers' rows; GMs take the
/// beginners'.
fn growth(job: i16) -> Growth {
    let (level_up, ap) = match job % 1000 / 100 {
        1 => ((24, 4), (20, 2)),
        2 => ((10, 22), (6, 18)),
        3 | 4 => ((20, 14), (16, 10)),
        5 => ((22, 18), (18, 14)),
        _ => ((12, 10), (8, 6)),
    };
    Growth { level_up, ap }
}

/// A first job's minimum level and the max HP/MP it adds: the middle of
/// the random ranges community servers use (e.g. warriors 201-250 HP).
fn first_job_rules(job: i16) -> Option<(u8, i16, i16)> {
    match job {
        100 => Some((10, 225, 0)),
        200 => Some((8, 0, 125)),
        300 | 400 => Some((10, 125, 38)),
        500 => Some((10, 175, 63)),
        _ => None,
    }
}

/// The max HP/MP the second job `job` adds. The client's least max HP and
/// MP (`ap_reset::min_hp`, `min_mp`) rise by 300 HP for a Fighter but by
/// 100 MP for a Page or Spearman, so they get MP where Fighters get HP.
fn second_job_bonus(job: i16) -> Option<(i16, i16)> {
    match job {
        120 | 130 => Some((0, 125)),
        _ => second_job_rules(job),
    }
}

/// The max HP/MP an Explorer second job adds, again the middle of the
/// community servers' ranges (e.g. warriors 300-350 HP), and each later
/// job adds again.
fn second_job_rules(job: i16) -> Option<(i16, i16)> {
    match job {
        110 | 120 | 130 => Some((325, 0)),
        210 | 220 | 230 => Some((0, 475)),
        310 | 320 | 410 | 420 | 510 | 520 => Some((325, 175)),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn test_stats() -> CharacterStats {
    CharacterStats {
        id: 7,
        name: "tester".to_string(),
        gender: 0,
        skin: 0,
        face: 20000,
        hair: 30030,
        level: 1,
        job: 0,
        str: 12,
        dex: 5,
        int: 4,
        luk: 4,
        hp: 50,
        max_hp: 50,
        mp: 5,
        max_mp: 5,
        ap: 0,
        sp: 0,
        exp: 0,
        gacha_exp: 0,
        fame: 0,
        map_id: 10000,
        spawn_point: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::equip_slot;

    #[test]
    fn the_scripts_saved_location_kinds_are_apart_and_below_the_invitations() {
        let kinds = [
            FREE_MARKET_RETURN,
            WORLD_TOUR_RETURN,
            FLORINA_RETURN,
            PYRAMID_RETURN,
            DOJO_RETURN,
            CARNIVAL_RETURN,
            EVENT_RETURN,
        ];
        for (i, kind) in kinds.iter().enumerate() {
            assert!(!kinds[..i].contains(kind));
            assert!((0..1000).contains(kind));
        }
    }

    #[test]
    fn cash_items_cover_regular_items_in_the_same_slot() {
        let look = AvatarLook::from_equipped(
            &test_stats(),
            [
                (equip_slot::TOP, 1040002),
                (equip_slot::SHOES, 1072001),
                (equip_slot::TOP - 100, 1042003),
                (equip_slot::CASH_WEAPON, 1702000),
            ],
        );
        assert_eq!(look.visible.get(&5), Some(&1042003));
        assert_eq!(look.visible.get(&7), Some(&1072001));
        assert_eq!(look.masked.get(&5), Some(&1040002));
        assert_eq!(look.cash_weapon, Some(1702000));
    }

    #[test]
    fn mutators_mark_the_player_dirty() {
        let mut player = Player::new(
            1,
            0,
            test_stats(),
            0,
            Inventory::default(),
            SkillBook::default(),
            Keymap::default(),
            QuestLog::default(),
        );
        assert!(!player.is_dirty());
        player.move_to(100000000, 2);
        assert!(player.is_dirty());
        assert_eq!(player.stats.spawn_point, 2);
        player.mark_saved();
        assert!(!player.is_dirty());
        player.set_spawn_point(2);
        assert!(!player.is_dirty());
        player.set_spawn_point(4);
        assert!(player.is_dirty());
    }

    /// Growth's least roll.
    fn floor(_spread: i16) -> i16 {
        0
    }

    fn player() -> Player {
        Player::new(
            1,
            0,
            test_stats(),
            0,
            Inventory::default(),
            SkillBook::default(),
            Keymap::default(),
            QuestLog::default(),
        )
    }

    #[test]
    fn a_wedding_ring_worn_by_one_of_a_couple_shows_their_ring() {
        let mut player = player();
        let id = player.stats.id;
        player.inventory.insert(
            InventoryType::Equipped,
            -12,
            Item::equipment(1112806, EquipStats::default()),
        );
        assert_eq!(player.marriage_ring(), None);
        player.marriage = Some(crate::marriage::MarriageRecord {
            id: 3,
            groom_id: 7,
            bride_id: id,
            status: crate::marriage::MARRIED,
            item_id: 2240002,
            groom_name: "Groom".into(),
            bride_name: "Bride".into(),
        });
        let ring = crate::marriage::MarriageRing {
            self_id: id,
            partner_id: 7,
            item_id: 1112807,
        };
        assert_eq!(player.marriage_ring(), Some(ring));
        assert_eq!(player.field_avatar(0, 0, 0).rings.marriage, Some(ring));
        player.inventory.unequip(-12, 1).unwrap();
        assert_eq!(player.marriage_ring(), None);
    }

    #[test]
    fn a_worn_crush_ring_of_a_pair_shows_in_the_look() {
        let mut player = player();
        let mut ring = Item::equipment(1112001, EquipStats::default());
        ring.cash_serial = Some(40);
        player.inventory.insert(InventoryType::Equipped, -112, ring);
        assert_eq!(player.ring_looks(), crate::ring::RingLooks::default());
        player.rings.push(crate::ring::CoupleRing {
            id: 40,
            partner_ring_id: 41,
            partner_id: 7,
            partner_name: "Partner".into(),
            item_id: 1112001,
        });
        let look = crate::ring::RingLook {
            ring_id: 40,
            partner_ring_id: 41,
            item_id: 1112001,
        };
        assert_eq!(player.field_avatar(0, 0, 0).rings.crush, Some(look));
        assert_eq!(player.ring_looks().friendship, None);
        player.inventory.unequip(-112, 1).unwrap();
        assert_eq!(player.ring_looks().crush, None);
    }

    #[test]
    fn a_linked_character_blesses_by_its_tens() {
        let mut player = player();
        player.link(Some(("Sib".into(), 57)));
        assert_eq!(player.linked_name.as_deref(), Some("Sib"));
        let record = player.skills.get(12).unwrap();
        assert_eq!((record.level, record.master_level), (5, 20));
        player.link(None);
        assert_eq!(player.linked_name, None);
        assert!(player.skills.get(12).is_none());

        player.stats.job = 1100;
        player.link(Some(("Sib".into(), 200)));
        assert_eq!(player.skills.level(10_000_012), 20);
    }

    #[test]
    fn equipment_and_hyper_body_raise_max_hp_and_mp() {
        let mut player = player();
        (player.stats.max_hp, player.stats.max_mp) = (1000, 500);
        let stats = EquipStats {
            hp: 200,
            mp: 100,
            ..EquipStats::default()
        };
        let top = Item::equipment(1040002, stats);
        player.inventory.insert(InventoryType::Equipped, -5, top);
        assert_eq!(
            (player.effective_max_hp(), player.effective_max_mp()),
            (1200, 600)
        );

        let now = std::time::Instant::now();
        let lasts = std::time::Duration::from_secs(60);
        let hp = crate::buff::Buff::new(1301007, BuffStat::MaxHp, 60, now, lasts);
        let mp = crate::buff::Buff::new(1301007, BuffStat::MaxMp, 40, now, lasts);
        player.buffs.give(hp);
        player.buffs.give(mp);
        assert_eq!(
            (player.effective_max_hp(), player.effective_max_mp()),
            (1920, 840)
        );
        assert_eq!(player.add_hp(5000), 1920);
        assert_eq!(player.add_mp(5000), 840);

        // Ending it takes HP and MP back down to the max.
        player.buffs.cancel(1301007);
        assert!(player.fit_hp_mp());
        assert_eq!((player.stats.hp, player.stats.mp), (1200, 600));
        assert!(!player.fit_hp_mp());

        // Never more than 30000.
        player.stats.max_hp = 29_900;
        assert_eq!(player.effective_max_hp(), 30_000);
    }

    #[test]
    fn a_pendant_in_the_second_slot_counts_only_while_the_slot_is_open() {
        use std::time::{Duration, SystemTime};
        let mut player = player();
        player.stats.max_hp = 1000;
        let stats = EquipStats {
            hp: 100,
            ..EquipStats::default()
        };
        let pendant = |item_id| Item::equipment(item_id, stats.clone());
        (player.inventory).insert(
            InventoryType::Equipped,
            equip_slot::PENDANT,
            pendant(1122000),
        );
        (player.inventory).insert(
            InventoryType::Equipped,
            equip_slot::PENDANT2,
            pendant(1122001),
        );

        player.extra_pendant = Some(SystemTime::now() + Duration::from_secs(60));
        assert_eq!(player.effective_max_hp(), 1200);

        // Run out: it stays worn but adds nothing; the first slot still does.
        player.extra_pendant = Some(SystemTime::now() - Duration::from_secs(60));
        assert_eq!(player.effective_max_hp(), 1100);
        player.extra_pendant = None;
        assert_eq!(player.effective_max_hp(), 1100);
        assert!(player
            .inventory
            .get(InventoryType::Equipped, equip_slot::PENDANT2)
            .is_some());
    }

    #[test]
    fn exp_levels_up_through_several_levels() {
        let mut player = player();
        player.stats.max_hp = 50;
        player.stats.hp = 40;
        // Rain's seven quizzes pay 62 EXP: level 1 -> 3 with 13 left over.
        assert_eq!(player.gain_exp(62, (0, 0), floor), 2);
        assert_eq!(
            (player.stats.level, player.stats.exp, player.stats.ap),
            (3, 13, 10)
        );
        // Two beginner levels at the least: +24 max HP, +20 max MP, no
        // refill.
        assert_eq!((player.stats.max_hp, player.stats.hp), (74, 40));
        assert_eq!((player.stats.max_mp, player.stats.mp), (25, 5));
        assert_eq!(player.gain_exp(0, (0, 0), floor), 0);
        assert!(player.is_dirty());
    }

    #[test]
    fn levels_past_beginner_grant_sp() {
        let mut player = player();
        player.gain_exp(62, (0, 0), floor);
        assert_eq!(player.stats.sp, 0);
        player.stats.job = 200;
        player.gain_exp(57 + 92 - 13, (0, 0), floor);
        assert_eq!((player.stats.level, player.stats.sp), (5, 6));
    }

    #[test]
    fn ap_raises_one_stat_until_it_runs_out_or_caps() {
        let mut player = player();
        player.stats.ap = 3;
        assert!(player.spend_ap(ApStat::Str, (0, 0), floor));
        assert_eq!((player.stats.str, player.stats.ap), (13, 2));
        assert!(player.is_dirty());
        player.stats.job = 200;
        assert!(player.spend_ap(ApStat::MaxMp, (0, 0), floor));
        assert_eq!((player.stats.max_mp, player.stats.ap), (23, 1));
        player.stats.luk = MAX_BASE_STAT;
        assert!(!player.spend_ap(ApStat::Luk, (0, 0), floor));
        player.stats.max_hp = MAX_HP_MP - 1;
        assert!(player.spend_ap(ApStat::MaxHp, (0, 0), floor));
        assert_eq!((player.stats.max_hp, player.stats.ap), (MAX_HP_MP, 0));
        assert!(!player.spend_ap(ApStat::Dex, (0, 0), floor));
        assert_eq!(player.stats.dex, 5);
    }

    #[test]
    fn skills_add_to_max_hp_and_mp_growth() {
        // A warrior with Improving MaxHP Increase at 10: 24 + 40 HP a level
        // at the least, 20 + 30 for an AP.
        let mut player = player();
        player.stats.job = 100;
        player.stats.level = 10;
        player.stats.ap = 1;
        player.stats.max_hp = 500;
        player.stats.max_mp = 100;
        player.gain_exp(
            crate::exp::to_next_level(10).unwrap() as i32,
            (40, 0),
            floor,
        );
        assert_eq!((player.stats.max_hp, player.stats.max_mp), (564, 104));
        assert!(player.spend_ap(ApStat::MaxHp, (30, 0), floor));
        assert_eq!(player.stats.max_hp, 614);
        // Strength takes no bonus.
        assert!(player.spend_ap(ApStat::Str, (30, 0), floor));
        assert_eq!(player.stats.str, 13);
    }

    #[test]
    fn growth_rolls_up_to_4_hp_and_2_mp_over_the_clients_floor() {
        // (job, level-up floor, AP floor) for each family.
        let rows = [
            (0, (12, 10), (8, 6)),
            (100, (24, 4), (20, 2)),
            (232, (10, 22), (6, 18)),
            (312, (20, 14), (16, 10)),
            (422, (20, 14), (16, 10)),
            (520, (22, 18), (18, 14)),
            // Cygnus Knights and Aran use the Explorers' rows, GMs the
            // beginners'.
            (1210, (10, 22), (6, 18)),
            (2100, (24, 4), (20, 2)),
            (2000, (12, 10), (8, 6)),
            (910, (12, 10), (8, 6)),
        ];
        let top = |spread| spread;
        for (job, level_up, ap) in rows {
            let gains = |roll: fn(i16) -> i16| {
                let mut player = player();
                player.stats.job = job;
                player.stats.ap = 2;
                let start = (player.stats.max_hp, player.stats.max_mp);
                player.gain_exp(15, (0, 0), roll);
                let levelled = (player.stats.max_hp, player.stats.max_mp);
                player.spend_ap(ApStat::MaxHp, (0, 0), roll);
                player.spend_ap(ApStat::MaxMp, (0, 0), roll);
                (
                    (levelled.0 - start.0, levelled.1 - start.1),
                    (
                        player.stats.max_hp - levelled.0,
                        player.stats.max_mp - levelled.1,
                    ),
                )
            };
            assert_eq!(gains(floor), (level_up, ap), "job {job}");
            let spread = |(hp, mp): (i16, i16)| (hp + 4, mp + 2);
            assert_eq!(gains(top), (spread(level_up), spread(ap)), "job {job}");
        }
    }

    #[test]
    fn level_ups_stop_at_30000_max_hp_and_mp() {
        let mut player = player();
        player.stats.max_hp = MAX_HP_MP - 10;
        player.stats.max_mp = MAX_HP_MP;
        player.gain_exp(15, (40, 50), floor);
        assert_eq!(
            (player.stats.max_hp, player.stats.max_mp),
            (MAX_HP_MP, MAX_HP_MP)
        );
    }

    #[test]
    fn beginners_take_a_first_job_with_its_level_and_stat() {
        let mut player = player();
        player.stats.level = 9;
        player.stats.int = 20;
        assert!(player.can_take_first_job(200));
        player.stats.str = 35;
        assert!(!player.can_take_first_job(100), "warriors need level 10");
        player.stats.level = 10;
        assert!(player.can_take_first_job(100));
        assert!(!player.can_take_first_job(300), "bowmen need 25 DEX");
        assert!(!player.can_take_first_job(110), "not a first job");

        player.take_first_job(200);
        assert_eq!(player.stats.job, 200);
        assert_eq!(player.stats.sp, 7, "1 SP and 3 for each level past 8");
        assert_eq!((player.stats.max_hp, player.stats.max_mp), (50, 130));
        assert_eq!((player.stats.hp, player.stats.mp), (50, 5));
        assert!(player.is_dirty());
        assert!(!player.can_take_first_job(100), "only beginners advance");
    }

    #[test]
    fn first_jobs_take_a_second_job_of_their_line_at_30() {
        let mut player = player();
        (player.stats.job, player.stats.level, player.stats.sp) = (300, 29, 2);
        assert!(!player.can_take_second_job(310), "level 30");
        player.stats.level = 30;
        assert!(player.can_take_second_job(320));
        assert!(!player.can_take_second_job(410), "another line");
        assert!(!player.can_take_second_job(330), "no such job");
        assert!(!player.can_take_second_job(300), "not a second job");

        player.take_second_job(320);
        assert_eq!(player.stats.job, 320);
        assert_eq!(player.stats.sp, 3);
        assert_eq!((player.stats.max_hp, player.stats.max_mp), (375, 180));
        assert!(!player.can_take_second_job(310), "only once");
    }

    #[test]
    fn a_second_job_keeps_above_the_clients_least_hp_and_mp() {
        for (first, second) in [
            (100, 110),
            (100, 120),
            (100, 130),
            (200, 210),
            (200, 220),
            (200, 230),
            (300, 310),
            (300, 320),
            (400, 410),
            (400, 420),
            (500, 510),
            (500, 520),
        ] {
            let mut player = player();
            player.stats.level = 30;
            player.stats.job = first;
            player.stats.max_hp = crate::ap_reset::min_hp(first, 30) as i16;
            player.stats.max_mp = crate::ap_reset::min_mp(first, 30) as i16;
            player.take_second_job(second);
            let (hp, mp) = (
                i32::from(player.stats.max_hp) - crate::ap_reset::min_hp(second, 30),
                i32::from(player.stats.max_mp) - crate::ap_reset::min_mp(second, 30),
            );
            // The middle of a 50-wide range above the floor's rise.
            assert!([0, 25].contains(&hp), "{second}: HP {hp}");
            assert!([0, 25].contains(&mp), "{second}: MP {mp}");
        }
    }

    #[test]
    fn second_jobs_take_the_next_job_at_70_with_their_sp_spent() {
        let mut player = player();
        (
            player.stats.job,
            player.stats.level,
            player.stats.sp,
            player.stats.ap,
        ) = (120, 69, 0, 0);
        assert!(!player.can_take_third_job(121), "level 70");
        player.stats.level = 75;
        player.stats.sp = 16;
        assert!(!player.can_take_third_job(121), "SP left past 3 a level");
        player.stats.sp = 15;
        assert!(player.can_take_third_job(121));
        assert!(!player.can_take_third_job(111), "another job");
        assert!(!player.can_take_third_job(122), "not a third job");
        assert!(!player.can_take_third_job(120), "not a third job");

        let max = (player.stats.max_hp, player.stats.max_mp);
        player.take_third_job(121);
        assert_eq!(player.stats.job, 121);
        assert_eq!((player.stats.sp, player.stats.ap), (16, 5));
        assert_eq!(
            (player.stats.max_hp, player.stats.max_mp),
            (max.0 + 325, max.1)
        );
        assert!(!player.can_take_third_job(121), "only once");

        for job in [111, 131, 211, 221, 231, 311, 321, 411, 421, 511, 521] {
            (player.stats.job, player.stats.sp) = (job - 1, 0);
            assert!(player.can_take_third_job(job), "{job}");
        }
    }

    #[test]
    fn third_jobs_take_the_next_job_at_120() {
        let mut player = player();
        (
            player.stats.job,
            player.stats.level,
            player.stats.sp,
            player.stats.ap,
        ) = (211, 119, 7, 0);
        assert!(!player.can_take_fourth_job(212), "level 120");
        player.stats.level = 120;
        assert!(player.can_take_fourth_job(212), "SP may be left");
        assert!(!player.can_take_fourth_job(222), "another job");
        assert!(!player.can_take_fourth_job(211), "not a fourth job");

        let max = (player.stats.max_hp, player.stats.max_mp);
        player.take_fourth_job(212);
        assert_eq!(player.stats.job, 212);
        assert_eq!((player.stats.sp, player.stats.ap), (10, 5));
        assert_eq!(
            (player.stats.max_hp, player.stats.max_mp),
            (max.0, max.1 + 475)
        );
        assert!(!player.can_take_fourth_job(212), "only once");

        for job in [112, 122, 132, 222, 232, 312, 322, 412, 422, 512, 522] {
            player.stats.job = job - 1;
            assert!(player.can_take_fourth_job(job), "{job}");
        }
    }

    #[test]
    fn beginner_skills_share_one_sp_per_level_up_to_six() {
        let mut player = player();
        assert_eq!(player.raise_skill(1000, 3), None);
        player.stats.level = 3;
        assert_eq!(player.raise_skill(1000, 3).unwrap().level, 1);
        assert_eq!(player.raise_skill(1001, 3).unwrap().level, 1);
        assert_eq!(player.raise_skill(1002, 3), None);
        assert_eq!(player.stats.sp, 0);
        player.stats.level = 30;
        assert_eq!(player.beginner_sp(), 4);
        player.raise_skill(1000, 3);
        player.raise_skill(1000, 3);
        assert_eq!(player.raise_skill(1000, 3), None);
        // A magician keeps the pool for the skills it never raised.
        player.stats.job = 200;
        player.stats.sp = 1;
        assert_eq!(player.raise_skill(1002, 3).unwrap().level, 1);
        assert_eq!((player.beginner_sp(), player.stats.sp), (1, 1));
    }

    #[test]
    fn skills_cost_sp_and_stop_at_their_cap() {
        let mut player = player();
        player.stats.job = 200;
        player.stats.sp = 2;
        assert_eq!(player.raise_skill(2001002, 1).unwrap().level, 1);
        assert_eq!(player.raise_skill(2001002, 1), None);
        assert_eq!(player.stats.sp, 1);
        assert_eq!(player.raise_skill(2000000, 10).unwrap().level, 1);
        assert_eq!(player.raise_skill(2000000, 10), None);
        assert_eq!(player.skills.level(2000000), 1);
    }

    #[test]
    fn fourth_job_skills_stop_at_their_master_level() {
        let mut player = player();
        player.stats.job = 212;
        player.stats.sp = 5;
        assert_eq!(player.raise_skill(2121000, 30), None);
        player.skills.set(SkillRecord {
            skill_id: 2121000,
            level: 0,
            master_level: 1,
            expiration: None,
        });
        assert_eq!(player.raise_skill(2121000, 30).unwrap().master_level, 1);
        assert_eq!(player.raise_skill(2121000, 30), None);
        assert_eq!(player.stats.sp, 4);
    }

    #[test]
    fn hp_and_mp_stay_within_bounds() {
        let mut player = player();
        player.stats.max_hp = 50;
        player.stats.hp = 25;
        assert_eq!(player.add_hp(30), 50);
        assert_eq!(player.add_hp(-80), 0);
        player.stats.max_mp = 5;
        assert_eq!(player.add_mp(10), 5);
    }
}
