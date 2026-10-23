//! Native map and portal scripts, keyed by the script names in `Map.nx`
//! (`info/onUserEnter`, portal `script`), and the effects all scripts
//! return. Lua is deferred; returning effects keeps the shape a scripting
//! runtime would have. Unknown names do nothing.

use crate::content;
use crate::conti::Portal;
use crate::npc_scripts;
use game::character::{FREE_MARKET_RETURN, PYRAMID_RETURN};
use game::{BuffStat, InventoryType, Player, QuestState};
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
/// Where the `obstacle` portal is, and the scripted items that open the
/// way past it for good (`killarmush`) and over the castle wall
/// (`removethorns`, quest 2324).
const DEEP_INSIDE: i32 = 106020300;
const KILLER_MUSHROOM_SPORE: i32 = 2430014;
const THORN_REMOVER: i32 = 2430015;
const THORN_REMOVER_QUEST: i16 = 2324;
/// Set once a Killer Mushroom Spore opened the `obstacle` portal (Cosmic
/// starts the unknown quest 100202).
const OBSTACLE_OPENED: i16 = 32500;
/// How near a scripted item's portal the player must stand.
const ITEM_REACH: f64 = 210.0;
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
/// Portals that just let the player through, and where to (Cosmic's).
const PLAIN_WARPS: [(&str, i32, Portal); 16] = [
    // Two of the Bottom of the Temple's ways back up the stairway; the
    // other (`OutPerrion_1`) says it's a shortcut.
    ("OutPerrion_2", UNDERGROUND_TEMPLE_STAIRS, Portal::Id(0)),
    // Leafre's Valley of the Antelope, and the forests' boss areas.
    ("minar_job4", 240010501, Portal::Named("out00")),
    ("gryphius", 240020101, Portal::Named("out00")),
    ("mayong", 240020401, Portal::Named("out00")),
    ("Pianus", 230040420, Portal::Named("out00")),
    // Out of the Horntail cave's entrance, back to the Peak of the Big
    // Nest.
    ("hontale_morph2", 240040600, Portal::Id(4)),
    ("eliza_Garden", 920020000, Portal::Id(2)),
    ("catPriest_map", 925000000, Portal::Id(2)),
    // Lith Harbor's Info Shop. Cosmic also brings out an Aran quest's
    // monster (21733) here; that isn't in.
    ("enterInfo", 104000004, Portal::Id(1)),
    // Ludibrium's Time Control Room into Ellin Forest, saying so.
    ("move_elin", 300000100, Portal::Named("out00")),
    // The Tower of Goddess's hub into its six rooms (Orbis's party quest).
    ("party3_room1", 920010200, Portal::Id(13)),
    ("party3_room2", 920010300, Portal::Id(1)),
    ("party3_room3", 920010400, Portal::Id(8)),
    ("party3_room4", 920010500, Portal::Id(3)),
    ("party3_room5", 920010600, Portal::Id(17)),
    ("party3_room6", 920010700, Portal::Id(23)),
];
/// Where each room off the Tower of Goddess's hub lets out on it (Cosmic
/// `party3_roomout`): the room, and the hub's portal. The storage is the
/// seventh; anywhere else lets out at the hub's portal 0, as in Cosmic.
const OPQ_ROOMS_OUT: [(i32, u8); 7] = [
    (920010200, 4),
    (920010300, 12),
    (920010400, 5),
    (920010500, 13),
    (920010600, 15),
    (920010700, 14),
    (920011000, 16),
];
/// The climb pads of the Tower of Goddess's lounge and tall room, which
/// the client actor answers from the field's draw (`events::climb`).
pub const OPQ_LOUNGE_CLIMB: &str = "party3_r4pt";
pub const OPQ_TOWER_CLIMB: &str = "party3_r6pt";
/// The Orbis party quest's way out of the garden, back to the hub.
pub const OPQ_GARDEN_OUT: &str = "party3_gardenin";
/// The hub's door to the hidden storage (`in06`), open once the event is
/// won.
pub const OPQ_STORAGE_IN: &str = "party3_room8";
/// The Dangerous Forest's ways out: back to Rien (`enterRienFirst`) and on
/// to the second forest (`rienTutor7`). An Aran beginner who hasn't finished
/// the tutorial (21014) yet is put at Rien's start and kept from going on.
const RIEN: i32 = 140000000;
const SECOND_DANGEROUS_FOREST: i32 = 140010100;
const RIEN_TUTORIAL: i16 = 21014;
/// The Snow Island training grounds Puo's lessons (21701 to 21703) send an
/// Aran to (`enterGym`), and the Mirror Cave, open on The Five Heroes'
/// quests (21201, 21302) (`rienCaveEnter`).
const TRAINING_GROUNDS: [(i16, i32); 3] =
    [(21701, 914010000), (21702, 914010100), (21703, 914010200)];
