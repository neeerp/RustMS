//! Native map and portal scripts, keyed by the script names in `Map.nx`
//! (`info/onUserEnter`, portal `script`), and the effects all scripts
//! return. Lua is deferred; returning effects keeps the shape a scripting
//! runtime would have. Unknown names do nothing.

use crate::content;
use crate::conti::Portal;
use crate::events::{
    EventKind, APPARATUS_ROOM, EAST_CASTLE_TOWER, JUNE, KERNING_SQUARE_6F, MACHINE_PART,
    PRIME_MINISTER, PRIME_MINISTER_AT, ROCK_SPIRIT_FLOOR, ROCK_SPIRIT_VIP_FLOOR, STOP_THE_WEDDING,
    STORY_OF_BETRAYAL, WEDDING_HALL, WEDDING_HALL_KEY, WHERES_VIOLETTA, YETI_BROTHERS,
};
use crate::npc_scripts;
use game::character::{CARNIVAL_RETURN, DOJO_RETURN, FREE_MARKET_RETURN, PYRAMID_RETURN};
use game::{BuffStat, InventoryType, Player, QuestState};
use game_data::server_content::keys;
use game_data::PortalTemplate;
use std::time::Duration;

const FREE_MARKET: i32 = 910000000;
const HENESYS: i32 = 100000000;
const LEAFRE_STATION_ENTRANCE: i32 = 240000100;
/// Corba's dragon flight (report a014): the Mini Draco Transformation
/// (Morph 16) that Corba casts and the flight's portals take off and put
/// back on, Leafre Station, the flight's second map and the Temple of
/// Time's road.
pub const MINI_DRACO: i32 = 2210016;
pub const FLIGHT_TO_TEMPLE: i32 = 200090500;
const LEAFRE_STATION: i32 = 240000110;
const FLIGHT_TO_MINAR: i32 = 200090510;
const TEMPLE_ROAD: i32 = 270000100;
/// The Haunted House's first hall, past the grounds' gate.
const HAUNTED_HOUSE: i32 = 682000100;
/// The Mushroom Castle's way in and through (Cosmic's `td_mushcastle`).
const MUSHROOM_FOREST_FIELD: i32 = 106020000;
/// The Mushroom Kingdom's opening scene (`TD_MC_Openning`), and the record
/// it sets, which The Test (2312) needs to start.
const MUSHROOM_OPENING: i32 = 106020001;
const MUSHROOM_OPENING_RECORD: i16 = 2311;
const MUSHROOM_OPENING_SEEN: &str = "1";
/// The least level the Empty House lets through to the kingdom.
const MUSHROOM_KINGDOM_LEVEL: u8 = 30;
/// The opening scene's parts (`Effect.nx/Direction2.img/open`), in the order
/// `TD_MC_Openning` reserves them; its `out` part leads to the forest.
const MUSHROOM_OPENING_SCENE: [&str; 10] = [
    "back0", "back1", "light", "pepeKing", "line", "violeta0", "violeta1", "frame", "chat", "out",
];
const SPLIT_ROAD: i32 = 106020400;
const CASTLE_WALL_EDGE: i32 = 106020501;
/// The castle wall's edge before its thorns are cleared, where Over the
/// Castle Walls (2) has the player look at the wall (`investigate2`).
const THORNY_WALL_EDGE: i32 = 106020500;
/// Exploring Mushroom Forest (1) and Over the Castle Walls (2): looking at
/// the forest's barrier (`investigate1`, beside `obstacle`) and at the
/// castle wall sets the record each completes on (`OWN_RECORD_QUESTS`).
const EXPLORING_MUSHROOM_FOREST: i16 = 2314;
const OVER_THE_CASTLE_WALLS: i16 = 2322;
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
/// Yulete's lab, the dark pipe's end, and its way out to Magatia
/// (`out00`), where the pipe leads.
const YULETE_LAB: i32 = 926130000;
const YULETE_LAB_WAY_OUT: u8 = 2;
/// How near a scripted item's portal the player must stand.
const ITEM_REACH: f64 = 210.0;
/// Kerning City's subway: Line 1's first area, and its second, where the
/// construction site's tunnel comes out.
const LINE_1_AREA_1: i32 = 103000101;
const LINE_1_AREA_2: i32 = 103000103;
/// Find Fresh Milk, whose bottles run empty to full; the stable's way out
/// (`end_cow`) leads back to Tangyoon's kitchen once the bottle is full.
pub(crate) const FRESH_MILK: i16 = 2180;
pub(crate) const MILK_BOTTLES: [i32; 4] = [4031847, 4031848, 4031849, 4031850];
const TANGYOONS_KITCHEN: i32 = 120000103;
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
/// Ellinia, where the hidden Magic Library lets out by `enterMagiclibrar`'s
/// own portal, `jobin00`.
const ELLINIA: i32 = 101000000;
/// The hidden Magic Library, 910110000, where a Cygnus Knight on "Maybe
/// it's Grendel!" (20718) searching Grendel's house finds a black shadow
/// instead (Cosmic `Cygnus_Magic_Library`), one at a time. Its black shadow
/// sets `LIBRARY_SHADOW_RECORD` to `SHADOW_SPOTTED`, which the quest's
/// completion reads.
pub const HIDDEN_MAGIC_LIBRARY: i32 = 910110000;
pub const MAYBE_ITS_GRENDEL: i16 = 20718;
pub const LIBRARY_SHADOW_RECORD: i16 = 20732;
pub const SHADOW_SPOTTED: &str = "1";
const UNDERGROUND_TEMPLE_STAIRS: i32 = 105100000;
/// Orbis's Neglected Strolling Path (`enterNepenthes`).
const NEGLECTED_STROLLING_PATH: i32 = 200060001;
/// The way into the Sealed Garden and the garden itself, which
/// `enterNepenthes` lets one Aran on To the Sealed Garden (21739) into
/// instead of the path. The garden's `sealGarden` brings out its guardian
/// for one who hasn't beaten it; Cosmic brings it out from the portal.
pub const SEALED_GARDEN_WAY: i32 = 920030000;
pub const SEALED_GARDEN: i32 = 920030001;
const TO_THE_SEALED_GARDEN: i16 = 21739;
const GARDEN_GUARDIAN: i32 = 9300348;
const GARDEN_GUARDIAN_AT: (i16, i16) = (591, -34);
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
pub const VERIFYING_THE_PASSWORD: i16 = 3360;
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
const PLAIN_WARPS: [(&str, i32, Portal); 17] = [
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
    // The Road in Between, back to the Entrance to Horntail's Cave.
    ("hontale_out1", 240050400, Portal::Id(0)),
    ("eliza_Garden", 920020000, Portal::Id(2)),
    ("catPriest_map", 925000000, Portal::Id(2)),
    // Lith Harbor's Info Shop; entering it brings out 21733's Puppeteer
    // (`unscripted_map_enter`), however the Aran comes in.
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
/// Romeo and Juliet's party quest's rope rooms, by side: the landing's
/// ways into them, each taken by one member, and their ropes, which the
/// client actor answers from the run's claims and the field's draw.
pub const MAGATIA_ROOMS_IN: [[&str; 4]; 2] = [
    ["rnj4_r1", "rnj4_r2", "rnj4_r3", "rnj4_r4"],
    ["jnr4_r1", "jnr4_r2", "jnr4_r3", "jnr4_r4"],
];
pub const MAGATIA_ROPES: [&str; 2] = ["rnj5_rp", "jnr5_rp"];
/// The Orbis party quest's way out of the garden, back to the hub.
pub const OPQ_GARDEN_OUT: &str = "party3_gardenin";
/// The hub's door to the hidden storage (`in06`), open once the event is
/// won.
pub const OPQ_STORAGE_IN: &str = "party3_room8";
/// The doors of the sealed room (920010900) to its three jails, open once
/// the event is won, as the storage is.
pub const OPQ_JAILS_IN: [&str; 3] = ["party3_jail1", "party3_jail2", "party3_jail3"];
/// A jail's way on (`in00`) to its box room, open to the levers that
/// match the jail's draw (`events::jail_misplaced`).
pub const OPQ_JAIL_ON: &str = "party3_jailin";
/// The Dead Mine's way out, on its hub (`ps01`), which the client actor
/// answers from the player's ticket.
pub const ZPQ_OUT: &str = "Zakum03";
/// The way to Papulatus at Deep Inside the Clocktower (`in00`), which the
/// client actor answers once the directory says whether the party may go
/// (`papulatus_portal`).
pub const PAPULATUS_IN: &str = "Populatus00";
/// The way from the Trial Grounds' first map into their rooms (`in00`),
/// which the client actor answers once it knows who else is on the map
/// (`htpq_into_rooms`).
pub const HTPQ_INTO_ROOMS: &str = "hontale_BtoB1";
/// The doors out of the Trial Grounds' rooms (`hontale_Bopen`): rooms 1 to
/// 4's open once their seals take their keys; the last room's the leader
/// lifts with the Blue Crystal Key (`htpq_last_seal`).
pub const HTPQ_SEAL: &str = "hontale_Bopen";
/// The lamp's room's way on (`htpq_lamp_door`), and the name of its lamp
/// (`reactors::HTPQ_LAMP`): lit (state 1) it leads to the light map, dark
/// (state 3) to the dark one.
pub const HTPQ_LAMP_DOOR: &str = "hontale_C";
pub const HTPQ_LAMP_NAME: &str = "light";
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
/// while quest 2335, which Violetta gives it for, is taken, and the door
/// completes the quest.
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
/// The Junior Balrog's other world, for the Bowmen's 4th-job quest 6108.
pub const BALROG_WORLD: i32 = 910500000;
/// Where Ardin's other self waits for a challenger on quest 3933.
pub const ARDINS_ROOM: i32 = 926000000;
/// The Puppeteer's hideout, where Francis (1104000) sends an intruder from
/// his room, and the Puppeteer each hunt brings out there on entry (Cosmic
/// map script `910510000`): (quest, mob), Find the Puppeteer! for the
/// Cygnus Knights and Eliminate the Puppeteer! for an Aran.
pub const PUPPETEERS_HIDEOUT: i32 = 910510000;
const PUPPETEER_HUNTS: [(i16, i32); 2] = [(20730, 9300285), (21731, 9300344)];
const PUPPETEER_AT: (i16, i16) = (680, 258);
/// The Puppeteer's arena at the end of the Secret Passage, which its end
/// statue (1063017) lets one challenger into. Its `dollCave02` brings out
/// the Puppeteer for an Aran on Traces of the Puppeteer (21734) who hasn't
/// beaten it yet; Cosmic brings it out from the statue instead.
pub const PUPPETEERS_ARENA: i32 = 910510202;
const TRACES_OF_THE_PUPPETEER: i16 = 21734;
const ARENA_PUPPETEER: i32 = 9300346;
const ARENA_PUPPETEER_AT: (i16, i16) = (95, 200);
/// Mu Gong's back alley behind the Mu Lung Dojo (925040000), which the
/// Dojo's `enterBackStreet` lets an Aran into while Mu Gong has business
/// with them (Cosmic `enterBackStreet`), and Mu Gong's arena for the Test
/// (21746), one challenger at a time, which `outSpecialSchool` leaves.
pub const BACK_STREET: i32 = 925040000;
pub const MU_GONGS_ARENA: i32 = 925040001;
const SHADOW_KNIGHTS_LETTER: i16 = 21744;
const HOW_TO_MEET_MU_GONG: i16 = 21745;
/// The Sealed Shrine, which the Entrance pillar (2091009) lets an Aran on
/// Catch the Shadow Knight! (21747) into, one at a time; its Shadow Knight
/// comes out on entry until it falls (Cosmic map script `925040100`).
pub const SEALED_SHRINE: i32 = 925040100;
pub const CATCH_THE_SHADOW_KNIGHT: i16 = 21747;
const SHADOW_KNIGHT: i32 = 9300351;
const SHADOW_KNIGHT_AT: (i16, i16) = (897, 51);
/// Ellin Forest's Warehouse (300000011): an Aran who has opened it for
/// Athena Pierce (21751) sees the Gentleman of the Black Wings make off
/// with its seal stone, which sets the record The Lost Seal Stone of Ellin
/// Forest (21753) asks for. Neither Cosmic nor the data says what sets it;
/// the scene's Gentleman (9300353) leaves by itself.
const ELLIN_WAREHOUSE: i32 = 300000011;
/// The Spirit of Rock's floors past their first (`crate::events`): the
/// Fancy Amplifiers' (420, 450), reached on Composition Fee (2287) and
/// left for the Spirit of Rock's stage on Say "NO" to Plagiarism (2288).
const FANCY_AMPLIFIER_FLOOR: i32 = 103040420;
const VIP_FANCY_AMPLIFIER_FLOOR: i32 = 103040450;
const COMPOSITION_FEE: i16 = 2287;
const SAY_NO_TO_PLAGIARISM: i16 = 2288;
const SEAL_STONE_OF_ELLIN: i16 = 21751;
const WAREHOUSE_RECORD: i16 = 21764;
const WAREHOUSE_SEEN: &str = "1";
const WAREHOUSE_GENTLEMAN: i32 = 9300353;
const WAREHOUSE_GENTLEMAN_AT: (i16, i16) = (200, 100);
/// Back in their own time, an Aran who has heard from Tru of Athena Pierce
/// (21753) finds Athena at the Bowman Instructional School, which sets the
/// record Info on Another Seal Stone (21754) asks for (its `QuestInfo/0`;
/// the setter is unsourced too).
const LOST_SEAL_STONE: i16 = 21753;
const ATHENA_FOUND_RECORD: i16 = 21765;
const ATHENA_FOUND: &str = "2";
/// The Field of Wolves past El Nath's Wolf-Shaped Rock, which its
/// `enterRider` lets an Aran on Scadur's Advice (21610) run, one at a time
/// (Cosmic `enterRider`), until its end (`clearRider`) sets the record the
/// quest ends on. Neither Cosmic nor the data says what sets it; Cosmic's
/// own data asks for a token instead.
pub const FIELD_OF_WOLVES: i32 = 921110000;
const SCADURS_ADVICE: i16 = 21610;
const SCADURS_ADVICE_RECORD: i16 = 21619;
/// The wolves' test room (914030000), which the wolf (1202008) sends an
/// Aran on The Wolves' Test (21613) into, one at a time; once every wolf
/// in it is down, its `out00` sets the record the quest ends on and leads
/// back to the wolf (Cosmic `outtestWolf` gives a token instead).
pub const WOLVES_TEST_ROOM: i32 = 914030000;
pub const THE_WOLVES_TEST: i16 = 21613;
pub const WOLVES_TEST_RECORD: i16 = 21620;
pub const WOLVES_DEN: i32 = 140010210;
/// What both records read once their run is done (`Check/1 infoex`).
pub const WOLF_RUN_DONE: &str = "0";
pub const SOLO_MAPS: [i32; 21] = [
    FIREHAWK_NEST,
    ICE_EAGLE_NEST,
    TORNADO_ROOM,
    RESURRECTION_ROOM,
    EGG_NEST,
    HITMAN_HIDEOUT,
    BERSERK_ROOM,
    crate::reactors::BART_ROOM,
    KYRIN_TESTS[0].room,
    KYRIN_TESTS[1].room,
    BALROG_WORLD,
    ARDINS_ROOM,
    HIDDEN_MAGIC_LIBRARY,
    PUPPETEERS_HIDEOUT,
    PUPPETEERS_ARENA,
    SEALED_GARDEN_WAY,
    SEALED_GARDEN,
    MU_GONGS_ARENA,
    SEALED_SHRINE,
    FIELD_OF_WOLVES,
    WOLVES_TEST_ROOM,
];

/// The Puppeteer `player` is out to beat, while a hunt for him is under
/// way and he isn't yet beaten.
pub fn hunted_puppeteer(player: &Player) -> Option<i32> {
    PUPPETEER_HUNTS
        .iter()
        .find(|&&(quest, _)| {
            player.quests.state(quest) == QuestState::Started
                && player.quests.kills(quest).iter().all(|&count| count == 0)
        })
        .map(|&(_, mob)| mob)
}

/// One of Kyrin's 4th-job tests: a Pirate on `quest` holds out in `room`
/// against its boss for two minutes, then tells Kyrin in
/// `KYRIN_PASSED_ROOM`, who sets `record` to `KYRIN_PASSED`.
#[derive(Debug, PartialEq, Eq)]
pub struct KyrinTest {
    pub quest: i16,
    pub record: i16,
    pub room: i32,
}

/// Super Transformation (6330, a Marauder's) and Battleship (6370, an
/// Outlaw's).
pub const KYRIN_TESTS: [KyrinTest; 2] = [
    KyrinTest {
        quest: 6330,
        record: 6331,
        room: 912010000,
    },
    KyrinTest {
        quest: 6370,
        record: 6371,
        room: 912010100,
    },
];
pub const KYRIN_PASSED_ROOM: i32 = 912010200;
/// What a test's record reads once passed (`Check/1 infoex`).
pub const KYRIN_PASSED: &str = "2";
/// How long a test lasts (Cosmic's `4jship`), and when its room's way out
/// leads where: before `KYRIN_MAY_LEAVE` nowhere, then back to Kyrin with
/// nothing, and from `KYRIN_HELD_OUT` to her, passed. Being an event's
/// times, they are shortened by `RUSTMS_EVENT_SCALE` for testing.
const KYRIN_TIME_LIMIT: Duration = Duration::from_secs(4 * 60);
const KYRIN_MAY_LEAVE: Duration = Duration::from_secs(60);
const KYRIN_HELD_OUT: Duration = Duration::from_secs(2 * 60);
/// The ways out of the two rooms (the scripts' names are crossed with the
/// rooms' event names, but both do the same).
pub const KYRIN_TEST_OUT: [&str; 2] = ["s4super_out", "s4ship_out"];
const KYRIN_MAP: i32 = 120000101;

/// A Kyrin test's time limit, in seconds.
pub fn kyrin_time_limit() -> u32 {
    let limit = crate::events::scaled(KYRIN_TIME_LIMIT).as_secs();
    u32::try_from(limit).unwrap_or(u32::MAX).max(1)
}

/// A Kyrin test's way out, `elapsed` after the Pirate came in.
pub fn kyrin_test_out(elapsed: Duration) -> Vec<Effect> {
    use crate::events::scaled;
    if elapsed < scaled(KYRIN_MAY_LEAVE) {
        let wait = crate::content::text().get(keys::KYRIN_TEST_WAIT, &[]);
        return vec![Effect::Pink(wait)];
    }
    let to = if elapsed < scaled(KYRIN_HELD_OUT) {
        KYRIN_MAP
    } else {
        KYRIN_PASSED_ROOM
    };
    vec![Effect::PortalSound, Effect::WarpToPortal(to, 0)]
}
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

/// Whether the player is disguised as Tigun by Tigun's potion.
pub fn disguised_as_tigun(player: &Player) -> bool {
    (player.buffs.active().iter())
        .any(|buff| buff.stat == BuffStat::Morph && buff.skill_id == -TIGUNS_DISGUISE)
}
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
    /// Play `Sound.nx/Field.img/<path>` for the player, as a missed or
    /// finished tune's jingle (`Party1/Failed`, `Party1/Clear`).
    Sound(String),
    /// Play `Sound.nx/Field.img/<path>` for everyone on the player's map,
    /// as a string of Eliza's harp sounds its note.
    SoundToMap(String),
    /// A UI image anchored to the player (`UI/tutorial.img/<n>` hints).
    AvatarEffect(String),
    /// A scene the client plays from `Effect.nx` (`Direction2.img/open`),
    /// one path per part, with the UI locked; it ends by asking for `exit`
    /// (`Player::scene_exit`), which the scene's own data names.
    Scene {
        parts: Vec<String>,
        exit: i32,
    },
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
    /// A page the player thinks to themselves (`NpcScript::thought`), as a
    /// portal shows one. Not over a conversation already open.
    Think(String),
    SetHp(i16),
    GiveItem {
        item_id: i32,
        quantity: i16,
    },
    /// One piece of equipment with its stats varied as a drop's are
    /// (`items::give_dropped`), as Mos forges with a stimulator.
    GiveVariedItem(i32),
    /// Trade a `ticket` (a Gachapon Ticket or a Remote Gachapon Ticket)
    /// for the prize rolled when the machine in `town` was opened
    /// (`gachapon::roll`). Nothing is applied if the ticket is gone or the
    /// prize doesn't fit; the world hears of an `announced` prize.
    Gachapon {
        ticket: i32,
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
    /// Evolve the pet with cash serial `serial` into `into`, which lives
    /// `days` from now; one out is shown anew. Nothing is applied if the
    /// player no longer has it.
    EvolvePet {
        serial: i64,
        into: i32,
        days: i32,
    },
    /// Nothing is applied if the player no longer has that much.
    SpendMeso(i32),
    /// Up to the most a player may hold.
    GainMeso(i32),
    GainExp(i32),
    /// Fame, shown in the chat log as a quest's is.
    GainFame(i16),
    StartQuest(i16),
    CompleteQuest(i16),
    /// Start a quest not yet started and complete it at once: a start script
    /// that completes its own quest (Cosmic's `forceCompleteQuest`), as the
    /// informants of quests 2216-2219 do. Checked like a start.
    FinishQuest(i16),
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
    /// Add to the player's guild's points; nothing without a guild.
    GainGuildPoints(i32),
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
    /// Add to the player's Mu Lung Dojo training points
    /// (`DojoRecord::floor_points`).
    DojoPoints(i32),
    /// Record the Dojo's rest floor `stage` for the player to start after
    /// next time (So Gong), or erase the record with 0.
    DojoStage(u8),
    /// Set the Dojo medal the player tries for (from 1; 0 for none) and the
    /// times its boss has been beaten toward it.
    DojoMedal {
        stage: i32,
        kills: i32,
    },
    /// Mark So Gong's first-time trial beaten, for the Dojo itself to open.
    DojoTrialDone,
    /// Play the portal sound, which the client leaves to the server for a
    /// scripted portal.
    PortalSound,
    /// Put an item's effect on the player without taking one (Cosmic's
    /// `useItem`), before any warp.
    ApplyItemBuff(i32),
    /// End an item's buff, if on, before any warp.
    CancelItemBuff(i32),
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
    /// Bring out `each` of `mob_id` on the ground below each point of `at`
    /// in the player's map, as the Magic Library's black shadow sets Blue
    /// Mushrooms on the player. They never respawn.
    BringOutMobs {
        mob_id: i32,
        at: &'static [(i16, i16)],
        each: u8,
    },
    /// Put the reactors named `name` on the player's map in `state`, shown
    /// to everyone there but without the events that lead to it, as
    /// Cosmic's `forceHitReactor` (Eak bringing out a stone box). Those
    /// already in it or past it stay as they are.
    SetReactorState {
        name: &'static str,
        state: u8,
    },
    /// Open the reactors named `name` on the player's map, shown to
    /// everyone there, until their open state times out, as Cosmic's
    /// `hitReactor` on a reactor whose first state has no events.
    OpenReactor(&'static str),
    /// NPC `npc_id` on the player's map plays its special action `action`
    /// (`Npc.nx/<npc>.img/<action>`, flagged `special`), seen by everyone
    /// there, as Pia uses the rainbow snail shell.
    NpcSpecialAction {
        npc_id: i32,
        action: &'static str,
    },
    /// Every mob on the player's map that isn't friendly goes, and no more
    /// come, as Cosmic's `killAllMonsters`.
    ClearMobs,
    /// Set character `slot` of a started info quest's string
    /// (`game::quest::INFO_QUESTS`) from `from` to `to`, as Ariant's houses
    /// mark each delivery. Nothing is applied unless it reads `from`.
    SetQuestChar {
        quest: i16,
        slot: u8,
        from: u8,
        to: u8,
    },
    /// Set a started info quest's whole string to `value`, as the Shiny
    /// Stone marks Shining Stone; nothing if it already reads so. After a
    /// `StartQuest` of the same quest, it is the string the quest starts
    /// with, as Parwen's password.
    SetQuestInfo {
        quest: i16,
        value: String,
    },
    /// Set a record that is no quest (`game::quest::RECORDS`) to `value`,
    /// as Kyrin marks a test passed; nothing if it already reads so.
    SetRecord {
        record: i16,
        value: &'static str,
    },
    /// Set `key` in a quest's record beside the quest log
    /// (`game::quest_ex`), which its `#j` tokens show, as a title challenge
    /// keeps the fame it started from; nothing if it already reads so.
    SetQuestEx {
        quest: i16,
        key: &'static str,
        value: String,
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
    /// Take Monster Carnival room `0` for the player's party, or challenge
    /// the party waiting there. Says how it went in a script of its own.
    CarnivalRoom(u8),
    /// Take a place at the event a GM opened on the player's channel, keep
    /// a team there, and go to the team's spot on its field, to come back
    /// to the map they left (`EVENT_RETURN`). Says so in a script of its
    /// own, by the Event Assistant `npc`, if it closed or filled up
    /// meanwhile.
    JoinGmEvent {
        npc: i32,
    },
    /// The player gives back their place at the event on `map_id`, if it
    /// is still open on their channel.
    LeaveGmEvent {
        map_id: i32,
    },
    /// The leader of a party waiting in a Monster Carnival room accepts
    /// or declines the challenge put to them by `challenger`'s party.
    CarnivalAnswer {
        challenger: crate::ClientId,
        accept: bool,
    },
    /// Tell the player's event something happened, once the effects before
    /// it have applied. Scripts end on it.
    EventCommand(crate::events::EventCommand),
    /// Tell the squad of `kind` on the player's channel something
    /// (`crate::squads`). Nothing after it is applied if it won't take it;
    /// the script says its `refused` text instead.
    Squad {
        kind: crate::events::EventKind,
        command: crate::squads::SquadCommand,
    },
    /// Register the player's guild for the Guild Quest on their channel, or
    /// let them into its waiting room (`crate::guild_quest`). Nothing after
    /// it is applied if the directory refuses; the script says its
    /// `refused` text instead.
    GuildQuest(crate::guild_quest::Command),
    /// Nothing is applied while the reactor named `name` on the player's
    /// map is in `state` (a missing reactor shuts nothing); the script says
    /// its `refused` text, or the portal its `occupied` one, instead.
    ShutByReactor {
        name: &'static str,
        state: u8,
    },
    /// Nothing after it is applied unless every reactor named here on the
    /// player's map has left state 0 (a missing one hasn't); the portal
    /// says its `occupied` line instead.
    RequireReactorsSet(&'static [&'static str]),
    /// Nothing after it is applied unless the player's event has cleared
    /// `stage`; they hear `refused` (a key) instead, as an item line, or
    /// without it a portal's `occupied` line.
    RequireStage {
        stage: u8,
        refused: Option<&'static str>,
    },
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
    /// Said in pink when a `RequireEmptyMap`, `RequireNoMobs`,
    /// `ShutByReactor` or `RequireStage` keeps the player back.
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

/// Lith Harbor's Info Shop, where the Puppeteer lies in wait for an Aran
/// on "Gathering Some Strange Information" (21733).
pub const LITH_INFO_SHOP: i32 = 104000004;
const PUPPETEER_IN_LITH: i32 = 9300345;
/// On the shop's floor, between the door and Tru (Cosmic: the ground below
/// (0, 0)).
const PUPPETEER_IN_LITH_AT: (i16, i16) = (0, 120);
const STRANGE_INFORMATION: i16 = 21733;
const STRANGE_INFORMATION_RECORD: i16 = 21762;

/// What the map `player` just entered does when it has no `onUserEnter`
/// script: the Puppeteer's hideout brings out the Puppeteer being hunted,
/// and Lith Harbor's Info Shop the Puppeteer an Aran on 21733 hasn't
/// beaten yet (Cosmic `enterInfo`). His fall sets 21733's record
/// (`crate::quests::kill_records`).
pub fn unscripted_map_enter(player: &Player) -> Vec<Effect> {
    match player.stats.map_id {
        PUPPETEERS_HIDEOUT => hunted_puppeteer(player)
            .map(|mob_id| Effect::BringOutMob {
                mob_id,
                at: PUPPETEER_AT,
            })
            .into_iter()
            .collect(),
        LITH_INFO_SHOP
            if player.quests.state(STRANGE_INFORMATION) == QuestState::Started
                && player.quests.info(STRANGE_INFORMATION_RECORD) != Some("2") =>
        {
            vec![Effect::BringOutMob {
                mob_id: PUPPETEER_IN_LITH,
                at: PUPPETEER_IN_LITH_AT,
            }]
        }
        SEALED_SHRINE
            if player.quests.state(CATCH_THE_SHADOW_KNIGHT) == QuestState::Started
                && (player.quests.kills(CATCH_THE_SHADOW_KNIGHT).iter())
                    .all(|&count| count == 0) =>
        {
            vec![Effect::BringOutMob {
                mob_id: SHADOW_KNIGHT,
                at: SHADOW_KNIGHT_AT,
            }]
        }
        ELLIN_WAREHOUSE
            if player.quests.state(SEAL_STONE_OF_ELLIN) == QuestState::Completed
                && player.quests.info(WAREHOUSE_RECORD) != Some(WAREHOUSE_SEEN) =>
        {
            vec![
                Effect::SetRecord {
                    record: WAREHOUSE_RECORD,
                    value: WAREHOUSE_SEEN,
                },
                Effect::BringOutMob {
                    mob_id: WAREHOUSE_GENTLEMAN,
                    at: WAREHOUSE_GENTLEMAN_AT,
                },
            ]
        }
        // Shuang tells whoever reaches Sharenian's outskirts what to do there.
        crate::guild_quest::OUTSKIRTS => vec![Effect::OpenNpc(crate::guild_quest::SHUANG)],
        BOWMAN_SCHOOL
            if player.quests.state(LOST_SEAL_STONE) == QuestState::Completed
                && player.quests.info(ATHENA_FOUND_RECORD).is_none() =>
        {
            vec![Effect::SetRecord {
                record: ATHENA_FOUND_RECORD,
                value: ATHENA_FOUND,
            }]
        }
        _ => Vec::new(),
    }
}

/// The map `player` just entered runs its `onUserEnter` script. `None` if
/// the script is unknown.
pub fn map_enter(script: &str, player: &Player) -> Option<Vec<Effect>> {
    let map_id = player.stats.map_id;
    match script {
        // Leafre Station takes the dragon off whoever lands there, the
        // flight's timer included (no source; by its name).
        "undomorphdarco" => Some(vec![Effect::CancelItemBuff(MINI_DRACO)]),
        // The Mushroom Kingdom's opening (Edelstein `TD_MC_Openning`): it
        // marks the scene seen (The Test, 2312, waits for that) and plays it.
        "TD_MC_Openning" => Some(vec![
            Effect::SetRecord {
                record: MUSHROOM_OPENING_RECORD,
                value: MUSHROOM_OPENING_SEEN,
            },
            Effect::Scene {
                parts: (MUSHROOM_OPENING_SCENE.iter())
                    .map(|part| format!("Effect/Direction2.img/open/{part}"))
                    .collect(),
                exit: MUSHROOM_FOREST_FIELD,
            },
        ]),
        "dollCave02" => Some(
            (player.quests.state(TRACES_OF_THE_PUPPETEER) == QuestState::Started
                && (player.quests.kills(TRACES_OF_THE_PUPPETEER).iter()).all(|&count| count == 0))
            .then_some(Effect::BringOutMob {
                mob_id: ARENA_PUPPETEER,
                at: ARENA_PUPPETEER_AT,
            })
            .into_iter()
            .collect(),
        ),
        "sealGarden" => Some(
            (player.quests.state(TO_THE_SEALED_GARDEN) == QuestState::Started
                && (player.quests.kills(TO_THE_SEALED_GARDEN).iter()).all(|&count| count == 0))
            .then_some(Effect::BringOutMob {
                mob_id: GARDEN_GUARDIAN,
                at: GARDEN_GUARDIAN_AT,
            })
            .into_iter()
            .collect(),
        ),
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
        // The Wedding Hall: Violetta is found, which 2332's own record
        // tells (its `Check/1 infoex` asks for "1"), and the Prime Minister
        // comes out for one on 2333 who has not beaten him (Cosmic
        // `MK_PrimeMinister`).
        "findvioleta" => {
            let quests = &player.quests;
            let mut effects = Vec::new();
            if quests.state(WHERES_VIOLETTA) == QuestState::Started {
                effects.push(Effect::QuestRecord {
                    quest_id: WHERES_VIOLETTA,
                    count: 1,
                });
            }
            if quests.state(STORY_OF_BETRAYAL) == QuestState::Started
                && quests
                    .kills(STORY_OF_BETRAYAL)
                    .iter()
                    .all(|&count| count == 0)
            {
                effects.push(Effect::BringOutMob {
                    mob_id: PRIME_MINISTER,
                    at: PRIME_MINISTER_AT,
                });
            }
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
/// Where the way of a Magatia hub's script `rest` (its name after the
/// side's `rnj`/`jnr`) leads on `side`: the reactor it waits on, and the
/// map and portal it leads to.
fn magatia_hub_way(
    side: crate::events::MagatiaSide,
    rest: &str,
) -> Option<(&'static [&'static str], i32, u8)> {
    use crate::events::MagatiaSide;
    let stages = side.stages();
    let hub = stages[3];
    let zenumist = side == MagatiaSide::Zenumist;
    let pick = |rnj: &'static [&'static str], jnr| if zenumist { rnj } else { jnr };
    Some(match rest {
        "3_in0" => (pick(&["rnj3_out1"], &["jnr3_out1"]), hub + 1, 0),
        "3_in1" => (pick(&["rnj3_out2"], &["jnr3_out2"]), hub + 2, 0),
        "_201_0" => (pick(&["rnj31_out"], &["jnr31_out"]), hub, 1),
        "_202" => (pick(&["rnj32_out"], &["jnr32_out"]), hub, 2),
        "3_out" => (pick(&["rnj3_out3"], &["jnr3_out3"]), stages[4], 0),
        _ => return None,
    })
}

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
        // Corba's flight: into the Temple of Time on the dragon's last map,
        // back out into the air, and down to the station on touching the
        // ground (Cosmic's `templeenter`, `outTemple`, `undodraco`).
        "templeenter" => ScriptOutcome {
            effects: vec![
                Effect::CancelItemBuff(MINI_DRACO),
                Effect::PortalSound,
                Effect::WarpToPortal(
                    TEMPLE_ROAD,
                    portal_id(TEMPLE_ROAD, |portal| portal.name == "out00"),
                ),
            ],
            block: false,
            occupied: None,
        },
        "outTemple" => ScriptOutcome {
            effects: vec![
                Effect::ApplyItemBuff(MINI_DRACO),
                Effect::PortalSound,
                Effect::WarpToPortal(FLIGHT_TO_MINAR, 0),
            ],
            block: false,
            occupied: None,
        },
        "undodraco" => ScriptOutcome {
            effects: vec![
                Effect::CancelItemBuff(MINI_DRACO),
                Effect::PortalSound,
                Effect::WarpToPortal(
                    LEAFRE_STATION,
                    portal_id(LEAFRE_STATION, |portal| portal.name == "po00"),
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
        // An Empty House's way to the Mushroom Kingdom, from level 30: the
        // first time through the kingdom's opening scene, which leads on to
        // its forest (Edelstein's `TD_MC_first`).
        "TD_MC_first" if player.stats.level < MUSHROOM_KINGDOM_LEVEL => {
            refused(keys::MC_FIRST_BLOCKED)
        }
        "TD_MC_first"
            if player.quests.info(MUSHROOM_OPENING_RECORD) == Some(MUSHROOM_OPENING_SEEN) =>
        {
            through(Effect::Warp(MUSHROOM_FOREST_FIELD))
        }
        "TD_MC_first" => through(Effect::Warp(MUSHROOM_OPENING)),
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
        // Over the castle wall, once the Thorn Remover quest is done; up to
        // its thorns while Over the Castle Walls (2) has the player look at
        // it.
        "gotocastle" if quest(2324) == QuestState::Completed => {
            through(Effect::Warp(CASTLE_WALL_EDGE))
        }
        "gotocastle" if quest(OVER_THE_CASTLE_WALLS) == QuestState::Started => {
            through(Effect::Warp(THORNY_WALL_EDGE))
        }
        // The barrier and the wall, looked at: the quest's record turns to
        // 1, and the player thinks it over under a balloon
        // (`OnUserEff.img/normalEffect/mushroomcastle/chatBalloon<n>`).
        "investigate1" => investigate(player, EXPLORING_MUSHROOM_FOREST, 1),
        "investigate2" => investigate(player, OVER_THE_CASTLE_WALLS, 2),
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
        "end_cow"
            if quest(FRESH_MILK) == QuestState::Started
                && MILK_BOTTLES[..3]
                    .iter()
                    .any(|&bottle| player.inventory.count(bottle) > 0)
                && player.inventory.count(MILK_BOTTLES[3]) == 0 =>
        {
            refused(keys::MILK_JUG_NOT_FULL)
        }
        "end_cow" => through(Effect::Warp(TANGYOONS_KITCHEN)),
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
        // Into the hidden Magic Library while Maybe it's Grendel! (20718)
        // is under way and its black shadow not yet found, by the portal
        // Cosmic's instance enters by; else into Grendel's house.
        "enterMagiclibrar"
            if quest(MAYBE_ITS_GRENDEL) == QuestState::Started
                && player.quests.info(LIBRARY_SHADOW_RECORD) != Some(SHADOW_SPOTTED) =>
        {
            solo_map_at(true, HIDDEN_MAGIC_LIBRARY, 1)
        }
        "enterMagiclibrar" => through(Effect::WarpToPortal(
            MAGIC_LIBRARY,
            named(MAGIC_LIBRARY, "jobout00"),
        )),
        // Out of the hidden Magic Library to Ellinia, once the black
        // shadow's Blue Mushrooms are all gone (Cosmic `outMagiclib`).
        "outMagiclib" => ScriptOutcome {
            effects: vec![
                Effect::RequireNoMobs,
                Effect::PortalSound,
                Effect::WarpToPortal(ELLINIA, named(ELLINIA, "jobin00")),
            ],
            block: false,
            occupied: Some(content::text().get(keys::MAGIC_LIBRARY_MUSHROOMS_LEFT, &[])),
        },
        "balogTemple" => through(Effect::WarpToPortal(
            UNDERGROUND_TEMPLE_STAIRS,
            named(UNDERGROUND_TEMPLE_STAIRS, "top00"),
        )),
        // An Aran on To the Sealed Garden (21739) goes to its fight, alone.
        "enterNepenthes" if quest(TO_THE_SEALED_GARDEN) == QuestState::Started => ScriptOutcome {
            effects: vec![
                Effect::RequireEmptyMap(SEALED_GARDEN_WAY),
                Effect::RequireEmptyMap(SEALED_GARDEN),
                Effect::PortalSound,
                Effect::WarpToPortal(SEALED_GARDEN_WAY, named(SEALED_GARDEN_WAY, "out00")),
            ],
            block: false,
            occupied: Some(content::text().get(keys::SEALED_GARDEN_OCCUPIED, &[])),
        },
        "enterNepenthes" => through(Effect::WarpToPortal(
            NEGLECTED_STROLLING_PATH,
            named(NEGLECTED_STROLLING_PATH, "out00"),
        )),
        // Cosmic opens it only for 21747 or for the letter, which shuts
        // an Aran out between them (Mu Gong's Test included).
        "enterBackStreet"
            if (quest(SHADOW_KNIGHTS_LETTER) == QuestState::Started
                && quest(HOW_TO_MEET_MU_GONG) == QuestState::Completed)
                || (quest(SHADOW_KNIGHTS_LETTER) == QuestState::Completed
                    && quest(CATCH_THE_SHADOW_KNIGHT) != QuestState::Completed) =>
        {
            through(Effect::WarpToPortal(BACK_STREET, 0))
        }
        "enterBackStreet" => refused(keys::BACK_STREET_NO_ENTRY),
        "outSpecialSchool" => through(Effect::WarpToPortal(
            BACK_STREET,
            named(BACK_STREET, "st00"),
        )),
        "enterRider" => solo_map_at(
            quest(SCADURS_ADVICE) == QuestState::Started
                && player.quests.info(SCADURS_ADVICE_RECORD) != Some(WOLF_RUN_DONE),
            FIELD_OF_WOLVES,
            named(FIELD_OF_WOLVES, "st00"),
        ),
        // Touching the Field of Wolves' end, where `clear01` leads out.
        "clearRider"
            if quest(SCADURS_ADVICE) == QuestState::Started
                && player.quests.info(SCADURS_ADVICE_RECORD) != Some(WOLF_RUN_DONE) =>
        {
            ScriptOutcome {
                effects: vec![
                    Effect::SetRecord {
                        record: SCADURS_ADVICE_RECORD,
                        value: WOLF_RUN_DONE,
                    },
                    Effect::Pink(content::text().get(keys::FIELD_OF_WOLVES_CLEARED, &[])),
                ],
                block: false,
                occupied: None,
            }
        }
        "clearRider" => ScriptOutcome::default(),
        // Out of the wolves' test room once every wolf is down.
        "outtestWolf" => ScriptOutcome {
            effects: [Effect::RequireNoMobs]
                .into_iter()
                .chain((quest(THE_WOLVES_TEST) == QuestState::Started).then_some(
                    Effect::SetRecord {
                        record: WOLVES_TEST_RECORD,
                        value: WOLF_RUN_DONE,
                    },
                ))
                .chain([Effect::PortalSound, Effect::WarpToPortal(WOLVES_DEN, 0)])
                .collect(),
            block: false,
            occupied: Some(content::text().get(keys::MONSTERS_LEFT, &[])),
        },
        // The Conference Hall's door to its Warehouse (Cosmic
        // `enterWarehouse`).
        "enterWarehouse" => through(Effect::WarpToPortal(ELLIN_WAREHOUSE, 0)),
        // Kerning Square's 7th floor: the way up is June's to give, and
        // the way down leads to the 6th floor's `fromUp` (Cosmic
        // `Depart_TopFloor`, `Depart_topOut`).
        "Depart_topFloor" => opens(JUNE),
        // Kerning Square Station's train back to Kerning City's subway
        // (Cosmic `Depart_ToKerning`).
        "Depart_ToKerning" => through(Effect::WarpToPortal(crate::rides::SQUARE_TO_KERNING.map, 0)),
        "Depart_topOut" => through(Effect::WarpToPortal(KERNING_SQUARE_6F, 1)),
        "Depart_goFoward0" => depart_on(player, "right00"),
        "Depart_goFoward1" => depart_on(player, "right01"),
        // Back down the Spirit of Rock's floors, to the floor before's
        // `left00` or `left01`.
        "Depart_goBack00" => through(to(player.stats.map_id - 10, Portal::Named("left00"))),
        "Depart_goBack01" => through(to(player.stats.map_id - 10, Portal::Named("left01"))),
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
        // The doors between Horntail's trials, sealed until the trial's
        // head is down (Cosmic's `defeatedHead`; `events::HORNTAIL_HEADS`).
        "hontale_BR" => {
            use crate::events::{HORNTAIL, HORNTAIL_TRIAL_I, HORNTAIL_TRIAL_II};
            let map_id = player.stats.map_id;
            if ![HORNTAIL_TRIAL_I, HORNTAIL_TRIAL_II].contains(&map_id) {
                return None;
            }
            let stage = HORNTAIL.stage_on(map_id)?;
            let next = HORNTAIL.stage_map(stage + 1);
            ScriptOutcome {
                effects: vec![
                    Effect::RequireStage {
                        stage,
                        refused: Some(keys::HORNTAIL_TRIAL_SEALED),
                    },
                    Effect::PortalSound,
                    Effect::WarpToPortal(next, 0),
                ],
                block: false,
                occupied: None,
            }
        }
        // The doors of the Trial Grounds' first four rooms, sealed until
        // the room's key is in its seal (`reactors::HTPQ_PASS_KEY`). The
        // last room's is the client actor's (`htpq_last_seal`).
        HTPQ_SEAL => {
            use crate::events::HTPQ;
            let stage = HTPQ
                .stage_on(player.stats.map_id)
                .filter(|&stage| stage < 5)?;
            ScriptOutcome {
                effects: vec![
                    Effect::RequireStage {
                        stage,
                        refused: Some(keys::HTPQ_SEALED),
                    },
                    Effect::PortalSound,
                    Effect::WarpToPortal(HTPQ.stage_map(stage + 1), 0),
                ],
                block: false,
                occupied: None,
            }
        }
        // The gate to Zakum: a chief's approval, the trials, an Eye of
        // Fire, and the gate reactor open, as no expedition is under way
        // (`events::ZAKUM`'s `gate`).
        "Zakum05" if !npc_scripts::zakum_approved(player) => refused(keys::ZAKUM_GATE_UNAPPROVED),
        "Zakum05" if quest(npc_scripts::ZAKUM_TRIALS_DONE) != QuestState::Started => {
            refused(keys::ZAKUM_GATE_TRIALS)
        }
        "Zakum05" if player.inventory.count(npc_scripts::EYE_OF_FIRE) == 0 => {
            refused(keys::ZAKUM_GATE_NO_EYE)
        }
        "Zakum05" => ScriptOutcome {
            effects: vec![
                Effect::ShutByReactor {
                    name: crate::events::ZAKUM_GATE.1,
                    state: 1,
                },
                Effect::PortalSound,
                Effect::WarpToPortal(
                    npc_scripts::ENTRANCE_TO_ZAKUM,
                    named(npc_scripts::ENTRANCE_TO_ZAKUM, "west00"),
                ),
            ],
            block: false,
            occupied: Some(content::text().get(keys::ZAKUM_GATE_BLOCKED, &[])),
        },
        // Into King Pepe's room while Stop the Wedding (2330) still wants a
        // Yeti brother, else on to the Wedding Hall (Cosmic
        // `TD_MC_enterboss1`).
        "TD_MC_enterboss1"
            if quest(STOP_THE_WEDDING) == QuestState::Started && !yeti_brothers_beaten(player) =>
        {
            ScriptOutcome {
                effects: vec![Effect::PortalSound, Effect::StartEvent(EventKind::KingPepe)],
                block: false,
                occupied: Some(content::text().get(keys::KING_PEPE_BUSY, &[])),
            }
        }
        "TD_MC_enterboss1" => through(Effect::WarpToPortal(
            WEDDING_HALL_ENTRANCE,
            named(WEDDING_HALL_ENTRANCE, "out00"),
        )),
        // The Wedding Hall's door (Cosmic `TD_MC_enterboss2`): straight into
        // the hall once the Prime Minister is beaten for good; into the
        // player's own copy of it from the key's 2332 on; locked before.
        "TD_MC_enterboss2" => match (quest(WHERES_VIOLETTA), quest(STORY_OF_BETRAYAL)) {
            (_, QuestState::Completed) => through(Effect::WarpToPortal(
                WEDDING_HALL,
                named(WEDDING_HALL, "left00"),
            )),
            (QuestState::NotStarted, _) => refused(keys::WEDDING_HALL_LOCKED),
            _ => ScriptOutcome {
                effects: vec![
                    Effect::PortalSound,
                    Effect::StartEvent(EventKind::WeddingHall),
                ],
                block: false,
                occupied: Some(content::text().get(keys::WEDDING_HALL_BUSY, &[])),
            },
        },
        // Out of King Pepe's room, back up the tower, with the key he drops
        // once all three brothers are beaten (Cosmic `out_pepeking`).
        "out_pepeking" => {
            let mut effects = Vec::new();
            let inventory = &player.inventory;
            if yeti_brothers_beaten(player) && inventory.count(WEDDING_HALL_KEY) == 0 {
                if !crate::items::fits_until(inventory, WEDDING_HALL_KEY, 1, None) {
                    return Some(refused(keys::KING_PEPE_KEY_NO_ROOM));
                }
                effects.extend([
                    Effect::GiveItem {
                        item_id: WEDDING_HALL_KEY,
                        quantity: 1,
                    },
                    Effect::Pink(content::text().get(keys::KING_PEPE_KEY, &[])),
                ]);
            }
            effects.extend([
                Effect::PortalSound,
                Effect::WarpToPortal(
                    EAST_CASTLE_TOWER,
                    named(EAST_CASTLE_TOWER, "TD_MC_enterboss1"),
                ),
            ]);
            ScriptOutcome {
                effects,
                block: false,
                occupied: None,
            }
        }
        // Out of the Secret Passage, back to the Toy Factory, leaving any
        // Machine Parts behind (Cosmic `ludi021`).
        "ludi021" => {
            let held = player.inventory.count(MACHINE_PART);
            let mut effects = Vec::new();
            if held > 0 {
                effects.push(Effect::TakeItem {
                    item_id: MACHINE_PART,
                    quantity: i16::try_from(held).unwrap_or(i16::MAX),
                });
            }
            effects.extend([Effect::PortalSound, Effect::Warp(APPARATUS_ROOM)]);
            ScriptOutcome {
                effects,
                block: false,
                occupied: None,
            }
        }
        // While Verifying the Password is under way, the door asks for the
        // password its start script set (the Secret Passage, Cosmic
        // `MagatiaPassword`), which once given is "1" (Cosmic `secretDoor`).
        "secretDoor"
            if quest(VERIFYING_THE_PASSWORD) == QuestState::Started
                && player
                    .quests
                    .info(VERIFYING_THE_PASSWORD)
                    .is_some_and(|password| password.len() > 1) =>
        {
            opens(crate::npc_scripts::SECRET_PASSAGE)
        }
        "secretDoor" if quest(VERIFYING_THE_PASSWORD) != QuestState::NotStarted => {
            through(through_secret_door(player))
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
        // Out of Romeo and Juliet's party quest, to the lobby of the side
        // it was (Alcadno's exit has the same script as Zenumist's).
        "rnj_exit" => {
            use crate::events::{EventKind, MagatiaSide};
            let side = MagatiaSide::ALL
                .into_iter()
                .find(|&side| EventKind::Magatia(side).event().exit == player.stats.map_id)?;
            through(Effect::WarpToPortal(
                EventKind::Magatia(side).event().lobby,
                0,
            ))
        }
        // A Magatia library's way down to its hall, once one of its books
        // held the switch, the hall's way on, once its monsters are all
        // gone, the beaker room's, once its three beakers were full, and
        // Yulete's office's, once the mobs he left are all gone (Cosmic
        // `rnj1_pt00`, `rnj1_out`, `rnj2_out`, `rnj6_out` and their Alcadno
        // twins).
        "rnj1_pt00" | "jnr1_pt00" | "rnj1_out" | "jnr1_out" | "rnj2_out" | "jnr2_out"
        | "rnj6_out" | "jnr6_out" => {
            let side = crate::events::MagatiaSide::of(player.stats.map_id)?;
            let stage = match &script[3..] {
                "1_pt00" => 1,
                "1_out" => 2,
                "6_out" => crate::events::MAGATIA_OFFICE_STAGE,
                _ => 3,
            };
            ScriptOutcome {
                effects: vec![
                    Effect::RequireStage {
                        stage,
                        refused: None,
                    },
                    Effect::PortalSound,
                    Effect::WarpToPortal(side.stages()[usize::from(stage)], 0),
                ],
                block: false,
                occupied: Some(content::text().get(keys::MAGATIA_PORTAL_SHUT, &[])),
            }
        }
        // The corridor's way into the boss's room, never shut (Cosmic
        // `rnj12_in` and `jnr12_in`).
        "rnj12_in" | "jnr12_in" => {
            let side = crate::events::MagatiaSide::of(player.stats.map_id)?;
            through(Effect::WarpToPortal(side.stages()[6], 0))
        }
        // The dark pipe off Magatia's Alcadno side, into Yulete's lab once
        // his traces have shown the way (Cosmic `magatia_dark0`, quest 7770).
        "magatia_dark0" if quest(npc_scripts::YULETE_LAB_OPENED) == QuestState::Started => {
            through(Effect::WarpToPortal(YULETE_LAB, YULETE_LAB_WAY_OUT))
        }
        "magatia_dark0" => refused(keys::MAGATIA_PIPE_DARK),
        // The hub's doors into the rooms beside it, once a Card Key opened
        // them, those rooms' ways back, once a door opened them too, and
        // the hub's way on, once Romeo or Juliet opened it for the files
        // (Cosmic `rnj3_in0`, `rnj3_in1`, `rnj_201_0`, `rnj_202`,
        // `rnj3_out` and their Alcadno twins). Each stays open only a few
        // seconds.
        "rnj3_in0" | "jnr3_in0" | "rnj3_in1" | "jnr3_in1" | "rnj_201_0" | "jnr_201_0"
        | "rnj_202" | "jnr_202" | "rnj3_out" | "jnr3_out" => {
            let side = crate::events::MagatiaSide::of(player.stats.map_id)?;
            let (door, map, portal) = magatia_hub_way(side, &script[3..])?;
            ScriptOutcome {
                effects: vec![
                    Effect::RequireReactorsSet(door),
                    Effect::PortalSound,
                    Effect::WarpToPortal(map, portal),
                ],
                block: false,
                occupied: Some(content::text().get(keys::MAGATIA_PORTAL_SHUT, &[])),
            }
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
        _ if OPQ_JAILS_IN.contains(&script) => refused(keys::OPQ_NOTICE_STORAGE_CLOSED),
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
        "enterDollcave" => opens(crate::npc_scripts::PUPPETEER_DOOR),
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
        "ariant_queens" if disguised_as_tigun(player) => ScriptOutcome::default(),
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
        // Out of the Carnival Field, back to where Spiegelmann sent the
        // player from (Cosmic `mc_out`).
        "mc_out" => {
            let back = player
                .saved_locations
                .get(&CARNIVAL_RETURN)
                .copied()
                .unwrap_or(crate::carnival::CARNIVAL_FALLBACK);
            go_back(CARNIVAL_RETURN, Effect::Warp(back))
        }
        // A Monster Carnival room's revive room, back into its battle
        // (Cosmic `MCrevive1` to `MCrevive6`).
        script if script.starts_with("MCrevive") => {
            let room = script["MCrevive".len()..].parse::<u8>().ok()?;
            let battle = crate::carnival::battle_map(room);
            (crate::carnival::battle_room(battle) == Some(room))
                .then(|| through(Effect::WarpToPortal(battle, 0)))?
        }
        // Out of the Mu Lung Dojo, back to where its notice board was
        // taken from (Cosmic `dojang_exit`).
        "dojang_exit" => {
            let back = player
                .saved_locations
                .get(&DOJO_RETURN)
                .copied()
                .unwrap_or(crate::events::DOJO_BOARD_MAP);
            go_back(DOJO_RETURN, Effect::Warp(back))
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
        // The Pirate Ship's way on from its first stage, once the stage's
        // monsters are all gone (Cosmic `davy_next0`).
        "davy_next0" => ScriptOutcome {
            effects: vec![
                Effect::RequireNoMobs,
                Effect::PortalSound,
                Effect::WarpToPortal(crate::events::PIRATE_STAGES[1], 0),
            ],
            block: false,
            occupied: Some(content::text().get(keys::PIRATE_PORTAL_SHUT, &[])),
        },
        // Once Guon has every Mark, which clears the stage.
        "davy_next1" => ScriptOutcome {
            effects: vec![
                Effect::RequireStage {
                    stage: 2,
                    refused: None,
                },
                Effect::PortalSound,
                Effect::WarpToPortal(crate::events::PIRATE_STAGES[2], 0),
            ],
            block: false,
            occupied: Some(content::text().get(keys::PIRATE_MARKS_SHUT, &[])),
        },
        // The Guild Quest's way on from the hall of statues, open once the
        // statue keeper's test is passed (Cosmic `statuegate_open`).
        "statuegate_open" => ScriptOutcome {
            effects: vec![
                Effect::RequireReactorsSet(&[crate::guild_quest::STATUE_GATE]),
                Effect::PortalSound,
                Effect::WarpToPortal(crate::guild_quest::STATUE_HALL + 1, 0),
            ],
            block: false,
            occupied: Some(content::text().get(keys::GUILD_QUEST_GATE_CLOSED, &[])),
        },
        // The knights' hall's way on, open once the spears are all set,
        // which clears the stage (Cosmic `speargate_open`).
        "speargate_open" => ScriptOutcome {
            effects: vec![
                Effect::RequireStage {
                    stage: crate::guild_quest::KNIGHT_STAGE,
                    refused: None,
                },
                Effect::PortalSound,
                Effect::WarpToPortal(crate::guild_quest::KNIGHT_HALL + 1, 0),
            ],
            block: false,
            occupied: Some(content::text().get(keys::GUILD_QUEST_GATE_CLOSED, &[])),
        },
        // Its door to the metal room, open once a stone is set in it, and
        // the metal room's to the room past it, once a piece of metal is
        // (Cosmic `stonegate_open`, `metalgate_open`).
        "stonegate_open" => ScriptOutcome {
            effects: vec![
                Effect::RequireReactorsSet(&[crate::guild_quest::STONE_GATE]),
                Effect::PortalSound,
                Effect::WarpToPortal(crate::guild_quest::KNIGHT_HALL + 30, 0),
            ],
            block: false,
            occupied: Some(content::text().get(keys::GUILD_QUEST_GATE_BLOCKED, &[])),
        },
        "metalgate_open" => ScriptOutcome {
            effects: vec![
                Effect::RequireReactorsSet(&[crate::guild_quest::METAL_GATE]),
                Effect::PortalSound,
                Effect::WarpToPortal(crate::guild_quest::KNIGHT_HALL + 31, 0),
            ],
            block: false,
            occupied: Some(content::text().get(keys::GUILD_QUEST_GATE_CLOSED, &[])),
        },
        // The decks' ways on, once their crews are all gone, as the first
        // stage's (Cosmic `davy_next2`, `davy_next3`).
        "davy_next2" | "davy_next3" => ScriptOutcome {
            effects: vec![
                Effect::RequireNoMobs,
                Effect::PortalSound,
                Effect::WarpToPortal(
                    crate::events::PIRATE_STAGES[if script == "davy_next2" { 3 } else { 4 }],
                    0,
                ),
            ],
            block: false,
            occupied: Some(content::text().get(keys::PIRATE_PORTAL_SHUT, &[])),
        },
        // The way past the doors, once all four are sealed and no monster
        // is left; the first member through finds the Lord Pirate (Cosmic
        // `davy_next4`).
        "davy_next4" => ScriptOutcome {
            effects: vec![
                Effect::RequireReactorsSet(&crate::reactors::PIRATE_SEAL_NAMES),
                Effect::RequireNoMobs,
                Effect::PortalSound,
                Effect::WarpToPortal(crate::events::PIRATE_STAGES[5], 0),
                Effect::EventCommand(crate::events::EventCommand::PastTheDoors),
            ],
            block: false,
            occupied: Some(content::text().get(keys::PIRATE_PORTAL_SHUT, &[])),
        },
        // Each deck's hidden way down to its crew's room, open whatever is
        // left aboard (Cosmic `davy2_hd1`, `davy3_hd1`).
        "davy2_hd1" | "davy3_hd1" => ScriptOutcome {
            effects: vec![
                Effect::PortalSound,
                Effect::WarpToPortal(
                    crate::events::PIRATE_STAGES[if script == "davy2_hd1" { 2 } else { 3 }] + 2,
                    0,
                ),
            ],
            block: false,
            occupied: None,
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

/// Whether `player`, on Stop the Wedding, has beaten each Yeti brother.
fn yeti_brothers_beaten(player: &Player) -> bool {
    let kills = player.quests.kills(STOP_THE_WEDDING);
    kills.len() == YETI_BROTHERS.len() && kills.iter().all(|&count| count > 0)
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

/// `investigate1`, `investigate2`: on `quest` and not yet having looked,
/// the player looks, which sets its record to 1; otherwise nothing.
fn investigate(player: &Player, quest: i16, balloon: u8) -> ScriptOutcome {
    let looked = player.quests.kills(quest).first() == Some(&1);
    let effects = if player.quests.state(quest) == QuestState::Started && !looked {
        let key = match quest {
            EXPLORING_MUSHROOM_FOREST => keys::MC_BARRIER_INVESTIGATED,
            _ => keys::MC_WALLS_INVESTIGATED,
        };
        vec![
            Effect::QuestRecord {
                quest_id: quest,
                count: 1,
            },
            Effect::Think(content::text().get(key, &[])),
            Effect::AvatarEffect(format!(
                "Effect/OnUserEff.img/normalEffect/mushroomcastle/chatBalloon{balloon}"
            )),
        ]
    } else {
        Vec::new()
    };
    ScriptOutcome {
        effects,
        block: false,
        occupied: None,
    }
}

/// Into Magatia's secret basement, on the side of the lab the player is
/// leaving (`secretDoor`).
pub fn through_secret_door(player: &Player) -> Effect {
    let side = if player.stats.map_id == ZENUMIST_HALLWAY {
        "sp_jenu"
    } else {
        "sp_alca"
    };
    Effect::WarpToPortal(SECRET_BASEMENT_PATH, named(SECRET_BASEMENT_PATH, side))
}

/// `Depart_goFoward0`, `Depart_goFoward1`: on up the Spirit of Rock's
/// floors to the next one's `side`: past the Cheap Amplifiers' floor from
/// Composition Fee (2287) on, past the Fancy Amplifiers' from Say "NO" to
/// Plagiarism (2288) on, and freely through the VIP Zone's; otherwise the
/// way stays shut, in pink (Cosmic `Depart_goFoward0`).
fn depart_on(player: &Player, side: &'static str) -> ScriptOutcome {
    let map = player.stats.map_id;
    let begun = |quest: i16| player.quests.state(quest) != QuestState::NotStarted;
    let open = match map {
        ROCK_SPIRIT_FLOOR => begun(COMPOSITION_FEE),
        FANCY_AMPLIFIER_FLOOR => begun(SAY_NO_TO_PLAGIARISM),
        ROCK_SPIRIT_VIP_FLOOR | VIP_FANCY_AMPLIFIER_FLOOR => true,
        _ => false,
    };
    if !open {
        return refused(keys::DEPART_NO_ACCESS);
    }
    through(to(map + 10, Portal::Named(side)))
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

/// The way into the Trial Grounds' rooms (Cosmic `hontale_BtoB1`): someone
/// must stay on the first map to gather the keys the rooms send there, so
/// it lets no one through who is `alone` on it, nor anyone holding the
/// first Crystal Key, which the leader hands in there; they are told in
/// pink.
pub fn htpq_into_rooms(alone: bool, holding_key: bool) -> Vec<Effect> {
    let pink = |key| vec![Effect::Pink(content::text().get(key, &[]))];
    if alone {
        pink(keys::HTPQ_ROOMS_ALONE)
    } else if holding_key {
        pink(keys::HTPQ_ROOMS_HOLDING_KEY)
    } else {
        let first_room = crate::events::HTPQ.stage_map(1);
        vec![Effect::PortalSound, Effect::WarpToPortal(first_room, 0)]
    }
}

/// The door out of the Trial Grounds' last room, back to the first map's
/// `st00`: open once its seal is `lifted` (stage 5); until then only the
/// leader holding the Blue Crystal Key goes through, which lifts it (Cosmic
/// `hontale_Bopen`).
pub fn htpq_last_seal(lifted: bool, leader_with_key: bool) -> Vec<Effect> {
    use crate::events::{EventCommand, HTPQ_ENTRY};
    let pink = |key| Effect::Pink(content::text().get(key, &[]));
    let out = [
        Effect::PortalSound,
        Effect::WarpToPortal(HTPQ_ENTRY, named(HTPQ_ENTRY, "st00")),
    ];
    if lifted {
        out.into()
    } else if leader_with_key {
        let lift = [
            Effect::EventCommand(EventCommand::ClearStage(5)),
            pink(keys::HTPQ_SEAL_LIFTED),
        ];
        lift.into_iter().chain(out).collect()
    } else {
        vec![pink(keys::HTPQ_SEAL_LEADER_ONLY)]
    }
}

/// The lamp's room's way on, for the leader or not, with the lamp in
/// `lamp`: the leader takes the whole team to the light map or the dark
/// one, as the lamp is lit or dark; at any other state, or for anyone
/// else, it says why not (Cosmic `hontale_C`).
pub fn htpq_lamp_door(leader: bool, lamp: Option<u8>) -> Vec<Effect> {
    use crate::events::{EventCommand, HTPQ_DARK, HTPQ_LIGHT};
    let pink = |key| vec![Effect::Pink(content::text().get(key, &[]))];
    let to = match lamp {
        _ if !leader => return pink(keys::HTPQ_LAMP_LEADER_ONLY),
        Some(1) => HTPQ_LIGHT,
        Some(3) => HTPQ_DARK,
        _ => return pink(keys::HTPQ_LAMP_HIT),
    };
    vec![
        Effect::PortalSound,
        Effect::EventCommand(EventCommand::MoveTeam(to)),
    ]
}

/// The Origin of the Clocktower in the past, where Papulatus (2043000)
/// stands for a Buccaneer on the Time Leap quests (6361-6363).
pub const PAST_CLOCKTOWER: i32 = 922020300;
/// Papulatus's Key (4031870), which 6361 takes to the past.
const PAPULATUS_KEY: i32 = 4031870;
/// "A New Revelation" (6363) and the record it completes on.
pub const NEW_REVELATION: i16 = 6363;
pub const SENT_BACK_RECORD: i16 = 6364;
pub const SENT_BACK: &str = "2";

/// The way to Papulatus as it leads into the past instead, for a player
/// carrying the key on 6361 or on the quests after it (Cosmic
/// `Populatus00`); `None` leads to the fight.
pub fn time_leap(player: &Player) -> Option<Vec<Effect>> {
    let quest = |id| player.quests.state(id);
    let into_past = (quest(6361) == QuestState::Started
        && player.inventory.count(PAPULATUS_KEY) > 0)
        || (quest(6361) == QuestState::Completed && quest(NEW_REVELATION) != QuestState::Completed);
    into_past.then(|| {
        vec![
            Effect::PortalSound,
            Effect::WarpToPortal(PAST_CLOCKTOWER, 0),
        ]
    })
}

/// The way out of Save Delli!, which the client actor answers from the
/// player's ticket (`delli_way_out`).
pub const DELLI_OUT: &str = "s4mind_end";
pub const DELLI_QUEST: i16 = 6410;
/// The record "Mission Free the Monsters" (6410) completes on, and what
/// the way out past a won rescue sets it to.
pub const DELLI_RECORD: i16 = 6411;
pub const DELLI_SAVED: &str = "p2";

/// The way out of Save Delli!: shut, with a pink line, until the party has
/// won; then it leads on to the Nautilus Hideout Room, marking the record
/// of a Corsair on 6410 (Cosmic `s4mind_end`).
pub fn delli_way_out(won: bool, player: &Player) -> Vec<Effect> {
    if !won {
        return vec![Effect::Pink(content::text().get(keys::DELLI_NOT_WON, &[]))];
    }
    let on_quest = player.quests.state(DELLI_QUEST) == QuestState::Started;
    let record = on_quest.then_some(Effect::SetRecord {
        record: DELLI_RECORD,
        value: DELLI_SAVED,
    });
    record
        .into_iter()
        .chain([
            Effect::PortalSound,
            Effect::WarpToPortal(crate::events::NAUTILUS_HIDEOUT, 0),
        ])
        .collect()
}

/// The way out of the room Tylus is protected in (our Map.nx's
/// `s4common1_clear`; Cosmic's `s4common1_exit`).
pub const TYLUS_OUT: &str = "s4common1_clear";

/// The way out of Tylus's room: on to his thanks for one holding his Trust,
/// and out of El Nath's valley for anyone else, which leaves the event
/// (Cosmic `s4common1_exit`).
pub fn tylus_way_out(player: &Player) -> Vec<Effect> {
    use crate::events::{TYLUS_EXIT, TYLUS_THANKS, TYLUS_TRUST};
    let to = if player.inventory.count(TYLUS_TRUST) > 0 {
        Effect::WarpToPortal(TYLUS_THANKS, 0)
    } else {
        Effect::Warp(TYLUS_EXIT)
    };
    vec![Effect::PortalSound, to]
}

/// The way on from the Forest of Poison Haze's entrance and its first two
/// stages (Cosmic `party6_stage`).
pub const ELLIN_STAGE: &str = "party6_stage";

/// Where the forest's way on takes a member on `map_id`: from the entrance
/// into the first stage, and from a stage the party has `cleared` into
/// the next; a stage not yet cleared keeps it shut, with a pink line why.
pub fn ellin_way_on(map_id: i32, cleared: impl Fn(u8) -> bool) -> Vec<Effect> {
    use crate::events::ELLIN;
    let on = |stage| {
        vec![
            Effect::PortalSound,
            Effect::WarpToPortal(ELLIN.stage_map(stage), 0),
        ]
    };
    if map_id == ELLIN.entry {
        return on(1);
    }
    let (stage, shut) = match ELLIN.stage_on(map_id) {
        Some(1) => (1, keys::ELLIN_STAGE1_SHUT),
        Some(2) => (2, keys::ELLIN_STAGE2_SHUT),
        _ => return Vec::new(),
    };
    if cleared(stage) {
        on(stage + 1)
    } else {
        vec![Effect::Pink(content::text().get(shut, &[]))]
    }
}

/// The ways out of the forest's maze, its third stage (Cosmic
/// `party6_stage501` to `party6_stage515`): each of its first fifteen
/// rooms has one script for all its ways out.
const ELLIN_MAZE_ROOM: &str = "party6_stage5";

/// The maze's room `16st`, at its end, where Ellin is.
pub const ELLIN_MAZE_END: u8 = 16;

/// Where a way out of a maze room takes a member when `lucky` (one time
/// in ten): to the maze's end, and otherwise to the next room's spot (the
/// fifteenth's back to the first's). `None` for any other portal.
pub fn ellin_maze_way(script: &str, lucky: bool) -> Option<Vec<Effect>> {
    let room = script.strip_prefix(ELLIN_MAZE_ROOM)?;
    let room: u8 = room.parse().ok().filter(|_| room.len() == 2)?;
    if !(1..ELLIN_MAZE_END).contains(&room) {
        return None;
    }
    let to = if lucky {
        ELLIN_MAZE_END
    } else {
        room % (ELLIN_MAZE_END - 1) + 1
    };
    let maze = crate::events::ELLIN_STAGES[2];
    Some(vec![Effect::PortalSound, Effect::WarpToPortal(maze, to)])
}

/// The way out of the forest's last stage (Cosmic `party6_out`).
pub const ELLIN_OUT: &str = "party6_out";
/// The prize map's way home (Cosmic `party6_stage800`).
pub const ELLIN_HOME: &str = "party6_stage800";

/// What the prize map's way home does for `player`: it takes all the
/// forest's items and its Golem's Marbles, and while the `fountain` has
/// taken a marble (its state above 0) gives an Altaire Fragment, or with
/// no room for it, after those are gone, stays shut with a pink line; then
/// home to the Deep Fairy Forest (Cosmic `party6_stage800`).
pub fn ellin_home(player: &Player, fountain: bool) -> Vec<Effect> {
    use crate::events::{ALTAIRE_FRAGMENT, ELLIN_ITEMS, ELLIN_LOBBY, GOLEMS_MARBLE};
    let mut left = player.inventory.clone();
    let mut effects = Vec::new();
    for item_id in ELLIN_ITEMS.into_iter().chain([GOLEMS_MARBLE]) {
        let quantity = i16::try_from(player.inventory.count(item_id)).unwrap_or(i16::MAX);
        if quantity > 0 && left.remove(item_id, quantity).is_ok() {
            effects.push(Effect::TakeItem { item_id, quantity });
        }
    }
    if fountain {
        if !crate::items::fits_until(&left, ALTAIRE_FRAGMENT, 1, None) {
            let shut = content::text().get(keys::ELLIN_HOME_NO_ROOM, &[]);
            return vec![Effect::Pink(shut)];
        }
        effects.push(Effect::GiveItem {
            item_id: ALTAIRE_FRAGMENT,
            quantity: 1,
        });
    }
    effects.extend([Effect::PortalSound, Effect::WarpToPortal(ELLIN_LOBBY, 0)]);
    effects
}

/// What the way out of the Poison Golem's stage does for a member: shut,
/// with a pink line why, until the golem is dead and the forest `won`;
/// then the leader takes the whole team out to the prize map, and anyone
/// else is told to wait for them.
pub fn ellin_way_out(won: bool, leader: bool) -> Vec<Effect> {
    use crate::events::{EventCommand, ELLIN};
    let pink = |key| vec![Effect::Pink(content::text().get(key, &[]))];
    match (won, leader) {
        (false, _) => pink(keys::ELLIN_STAGE6_SHUT),
        (true, false) => pink(keys::ELLIN_STAGE6_WAIT),
        (true, true) => vec![
            Effect::PortalSound,
            Effect::EventCommand(EventCommand::MoveTeam(ELLIN.exit)),
        ],
    }
}

/// The Dojo's way up to a floor's upper tier, by its door (Cosmic
/// `dojang_up`).
pub const DOJO_UP: &str = "dojang_up";
/// The Dojo's way on, behind a floor's door (Cosmic `dojang_next`).
pub const DOJO_NEXT: &str = "dojang_next";

/// The way out of So Gong's trial, up its ladder (Cosmic `dojang_tuto`).
pub const DOJO_TUTO: &str = "dojang_tuto";

/// What the way out of So Gong's trial does: once So Gong is `beaten`, it
/// marks the trial done and takes the challenger back to the hall;
/// before, a pink line says he won't let them go.
pub fn dojo_tuto(beaten: bool) -> Vec<Effect> {
    if !beaten {
        return vec![Effect::Pink(
            content::text().get(keys::DOJO_TRIAL_STAY, &[]),
        )];
    }
    vec![
        Effect::DojoTrialDone,
        Effect::PortalSound,
        Effect::WarpToPortal(crate::events::DOJO_HALL, 0),
    ]
}

/// What the way on from the Dojo's `floor`, once cleared, does for a
/// challenger with `record`: the floor's training points and a pink line
/// of them, then the next floor; from the last, the rooftop and 2,000 EXP
/// for each point held (Cosmic `dojang_next`). Going on from the boss of
/// the medal tried for counts one more win toward it (Cosmic `dojang_up`,
/// whose door counts it).
///
/// With a `party` there is no point for going alone and no medal win, and
/// the way on to a rest floor takes everyone on the five floors before it
/// there (`EventCommand::DojoRest`), each with their points then.
pub fn dojo_next(floor: u8, record: &game::DojoRecord, party: bool) -> Vec<Effect> {
    use crate::events::{
        dojo_round, rest_floor, EventCommand, DOJO, DOJO_FLOORS, DOJO_PARTY, DOJO_ROOFTOP,
    };
    let win = (!party && i32::from(dojo_round(floor)) == record.vanquisher_stage).then(|| {
        Effect::DojoMedal {
            stage: record.vanquisher_stage,
            kills: record.vanquisher_kills + 1,
        }
    });
    if floor >= DOJO_FLOORS {
        return win
            .into_iter()
            .chain([
                Effect::PortalSound,
                Effect::WarpToPortal(DOJO_ROOFTOP, 0),
                Effect::GainExp(record.points.saturating_mul(2000)),
            ])
            .collect();
    }
    if party && rest_floor(floor + 1) {
        return vec![Effect::EventCommand(EventCommand::DojoRest(floor + 1))];
    }
    let event = if party { &DOJO_PARTY } else { &DOJO };
    win.into_iter()
        .chain(dojo_points(floor..=floor, record, !party))
        .chain([
            Effect::PortalSound,
            Effect::WarpToPortal(event.stage_map(floor + 1), 0),
        ])
        .collect()
}

/// The training points for leaving the Dojo's `floors`, one after
/// another, for a challenger with `record`, each with a pink line of them
/// and the new total (Cosmic `dojang_next`'s messages).
pub fn dojo_points(
    floors: std::ops::RangeInclusive<u8>,
    record: &game::DojoRecord,
    alone: bool,
) -> Vec<Effect> {
    let mut held = *record;
    floors
        .flat_map(|floor| {
            let points = held.floor_points(floor, alone);
            held.points += points;
            let (earned, total) = (points.to_string(), held.points.to_string());
            let line = content::text().get(
                keys::DOJO_POINTS,
                &[("points", earned.as_str()), ("total", total.as_str())],
            );
            [Effect::DojoPoints(points), Effect::Pink(line)]
        })
        .collect()
}

/// The way to Papulatus, for a player whose party stands as `entry`: the
/// leader takes it in, or is told in pink why not (Cosmic `Populatus00`).
pub fn papulatus_portal(entry: crate::events::Entry) -> Vec<Effect> {
    use crate::events::{Entry, EventKind, PAPULATUS};
    let pink = |key| vec![Effect::Pink(content::text().get(key, &[]))];
    match entry {
        Entry::NoParty => pink(keys::PAPULATUS_NO_PARTY),
        Entry::NotLeader => pink(keys::PAPULATUS_NOT_LEADER),
        Entry::NotEligible => {
            let (least, most) = (
                PAPULATUS.least().to_string(),
                PAPULATUS.team.end().to_string(),
            );
            let vars = [("least", least.as_str()), ("most", most.as_str())];
            vec![Effect::Pink(
                content::text().get(keys::PAPULATUS_NOT_ELIGIBLE, &vars),
            )]
        }
        Entry::Busy => pink(keys::PAPULATUS_BUSY),
        Entry::Ready => vec![
            Effect::PortalSound,
            Effect::StartEvent(EventKind::Papulatus),
        ],
    }
}

/// The portals along the top of each level of Ola Ola's variant 1.
pub const RAND_OLA: &str = "rand_ola";

/// Where `portal`, one of the `rand_ola` portals on `map_id`, takes a
/// player: one of a level's portals leads on to the next level's start, or
/// out to [`gm_event::WON_MAP`] from the last, and the rest back to this
/// level's start. Which one leads on is the same for a player all session,
/// `seed` being theirs, and differs from level to level (QUESTIONS b172
/// #5). `None` off Ola Ola's maps.
pub fn rand_ola(map_id: i32, portal: &str, seed: u64) -> Option<Vec<Effect>> {
    use std::hash::{Hash, Hasher};
    let maps = crate::gm_event::timed_event(map_id)?.maps;
    let level = maps.iter().position(|map| *map == map_id)?;
    let field = crate::game_data::get().ok()?.field(map_id)?;
    let ways: Vec<&str> = field
        .portals()
        .filter(|portal| portal.script.as_deref() == Some(RAND_OLA))
        .map(|portal| portal.name.as_str())
        .collect();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (seed, map_id).hash(&mut hasher);
    let on = *ways.get(usize::try_from(hasher.finish() % ways.len().max(1) as u64).ok()?)?;
    let to = match maps.get(level + 1) {
        _ if portal != on => map_id,
        Some(next) => *next,
        None => crate::gm_event::WON_MAP,
    };
    Some(vec![
        Effect::PortalSound,
        Effect::WarpToPortal(to, named(to, "start00")),
    ])
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
                QuestState::Started => QuestStatus::Started {
                    kills: Vec::new(),
                    info: None,
                },
                _ => QuestStatus::Completed {
                    at: SystemTime::UNIX_EPOCH,
                },
            };
            player.quests.upsert(QuestRecord { quest_id, status });
        }
        player
    }

    #[test]
    fn kyrins_rooms_let_out_after_a_minute_and_pass_after_two() {
        let wait = crate::content::text().get(keys::KYRIN_TEST_WAIT, &[]);
        let at = Duration::from_secs;
        assert_eq!(kyrin_test_out(at(59)), [Effect::Pink(wait)]);
        assert_eq!(
            kyrin_test_out(at(60)),
            [Effect::PortalSound, Effect::WarpToPortal(KYRIN_MAP, 0)]
        );
        assert_eq!(
            kyrin_test_out(at(120)),
            [
                Effect::PortalSound,
                Effect::WarpToPortal(KYRIN_PASSED_ROOM, 0)
            ]
        );
        assert!(KYRIN_TESTS
            .iter()
            .all(|test| SOLO_MAPS.contains(&test.room)));
        let Ok(maps) = crate::game_data::get() else {
            return;
        };
        for (test, out) in KYRIN_TESTS.iter().zip(["s4super_out", "s4ship_out"]) {
            let room = maps.field(test.room).unwrap();
            assert!(room.portals().any(|p| p.script.as_deref() == Some(out)));
            assert_eq!(room.forced_return, Some(KYRIN_MAP));
        }
    }

    #[test]
    fn the_way_to_papulatus_lets_a_ready_party_in_and_says_why_not_otherwise() {
        use crate::events::{Entry, EventKind};
        let text = |key, vars: &[(&str, &str)]| content::text().get(key, vars);
        let refused = [
            (Entry::NoParty, text(keys::PAPULATUS_NO_PARTY, &[])),
            (Entry::NotLeader, text(keys::PAPULATUS_NOT_LEADER, &[])),
            (
                Entry::NotEligible,
                text(
                    keys::PAPULATUS_NOT_ELIGIBLE,
                    &[("least", "1"), ("most", "6")],
                ),
            ),
            (Entry::Busy, text(keys::PAPULATUS_BUSY, &[])),
        ];
        for (entry, line) in refused {
            assert_eq!(papulatus_portal(entry), [Effect::Pink(line)]);
        }
        assert_eq!(
            papulatus_portal(Entry::Ready),
            [
                Effect::PortalSound,
                Effect::StartEvent(EventKind::Papulatus)
            ]
        );
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
        let first = |player: &Player| portal("TD_MC_first", player).unwrap();
        let mut player = player_with(&[(2300, QuestState::Started)]);
        player.stats.level = 29;
        assert!(pink(&first(&player)));
        // The first time through the opening scene, then to the forest.
        player.stats.level = 30;
        assert_eq!(first(&player).effects, through(Effect::Warp(106020001)));
        player.quests.set_record(2311, "1");
        assert_eq!(first(&player).effects, through(Effect::Warp(106020000)));

        // The scene marks itself seen, then plays and leads to the forest.
        let opening = map_enter("TD_MC_Openning", &player_with(&[])).unwrap();
        assert_eq!(
            opening[0],
            Effect::SetRecord {
                record: 2311,
                value: "1"
            }
        );
        let Effect::Scene { parts, exit } = &opening[1] else {
            panic!("{opening:?}");
        };
        assert_eq!(parts.len(), 10);
        assert_eq!(parts[0], "Effect/Direction2.img/open/back0");
        assert_eq!(parts[9], "Effect/Direction2.img/open/out");
        assert_eq!(*exit, 106020000);

        let castle = |quests| portal("gotocastle", &player_with(quests)).unwrap();
        assert!(pink(&castle(&[(2324, QuestState::Started)])));
        assert_eq!(
            castle(&[(2324, QuestState::Completed)]).effects,
            through(Effect::Warp(106020501))
        );
        assert_eq!(
            castle(&[(2322, QuestState::Started)]).effects,
            through(Effect::Warp(106020500))
        );
        assert!(pink(&castle(&[(2322, QuestState::Completed)])));
        assert_eq!(
            portal("TD_MC_jump", &player_with(&[])).unwrap().effects,
            through(Effect::Warp(106020501))
        );
    }

    #[test]
    fn looking_at_the_barrier_and_the_wall_sets_their_quests_records() {
        for (script, quest, key, balloon) in [
            ("investigate1", 2314, keys::MC_BARRIER_INVESTIGATED, 1),
            ("investigate2", 2322, keys::MC_WALLS_INVESTIGATED, 2),
        ] {
            let mut player = player_with(&[(quest, QuestState::Started)]);
            let outcome = portal(script, &player).unwrap();
            assert_eq!(
                outcome.effects,
                [
                    Effect::QuestRecord {
                        quest_id: quest,
                        count: 1
                    },
                    Effect::Think(content::text().get(key, &[])),
                    Effect::AvatarEffect(format!(
                        "Effect/OnUserEff.img/normalEffect/mushroomcastle/chatBalloon{balloon}"
                    )),
                ]
            );
            assert!(!outcome.block);
            // Once looked at, or off the quest, nothing more.
            *player.quests.kills_mut(quest).unwrap() = vec![1];
            assert_eq!(portal(script, &player).unwrap().effects, []);
            for quests in [&[][..], &[(quest, QuestState::Completed)]] {
                assert_eq!(portal(script, &player_with(quests)).unwrap().effects, []);
            }
        }
    }

    /// A player on Stop the Wedding with `kills` of each Yeti brother.
    fn stopping_the_wedding(kills: [u16; 3]) -> Player {
        let mut player = player_with(&[(STOP_THE_WEDDING, QuestState::Started)]);
        *player.quests.kills_mut(STOP_THE_WEDDING).unwrap() = kills.to_vec();
        player
    }

    #[test]
    fn king_pepes_room_is_open_until_the_three_yeti_brothers_are_beaten() {
        let to_hall = vec![
            Effect::PortalSound,
            Effect::WarpToPortal(WEDDING_HALL_ENTRANCE, named(WEDDING_HALL_ENTRANCE, "out00")),
        ];
        let tower = |player: &Player| portal("TD_MC_enterboss1", player).unwrap();
        for kills in [[0, 0, 0], [1, 0, 1], [1, 1, 0]] {
            let way = tower(&stopping_the_wedding(kills));
            assert_eq!(
                way.effects,
                [Effect::PortalSound, Effect::StartEvent(EventKind::KingPepe)]
            );
            assert_eq!(
                way.occupied,
                Some(content::text().get(keys::KING_PEPE_BUSY, &[]))
            );
        }
        assert_eq!(tower(&stopping_the_wedding([1, 1, 1])).effects, to_hall);
        for state in [QuestState::NotStarted, QuestState::Completed] {
            assert_eq!(
                tower(&player_with(&[(STOP_THE_WEDDING, state)])).effects,
                to_hall
            );
        }

        // Out, with the key once all three are beaten.
        let up = [
            Effect::PortalSound,
            Effect::WarpToPortal(
                EAST_CASTLE_TOWER,
                named(EAST_CASTLE_TOWER, "TD_MC_enterboss1"),
            ),
        ];
        let out = |player: &Player| portal("out_pepeking", player).unwrap().effects;
        assert_eq!(out(&stopping_the_wedding([1, 0, 1])), up);
        let mut player = stopping_the_wedding([1, 1, 2]);
        let key = Effect::GiveItem {
            item_id: WEDDING_HALL_KEY,
            quantity: 1,
        };
        let found = Effect::Pink(content::text().get(keys::KING_PEPE_KEY, &[]));
        assert_eq!(out(&player), [&[key, found][..], &up].concat());
        player.inventory.insert(
            InventoryType::Etc,
            1,
            game::Item::stack(WEDDING_HALL_KEY, 1),
        );
        assert_eq!(out(&player), up, "only one key");
        let mut full = stopping_the_wedding([1, 1, 1]);
        full.inventory.set_slot_limit(InventoryType::Etc, 1);
        full.inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(4_000_000, 1));
        assert_eq!(
            out(&full),
            [Effect::Pink(
                content::text().get(keys::KING_PEPE_KEY_NO_ROOM, &[])
            )]
        );
    }

    #[test]
    fn the_secret_passage_leads_back_to_the_factory_without_the_parts() {
        let back = [Effect::PortalSound, Effect::Warp(APPARATUS_ROOM)];
        let mut player = player_with(&[]);
        assert_eq!(portal("ludi021", &player).unwrap().effects, back);
        player
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(MACHINE_PART, 7));
        let taken = Effect::TakeItem {
            item_id: MACHINE_PART,
            quantity: 7,
        };
        assert_eq!(
            portal("ludi021", &player).unwrap().effects,
            [&[taken][..], &back].concat()
        );
    }

    #[test]
    fn the_wedding_hall_opens_alone_from_the_keys_quest_on() {
        let door = |quests: &[(i16, QuestState)]| {
            portal("TD_MC_enterboss2", &player_with(quests)).unwrap()
        };
        let locked = content::text().get(keys::WEDDING_HALL_LOCKED, &[]);
        assert_eq!(door(&[]).effects, [Effect::Pink(locked)]);
        let alone = [
            Effect::PortalSound,
            Effect::StartEvent(EventKind::WeddingHall),
        ];
        let started = QuestState::Started;
        let done = QuestState::Completed;
        for quests in [
            &[(WHERES_VIOLETTA, started)][..],
            &[(WHERES_VIOLETTA, done)],
            &[(WHERES_VIOLETTA, done), (STORY_OF_BETRAYAL, started)],
        ] {
            let way = door(quests);
            assert_eq!(way.effects, alone);
            assert_eq!(
                way.occupied,
                Some(content::text().get(keys::WEDDING_HALL_BUSY, &[]))
            );
        }
        assert_eq!(
            door(&[(WHERES_VIOLETTA, done), (STORY_OF_BETRAYAL, done)]).effects,
            [
                Effect::PortalSound,
                Effect::WarpToPortal(WEDDING_HALL, named(WEDDING_HALL, "left00")),
            ]
        );
    }

    #[test]
    fn violetta_is_found_and_the_prime_minister_comes_out_in_the_hall() {
        let enter = |player: &Player| map_enter("findvioleta", player).unwrap();
        let found = Effect::QuestRecord {
            quest_id: WHERES_VIOLETTA,
            count: 1,
        };
        let prime_minister = Effect::BringOutMob {
            mob_id: PRIME_MINISTER,
            at: PRIME_MINISTER_AT,
        };
        assert!(enter(&player_with(&[])).is_empty());
        let looking = player_with(&[(WHERES_VIOLETTA, QuestState::Started)]);
        assert_eq!(enter(&looking), [found]);
        let mut betrayed = player_with(&[
            (WHERES_VIOLETTA, QuestState::Completed),
            (STORY_OF_BETRAYAL, QuestState::Started),
        ]);
        assert_eq!(enter(&betrayed), [prime_minister.clone()]);
        *betrayed.quests.kills_mut(STORY_OF_BETRAYAL).unwrap() = vec![0];
        assert_eq!(enter(&betrayed), [prime_minister]);
        *betrayed.quests.kills_mut(STORY_OF_BETRAYAL).unwrap() = vec![1];
        assert!(enter(&betrayed).is_empty(), "beaten already");
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
    fn corbas_flight_takes_the_dragon_off_and_puts_it_back_on() {
        let player = player_with(&[]);
        let effects = |script| portal(script, &player).unwrap().effects;
        assert_eq!(
            effects("templeenter"),
            [
                Effect::CancelItemBuff(MINI_DRACO),
                Effect::PortalSound,
                Effect::WarpToPortal(TEMPLE_ROAD, 2)
            ]
        );
        assert_eq!(
            effects("outTemple"),
            [
                Effect::ApplyItemBuff(MINI_DRACO),
                Effect::PortalSound,
                Effect::WarpToPortal(FLIGHT_TO_MINAR, 0)
            ]
        );
        assert_eq!(
            effects("undodraco"),
            [
                Effect::CancelItemBuff(MINI_DRACO),
                Effect::PortalSound,
                Effect::WarpToPortal(LEAFRE_STATION, 2)
            ]
        );
        assert_eq!(
            map_enter("undomorphdarco", &player),
            Some(vec![Effect::CancelItemBuff(MINI_DRACO)])
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
    fn so_gongs_trial_lets_the_challenger_out_only_once_he_is_beaten() {
        let stay = content::text().get(keys::DOJO_TRIAL_STAY, &[]);
        assert_eq!(dojo_tuto(false), [Effect::Pink(stay)]);
        assert_eq!(
            dojo_tuto(true),
            [
                Effect::DojoTrialDone,
                Effect::PortalSound,
                Effect::WarpToPortal(crate::events::DOJO_HALL, 0),
            ]
        );
        let data = crate::get_game_data().unwrap();
        let room = data.field(crate::events::DOJO_TRIAL_MAP).unwrap();
        let out = room.portal_by_name("out000").unwrap();
        assert_eq!(out.script.as_deref(), Some(DOJO_TUTO));
    }

    #[test]
    fn horntails_trial_doors_open_on_to_the_next_once_their_stage_is_cleared() {
        use crate::events::{HORNTAILS_CAVE, HORNTAIL_TRIAL_I, HORNTAIL_TRIAL_II};
        let mut player = player_with(&[]);
        for (map, stage, next) in [
            (HORNTAIL_TRIAL_I, 1, HORNTAIL_TRIAL_II),
            (HORNTAIL_TRIAL_II, 2, HORNTAILS_CAVE),
        ] {
            player.stats.map_id = map;
            let out = portal("hontale_BR", &player).unwrap();
            assert_eq!(
                out.effects,
                [
                    Effect::RequireStage {
                        stage,
                        refused: Some(keys::HORNTAIL_TRIAL_SEALED)
                    },
                    Effect::PortalSound,
                    Effect::WarpToPortal(next, 0),
                ]
            );
        }
        player.stats.map_id = HORNTAILS_CAVE;
        assert_eq!(portal("hontale_BR", &player), None);
    }

    #[test]
    fn the_way_into_the_trial_grounds_rooms_keeps_someone_and_the_first_key_back() {
        let pink = |key| vec![Effect::Pink(content::text().get(key, &[]))];
        assert_eq!(htpq_into_rooms(true, false), pink(keys::HTPQ_ROOMS_ALONE));
        assert_eq!(htpq_into_rooms(true, true), pink(keys::HTPQ_ROOMS_ALONE));
        assert_eq!(
            htpq_into_rooms(false, true),
            pink(keys::HTPQ_ROOMS_HOLDING_KEY)
        );
        assert_eq!(
            htpq_into_rooms(false, false),
            [Effect::PortalSound, Effect::WarpToPortal(240_050_101, 0)]
        );
    }

    #[test]
    fn the_trial_grounds_rooms_open_on_their_keys_and_the_leader_lifts_the_last_seal() {
        use crate::events::{EventCommand, HTPQ_ENTRY};
        let mut player = player_with(&[]);
        for room in 1..=4 {
            player.stats.map_id = HTPQ_ENTRY + room;
            let door = portal(HTPQ_SEAL, &player).unwrap();
            assert_eq!(
                door.effects,
                [
                    Effect::RequireStage {
                        stage: room as u8,
                        refused: Some(keys::HTPQ_SEALED),
                    },
                    Effect::PortalSound,
                    Effect::WarpToPortal(HTPQ_ENTRY + room + 1, 0),
                ]
            );
        }
        player.stats.map_id = HTPQ_ENTRY + 5;
        assert_eq!(portal(HTPQ_SEAL, &player), None);

        let pink = |key| Effect::Pink(content::text().get(key, &[]));
        let out = [
            Effect::PortalSound,
            Effect::WarpToPortal(HTPQ_ENTRY, named(HTPQ_ENTRY, "st00")),
        ];
        assert_eq!(htpq_last_seal(true, false), out);
        assert_eq!(htpq_last_seal(true, true), out);
        assert_eq!(
            htpq_last_seal(false, false),
            [pink(keys::HTPQ_SEAL_LEADER_ONLY)]
        );
        let lift = htpq_last_seal(false, true);
        assert_eq!(
            lift[..2],
            [
                Effect::EventCommand(EventCommand::ClearStage(5)),
                pink(keys::HTPQ_SEAL_LIFTED),
            ]
        );
        assert_eq!(lift[2..], out);
        if crate::get_game_data().is_ok() {
            assert_eq!(named(HTPQ_ENTRY, "st00"), 1);
        }
    }

    #[test]
    fn the_lamp_sends_the_leaders_team_to_the_light_or_the_dark() {
        use crate::events::{EventCommand, HTPQ_DARK, HTPQ_LAMP, HTPQ_LIGHT};
        let pink = |key| vec![Effect::Pink(content::text().get(key, &[]))];
        let to = |map| {
            vec![
                Effect::PortalSound,
                Effect::EventCommand(EventCommand::MoveTeam(map)),
            ]
        };
        assert_eq!(htpq_lamp_door(true, Some(1)), to(HTPQ_LIGHT));
        assert_eq!(htpq_lamp_door(true, Some(3)), to(HTPQ_DARK));
        for lamp in [Some(0), Some(2), None] {
            assert_eq!(htpq_lamp_door(true, lamp), pink(keys::HTPQ_LAMP_HIT));
        }
        for lamp in [Some(1), Some(3)] {
            assert_eq!(
                htpq_lamp_door(false, lamp),
                pink(keys::HTPQ_LAMP_LEADER_ONLY)
            );
        }
        let Ok(maps) = crate::get_game_data() else {
            return;
        };
        let lamp_room = maps.field(HTPQ_LAMP).unwrap();
        let door = lamp_room.portal_by_name("east00").unwrap();
        assert_eq!(door.script.as_deref(), Some(HTPQ_LAMP_DOOR));
        let lamps: Vec<_> = lamp_room
            .reactors
            .iter()
            .filter(|reactor| reactor.name == HTPQ_LAMP_NAME)
            .map(|reactor| reactor.reactor_id)
            .collect();
        assert_eq!(lamps, [crate::reactors::HTPQ_LAMP]);
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
    fn the_hidden_magic_library_holds_a_cygnus_knight_until_its_shadow_is_dealt_with() {
        let mut searching = player_with(&[(MAYBE_ITS_GRENDEL, QuestState::Started)]);
        let way_in = portal("enterMagiclibrar", &searching).unwrap();
        assert_eq!(
            way_in.effects,
            [
                Effect::RequireEmptyMap(HIDDEN_MAGIC_LIBRARY),
                Effect::PortalSound,
                Effect::WarpToPortal(HIDDEN_MAGIC_LIBRARY, 1),
            ]
        );
        assert!(way_in.occupied.is_some());
        assert!(SOLO_MAPS.contains(&HIDDEN_MAGIC_LIBRARY));
        // Once the shadow is found, or with the quest done, Grendel's house.
        searching
            .quests
            .set_record(LIBRARY_SHADOW_RECORD, SHADOW_SPOTTED);
        let done = player_with(&[(MAYBE_ITS_GRENDEL, QuestState::Completed)]);
        for player in [&searching, &done] {
            assert_eq!(
                warp_of(&portal("enterMagiclibrar", player).unwrap()),
                Some((MAGIC_LIBRARY, 8))
            );
        }

        let way_out = portal("outMagiclib", &searching).unwrap();
        assert_eq!(
            way_out.effects,
            [
                Effect::RequireNoMobs,
                Effect::PortalSound,
                Effect::WarpToPortal(ELLINIA, 26),
            ]
        );
        assert_eq!(
            way_out.occupied,
            Some(content::text().get(keys::MAGIC_LIBRARY_MUSHROOMS_LEFT, &[]))
        );
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
            go("TD_MC_enterboss1", &player_with(&[])),
            Some((WEDDING_HALL_ENTRANCE, 1))
        );

        // Magatia's secret doors open to each lab's side of the basement.
        assert!(pink(&portal("secretDoor", &fresh).unwrap()));
        let mut player = player_with(&[(VERIFYING_THE_PASSWORD, QuestState::Started)]);
        player.stats.map_id = ZENUMIST_HALLWAY;
        assert_eq!(go("secretDoor", &player), Some((SECRET_BASEMENT_PATH, 2)));
        player.stats.map_id = 261020200;
        assert_eq!(go("secretDoor", &player), Some((SECRET_BASEMENT_PATH, 1)));
        // With a password to give, the Secret Passage asks for it.
        player.quests.upsert(game::quest::QuestRecord {
            quest_id: VERIFYING_THE_PASSWORD,
            status: game::quest::QuestStatus::Started {
                kills: Vec::new(),
                info: Some("A1B2C3D4E5".into()),
            },
        });
        assert_eq!(
            portal("secretDoor", &player).unwrap().effects,
            [Effect::OpenNpc(crate::npc_scripts::SECRET_PASSAGE)]
        );
        player.quests.set_info(VERIFYING_THE_PASSWORD, "1");
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
    fn romeo_and_juliets_exit_leads_to_its_sides_lobby() {
        use crate::events::{EventKind, MagatiaSide, ALCADNO_LOBBY, ZENUMIST_LOBBY};
        let mut player = player_with(&[]);
        for (side, lobby) in [
            (MagatiaSide::Zenumist, ZENUMIST_LOBBY),
            (MagatiaSide::Alcadno, ALCADNO_LOBBY),
        ] {
            player.stats.map_id = EventKind::Magatia(side).event().exit;
            let outcome = portal("rnj_exit", &player).unwrap();
            assert_eq!(warp_of(&outcome), Some((lobby, 0)), "{side:?}");
        }
        player.stats.map_id = ZENUMIST_LOBBY;
        assert!(portal("rnj_exit", &player).is_none());
    }

    #[test]
    fn a_magatia_runs_ways_on_wait_for_their_stages() {
        use crate::events::MagatiaSide;
        let mut player = player_with(&[]);
        let shut = content::text().get(keys::MAGATIA_PORTAL_SHUT, &[]);
        for (side, prefix) in [
            (MagatiaSide::Zenumist, "rnj"),
            (MagatiaSide::Alcadno, "jnr"),
        ] {
            let stages = side.stages();
            for (stage, script) in [(1u8, "1_pt00"), (2, "1_out"), (3, "2_out"), (5, "6_out")] {
                player.stats.map_id = stages[usize::from(stage) - 1];
                let outcome = portal(&format!("{prefix}{script}"), &player).unwrap();
                assert_eq!(
                    outcome.effects,
                    [
                        Effect::RequireStage {
                            stage,
                            refused: None
                        },
                        Effect::PortalSound,
                        Effect::WarpToPortal(stages[usize::from(stage)], 0)
                    ]
                );
                assert_eq!(outcome.occupied.as_ref(), Some(&shut));
            }
            // The corridor's way into the boss's room is never shut.
            player.stats.map_id = stages[6] - 1;
            let outcome = portal(&format!("{prefix}12_in"), &player).unwrap();
            assert_eq!(
                outcome.effects,
                [Effect::PortalSound, Effect::WarpToPortal(stages[6], 0)]
            );
            assert_eq!(outcome.occupied, None);
        }
        player.stats.map_id = crate::events::ZENUMIST_LOBBY;
        assert!(portal("rnj1_pt00", &player).is_none());
    }

    #[test]
    fn the_dark_pipe_leads_into_yuletes_lab_once_his_traces_showed_the_way() {
        let mut player = player_with(&[]);
        player.stats.map_id = 261020600;
        let dark = portal("magatia_dark0", &player).unwrap();
        let line = content::text().get(keys::MAGATIA_PIPE_DARK, &[]);
        assert_eq!(dark.effects, [Effect::Pink(line)]);
        assert!(!dark.block);

        player.quests.mark(npc_scripts::YULETE_LAB_OPENED);
        let open = portal("magatia_dark0", &player).unwrap();
        assert_eq!(
            open.effects,
            [Effect::PortalSound, Effect::WarpToPortal(926130000, 2)]
        );
        // `out00`, its way out to Magatia.
        let lab = crate::game_data::get().unwrap().field(926130000).unwrap();
        assert_eq!(lab.portal_by_name("out00").unwrap().id, 2);
    }

    #[test]
    fn a_magatia_hubs_ways_wait_for_their_doors() {
        use crate::events::MagatiaSide;
        let mut player = player_with(&[]);
        let shut = content::text().get(keys::MAGATIA_PORTAL_SHUT, &[]);
        for (side, p) in [
            (MagatiaSide::Zenumist, "rnj"),
            (MagatiaSide::Alcadno, "jnr"),
        ] {
            let stages = side.stages();
            let hub = stages[3];
            for (map, script, door, to) in [
                (hub, "3_in0", "3_out1", (hub + 1, 0)),
                (hub, "3_in1", "3_out2", (hub + 2, 0)),
                (hub + 1, "_201_0", "31_out", (hub, 1)),
                (hub + 2, "_202", "32_out", (hub, 2)),
                (hub, "3_out", "3_out3", (stages[4], 0)),
            ] {
                player.stats.map_id = map;
                let outcome = portal(&format!("{p}{script}"), &player).unwrap();
                let Effect::RequireReactorsSet(names) = outcome.effects[0] else {
                    panic!("{outcome:?}");
                };
                assert_eq!(names, [format!("{p}{door}")]);
                assert_eq!(
                    outcome.effects[1..],
                    [Effect::PortalSound, Effect::WarpToPortal(to.0, to.1)]
                );
                assert_eq!(outcome.occupied.as_ref(), Some(&shut));
            }
        }
        // The doors open the rooms beside them, and their ways back.
        for side in MagatiaSide::ALL {
            let hub = side.stages()[3];
            for (n, (door, room, exit)) in side.side_doors().into_iter().enumerate() {
                assert_eq!(room, hub + 1 + n as i32);
                assert_eq!(
                    magatia_hub_way(side, &format!("3_in{n}")).unwrap().0,
                    [door]
                );
                let back = if n == 0 { "_201_0" } else { "_202" };
                assert_eq!(magatia_hub_way(side, back).unwrap().0, [exit]);
            }
            assert_eq!(magatia_hub_way(side, "3_out").unwrap().0, [side.hub_door()]);
        }
        player.stats.map_id = crate::events::ZENUMIST_LOBBY;
        assert!(portal("rnj3_in0", &player).is_none());
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
    fn the_way_out_of_save_delli_opens_on_the_win_and_marks_the_record() {
        let player = player_with(&[]);
        assert!(matches!(
            &delli_way_out(false, &player)[..],
            [Effect::Pink(_)]
        ));
        let on = [
            Effect::PortalSound,
            Effect::WarpToPortal(crate::events::NAUTILUS_HIDEOUT, 0),
        ];
        assert_eq!(delli_way_out(true, &player), on);
        let player = player_with(&[(DELLI_QUEST, QuestState::Started)]);
        let mut marked = vec![Effect::SetRecord {
            record: DELLI_RECORD,
            value: DELLI_SAVED,
        }];
        marked.extend(on);
        assert_eq!(delli_way_out(true, &player), marked);
    }

    #[test]
    fn the_way_out_of_tyluss_room_leads_on_only_with_his_trust() {
        use crate::events::{TYLUS_EXIT, TYLUS_THANKS, TYLUS_TRUST};
        assert_eq!(
            tylus_way_out(&player_with(&[])),
            [Effect::PortalSound, Effect::Warp(TYLUS_EXIT)]
        );
        let mut player = player_with(&[]);
        player
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(TYLUS_TRUST, 1));
        assert_eq!(
            tylus_way_out(&player),
            [Effect::PortalSound, Effect::WarpToPortal(TYLUS_THANKS, 0)]
        );
    }

    #[test]
    fn the_forests_way_on_opens_stage_by_stage_and_its_prize_map_leads_home() {
        use crate::events::{ELLIN, ELLIN_EXIT, ELLIN_STAGES};
        let on = |map| vec![Effect::PortalSound, Effect::WarpToPortal(map, 0)];
        assert_eq!(ellin_way_on(ELLIN.entry, |_| false), on(ELLIN_STAGES[0]));
        let shut = content::text().get(keys::ELLIN_STAGE1_SHUT, &[]);
        assert_eq!(
            ellin_way_on(ELLIN_STAGES[0], |_| false),
            [Effect::Pink(shut)]
        );
        assert_eq!(
            ellin_way_on(ELLIN_STAGES[0], |s| s == 1),
            on(ELLIN_STAGES[1])
        );
        let shut = content::text().get(keys::ELLIN_STAGE2_SHUT, &[]);
        assert_eq!(
            ellin_way_on(ELLIN_STAGES[1], |s| s == 1),
            [Effect::Pink(shut)]
        );
        assert_eq!(
            ellin_way_on(ELLIN_STAGES[1], |s| s == 2),
            on(ELLIN_STAGES[2])
        );
        assert!(ellin_way_on(ELLIN_STAGES[2], |_| true).is_empty());
        assert!(ellin_way_on(ELLIN_EXIT, |_| true).is_empty());
        assert!(portal(ELLIN_HOME, &player_with(&[])).is_none());
    }

    #[test]
    fn the_mazes_rooms_lead_on_in_turn_or_now_and_then_to_its_end() {
        let maze = crate::events::ELLIN_STAGES[2];
        let to = |spot| Some(vec![Effect::PortalSound, Effect::WarpToPortal(maze, spot)]);
        assert_eq!(ellin_maze_way("party6_stage501", false), to(2));
        assert_eq!(ellin_maze_way("party6_stage514", false), to(15));
        assert_eq!(ellin_maze_way("party6_stage515", false), to(1));
        assert_eq!(ellin_maze_way("party6_stage507", true), to(ELLIN_MAZE_END));
        for script in [
            "party6_stage500",
            "party6_stage516",
            "party6_stage5",
            "party6_stage800",
        ] {
            assert_eq!(ellin_maze_way(script, false), None, "{script}");
        }
        assert_eq!(ellin_maze_way(ELLIN_STAGE, true), None);
    }

    #[test]
    fn the_way_home_takes_the_forests_items_and_gives_a_fragment_after_the_fountain() {
        use crate::events::{ALTAIRE_FRAGMENT, ELLIN_LOBBY, GOLEMS_MARBLE, MONSTER_MARBLE};
        let home = [Effect::PortalSound, Effect::WarpToPortal(ELLIN_LOBBY, 0)];
        let mut player = player_with(&[]);
        assert_eq!(ellin_home(&player, false), home);
        player.inventory.set_slot_limit(InventoryType::Etc, 2);
        player
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(MONSTER_MARBLE, 20));
        player
            .inventory
            .insert(InventoryType::Etc, 2, game::Item::stack(GOLEMS_MARBLE, 1));
        let taken = [
            Effect::TakeItem {
                item_id: MONSTER_MARBLE,
                quantity: 20,
            },
            Effect::TakeItem {
                item_id: GOLEMS_MARBLE,
                quantity: 1,
            },
        ];
        assert_eq!(ellin_home(&player, false), [&taken[..], &home].concat());
        // With the fountain used, a fragment, in the room the marbles leave.
        let fragment = Effect::GiveItem {
            item_id: ALTAIRE_FRAGMENT,
            quantity: 1,
        };
        assert_eq!(
            ellin_home(&player, true),
            [&taken[..], &[fragment], &home].concat()
        );
        // With no room even then, the way stays shut.
        player
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(4_000_000, 1));
        player
            .inventory
            .insert(InventoryType::Etc, 2, game::Item::stack(4_000_001, 1));
        let shut = content::text().get(keys::ELLIN_HOME_NO_ROOM, &[]);
        assert_eq!(ellin_home(&player, true), [Effect::Pink(shut)]);
    }

    #[test]
    fn the_last_stages_way_out_waits_for_the_golem_and_the_leader() {
        use crate::events::{EventCommand, ELLIN};
        let pink = |key| vec![Effect::Pink(content::text().get(key, &[]))];
        assert_eq!(ellin_way_out(false, true), pink(keys::ELLIN_STAGE6_SHUT));
        assert_eq!(ellin_way_out(false, false), pink(keys::ELLIN_STAGE6_SHUT));
        assert_eq!(ellin_way_out(true, false), pink(keys::ELLIN_STAGE6_WAIT));
        assert_eq!(
            ellin_way_out(true, true),
            [
                Effect::PortalSound,
                Effect::EventCommand(EventCommand::MoveTeam(ELLIN.exit))
            ]
        );
    }

    #[test]
    fn the_way_to_papulatus_leads_into_the_past_for_time_leap() {
        use QuestState::{Completed, Started};
        let past = Some(vec![
            Effect::PortalSound,
            Effect::WarpToPortal(PAST_CLOCKTOWER, 0),
        ]);
        // On 6361 with the key, and on the quests after it, into the past.
        assert_eq!(time_leap(&holding(&[(6361, Started)], PAPULATUS_KEY)), past);
        assert_eq!(time_leap(&player_with(&[(6361, Completed)])), past);
        let revealing = [(6361, Completed), (NEW_REVELATION, Started)];
        assert_eq!(time_leap(&player_with(&revealing)), past);
        // Else to the fight.
        assert_eq!(time_leap(&player_with(&[(6361, Started)])), None);
        assert_eq!(time_leap(&player_with(&[])), None);
        let done = [(6361, Completed), (NEW_REVELATION, Completed)];
        assert_eq!(time_leap(&player_with(&done)), None);
    }

    #[test]
    fn the_stable_lets_players_out_with_a_full_bottle_or_none() {
        let quest = [(FRESH_MILK, QuestState::Started)];
        let kitchen = [Effect::PortalSound, Effect::Warp(TANGYOONS_KITCHEN)];
        for bottle in &MILK_BOTTLES[..3] {
            let outcome = portal("end_cow", &holding(&quest, *bottle)).unwrap();
            assert!(matches!(&outcome.effects[..], [Effect::Pink(_)]));
        }
        let full = holding(&quest, MILK_BOTTLES[3]);
        assert_eq!(portal("end_cow", &full).unwrap().effects, kitchen);
        let done = holding(&[(FRESH_MILK, QuestState::Completed)], MILK_BOTTLES[1]);
        assert_eq!(portal("end_cow", &done).unwrap().effects, kitchen);
        assert_eq!(
            portal("end_cow", &player_with(&quest)).unwrap().effects,
            kitchen
        );
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
            // Without the quest in the catalog, the door stays shut rather
            // than fail.
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
    fn shuang_briefs_whoever_reaches_sharenians_outskirts() {
        use crate::guild_quest::{OUTSKIRTS, SHUANG, WAITING_ROOM};
        let data = crate::get_game_data().unwrap();
        assert_eq!(data.field(OUTSKIRTS).unwrap().on_user_enter, None);
        let mut player = player_with(&[]);
        player.stats.map_id = OUTSKIRTS;
        assert_eq!(unscripted_map_enter(&player), [Effect::OpenNpc(SHUANG)]);
        player.stats.map_id = WAITING_ROOM;
        assert!(unscripted_map_enter(&player).is_empty());
    }

    #[test]
    fn the_knights_hall_s_gates_open_on_the_spears_the_stone_and_the_metal() {
        use crate::guild_quest::{KNIGHT_HALL, KNIGHT_STAGE, METAL_GATE, STONE_GATE};
        let data = crate::get_game_data().unwrap();
        let script = |map, name: &str| {
            let field = data.field(map).unwrap();
            field.portal_by_name(name).unwrap().script.clone()
        };
        assert_eq!(
            script(KNIGHT_HALL, "next00").as_deref(),
            Some("speargate_open")
        );
        assert_eq!(
            script(KNIGHT_HALL, "in02").as_deref(),
            Some("stonegate_open")
        );
        assert_eq!(
            script(KNIGHT_HALL + 30, "in00").as_deref(),
            Some("metalgate_open")
        );
        let player = player_with(&[]);
        let spears = portal("speargate_open", &player).unwrap();
        assert_eq!(
            spears.effects[0],
            Effect::RequireStage {
                stage: KNIGHT_STAGE,
                refused: None
            }
        );
        assert_eq!(
            spears.effects[1..],
            [
                Effect::PortalSound,
                Effect::WarpToPortal(KNIGHT_HALL + 1, 0)
            ]
        );
        for (name, gate, to) in [
            ("stonegate_open", &[STONE_GATE], KNIGHT_HALL + 30),
            ("metalgate_open", &[METAL_GATE], KNIGHT_HALL + 31),
        ] {
            let outcome = portal(name, &player).unwrap();
            assert_eq!(outcome.effects[0], Effect::RequireReactorsSet(gate));
            assert_eq!(
                outcome.effects[1..],
                [Effect::PortalSound, Effect::WarpToPortal(to, 0)]
            );
            assert!(outcome.occupied.is_some());
        }
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
        // Before, the cave wall asks for the password.
        let started = player_with(&[(20730, QuestState::Started)]);
        assert_eq!(
            portal("enterDollcave", &started),
            Some(opens(crate::npc_scripts::PUPPETEER_DOOR))
        );
    }

    #[test]
    fn the_puppeteer_waits_in_his_hideout_for_whoever_hunts_him() {
        let at_hideout = |quests: &[(i16, QuestState)]| {
            let mut player = player_with(quests);
            player.stats.map_id = PUPPETEERS_HIDEOUT;
            player
        };
        assert!(SOLO_MAPS.contains(&PUPPETEERS_HIDEOUT));
        let data = crate::get_game_data().unwrap();
        assert_eq!(data.field(PUPPETEERS_HIDEOUT).unwrap().on_user_enter, None);
        assert!(unscripted_map_enter(&at_hideout(&[])).is_empty());
        for (quest, mob_id) in PUPPETEER_HUNTS {
            let mut hunting = at_hideout(&[(quest, QuestState::Started)]);
            assert_eq!(
                unscripted_map_enter(&hunting),
                [Effect::BringOutMob {
                    mob_id,
                    at: PUPPETEER_AT
                }]
            );
            *hunting.quests.kills_mut(quest).unwrap() = vec![1];
            assert!(unscripted_map_enter(&hunting).is_empty(), "beaten");
            let done = at_hideout(&[(quest, QuestState::Completed)]);
            assert!(unscripted_map_enter(&done).is_empty());
        }
        // Elsewhere, nothing.
        let elsewhere = player_with(&[(20730, QuestState::Started)]);
        assert!(unscripted_map_enter(&elsewhere).is_empty());
    }

    #[test]
    fn the_puppeteers_arena_brings_the_puppeteer_out_until_it_falls() {
        let in_arena = |quests: &[(i16, QuestState)]| {
            let mut player = player_with(quests);
            player.stats.map_id = PUPPETEERS_ARENA;
            player
        };
        assert!(SOLO_MAPS.contains(&PUPPETEERS_ARENA));
        let data = crate::get_game_data().unwrap();
        let script = data.field(PUPPETEERS_ARENA).unwrap().on_user_enter.clone();
        assert_eq!(script.as_deref(), Some("dollCave02"));
        let enter = |player: &Player| map_enter("dollCave02", player).unwrap();
        assert!(enter(&in_arena(&[])).is_empty());
        let mut aran = in_arena(&[(TRACES_OF_THE_PUPPETEER, QuestState::Started)]);
        assert_eq!(
            enter(&aran),
            [Effect::BringOutMob {
                mob_id: ARENA_PUPPETEER,
                at: ARENA_PUPPETEER_AT
            }]
        );
        *aran.quests.kills_mut(TRACES_OF_THE_PUPPETEER).unwrap() = vec![1];
        assert!(enter(&aran).is_empty(), "beaten");
        let done = in_arena(&[(TRACES_OF_THE_PUPPETEER, QuestState::Completed)]);
        assert!(enter(&done).is_empty());
    }

    #[test]
    fn an_aran_on_to_the_sealed_garden_fights_its_guardian_alone() {
        let aran = player_with(&[(TO_THE_SEALED_GARDEN, QuestState::Started)]);
        let outcome = portal("enterNepenthes", &aran).unwrap();
        assert_eq!(
            outcome.effects,
            [
                Effect::RequireEmptyMap(SEALED_GARDEN_WAY),
                Effect::RequireEmptyMap(SEALED_GARDEN),
                Effect::PortalSound,
                Effect::WarpToPortal(SEALED_GARDEN_WAY, 2),
            ]
        );
        assert!(outcome.occupied.is_some());
        let done = player_with(&[(TO_THE_SEALED_GARDEN, QuestState::Completed)]);
        assert_eq!(
            warp_of(&portal("enterNepenthes", &done).unwrap()),
            Some((NEGLECTED_STROLLING_PATH, 2))
        );
        assert!(SOLO_MAPS.contains(&SEALED_GARDEN_WAY) && SOLO_MAPS.contains(&SEALED_GARDEN));

        let in_garden = |quests: &[(i16, QuestState)]| {
            let mut player = player_with(quests);
            player.stats.map_id = SEALED_GARDEN;
            player
        };
        let data = crate::get_game_data().unwrap();
        let script = data.field(SEALED_GARDEN).unwrap().on_user_enter.clone();
        assert_eq!(script.as_deref(), Some("sealGarden"));
        let enter = |player: &Player| map_enter("sealGarden", player).unwrap();
        assert!(enter(&in_garden(&[])).is_empty());
        let mut aran = in_garden(&[(TO_THE_SEALED_GARDEN, QuestState::Started)]);
        assert_eq!(
            enter(&aran),
            [Effect::BringOutMob {
                mob_id: GARDEN_GUARDIAN,
                at: GARDEN_GUARDIAN_AT
            }]
        );
        *aran.quests.kills_mut(TO_THE_SEALED_GARDEN).unwrap() = vec![1];
        assert!(enter(&aran).is_empty(), "beaten");
    }

    #[test]
    fn the_spirit_of_rocks_floors_open_with_blakes_requests() {
        let on = |map: i32, quests: &[(i16, QuestState)]| {
            let mut player = player_with(quests);
            player.stats.map_id = map;
            player
        };
        let up = |map: i32, side: &'static str| through(to(map + 10, Portal::Named(side)));
        let shut = || Some(refused(keys::DEPART_NO_ACCESS));
        assert_eq!(
            portal("Depart_topFloor", &on(103040400, &[])),
            Some(opens(JUNE))
        );
        assert_eq!(
            portal("Depart_topOut", &on(103040400, &[])),
            Some(through(Effect::WarpToPortal(KERNING_SQUARE_6F, 1)))
        );
        assert_eq!(
            portal("Depart_ToKerning", &on(103000310, &[])),
            Some(through(Effect::WarpToPortal(103000302, 0)))
        );

        // Past the Cheap Amplifiers from Composition Fee on, past the Fancy
        // Amplifiers from Say "NO" to Plagiarism on.
        let fee = [(COMPOSITION_FEE, QuestState::Started)];
        let song = [(2286, QuestState::Started)];
        assert_eq!(portal("Depart_goFoward0", &on(103040410, &song)), shut());
        assert_eq!(
            portal("Depart_goFoward0", &on(103040410, &fee)),
            Some(up(103040410, "right00"))
        );
        assert_eq!(portal("Depart_goFoward1", &on(103040420, &fee)), shut());
        let done = [
            (COMPOSITION_FEE, QuestState::Completed),
            (SAY_NO_TO_PLAGIARISM, QuestState::Completed),
        ];
        assert_eq!(
            portal("Depart_goFoward1", &on(103040420, &done)),
            Some(up(103040420, "right01"))
        );
        // The VIP Zone's floors are open.
        for map in [103040440, 103040450] {
            assert_eq!(
                portal("Depart_goFoward1", &on(map, &[])),
                Some(up(map, "right01"))
            );
        }
        assert_eq!(
            portal("Depart_goBack00", &on(103040420, &[])),
            Some(through(to(103040410, Portal::Named("left00"))))
        );
        assert_eq!(
            portal("Depart_goBack01", &on(103040460, &[])),
            Some(through(to(103040450, Portal::Named("left01"))))
        );
    }

    #[test]
    fn an_aran_runs_the_field_of_wolves_and_leaves_the_test_room_once_the_wolves_are_down() {
        let advised = [(SCADURS_ADVICE, QuestState::Started)];
        let rider = portal("enterRider", &player_with(&advised)).unwrap();
        assert_eq!(
            rider.effects,
            [
                Effect::RequireEmptyMap(FIELD_OF_WOLVES),
                Effect::PortalSound,
                Effect::WarpToPortal(FIELD_OF_WOLVES, 1),
            ]
        );
        assert!(rider.occupied.is_some());
        let shut = portal("enterRider", &player_with(&[])).unwrap();
        assert!(matches!(shut.effects[..], [Effect::Pink(_)]));

        let clear = portal("clearRider", &player_with(&advised)).unwrap();
        assert!(matches!(
            clear.effects[..],
            [
                Effect::SetRecord {
                    record: SCADURS_ADVICE_RECORD,
                    value: WOLF_RUN_DONE,
                },
                Effect::Pink(_),
            ]
        ));
        let mut cleared = player_with(&advised);
        cleared
            .quests
            .set_record(SCADURS_ADVICE_RECORD, WOLF_RUN_DONE);
        assert!(portal("clearRider", &cleared).unwrap().effects.is_empty());
        assert!(matches!(
            portal("enterRider", &cleared).unwrap().effects[..],
            [Effect::Pink(_)]
        ));
        assert!(portal("clearRider", &player_with(&[]))
            .unwrap()
            .effects
            .is_empty());

        let out = |quests: &[(i16, QuestState)]| portal("outtestWolf", &player_with(quests));
        let tested = out(&[(THE_WOLVES_TEST, QuestState::Started)]).unwrap();
        assert_eq!(
            tested.effects,
            [
                Effect::RequireNoMobs,
                Effect::SetRecord {
                    record: WOLVES_TEST_RECORD,
                    value: WOLF_RUN_DONE,
                },
                Effect::PortalSound,
                Effect::WarpToPortal(WOLVES_DEN, 0),
            ]
        );
        assert!(tested.occupied.is_some());
        assert_eq!(
            out(&[]).unwrap().effects,
            [
                Effect::RequireNoMobs,
                Effect::PortalSound,
                Effect::WarpToPortal(WOLVES_DEN, 0),
            ]
        );
        assert!(SOLO_MAPS.contains(&FIELD_OF_WOLVES) && SOLO_MAPS.contains(&WOLVES_TEST_ROOM));
    }

    #[test]
    fn mu_gongs_back_street_and_shrine_open_for_an_aran_on_its_errands() {
        let on = |quests: &[(i16, QuestState)]| portal("enterBackStreet", &player_with(quests));
        let letter = [
            (SHADOW_KNIGHTS_LETTER, QuestState::Started),
            (HOW_TO_MEET_MU_GONG, QuestState::Completed),
        ];
        assert_eq!(warp_of(&on(&letter).unwrap()), Some((BACK_STREET, 0)));
        let refused = on(&letter[..1]).unwrap();
        assert_eq!(warp_of(&refused), None);
        assert!(matches!(refused.effects[..], [Effect::Pink(_)]));
        // From the letter's end until the Shadow Knight is caught, Mu Gong
        // still has business with them, the Test included.
        let letter_done = (SHADOW_KNIGHTS_LETTER, QuestState::Completed);
        for errand in [
            vec![letter_done],
            vec![letter_done, (21746, QuestState::Started)],
            vec![letter_done, (21746, QuestState::Completed)],
            vec![letter_done, (CATCH_THE_SHADOW_KNIGHT, QuestState::Started)],
        ] {
            assert_eq!(
                warp_of(&on(&errand).unwrap()),
                Some((BACK_STREET, 0)),
                "{errand:?}"
            );
        }
        let done = on(&[
            letter_done,
            (CATCH_THE_SHADOW_KNIGHT, QuestState::Completed),
        ])
        .unwrap();
        assert_eq!(warp_of(&done), None);
        assert_eq!(
            warp_of(&portal("outSpecialSchool", &player_with(&[])).unwrap()),
            Some((BACK_STREET, 1))
        );
        assert!(SOLO_MAPS.contains(&MU_GONGS_ARENA) && SOLO_MAPS.contains(&SEALED_SHRINE));

        let in_shrine = |quests: &[(i16, QuestState)]| {
            let mut player = player_with(quests);
            player.stats.map_id = SEALED_SHRINE;
            player
        };
        let data = crate::get_game_data().unwrap();
        assert_eq!(data.field(SEALED_SHRINE).unwrap().on_user_enter, None);
        assert!(unscripted_map_enter(&in_shrine(&[])).is_empty());
        let mut aran = in_shrine(&[(CATCH_THE_SHADOW_KNIGHT, QuestState::Started)]);
        assert_eq!(
            unscripted_map_enter(&aran),
            [Effect::BringOutMob {
                mob_id: SHADOW_KNIGHT,
                at: SHADOW_KNIGHT_AT
            }]
        );
        *aran.quests.kills_mut(CATCH_THE_SHADOW_KNIGHT).unwrap() = vec![1];
        assert!(unscripted_map_enter(&aran).is_empty(), "beaten");
    }

    #[test]
    fn an_aran_sees_the_ellin_stone_taken_then_finds_athena_in_their_own_time() {
        let at = |map_id, quests: &[(i16, QuestState)]| {
            let mut player = player_with(quests);
            player.stats.map_id = map_id;
            player
        };
        let data = crate::get_game_data().unwrap();
        assert_eq!(
            warp_of(&portal("enterWarehouse", &player_with(&[])).unwrap()),
            Some((ELLIN_WAREHOUSE, 0))
        );
        assert_eq!(data.field(ELLIN_WAREHOUSE).unwrap().on_user_enter, None);
        assert!(unscripted_map_enter(&at(ELLIN_WAREHOUSE, &[])).is_empty());
        let mut aran = at(
            ELLIN_WAREHOUSE,
            &[(SEAL_STONE_OF_ELLIN, QuestState::Completed)],
        );
        assert_eq!(
            unscripted_map_enter(&aran),
            [
                Effect::SetRecord {
                    record: WAREHOUSE_RECORD,
                    value: WAREHOUSE_SEEN,
                },
                Effect::BringOutMob {
                    mob_id: WAREHOUSE_GENTLEMAN,
                    at: WAREHOUSE_GENTLEMAN_AT,
                },
            ]
        );
        aran.quests.set_record(WAREHOUSE_RECORD, WAREHOUSE_SEEN);
        assert!(unscripted_map_enter(&aran).is_empty(), "seen");

        assert_eq!(data.field(BOWMAN_SCHOOL).unwrap().on_user_enter, None);
        assert!(unscripted_map_enter(&at(BOWMAN_SCHOOL, &[])).is_empty());
        let mut aran = at(BOWMAN_SCHOOL, &[(LOST_SEAL_STONE, QuestState::Completed)]);
        assert_eq!(
            unscripted_map_enter(&aran),
            [Effect::SetRecord {
                record: ATHENA_FOUND_RECORD,
                value: ATHENA_FOUND,
            }]
        );
        aran.quests.set_record(ATHENA_FOUND_RECORD, ATHENA_FOUND);
        assert!(unscripted_map_enter(&aran).is_empty(), "found");
    }

    #[test]
    fn the_puppeteer_waits_in_lith_harbors_info_shop_until_he_falls() {
        let in_shop = |quests: &[(i16, QuestState)]| {
            let mut player = player_with(quests);
            player.stats.map_id = LITH_INFO_SHOP;
            player
        };
        let data = crate::get_game_data().unwrap();
        assert_eq!(data.field(LITH_INFO_SHOP).unwrap().on_user_enter, None);
        assert!(unscripted_map_enter(&in_shop(&[])).is_empty());
        let mut aran = in_shop(&[(STRANGE_INFORMATION, QuestState::Started)]);
        let waiting = [Effect::BringOutMob {
            mob_id: PUPPETEER_IN_LITH,
            at: PUPPETEER_IN_LITH_AT,
        }];
        assert_eq!(unscripted_map_enter(&aran), waiting);
        assert_eq!(
            crate::quests::kill_records(&mut aran, PUPPETEER_IN_LITH),
            [(STRANGE_INFORMATION_RECORD, "2")]
        );
        assert!(unscripted_map_enter(&aran).is_empty(), "beaten");
        assert!(crate::quests::kill_records(&mut aran, PUPPETEER_IN_LITH).is_empty());
        let done = in_shop(&[(STRANGE_INFORMATION, QuestState::Completed)]);
        assert!(unscripted_map_enter(&done).is_empty());
        // Without the quest, his fall sets nothing.
        let mut passing = in_shop(&[]);
        assert!(crate::quests::kill_records(&mut passing, PUPPETEER_IN_LITH).is_empty());
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
    fn the_pirate_ships_first_way_on_waits_for_its_monsters() {
        let outcome = portal("davy_next0", &player_with(&[])).unwrap();
        assert_eq!(
            outcome.effects,
            [
                Effect::RequireNoMobs,
                Effect::PortalSound,
                Effect::WarpToPortal(crate::events::PIRATE_STAGES[1], 0)
            ]
        );
        assert!(outcome.occupied.is_some());
        // The second stage's waits for Guon to have every Mark, and says
        // so in pink only.
        let outcome = portal("davy_next1", &player_with(&[])).unwrap();
        assert_eq!(
            outcome.effects,
            [
                Effect::RequireStage {
                    stage: 2,
                    refused: None
                },
                Effect::PortalSound,
                Effect::WarpToPortal(crate::events::PIRATE_STAGES[2], 0)
            ]
        );
        let shut = content::text().get(keys::PIRATE_MARKS_SHUT, &[]);
        assert_eq!(outcome.occupied, Some(shut));
        // The decks' wait for their crews, and their hidden ways down to
        // the crews' rooms for nothing.
        use crate::events::PIRATE_STAGES;
        for (script, to) in [
            ("davy_next2", PIRATE_STAGES[3]),
            ("davy_next3", PIRATE_STAGES[4]),
        ] {
            let outcome = portal(script, &player_with(&[])).unwrap();
            assert_eq!(
                outcome.effects,
                [
                    Effect::RequireNoMobs,
                    Effect::PortalSound,
                    Effect::WarpToPortal(to, 0)
                ],
                "{script}"
            );
            assert!(outcome.occupied.is_some());
        }
        let outcome = portal("davy_next4", &player_with(&[])).unwrap();
        assert_eq!(
            outcome.effects,
            [
                Effect::RequireReactorsSet(&["sMob1", "sMob2", "sMob3", "sMob4"]),
                Effect::RequireNoMobs,
                Effect::PortalSound,
                Effect::WarpToPortal(PIRATE_STAGES[5], 0),
                Effect::EventCommand(crate::events::EventCommand::PastTheDoors),
            ]
        );
        assert!(outcome.occupied.is_some());
        for (script, to) in [("davy2_hd1", 925_100_202), ("davy3_hd1", 925_100_302)] {
            let outcome = portal(script, &player_with(&[])).unwrap();
            assert_eq!(
                outcome.effects,
                [Effect::PortalSound, Effect::WarpToPortal(to, 0)],
                "{script}"
            );
        }
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

    #[test]
    fn the_carnival_field_leads_back_and_the_revive_rooms_to_battle() {
        let mut player = player_with(&[]);
        let out = |to| {
            vec![
                Effect::SaveLocation(CARNIVAL_RETURN, None),
                Effect::PortalSound,
                Effect::Warp(to),
            ]
        };
        assert_eq!(portal("mc_out", &player).unwrap().effects, out(102000000));
        player.saved_locations.insert(CARNIVAL_RETURN, 220000000);
        assert_eq!(portal("mc_out", &player).unwrap().effects, out(220000000));
        assert_eq!(
            portal("MCrevive3", &player).unwrap().effects,
            [Effect::PortalSound, Effect::WarpToPortal(980000301, 0)]
        );
        assert_eq!(portal("MCrevive7", &player), None);
    }

    #[test]
    fn the_dojo_leads_on_floor_by_floor_and_out_to_the_board() {
        use crate::events::{DOJO_BOARD_MAP, DOJO_FIRST, DOJO_ROOFTOP};
        let record = game::DojoRecord {
            points: 40,
            ..Default::default()
        };
        let on = |points: i32, total: i32, to| {
            let (points_s, total_s) = (points.to_string(), total.to_string());
            vec![
                Effect::DojoPoints(points),
                Effect::Pink(content::text().get(
                    keys::DOJO_POINTS,
                    &[("points", points_s.as_str()), ("total", total_s.as_str())],
                )),
                Effect::PortalSound,
                Effect::WarpToPortal(to, 0),
            ]
        };
        assert_eq!(dojo_next(1, &record, false), on(2, 42, DOJO_FIRST + 100));
        assert_eq!(dojo_next(37, &record, false), on(8, 48, DOJO_FIRST + 3700));
        // The last floor earns none, but pays for every point held.
        assert_eq!(
            dojo_next(38, &record, false),
            [
                Effect::PortalSound,
                Effect::WarpToPortal(DOJO_ROOFTOP, 0),
                Effect::GainExp(80_000)
            ]
        );
        // Going on from the medal's boss, round 5 on floor 5, wins toward it.
        let trying = game::DojoRecord {
            vanquisher_stage: 5,
            vanquisher_kills: 9,
            ..record
        };
        let win = Effect::DojoMedal {
            stage: 5,
            kills: 10,
        };
        assert_eq!(dojo_next(5, &trying, false)[0], win);
        assert_eq!(dojo_next(4, &trying, false), on(2, 42, DOJO_FIRST + 400));
        assert!(!dojo_next(7, &trying, false).contains(&win));

        // With a party: no point for going alone and no medal win, and the
        // way on to a rest floor gathers everyone there instead.
        use crate::events::{EventCommand, DOJO_PARTY_FIRST};
        assert_eq!(
            dojo_next(4, &trying, true),
            on(1, 41, DOJO_PARTY_FIRST + 400)
        );
        assert_eq!(
            dojo_next(5, &trying, true),
            [Effect::EventCommand(EventCommand::DojoRest(6))]
        );
        assert_eq!(dojo_next(38, &record, true), dojo_next(38, &record, false));
        // Those gathered from floor 3 earn floors 3 to 5's in turn.
        let gathered = dojo_points(3..=5, &record, false);
        assert_eq!(gathered.len(), 6);
        assert_eq!(gathered[4], Effect::DojoPoints(1));
        assert_eq!(
            gathered[5],
            Effect::Pink(
                content::text().get(keys::DOJO_POINTS, &[("points", "1"), ("total", "43")])
            )
        );

        // Out to where the board was touched, or to the board.
        let out = |to| {
            vec![
                Effect::SaveLocation(DOJO_RETURN, None),
                Effect::PortalSound,
                Effect::Warp(to),
            ]
        };
        let mut player = player_with(&[]);
        assert_eq!(
            portal("dojang_exit", &player).unwrap().effects,
            out(DOJO_BOARD_MAP)
        );
        player.saved_locations.insert(DOJO_RETURN, 250000000);
        assert_eq!(
            portal("dojang_exit", &player).unwrap().effects,
            out(250000000)
        );
    }

    #[test]
    fn one_rand_ola_portal_a_level_leads_on_and_the_rest_back() {
        if crate::get_game_data().is_err() {
            return;
        }
        let warp = |to, start| Some(vec![Effect::PortalSound, Effect::WarpToPortal(to, start)]);
        let mut picks = std::collections::HashSet::new();
        for seed in 0..20 {
            for (map_id, start, ways, next) in [
                (109_030_001, 13, 5, (109_030_002, 1)),
                (109_030_002, 1, 8, (109_030_003, 1)),
                (109_030_003, 1, 16, (crate::gm_event::WON_MAP, 2)),
            ] {
                let names = (0..ways).map(|n| format!("ch{n:02}"));
                let on: Vec<_> = names
                    .filter(|name| rand_ola(map_id, name, seed) == warp(next.0, next.1))
                    .collect();
                assert_eq!(on.len(), 1, "{map_id} {seed}");
                let back = format!("ch{:02}", (on[0][2..].parse::<u8>().unwrap() + 1) % ways);
                assert_eq!(rand_ola(map_id, &back, seed), warp(map_id, start));
                assert_eq!(rand_ola(map_id, &on[0], seed), warp(next.0, next.1));
                picks.insert((map_id, on[0].clone()));
            }
        }
        // The portal that leads on isn't the same for everyone.
        assert!(picks.len() > 3);
        assert_eq!(rand_ola(109_040_000, "ch00", 0), None);
    }
}