const MIRROR_CAVE: i32 = 140030000;
/// Fox Ridge's way back to Tiger Ridge (`foxLaidy_map`) leads into the
/// Gloomy Forest instead for a player on quest 3647 who holds the Old Fox's
/// Tail.
const TIGER_RIDGE: i32 = 222010200;
const GLOOMY_FOREST: i32 = 922220000;
const OLD_FOX_TAIL: i32 = 4031793;
/// The Mushroom Castle's Security Room (`go_secretroom`): its key opens it
/// while quest 2335 is taken, which the door completes. (The quest isn't
/// supported yet, so only players who finished it some other way get in.)
const SECURITY_ROOM: i32 = 106021001;
const SECURITY_ROOM_KEY: i32 = 4032405;
const SECURITY_ROOM_QUEST: i16 = 2335;
/// Ereve's Training Forests' ways on (`enterDisguise1` to `5`), into the
/// next forest, or while the Master of Disguise quests (20301 to 20305)
/// are taken, into the forest he hides in for each job's Knight, which one
/// player searches at a time; his proof (4032101 on) shows it's done.
const DISGUISE_WAYS: [(i32, &str); 5] = [
    (130010010, "out00"),
    (130010020, "out00"),
    (130010110, "out00"),
    (130010120, "out00"),
    (130020000, "east00"),
];
const DISGUISE_FORESTS: i32 = 108010600;
const DISGUISE_PROOF: i32 = 4032101;
/// Ariant's Treasure Room of Queen (`skyrom`), which one player at a time
/// searches for the jewel quest 3935 is after.
const QUEENS_TREASURE_ROOM: i32 = 926000010;
const QUEENS_JEWEL: i32 = 4031574;
/// The job instructors' rooms, whose `tutorial` portal (`tutorialNPC`) has
/// a beginner up to level 10 hear their instructor out.
const TUTORIAL_INSTRUCTORS: [(i32, i32); 5] = [
    (101000003, 1032001),
    (100000201, 1012100),
    (102000003, 1022000),
    (103000003, 1052001),
    (120000101, 1090000),
];
/// Leafre's Time Gate, beside its `TD_neo` portal (`TD_chat_enter`).
const TIME_GATE: i32 = 2083006;
/// The Cave of Evil Eye III's back way (`enterDollcave`), open once the
/// Puppeteer is beaten.
const DOLL_CAVE_WAY_BACK: (i32, u8) = (105040201, 2);
/// The fourth job's rooms one player at a time takes on alone, which start
/// afresh once they're left (`npc_scripts::resets_when_empty`): the Fire
/// Hawk's and Ice Eagle's nests by El Nath (`s4firehawk`, `s4iceeagle`),
/// Ludibrium's Tornado room (`s4tornado_enter`), Aqua Road's resurrection
/// mound (`s4resurrection`), Leafre's nest of the eggs (`s4nest`), the
/// Hitman's hideout by Kerning City (`s4hitman`) and the Berserk room in
/// the Sleepy Dungeon (`s4berserk`).
const FIREHAWK_NEST: i32 = 921100200;
const ICE_EAGLE_NEST: i32 = 921100210;
const TORNADO_ROOM: i32 = 922020200;
const RESURRECTION_ROOM: i32 = 923000100;
const EGG_NEST: i32 = 924000100;
const HITMAN_HIDEOUT: i32 = 910200000;
const BERSERK_ROOM: i32 = 910500200;
pub const SOLO_MAPS: [i32; 7] = [
    FIREHAWK_NEST,
    ICE_EAGLE_NEST,
    TORNADO_ROOM,
    RESURRECTION_ROOM,
    EGG_NEST,
    HITMAN_HIDEOUT,
    BERSERK_ROOM,
];
/// The key that opens the Berserk room.
const BERSERK_KEY: i32 = 4031475;
/// The Cold Flame, which opens the way to the resurrection mound.
const COLD_FLAME: i32 = 4001108;
const TORNADO_PROOF: i32 = 4001110;
/// The shortcut between the Field Up North of Ellinia and Leafre's Minar
/// Forest (`minar_elli`), which spends a Magic Seed each way.
const FIELD_NORTH_OF_ELLINIA: i32 = 101010000;
const MINAR_FOREST_WEST_BORDER: i32 = 240010100;
const MAGIC_SEED: i32 = 4031346;
/// The Nautilus's Navigation Room and Omega Sector's Command Center
/// (`enter_earth00`, `enter_earth01`), linked for holders of a Warp Card.
const NAVIGATION_ROOM: i32 = 120000101;
const OMEGA_COMMAND_CENTER: i32 = 221000300;
const WARP_CARD: i32 = 4031890;
/// Ariant's palace rooms (`ariant_queens`) turn out anyone not disguised by
/// Tigun's potion, whose morph is listed as a buff of the item's negative
/// id.
const ARIANT_CASTLE: i32 = 260000300;
const TIGUNS_DISGUISE: i32 = 2210005;
/// Nett's Pyramid's lobby (`nets_in` from Sahel 3's `piramid00`, `nets_out`
/// back to where the player came from). The party quest itself isn't in.
const PYRAMID_DUNES: i32 = 926010000;
const SAHEL_3: i32 = 260020500;
/// The original Maple Island's instruction balloons (`advice00` to
/// `advice09`, `adviceMap`): a text, and the balloon's width and height
/// (Cosmic's).
const ADVICE: [(&str, &str, i16, i16); 11] = [
    ("advice00", keys::ADVICE_MOVE, 250, 5),
    ("advice01", keys::ADVICE_HEENA, 100, 5),
    ("advice02", keys::ADVICE_JUMP, 100, 5),
    ("advice03", keys::ADVICE_CLIMB, 350, 5),
    ("advice04", keys::ADVICE_SERA, 100, 5),
    ("advice05", keys::ADVICE_QUEST_WINDOW, 250, 5),
    ("advice06", keys::ADVICE_PORTAL, 230, 5),
    ("advice07", keys::ADVICE_WORLD_MAP, 350, 5),
    ("advice08", keys::ADVICE_STATS, 350, 5),
    ("advice09", keys::ADVICE_JUMP_DOWN, 450, 6),
    ("adviceMap", keys::ADVICE_PORTAL, 230, 5),
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
    /// Open an NPC's conversation without a click, as a portal does; the
    /// NPC needn't be on the map. Not over a conversation already open.
    OpenNpc(i32),
    SetHp(i16),
    GiveItem {
        item_id: i32,
        quantity: i16,
    },
    /// One piece of equipment with its stats varied as a drop's are
    /// (`items::give_dropped`), as Mos forges with a stimulator.
    GiveVariedItem(i32),
    /// Trade a Gachapon Ticket for the prize rolled when the machine in
    /// `town` was opened (`gachapon::roll`). Nothing is applied if the
    /// ticket is gone or the prize doesn't fit; the world hears of an
    /// `announced` prize.
    Gachapon {
        item_id: i32,
        quantity: i16,
        town: &'static str,
        announced: bool,
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
    /// Set a started quest's own record to `count`, toward its completion
    /// (`QuestTemplate::end_record`); nothing is applied unless it is started.
    QuestRecord {
        quest_id: i16,
        count: u16,
    },
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
    /// Book the player's wedding at `venue` with its ticket, premium or
    /// not, once their partner beside them has room for the invitations
    /// (Nicole, Bonnie). The assistant then says how it went, so nothing
    /// may follow it.
    BookWedding {
        venue: game::wedding::Venue,
        premium: bool,
    },
    /// Trade a Wedding Invitation Ticket for more invitations to the
    /// couple's wedding booked at `venue`. The assistant then says how it
    /// went, so nothing may follow it.
    ExtraInvitations {
        venue: game::wedding::Venue,
    },
    /// Begin the player's booked wedding at `venue` with their partner
    /// beside them (report b071). The starter says why it didn't begin, so
    /// nothing may follow it.
    StartWedding {
        venue: game::wedding::Venue,
    },
    /// The player takes their vows at their wedding's altar; the second of
    /// the couple to do so marries them. The priest then says how it went,
    /// so nothing may follow it.
    Vows,
    /// End the player's wedding, once married: everyone goes out, or on
    /// to the party room if it is premium.
    EndWedding,
    /// Go into the wedding at `venue` the player is invited to, while its
    /// couple gather. The assistant says why not, so nothing may follow it.
    EnterWedding {
        venue: game::wedding::Venue,
    },
    /// A guest leaves the wedding they are in for its exit.
    LeaveWedding,
    /// Open the couple's wishlist input window (Travis, Debbie); what they
    /// send is checked when it comes.
    OpenWishlist,
    /// Open the give window on the wishlist of the groom, or of the
    /// `bride`, of the wedding the player is in (Pila). She then says why
    /// not, so nothing may follow it.
    OpenRegistry {
        bride: bool,
    },
    /// Open the receive window on the player's gifts (Pila).
    OpenGifts,
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
    /// Nothing is applied while a monster is left on the player's map; the
    /// script says its `refused` text instead.
    RequireNoMobs,
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
    /// Bring out `mob_id` on the ground below `at` in the player's map,
    /// unless one is out already. It never respawns.
    BringOutMob {
        mob_id: i32,
        at: (i16, i16),
    },
    /// Put the reactors named `name` on the player's map in `state`, shown
    /// to everyone there but without the events that lead to it, as
    /// Cosmic's `forceHitReactor` (Eak bringing out a stone box). Those
    /// already in it or past it stay as they are.
    SetReactorState {
        name: &'static str,
        state: u8,
    },
    /// Count `map_id` toward an explorer medal (`crate::medals`), starting
    /// it on the first.
    Explore {
        medal: i16,
        map_id: i32,
    },
    /// Take the player's party into its own copy of an event's maps
    /// (`events::Event::team`). Nothing is applied unless the party still
    /// may go; the script says its `refused` text instead.
    StartEvent(crate::events::EventKind),
    /// Tell the player's event something happened, once the effects before
    /// it have applied. Scripts end on it.
    EventCommand(crate::events::EventCommand),
    /// Take the player, or the party they lead with the members on their
    /// map, to `portal` on the first free copy of a mini-dungeon
    /// (`dungeons::Dungeon::copies`). Nothing after it is applied if they
    /// may not go; the runtime says why in pink.
    EnterDungeon {
        first: i32,
        count: i32,
        portal: u8,
    },
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ScriptOutcome {
    pub effects: Vec<Effect>,
    /// Don't run this portal's script again this session.
    pub block: bool,
    /// Said in pink when a `RequireEmptyMap` or `RequireNoMobs` keeps the
    /// player back.
    pub occupied: Option<String>,
}

/// The theme banners (`Effect.img/temaD/enter`) of the maps whose
/// `TD_NC_title` or `TD_MC_title` script shows one. Neither script is in
/// Cosmic; each banner goes to the map it is named after, the six Neo City
/// ones in the order of the maps' years.
const THEME_BANNERS: [(i32, &str); 8] = [
    (240070000, "teraForest"),
    (240070100, "neoCity1"),
    (240070200, "neoCity2"),
    (240070300, "neoCity3"),
    (240070400, "neoCity4"),
    (240070500, "neoCity5"),
    (240070600, "neoCity6"),
    (106020000, "mushCatle"),
];

const LITH_HARBOR: i32 = 104000000;
/// Where the Disciples of the Black Magician hide (quest 2175), and the
/// one the player is out to find.
const DISCIPLES_HIDEOUT: i32 = 100000006;
const DISCIPLES_QUEST: i16 = 2175;
const DISCIPLE: i32 = 9300156;

/// The map `player` just entered runs its `onUserEnter` script. `None` if
/// the script is unknown.
pub fn map_enter(script: &str, player: &Player) -> Option<Vec<Effect>> {
    let map_id = player.stats.map_id;
    match script {
        "TD_NC_title" | "TD_MC_title" => Some(
            THEME_BANNERS
                .iter()
                .filter(|&&(map, _)| map == map_id)
                .map(|(_, banner)| Effect::FieldEffect(format!("temaD/enter/{banner}")))
                .collect(),
        ),
        // The map counts toward its region's explorer medal; Lith Harbor
        // also shows its banner, and the Disciple shows himself to whoever
        // hunts him.
        "explorationPoint" => {
            let mut effects = match map_id {
                LITH_HARBOR => vec![Effect::FieldEffect(format!("maplemap/enter/{map_id}"))],
                DISCIPLES_HIDEOUT
                    if player.quests.state(DISCIPLES_QUEST) == QuestState::Started =>
                {
                    vec![Effect::BringOutMob {
                        mob_id: DISCIPLE,
                        at: (-1027, 216),
                    }]
                }
                _ => Vec::new(),
            };
            effects.extend(
                crate::medals::region(map_id)
                    .and_then(|medal| crate::medals::explore(medal, map_id, player)),
            );
            Some(effects)
        }
        // go<map id>: the map's name banner. HeavenMS also unlocks the UI
        // here, which HeavenClient never locks.
        _ => {
            let map = script.strip_prefix("go")?;
            if map.is_empty() || !map.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            Some(vec![Effect::FieldEffect(format!("maplemap/enter/{map}"))])
        }
    }
}

/// What the scripted item with `script` (243xxxx `spec/script`) does when
/// `player`, standing `at`, uses it. Each works near one portal of one map
/// and takes every one the player holds; elsewhere nothing happens. `None`
/// if the script is unknown.
pub fn item(script: &str, player: &Player, at: (i16, i16)) -> Option<Vec<Effect>> {
    let near = |map_id: i32, portal: &str| {
        player.stats.map_id == map_id
            && crate::game_data::get()
                .ok()
                .and_then(|data| data.field(map_id))
                .and_then(|field| field.portal_by_name(portal))
                .is_some_and(|portal| {
                    let dx = f64::from(portal.x) - f64::from(at.0);
                    let dy = f64::from(portal.y) - f64::from(at.1);
                    dx.hypot(dy) < ITEM_REACH
                })
    };
    let all = |item_id: i32| Effect::TakeItem {
        item_id,
        quantity: i16::try_from(player.inventory.count(item_id)).unwrap_or(i16::MAX),
    };
    let pink = |key: &str| Effect::Pink(content::text().get(key, &[]));
    match script {
        // The Killer Mushroom Spore (2430014), beside the poisonous
        // mushrooms: the `obstacle` portal lets the player by from now on.
        "killarmush" if near(DEEP_INSIDE, "obstacle") => {
            let mut effects = Vec::new();
            if player.quests.state(OBSTACLE_OPENED) == QuestState::NotStarted {
                effects.push(Effect::Mark(OBSTACLE_OPENED));
            }
            effects.extend([all(KILLER_MUSHROOM_SPORE), pink(keys::MC_SPORE_USED)]);
            Some(effects)
        }
        // The Thorn Remover (2430015), beside the castle wall's thorns
        // while quest 2324 is under way: its record turns to "1", which
        // its `Check/1 infoex` asks for, so the Secretary of Magic
        // (1300004) can complete it; that opens `gotocastle`.
        "removethorns"
            if player.quests.state(THORN_REMOVER_QUEST) == QuestState::Started
                && near(SPLIT_ROAD, "right00") =>
        {
            Some(vec![
                Effect::QuestRecord {
                    quest_id: THORN_REMOVER_QUEST,
                    count: 1,
                },
                all(THORN_REMOVER),
                pink(keys::MC_THORNS_REMOVED),
            ])
        }
        "killarmush" | "removethorns" => Some(Vec::new()),
        _ => None,
    }
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
                occupied: None,
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
    let aran_in_tutorial =
        || player.stats.job == ARAN_BEGINNER && quest(RIEN_TUTORIAL) != QuestState::Completed;
    let outcome = match script {
        "infoMinimap" => hint(quest(1031) == QuestState::Started, 25),
        "infoAttack" => hint(quest(1035) == QuestState::Started, 20),
        "infoPickup" => hint(quest(1035) == QuestState::Started, 21),
        "infoSkill" => hint(quest(1035) == QuestState::Completed, 23),
        "infoWorldmap" => hint(true, 26),
        "infoReactor" if quest(1008) == QuestState::Completed => hint(true, 22),
        "infoReactor" => hint(quest(1020) == QuestState::Completed, 27),
        // The highest places count toward One Who's Touched the Sky; the
        // portal goes nowhere.
        "highposition" => ScriptOutcome {
            effects: (crate::medals::explore(
                crate::medals::TOUCHED_THE_SKY,
                player.stats.map_id,
                player,
            ))
            .into_iter()
            .collect(),
            block: false,
            occupied: None,
        },
        "tutoChatNPC" => ScriptOutcome {
            effects: vec![Effect::OfferTutorialSkip],
            block: true,
            occupied: None,
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
            occupied: None,
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
            occupied: None,
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
            occupied: None,
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
        // Opened for good by a Killer Mushroom Spore (`item`).
        "obstacle" if quest(OBSTACLE_OPENED) != QuestState::NotStarted => through(
            Effect::WarpToPortal(SPLIT_ROAD, named(SPLIT_ROAD, "left00")),
        ),
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
            occupied: None,
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
                        occupied: None,
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
        // The gate to Zakum: a chief's approval, the trials, and an Eye of
        // Fire. Upstream also keeps it shut while the gate reactor is
        // closed after a win; nothing closes it yet.
        "Zakum05" if !npc_scripts::zakum_approved(player) => refused(keys::ZAKUM_GATE_UNAPPROVED),
        "Zakum05" if quest(npc_scripts::ZAKUM_TRIALS_DONE) != QuestState::Started => {
            refused(keys::ZAKUM_GATE_TRIALS)
        }
        "Zakum05" if player.inventory.count(npc_scripts::EYE_OF_FIRE) == 0 => {
            refused(keys::ZAKUM_GATE_NO_EYE)
        }
        "Zakum05" => through(Effect::WarpToPortal(
            npc_scripts::ENTRANCE_TO_ZAKUM,
            named(npc_scripts::ENTRANCE_TO_ZAKUM, "west00"),
        )),
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
                    occupied: None,
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
        script if PLAIN_WARPS.iter().any(|&(name, _, _)| name == script) => {
            let &(_, map, portal) = PLAIN_WARPS.iter().find(|&&(name, _, _)| name == script)?;
            let mut outcome = through(to(map, portal));
            if script == "move_elin" {
                outcome.effects.insert(
                    0,
                    Effect::Pink(content::text().get(keys::ELLIN_GATE_PASSING, &[])),
                );
            }
            outcome
        }
        "party3_roomout" => {
            let map = player.stats.map_id;
            let out = OPQ_ROOMS_OUT.iter().find(|&&(room, _)| room == map);
            through(Effect::WarpToPortal(
                crate::events::OPQ_HUB,
                out.map_or(0, |&(_, portal)| portal),
            ))
        }
        // The hub's storage stays shut until Minerva is free; the event
        // opens it for a team that freed her.
        OPQ_STORAGE_IN => refused(keys::OPQ_NOTICE_STORAGE_CLOSED),
        "OutPerrion_1" => ScriptOutcome {
            effects: vec![
                Effect::Pink(content::text().get(keys::TEMPLE_SHORTCUT, &[])),
                Effect::PortalSound,
                Effect::WarpToPortal(UNDERGROUND_TEMPLE_STAIRS, 2),
            ],
            block: false,
            occupied: None,
        },
        "enterRienFirst" => {
            let start = if aran_in_tutorial() { "st00" } else { "west00" };
            through(Effect::WarpToPortal(RIEN, named(RIEN, start)))
        }
        "rienTutor7" if aran_in_tutorial() => ScriptOutcome {
            effects: vec![Effect::InfoText(
                content::text().get(keys::RIEN_TUTOR_TO_TOWN, &[]),
            )],
            block: false,
            occupied: None,
        },
        "rienTutor7" => through(Effect::WarpToPortal(SECOND_DANGEROUS_FOREST, 2)),
        "enterGym" => match TRAINING_GROUNDS
            .iter()
            .find(|&&(id, _)| quest(id) == QuestState::Started)
        {
            Some(&(_, ground)) => through(Effect::WarpToPortal(ground, 1)),
            None => refused(keys::TRAINING_GROUND_NO_LESSON),
        },
        "rienCaveEnter"
            if [21201, 21302]
                .iter()
                .any(|&id| quest(id) == QuestState::Started) =>
        {
            through(Effect::WarpToPortal(MIRROR_CAVE, 1))
        }
        "rienCaveEnter" => refused(keys::MIRROR_CAVE_BLOCKED),
        // Cosmic also starts quest 23647 here, which isn't in Quest.nx.
        "foxLaidy_map"
            if quest(3647) == QuestState::Started && player.inventory.count(OLD_FOX_TAIL) > 0 =>
        {
            through(Effect::WarpToPortal(
                GLOOMY_FOREST,
                named(GLOOMY_FOREST, "east00"),
            ))
        }
        "foxLaidy_map" => through(Effect::WarpToPortal(
            TIGER_RIDGE,
            named(TIGER_RIDGE, "east00"),
        )),
        // Cosmic starts the Nine Spirit's egg event for a player on quest
        // 100203 or holding item 4001094; that isn't in, so they're kept
        // out as anyone else.
        "dragonNest" if quest(3706) == QuestState::Completed => {
            through(Effect::WarpToPortal(240040612, named(240040612, "out00")))
        }
        "dragonNest" => refused(keys::NINE_SPIRIT_NEST_BLOCKED),
        "go_secretroom" if quest(SECURITY_ROOM_QUEST) == QuestState::Completed => {
            through(Effect::WarpToPortal(SECURITY_ROOM, 1))
        }
        // Completing the quest needs it in the catalog.
        "go_secretroom"
            if quest(SECURITY_ROOM_QUEST) == QuestState::Started
                && player.inventory.count(SECURITY_ROOM_KEY) > 0
                && crate::quests::catalog()
                    .is_ok_and(|catalog| catalog.quest(SECURITY_ROOM_QUEST).is_some()) =>
        {
            ScriptOutcome {
                effects: vec![
                    Effect::CompleteQuest(SECURITY_ROOM_QUEST),
                    Effect::GainExp(5000),
                    Effect::TakeItem {
                        item_id: SECURITY_ROOM_KEY,
                        quantity: 1,
                    },
                    Effect::PortalSound,
                    Effect::WarpToPortal(SECURITY_ROOM, 1),
                ],
                block: false,
                occupied: None,
            }
        }
        "go_secretroom" => refused(keys::SECURITY_ROOM_LOCKED),
        "tristanEnter" if quest(2238) == QuestState::Completed => {
            through(Effect::WarpToPortal(105100101, named(105100101, "in00")))
        }
        "q2073" if quest(2073) == QuestState::Started => {
            through(Effect::WarpToPortal(900000000, 0))
        }
        "q2073" => refused(keys::PIG_FARM_PRIVATE),
        "enterDollWay"
            if [20730, 21734]
                .iter()
                .any(|&id| quest(id) == QuestState::Completed) =>
        {
            through(Effect::WarpToPortal(105070300, 3))
        }
        "enterDollWay" if quest(21734) == QuestState::Started => {
            through(Effect::WarpToPortal(910510100, 0))
        }
        "enterDollWay" => refused(keys::DOLL_WAY_BLOCKED),
        "enterDollcave"
            if quest(20730) == QuestState::Completed || quest(21734) == QuestState::Completed =>
        {
            let (map, portal) = DOLL_CAVE_WAY_BACK;
            through(Effect::WarpToPortal(map, portal))
        }
        // Cosmic asks for a password here; not yet.
        "enterDollcave" => ScriptOutcome::default(),
        "tutorialNPC" => {
            let stats = &player.stats;
            match TUTORIAL_INSTRUCTORS
                .iter()
                .find(|&&(map, _)| map == stats.map_id)
                .filter(|_| stats.level <= 10 && stats.job == 0)
            {
                Some(&(_, npc)) => opens(npc),
                None => ScriptOutcome::default(),
            }
        }
        "TD_chat_enter" => opens(TIME_GATE),
        "ariant_Agit"
            if [3928, 3931, 3934]
                .iter()
                .all(|&id| quest(id) == QuestState::Completed) =>
        {
            through(Effect::WarpToPortal(260000201, 1))
        }
        "ariant_Agit" => refused(keys::SAND_BANDITS_ONLY),
        "skyrom"
            if quest(3935) == QuestState::Started && player.inventory.count(QUEENS_JEWEL) == 0 =>
        {
            ScriptOutcome {
                effects: vec![
                    Effect::RequireEmptyMap(QUEENS_TREASURE_ROOM),
                    Effect::PortalSound,
                    Effect::WarpToPortal(QUEENS_TREASURE_ROOM, 0),
                ],
                block: false,
                occupied: Some(content::text().get(keys::TREASURE_ROOM_OCCUPIED, &[])),
            }
        }
        "skyrom" => ScriptOutcome::default(),
        "minar_elli" if player.inventory.count(MAGIC_SEED) > 0 => {
            let (map, name) = if player.stats.map_id == FIELD_NORTH_OF_ELLINIA {
                (MINAR_FOREST_WEST_BORDER, "elli00")
            } else {
                (FIELD_NORTH_OF_ELLINIA, "minar00")
            };
            ScriptOutcome {
                effects: vec![
                    Effect::Pink(content::text().get(keys::MAGIC_SEED_SPENT, &[])),
                    Effect::TakeItem {
                        item_id: MAGIC_SEED,
                        quantity: 1,
                    },
                    Effect::PortalSound,
                    to(map, Portal::Named(name)),
                ],
                block: false,
                occupied: None,
            }
        }
        "minar_elli" => refused(keys::MAGIC_SEED_NEEDED),
        "enter_earth00" | "enter_earth01" if player.inventory.count(WARP_CARD) > 0 => {
            through(if script == "enter_earth00" {
                to(OMEGA_COMMAND_CENTER, Portal::Named("earth00"))
            } else {
                to(NAVIGATION_ROOM, Portal::Named("earth01"))
            })
        }
        "enter_earth00" | "enter_earth01" => refused(keys::WARP_CARD_NEEDED),
        "ariant_queens"
            if player
                .buffs
                .active()
                .iter()
                .any(|buff| buff.stat == BuffStat::Morph && buff.skill_id == -TIGUNS_DISGUISE) =>
        {
            ScriptOutcome::default()
        }
        "ariant_queens" => ScriptOutcome {
            effects: vec![
                Effect::PortalSound,
                to(ARIANT_CASTLE, Portal::Named("R001")),
                Effect::Pink(content::text().get(keys::PALACE_INTRUDER, &[])),
            ],
            block: false,
            occupied: None,
        },
        "nets_in" => ScriptOutcome {
            effects: vec![
                Effect::SaveLocation(PYRAMID_RETURN, Some(player.stats.map_id)),
                Effect::PortalSound,
                to(PYRAMID_DUNES, Portal::Named("out00")),
            ],
            block: false,
            occupied: None,
        },
        "nets_out" => {
            let back = player
                .saved_locations
                .get(&PYRAMID_RETURN)
                .copied()
                .unwrap_or(SAHEL_3);
            go_back(
                PYRAMID_RETURN,
                if back == SAHEL_3 {
                    to(SAHEL_3, Portal::Named("piramid00"))
                } else {
                    Effect::Warp(back)
                },
            )
        }
        script if script.starts_with("MD_") => {
            let dungeon = crate::dungeons::by_script(script)?;
            if player.stats.map_id != dungeon.base {
                return Some(through(to(dungeon.base, Portal::Named("MD00"))));
            }
            ScriptOutcome {
                effects: vec![
                    Effect::EnterDungeon {
                        first: dungeon.first,
                        count: dungeon.count,
                        portal: named(dungeon.first, "out00"),
                    },
                    Effect::PortalSound,
                ],
                block: false,
                occupied: None,
            }
        }
        script if script.starts_with("advice") => {
            let &(_, key, width, height) = ADVICE.iter().find(|(name, ..)| *name == script)?;
            ScriptOutcome {
                effects: vec![Effect::Hint {
                    text: content::text().get(key, &[]),
                    width,
                    height,
                }],
                block: false,
                occupied: None,
            }
        }
        script if script.starts_with("enterDisguise") => {
            let job = script["enterDisguise".len()..]
                .parse::<usize>()
                .ok()
                .and_then(|n| n.checked_sub(1))
                .filter(|&job| job < DISGUISE_WAYS.len())?;
            disguise(player, job)
        }
        "s4resur_enter" if quest(6134) == QuestState::Started => {
            through(Effect::WarpToPortal(922020000, 0))
        }
        "s4rush" if quest(6110) == QuestState::Started => {
            through(Effect::WarpToPortal(910500100, 0))
        }
        "s4firehawk" => solo_map(quest(6240) == QuestState::Started, FIREHAWK_NEST),
        "s4iceeagle" => solo_map(quest(6242) == QuestState::Started, ICE_EAGLE_NEST),
        // Whoever holds a Cold Flame, on the quest or not, as in Cosmic.
        "s4resurrection" => solo_map(player.inventory.count(COLD_FLAME) > 0, RESURRECTION_ROOM),
        "s4nest" => solo_map(
            [6241, 6243]
                .iter()
                .any(|&id| quest(id) == QuestState::Started),
            EGG_NEST,
        ),
        "s4hitman" => solo_map(quest(6201) == QuestState::Started, HITMAN_HIDEOUT),
        "s4berserk" => solo_map_at(
            quest(6153) == QuestState::Started && player.inventory.count(BERSERK_KEY) > 0,
            BERSERK_ROOM,
            named(BERSERK_ROOM, "out01"),
        ),
        // Up to the statues, once the room's monsters are all gone.
        "s4berserk_move" => ScriptOutcome {
            effects: vec![
                Effect::RequireNoMobs,
                Effect::PortalSound,
                Effect::WarpToPortal(BERSERK_ROOM, named(BERSERK_ROOM, "out00")),
            ],
            block: false,
            occupied: Some(content::text().get(keys::MONSTERS_LEFT, &[])),
        },
        // Closed, it says nothing, as in Cosmic.
        "s4tornado_enter"
            if [6230, 6231]
                .iter()
                .any(|&id| quest(id) == QuestState::Started)
                || player.inventory.count(TORNADO_PROOF) > 0 =>
        {
            solo_map(true, TORNADO_ROOM)
        }
        "s4tornado_enter" => ScriptOutcome::default(),
        "tristanEnter" | "s4resur_enter" | "s4rush" => refused(keys::MYSTERIOUS_FORCE),
        "glTutoMsg0" => ScriptOutcome {
            effects: vec![Effect::Hint {
                text: content::text().get(keys::LEAVE_AREA_WARNING, &[]),
                width: 150,
                height: 5,
            }],
            block: false,
            occupied: None,
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

/// Ereve's Training Forest way on for the Knight `job` (0 to 4); see
/// `DISGUISE_WAYS`.
fn disguise(player: &Player, job: usize) -> ScriptOutcome {
    let searching = (20301..=20305).any(|id| player.quests.state(id) == QuestState::Started);
    if !searching {
        let (map, portal) = DISGUISE_WAYS[job];
        return through(Effect::WarpToPortal(map, named(map, portal)));
    }
    let job = job as i32;
    let forest = DISGUISE_FORESTS + 10 * job;
    // Someone else in the forest is said first, as in Cosmic.
    let effects = if player.inventory.count(DISGUISE_PROOF + job) > 0 {
        vec![
            Effect::RequireEmptyMap(forest),
            Effect::Pink(content::text().get(keys::DISGUISE_ALREADY_FOUND, &[])),
        ]
    } else {
        let portal = if job == 4 { "east00" } else { "out00" };
        vec![
            Effect::RequireEmptyMap(forest),
            Effect::PortalSound,
            Effect::WarpToPortal(forest, named(forest, portal)),
        ]
    };
    ScriptOutcome {
        effects,
        block: false,
        occupied: Some(content::text().get(keys::DISGUISE_FOREST_OCCUPIED, &[])),
    }
}

/// Into `map`, one of `SOLO_MAPS`, when `open` and nobody is in it.
fn solo_map(open: bool, map: i32) -> ScriptOutcome {
    solo_map_at(open, map, 0)
}

/// Into `map`'s `portal`, as `solo_map`.
fn solo_map_at(open: bool, map: i32, portal: u8) -> ScriptOutcome {
    if !open {
        return refused(keys::MYSTERIOUS_FORCE);
    }
    ScriptOutcome {
        effects: vec![
            Effect::RequireEmptyMap(map),
            Effect::PortalSound,
            Effect::WarpToPortal(map, portal),
        ],
        block: false,
        occupied: Some(content::text().get(keys::SOLO_MAP_OCCUPIED, &[])),
    }
}

/// To `map`'s `portal`.
fn to(map: i32, portal: Portal) -> Effect {
    let id = match portal {
        Portal::Named(name) => named(map, name),
        Portal::Id(id) => id,
    };
    Effect::WarpToPortal(map, id)
}

/// A portal that lets the player through with its sound.
fn through(warp: Effect) -> ScriptOutcome {
    ScriptOutcome {
        effects: vec![Effect::PortalSound, warp],
        block: false,
        occupied: None,
    }
}

/// A portal that opens `npc`'s conversation.
fn opens(npc: i32) -> ScriptOutcome {
    ScriptOutcome {
        effects: vec![Effect::OpenNpc(npc)],
        block: false,
        occupied: None,
    }
}

/// A portal that keeps the player back, saying why in pink chat.
fn refused(key: &str) -> ScriptOutcome {
    ScriptOutcome {
        effects: vec![Effect::Pink(content::text().get(key, &[]))],
        block: false,
        occupied: None,
    }
}

/// End a trip: forget the saved location `kind` and take the player back
/// with `warp`.
fn go_back(kind: i16, warp: Effect) -> ScriptOutcome {
    ScriptOutcome {
        effects: vec![Effect::SaveLocation(kind, None), Effect::PortalSound, warp],
        block: false,
        occupied: None,
    }
}

fn is_market_portal(portal: &PortalTemplate) -> bool {
    portal
        .script
        .as_deref()
        .is_some_and(|script| script.contains("market"))
}

/// The id of `map_id`'s portal called `name`, else 0.
pub fn named(map_id: i32, name: &str) -> u8 {
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
            occupied: None,
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

    /// Where `map_id`'s portal `name` is, as a player position.
    fn at_portal(map_id: i32, name: &str) -> (i16, i16) {
        let data = crate::game_data::get().unwrap();
        let portal = data.field(map_id).unwrap().portal_by_name(name).unwrap();
        (portal.x as i16, portal.y as i16)
    }

    #[test]
    fn a_killer_mushroom_spore_opens_the_obstacle_for_good() {
        let mut player = player_with(&[]);
        player.stats.map_id = DEEP_INSIDE;
        player.inventory.insert(
            InventoryType::Use,
            1,
            game::Item::stack(KILLER_MUSHROOM_SPORE, 2),
        );
        let (x, y) = at_portal(DEEP_INSIDE, "obstacle");
        let effects = item("killarmush", &player, (x - 150, y - 100)).unwrap();
        assert!(matches!(
            &effects[..],
            [
                Effect::Mark(OBSTACLE_OPENED),
                Effect::TakeItem {
                    item_id: KILLER_MUSHROOM_SPORE,
                    quantity: 2,
                },
                Effect::Pink(_),
            ]
        ));
        // Too far, or on another map, nothing happens.
        assert_eq!(
            item("killarmush", &player, (x - 150, y - 150)),
            Some(vec![])
        );
        player.stats.map_id = SPLIT_ROAD;
        assert_eq!(item("killarmush", &player, (x, y)), Some(vec![]));

        // Once opened, the portal lets the player by without a spore, and
        // another spore doesn't mark it again.
        player.stats.map_id = DEEP_INSIDE;
        player.quests.mark(OBSTACLE_OPENED);
        assert!(matches!(
            item("killarmush", &player, (x, y)).unwrap()[..],
            [Effect::TakeItem { .. }, Effect::Pink(_)]
        ));
        assert_eq!(
            portal("obstacle", &player).unwrap().effects,
            [
                Effect::PortalSound,
                Effect::WarpToPortal(SPLIT_ROAD, named(SPLIT_ROAD, "left00")),
            ]
        );
    }

    #[test]
    fn a_thorn_remover_sets_its_quests_record_by_the_castle_wall() {
        let mut player = player_with(&[(THORN_REMOVER_QUEST, QuestState::Started)]);
        player.stats.map_id = SPLIT_ROAD;
        player
            .inventory
            .insert(InventoryType::Use, 1, game::Item::stack(THORN_REMOVER, 1));
        let (x, y) = at_portal(SPLIT_ROAD, "right00");
        let effects = item("removethorns", &player, (x, y)).unwrap();
        assert!(matches!(
            &effects[..],
            [
                Effect::QuestRecord {
                    quest_id: THORN_REMOVER_QUEST,
                    count: 1,
                },
                Effect::TakeItem {
                    item_id: THORN_REMOVER,
                    quantity: 1,
                },
                Effect::Pink(_),
            ]
        ));
        assert_eq!(item("removethorns", &player, (x + 210, y)), Some(vec![]));
        let mut done = player_with(&[(THORN_REMOVER_QUEST, QuestState::Completed)]);
        done.stats.map_id = SPLIT_ROAD;
        assert_eq!(item("removethorns", &done, (x, y)), Some(vec![]));
        assert_eq!(item("nosuchscript", &done, (x, y)), None);
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

    fn at(map_id: i32, quests: &[(i16, QuestState)]) -> Player {
        let mut player = player_with(quests);
        player.stats.map_id = map_id;
        player
    }

    #[test]
    fn map_entry_shows_the_maps_banner() {
        let player = player_with(&[]);
        assert_eq!(
            map_enter("go10000", &player),
            Some(vec![Effect::FieldEffect("maplemap/enter/10000".into())])
        );
        assert_eq!(map_enter("goAdventure", &player), None);
        assert_eq!(map_enter("go", &player), None);
        assert_eq!(
            map_enter("explorationPoint", &at(LITH_HARBOR, &[])),
            Some(vec![Effect::FieldEffect("maplemap/enter/104000000".into())])
        );
    }

    #[test]
    fn exploring_counts_toward_the_regions_medal() {
        let mut player = at(LITH_HARBOR, &[]);
        player.stats.level = 20;
        assert_eq!(
            map_enter("explorationPoint", &player),
            Some(vec![
                Effect::FieldEffect("maplemap/enter/104000000".into()),
                Effect::Explore {
                    medal: 29005,
                    map_id: LITH_HARBOR,
                },
            ])
        );
        // The highest places' portals count toward their own medal.
        player.stats.map_id = 101010103;
        let outcome = portal("highposition", &player).unwrap();
        assert_eq!(
            outcome.effects,
            [Effect::Explore {
                medal: crate::medals::TOUCHED_THE_SKY,
                map_id: 101010103,
            }]
        );
        assert!(!outcome.block);
        // Touched the Sky counts from level 20.
        player.stats.level = 19;
        assert!(portal("highposition", &player).unwrap().effects.is_empty());
    }

    #[test]
    fn the_time_gate_and_mushroom_castle_show_their_theme_banners() {
        let data = crate::get_game_data().unwrap();
        for (map_id, banner) in THEME_BANNERS {
            let script = data.field(map_id).unwrap().on_user_enter.clone().unwrap();
            assert_eq!(
                map_enter(&script, &at(map_id, &[])),
                Some(vec![Effect::FieldEffect(format!("temaD/enter/{banner}"))]),
                "{map_id}"
            );
        }
        // A map with the script but no banner of its own shows none.
        assert_eq!(map_enter("TD_NC_title", &at(HENESYS, &[])), Some(vec![]));
    }

    #[test]
    fn the_disciple_comes_out_for_whoever_hunts_him() {
        let data = crate::get_game_data().unwrap();
        let script = data
            .field(DISCIPLES_HIDEOUT)
            .unwrap()
            .on_user_enter
            .clone()
            .unwrap();
        let hunting = at(DISCIPLES_HIDEOUT, &[(DISCIPLES_QUEST, QuestState::Started)]);
        assert_eq!(
            map_enter(&script, &hunting),
            Some(vec![Effect::BringOutMob {
                mob_id: DISCIPLE,
                at: (-1027, 216)
            }])
        );
        for quests in [&[][..], &[(DISCIPLES_QUEST, QuestState::Completed)]] {
            assert_eq!(
                map_enter(&script, &at(DISCIPLES_HIDEOUT, quests)),
                Some(vec![])
            );
        }
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
                block: true,
                occupied: None
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
    fn the_tower_of_goddess_rooms_let_out_by_their_own_hub_portals() {
        let mut player = player_with(&[]);
        player.stats.map_id = 920010500;
        let out = portal("party3_roomout", &player).unwrap();
        assert_eq!(warp_of(&out), Some((crate::events::OPQ_HUB, 13)));
        player.stats.map_id = 920011000;
        let out = portal("party3_roomout", &player).unwrap();
        assert_eq!(warp_of(&out), Some((crate::events::OPQ_HUB, 16)));
        let shut = portal("party3_room8", &player).unwrap();
        assert!(matches!(shut.effects.as_slice(), [Effect::Pink(_)]));
        let room = portal("party3_room5", &player).unwrap();
        assert_eq!(warp_of(&room), Some((920010600, 17)));
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

    #[test]
    fn plain_warps_lead_to_portals_that_exist() {
        let fresh = player_with(&[]);
        for (script, map, portal) in PLAIN_WARPS {
            if let Portal::Named(name) = portal {
                assert_ne!(named(map, name), 0, "{script}");
            }
            let outcome = super::portal(script, &fresh).unwrap();
            assert_eq!(outcome.effects.last(), Some(&to(map, portal)), "{script}");
        }
        // Only the Time Gate says something on the way.
        let gate = portal("move_elin", &fresh).unwrap();
        assert!(matches!(gate.effects[0], Effect::Pink(_)));
        assert_eq!(
            warp_of(&portal("Pianus", &fresh).unwrap()),
            Some((230040420, 2))
        );
        let shortcut = portal("OutPerrion_1", &fresh).unwrap();
        assert!(matches!(
            shortcut.effects.as_slice(),
            [
                Effect::Pink(_),
                Effect::PortalSound,
                Effect::WarpToPortal(UNDERGROUND_TEMPLE_STAIRS, 2)
            ]
        ));
    }

    #[test]
    fn the_dangerous_forest_keeps_an_aran_on_the_tutorial_near_rien() {
        let mut aran = player_with(&[]);
        aran.stats.job = ARAN_BEGINNER;
        let rien = |player: &Player| warp_of(&portal("enterRienFirst", player).unwrap());
        assert_ne!(named(RIEN, "st00"), named(RIEN, "west00"));
        assert_eq!(rien(&aran), Some((RIEN, named(RIEN, "st00"))));
        assert_eq!(rien(&player_with(&[])), Some((RIEN, named(RIEN, "west00"))));
        let held = portal("rienTutor7", &aran).unwrap();
        assert!(matches!(held.effects.as_slice(), [Effect::InfoText(_)]));
        let mut done = player_with(&[(RIEN_TUTORIAL, QuestState::Completed)]);
        done.stats.job = ARAN_BEGINNER;
        assert_eq!(rien(&done), Some((RIEN, named(RIEN, "west00"))));
        assert_eq!(
            warp_of(&portal("rienTutor7", &done).unwrap()),
            Some((SECOND_DANGEROUS_FOREST, 2))
        );
    }

    #[test]
    fn quest_gated_ways_open_on_their_quests() {
        let started = QuestState::Started;
        let done = QuestState::Completed;
        let go =
            |script, quests: &[(i16, QuestState)]| portal(script, &player_with(quests)).unwrap();
        for (script, open, to) in [
            ("enterGym", (21702, started), (914010100, 1)),
            ("rienCaveEnter", (21302, started), (MIRROR_CAVE, 1)),
            (
                "dragonNest",
                (3706, done),
                (240040612, named(240040612, "out00")),
            ),
            ("go_secretroom", (2335, done), (SECURITY_ROOM, 1)),
            (
                "tristanEnter",
                (2238, done),
                (105100101, named(105100101, "in00")),
            ),
            ("q2073", (2073, started), (900000000, 0)),
            ("enterDollWay", (20730, done), (105070300, 3)),
            ("enterDollWay", (21734, done), (105070300, 3)),
            ("enterDollWay", (21734, started), (910510100, 0)),
            ("s4resur_enter", (6134, started), (922020000, 0)),
            ("s4rush", (6110, started), (910500100, 0)),
        ] {
            assert!(pink(&go(script, &[])), "{script}");
            assert_eq!(warp_of(&go(script, &[open])), Some(to), "{script}");
        }
        let bandits = [(3928, done), (3931, done), (3934, done)];
        assert!(pink(&go("ariant_Agit", &bandits[..2])));
        assert_eq!(warp_of(&go("ariant_Agit", &bandits)), Some((260000201, 1)));
        assert_ne!(named(240040612, "out00"), 0);
        assert_ne!(named(105100101, "in00"), 0);
    }

    fn holding(quests: &[(i16, QuestState)], item: i32) -> Player {
        let mut player = player_with(quests);
        player
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(item, 1));
        player
    }

    #[test]
    fn the_security_room_key_completes_its_quest_at_the_door() {
        let quest = [(SECURITY_ROOM_QUEST, QuestState::Started)];
        assert!(pink(
            &portal("go_secretroom", &player_with(&quest)).unwrap()
        ));
        let opened = portal("go_secretroom", &holding(&quest, SECURITY_ROOM_KEY)).unwrap();
        if crate::quests::catalog()
            .unwrap()
            .quest(SECURITY_ROOM_QUEST)
            .is_none()
        {
            // Not supported yet: the door stays shut rather than fail.
            assert!(pink(&opened));
            return;
        }
        assert_eq!(
            opened.effects,
            [
                Effect::CompleteQuest(SECURITY_ROOM_QUEST),
                Effect::GainExp(5000),
                Effect::TakeItem {
                    item_id: SECURITY_ROOM_KEY,
                    quantity: 1
                },
                Effect::PortalSound,
                Effect::WarpToPortal(SECURITY_ROOM, 1),
            ]
        );
    }

    #[test]
    fn fox_ridge_leads_a_fox_hunter_into_the_gloomy_forest() {
        let tiger = Some((TIGER_RIDGE, named(TIGER_RIDGE, "east00")));
        let quest = [(3647, QuestState::Started)];
        assert_eq!(
            warp_of(&portal("foxLaidy_map", &player_with(&quest)).unwrap()),
            tiger
        );
        assert_eq!(
            warp_of(&portal("foxLaidy_map", &holding(&[], OLD_FOX_TAIL)).unwrap()),
            tiger
        );
        assert_eq!(
            warp_of(&portal("foxLaidy_map", &holding(&quest, OLD_FOX_TAIL)).unwrap()),
            Some((GLOOMY_FOREST, named(GLOOMY_FOREST, "east00")))
        );
        assert_ne!(named(GLOOMY_FOREST, "east00"), 0);
    }

    #[test]
    fn the_queens_treasure_room_takes_one_searcher_at_a_time() {
        let quest = [(3935, QuestState::Started)];
        let outcome = portal("skyrom", &player_with(&quest)).unwrap();
        assert_eq!(
            outcome.effects,
            [
                Effect::RequireEmptyMap(QUEENS_TREASURE_ROOM),
                Effect::PortalSound,
                Effect::WarpToPortal(QUEENS_TREASURE_ROOM, 0),
            ]
        );
        assert!(outcome.occupied.is_some());
        // Without the quest, or with the jewel found, nothing happens.
        assert_eq!(
            portal("skyrom", &player_with(&[])),
            Some(ScriptOutcome::default())
        );
        assert_eq!(
            portal("skyrom", &holding(&quest, QUEENS_JEWEL)),
            Some(ScriptOutcome::default())
        );
    }

    #[test]
    fn an_instructors_room_has_a_young_beginner_hear_the_instructor_out() {
        let data = crate::get_game_data().unwrap();
        let in_room = |map, level, job| {
            let mut player = player_with(&[]);
            player.stats.map_id = map;
            player.stats.level = level;
            player.stats.job = job;
            portal("tutorialNPC", &player).unwrap()
        };
        for (map, npc) in TUTORIAL_INSTRUCTORS {
            let way = data.field(map).unwrap().portal_by_name("tutorial").unwrap();
            assert_eq!(way.script.as_deref(), Some("tutorialNPC"), "{map}");
            for level in [1, 10] {
                let outcome = in_room(map, level, 0);
                assert_eq!(outcome.effects, [Effect::OpenNpc(npc)], "{map}");
                assert!(!outcome.block);
            }
            assert_eq!(in_room(map, 11, 0), ScriptOutcome::default());
            assert_eq!(in_room(map, 10, 100), ScriptOutcome::default());
        }
        assert_eq!(in_room(100000000, 1, 0), ScriptOutcome::default());
    }

    #[test]
    fn the_time_gate_and_the_doll_caves_back_way_open_from_their_portals() {
        let data = crate::get_game_data().unwrap();
        let script = |map, name: &str| {
            let field = data.field(map).unwrap();
            field.portal_by_name(name).unwrap().script.clone()
        };
        assert_eq!(
            script(240070000, "TD_neo").as_deref(),
            Some("TD_chat_enter")
        );
        assert_eq!(
            portal("TD_chat_enter", &player_with(&[])).unwrap().effects,
            [Effect::OpenNpc(TIME_GATE)]
        );
        assert_eq!(script(105070300, "in00").as_deref(), Some("enterDollcave"));
        let (map, id) = DOLL_CAVE_WAY_BACK;
        assert_eq!(named(map, "in00"), id);
        for quest in [20730, 21734] {
            let done = player_with(&[(quest, QuestState::Completed)]);
            assert_eq!(
                warp_of(&portal("enterDollcave", &done).unwrap()),
                Some(DOLL_CAVE_WAY_BACK)
            );
        }
        let started = player_with(&[(20730, QuestState::Started)]);
        assert_eq!(
            portal("enterDollcave", &started),
            Some(ScriptOutcome::default())
        );
    }

    #[test]
    fn a_fourth_jobs_solo_room_takes_one_player_at_a_time() {
        let opened = |script, player: &Player, map| {
            let outcome = portal(script, player).unwrap();
            assert_eq!(
                outcome.effects,
                [
                    Effect::RequireEmptyMap(map),
                    Effect::PortalSound,
                    Effect::WarpToPortal(map, 0),
                ],
                "{script}"
            );
            assert!(outcome.occupied.is_some(), "{script}");
        };
        let fresh = player_with(&[]);
        for (script, quest, map) in [
            ("s4firehawk", 6240, FIREHAWK_NEST),
            ("s4iceeagle", 6242, ICE_EAGLE_NEST),
            ("s4hitman", 6201, HITMAN_HIDEOUT),
        ] {
            opened(script, &player_with(&[(quest, QuestState::Started)]), map);
            assert!(pink(&portal(script, &fresh).unwrap()), "{script}");
            let done = player_with(&[(quest, QuestState::Completed)]);
            assert!(pink(&portal(script, &done).unwrap()), "{script}");
        }
        for quest in [6230, 6231] {
            let player = player_with(&[(quest, QuestState::Started)]);
            opened("s4tornado_enter", &player, TORNADO_ROOM);
        }
        for quest in [6241, 6243] {
            opened(
                "s4nest",
                &player_with(&[(quest, QuestState::Started)]),
                EGG_NEST,
            );
        }
        assert!(pink(&portal("s4nest", &fresh).unwrap()));
        opened(
            "s4resurrection",
            &holding(&[], COLD_FLAME),
            RESURRECTION_ROOM,
        );
        assert!(pink(&portal("s4resurrection", &fresh).unwrap()));
        opened(
            "s4tornado_enter",
            &holding(&[], TORNADO_PROOF),
            TORNADO_ROOM,
        );
        // Closed, the Tornado room's way says nothing.
        assert_eq!(
            portal("s4tornado_enter", &fresh),
            Some(ScriptOutcome::default())
        );
        // The Berserk room wants its key as well as the quest, and lets
        // the player in at the bottom.
        let on_quest = player_with(&[(6153, QuestState::Started)]);
        assert!(pink(&portal("s4berserk", &on_quest).unwrap()));
        assert!(pink(
            &portal("s4berserk", &holding(&[], BERSERK_KEY)).unwrap()
        ));
        let keyed = holding(&[(6153, QuestState::Started)], BERSERK_KEY);
        let outcome = portal("s4berserk", &keyed).unwrap();
        assert_eq!(outcome.effects[0], Effect::RequireEmptyMap(BERSERK_ROOM));
        assert!(matches!(
            outcome.effects[2],
            Effect::WarpToPortal(BERSERK_ROOM, _)
        ));
    }

    #[test]
    fn the_berserk_rooms_way_up_waits_for_its_monsters() {
        let outcome = portal("s4berserk_move", &player_with(&[])).unwrap();
        assert_eq!(
            outcome.effects[..2],
            [Effect::RequireNoMobs, Effect::PortalSound]
        );
        assert!(matches!(
            outcome.effects[2],
            Effect::WarpToPortal(BERSERK_ROOM, _)
        ));
        assert!(outcome.occupied.is_some());
        if crate::get_game_data().is_err() {
            return;
        }
        let way_up = named(BERSERK_ROOM, "out00");
        assert_eq!(
            portal(
                "s4berserk",
                &holding(&[(6153, QuestState::Started)], BERSERK_KEY)
            )
            .unwrap()
            .effects[2],
            Effect::WarpToPortal(BERSERK_ROOM, 2)
        );
        assert_eq!(
            outcome.effects[2],
            Effect::WarpToPortal(BERSERK_ROOM, way_up)
        );
        assert_eq!(way_up, 3);
    }

    #[test]
    fn each_solo_room_is_reached_by_its_script_and_has_a_way_back() {
        let data = crate::get_game_data().unwrap();
        for (from, name, script, map) in [
            (211042200, "in00", "s4firehawk", FIREHAWK_NEST),
            (211040700, "in00", "s4iceeagle", ICE_EAGLE_NEST),
            (220010001, "pt00", "s4tornado_enter", TORNADO_ROOM),
            (230040300, "in01", "s4resurrection", RESURRECTION_ROOM),
            (240010700, "pt00", "s4nest", EGG_NEST),
            (101030104, "pt00", "s4hitman", HITMAN_HIDEOUT),
            (105090800, "in00", "s4berserk", BERSERK_ROOM),
        ] {
            let way_in = data.field(from).unwrap().portal_by_name(name).unwrap();
            assert_eq!(way_in.script.as_deref(), Some(script));
            let room = data.field(map).unwrap();
            assert!(room.portal_by_id(0).is_some(), "{map}");
            let way_out = room.portal_by_name("out00").unwrap();
            let back = way_out.to_map.and_then(|to| data.field(to));
            assert!(back.is_some(), "{map}");
            assert!(crate::npc_scripts::resets_when_empty(map));
        }
    }

    #[test]
    fn ereves_forests_lead_on_or_to_the_master_of_disguise() {
        let fresh = player_with(&[]);
        for (n, (map, portal)) in DISGUISE_WAYS.into_iter().enumerate() {
            let script = format!("enterDisguise{}", n + 1);
            assert_ne!(named(map, portal), 0);
            assert_eq!(
                warp_of(&super::portal(&script, &fresh).unwrap()),
                Some((map, named(map, portal)))
            );
        }
        let quest = [(20305, QuestState::Started)];
        let tiru = super::portal("enterDisguise4", &player_with(&quest)).unwrap();
        assert_eq!(
            tiru.effects,
            [
                Effect::RequireEmptyMap(108010630),
                Effect::PortalSound,
                Effect::WarpToPortal(108010630, named(108010630, "out00")),
            ]
        );
        assert!(tiru.occupied.is_some());
        let drill_hall = super::portal("enterDisguise5", &player_with(&quest)).unwrap();
        assert_eq!(
            drill_hall.effects.last(),
            Some(&Effect::WarpToPortal(108010640, named(108010640, "east00")))
        );
        // His proof means he has been found already.
        let found = super::portal("enterDisguise2", &holding(&quest, DISGUISE_PROOF + 1)).unwrap();
        assert!(matches!(
            found.effects.as_slice(),
            [Effect::RequireEmptyMap(108010610), Effect::Pink(_)]
        ));
        assert_eq!(super::portal("enterDisguise6", &fresh), None);
    }

    #[test]
    fn a_magic_seed_carries_the_player_between_ellinia_and_leafre() {
        assert!(pink(&portal("minar_elli", &player_with(&[])).unwrap()));
        for (from, to, name) in [
            (FIELD_NORTH_OF_ELLINIA, MINAR_FOREST_WEST_BORDER, "elli00"),
            (MINAR_FOREST_WEST_BORDER, FIELD_NORTH_OF_ELLINIA, "minar00"),
        ] {
            let mut player = holding(&[], MAGIC_SEED);
            player.stats.map_id = from;
            let outcome = portal("minar_elli", &player).unwrap();
            let [Effect::Pink(_), take, Effect::PortalSound, warp] = outcome.effects.as_slice()
            else {
                panic!("{outcome:?}");
            };
            assert_eq!(
                *take,
                Effect::TakeItem {
                    item_id: MAGIC_SEED,
                    quantity: 1
                }
            );
            assert_ne!(named(to, name), 0);
            assert_eq!(*warp, Effect::WarpToPortal(to, named(to, name)));
        }
    }

    #[test]
    fn a_warp_card_links_the_nautilus_and_omega_sector() {
        for script in ["enter_earth00", "enter_earth01"] {
            assert!(pink(&portal(script, &player_with(&[])).unwrap()));
        }
        let holder = holding(&[], WARP_CARD);
        for (script, to, name) in [
            ("enter_earth00", OMEGA_COMMAND_CENTER, "earth00"),
            ("enter_earth01", NAVIGATION_ROOM, "earth01"),
        ] {
            assert_ne!(named(to, name), 0);
            // The card is kept.
            assert_eq!(
                warp_of(&portal(script, &holder).unwrap()),
                Some((to, named(to, name)))
            );
        }
    }

    #[test]
    fn the_ariant_palace_turns_out_all_but_tiguns_disguise() {
        let mut player = player_with(&[]);
        let outcome = portal("ariant_queens", &player).unwrap();
        assert_eq!(named(ARIANT_CASTLE, "R001"), 7);
        assert!(matches!(
            outcome.effects.as_slice(),
            [
                Effect::PortalSound,
                Effect::WarpToPortal(ARIANT_CASTLE, 7),
                Effect::Pink(_)
            ]
        ));
        // Another morph, the Brawler's barrel, doesn't fool the guards.
        let now = std::time::Instant::now();
        let long = std::time::Duration::from_secs(60);
        let morph = |id| game::Buff::new(id, BuffStat::Morph, 1, now, long);
        player.buffs.give(morph(crate::morph::OAK_BARREL));
        assert_eq!(portal("ariant_queens", &player).unwrap().effects.len(), 3);
        player.buffs.give(morph(-TIGUNS_DISGUISE));
        assert_eq!(
            portal("ariant_queens", &player),
            Some(ScriptOutcome::default())
        );
    }

    #[test]
    fn maple_islands_advice_portals_show_their_balloons() {
        let fresh = player_with(&[]);
        for (script, _, width, height) in ADVICE {
            let outcome = portal(script, &fresh).unwrap();
            let [Effect::Hint {
                text,
                width: w,
                height: h,
            }] = outcome.effects.as_slice()
            else {
                panic!("{script}: {outcome:?}");
            };
            assert!(!text.is_empty());
            assert_eq!((*w, *h), (width, height));
        }
        assert_eq!(portal("advice10", &fresh), None);
    }

    #[test]
    fn a_mini_dungeons_way_in_asks_for_a_copy_and_its_way_out_leads_back() {
        let mut player = player_with(&[]);
        player.stats.map_id = 100020000;
        let outcome = portal("MD_pig", &player).unwrap();
        assert_eq!(
            outcome.effects,
            [
                Effect::EnterDungeon {
                    first: 100020100,
                    count: 30,
                    portal: 4,
                },
                Effect::PortalSound,
            ]
        );
        player.stats.map_id = 100020105;
        let outcome = portal("MD_pig", &player).unwrap();
        assert_eq!(
            outcome.effects,
            [Effect::PortalSound, Effect::WarpToPortal(100020000, 8)]
        );
        assert_eq!(portal("MD_cakeEnter", &player), None);
    }

    #[test]
    fn the_pyramid_dunes_lead_back_where_the_player_came_from() {
        let mut player = player_with(&[]);
        player.stats.map_id = SAHEL_3;
        let outcome = portal("nets_in", &player).unwrap();
        assert_eq!(named(PYRAMID_DUNES, "out00"), 4);
        assert_eq!(
            outcome.effects,
            [
                Effect::SaveLocation(PYRAMID_RETURN, Some(SAHEL_3)),
                Effect::PortalSound,
                Effect::WarpToPortal(PYRAMID_DUNES, 4),
            ]
        );
        // Back to Sahel 3's pyramid portal, also when nothing was saved.
        let back = [
            Effect::SaveLocation(PYRAMID_RETURN, None),
            Effect::PortalSound,
            Effect::WarpToPortal(SAHEL_3, named(SAHEL_3, "piramid00")),
        ];
        assert_eq!(named(SAHEL_3, "piramid00"), 3);
        assert_eq!(portal("nets_out", &player_with(&[])).unwrap().effects, back);
        player.saved_locations.insert(PYRAMID_RETURN, SAHEL_3);
        assert_eq!(portal("nets_out", &player).unwrap().effects, back);
        player.saved_locations.insert(PYRAMID_RETURN, HENESYS);
        assert_eq!(
            portal("nets_out", &player).unwrap().effects[2],
            Effect::Warp(HENESYS)
        );
    }
}
