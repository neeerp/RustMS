//! Native NPC and quest scripts. Each is a list of pages with effects on its
//! answers; the runtime applies the effects (and persists them with any
//! quest change) before showing the next page. Text comes from the content
//! pack (`crate::content`), with committed placeholders. Lua is deferred;
//! these keep the effect-returning shape a scripting runtime would have.

use crate::carnival;
use crate::content;
use crate::conti::{self, Route, State};
use crate::dialogue::{DialoguePage, Prompt};
use crate::events::{
    self, Aura, Balloon, Cloto, Entry, EventCommand, EventKind, Find, HandIn, HeadCount,
    LeverCheck, MagatiaSide, PyramidMode, Standing, Statue,
};
use crate::gachapon::{self, Prize};
use crate::guild_quest;
use crate::item_makers::{self, Recipe};
use crate::items;
use crate::packet::handle::world;
use crate::refiners::{self, Refine};
use crate::rides;
use crate::scripts::{self, Effect, FLIGHT_TO_TEMPLE, FRESH_MILK, MILK_BOTTLES, MINI_DRACO};
use crate::squads::{self, SquadCommand, SquadView};
use crate::stylists::{self, Look, Pick, Service};
use game::alliance::{self, Alliance};
use game::character::{
    CARNIVAL_RETURN, DOJO_RETURN, EVENT_RETURN, FLORINA_RETURN, PYRAMID_RETURN, WORLD_TOUR_RETURN,
};
use game::family::FamilyInfo;
use game::guild::{self, Guild};
use game::marriage;
use game::wedding::{self, GuestRefusal, PartnerRefusal, Reservation, Venue, Vows, WeddingStage};
use game::{Inventory, InventoryType, Player, QuestState};
use game_data::server_content::keys;
use rand::seq::SliceRandom;
use rand::Rng;
use std::time::SystemTime;

/// Roger's Apple, the tutorial's lesson on items.
pub const ROGERS_APPLE_QUEST: i16 = 1021;
const ROGERS_APPLE: i32 = 2010007;
const APPLE: i32 = 2010000;
const GREEN_APPLE: i32 = 2010009;
const ROGER: i32 = 2000;
const ROBIN: i32 = 2003;
const MAPLE_ADMINISTRATOR: i32 = 2007;
const SHANKS: i32 = 22000;
/// Meren's Maker lessons (quests 6029-6036): the three teachers.
const CARSON: i32 = 2111000;
const HUGHES: i32 = 2012017;
const MEREN: i32 = 2110004;
const GOLD_ANVIL: i32 = 4031980;
/// Trainer Bartos on the Pet-Walking Road, who teaches Follow the Lead
/// (quests 4646 and 4647), and the hidden spot at the top of the road
/// (`multipet_success`) where his test's note lies.
const BARTOS: i32 = 1012006;
pub const PET_TEST_NOTE_SPOT: i32 = 9102100;
const PET_TEST_QUEST: i16 = 4646;
const PET_SECRET_QUEST: i16 = 4647;
const HIDDEN_NOTE: i32 = 4031921;
const PET_SNACK: i32 = 5460000;
/// Garnox in New Leaf City and Singapore, who evolves pets with the Rock of Evolution
/// for a fee (Cosmic's 10,000 mesos): the Baby Dragon (quest 8185), the
/// Baby Robo with 50 Cheap Batteries (4659), and a grown pet again (8189).
const GARNOX: i32 = 9102001;
const EVOLUTION_FEE: i32 = 10_000;
const CHEAP_BATTERY: i32 = 4000111;
const ROBO_UPGRADE_QUEST: i16 = 4659;
/// Heracle, at the guild headquarters. The runtime loads the player's
/// guild for his script (`NpcScript::heracle`).
pub const HERACLE: i32 = 2010007;
/// Lea, beside him, who makes guild emblems; her script needs the guild
/// too (`NpcScript::lea`).
pub const LEA: i32 = 2010008;
/// Lenario, beside Heracle, who founds alliances; his script needs the
/// guild, its alliance, and the party (`NpcScript::lenario`).
pub const LENARIO: i32 = 2010009;
/// The boards that show the guild ranking when clicked, without a word:
/// the Honorable Rock in Perion and the Guild Rank Board at the Excavation
/// Site's camp (Cosmic's `displayGuildRanks`).
pub const RANK_BOARDS: [i32; 2] = [9040004, 9040008];
/// Nicole, the Cathedral's assistant, and Bonnie, the Chapel's, who book
/// weddings in Amoria; their script needs the couple's booking
/// (`NpcScript::wedding_assistant`).
pub const NICOLE: i32 = 9201005;
pub const BONNIE: i32 = 9201008;
/// Victoria, the Cathedral's wedding coordinator in Amoria, who explains
/// how a wedding is booked (report b066).
const VICTORIA: i32 = 9201013;
/// Wayne, the Chapel's wedding coordinator in Amoria, who begins a booked
/// wedding there (report b071); his script needs the couple's booking
/// (`NpcScript::wedding_starter`).
pub const WAYNE: i32 = 9201012;
/// Pelvis Bebop, who marries the couple at the Chapel's altar; his script
/// needs the wedding (`NpcScript::wedding_priest`).
pub const CHAPEL_PRIEST: i32 = 9201011;
/// Jackie, at the Chapel's altar, who sees the wedding out
/// (`NpcScript::wedding_staff`).
pub const JACKIE: i32 = 9201009;
/// High Priest John, who begins the Cathedral's weddings in Amoria and
/// marries the couple at its altar (report b075).
pub const HIGH_PRIEST: i32 = 9201002;
/// Nancy, the Cathedral's assistant at its altar, Jackie's counterpart.
pub const NANCY: i32 = 9201007;
/// Travis, who in a wedding's party room sends guests back to Amoria
/// (`NpcScript::wedding_party`).
pub const TRAVIS: i32 = 9201010;
/// Robin, who in a wedding's party room shows guests to its exit.
pub const WEDDING_ROBIN: i32 = 9201021;
/// Debbie, who in the Cathedral's hall and at its altar takes the couple's
/// wishlist, as Travis does at the Chapel (`NpcScript::wedding_greeter`).
pub const DEBBIE: i32 = 9201006;
/// Pila, in Amoria and the halls, who keeps the wedding gifts
/// (`NpcScript::pila`).
pub const PILA: i32 = 9201014;
/// Lakelis, who sends parties into Kerning City's party quest; her script
/// needs to know whether the party may go (`NpcScript::lakelis`).
pub const LAKELIS: i32 = 9020000;
/// Nella, who lets players out of the party quest and back to town.
const NELLA: i32 = 9020002;
/// Cloto, who sets each stage of Kerning City's party quest.
pub const CLOTO: i32 = 9020001;
/// Tory, who takes parties up to Primrose Hill from Henesys Park.
pub const TORY: i32 = 1012112;
/// Tommy, at Primrose Hill's way out.
const TOMMY: i32 = 1012113;
/// Growlie, the hungry one on Primrose Hill.
const GROWLIE: i32 = 1012114;
/// The Red Sign on Eos Tower's 101st floor, which sends parties into
/// Ludibrium's party quest; its script needs to know whether the party may
/// go (`NpcScript::red_sign`).
pub const RED_SIGN: i32 = 2040034;
/// Wonky the Fairy in Orbis, who sends parties into Orbis's party quest
/// (`NpcScript::wonky`), and lets players out from its cloud stage.
pub const WONKY: i32 = 2013000;
/// Eak, at the Orbis party quest's way out.
pub const EAK: i32 = 2013001;
/// Minerva, whose statue brought back to life wins Orbis's party quest,
/// and who gives the team that won it its prizes.
pub const MINERVA: i32 = 2013002;
/// The balloon on Ludibrium's maze stage.
pub const AQUA_BALLOON: i32 = 2040041;
/// The balloon on Ludibrium's stage 8, whose puzzle is its crates.
pub const BLUE_BALLOON: i32 = 2040043;
/// The balloon on Ludibrium's bonus stage, which lets players out early.
pub const PINK_BALLOON: i32 = 2040045;
/// Arturo, who gives the team that won Ludibrium's party quest its reward.
pub const ARTURO: i32 = 2040035;
/// Sgt. Anderson, who lets players out of Ludibrium's party quest and back
/// to Eos Tower.
const ANDERSON: i32 = 2040047;
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
/// Corba at Leafre Station, who turns the player into a dragon for the
/// flight to the Temple of Time.
const CORBA: i32 = 2082003;
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
/// destination, passage), as `station_in` names them.
const ISA: i32 = 2012006;
const ORBIS_TUNNEL: i32 = 200000110;
const ISA_PLATFORMS: [(&str, &str, i32); 6] = [
    ("Ship", "Victoria Island", ORBIS_TUNNEL),
    ("Ship", "Ludibrium", 200000120),
    ("Ship", "Leafre", 200000130),
    ("Hak", "Mu Lung", 200000140),
    ("Genie", "Ariant", 200000150),
    ("Ship", "Ereve", 200000160),
];
/// The ferries' pilots, each with the one ride they sell and its fare
/// (Cosmic's): Kiru between Orbis and Ereve, Kiriru between Ellinia and
/// Ereve, and Puro between Lith Harbor and Rien.
const FERRIES: [(i32, rides::Ride, i32); 6] = [
    (1100008, rides::ORBIS_TO_EREVE, 1000),
    (1100004, rides::EREVE_TO_ORBIS, 1000),
    (1100007, rides::ELLINIA_TO_EREVE, 1000),
    (1100003, rides::EREVE_TO_ELLINIA, 1000),
    (1200004, rides::LITH_TO_RIEN, 800),
    (1200003, rides::RIEN_TO_LITH, 800),
];
/// The crane (`crane`), on Orbis Station's platform, at Mu Lung Temple and
/// in Herb Town: a ride between Orbis and Mu Lung, and straight between Mu
/// Lung and Herb Town (Cosmic's fares).
const CRANE: i32 = 2090005;
const CRANE_PLATFORM: i32 = 200000141;
const MU_LUNG_TEMPLE: i32 = 250000100;
const HERB_TOWN: i32 = 251000000;
const CRANE_FARE: i32 = 1500;
const HERB_TOWN_FARE: i32 = 500;
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
/// offers the free train to Kerning Square and Line 3's construction
/// sites) and New Leaf City's (`NLC_Move`).
const SUBWAY_GATES: [(i32, &Route); 2] = [
    (KERNING_CITY_GATE, &conti::TO_NLC),
    (9201068, &conti::TO_KERNING_CITY),
];
const KERNING_CITY_GATE: i32 = 1052007;
/// Line 3's construction sites under Kerning City's station, B1 to B3:
/// Jake sells a ticket to each site whose quest of Shumi's (2055 to 2057)
/// the player has begun, the gate takes it to the site's first map, and
/// the site's way out sends the player back to the ticketing booth. The
/// box at each site's end is a pile (`PATIENCE_PILES`).
const JAKE: i32 = 1052006;
const SITE_EXIT: i32 = 1052011;
/// The item makers (`item_makers`).
const SERRYL: i32 = 1091003;
const NEVE: i32 = 2010003;
const MOS: i32 = 2080000;
const FRANCOIS: i32 = 1032002;
const CHRISHRAMA: i32 = 1061000;
const GORDON: i32 = 2020002;
const SARAH: i32 = 2040020;
const TARA: i32 = 2040021;
const RYDOLE: i32 = 2040022;
/// Mos makes Cornian's Dagger while either of these quests is under way.
const CORNIAN_QUESTS: [i16; 2] = [7301, 7303];
/// The money sack drawn beside a price.
const MESO_ICON: i32 = 4031138;
/// NPCs that set one character of an info quest's string
/// (`game::quest::INFO_QUESTS`): `(npc, quest, slot, from, to)`.
const INFO_SETTERS: [(i32, i16, u8, u8, u8); 3] = [
    (PALACE_OASIS, 3900, 0, b'4', b'5'),
    (SECRET_WALL, 3927, 0, b'0', b'1'),
    (MAGATIA_WALL, 3311, 0, b'4', b'5'),
];
const PALACE_OASIS: i32 = 2103000;
/// How to Shoo Away the Evil (2236): the Shaman Rocks of Victoria's central
/// dungeon each take one of Chrishrama's six charms, marking their
/// character of the quest's string: `(map, rock, which of that rock's NPCs
/// on the map in Map.nx order, slot)`. Sleepywood's Swamp has two.
const SHAMAN_ROCKS: [(i32, i32, u8, u8); 6] = [
    (105050200, 1063012, 0, 0),
    (105060000, 1063012, 0, 1),
    (105070000, 1063012, 0, 2),
    (105090000, 1063013, 0, 3),
    (105090000, 1063013, 1, 4),
    (105090100, 1063013, 0, 5),
];
const SHOO_AWAY_THE_EVIL: i16 = 2236;
const SHAMAN_CHARM: i32 = 4032263;
const SECRET_WALL: i32 = 2103001;
const MAGATIA_WALL: i32 = 2111011;
/// Ariant's houses and cupboards, each taking one of a quest's four
/// deliveries: `(npc, quest, item, slot)`. A delivery turns its slot of
/// the info string from '2' to '3'.
const ARIANT_DELIVERIES: [(i32, i16, i32, u8); 8] = [
    (2103003, 3929, 4031580, 0),
    (2103005, 3929, 4031580, 1),
    (2103004, 3929, 4031580, 2),
    (2103006, 3929, 4031580, 3),
    (2103009, 3926, 4031579, 0),
    (2103011, 3926, 4031579, 1),
    (2103010, 3926, 4031579, 2),
    (2103012, 3926, 4031579, 3),
];
/// Collecting Meteorite Samples (3421): the six meteorites in Kulan Field
/// (2050014 to 2050019) each give a Meteorite Sample (4031117) once, the
/// info string's slot for each turning from '0' to '1'; with all six had
/// and fewer than six held, any gives another.
const METEORITE_SAMPLES: i16 = 3421;
const METEORITE_SAMPLE: i32 = 4031117;
const METEORITES: std::ops::RangeInclusive<i32> = 2050014..=2050019;
/// Find Fresh Milk (2180): Tangyoon (1092000) hands over an empty bottle
/// and sends the player to his stable, where each mother cow fills a third
/// of it, but not twice running (the quest's record holds the last one),
/// and a calf drinks it all.
const TANGYOON: i32 = 1092000;
const STABLE: i32 = 912000100;
/// The mother cows, each with the number the quest's record keeps for it.
const MOTHER_COWS: [(i32, u16); 2] = [(1092090, 1), (1092091, 2)];
const CALVES: [i32; 2] = [1092094, 1092095];
/// Shining Stone (2166): the Shiny Stone (1092016, `nautil_stone`) in the
/// Nautilus's Generator Room sets the quest's string to what Sharyl's
/// completion reads (`Check/1/infoex`).
const SHINY_STONE: i32 = 1092016;
const SHINING_STONE: i16 = 2166;
const SHINING_STONE_TOUCHED: &str = "5";
/// Eliza's Eternal Sleep (3114): the seven strings of the goddess's harp in
/// Eliza's Garden (920020000), 2012027 to 2012033, each sound their note
/// (`Sound.nx/Field.img/orbis/<note>`) to the map. While the quest is under
/// way, its string counts the notes of Twinkle Twinkle Little Star (the
/// book its start gives) played right so far; a wrong one starts it over,
/// and the whole tune is what Eliza's completion reads (`Check/1/infoex`
/// "42").
const HARP_STRINGS: std::ops::RangeInclusive<i32> = 2012027..=2012033;
const HARP_SOUNDS: [&str; 7] = ["do", "re", "mi", "pa", "sol", "la", "si"];
/// The tune, as the notes of the strings from 2012027 up.
const HARP_NOTES: &[u8; 7] = b"CDEFGAB";
const TWINKLE_TWINKLE: &[u8; 42] = b"CCGGAAGFFEEDDCGGFFEEDGGFFEEDCCGGAAGFFEEDDC";
const ELIZAS_ETERNAL_SLEEP: i16 = 3114;
/// The Black Magician's Magic Pentagram (3345): the four circles of the
/// pentagram in the Black Magician's lab (261040000), 2111020 to 2111023,
/// touched in order while the quest is under way, count up its string, the
/// last taking the three Magic Stones; "4" is what Alcaster's completion
/// reads. One touched out of turn, or the last without the stones, starts
/// it over.
const PENTAGRAM_CIRCLES: std::ops::RangeInclusive<i32> = 2111020..=2111023;
const MAGIC_PENTAGRAM: i16 = 3345;
const MAGIC_STONES: [i32; 3] = [4031739, 4031740, 4031741];
const SITE_QUESTS: [i16; 3] = [2055, 2056, 2057];
const SITE_TICKETS: [i32; 3] = [4031036, 4031037, 4031038];
const SITE_ENTRANCES: [i32; 3] = [103000900, 103000903, 103000906];
const SITE_TICKET_PRICE: i32 = 1000;
const SUBWAY_TICKETING: i32 = 103000100;
/// The airports' booths, Irene (`sellticket_sg`) in Kerning City and
/// Shalon (`sellticket_cbd`) at Changi, with where each one's plane goes.
const PLANE_BOOTHS: [(i32, &Route, &str); 2] = [
    (9270041, &conti::TO_CHANGI, "Singapore"),
    (9270038, &conti::AIR_TO_KERNING_CITY, "Kerning City"),
];
/// The pilots: Xinga (`goback_kerning`) in Kerning City's departure room,
/// and Kerny (`goback_cbd`) in Changi's and on both planes.
const PILOTS: [i32; 2] = [9270017, 9270018];
/// Bell (`NLC_ticketing`) stands on both platforms and in both waiting
/// rooms.
const BELL: i32 = 9201057;
/// What Bell says about each line: her menu, its one choice, the ask, and
/// the way back out of the line's waiting room; and its Basic ticket, which
/// she sells to players of `BASIC_LEVEL` and under for `BASIC_FARE`
/// (Edelstein's `NLC_ticketing`; Cosmic sells only the Regular one).
struct SubwayLine {
    route: &'static Route,
    basic: i32,
    menu: &'static str,
    choice: &'static str,
    ask: &'static str,
    leave: &'static str,
}
const SUBWAY_LINES: [SubwayLine; 2] = [
    SubwayLine {
        route: &conti::TO_NLC,
        basic: 4031710,
        menu: keys::SUBWAY_TO_NLC_MENU,
        choice: keys::SUBWAY_TO_NLC_CHOICE,
        ask: keys::SUBWAY_TO_NLC_ASK,
        leave: keys::SUBWAY_TO_NLC_LEAVE,
    },
    SubwayLine {
        route: &conti::TO_KERNING_CITY,
        basic: 4031712,
        menu: keys::SUBWAY_TO_KERNING_CITY_MENU,
        choice: keys::SUBWAY_TO_KERNING_CITY_CHOICE,
        ask: keys::SUBWAY_TO_KERNING_CITY_ASK,
        leave: keys::SUBWAY_TO_KERNING_CITY_LEAVE,
    },
];
const BASIC_LEVEL: u8 = 10;
const BASIC_FARE: i32 = 1000;

impl SubwayLine {
    /// The ticket Bell sells a player of `level`, and its price.
    fn fare(&self, level: u8) -> (i32, i32) {
        if level <= BASIC_LEVEL {
            (self.basic, BASIC_FARE)
        } else {
            (self.route.ticket, self.route.price)
        }
    }

    /// The tickets its gate takes, the one taken first first (Edelstein
    /// `removeNLCTicket`).
    fn tickets(&self) -> [i32; 2] {
        [self.basic, self.route.ticket]
    }
}

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
/// Spinel of the Maple Travel Agency (`world_trip`): from a town to the
/// Mushroom Shrine in Japan, or from Boat Quay Town to Malaysia's Trend
/// Zone Metropolis, and from the shrine back where the tour began, else
/// to Lith Harbor (Cosmic's fares). Audrey, not Spinel, stands in Malaysia.
const SPINEL: i32 = 9000020;
const MUSHROOM_SHRINE: i32 = 800000000;
const WORLD_TOUR_FARE: i32 = 3000;
const BOAT_QUAY_TOWN: i32 = 541000000;
const TREND_ZONE_METROPOLIS: i32 = 550000000;
const MALAYSIA_FARE: i32 = 10000;
/// The guides to Florina Beach (`florina2`) and their fares: Pason in Lith
/// Harbor, Shuri in Orbis and Nara in Ludibrium (Cosmic's). The VIP ticket
/// takes its holder for free and is kept.
const FLORINA_GUIDES: [(i32, i32); 3] = [(1002002, 1500), (2010005, 2000), (2040048, 1800)];
const VIP_TICKET_TO_FLORINA: i32 = 4031134;
/// Florina Beach and its `st00` portal.
const FLORINA_BEACH: (i32, u8) = (110000000, 4);
/// Pison on Florina Beach (`florina1`), who takes players back where they
/// came from, else to Lith Harbor.
const PISON: i32 = 1081001;
/// Thomas Swift, free between Henesys and Amoria: to Amoria's portal 0, and
/// back to Henesys's portal 5, at the town's east end.
const THOMAS_SWIFT: i32 = 9201022;
const HENESYS: (i32, u8) = (100000000, 5);
const AMORIA: (i32, u8) = (680000000, 0);
/// Moony (`EngageRing`), Amoria's ring maker: once the player has their
/// parents' blessing, a groom trades a ring's stone, a Diamond and a fee for
/// an engagement ring box (report b064).
const MOONY: i32 = 9201000;
/// Mom and Dad (`MeetPare`), in Meet the Parents (680000004): they ask for
/// four of the six Proofs of Love and give their blessing.
const MOM_AND_DAD: i32 = 9201003;
/// The Nanas (`ProofHene` and so on): each gives her Proof of Love for 50 of
/// a town's monster drops while the blessing is sought. One script for the
/// six; the town decides which she is.
/// The Nanas, in the order of their towns in `NANA_TASKS`.
const NANAS: [i32; 6] = [9201001, 9201023, 9201027, 9201024, 9201025, 9201026];
/// By Nana: her town, the drop she asks for and the EXP she gives the first
/// time. Her Proof of Love is `PROOF_OF_LOVE` plus her place here (Cosmic).
const NANA_TASKS: [(i32, i32, i32); 6] = [
    (100000000, 4000001, 2000),
    (103000000, 4000037, 5000),
    (102000000, 4000215, 10000),
    (101000000, 4000026, 17000),
    (200000000, 4000070, 22000),
    (220000000, 4000128, 30000),
];
/// How many of her drop a Nana asks for.
const NANA_TASK_COUNT: i16 = 50;
const PROOF_OF_LOVE: i32 = 4031367;
/// How many different Proofs of Love Mom and Dad want.
const PROOFS_WANTED: usize = 4;
const BLESSING_EXP: i32 = 20000;
/// Moony's rings: the box, its stone and the fee; each also takes a
/// Diamond (Cosmic).
const RING_BOXES: [(i32, i32, i32); 4] = [
    (2240000, 4011007, 30000),
    (2240001, 4021009, 20000),
    (2240002, 4011006, 10000),
    (2240003, 4011004, 5000),
];
const DIAMOND: i32 = 4021007;
/// Audrey (`Malay_Warp2`), Malaysia's tour guide (Cosmic's fares): from the
/// Central Business District, where the plane lands, to Trend Zone
/// Metropolis; between Trend Zone and Kampung Village; and from Trend Zone
/// free back where the tour began, else Boat Quay Town. The destinations
/// are `(map, portal)`.
const AUDREY: i32 = 9201135;
const MALAYSIA_CBD: i32 = 540000000;
const KAMPUNG_VILLAGE: i32 = 551000000;
const CBD_FARE: i32 = 42000;
const CBD_TO_TREND_ZONE: (i32, u8) = (TREND_ZONE_METROPOLIS, 0);
const TO_KAMPUNG_VILLAGE: (i32, u8) = (KAMPUNG_VILLAGE, 2);
const BACK_TO_TREND_ZONE: (i32, u8) = (TREND_ZONE_METROPOLIS, 4);
/// The portal the way back arrives at, on either town a tour of Malaysia
/// begins in.
const MALAYSIA_HOME_PORTAL: u8 = 4;
/// Mar the Fairy, who pours the Water of Life on dolls.
const MAR_THE_FAIRY: i32 = 1032102;
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
/// Moony has sent the player for their parents' blessing.
const BLESSING_SOUGHT: i16 = 32300;
/// Mom and Dad have asked for the Proofs of Love.
const PROOFS_ASKED: i16 = 32301;
/// Mom and Dad have given their blessing; Moony makes rings.
const BLESSED: i16 = 32302;
/// A Nana's favour, by her place in `NANA_TASKS`: asked, and done once.
const NANA_ASKED: i16 = 32310;
const NANA_DONE: i16 = 32320;
/// A chief has approved the player for Zakum (upstream quest 100200).
pub(crate) const ZAKUM_APPROVED: i16 = 32400;
/// Adobis forged the player's Eyes of Fire: the trials are done (upstream
/// quest 100201).
pub(crate) const ZAKUM_TRIALS_DONE: i16 = 32401;
/// Yulete's traces have shown the player the way into his lab: the pipe
/// beyond Magatia's Alcadno side leads there (upstream quest 7770, which
/// Quest.nx lacks).
pub(crate) const YULETE_LAB_OPENED: i16 = 32600;
const DOOR_OF_DIMENSION: i32 = 1061009;
const LORD_JONATHAN: i32 = 1092019;
/// How long a Corsair has to find the real Bart (Cosmic's `4jaerial`).
const BART_TIME_LIMIT: u32 = 2 * 60;
const INSIGNIFICANT_BEING: i32 = 1061012;
const PAST_PAPULATUS: i32 = 2043000;
/// Shulynch in the Nautilus's Training Room, who sends a Corsair on
/// "Mission Free the Monsters" (6410) to look for Delli; Shulynch on
/// Looking for Delli 1, who offers the way back; and Delli.
const SHULYNCH: i32 = 1092008;
const SHULYNCH_ON_THE_WAY: i32 = 1095000;
pub const DELLI: i32 = 2095000;
const DELLI_QUEST: i16 = 6410;
const TRAINING_ROOM: i32 = 120_000_104;
const LOOKING_FOR_DELLI: i32 = 925_010_000;
/// Rolly in Ludibrium, who takes a party into the Ludibrium Maze, and
/// Rolly on its way out, who sends everyone back to Ludibrium.
pub const ROLLY: i32 = 9103001;
/// Pietri, in the Ludibrium Maze's goal room.
pub const PIETRI: i32 = 9103000;
const ROLLY_HOME: i32 = 9103003;
/// Rolly in the Ludibrium Maze's clear room, with the presents.
pub const ROLLY_PRESENT: i32 = 9103002;
/// Tylus in the Chief's Residence, who takes a party in to protect him on
/// "Protecting Tylus" (6192), and Tylus past the room, who thanks them.
pub const TYLUS: i32 = 2020008;
const TYLUS_GRATEFUL: i32 = 2022004;
/// Ellin in the Deep Fairy Forest, who takes a party into the Forest of
/// Poison Haze, and Ellin on each of its maps, who lets a member out; the
/// mileposts in its maze, who do too; and the Sprite on its fifth stage.
pub const ELLIN: i32 = 2133000;
pub const ELLIN_GUIDE: i32 = 2133001;
pub const ELLIN_MILEPOST: i32 = 2133002;
pub const ELLIN_SPRITE: i32 = 2133004;
/// Romeo and Juliet in their rooms in Magatia, who each take a party into
/// their side's copy of the party quest, and in its maps let a member out.
pub const ROMEO: i32 = 2112004;
pub const JULIET: i32 = 2112003;
/// Romeo and Juliet in their side's hub of files, past the beaker room.
pub const ROMEO_HUB: i32 = 2112006;
pub const JULIET_HUB: i32 = 2112005;
/// The books of each side's library, one of which holds the switch that
/// clears its first stage.
pub const ZENUMIST_BOOK: i32 = 2112007;
pub const ALCADNO_BOOK: i32 = 2112013;
/// Yulete, in each side's office until he leaves it.
pub const YULETE: i32 = 2112000;
pub const YULETE_ALCADNO: i32 = 2112010;
/// Yulete's traces, in each side's room of them after the win, and the
/// one who thanks the party in the last room.
pub const YULETE_TRACES: i32 = 2112001;
pub const YULETE_TRACES_ALCADNO: i32 = 2112011;
pub const MAGATIA_THANKS: i32 = 2112018;
/// Guon Over the Pirate Ship, who sends a party aboard against the Lord
/// Pirate, and Guon on each of the ship's maps and its way out.
pub const GUON: i32 = 2094000;
pub const GUON_ABOARD: i32 = 2094002;
/// So Gong, who takes a challenger into the Mu Lung Dojo from its hall and
/// stands on each of its floors, and the notice board in Mu Lung that
/// leads to the Dojo.
pub const SO_GONG: i32 = 2091005;
pub const DOJO_BOARD: i32 = 2091006;
/// Duarte, who takes players into Nett's Pyramid from its lobby and
/// stands on each of its stages, its result maps and the Shades.
pub const DUARTE: i32 = 2103013;
/// Spiegelmann, who sends players to the Monster Carnival's Carnival
/// Field from Kerning City, Ludibrium and Orbis, and there gives out its
/// rooms.
pub const SPIEGELMANN: i32 = 2042000;
/// The Event Assistants (script `Event00`), who take players to the event
/// a GM opened on their channel: Paul in Southperry and Magatia, Jean in
/// Lith Harbor, Singapore and Kampung, Martin in Orbis and Tony in
/// Ludibrium (report b170).
pub const EVENT_ASSISTANTS: [i32; 4] = [9000000, 9000001, 9000011, 9000013];
/// Pietro, where a GM's event sends its winners (109050000, `Event02`).
const PIETRO: i32 = 9000002;
/// What Pietro gives each winner: the Scroll of Secrets (report b172).
const SCROLL_OF_SECRETS: i32 = 4031019;
/// Pietra, where it sends the others (109050001, `Event06`).
const PIETRA: i32 = 9000010;
/// Harry, on the Coconut Season fields (`Event09`).
const HARRY: i32 = 9000012;
/// The Dimensional Mirror, in twenty towns, which sends players to the
/// Mu Lung Dojo, the Carnival Field and Nett's Pyramid (Cosmic `9010022`).
pub const DIMENSIONAL_MIRROR: i32 = 9010022;
/// Where the Mirror sets each down (Cosmic's portals).
const MIRROR_DOJO_PORTAL: u8 = 0;
const MIRROR_CARNIVAL_PORTAL: u8 = 3;
const MIRROR_PYRAMID_PORTAL: u8 = 4;
/// Spiegelmann again, in the Monster Carnival's exit room.
pub const CARNIVAL_GUIDE: i32 = 2042002;
/// Spiegelmann in a Monster Carnival waiting room, who puts challenges to
/// the party waiting there.
pub const CARNIVAL_ROOM_NPC: i32 = 2042001;
/// The waiting room's way back to the Carnival Field.
pub const CARNIVAL_ROOM_EXIT: i32 = 2042003;
/// Wu Yang, Guon's son, freed on the Pirate Ship's last stage and met
/// again in its clear room.
pub const WU_YANG: i32 = 2094001;
/// El Nath, and its portal from the Chief's Residence.
const EL_NATH: i32 = 211_000_000;
const EL_NATH_FROM_RESIDENCE: u8 = 14;
/// The Bowmen's 4th-job quests on the Junior Balrog in another world.
const BALROG_WORLD_QUESTS: [i16; 2] = [6107, 6108];
/// Bowmaster, Marksman and GM, as in Cosmic.
const BALROG_WORLD_JOBS: [i16; 3] = [312, 322, 900];
/// How long a Bowman has in the other world (Cosmic's `s4aWorld`).
const BALROG_WORLD_TIME_LIMIT: u32 = 20 * 60;
const SPARKLING_CRYSTAL: i32 = 1061010;
const HOLY_STONE: i32 = 2030006;
const SPIRUNA: i32 = 2032001;
/// Dropped by the clone, for the player sent to fight it.
const BLACK_CHARM: i32 = 4031059;
const NECKLACE_OF_STRENGTH: i32 = 4031057;
const NECKLACE_OF_WISDOM: i32 = 4031058;
const DARK_CRYSTAL: i32 = 4005004;
/// Spiruna refines for those who finished her quest (upstream).
const SPIRUNA_QUEST: i16 = 3034;
/// Questions asked of each challenger, any wrong answer failing.
const QUIZ_LENGTH: usize = 5;
/// How long a challenger has with the clone (upstream).
const CLONE_TIME_LIMIT: u32 = 20 * 60;

/// The player's answer to the current prompt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptInput {
    Next,
    Previous,
    Yes,
    No,
    Select(i32),
    Text(String),
    Number(i32),
}

/// What a script does with a line of text the player typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OnText {
    /// Lenario confirms the name for the new alliance.
    NameAlliance,
    /// The Secret Passage opens, `through` it, for Parwen's `password`.
    SecretPassage { password: String, through: Effect },
    /// The Puppeteer's door lets one who is `hunting` him through.
    PuppeteerDoor { hunting: bool },
    /// The Sealed Shrine's pillar lets one `hunting` its Shadow Knight in.
    ShrinePillar { hunting: bool },
}

impl OnText {
    /// The pages that follow the text.
    fn pages(&self, text: &str) -> Vec<Page> {
        match self {
            OnText::NameAlliance => confirm_alliance_name(text),
            OnText::SecretPassage { password, through } => {
                secret_passage_opens(text == password, through)
            }
            OnText::PuppeteerDoor { hunting } => puppeteer_door_opens(text, *hunting),
            OnText::ShrinePillar { hunting } => shrine_pillar_opens(text, *hunting),
        }
    }
}

/// What a script does with a number the player picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnNumber {
    /// Ask to make that many batches of `refine` at `percent` of its
    /// price.
    Refine {
        refine: &'static Refine,
        percent: i32,
    },
    /// Spiruna refines that many Dark Crystals at once.
    Spiruna,
    /// An Altaire camper trades that many hundred of `input` for Perfect
    /// Pitches.
    Pitches { input: i32 },
}

impl OnNumber {
    /// The pages that follow the number.
    fn pages(self, number: i32) -> Vec<Page> {
        match self {
            OnNumber::Refine { refine, percent } => refine_batches(refine, percent, number),
            OnNumber::Spiruna => vec![Page::Act {
                text: text(keys::SPIRUNA_DONE),
                effects: refining(&refiners::SPIRUNA_DARK_CRYSTAL, 100, number),
            }],
            OnNumber::Pitches { input } => vec![Page::Act {
                text: text(keys::ALTAIRE_DONE),
                effects: vec![
                    Effect::TakeItem {
                        item_id: input,
                        quantity: times(refiners::PER_PITCH, number),
                    },
                    Effect::GiveItem {
                        item_id: refiners::PERFECT_PITCH,
                        quantity: times(1, number),
                    },
                ],
            }],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Say(String),
    /// Said by the player in the NPC's conversation (speaker bit 0x2), as
    /// a quest's `Target.Speech` page.
    Reply(String),
    /// Yes applies `on_yes` and moves on; no ends the conversation, saying
    /// `on_no` first if there is one.
    Ask {
        text: String,
        on_yes: Vec<Effect>,
        on_no: Option<String>,
    },
    /// A yes or no whose answer's pages replace the rest of the script, as
    /// a menu choice's do.
    Decide {
        text: String,
        on_yes: Vec<Page>,
        on_no: Vec<Page>,
    },
    /// Each choice, a label and its pages, replaces the menu.
    Menu {
        text: String,
        choices: Vec<(String, Vec<Page>)>,
    },
    /// A line of text from `min` to `max` bytes long (`Prompt::AskText`);
    /// what `on_text` makes of it replaces the rest of the script.
    AskText {
        text: String,
        min: i16,
        max: i16,
        on_text: OnText,
    },
    /// A number from `min` to `max` (`Prompt::AskNumber`); what
    /// `on_number` makes of it replaces the rest of the script.
    AskNumber {
        text: String,
        default: i32,
        min: i32,
        max: i32,
        on_number: OnNumber,
    },
    /// A slide menu (`Prompt::Slide`) of art set `set`: each choice, an
    /// icon id, its label and its pages, replaces the menu. An id below 0
    /// is no choice: its label says there is nowhere to go.
    Slide {
        set: i32,
        choices: Vec<(i32, String, Vec<Page>)>,
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
    /// Its pages are the player's own thoughts, not the NPC's words.
    player_speaks: bool,
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
            player_speaks: false,
        }
    }

    /// One page the player thinks to themselves (speaker bit 0x2), as a
    /// portal's script has them do. The NPC goes on the wire, but the
    /// client shows the player.
    pub fn thought(text: String) -> Self {
        let mut script = Self::new(MAPLE_ADMINISTRATOR, vec![Page::Say(text)]);
        script.player_speaks = true;
        script
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

    /// Lenario's script as it plays for `player`.
    pub fn lenario(player: &Player, view: AllianceView) -> Self {
        lenario(player, view)
    }

    /// Lenario asks for another name for the alliance: the last one was
    /// taken.
    pub fn lenario_name_again() -> Self {
        Self::new(LENARIO, vec![name_alliance(true)])
    }

    /// Lenario says how founding, growing, or disbanding went.
    pub fn lenario_says(text: String) -> Self {
        Self::new(LENARIO, vec![Page::Say(text)])
    }

    /// `npc_id`'s script, if it is a wedding assistant in Amoria.
    pub fn wedding_assistant(npc_id: i32, player: &Player, view: WeddingView) -> Option<Self> {
        let venue = wedding_venue(npc_id)?;
        // In their lobbies they lead the guests in instead.
        (player.stats.map_id == AMORIA.0).then(|| wedding_assistant(venue, player, view))
    }

    /// The assistant of `venue` says how a booking went.
    pub fn wedding_assistant_says(venue: Venue, text: String) -> Self {
        Self::new(wedding_assistant_of(venue), vec![Page::Say(text)])
    }

    /// `npc_id`'s script, if it begins weddings in Amoria: for the couple
    /// whose booked wedding is due on `channel`, the question whether to
    /// begin it.
    pub fn wedding_starter(
        npc_id: i32,
        player: &Player,
        reservation: Option<&Reservation>,
        channel: u8,
        now: SystemTime,
    ) -> Option<Self> {
        let venue = wedding_starter_venue(npc_id)?;
        (player.stats.map_id == AMORIA.0)
            .then(|| wedding_starter(venue, player, reservation, channel, now))
    }

    /// The wedding starter of `venue` says why the wedding didn't begin.
    pub fn wedding_starter_says(venue: Venue, text: String) -> Self {
        Self::new(wedding_starter_of(venue), vec![Page::Say(text)])
    }

    /// The priest at `venue`'s altar, to one of the couple (`stage` their
    /// wedding's) or, without one, to anyone else.
    pub fn wedding_priest(venue: Venue, player: &Player, stage: Option<WeddingStage>) -> Self {
        wedding_priest(venue, player, stage)
    }

    /// The priest at `venue`'s altar says how the vows went.
    pub fn wedding_priest_says(venue: Venue, vows: Vows) -> Self {
        let text = match vows {
            Vows::Waiting => text(keys::WEDDING_PRIEST_WAITING),
            Vows::Married => venue_text(
                venue,
                keys::WEDDING_PRIEST_MARRIED,
                keys::WEDDING_CATHEDRAL_PRIEST_MARRIED,
            ),
            Vows::Refused => text(keys::WEDDING_PRIEST_FAILED),
        };
        Self::new(wedding_priest_of(venue), vec![Page::Say(text)])
    }

    /// `npc_id`, staff in a wedding's hall or at its altar, to someone in
    /// the wedding at `stage`: to a guest, the way out; to one of the
    /// couple, once married, the end of the wedding, or of a `premium` one
    /// the party room.
    pub fn wedding_staff(npc_id: i32, stage: WeddingStage, guest: bool, premium: bool) -> Self {
        let page = match stage {
            _ if guest => Page::Ask {
                text: text(keys::WEDDING_STAFF_GUEST_LEAVE),
                on_yes: vec![Effect::LeaveWedding],
                on_no: None,
            },
            WeddingStage::Married => Page::Ask {
                text: text(if premium {
                    keys::WEDDING_STAFF_PARTY_ASK
                } else {
                    keys::WEDDING_STAFF_ASK
                }),
                on_yes: vec![Effect::EndWedding],
                on_no: None,
            },
            _ => Page::Say(text(keys::WEDDING_STAFF_WAIT)),
        };
        Self::new(npc_id, vec![page])
    }

    /// `npc_id`'s script on `map_id` after a premium wedding, if one of its
    /// own there, to anyone in the wedding holding `keys` Heart Keys:
    /// Travis's way back to Amoria (report b080), and Robin's menu of the
    /// hunting ground, the box room and the exit (report b082).
    pub fn wedding_party(npc_id: i32, map_id: i32, held: i32) -> Option<Self> {
        let leave = |key, to| Page::Ask {
            text: text(key),
            on_yes: vec![Effect::Warp(to)],
            on_no: None,
        };
        let page = match (npc_id, map_id) {
            (TRAVIS, wedding::PARTY_ROOM) => leave(keys::WEDDING_PARTY_QUIT_ASK, AMORIA.0),
            (WEDDING_ROBIN, wedding::BOX_ROOM) => {
                leave(keys::WEDDING_ROBIN_BACK_ASK, wedding::HUNTING_GROUND)
            }
            (WEDDING_ROBIN, wedding::PARTY_ROOM | wedding::HUNTING_GROUND) => {
                let onward = if map_id == wedding::PARTY_ROOM {
                    let hunt = Page::Done(vec![Effect::Warp(wedding::HUNTING_GROUND)]);
                    (text(keys::WEDDING_ROBIN_HUNT), vec![hunt])
                } else if held >= i32::from(wedding::BOX_KEYS) {
                    let boxes = Page::Done(vec![
                        Effect::TakeItem {
                            item_id: wedding::HEART_KEY,
                            quantity: wedding::BOX_KEYS,
                        },
                        Effect::Warp(wedding::BOX_ROOM),
                    ]);
                    (text(keys::WEDDING_ROBIN_BOXES), vec![boxes])
                } else {
                    let short = Page::End(text(keys::WEDDING_ROBIN_NO_KEYS));
                    (text(keys::WEDDING_ROBIN_BOXES), vec![short])
                };
                let out = leave(keys::WEDDING_PARTY_EXIT_ASK, wedding::WEDDING_EXIT);
                Page::Menu {
                    text: text(keys::WEDDING_ROBIN_MENU),
                    choices: vec![onward, (text(keys::WEDDING_ROBIN_OUT), vec![out])],
                }
            }
            _ => return None,
        };
        let mut script = Self::new(npc_id, vec![page]);
        if map_id == wedding::HUNTING_GROUND {
            // The keys may have gone since the menu was drawn.
            script.refused = Some(text(keys::WEDDING_ROBIN_NO_KEYS));
        }
        Some(script)
    }

    /// `npc_id`'s script on `map_id`, if it takes the wishlists of a
    /// wedding at its venue there (Travis, Debbie; report b085), to someone
    /// in the wedding: to one of the couple who has `wished` or not, or to
    /// a guest.
    pub fn wedding_greeter(npc_id: i32, map_id: i32, couple: bool, wished: bool) -> Option<Self> {
        let venue = match npc_id {
            TRAVIS => Venue::Chapel,
            DEBBIE => Venue::Cathedral,
            _ => return None,
        };
        if map_id != venue.hall() && map_id != venue.altar() {
            return None;
        }
        let page = match (couple, wished) {
            (true, false) => Page::Ask {
                text: text(keys::WEDDING_WISHLIST_ASK),
                on_yes: vec![Effect::OpenWishlist],
                on_no: None,
            },
            (true, true) => Page::Say(text(keys::WEDDING_WISHLIST_SENT)),
            (false, _) => Page::Say(text(keys::WEDDING_WISHLIST_GUEST)),
        };
        Some(Self::new(npc_id, vec![page]))
    }

    /// Pila's script: in a wedding, the choice of whose wishlist to give
    /// from; elsewhere, to someone with `gifts` waiting, whether to take
    /// them.
    pub fn pila(in_wedding: bool, gifts: bool) -> Self {
        let registry = |bride| vec![Page::Done(vec![Effect::OpenRegistry { bride }])];
        let page = if in_wedding {
            Page::Menu {
                text: text(keys::WEDDING_PILA_MENU),
                choices: vec![
                    (text(keys::WEDDING_PILA_GROOM), registry(false)),
                    (text(keys::WEDDING_PILA_BRIDE), registry(true)),
                ],
            }
        } else if gifts {
            Page::Ask {
                text: text(keys::WEDDING_PILA_GIFTS),
                on_yes: vec![Effect::OpenGifts],
                on_no: None,
            }
        } else {
            Page::Say(text(keys::WEDDING_PILA_HELLO))
        };
        Self::new(PILA, vec![page])
    }

    /// Pila says why the give window didn't open.
    pub fn pila_says(key: &str) -> Self {
        Self::new(PILA, vec![Page::Say(text(key))])
    }

    /// Lakelis's script, for a player whose party may or may not go into
    /// the party quest.
    pub fn lakelis(entry: Entry) -> Self {
        lakelis(entry)
    }

    /// Tory's script in Henesys Park, for a player whose party may or may
    /// not go up to Primrose Hill.
    pub fn tory(entry: Entry) -> Self {
        tory(entry)
    }

    /// The Red Sign's script on Eos Tower's 101st floor, for a player whose
    /// party may or may not go into the Dimensional Crack.
    pub fn red_sign(entry: Entry) -> Self {
        red_sign(entry)
    }

    /// Wonky's script in Orbis, for a player whose party may or may not go
    /// into the Tower of Goddess.
    pub fn wonky(entry: Entry) -> Self {
        wonky(entry)
    }

    /// Adobis at the Door to Zakum, for a player whose party may or may not
    /// go into the Dead Mine.
    pub fn adobis(entry: Entry, player: &Player) -> Self {
        adobis(entry, player)
    }

    /// The NPC who gathers `kind`'s expeditions (Adobis at the Entrance to
    /// Zakum, the Mark of the Squad at the Entrance to Horntail's Cave), to
    /// `player`, who finds the channel's as `view`. None for a kind no
    /// squad goes to.
    pub fn squad(kind: EventKind, view: SquadView, player: &Player) -> Option<Self> {
        squad(kind, view, player)
    }

    /// Shuang (9040000) at the Excavation Site's camp, to a member of rank
    /// `rank` in their guild (none without one) who finds the channel's
    /// Guild Quest line as `view` (Cosmic `9040000`): the master or a Jr.
    /// Master puts the guild in line, any member goes into its waiting room
    /// while it gathers, and anyone may hear what the quest is.
    pub fn shuang(rank: Option<u8>, view: guild_quest::View) -> Self {
        let register = match rank {
            None => Page::Say(text(keys::GUILD_QUEST_NO_GUILD)),
            Some(rank) if !guild_quest::REGISTRANT_RANKS.contains(&rank) => {
                Page::Say(text(keys::GUILD_QUEST_NOT_MASTER))
            }
            Some(_) if view.listed => Page::Say(text(keys::GUILD_QUEST_LISTED)),
            Some(_) if view.full => Page::Say(text(keys::GUILD_QUEST_FULL)),
            Some(_) => Page::Decide {
                text: content::text().get(
                    keys::GUILD_QUEST_ASK,
                    &[("waiting", &view.waiting.to_string())],
                ),
                on_yes: vec![Page::Act {
                    text: text(keys::GUILD_QUEST_REGISTERED),
                    effects: vec![Effect::GuildQuest(guild_quest::Command::Register)],
                }],
                on_no: vec![],
            },
        };
        let join = match rank {
            None => Page::Say(text(keys::GUILD_QUEST_NO_GUILD)),
            Some(_) if !view.gathering => Page::Say(text(keys::GUILD_QUEST_NOT_CALLED)),
            Some(_) => Page::Done(vec![Effect::GuildQuest(guild_quest::Command::Join)]),
        };
        let mut script = Self::new(
            guild_quest::SHUANG,
            vec![Page::Menu {
                text: text(keys::GUILD_QUEST_MENU),
                choices: vec![
                    (text(keys::GUILD_QUEST_REGISTER_CHOICE), vec![register]),
                    (text(keys::GUILD_QUEST_JOIN_CHOICE), vec![join]),
                    (
                        text(keys::GUILD_QUEST_DETAILS_CHOICE),
                        vec![Page::Say(text(keys::GUILD_QUEST_DETAILS))],
                    ),
                ],
            }],
        );
        // Another guild took the last place, or the guild's turn passed.
        script.refused = Some(text(keys::GUILD_QUEST_REFUSED));
        script
    }

    /// The gatekeeper (2083001) at the Cave Entrance, for a player whose
    /// party may or may not go into the Trial Grounds.
    pub fn htpq_gatekeeper(entry: Entry) -> Self {
        htpq_gatekeeper(entry)
    }

    /// Delli on Looking for Delli 3, for a player whose party may or may
    /// not go in to protect Willi.
    pub fn delli(entry: Entry) -> Self {
        delli(entry)
    }

    /// Tylus in the Chief's Residence, to a warrior on "Protecting Tylus"
    /// whose party may or may not go in to protect him.
    pub fn tylus(entry: Entry) -> Self {
        tylus(entry)
    }

    /// Ellin in the Deep Fairy Forest, for `player`, whose party may or
    /// may not go into the Forest of Poison Haze.
    pub fn ellin(entry: Entry, player: &Player) -> Self {
        ellin(entry, player)
    }

    /// Romeo or Juliet in their room, for a player whose party may or may
    /// not go into `side`'s party quest.
    pub fn magatia(side: MagatiaSide, entry: Entry) -> Self {
        magatia(side, entry)
    }

    /// One of a Magatia library's books, `npc_id`, found to hold `find`,
    /// or nothing (any longer).
    pub fn magatia_book(npc_id: i32, find: Option<Find>) -> Self {
        magatia_book(npc_id, find)
    }

    /// Yulete, `npc_id`, in his office, saying the line his field chose
    /// (`events::yulete_talk`), as text key.
    pub fn yulete(npc_id: i32, key: &str) -> Self {
        NpcScript::new(npc_id, vec![Page::Say(text(key))])
    }

    /// Guon Over the Pirate Ship, for a player whose party may or may not
    /// board it.
    pub fn guon(entry: Entry) -> Self {
        guon(entry)
    }

    /// So Gong in the Dojo's hall, for `player`, who may or may not go in.
    pub fn so_gong(entry: Entry, party: Entry, player: &Player) -> Self {
        so_gong(entry, party, player)
    }

    /// Duarte in Nett's Pyramid's lobby, for `player`, whose `entries` say
    /// which runs they may go on.
    pub fn duarte(entries: &[(EventKind, Entry)], player: &Player) -> Self {
        duarte(entries, player)
    }

    /// An Event Assistant (`EVENT_ASSISTANTS`), while a GM's event is
    /// `open` on the player's channel or not.
    pub fn event_assistant(npc_id: i32, open: bool) -> Self {
        let go = match open {
            true => Page::Done(vec![Effect::JoinGmEvent { npc: npc_id }]),
            false => Page::Say(text(keys::GM_EVENT_NOT_OPEN)),
        };
        NpcScript::new(
            npc_id,
            vec![Page::Menu {
                text: text(keys::GM_EVENT_MENU),
                choices: vec![
                    (
                        text(keys::GM_EVENT_ABOUT),
                        vec![Page::Say(text(keys::GM_EVENT_INFO))],
                    ),
                    (text(keys::GM_EVENT_GO), vec![go]),
                ],
            }],
        )
    }

    /// An Event Assistant on a player who couldn't join after all.
    pub fn event_assistant_refused(npc_id: i32) -> Self {
        NpcScript::new(npc_id, vec![Page::Say(text(keys::GM_EVENT_NOT_OPEN))])
    }

    /// Spiegelmann on the Carnival Field, for a player whose party may or
    /// may not take each of the Monster Carnival's rooms (`entries`).
    pub fn spiegelmann(
        entries: &[(EventKind, Entry)],
        rooms: &[carnival::Room],
        player: &Player,
    ) -> Self {
        spiegelmann(entries, rooms, player)
    }

    /// Spiegelmann on a pick of Monster Carnival room `room` that didn't
    /// take it: the challenge was put to the party waiting there (`None`),
    /// or why not.
    pub fn spiegelmann_picked(room: u8, refused: Option<carnival::Refusal>) -> Self {
        use carnival::Refusal;
        let text = match refused {
            None => text(keys::CARNIVAL_SPIEGELMANN_SENT),
            Some(Refusal::NoParty) => text(keys::CARNIVAL_SPIEGELMANN_NO_PARTY),
            Some(Refusal::NotLeader) => text(keys::CARNIVAL_SPIEGELMANN_NOT_LEADER),
            Some(Refusal::NotEligible) => event_terms(
                EventKind::Carnival(room).event(),
                keys::CARNIVAL_SPIEGELMANN_TERMS,
            ),
            Some(Refusal::Unequal) => text(keys::CARNIVAL_SPIEGELMANN_UNEQUAL),
            Some(Refusal::Facing) => text(keys::CARNIVAL_SPIEGELMANN_FACING),
            Some(Refusal::Full) => text(keys::CARNIVAL_SPIEGELMANN_FULL),
        };
        NpcScript::new(SPIEGELMANN, vec![Page::Say(text)])
    }

    /// `challenger`'s party challenges the one the player leads in a
    /// Monster Carnival waiting room: will they fight (Cosmic
    /// `cpqchallenge`)?
    pub fn carnival_challenge(
        challenger: crate::ClientId,
        challengers: &[carnival::Challenger],
    ) -> Self {
        let party: Vec<_> = challengers
            .iter()
            .map(|c| {
                let level = c.level.to_string();
                let params = [("name", c.name.as_str()), ("level", level.as_str())];
                content::text().get(keys::CARNIVAL_CHALLENGE_MEMBER, &params)
            })
            .collect();
        let party = party.join("\r\n");
        let answer = |accept| {
            let effect = Effect::CarnivalAnswer { challenger, accept };
            vec![Page::Done(vec![effect])]
        };
        NpcScript::new(
            CARNIVAL_ROOM_NPC,
            vec![Page::Decide {
                text: content::text().get(keys::CARNIVAL_CHALLENGE_ASK, &[("party", &party)]),
                on_yes: answer(true),
                on_no: answer(false),
            }],
        )
    }

    /// Pietri in the maze's goal room, to a member who may be its
    /// `leader`, whose team may have `won` it and may all be there
    /// (`together`).
    pub fn pietri(leader: bool, won: bool, together: bool, player: &Player) -> Self {
        pietri(leader, won, together, player)
    }

    /// Rolly in Ludibrium, for a player whose party may or may not go into
    /// the Ludibrium Maze.
    pub fn rolly(entry: Entry) -> Self {
        rolly(entry)
    }

    /// Aura on the Dead Mine's hub, to `player`, who stands with her as
    /// `view`.
    pub fn aura(view: Aura, player: &Player) -> Self {
        aura(view, player)
    }

    /// The balloon that clears `hand_in`'s stage, to `player`, who stands
    /// with it as `view`.
    pub fn balloon(hand_in: &HandIn, view: Balloon, player: &Player) -> Self {
        balloon(hand_in, view, player)
    }

    /// The Blue Balloon on Ludibrium's puzzle stage `stage`, as the player
    /// stands with it as `view`; a leader answering it is told how the team
    /// `stood`, and clears the stage when right.
    pub fn blue_balloon(stage: u8, view: Balloon, stood: Option<Standing>) -> Self {
        let page = match (view, stood) {
            (Balloon::Cleared, _) => Page::Say(text(keys::LPQ_BALLOON_HURRY)),
            (Balloon::NotLeader, _) => Page::Say(text(keys::LPQ_BALLOON_NOT_LEADER)),
            (Balloon::Intro, _) => Page::Say(text(keys::LPQ_BALLOON_INTRO_8)),
            (Balloon::Answer { .. }, Some(Standing::Right)) => Page::Act {
                text: text(keys::LPQ_BALLOON_DONE),
                effects: vec![Effect::EventCommand(EventCommand::ClearStage(stage))],
            },
            (Balloon::Answer { .. }, Some(Standing::Wrong)) => {
                Page::Say(text(keys::LPQ_BALLOON_WRONG))
            }
            _ => Page::Say(text(keys::LPQ_BALLOON_FEW)),
        };
        NpcScript::new(BLUE_BALLOON, vec![page])
    }

    /// The Aqua Balloon on Ludibrium's maze stage, which clears nothing.
    pub fn aqua_balloon() -> Self {
        NpcScript::new(AQUA_BALLOON, vec![Page::Say(text(keys::LPQ_BALLOON_S6))])
    }

    /// A script on an event's maps, for a player in the event, who may be
    /// its `leader`, and whose party may have `won` it: Growlie's, Tommy's
    /// or Tory's on Primrose Hill, the Pink Balloon's or Arturo's in
    /// Ludibrium, Minerva's in the Tower of Goddess, Rolly's in the
    /// Ludibrium Maze's clear room.
    pub fn in_event(npc_id: i32, player: &Player, leader: bool, won: bool) -> Option<Self> {
        match npc_id {
            GROWLIE => growlie(player, leader, won),
            TOMMY => tommy(player, leader, won),
            TORY => tory_after(player, won),
            PINK_BALLOON => pink_balloon(player),
            ARTURO if won => arturo(player, &mut rand::thread_rng()),
            MINERVA if won => minerva(player, &mut rand::thread_rng()),
            ROLLY_PRESENT if won => maze_present(player, &mut rand::thread_rng()),
            HTPQ_KEEPER if !won => htpq_keeper(player, leader),
            ELLIN_SPRITE if player.stats.map_id == events::ELLIN_STAGES[4] => {
                Some(ellin_sprite(player, leader))
            }
            SO_GONG if events::dojo_floor(player.stats.map_id).is_some() => {
                Some(so_gong_on_floor(player, leader))
            }
            DUARTE if events::pyramid_stage(player.stats.map_id).is_some() => {
                Some(duarte_forfeit())
            }
            WU_YANG if !won && player.stats.map_id == events::PIRATE_STAGES[5] => {
                Some(wu_yang_freed(leader))
            }
            WU_YANG if won && player.stats.map_id == events::PIRATE_CLEAR => {
                Some(wu_yang_clear(player))
            }
            _ => None,
        }
    }

    /// Guon aboard the Pirate Ship (Cosmic `2094002`), to `player`, who
    /// may be its `leader`, whose team has handed him `marks` Marks, on a
    /// map that may be `bare` of monsters.
    pub fn guon_aboard(player: &Player, leader: bool, marks: u8, bare: bool) -> Self {
        guon_aboard(player, leader, marks, bare)
    }

    /// Romeo or Juliet in `side`'s hub of files, to `player`, who may be
    /// the team's `leader`, once the hub's stage is `cleared` or not, and
    /// with its way on shut (`door_shut`) or open.
    pub fn magatia_hub(
        side: MagatiaSide,
        player: &Player,
        leader: bool,
        cleared: bool,
        door_shut: bool,
    ) -> Self {
        magatia_hub(side, player, leader, cleared, door_shut)
    }

    /// Romeo or Juliet in `side`'s boss's room once it is won, by whether
    /// the escort fainted and whether Yulete was `caught`, which made his
    /// Frankenroid the mighty one.
    pub fn magatia_saved(side: MagatiaSide, escort_failed: bool, caught: bool) -> Self {
        magatia_saved(side, escort_failed, caught)
    }

    /// Yulete's traces in `side`'s room of them.
    pub fn yulete_traces(side: MagatiaSide) -> Self {
        yulete_traces(side)
    }

    /// The one who thanks the party in `side`'s last room, to `player`,
    /// by whether the escort fainted.
    pub fn magatia_thanks(
        side: MagatiaSide,
        player: &Player,
        escort_failed: bool,
        rng: &mut impl Rng,
    ) -> Self {
        magatia_thanks(side, player, escort_failed, rng)
    }

    /// Eak (Cosmic `2013001`) to a player in the Orbis party quest on
    /// `map_id`, who may be its `leader`, whose team may have `cleared` the
    /// cloud stage. None elsewhere.
    pub fn opq_eak(map_id: i32, leader: bool, cleared: bool) -> Option<Self> {
        opq_eak(map_id, leader, cleared)
    }

    /// Ellin on the forest's fourth stage (Cosmic `2133001`), to `player`,
    /// who `leads` the team or not and has `heard` her out there or not,
    /// with `flowers` Poison Flowers left (see `ellin_flowers`).
    pub fn ellin_flowers(leads: bool, heard: bool, flowers: usize, player: &Player) -> Self {
        ellin_flowers(leads, heard, flowers, player)
    }

    /// Eak in the Tower of Goddess's storage room, once it is `bare` of
    /// monsters and drops or was `cleared`.
    pub fn opq_salion(bare: bool, cleared: bool) -> Self {
        opq_salion(bare, cleared)
    }

    /// Eak in the Tower of Goddess's music room, once the music box
    /// `played` the day's record or the room was `cleared`.
    pub fn opq_music(played: bool, cleared: bool) -> Self {
        opq_music(played, cleared)
    }

    /// Eak in the Tower of Goddess's sealed room, given how the team
    /// stands on its platforms; `None` once the room is cleared.
    pub fn opq_head_count(count: Option<HeadCount>) -> Self {
        opq_head_count(count)
    }

    /// Eak in the Tower of Goddess's tall room, given how its levers
    /// stand; `None` once the room is cleared.
    pub fn opq_levers(check: Option<LeverCheck>) -> Self {
        opq_levers(check)
    }

    /// Eak to the leader on the Tower of Goddess's hub, given how
    /// Minerva's statue stands.
    pub fn opq_hub(statue: Statue) -> Self {
        opq_hub(statue)
    }

    /// Eak, still only a light, telling a player who has just come into
    /// the tower what to bring him (Cosmic `OrbisPQ` `playerEntry`).
    pub fn opq_arrival() -> Self {
        NpcScript::new(EAK, vec![Page::Say(text(keys::OPQ_EAK_ARRIVAL))])
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
        if npc_id == KYRIN {
            if let Some(script) = kyrin_fourth_job_test(player) {
                return Some(script);
            }
        }
        if npc_id == DOJO_BOARD {
            return Some(dojo_board(player));
        }
        if npc_id == OLSON && player.quests.state(events::DOLLHOUSE_QUEST) == QuestState::Started {
            return Some(olson());
        }
        if npc_id == MARK {
            return Some(mark(player));
        }
        if npc_id == JUNE {
            return Some(june(player));
        }
        if npc_id == CHENG {
            return cheng(player, &mut rand::thread_rng());
        }
        if npc_id == DIMENSIONAL_MIRROR {
            return Some(dimensional_mirror(player));
        }
        if npc_id == SPIEGELMANN && player.stats.map_id != carnival::CARNIVAL_LOBBY {
            return Some(spiegelmann_away(player));
        }
        if npc_id == guild_quest::NURIS && player.stats.map_id == guild_quest::RETURNING_PATH {
            let camp = guild_quest::CAMP;
            return Some(ask_warp(
                guild_quest::NURIS,
                keys::GUILD_QUEST_NURIS_ASK,
                camp,
            ));
        }
        if npc_id == CARNIVAL_GUIDE && player.stats.map_id == carnival::CARNIVAL_EXIT {
            let back = carnival::CARNIVAL_LOBBY;
            return Some(ask_warp(CARNIVAL_GUIDE, keys::CARNIVAL_GUIDE_BACK, back));
        }
        if npc_id == CARNIVAL_GUIDE && carnival::result_room(player.stats.map_id).is_some() {
            return Some(carnival_grade(player));
        }
        if carnival::waiting_room(player.stats.map_id).is_some() {
            // The leader's leaving gives the room up (`EventActor`).
            let back = carnival::CARNIVAL_LOBBY;
            match npc_id {
                CARNIVAL_ROOM_NPC => {
                    let page = Page::Say(text(keys::CARNIVAL_ROOM_WAIT));
                    return Some(NpcScript::new(CARNIVAL_ROOM_NPC, vec![page]));
                }
                CARNIVAL_ROOM_EXIT => {
                    return Some(ask_warp(
                        CARNIVAL_ROOM_EXIT,
                        keys::CARNIVAL_ROOM_LEAVE,
                        back,
                    ));
                }
                _ => {}
            }
        }
        if let Some(script) = gm_event_exit(npc_id, player) {
            return Some(script);
        }
        if npc_id == DUARTE {
            if let Some(script) = duarte_after(player) {
                return Some(script);
            }
        }
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
        if let Some(&(_, key)) = ONE_LINERS.iter().find(|(id, _)| *id == npc_id) {
            return Some(NpcScript::new(npc_id, vec![Page::Say(text(key))]));
        }
        match npc_id {
            ALI => return Some(ali()),
            LIRA => return Some(lira()),
            AMON => return Some(amon(player)),
            TOMBSTONE => {
                return Some(NpcScript::new(
                    TOMBSTONE,
                    vec![Page::Say(text(keys::PAPULATUS_TOMBSTONE))],
                ))
            }
            MACHINE_APPARATUS => return Some(machine_apparatus()),
            PARWEN if player.quests.state(WHAT_PARWEN_KNOWS) != QuestState::NotStarted => {
                return Some(parwen())
            }
            SECRET_PASSAGE => return secret_passage(player),
            KEROBEN => return Some(keroben(player)),
            SQUAD_SLATE => return Some(squad_slate(player)),
            CRYSTAL_OF_ROOTS => return Some(crystal_of_roots(player)),
            JAKE => return Some(jake(player)),
            LORD_JONATHAN => return Some(lord_jonathan(player)),
            INSIGNIFICANT_BEING => return Some(insignificant_being(player)),
            PAST_PAPULATUS => return Some(past_papulatus(player)),
            SHULYNCH if player.quests.state(DELLI_QUEST) == QuestState::Started => {
                return Some(ask_warp(
                    SHULYNCH,
                    keys::DELLI_SHULYNCH_ASK,
                    LOOKING_FOR_DELLI,
                ))
            }
            SHULYNCH_ON_THE_WAY => {
                return Some(ask_warp(
                    SHULYNCH_ON_THE_WAY,
                    keys::DELLI_SHULYNCH_LEAVE,
                    TRAINING_ROOM,
                ))
            }
            DELLI if player.stats.map_id == events::NAUTILUS_HIDEOUT => {
                return Some(ask_warp(DELLI, keys::DELLI_HOME_ASK, TRAINING_ROOM))
            }
            TYLUS_GRATEFUL if player.stats.map_id == events::TYLUS_THANKS => {
                return Some(NpcScript::new(
                    TYLUS_GRATEFUL,
                    vec![
                        Page::Say(text(keys::TYLUS_THANKS)),
                        Page::Done(vec![Effect::WarpToPortal(EL_NATH, EL_NATH_FROM_RESIDENCE)]),
                    ],
                ))
            }
            // She welcomes the party at the entrance, takes it on from the
            // maze's end, and has the fourth stage's marbles (see
            // `ellin_flowers`); elsewhere she lets a member out.
            ELLIN_GUIDE if player.stats.map_id == events::ELLIN_ENTRY => {
                return Some(NpcScript::new(
                    ELLIN_GUIDE,
                    vec![Page::Say(text(keys::ELLIN_GUIDE_WELCOME))],
                ))
            }
            ELLIN_GUIDE if player.stats.map_id == events::ELLIN_STAGES[2] => {
                return Some(ellin_maze_end())
            }
            ELLIN_GUIDE
                if events::ELLIN.holds(player.stats.map_id)
                    && player.stats.map_id != events::ELLIN_STAGES[3] =>
            {
                return Some(ask_warp(
                    ELLIN_GUIDE,
                    keys::ELLIN_GUIDE_LEAVE,
                    events::ELLIN.exit,
                ))
            }
            ELLIN_MILEPOST if events::ELLIN.holds(player.stats.map_id) => {
                return Some(ask_warp(
                    ELLIN_MILEPOST,
                    keys::ELLIN_MILEPOST_LEAVE,
                    events::ELLIN.exit,
                ))
            }
            // Inside, either asks whether the member would rather leave.
            ROMEO | JULIET => {
                let side = magatia_side(npc_id);
                let event = EventKind::Magatia(side).event();
                if event.holds(player.stats.map_id) {
                    let retreat = magatia_keys(side).retreat;
                    return Some(ask_warp(npc_id, retreat, event.exit));
                }
            }
            GUON_ABOARD if player.stats.map_id == events::PIRATE_EXIT => {
                return Some(NpcScript::new(
                    GUON_ABOARD,
                    vec![Page::Done(vec![Effect::WarpToPortal(
                        events::PIRATE_LOBBY,
                        0,
                    )])],
                ))
            }
            ROLLY_HOME if player.stats.map_id == events::MAZE_EXIT => {
                return Some(NpcScript::new(
                    ROLLY_HOME,
                    vec![
                        Page::Say(text(keys::LUDIMAZE_HOME)),
                        Page::Done(vec![Effect::WarpToPortal(events::LUDIBRIUM, 0)]),
                    ],
                ))
            }
            SITE_EXIT => return Some(site_exit()),
            SERRYL => {
                return Some(item_maker(
                    SERRYL,
                    keys::MAKER_SERRYL_HELLO,
                    &[
                        (keys::MAKER_SERRYL_KNUCKLERS, item_makers::SERRYL_KNUCKLER),
                        (keys::MAKER_SERRYL_GUNS, item_makers::SERRYL_GUN),
                        (keys::MAKER_SERRYL_GLOVES, item_makers::SERRYL_GLOVES),
                    ],
                ))
            }
            NEVE => {
                return Some(item_maker(
                    NEVE,
                    keys::MAKER_NEVE_HELLO,
                    &[
                        (keys::MAKER_NEVE_WARRIOR, item_makers::NEVE_WARRIOR),
                        (keys::MAKER_NEVE_BOWMAN, item_makers::NEVE_BOWMAN),
                        (keys::MAKER_NEVE_MAGICIAN, item_makers::NEVE_MAGICIAN),
                        (keys::MAKER_NEVE_THIEF, item_makers::NEVE_THIEF),
                    ],
                ))
            }
            MOS => return Some(mos(player, &mut rand::thread_rng())),
            FRANCOIS => {
                return Some(item_maker(
                    FRANCOIS,
                    keys::MAKER_FRANCOIS_HELLO,
                    &[
                        (keys::REFINER_MAKE_GLOVE, item_makers::FRANCOIS_GLOVES),
                        (
                            keys::REFINER_UPGRADE_GLOVE,
                            item_makers::FRANCOIS_GLOVE_UPGRADES,
                        ),
                        (keys::MAKER_UPGRADE_HAT, item_makers::FRANCOIS_HATS),
                        (keys::MAKER_MAKE_WAND, item_makers::FRANCOIS_WANDS),
                        (keys::MAKER_MAKE_STAFF, item_makers::FRANCOIS_STAFFS),
                    ],
                ))
            }
            CHRISHRAMA => {
                return Some(item_maker(
                    CHRISHRAMA,
                    keys::MAKER_CHRISHRAMA_HELLO,
                    &[
                        (keys::MAKER_SHOES_WARRIOR, item_makers::CHRISHRAMA_WARRIOR),
                        (keys::MAKER_SHOES_BOWMAN, item_makers::CHRISHRAMA_BOWMAN),
                        (keys::MAKER_SHOES_MAGICIAN, item_makers::CHRISHRAMA_MAGICIAN),
                        (keys::MAKER_SHOES_THIEF, item_makers::CHRISHRAMA_THIEF),
                    ],
                ))
            }
            GORDON => {
                return Some(item_maker(
                    GORDON,
                    keys::MAKER_GORDON_HELLO,
                    &[
                        (keys::MAKER_SHOES_WARRIOR, item_makers::GORDON_WARRIOR),
                        (keys::MAKER_SHOES_BOWMAN, item_makers::GORDON_BOWMAN),
                        (keys::MAKER_SHOES_MAGICIAN, item_makers::GORDON_MAGICIAN),
                        (keys::MAKER_SHOES_THIEF, item_makers::GORDON_THIEF),
                    ],
                ))
            }
            SARAH => {
                return Some(stimulating_maker(
                    SARAH,
                    keys::MAKER_SARAH_HELLO,
                    &[
                        (keys::MAKER_GLOVES_WARRIOR, item_makers::SARAH_WARRIOR),
                        (keys::MAKER_GLOVES_BOWMAN, item_makers::SARAH_BOWMAN),
                        (keys::MAKER_GLOVES_MAGICIAN, item_makers::SARAH_MAGICIAN),
                        (keys::MAKER_GLOVES_THIEF, item_makers::SARAH_THIEF),
                    ],
                    &[
                        (
                            keys::MAKER_GLOVES_WARRIOR_UPGRADED,
                            item_makers::SARAH_WARRIOR_UPGRADES,
                        ),
                        (
                            keys::MAKER_GLOVES_BOWMAN_UPGRADED,
                            item_makers::SARAH_BOWMAN_UPGRADES,
                        ),
                        (
                            keys::MAKER_GLOVES_MAGICIAN_UPGRADED,
                            item_makers::SARAH_MAGICIAN_UPGRADES,
                        ),
                        (
                            keys::MAKER_GLOVES_THIEF_UPGRADED,
                            item_makers::SARAH_THIEF_UPGRADES,
                        ),
                    ],
                    &mut rand::thread_rng(),
                ))
            }
            TARA => {
                return Some(stimulating_maker(
                    TARA,
                    keys::MAKER_TARA_HELLO,
                    &[
                        (keys::MAKER_SHOES_WARRIOR, item_makers::TARA_WARRIOR),
                        (keys::MAKER_SHOES_BOWMAN, item_makers::TARA_BOWMAN),
                        (keys::MAKER_SHOES_MAGICIAN, item_makers::TARA_MAGICIAN),
                        (keys::MAKER_SHOES_THIEF, item_makers::TARA_THIEF),
                    ],
                    &[
                        (
                            keys::MAKER_SHOES_WARRIOR_STIMULATED,
                            item_makers::TARA_WARRIOR,
                        ),
                        (
                            keys::MAKER_SHOES_BOWMAN_STIMULATED,
                            item_makers::TARA_BOWMAN,
                        ),
                        (
                            keys::MAKER_SHOES_MAGICIAN_STIMULATED,
                            item_makers::TARA_MAGICIAN,
                        ),
                        (keys::MAKER_SHOES_THIEF_STIMULATED, item_makers::TARA_THIEF),
                    ],
                    &mut rand::thread_rng(),
                ))
            }
            RYDOLE => {
                return Some(stimulating_maker(
                    RYDOLE,
                    keys::MAKER_RYDOLE_HELLO,
                    &[
                        (keys::MAKER_MOS_WARRIOR, item_makers::RYDOLE_WARRIOR),
                        (keys::MAKER_MOS_BOWMAN, item_makers::RYDOLE_BOWMAN),
                        (keys::MAKER_MOS_MAGICIAN, item_makers::RYDOLE_MAGICIAN),
                        (keys::MAKER_MOS_THIEF, item_makers::RYDOLE_THIEF),
                    ],
                    &[
                        (
                            keys::MAKER_MOS_WARRIOR_STIMULATED,
                            item_makers::RYDOLE_WARRIOR,
                        ),
                        (
                            keys::MAKER_MOS_BOWMAN_STIMULATED,
                            item_makers::RYDOLE_BOWMAN,
                        ),
                        (
                            keys::MAKER_MOS_MAGICIAN_STIMULATED,
                            item_makers::RYDOLE_MAGICIAN,
                        ),
                        (
                            keys::MAKER_MOS_THIEF_STIMULATED,
                            item_makers::RYDOLE_THIEF_STIMULATED,
                        ),
                    ],
                    &mut rand::thread_rng(),
                ))
            }
            PIG_FARM_SHADOW | PIG_FARM_DECOY | LIBRARY_SHADOW | LIBRARY_CORNER => {
                return Some(black_shadow(npc_id, player));
            }
            PUPPETEER_DOOR => return Some(puppeteer_door(player)),
            FRANCIS => return Some(francis()),
            CAVE_FRANCIS => return Some(cave_francis(player)),
            PACK_WOLF if retakes_the_wolves_test(player) => {
                return Some(the_wolves_test_again());
            }
            PILL_DESK if takes_a_pill(player) => return Some(pill_desk()),
            SWALLOW if player.quests.state(THE_SEED_SWALLOW_LOST) == QuestState::Started => {
                return Some(swallow_new_seed());
            }
            PASSAGE_END_STATUE => return Some(passage_end_statue()),
            PASSAGE_EXIT_STATUE => return Some(passage_exit_statue()),
            SHRINE_PILLAR => return Some(shrine_pillar(player)),
            _ => {}
        }
        // These fall through when silent, so the same NPCs can say more.
        if let Some(script) = INFO_SETTERS
            .iter()
            .find(|setter| setter.0 == npc_id)
            .and_then(|&setter| info_setter(setter, player))
        {
            return Some(script);
        }
        if let Some(script) = ARIANT_DELIVERIES
            .iter()
            .find(|d| d.0 == npc_id)
            .and_then(|&delivery| ariant_delivery(delivery, player))
        {
            return Some(script);
        }
        if METEORITES.contains(&npc_id) {
            if let Some(script) = meteorite(npc_id, player) {
                return Some(script);
            }
        }
        if HARP_STRINGS.contains(&npc_id) {
            return harp_string(npc_id, player);
        }
        if PENTAGRAM_CIRCLES.contains(&npc_id) {
            if let Some(script) = pentagram_circle(npc_id, player) {
                return Some(script);
            }
        }
        if NAUTILUS_BUSHES.contains(&npc_id)
            && player.quests.state(FIND_MY_GLASSES) == QuestState::Started
        {
            return Some(nautilus_bush(npc_id, player, &mut rand::thread_rng()));
        }
        if npc_id == ABEL {
            if let Some(script) = abel_takes_other_glasses(player) {
                return Some(script);
            }
        }
        if npc_id == TANGYOON && player.quests.state(FRESH_MILK) == QuestState::Started {
            return Some(tangyoon());
        }
        if let Some(&(_, cow)) = MOTHER_COWS.iter().find(|cow| cow.0 == npc_id) {
            return Some(mother_cow(npc_id, cow, player));
        }
        if CALVES.contains(&npc_id) {
            return Some(calf(npc_id, player));
        }
        if npc_id == SHINY_STONE {
            if let Some(script) = shiny_stone(player) {
                return Some(script);
            }
        }
        if let Some(rock) = TOWER_ROCKS.iter().find(|rock| rock.npc == npc_id) {
            return Some(tower_rock(rock, player));
        }
        if let Some(pile) = PATIENCE_PILES.iter().find(|pile| pile.npc == npc_id) {
            return Some(patience_pile(pile, player, &mut rand::thread_rng()));
        }
        if let Some((_, route)) = SELLERS.iter().find(|(seller, _)| *seller == npc_id) {
            return Some(ticket_seller(npc_id, route, player));
        }
        if let Some((_, route)) = SUBWAY_GATES.iter().find(|(gate, _)| *gate == npc_id) {
            return Some(subway_gate(npc_id, route, player, conti::now()));
        }
        if let Some(&(_, route, place)) = PLANE_BOOTHS.iter().find(|(booth, ..)| *booth == npc_id) {
            return Some(plane_booth(npc_id, route, place, player, conti::now()));
        }
        if PILOTS.contains(&npc_id) {
            return pilot(npc_id, player);
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
        if let Some(&(_, fare)) = FLORINA_GUIDES.iter().find(|(guide, _)| *guide == npc_id) {
            return Some(florina_guide(npc_id, fare, player));
        }
        if let Some(&(_, ride, fare)) = FERRIES.iter().find(|(pilot, ..)| *pilot == npc_id) {
            return Some(ferry(npc_id, ride, fare, player));
        }
        if let Some(town) = gachapon::town(npc_id) {
            let inventory = &player.inventory;
            if inventory.count(gachapon::TICKET) == 0 {
                return Some(gachapon_machine(npc_id, town, None));
            }
            if let Some(full) = gachapon_bag_full(npc_id, player) {
                return Some(full);
            }
            let prize = gachapon::roll(npc_id, &mut gachapon::rng());
            return Some(gachapon_machine(npc_id, town, prize));
        }
        match npc_id {
            DOOR_OF_DIMENSION => Some(door_of_dimension(player)),
            SPARKLING_CRYSTAL => sparkling_crystal(player),
            HOLY_STONE => Some(holy_stone(player, &mut rand::thread_rng())),
            SPIRUNA => Some(spiruna(player)),
            MR_THUNDER => Some(refiner(&THUNDER)),
            CHRIS => Some(refiner(&CHRIS_REFINER)),
            VOGEN => Some(refiner(&VOGEN_REFINER)),
            PI => Some(refiner(&PI_REFINER)),
            MUHAMAD => Some(refiner(&MUHAMAD_REFINER)),
            MR_SMITH => Some(refiner(&SMITH_REFINER)),
            VICIOUS => Some(refiner(&VICIOUS_REFINER)),
            JM => Some(refiner(&JM_REFINER)),
            id if refiners::PITCH_TRADERS.iter().any(|&(npc, _)| npc == id) => {
                Some(pitch_trader(id, player))
            }
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
            SPINEL => Some(spinel(player)),
            PISON => Some(pison(player)),
            THOMAS_SWIFT => thomas_swift(player),
            MOONY => Some(moony(player)),
            MOM_AND_DAD => mom_and_dad(player),
            id if NANAS.contains(&id) => nana(id, player),
            AUDREY => audrey(player),
            CRANE => crane(player),
            id if stylists::services(id).next().is_some() => {
                stylist(id, player, &mut rand::thread_rng())
            }
            NELLA => nella(player),
            ANDERSON => anderson(player),
            WONKY => wonky_inside(player),
            EAK => eak(player),
            GROWLIE => growlie(player, false, false),
            TOMMY => tommy(player, false, false),
            TORY => tory_after(player, false),
            JEFF => Some(jeff(player)),
            TIME_GATE => Some(time_gate(player)),
            CORBA => Some(corba()),
            SHANE => Some(shane(player)),
            LOUIS => Some(Self::new(
                LOUIS,
                vec![Page::Ask {
                    text: text(keys::LOUIS_ASK),
                    on_yes: vec![Effect::WarpToPortal(ELLINIA, 0)],
                    on_no: None,
                }],
            )),
            VICTORIA => Some(Self::new(
                VICTORIA,
                vec![Page::Say(text(keys::WEDDING_VICTORIA))],
            )),
            MYSTERIOUS_STATUE => Some(mysterious_statue(player)),
            PET_TEST_NOTE_SPOT => Some(hidden_note(player)),
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

    /// Gachapon `machine` played from afar with a Remote Gachapon Ticket
    /// (report b196). As Cosmic's `gachaponRemote` has it, there is no
    /// yes or no: the ticket goes for the prize at once, the machine
    /// naming it, unless a bag is full, which keeps the ticket. `None` if
    /// the player holds no ticket, or `machine` is none.
    pub fn remote_gachapon(machine: i32, player: &Player) -> Option<Self> {
        let town = gachapon::town(machine)?;
        if player.inventory.count(gachapon::REMOTE_TICKET) == 0 {
            return None;
        }
        if let Some(full) = gachapon_bag_full(machine, player) {
            return Some(full);
        }
        let prize = gachapon::roll(machine, &mut gachapon::rng())?;
        let item = prize.item_id.to_string();
        let count = prize.quantity.to_string();
        let mut script = NpcScript::new(
            machine,
            vec![Page::Act {
                text: content::text()
                    .get(keys::GACHAPON_WON, &[("item", &item), ("count", &count)]),
                effects: vec![Effect::Gachapon {
                    ticket: gachapon::REMOTE_TICKET,
                    item_id: prize.item_id,
                    quantity: prize.quantity,
                    town,
                    announced: prize.announced(),
                }],
            }],
        );
        script.inventory_full = Some(text(keys::GACHAPON_FULL));
        script.refused = Some(text(keys::GACHAPON_NO_TICKET));
        Some(script)
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
            "q1028s" => Some(shanks_sails_for_lucas()),
            "q2214e" | "q2215e" => {
                let hour = conti::time_of_day(conti::now()).0;
                Some(trash_can(player, script == "q2215e", hour))
            }
            "q2216s" => informant(player, 0),
            "q2217s" => informant(player, 1),
            "q2218s" => informant(player, 2),
            "q2219s" => informant(player, 3),
            "q2228s" => Some(reef()),
            "q2238s" => Some(NpcScript::new(
                MU_YOUNG,
                vec![Page::Done(vec![Effect::StartQuest(2238)])],
            )),
            "q2245s" => Some(tristan_tomb()),
            "q2300s" | "q2301s" | "q2302s" | "q2303s" | "q2304s" => {
                letter_for_the_kingdom(script, player)
            }
            "q2300e" | "q2301e" | "q2302e" | "q2303e" | "q2304e" => letter_read(script),
            "q2325e" => Some(james_found()),
            "q2327s" => Some(james_escapes()),
            "q2338s" => Some(scarrs_spare_spore()),
            "q2251e" => Some(recording_charms()),
            "q2254s" => Some(rememberer_points_to_karcasa()),
            "q2257e" => Some(camel_cab_says(keys::CAMEL_CAB_KARCASA, KARCASA_RELENTS)),
            "q2258s" => Some(meerkats_ask()),
            "q2258e" => Some(camel_cab_says(keys::CAMEL_CAB_MEERKATS_DONE, MEERKATS)),
            "q2259s" => Some(camel_cab_starts(keys::CAMEL_CAB_SAHEL, SCORPIONS)),
            "q2259e" => Some(scorpions_cant_listen(player)),
            "q2260s" => Some(camel_cab_starts(keys::CAMEL_CAB_LEVEL_30, TO_THE_CASTLE)),
            "q2260e" => Some(camel_cab_says(keys::CAMEL_CAB_CASTLE_WAY, TO_THE_CASTLE)),
            "q3301e" => Some(han_the_broker(
                player,
                ZENUMIST_TEST,
                keys::HAN_ZENUMIST_ORES,
                keys::HAN_ZENUMIST_WAIT,
            )),
            "q3303e" => Some(han_the_broker(
                player,
                ALCADNO_TEST,
                keys::HAN_ALCADNO_ORES,
                keys::HAN_ALCADNO_WAIT,
            )),
            "q3305s" => Some(cape_again(
                CARSON,
                ZENUMIST_CAPE_AGAIN,
                [
                    keys::CARSON_CAPE_LOST,
                    keys::CARSON_CAPE_ASK,
                    keys::CARSON_CAPE_GO,
                ],
            )),
            "q3306s" => Some(cape_again(
                MAED,
                ALCADNO_CAPE_AGAIN,
                [
                    keys::MAED_CAPE_LOST,
                    keys::MAED_CAPE_ASK,
                    keys::MAED_CAPE_GO,
                ],
            )),
            "q3320s" => Some(what_parwen_knows()),
            "q3321s" => Some(the_missing_alchemist()),
            "q3353s" => Some(what_de_lang_wants()),
            "q3354s" => Some(de_langs_potion()),
            "q3360s" => Some(verifying_the_password(&mut rand::thread_rng())),
            "q3452e" => Some(dr_kims_thanks()),
            "q3833e" => Some(mr_kus_bellflowers(player)),
            "q3953e" => Some(convince_muhamad()),
            "q3933s" => Some(ardins_sand_bandits()),
            "q3514e" => Some(the_sorcerers_potion()),
            "q3455s" => Some(the_faulty_translator()),
            "q2124e" => Some(moppies_box(SAND_CREW_SUPPLY, MOPPIES_BOX)),
            "q2126e" => Some(moppies_box(SAND_CREW_SUPPLY_AGAIN, MOPPIES_BOX_AGAIN)),
            "q2127e" => Some(to_the_desert()),
            "q2186e" => Some(abels_glasses_found()),
            "q2148s" => truth_of_the_rumor(2148),
            "q2149s" => truth_of_the_rumor(2149),
            "q2150s" => truth_of_the_rumor(2150),
            "q2151s" => truth_of_the_rumor(2151),
            "q2152s" => truth_of_the_rumor(2152),
            "q2197e" => Some(tienks_greeting()),
            "q2293s" => Some(the_last_song()),
            "q8219e" => Some(jack_meets_the_player()),
            "q8222e" => Some(lukans_badges(player)),
            "q8227s" => Some(lost_in_translation_start(player, 8227)),
            "q8228s" => Some(lost_in_translation_start(player, 8228)),
            "q8229s" => Some(lost_in_translation_start(player, 8229)),
            "q8228e" => Some(elpam_translates(player)),
            "q8229e" => Some(jack_reads_the_translation(player)),
            "q8230e" => Some(jack_sees_the_keystone(player)),
            "q29900s" | "q29901s" | "q29902s" | "q29903s" => adventurer_title(script),
            "q29900e" | "q29901e" | "q29902e" | "q29903e" => adventurer_medal(script),
            "q29002s" => Some(celebrity_start(player)),
            "q29002e" => Some(celebrity_end(player, SystemTime::now())),
            "q29400s" => Some(veteran_hunter_start()),
            "q29400e" => Some(veteran_hunter_end(player, SystemTime::now())),
            "q29910s" | "q29911s" | "q29912s" | "q29913s" | "q29914s" | "q29915s" | "q29916s"
            | "q29917s" | "q29918s" | "q29919s" | "q29920s" | "q29921s" | "q29933s" | "q29922s"
            | "q29923s" | "q29924s" | "q29925s" | "q29926s" | "q29927s" | "q29928s" => {
                medal_title(script)
            }
            "q29906s" | "q29907s" | "q29908s" | "q29909s" => knight_title(script),
            "q2314s" => Some(secretary_sends(
                EXPLORING_MUSHROOM_FOREST,
                keys::MC_FOREST_ASK,
                Some(keys::MC_FOREST_DECLINE),
                keys::MC_FOREST_GO,
            )),
            "q2322s" => Some(secretary_sends(
                OVER_THE_CASTLE_WALLS,
                keys::MC_WALLS_ASK,
                None,
                keys::MC_WALLS_GO,
            )),
            "q2332s" => Some(NpcScript::new(
                VIOLETTA,
                vec![Page::Done(vec![Effect::StartQuest(
                    events::WHERES_VIOLETTA,
                )])],
            )),
            "q2332e" => Some(violetta_found()),
            "q2333s" => Some(violetta_betrayed()),
            "q2333e" => Some(prime_minister_beaten()),
            "q2334s" => Some(violettas_face()),
            "q2335s" => Some(violettas_key()),
            "q2342s" => Some(violettas_seal(player)),
            "q3108s" => Some(snow_statue_clue()),
            "q3523s" => lost_memory(3523),
            "q3524s" => lost_memory(3524),
            "q3525s" => lost_memory(3525),
            "q3526s" => lost_memory(3526),
            "q3527s" => lost_memory(3527),
            "q3529s" => lost_memory(3529),
            "q3539s" => lost_memory(3539),
            "q3714s" => Some(baby_dragon(player)),
            "q21733s" => Some(fake_tru_calls()),
            "q21733e" => Some(tru_on_the_trap()),
            "q21734s" => Some(puppeteer_traces()),
            "q21734e" => Some(puppeteer_trail_beaten()),
            "q21735s" => Some(seal_stone_for_lilin(player)),
            "q21735e" => Some(seal_stone_kept()),
            "q21736s" => Some(off_to_orbis()),
            "q21738s" => Some(sealed_garden()),
            "q21739e" => Some(sealed_garden_cleared()),
            "q21740s" => Some(orbis_stone_stolen()),
            "q21740e" => Some(lilin_teaches_combo_smash()),
            "q21741s" => Some(off_to_mu_lung()),
            "q21742s" => Some(black_scroll(player)),
            "q21742e" => Some(black_scroll_read()),
            "q21746s" => Some(mu_gongs_test()),
            "q21747s" => Some(catch_the_shadow_knight()),
            "q21747e" => Some(shadow_knight_caught()),
            "q21748e" => Some(lilin_teaches_final_charge()),
            "q21749s" => Some(back_to_the_past()),
            "q21750e" => Some(athena_remembers()),
            "q21753s" => Some(athena_keeps_the_letter()),
            "q21754s" => Some(athena_hands_over_the_letter(player)),
            "q21757e" => Some(neinheart_takes_the_letter()),
            "q21700s" => Some(new_beginnings()),
            "q21703s" => Some(target_pigs()),
            "q21703e" => Some(combo_ability()),
            "q21704s" => Some(baby_steps()),
            "q21712s" => Some(assignment_done(
                BRUCE,
                FIRST_ASSIGNMENT_DONE,
                keys::BRUCE_FIRST_ASSIGNMENT,
            )),
            "q21716s" => Some(assignment_done(
                ROWEN,
                SECOND_ASSIGNMENT_DONE,
                keys::ROWEN_SECOND_ASSIGNMENT,
            )),
            "q21719s" => Some(puppeteers_invitation()),
            "q21720e" => Some(polearm_booster()),
            "q21729s" => Some(fourth_assignment_done()),
            "q21766s" => Some(suspicious_john()),
            "q21766e" => Some(john_cleared()),
            "q21767s" => Some(wooden_box(player)),
            "q21600s" => Some(puccis_request()),
            "q21604s" => Some(wolf_mount()),
            "q21613s" => Some(the_wolves_test()),
            "q21618s" => Some(the_wolfs_evolution()),
            "q21618e" => Some(the_wolf_evolved(player)),
            "q3314e" => Some(life_alchemy_done(player, &mut rand::thread_rng())),
            "q3382e" => Some(yuletes_reward(player)),
            "q3941s" => Some(karcasa_silk(player, false)),
            "q3941e" => Some(karcasa_silk(player, true)),
            "q8255s" => Some(adonis_lost_spirits()),
            "q2156e" => Some(pia_rainbow_shell(player)),
            "q2291e" => Some(vip_admission(player)),
            "q3250s" => Some(baby_bird_offer(player)),
            "q3250e" => Some(baby_bird_grown()),
            "q6030e" => Some(maker_class(CARSON, keys::MAKER_ALCHEMY, 6030)),
            "q6031e" => Some(maker_class(HUGHES, keys::MAKER_SCIENCE, 6031)),
            "q6032e" => Some(maker_class(MEREN, keys::MAKER_PRACTICE, 6032)),
            "q6033e" => Some(maker_lesson(player, keys::MAKER_CRYSTAL_TEST, 6033)),
            "q6036e" => Some(maker_lesson(player, keys::MAKER_GOLD_ANVIL, 6036)),
            "q4647e" => Some(pet_secret(player)),
            "q8185e" => Some(pet_evolution(player, 8185)),
            "q4659e" => Some(pet_evolution(player, ROBO_UPGRADE_QUEST)),
            "q8189e" => Some(pet_evolution(player, 8189)),
            _ => new_leaf_city_start(script),
        }
    }

    /// Captain Al's end scripts for his lessons in leading a family
    /// (`q2232e`-`q2234e`, quests 2232-2234), which read the Family window's
    /// numbers (`family`): a junior taken (2232), 1,000 Rep earned in all
    /// (2233), then 2,000 earned with the Rep to spend used below 500
    /// (2234). The data gives no rewards; 2232 gives Cosmic's 3,000 EXP,
    /// and Cosmic has no script for the other two. Dalair's end script for
    /// Outstanding Citizen (`q29508e`) also reads them.
    pub fn for_family_quest(script: &str, player: &Player, family: &FamilyInfo) -> Option<Self> {
        if script == "q29508e" {
            return Some(outstanding_citizen(player, family));
        }
        let (quest, done, [done_key, not_yet_key]) = match script {
            "q2232e" => (2232, family.juniors > 0, keys::CAPTAIN_AL_JUNIOR),
            "q2233e" => (2233, family.total_reputation >= 1000, keys::CAPTAIN_AL_REP),
            "q2234e" => (
                2234,
                family.total_reputation >= 2000 && family.reputation < 500,
                keys::CAPTAIN_AL_ENTITLEMENT,
            ),
            _ => return None,
        };
        if !done {
            return Some(Self::new(CAPTAIN_AL, vec![Page::Say(text(not_yet_key))]));
        }
        let mut script = Self::new(CAPTAIN_AL, vec![Page::Say(text(done_key))]);
        script.on_end.push(Effect::CompleteQuest(quest));
        if quest == 2232 {
            script.on_end.push(Effect::GainExp(3000));
        }
        Some(script)
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
            (
                Page::Say(_) | Page::Reply(_) | Page::Act { .. } | Page::End(_),
                ScriptInput::Next,
            ) => Some(self.advance(Vec::new())),
            (Page::Say(_) | Page::Reply(_), ScriptInput::Previous) if self.has_previous() => {
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
            (Page::Decide { on_yes, on_no, .. }, input @ (ScriptInput::Yes | ScriptInput::No)) => {
                self.pages = if input == ScriptInput::Yes {
                    on_yes.clone()
                } else {
                    on_no.clone()
                };
                self.page = 0;
                if self.pages.is_empty() {
                    return Some(Step::default());
                }
                if matches!(self.pages.last(), Some(Page::End(_))) {
                    self.on_end.clear();
                }
                Some(self.begin())
            }
            (Page::Menu { choices, .. }, ScriptInput::Select(choice)) => {
                let (_, pages) = choices.get(usize::try_from(choice).ok()?)?;
                self.pages = pages.clone();
                self.page = 0;
                if matches!(self.pages.last(), Some(Page::End(_))) {
                    self.on_end.clear();
                }
                Some(self.begin())
            }
            (Page::Slide { choices, .. }, ScriptInput::Select(id)) if id >= 0 => {
                let (_, _, pages) = choices.iter().find(|(choice, ..)| *choice == id)?;
                self.pages = pages.clone();
                self.page = 0;
                if matches!(self.pages.last(), Some(Page::End(_))) {
                    self.on_end.clear();
                }
                Some(self.begin())
            }
            (Page::AskText { on_text, .. }, ScriptInput::Text(text)) => {
                self.pages = on_text.pages(&text);
                self.page = 0;
                Some(self.begin())
            }
            (Page::AskNumber { on_number, .. }, ScriptInput::Number(number)) => {
                self.pages = on_number.pages(number);
                self.page = 0;
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
        self.page > 0 && matches!(self.pages[self.page - 1], Page::Say(_) | Page::Reply(_))
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
            Page::Say(text) | Page::Reply(text) => Prompt::Text(DialoguePage {
                npc_id,
                text: text.clone(),
                has_previous: self.has_previous(),
                has_next: self.page + 1 < self.pages.len() || !self.on_end.is_empty(),
                player_speaks: self.player_speaks
                    || matches!(self.pages[self.page], Page::Reply(_)),
            }),
            Page::Act { text, effects: own } => {
                effects.extend(own.iter().cloned());
                Prompt::Text(DialoguePage {
                    npc_id,
                    text: text.clone(),
                    has_previous: false,
                    has_next: self.page + 1 < self.pages.len() || !self.on_end.is_empty(),
                    player_speaks: self.player_speaks,
                })
            }
            Page::End(text) => Prompt::Text(DialoguePage {
                npc_id,
                text: text.clone(),
                has_previous: false,
                has_next: false,
                player_speaks: self.player_speaks,
            }),
            Page::Ask { text, .. } | Page::Decide { text, .. } => Prompt::YesNo {
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
            // The client splits the menu at each "#" and adds one at the
            // end, so a label may hold none and the last ends bare.
            Page::Slide { set, choices } => Prompt::Slide {
                npc_id,
                set: *set,
                menu: choices
                    .iter()
                    .map(|(id, label, _)| format!("#{id}#{}", label.replace('#', "")))
                    .collect(),
            },
            Page::AskText { text, min, max, .. } => Prompt::AskText {
                npc_id,
                text: text.clone(),
                min: *min,
                max: *max,
            },
            Page::AskNumber {
                text,
                default,
                min,
                max,
                ..
            } => Prompt::AskNumber {
                npc_id,
                text: text.clone(),
                default: *default,
                min: *min,
                max: *max,
            },
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

/// `q6030e`-`q6032e`: a Maker class ends. The fee was taken when it
/// started (`Act/0 money`).
fn maker_class(npc: i32, pages: [&str; 2], quest: i16) -> NpcScript {
    let mut script = NpcScript::new(npc, pages.map(|key| Page::Say(text(key))).to_vec());
    script.on_end = vec![Effect::CompleteQuest(quest)];
    script
}

/// `q6033e` and `q6036e`: Meren teaches the Maker's level 2 for the
/// crystal test and level 3 for the Gold Anvil, which he keeps. The data
/// has no reward for either, so the EXP and levels are Cosmic's.
fn maker_lesson(player: &Player, pages: [&str; 2], quest: i16) -> NpcScript {
    let mut script = NpcScript::new(MEREN, pages.map(|key| Page::Say(text(key))).to_vec());
    let (level, exp) = if quest == 6033 {
        (2, 230_000)
    } else {
        (3, 300_000)
    };
    if quest == 6036 {
        script.on_end.push(Effect::TakeItem {
            item_id: GOLD_ANVIL,
            quantity: 1,
        });
    }
    script.on_end.extend([
        Effect::CompleteQuest(quest),
        Effect::TeachSkill {
            skill_id: crate::skills::maker_skill(player.stats.job),
            level,
            master_level: 3,
        },
        Effect::GainExp(exp),
    ]);
    script
}

/// `multipet_success`: the hidden note of Bartos's test, which only someone
/// on the test (4646) finds, once.
fn hidden_note(player: &Player) -> NpcScript {
    let say = |key| NpcScript::new(PET_TEST_NOTE_SPOT, vec![Page::Say(text(key))]);
    if player.quests.state(PET_TEST_QUEST) != QuestState::Started {
        return say(keys::PET_NOTE_NOTHING);
    }
    if player.inventory.count(HIDDEN_NOTE) > 0 {
        return say(keys::PET_NOTE_HELD);
    }
    let mut script = NpcScript::new(
        PET_TEST_NOTE_SPOT,
        vec![Page::Ask {
            text: text(keys::PET_NOTE_ASK),
            on_yes: vec![Effect::GiveItem {
                item_id: HIDDEN_NOTE,
                quantity: 1,
            }],
            on_no: None,
        }],
    );
    script.inventory_full = Some(text(keys::PET_NOTE_FULL));
    script
}

/// `q4647e`: Bartos takes a Pet Snack and teaches Follow the Lead, which
/// the quest data doesn't give.
fn pet_secret(player: &Player) -> NpcScript {
    if player.inventory.count(PET_SNACK) == 0 {
        return NpcScript::new(BARTOS, vec![Page::Say(text(keys::PET_SECRET_NO_SNACK))]);
    }
    let mut script = NpcScript::new(
        BARTOS,
        keys::PET_SECRET.map(|key| Page::Say(text(key))).to_vec(),
    );
    script.on_end = vec![
        Effect::TakeItem {
            item_id: PET_SNACK,
            quantity: 1,
        },
        Effect::CompleteQuest(PET_SECRET_QUEST),
        Effect::TeachSkill {
            skill_id: game::pet::follow_the_lead(player.stats.job),
            level: 1,
            master_level: 1,
        },
    ];
    script
}

/// JM's list of informants (Kerning City, quests 2214-2219).
const INFORMANT_LIST: i32 = 4031894;
/// The Knocked Trash Can in the Hut in the Swamp (107000301), where JM
/// leaves it.
const TRASH_CAN: i32 = 1052108;
/// The fee for a lost list (`q2215e`, `Act/1 money`).
const INFORMANT_LIST_FEE: i32 = 2000;
/// JM's informants and their quests: Mr. Pickall, Shumi, Nella and Jake.
const INFORMANTS: [(i16, i32); 4] = [
    (2216, 9000008),
    (2217, 1052102),
    (2218, 1052103),
    (2219, 1052006),
];

/// `q2214e`, and `q2215e` for a `lost` list: the trash can in the Hut in
/// the Swamp holds JM's list from 5 to 8 p.m. (QuestInfo 2214), by the hour the stations'
/// clocks show. The first list comes with 20,000 EXP (Cosmic's; the data
/// gives only the list); a lost one costs the fee.
fn trash_can(player: &Player, lost: bool, hour: u8) -> NpcScript {
    let say = |key| NpcScript::new(TRASH_CAN, vec![Page::Say(text(key))]);
    if !(17..20).contains(&hour) {
        return say(keys::DYLE_TRASH_CAN_NOT_YET);
    }
    if lost && player.meso < INFORMANT_LIST_FEE {
        return say(keys::DYLE_TRASH_CAN_SHORT);
    }
    let (mut script, quest) = if lost {
        (say(keys::DYLE_TRASH_CAN_PAID), 2215)
    } else {
        (say(keys::DYLE_TRASH_CAN_FOUND), 2214)
    };
    if lost {
        script.on_end.push(Effect::SpendMeso(INFORMANT_LIST_FEE));
    }
    script.on_end.extend([
        Effect::CompleteQuest(quest),
        Effect::GiveItem {
            item_id: INFORMANT_LIST,
            quantity: 1,
        },
    ]);
    if !lost {
        script.on_end.push(Effect::GainExp(20_000));
    }
    script.inventory_full = Some(text(keys::DYLE_TRASH_CAN_FULL));
    script
}

/// `q2228s`, Reef's Gratitude: Reef's spirit, at rest once Faust is beaten
/// (2227), thanks the player, which completes the quest as it starts, with
/// the data's 8 fame (`Act/1 pop`, which a start does not give).
fn reef() -> NpcScript {
    let mut script = NpcScript::new(REEF, vec![Page::Say(text(keys::REEF_THANKS))]);
    script.on_end = vec![Effect::FinishQuest(2228), Effect::GainFame(8)];
    script
}

/// Reef, in 910100001.
const REEF: i32 = 1032108;
/// Mu Young, in the Sleepywood dungeon (105100100).
const MU_YOUNG: i32 = 1061014;

/// `q2245s`, To Tristan's Tomb: Mu Young sends a player on 2238 into the
/// unknown Jr. Balrog's room at once, with no word (Cosmic `2245.js`), and
/// 2245 completes as it starts, so that `interval 0` lets them go again.
/// Only one may be in the room on a channel at a time; another is told so
/// and nothing is applied.
fn tristan_tomb() -> NpcScript {
    let go = vec![
        Effect::FinishQuest(2245),
        Effect::StartEvent(EventKind::Tristan),
    ];
    let mut script = NpcScript::new(MU_YOUNG, vec![Page::Done(go)]);
    script.refused = Some(text(keys::TRISTAN_ROOM_TAKEN));
    script
}

const JAMES: i32 = 1300008;
const SCARRS: i32 = 1300007;
const KILLER_MUSHROOM_SPORE: i32 = 2430014;
const JAMES_FOUND: i16 = 2325;
const JAMES_ESCAPES: i16 = 2327;
const SPORES_AGAIN: i16 = 2338;

/// `q2325e`, James's Whereabouts (1): James, hiding in the castle, is
/// afraid until the player answers that his brother, the Head Security
/// Officer, sent them; then the quest completes for 6,000 EXP (its `Act`,
/// which a script's completion doesn't apply; Cosmic `2325.js`). The text
/// is unsourced.
fn james_found() -> NpcScript {
    let mut script = NpcScript::new(
        JAMES,
        vec![
            Page::Say(text(keys::JAMES_SCARED)),
            Page::Reply(text(keys::JAMES_SENT_BY_BROTHER)),
            Page::Say(text(keys::JAMES_SAFE)),
        ],
    );
    script.on_end = vec![Effect::CompleteQuest(JAMES_FOUND), Effect::GainExp(6000)];
    script
}

/// `q2327s`, James's Whereabouts (3): James thanks the player for the
/// Mushroom Disguise of 2326 and starts the quest to put it on (Cosmic
/// `2327.js`); he completes it himself, from the data. The text is
/// unsourced.
fn james_escapes() -> NpcScript {
    let mut script = NpcScript::new(
        JAMES,
        vec![
            Page::Say(text(keys::JAMES_THANKS)),
            Page::Say(text(keys::JAMES_DISGUISE)),
        ],
    );
    script.on_end = vec![Effect::StartQuest(JAMES_ESCAPES)];
    script
}

/// `q2338s`, Killer Mushroom Spores, Again: for one who used up the spore
/// of 2318 before 2322 (the start asks that none be held), Scarrs gives a
/// spare (2430014) and the quest completes as it starts (Cosmic
/// `2338.js`). Without a USE slot he says so. The text is unsourced.
fn scarrs_spare_spore() -> NpcScript {
    let mut script = NpcScript::new(SCARRS, vec![Page::Say(text(keys::SCARRS_SPARE_SPORE))]);
    script.on_end = vec![
        Effect::GiveItem {
            item_id: KILLER_MUSHROOM_SPORE,
            quantity: 1,
        },
        Effect::FinishQuest(SPORES_AGAIN),
    ];
    script.inventory_full = Some(text(keys::SCARRS_SPORE_FULL));
    script
}

const REMEMBERER: i32 = 1061011;
const RECORDING_CHARM: i32 = 4032399;
const ZOMBIE_SIGNAL_3: i16 = 2251;
const KARCASA_OF_THE_DESERT: i16 = 2254;
const KARCASA_RELENTS: i16 = 2257;
const MEERKATS: i16 = 2258;
const SCORPIONS: i16 = 2259;
const TO_THE_CASTLE: i16 = 2260;

/// `q2251e`, Zombie Mushroom Signal 3: The Rememberer takes the 20
/// Recording Charms the end asks for (4032399, a Zombie Mushroom's quest
/// drop) and completes the quest for its 8,000 EXP, which a script's
/// completion doesn't apply (Cosmic `2251.js`). The text is unsourced.
fn recording_charms() -> NpcScript {
    let mut script = NpcScript::new(REMEMBERER, vec![Page::Say(text(keys::REMEMBERER_CHARMS))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: RECORDING_CHARM,
            quantity: 20,
        },
        Effect::CompleteQuest(ZOMBIE_SIGNAL_3),
        Effect::GainExp(8000),
    ];
    script
}

/// `q2254s`, Karcasa of the Desert: The Rememberer's research says the
/// Mushroom Castle is in trouble, and Karcasa (2101013) may know more;
/// he ends the quest from the data. No source has the script, so its one
/// page follows `QuestInfo` and is unsourced.
fn rememberer_points_to_karcasa() -> NpcScript {
    let mut script = NpcScript::new(REMEMBERER, vec![Page::Say(text(keys::REMEMBERER_KARCASA))]);
    script.on_end = vec![Effect::StartQuest(KARCASA_OF_THE_DESERT)];
    script
}

/// The Camel Cab's page for an About the Mushroom Castle... quest it
/// ends (`q2257e`, `q2258e`, `q2260e`; Cosmic `2257.js` to `2260.js`),
/// whose checks are the data's: 2258's 40 Meerkats, 2260's level 30. The
/// text is unsourced.
fn camel_cab_says(key: &str, quest: i16) -> NpcScript {
    let mut script = NpcScript::new(CAMEL_CAB, vec![Page::Say(text(key))]);
    script.on_end = vec![Effect::CompleteQuest(quest)];
    script
}

/// The Camel Cab's page for a quest it starts (`q2259s`, `q2260s`).
fn camel_cab_starts(key: &str, quest: i16) -> NpcScript {
    let mut script = NpcScript::new(CAMEL_CAB, vec![Page::Say(text(key))]);
    script.on_end = vec![Effect::StartQuest(quest)];
    script
}

/// `q2258s`, Meerkats Listen During the Day: the Camel Cab asks the
/// player to clear Meerkats from his way; on yes he says how many and the
/// quest starts, with its five-minute clock (`QuestInfo` `timeLimit`).
fn meerkats_ask() -> NpcScript {
    let mut script = NpcScript::new(
        CAMEL_CAB,
        vec![
            Page::Ask {
                text: text(keys::CAMEL_CAB_MEERKATS_ASK),
                on_yes: Vec::new(),
                on_no: None,
            },
            Page::Say(text(keys::CAMEL_CAB_MEERKATS_GO)),
        ],
    );
    script.on_end = vec![Effect::StartQuest(MEERKATS)];
    script
}

/// `q2259e`, Scorpions Can't Listen at Night: the Camel Cab tells what he
/// knows only at Sahel 1, where no Meerkat listens; outside Ariant he
/// sends the player on and the quest stays open (Cosmic `2259.js`).
fn scorpions_cant_listen(player: &Player) -> NpcScript {
    if player.stats.map_id == OUTSIDE_ARIANT {
        return NpcScript::new(CAMEL_CAB, vec![Page::Say(text(keys::CAMEL_CAB_NOT_HERE))]);
    }
    camel_cab_says(keys::CAMEL_CAB_PRIVATE, SCORPIONS)
}

/// Joining Zenumist and Joining Alcadno (3301, 3303): Han the Broker
/// (2111007) gets a society's test ready for two of any one jewel ore.
const HAN_THE_BROKER: i32 = 2111007;
const MAED: i32 = 2111001;
const ZENUMIST_TEST: i16 = 3301;
const ALCADNO_TEST: i16 = 3303;
/// The jewel ores, Garnet Ore to Black Crystal Ore.
const JEWEL_ORES: std::ops::RangeInclusive<i32> = 4020000..=4020008;
/// Re-acquiring a society's cape (3305, 3306).
const ZENUMIST_CAPE_AGAIN: i16 = 3305;
const ALCADNO_CAPE_AGAIN: i16 = 3306;

/// `q3301e` and `q3303e`, Test from the Head of a society: Han the Broker
/// takes two of whichever jewel ore the player picks from those they hold
/// two of, and the quest completes (Cosmic `3301.js`, `3303.js`); without
/// any, there is no deal. The data's Korean start pages ask 30000 and 50000
/// mesos instead, which no end script takes. The text is unsourced.
fn han_the_broker(player: &Player, quest: i16, ores: &str, wait: &str) -> NpcScript {
    let choices: Vec<_> = JEWEL_ORES
        .filter(|&ore| player.inventory.count(ore) >= 2)
        .map(|ore| {
            let effects = vec![
                Effect::TakeItem {
                    item_id: ore,
                    quantity: 2,
                },
                Effect::CompleteQuest(quest),
            ];
            let pages = vec![Page::Act {
                text: text(wait),
                effects,
            }];
            (format!("#t{ore}#"), pages)
        })
        .collect();
    if choices.is_empty() {
        return NpcScript::new(HAN_THE_BROKER, vec![Page::Say(text(keys::HAN_NO_ORES))]);
    }
    let mut script = NpcScript::new(
        HAN_THE_BROKER,
        vec![Page::Menu {
            text: text(ores),
            choices,
        }],
    );
    script.refused = Some(text(keys::HAN_NO_ORES));
    script
}

/// `q3305s` and `q3306s`, Re-acquiring a society's cape: its head (Carson,
/// Maed) will make another for 5 of a jewel, 10 Leather and 10000 mesos,
/// which the data's end takes; on yes the quest starts (Cosmic `3305.js`,
/// `3306.js`). The text is unsourced.
fn cape_again(npc: i32, quest: i16, keys: [&str; 3]) -> NpcScript {
    let [lost, ask, go] = keys;
    let mut script = NpcScript::new(
        npc,
        vec![
            Page::Say(text(lost)),
            Page::Ask {
                text: text(ask),
                on_yes: Vec::new(),
                on_no: None,
            },
            Page::Say(text(go)),
        ],
    );
    script.on_end = vec![Effect::StartQuest(quest)];
    script
}

/// Dr. De Lang, Magatia's missing alchemist, hides in Dran's Lab
/// (926120200), which has no portal: Parwen (2111006) sends the player
/// there, and its forced return is Parwen's map.
const PARWEN: i32 = 2111006;
const DE_LANG: i32 = 2111002;
const DRANS_LAB: i32 = 926120200;
const WHAT_PARWEN_KNOWS: i16 = 3320;
const THE_MISSING_ALCHEMIST: i16 = 3321;
const WHAT_DE_LANG_WANTS: i16 = 3353;
const DE_LANGS_POTION: i16 = 3354;
/// The Secret Passage, the door's password prompt (Cosmic
/// `MagatiaPassword`).
pub const SECRET_PASSAGE: i32 = 2111024;
/// Parwen's password: this many of `PASSWORD_CHARS`.
const PASSWORD_LEN: usize = 10;
const PASSWORD_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Parwen once What Parwen Knows is taken: straight to Dran's Lab (Cosmic
/// `2111006.js`).
fn parwen() -> NpcScript {
    NpcScript::new(PARWEN, vec![Page::Done(vec![Effect::Warp(DRANS_LAB)])])
}

/// `q3320s`, What Parwen Knows: Parwen asks, then the quest starts and the
/// player is in Dran's Lab, where De Lang ends it (Cosmic `3320.js`). The
/// text is unsourced.
fn what_parwen_knows() -> NpcScript {
    let mut script = NpcScript::new(
        PARWEN,
        vec![Page::Ask {
            text: text(keys::PARWEN_KNOWS_ASK),
            on_yes: Vec::new(),
            on_no: None,
        }],
    );
    script.on_end = vec![
        Effect::StartQuest(WHAT_PARWEN_KNOWS),
        Effect::Warp(DRANS_LAB),
    ];
    script
}

/// `q3321s`, Dr. De Lang, the Missing Alchemist: De Lang tells of the
/// pendant for Phyllia and the pipes' password, asks, and the quest starts
/// (Cosmic `3321.js`). The text is unsourced.
fn the_missing_alchemist() -> NpcScript {
    let mut script = NpcScript::new(
        DE_LANG,
        vec![
            Page::Say(text(keys::DE_LANG_MISSING_STORY)),
            Page::Say(text(keys::DE_LANG_MISSING_HELP)),
            Page::Ask {
                text: text(keys::DE_LANG_MISSING_ASK),
                on_yes: Vec::new(),
                on_no: None,
            },
            Page::Say(text(keys::DE_LANG_MISSING_PENDANT)),
            Page::Say(text(keys::DE_LANG_MISSING_PIPES)),
            Page::Say(text(keys::DE_LANG_MISSING_GO)),
        ],
    );
    script.on_end = vec![Effect::StartQuest(THE_MISSING_ALCHEMIST)];
    script
}

/// `q3353s`, What De Lang Wants: Parwen tells, asks, and on yes starts the
/// quest and sends the player to Dran's Lab (Cosmic `3353.js`). The text is
/// unsourced.
fn what_de_lang_wants() -> NpcScript {
    let mut script = NpcScript::new(
        PARWEN,
        vec![
            Page::Say(text(keys::PARWEN_DE_LANG_WANTS)),
            Page::Ask {
                text: text(keys::PARWEN_DE_LANG_WANTS_ASK),
                on_yes: Vec::new(),
                on_no: None,
            },
        ],
    );
    script.on_end = vec![
        Effect::StartQuest(WHAT_DE_LANG_WANTS),
        Effect::Warp(DRANS_LAB),
    ];
    script
}

/// `q3354s`, De Lang's Potion: De Lang asks, and on yes the quest starts
/// (Cosmic `3354.js`). The text is unsourced.
fn de_langs_potion() -> NpcScript {
    let mut script = NpcScript::new(
        DE_LANG,
        vec![Page::Ask {
            text: text(keys::DE_LANG_POTION_ASK),
            on_yes: Vec::new(),
            on_no: None,
        }],
    );
    script.on_end = vec![Effect::StartQuest(DE_LANGS_POTION)];
    script
}

/// `q3360s`, Verifying the Password: Parwen makes up a password of ten
/// digits and capitals, tells it once, and the quest starts with it as its
/// info; the Secret Passage asks for it (Cosmic `3360.js`). No is a word
/// to hurry. The text is unsourced.
fn verifying_the_password(rng: &mut impl Rng) -> NpcScript {
    let password: String = (0..PASSWORD_LEN)
        .map(|_| char::from(*PASSWORD_CHARS.choose(rng).unwrap()))
        .collect();
    let code = content::text().get(
        keys::PARWEN_PASSWORD_CODE,
        &[("password", password.as_str())],
    );
    let mut script = NpcScript::new(
        PARWEN,
        vec![
            Page::Say(text(keys::PARWEN_PASSWORD_KEY)),
            Page::Ask {
                text: text(keys::PARWEN_PASSWORD_ASK),
                on_yes: Vec::new(),
                on_no: Some(text(keys::PARWEN_PASSWORD_HURRY)),
            },
            Page::Say(code),
        ],
    );
    script.on_end = vec![
        Effect::StartQuest(scripts::VERIFYING_THE_PASSWORD),
        Effect::SetQuestInfo {
            quest: scripts::VERIFYING_THE_PASSWORD,
            value: password,
        },
    ];
    script
}

/// The Secret Passage while Parwen's password is still to be given (the
/// `secretDoor` portal opens it): it asks for the password.
fn secret_passage(player: &Player) -> Option<NpcScript> {
    let password = player
        .quests
        .info(scripts::VERIFYING_THE_PASSWORD)
        .filter(|password| password.len() > 1)?;
    Some(NpcScript::new(
        SECRET_PASSAGE,
        vec![Page::AskText {
            text: text(keys::SECRET_PASSAGE_PASSWORD),
            min: 1,
            max: PASSWORD_LEN as i16,
            on_text: OnText::SecretPassage {
                password: password.to_owned(),
                through: scripts::through_secret_door(player),
            },
        }],
    ))
}

/// The right password marks Verifying the Password done ("1") and opens
/// the passage; any other is wrong (Cosmic `MagatiaPassword.js`).
fn secret_passage_opens(right: bool, through: &Effect) -> Vec<Page> {
    if !right {
        return vec![Page::Say(text(keys::SECRET_PASSAGE_WRONG))];
    }
    vec![Page::Done(vec![
        Effect::SetQuestInfo {
            quest: scripts::VERIFYING_THE_PASSWORD,
            value: "1".into(),
        },
        Effect::PortalSound,
        through.clone(),
    ])]
}

/// Blocktopus is an Alien? (3452): Dr. Kim (2050001) in Omega Sector.
const DR_KIM: i32 = 2050001;
const BLOCKTOPUS_ALIEN: i16 = 3452;
const BLOCTOPUS_KEY_CHAIN: i32 = 4000099;
const MANA_ELIXIR_PILL: i32 = 2000011;

/// `q3452e`, Blocktopus is an Alien?: Dr. Kim takes the Bloctopus Key
/// Chain the end asks for and gives 50 Mana Elixir Pills and 8,000 EXP,
/// which a script's completion doesn't apply (Cosmic `3452.js`). Without a
/// USE slot Dr. Kim says so. The text is unsourced.
fn dr_kims_thanks() -> NpcScript {
    let mut script = NpcScript::new(DR_KIM, vec![Page::Say(text(keys::DR_KIM_THANKS))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: BLOCTOPUS_KEY_CHAIN,
            quantity: 1,
        },
        Effect::GiveItem {
            item_id: MANA_ELIXIR_PILL,
            quantity: 50,
        },
        Effect::GainExp(8000),
        Effect::CompleteQuest(BLOCKTOPUS_ALIEN),
    ];
    script.inventory_full = Some(text(keys::DR_KIM_FULL));
    script
}

/// Gathering Up the Lacking Ingredients (3833): Mr. Ku (2092000) in the
/// Herb Town takes 100-Year-Old Bellflowers (4000294).
const MR_KU: i32 = 2092000;
const LACKING_INGREDIENTS: i16 = 3833;
const BELLFLOWER: i32 = 4000294;
/// What Mr. Ku gives for the most of these Bellflowers the player holds:
/// (Bellflowers taken, items given, EXP), the most first (Cosmic
/// `3833.js`).
const BELLFLOWER_REWARDS: [(i16, &[(i32, i16)], i32); 6] = [
    (1000, &[(2040501, 1), (2000005, 50)], 54000),
    (600, &[(2020013, 50)], 54000),
    (500, &[], 54000),
    (100, &[], 45000),
    (50, &[(2020007, 50)], 10000),
    (1, &[(2000000, 1)], 10),
];

/// `q3833e`, Gathering Up the Lacking Ingredients: Mr. Ku takes the most
/// Bellflowers of a tier the player holds (1,000, 600, 500, 100, 50 or 1)
/// and gives that tier's items and EXP (`BELLFLOWER_REWARDS`). Without the
/// USE slots Mr. Ku says so. The text is unsourced.
fn mr_kus_bellflowers(player: &Player) -> NpcScript {
    let held = player.inventory.count(BELLFLOWER);
    let mut script = NpcScript::new(MR_KU, vec![Page::Say(text(keys::MR_KU_THANKS))]);
    let Some(&(taken, items, exp)) = BELLFLOWER_REWARDS
        .iter()
        .find(|(taken, ..)| held >= i32::from(*taken))
    else {
        return script;
    };
    script.on_end = vec![Effect::TakeItem {
        item_id: BELLFLOWER,
        quantity: taken,
    }];
    script.on_end.extend(
        items
            .iter()
            .map(|&(item_id, quantity)| Effect::GiveItem { item_id, quantity }),
    );
    script.on_end.extend([
        Effect::GainExp(exp),
        Effect::CompleteQuest(LACKING_INGREDIENTS),
    ]);
    script.inventory_full = Some(text(keys::MR_KU_FULL));
    script
}

/// Convince Muhamad (3953): Muhamad (2100001) in Ariant.
const CONVINCE_MUHAMAD: i16 = 3953;
const LIDIUM: i32 = 4011008;

/// `q3953e`, Convince Muhamad: four times Muhamad speaks and the player
/// has one thing to answer; then Muhamad is convinced, the Lidium the end
/// asks for is taken and 20,000 EXP given, which a script's completion
/// doesn't apply (Cosmic `3953.js`). The text is unsourced.
fn convince_muhamad() -> NpcScript {
    let mut pages = vec![Page::Act {
        text: text(keys::MUHAMAD_CONVINCED),
        effects: vec![
            Effect::CompleteQuest(CONVINCE_MUHAMAD),
            Effect::TakeItem {
                item_id: LIDIUM,
                quantity: 1,
            },
            Effect::GainExp(20000),
        ],
    }];
    for (says, answer) in [
        (keys::MUHAMAD_DEO_PEACEFUL, keys::MUHAMAD_MERCHANTS),
        (keys::MUHAMAD_TRESPASSED, keys::MUHAMAD_QUEEN),
        (keys::MUHAMAD_GUARDIANS, keys::MUHAMAD_GUARDIANS_WHERE),
        (keys::MUHAMAD_EXPEDITION, keys::MUHAMAD_DEO_MONSTER),
    ]
    .into_iter()
    .rev()
    {
        pages = vec![Page::Menu {
            text: text(says),
            choices: vec![(text(answer), pages)],
        }];
    }
    NpcScript::new(MUHAMAD, pages)
}

const SORCERER: i32 = 2140002;
const SELLS_EMOTIONS: i16 = 3514;

/// `q3514e`, The Sorcerer Who Sells Emotions: once the potion is gone,
/// which the data's end checks, the Sorcerer sees it worked and gives the
/// end's 891,500 EXP, which a script's completion doesn't apply (Cosmic
/// `3514.js`). The text is unsourced.
fn the_sorcerers_potion() -> NpcScript {
    let mut script = NpcScript::new(
        SORCERER,
        vec![Page::Say(text(keys::SORCERER_POTION_WORKED))],
    );
    script.on_end = vec![
        Effect::GainExp(891_500),
        Effect::CompleteQuest(SELLS_EMOTIONS),
    ];
    script
}

/// A Supply from the Sand Crew (2124, and 2126 for the other way in):
/// Eleska (2101002) in Ariant takes the box Moppie (2012019) gave, which
/// the data's end only checks for (Cosmic `2124.js`, whose `2126.js`
/// takes 2124's box instead of the data's). The text is unsourced.
const ELESKA: i32 = 2101002;
const SAND_CREW_SUPPLY: i16 = 2124;
const SAND_CREW_SUPPLY_AGAIN: i16 = 2126;
const MOPPIES_BOX: i32 = 4031619;
const MOPPIES_BOX_AGAIN: i32 = 4031624;

fn moppies_box(quest: i16, box_id: i32) -> NpcScript {
    let mut script = NpcScript::new(ELESKA, vec![Page::Say(text(keys::ELESKA_BOX_THANKS))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: box_id,
            quantity: 1,
        },
        Effect::CompleteQuest(quest),
    ];
    script
}

/// To the Desert... (2127): Manji (1022002) in Perion tells the player the
/// mission is on and completes the quest, which opens A Special Assignment
/// (2123) (Cosmic `2127.js`). The text is unsourced.
const MANJI: i32 = 1022002;
const TO_THE_DESERT: i16 = 2127;

fn to_the_desert() -> NpcScript {
    let mut script = NpcScript::new(MANJI, vec![Page::Say(text(keys::MANJI_DESERT_MISSION))]);
    script.on_end = vec![Effect::CompleteQuest(TO_THE_DESERT)];
    script
}

/// Truth of the Rumor (2148 to 2152): while Winston's Truth of the Rumor
/// (2144) is under way, each of five people in Perion and Ellinia tells
/// the player one thing about the old tree; the start script completes its
/// quest as it starts, with the data's EXP, and 2144 asks for all five
/// (Cosmic `2148.js` to `2152.js`). The text is unsourced.
const RUMORS: [(i16, i32, &str, i32); 5] = [
    (2148, 1020000, keys::RUMOR_BLACKBULL, 10),
    (2149, MANJI, keys::RUMOR_MANJI, 10),
    (2150, 1022007, keys::RUMOR_AYAN, 10),
    (2151, 1022000, keys::RUMOR_DANCES_WITH_BALROG, 10),
    (2152, 1032104, keys::RUMOR_BETTY, 15),
];

/// One teller of the rumor: what they know, and the quest done.
fn truth_of_the_rumor(quest: i16) -> Option<NpcScript> {
    let &(_, npc, words, exp) = RUMORS.iter().find(|r| r.0 == quest)?;
    let mut script = NpcScript::new(npc, vec![Page::Say(text(words))]);
    script.on_end = vec![Effect::FinishQuest(quest), Effect::GainExp(exp)];
    Some(script)
}

/// The start scripts of New Leaf City's Masteria quests: John Barricade's
/// The Mark of Heroism (8221) at the Mushroom Marketplace, Lukan's Storming
/// the Castle (8223) and Taggrin's three Raven Ninja Clan tasks (8224 to
/// 8226) in the Phantom Forest, and Lita Lawless's bounties in New Leaf
/// City (8231 to 8238, every second one repeatable). Each asks whether the
/// player is in; a yes says what to bring or hunt and starts the quest, and
/// the data's end checks and rewards it (Cosmic `8221.js` to `8238.js`).
/// John's Finding Jack (8219), Lukan's The Brewing Storm (8222) and Jack's
/// Stemming the Tide (8230) start the same way but end in scripts. The text
/// is unsourced.
const NEW_LEAF_CITY_STARTS: [(i16, i32, &str, &str); 16] = [
    (
        8219,
        JOHN_BARRICADE,
        keys::BARRICADE_JACK_ASK,
        keys::BARRICADE_JACK_GO,
    ),
    (
        8221,
        JOHN_BARRICADE,
        keys::BARRICADE_MARK_ASK,
        keys::BARRICADE_MARK_GO,
    ),
    (8222, LUKAN, keys::LUKAN_BADGES_ASK, keys::LUKAN_BADGES_GO),
    (8223, LUKAN, keys::LUKAN_STORM_ASK, keys::LUKAN_STORM_GO),
    (
        8224,
        TAGGRIN,
        keys::TAGGRIN_WOODS_ASK,
        keys::TAGGRIN_WOODS_GO,
    ),
    (8225, TAGGRIN, keys::TAGGRIN_PATH_ASK, keys::TAGGRIN_PATH_GO),
    (
        8226,
        TAGGRIN,
        keys::TAGGRIN_WARRIORS_ASK,
        keys::TAGGRIN_WARRIORS_GO,
    ),
    (8230, JACK, keys::JACK_TIDE_ASK, keys::JACK_TIDE_GO),
    (
        8231,
        LITA_LAWLESS,
        keys::LITA_LEPRECHAUNS_ASK,
        keys::LITA_BOUNTY_GO,
    ),
    (
        8232,
        LITA_LAWLESS,
        keys::LITA_LEPRECHAUNS_ASK,
        keys::LITA_BOUNTY_GO,
    ),
    (
        8233,
        LITA_LAWLESS,
        keys::LITA_ELDERWRAITHS_ASK,
        keys::LITA_BOUNTY_GO,
    ),
    (
        8234,
        LITA_LAWLESS,
        keys::LITA_ELDERWRAITHS_ASK,
        keys::LITA_BOUNTY_GO,
    ),
    (
        8235,
        LITA_LAWLESS,
        keys::LITA_HORSEMAN_ASK,
        keys::LITA_BOUNTY_GO,
    ),
    (
        8236,
        LITA_LAWLESS,
        keys::LITA_HORSEMAN_ASK,
        keys::LITA_BOUNTY_GO,
    ),
    (
        8237,
        LITA_LAWLESS,
        keys::LITA_BIGFOOT_ASK,
        keys::LITA_BOUNTY_GO,
    ),
    (
        8238,
        LITA_LAWLESS,
        keys::LITA_BIGFOOT_ASK,
        keys::LITA_BOUNTY_GO,
    ),
];
const JOHN_BARRICADE: i32 = 9201051;
const LUKAN: i32 = 9201098;
const TAGGRIN: i32 = 9201100;
const LITA_LAWLESS: i32 = 9201054;
const JACK: i32 = 9201096;
const ELPAM_GORLAB: i32 = 9201055;
const STORMBREAKER_BADGE: i32 = 4032006;
const PHANTOM_FOREST_MAP: i32 = 3992040;
const CRIMSONWOOD_KEYSTONE: i32 = 3992041;
const CODED_COMMUNIQUE: i32 = 4032032;
const TRANSLATED_COMMUNIQUE: i32 = 4032018;

/// `script`, if it is one of New Leaf City's start scripts.
fn new_leaf_city_start(script: &str) -> Option<NpcScript> {
    let &(quest, npc, ask, go) = NEW_LEAF_CITY_STARTS
        .iter()
        .find(|start| script == format!("q{}s", start.0))?;
    let mut script = NpcScript::new(
        npc,
        vec![
            Page::Ask {
                text: text(ask),
                on_yes: Vec::new(),
                on_no: Some(text(keys::NEW_LEAF_CITY_DECLINE)),
            },
            Page::Say(text(go)),
        ],
    );
    script.on_end = vec![Effect::StartQuest(quest)];
    Some(script)
}

/// One of `item_id`.
fn give_one(item_id: i32) -> Effect {
    Effect::GiveItem {
        item_id,
        quantity: 1,
    }
}

/// `q8219e`, Finding Jack: Jack, past the Phantom Forest, greets the
/// player John sent and gives the Map of the Phantom Forest (3992040) and
/// 175,000 EXP (Cosmic `8219.js`).
fn jack_meets_the_player() -> NpcScript {
    let mut script = NpcScript::new(
        JACK,
        vec![
            Page::Say(text(keys::JACK_BROTHER)),
            Page::Say(text(keys::JACK_MAP)),
        ],
    );
    script.on_end = vec![
        give_one(PHANTOM_FOREST_MAP),
        Effect::GainExp(175_000),
        Effect::CompleteQuest(8219),
    ];
    script.inventory_full = Some(text(keys::JACK_SETUP_FULL));
    script
}

/// `q8222e`, The Brewing Storm: Lukan takes the ten Stormbreaker Badges
/// (4032006) the data's end asks for. There is no Cosmic script and the
/// data gives nothing; 8221 and 8223 ask for this quest done.
fn lukans_badges(player: &Player) -> NpcScript {
    if player.inventory.count(STORMBREAKER_BADGE) < 10 {
        return NpcScript::new(LUKAN, vec![Page::Say(text(keys::LUKAN_BADGES_MISSING))]);
    }
    let mut script = NpcScript::new(LUKAN, vec![Page::Say(text(keys::LUKAN_BADGES_THANKS))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: STORMBREAKER_BADGE,
            quantity: 10,
        },
        Effect::CompleteQuest(8222),
    ];
    script
}

/// Lost in Translation (8227 to 8229): Jack swipes a Coded Communique
/// (4032032) from the Keep for John (8227, whose end is the data's); John
/// can't read it and sends it to Elpam Gorlab, the outsider (8228, given
/// again if it was lost), who takes it; John then hands the Translated
/// Communique (4032018) over for Jack, who takes it for 50,000 EXP (8229).
/// A no ends each start (Cosmic `8227.js` to `8229.js`). The text is
/// unsourced.
fn lost_in_translation_start(player: &Player, quest: i16) -> NpcScript {
    let (npc, ask, go, letter) = match quest {
        8227 => (
            JACK,
            keys::JACK_LETTER_ASK,
            Some(keys::JACK_LETTER_GO),
            CODED_COMMUNIQUE,
        ),
        8228 => (
            JOHN_BARRICADE,
            keys::BARRICADE_GLYPHS_ASK,
            Some(keys::BARRICADE_GLYPHS_GO),
            CODED_COMMUNIQUE,
        ),
        _ => (
            JOHN_BARRICADE,
            keys::BARRICADE_TRANSLATED_ASK,
            None,
            TRANSLATED_COMMUNIQUE,
        ),
    };
    let mut pages = vec![Page::Ask {
        text: text(ask),
        on_yes: Vec::new(),
        on_no: Some(text(keys::NEW_LEAF_CITY_NEEDS_YOU)),
    }];
    pages.extend(go.map(|go| Page::Say(text(go))));
    let mut script = NpcScript::new(npc, pages);
    if player.inventory.count(letter) == 0 {
        script.on_end.push(give_one(letter));
    }
    script.on_end.push(Effect::StartQuest(quest));
    script.inventory_full = Some(text(keys::NEW_LEAF_CITY_ETC_FULL));
    script
}

/// `q8228e`: Elpam Gorlab takes the Coded Communique to translate.
fn elpam_translates(player: &Player) -> NpcScript {
    if player.inventory.count(CODED_COMMUNIQUE) == 0 {
        return NpcScript::new(ELPAM_GORLAB, vec![Page::Say(text(keys::ELPAM_NO_LETTER))]);
    }
    let mut script = NpcScript::new(ELPAM_GORLAB, vec![Page::Say(text(keys::ELPAM_TRANSLATE))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: CODED_COMMUNIQUE,
            quantity: 1,
        },
        Effect::CompleteQuest(8228),
    ];
    script
}

/// `q8229e`: Jack takes the Translated Communique for 50,000 EXP.
fn jack_reads_the_translation(player: &Player) -> NpcScript {
    if player.inventory.count(TRANSLATED_COMMUNIQUE) == 0 {
        return NpcScript::new(JACK, vec![Page::Say(text(keys::JACK_NO_TRANSLATION))]);
    }
    let mut script = NpcScript::new(JACK, vec![Page::Say(text(keys::JACK_TRANSLATION))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: TRANSLATED_COMMUNIQUE,
            quantity: 1,
        },
        Effect::GainExp(50_000),
        Effect::CompleteQuest(8229),
    ];
    script
}

/// `q8230e`, Stemming the Tide: with the Crimsonwood Keystone (3992041)
/// that Storming the Castle (8223) gives, Jack says it opens the Inner
/// Sanctum and completes the quest, leaving the key with the player;
/// without it, Jack says where to turn (Cosmic `8230.js`).
fn jack_sees_the_keystone(player: &Player) -> NpcScript {
    if player.inventory.count(CRIMSONWOOD_KEYSTONE) > 0 {
        let mut script = NpcScript::new(JACK, vec![Page::Say(text(keys::JACK_KEYSTONE))]);
        script.on_end = vec![Effect::CompleteQuest(8230)];
        return script;
    }
    let words = if player.quests.state(8223) == QuestState::Completed {
        keys::JACK_KEYSTONE_LOST
    } else {
        keys::JACK_HURRY
    };
    NpcScript::new(JACK, vec![Page::Say(text(words))])
}

/// The Adventurer titles (29900 to 29903): quest, title and medal. Each
/// starts by itself (`normalAutoStart`) once an Explorer reaches its job
/// and level without the medal, and Dalair (9000040) gives the medal.
const ADVENTURER_TITLES: [(i16, &str, i32); 4] = [
    (29900, "Beginner Adventurer", 1142107),
    (29901, "Junior Adventurer", 1142108),
    (29902, "Veteran Adventurer", 1142109),
    (29903, "Master Adventurer", 1142110),
];
const DALAIR: i32 = 9000040;

/// The Adventurer title whose start or end script is `script`.
fn adventurer(script: &str) -> Option<(i16, &'static str, i32)> {
    ADVENTURER_TITLES
        .iter()
        .find(|title| script[1..].starts_with(&title.0.to_string()))
        .copied()
}

/// `q29900s` to `q29903s`: the title is earned. The quest starts with no
/// dialogue and a chat-log line names it (Cosmic `29900.js` to
/// `29903.js`). The text is unsourced.
fn adventurer_title(script: &str) -> Option<NpcScript> {
    let (quest, title, _) = adventurer(script)?;
    let line = content::text().get(keys::ADVENTURER_TITLE, &[("title", title)]);
    Some(NpcScript::new(
        DALAIR,
        vec![Page::Done(vec![
            Effect::StartQuest(quest),
            Effect::InfoText(line),
        ])],
    ))
}

/// `q29900e` to `q29903e`: Dalair gives the title's medal and completes
/// the quest. The text is unsourced.
fn adventurer_medal(script: &str) -> Option<NpcScript> {
    let (quest, title, medal) = adventurer(script)?;
    let medal_id = medal.to_string();
    let says = content::text().get(
        keys::ADVENTURER_MEDAL,
        &[("title", title), ("medal", &medal_id)],
    );
    let mut script = NpcScript::new(DALAIR, vec![Page::Say(says)]);
    script.on_end = vec![give_one(medal), Effect::CompleteQuest(quest)];
    script.inventory_full = Some(text(keys::ADVENTURER_MEDAL_FULL));
    Some(script)
}

/// The Knight titles (29906 to 29909): quest and title. Each starts by
/// itself (`normalAutoStart`) once a Cygnus Knight reaches its job and
/// level, and Cygnus (1101000) completes it with the medal
/// (`game_data::quests::KNIGHT_TITLES`).
const KNIGHT_TITLES: [(i16, &str); 4] = [
    (29906, "Knight-in-Training"),
    (29907, "Official Knight"),
    (29908, "Advanced Knight"),
    (29909, "Chief Knight"),
];
const CYGNUS: i32 = 1101000;

/// `q29906s` to `q29909s`: the title is earned. The quest starts with no
/// dialogue and a chat-log line names it, as for the Adventurer titles
/// (Cosmic `29906.js` to `29909.js`). The text is unsourced.
fn knight_title(script: &str) -> Option<NpcScript> {
    let &(quest, title) = KNIGHT_TITLES
        .iter()
        .find(|(quest, _)| script[1..].starts_with(&quest.to_string()))?;
    let line = content::text().get(keys::KNIGHT_TITLE, &[("title", title)]);
    Some(NpcScript::new(
        CYGNUS,
        vec![Page::Done(vec![
            Effect::StartQuest(quest),
            Effect::InfoText(line),
        ])],
    ))
}

/// The titles of medals won elsewhere (29910 to 29928 and 29933): each
/// quest's data starts it by itself (`autoStart`, `autoAccept`) once the
/// player holds the medal, and its start script starts and completes it at
/// once with a line in the chat log naming the title, as Cosmic's
/// `29924.js` does for the Aran titles. Quest, title, and the NPC the data
/// names.
const MEDAL_TITLES: [(i16, &str, i32); 20] = [
    (29910, "Gallant Warrior", 9000066),
    (29911, "Wiseman", 9000066),
    (29912, "Lord Sniper", 9000066),
    (29913, "Legendary Thief", 9000066),
    (29914, "King Pirate", 9000066),
    (29915, "Mu Lung Dojo Vanquisher", 2091005),
    (29916, "Henesys Donor", 9000066),
    (29917, "Ellinia Donor", 9000066),
    (29918, "Perion Donor", 9000066),
    (29919, "Kerning City Donor", 9000066),
    (29920, "Sleepywood Donor", 9000066),
    (29921, "Nautilus Donor", 9000066),
    (29933, "Lith Harbor Donor", 9000066),
    (29922, "Veteran Hunter", 9000066),
    (29923, "Legendary Hunter", 9000066),
    (29924, "The Revived Aran", 9000066),
    (29925, "Aran and Memory", 9000066),
    (29926, "Aran in Agony", 9000066),
    (29927, "Aran of Hope", 9000066),
    (29928, "Aran the Hero", 9000066),
];

fn medal_title(script: &str) -> Option<NpcScript> {
    let &(quest, title, npc) = MEDAL_TITLES
        .iter()
        .find(|(quest, ..)| script[1..].starts_with(&quest.to_string()))?;
    let line = content::text().get(keys::MEDAL_TITLE, &[("title", title)]);
    Some(NpcScript::new(
        npc,
        vec![Page::Done(vec![
            Effect::FinishQuest(quest),
            Effect::InfoText(line),
        ])],
    ))
}

/// Title Challenge - Outstanding Citizen (29508): the data starts it at
/// Dalair from level 10, and Dalair's end script gives the Outstanding
/// Citizen Medal (the quest's `viewMedalItem`) to a player who is married,
/// in a guild, and has a Junior, as its `demandSummary` asks. Cosmic has no
/// script; the text is unsourced.
const OUTSTANDING_CITIZEN: i16 = 29508;
const OUTSTANDING_CITIZEN_MEDAL: i32 = 1142081;

fn outstanding_citizen(player: &Player, family: &FamilyInfo) -> NpcScript {
    let married = player.marriage.as_ref().is_some_and(|m| m.married());
    if !married || player.guild.is_none() || family.juniors == 0 {
        return NpcScript::new(
            DALAIR,
            vec![Page::Say(text(keys::OUTSTANDING_CITIZEN_NOT_YET))],
        );
    }
    let medal_id = OUTSTANDING_CITIZEN_MEDAL.to_string();
    let says = content::text().get(
        keys::ADVENTURER_MEDAL,
        &[("title", "Outstanding Citizen"), ("medal", &medal_id)],
    );
    let mut script = NpcScript::new(DALAIR, vec![Page::Say(says)]);
    script.on_end = vec![
        give_one(OUTSTANDING_CITIZEN_MEDAL),
        Effect::CompleteQuest(OUTSTANDING_CITIZEN),
    ];
    script.inventory_full = Some(text(keys::ADVENTURER_MEDAL_FULL));
    script
}

/// Title Challenge - Celebrity (29002): Dalair (9000040) sets the player
/// to raise their fame by 1,000 in the quest's 30 days (`timeLimit2`). The
/// start keeps the fame it began from and the goal in the quest's record
/// (`popS`, `popG`), which the Quest Log's `#jpopgap#`, `#jpopG#` and gauge
/// read, and Dalair's end script gives the Celebrity Medal (1142003, the
/// quest's `viewMedalItem`) once the fame gained reaches the goal before
/// the end date. Cosmic has no scripts for it, and the text is unsourced.
const CELEBRITY: i16 = 29002;
const CELEBRITY_MEDAL: i32 = 1142003;
const CELEBRITY_FAME_GOAL: i16 = 1000;

fn celebrity_start(player: &Player) -> NpcScript {
    let mut script = NpcScript::new(DALAIR, vec![Page::Say(text(keys::CELEBRITY_START))]);
    script.on_end = vec![
        Effect::StartQuest(CELEBRITY),
        Effect::SetQuestEx {
            quest: CELEBRITY,
            key: "popS",
            value: player.stats.fame.to_string(),
        },
        Effect::SetQuestEx {
            quest: CELEBRITY,
            key: "popG",
            value: CELEBRITY_FAME_GOAL.to_string(),
        },
    ];
    script
}

fn celebrity_end(player: &Player, now: SystemTime) -> NpcScript {
    if crate::quests::title_ended(player, CELEBRITY, now) {
        return NpcScript::new(DALAIR, vec![Page::Say(text(keys::CELEBRITY_TOO_LATE))]);
    }
    let started_at = player
        .quest_ex
        .get(CELEBRITY, "popS")
        .and_then(|v| v.parse().ok());
    let gained = player.stats.fame as i32 - started_at.unwrap_or(0i32);
    let goal = i32::from(CELEBRITY_FAME_GOAL);
    if gained < goal {
        let says = content::text().get(
            keys::CELEBRITY_NOT_YET,
            &[("gained", &gained.to_string()), ("goal", &goal.to_string())],
        );
        return NpcScript::new(DALAIR, vec![Page::Say(says)]);
    }
    let medal_id = CELEBRITY_MEDAL.to_string();
    let says = content::text().get(
        keys::ADVENTURER_MEDAL,
        &[("title", "Celebrity"), ("medal", &medal_id)],
    );
    let mut script = NpcScript::new(DALAIR, vec![Page::Say(says)]);
    script.on_end = vec![give_one(CELEBRITY_MEDAL), Effect::CompleteQuest(CELEBRITY)];
    script.inventory_full = Some(text(keys::ADVENTURER_MEDAL_FULL));
    script
}

/// Title Challenge - Veteran Hunter (29400): Dalair (9000040) sets a hunt
/// of a million mobs above the player's level in 30 days. The start script
/// writes the count (`mon`) and the goal (`mg`) into the quest's record
/// beside the quest log, each kill that counts adds one
/// (`quests::count_hunt`), and the end script gives the Veteran Hunter Medal
/// (1142004) once the count reaches the goal before the end date.
const VETERAN_HUNTER_MEDAL: i32 = 1142004;
const VETERAN_HUNTER_GOAL: u32 = 1_000_000;

fn veteran_hunter_start() -> NpcScript {
    use crate::quests::VETERAN_HUNTER;
    let mut script = NpcScript::new(DALAIR, vec![Page::Say(text(keys::VETERAN_HUNTER_START))]);
    script.on_end = vec![
        Effect::StartQuest(VETERAN_HUNTER),
        Effect::SetQuestEx {
            quest: VETERAN_HUNTER,
            key: "mon",
            value: "0".to_string(),
        },
        Effect::SetQuestEx {
            quest: VETERAN_HUNTER,
            key: "mg",
            value: VETERAN_HUNTER_GOAL.to_string(),
        },
    ];
    script
}

fn veteran_hunter_end(player: &Player, now: SystemTime) -> NpcScript {
    use crate::quests::VETERAN_HUNTER;
    if crate::quests::title_ended(player, VETERAN_HUNTER, now) {
        return NpcScript::new(DALAIR, vec![Page::Say(text(keys::VETERAN_HUNTER_TOO_LATE))]);
    }
    let hunted = player.quest_ex.number(VETERAN_HUNTER, "mon");
    if hunted < VETERAN_HUNTER_GOAL {
        let says = content::text().get(
            keys::VETERAN_HUNTER_NOT_YET,
            &[
                ("hunted", &hunted.to_string()),
                ("goal", &VETERAN_HUNTER_GOAL.to_string()),
            ],
        );
        return NpcScript::new(DALAIR, vec![Page::Say(says)]);
    }
    let medal_id = VETERAN_HUNTER_MEDAL.to_string();
    let says = content::text().get(
        keys::ADVENTURER_MEDAL,
        &[("title", "Veteran Hunter"), ("medal", &medal_id)],
    );
    let mut script = NpcScript::new(DALAIR, vec![Page::Say(says)]);
    script.on_end = vec![
        give_one(VETERAN_HUNTER_MEDAL),
        Effect::CompleteQuest(VETERAN_HUNTER),
    ];
    script.inventory_full = Some(text(keys::ADVENTURER_MEDAL_FULL));
    script
}

/// Tienk, the Monster Book Salesman (2197): Tienk (2006) in Lith Harbor
/// greets a new Beginner, whose end script completes the quest with nothing
/// given (Cosmic `2197.js`). The text is unsourced.
const TIENK: i32 = 2006;
const TIENKS_GREETING: i16 = 2197;

fn tienks_greeting() -> NpcScript {
    let mut script = NpcScript::new(TIENK, vec![Page::Say(text(keys::TIENK_GREETING))]);
    script.on_end = vec![Effect::CompleteQuest(TIENKS_GREETING)];
    script
}

/// The Last Song (2293), after Say "NO" to Plagiarism (2288): Maestro Rho
/// (1052120) in the Kerning City subway asks about the last song the Spirit
/// of Rock played, and the start script starts the quest. The data's end
/// asks for nothing and gives nothing; Cosmic's guessing game at the end
/// has no end script in the data to run from. The text is unsourced.
const MAESTRO_RHO: i32 = 1052120;
const THE_LAST_SONG: i16 = 2293;

fn the_last_song() -> NpcScript {
    let mut script = NpcScript::new(MAESTRO_RHO, vec![Page::Say(text(keys::RHO_LAST_SONG))]);
    script.on_end = vec![Effect::StartQuest(THE_LAST_SONG)];
    script
}

/// Help Me Find My Glasses (2186): Abel (1094001) in Nautilus Harbor lost
/// his glasses in the bushes there (1094002 to 1094006). While the quest is
/// under way, a bush searched with no glasses held gives one of three
/// pairs at even odds: Abel's own (4031853), which the data's end checks
/// for, or one of two others (4031854, 4031855). Abel's end script takes
/// his own for 1,700 EXP and ten Nautilus Return Scrolls (2030019); shown
/// another pair, he takes it instead for 1,000 EXP and five scrolls and
/// completes the quest all the same (Cosmic `2186.js` and the bushes'
/// scripts). The text is unsourced.
const ABEL: i32 = 1094001;
const FIND_MY_GLASSES: i16 = 2186;
const NAUTILUS_BUSHES: std::ops::RangeInclusive<i32> = 1094002..=1094006;
const ABELS_GLASSES: i32 = 4031853;
const OTHER_GLASSES: [i32; 2] = [4031854, 4031855];
const NAUTILUS_RETURN: i32 = 2030019;

/// A bush: a pair of glasses, unless one is already held.
fn nautilus_bush(npc: i32, player: &Player, rng: &mut impl Rng) -> NpcScript {
    let held = std::iter::once(ABELS_GLASSES)
        .chain(OTHER_GLASSES)
        .any(|glasses| player.inventory.count(glasses) > 0);
    if held {
        return NpcScript::new(npc, vec![Page::Say(text(keys::NAUTILUS_BUSH_SEARCHED))]);
    }
    let found = match rng.gen_range(0, 3) {
        0 => ABELS_GLASSES,
        n => OTHER_GLASSES[n - 1],
    };
    let mut script = NpcScript::new(npc, vec![Page::Say(text(keys::NAUTILUS_BUSH_SEARCH))]);
    script.on_end = vec![Effect::GiveItem {
        item_id: found,
        quantity: 1,
    }];
    script.inventory_full = Some(text(keys::NAUTILUS_BUSH_FULL));
    script
}

/// Abel's thanks for `glasses`: `exp` and `scrolls` Nautilus Return Scrolls,
/// and Abel plays the data's `npcAct` ("quest").
fn abels_thanks(words: &str, glasses: i32, exp: i32, scrolls: i16) -> NpcScript {
    let mut script = NpcScript::new(ABEL, vec![Page::Say(text(words))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: glasses,
            quantity: 1,
        },
        Effect::GainExp(exp),
        Effect::GiveItem {
            item_id: NAUTILUS_RETURN,
            quantity: scrolls,
        },
        Effect::CompleteQuest(FIND_MY_GLASSES),
        Effect::NpcSpecialAction {
            npc_id: ABEL,
            action: "quest",
        },
    ];
    script.inventory_full = Some(text(keys::NAUTILUS_ABEL_FULL));
    script
}

/// `q2186e`: Abel's own glasses back.
fn abels_glasses_found() -> NpcScript {
    abels_thanks(keys::NAUTILUS_ABEL_FOUND, ABELS_GLASSES, 1700, 10)
}

/// Abel, clicked with another pair and not his own while the quest is
/// under way.
fn abel_takes_other_glasses(player: &Player) -> Option<NpcScript> {
    if player.quests.state(FIND_MY_GLASSES) != QuestState::Started
        || player.inventory.count(ABELS_GLASSES) > 0
    {
        return None;
    }
    let other = OTHER_GLASSES
        .into_iter()
        .find(|&glasses| player.inventory.count(glasses) > 0)?;
    Some(abels_thanks(keys::NAUTILUS_ABEL_OTHER, other, 1000, 5))
}

/// The Endangered Lives of Grays (3455): Dr. Kim's Wave Translator
/// (3454, repeatable) comes out working (4031928, for 3456) or faulty
/// (4031927, for this one) at even odds. Held up to the Alien Gray
/// (2050002), the faulty one gives only noise and is used up; the quest
/// starts, ends at the Gray with nothing, and can be taken again. No
/// script survives; the text is unsourced.
const ALIEN_GRAY: i32 = 2050002;
const GRAYS_GARBLED: i16 = 3455;
const FAULTY_TRANSLATOR: i32 = 4031927;

fn the_faulty_translator() -> NpcScript {
    let mut script = NpcScript::new(ALIEN_GRAY, vec![Page::Say(text(keys::GRAY_GARBLED))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: FAULTY_TRANSLATOR,
            quantity: 1,
        },
        Effect::StartQuest(GRAYS_GARBLED),
    ];
    script
}

const ARDIN: i32 = 2101003;
const ARDINS_SAND_BANDITS: i16 = 3933;

/// `q3933s`, Ardin's Sand Bandits: Ardin asks for one more test, and on
/// yes, with nobody in the room, starts the quest and sends the player to
/// face the other self Ardin summons there (Cosmic `3933.js`); the room
/// fills again once empty. The text is unsourced.
fn ardins_sand_bandits() -> NpcScript {
    let mut script = NpcScript::new(
        ARDIN,
        vec![
            Page::Say(text(keys::ARDIN_TEST_AGAIN)),
            Page::Ask {
                text: text(keys::ARDIN_TEST_ASK),
                on_yes: Vec::new(),
                on_no: None,
            },
            Page::Say(text(keys::ARDIN_TEST_GO)),
        ],
    );
    script.on_end = vec![
        Effect::RequireEmptyMap(crate::scripts::ARDINS_ROOM),
        Effect::StartQuest(ARDINS_SAND_BANDITS),
        Effect::WarpToPortal(crate::scripts::ARDINS_ROOM, 1),
    ];
    script.refused = Some(text(keys::ARDIN_TEST_BUSY));
    script
}

/// Endangered Mushking Empire (2300-2304): an Explorer's job instructor
/// writes a recommendation letter (4032375) to the Head Security Officer
/// (1300005) in the Mushroom Forest Field. (quest, instructor), one per
/// branch; 2305-2310, the same for the Cygnus Knights, are left out with
/// them.
const KINGDOM_LETTERS: [(i16, i32); 5] = [
    (2300, 1022000),
    (2301, 1032001),
    (2302, 1052001),
    (2303, 1012100),
    (2304, KYRIN),
];
const KINGDOM_LETTER: i32 = 4032375;
const HEAD_SECURITY_OFFICER: i32 = 1300005;
/// The kingdom's opening scene (`TD_MC_Openning`), where an instructor
/// can send the player straight away.
const MUSHROOM_OPENING: i32 = 106020001;

/// The quest a `q23xxs`/`q23xxe` script belongs to, and its instructor.
fn kingdom_letter(script: &str) -> Option<(i16, i32)> {
    let quest: i16 = script.get(1..5)?.parse().ok()?;
    KINGDOM_LETTERS.iter().copied().find(|&(q, _)| q == quest)
}

/// `q2300s`-`q2304s` (all Edelstein's `q2300s`): once the player accepts,
/// the instructor tells of the kingdom's trouble and hands over the
/// letter, which starts the quest, then offers to send the player to the
/// kingdom at once. Without a free ETC slot the instructor says so and
/// nothing is given.
fn letter_for_the_kingdom(script: &str, player: &Player) -> Option<NpcScript> {
    let (quest, instructor) = kingdom_letter(script)?;
    let accept = Page::Ask {
        text: text(keys::MC_LETTER_ASK),
        on_yes: Vec::new(),
        on_no: Some(text(keys::MC_LETTER_LATER)),
    };
    if player.inventory.free(InventoryType::Etc) == 0 {
        return Some(NpcScript::new(
            instructor,
            vec![accept, Page::Say(text(keys::MC_LETTER_FULL))],
        ));
    }
    // One still holding a letter, from a quest given up, keeps it.
    let mut letter = Vec::new();
    if player.inventory.count(KINGDOM_LETTER) == 0 {
        letter.push(Effect::GiveItem {
            item_id: KINGDOM_LETTER,
            quantity: 1,
        });
    }
    letter.push(Effect::StartQuest(quest));
    let mut script = NpcScript::new(
        instructor,
        vec![
            accept,
            Page::Say(text(keys::MC_LETTER_KINGDOM)),
            Page::Act {
                text: text(keys::MC_LETTER_WRITTEN),
                effects: letter,
            },
            Page::Decide {
                text: text(keys::MC_LETTER_TAKE_YOU),
                on_yes: vec![Page::Done(vec![Effect::Warp(MUSHROOM_OPENING)])],
                on_no: vec![Page::Say(text(keys::MC_LETTER_ROUTE))],
            },
        ],
    );
    script.inventory_full = Some(text(keys::MC_LETTER_FULL));
    Some(script)
}

/// `q2300e`-`q2304e` (Edelstein's `q2300e`): the Head Security Officer
/// reads the letter, the player answers, and he welcomes them, taking the
/// letter for the quest and 1,200 EXP (the data has no rewards). Without
/// the letter nothing is applied.
fn letter_read(script: &str) -> Option<NpcScript> {
    let (quest, _) = kingdom_letter(script)?;
    let mut script = NpcScript::new(
        HEAD_SECURITY_OFFICER,
        vec![
            Page::Say(text(keys::MC_OFFICER_LETTER)),
            Page::Reply(text(keys::MC_OFFICER_LETTER_REPLY)),
            Page::Say(text(keys::MC_OFFICER_WELCOME)),
        ],
    );
    script.on_end = vec![
        Effect::TakeItem {
            item_id: KINGDOM_LETTER,
            quantity: 1,
        },
        Effect::CompleteQuest(quest),
        Effect::GainExp(1200),
    ];
    Some(script)
}

/// The Mushking Empire's Secretary of Domestic Affairs (1300003), and the
/// two quests he sends the player to look into: Exploring Mushroom Forest
/// (1) and Over the Castle Walls (2). Their `investigate1` and
/// `investigate2` portals set the record each completes on
/// (`net::scripts`).
const DOMESTIC_SECRETARY: i32 = 1300003;
const EXPLORING_MUSHROOM_FOREST: i16 = 2314;
const OVER_THE_CASTLE_WALLS: i16 = 2322;

/// `q2314s`, `q2322s`: the Secretary asks the player to look into
/// something; a yes says where to go and starts `quest`, a no ends it
/// (with `decline` said first, if there is one).
fn secretary_sends(quest: i16, ask: &str, decline: Option<&str>, go: &str) -> NpcScript {
    let mut script = NpcScript::new(
        DOMESTIC_SECRETARY,
        vec![
            Page::Ask {
                text: text(ask),
                on_yes: Vec::new(),
                on_no: decline.map(text),
            },
            Page::Say(text(go)),
        ],
    );
    script.on_end = vec![Effect::StartQuest(quest)];
    script
}

/// Princess Violetta, in the Wedding Hall (106021600).
const VIOLETTA: i32 = 1300002;
/// The Royal Seal (4001318), which The Seal of the Empire (2331) takes to
/// the Mushking.
const ROYAL_SEAL: i32 = 4001318;
const SEAL_OF_THE_EMPIRE: i16 = 2331;
/// Eliminating the Rest, and the key to the Security Room it is for.
const ELIMINATING_THE_REST: i16 = 2335;
const SECURITY_ROOM_KEY: i32 = 4032405;

/// `q2332e`, Where's Violetta?: found in the Wedding Hall (`findvioleta`
/// set the record its end asks for), Violetta thanks the player, with the
/// data's 4,400 EXP (`QuestInfo rewardSummary`; `Act` is empty, and Cosmic
/// gives the same at the hall's door).
fn violetta_found() -> NpcScript {
    let mut script = NpcScript::new(VIOLETTA, vec![Page::Say(text(keys::VIOLETTA_FOUND))]);
    script.on_end = vec![
        Effect::CompleteQuest(events::WHERES_VIOLETTA),
        Effect::GainExp(4400),
    ];
    script
}

/// `q2333s`, The Story of Betrayal: Violetta asks for help, names the
/// Prime Minister, and he comes out (Cosmic `2333.js` and
/// `MK_PrimeMinister`).
fn violetta_betrayed() -> NpcScript {
    let mut script = NpcScript::new(
        VIOLETTA,
        vec![
            Page::Ask {
                text: text(keys::VIOLETTA_HELP),
                on_yes: Vec::new(),
                on_no: None,
            },
            Page::Say(text(keys::VIOLETTA_PRIME_MINISTER)),
        ],
    );
    script.on_end = vec![
        Effect::StartQuest(events::STORY_OF_BETRAYAL),
        Effect::BringOutMob {
            mob_id: events::PRIME_MINISTER,
            at: events::PRIME_MINISTER_AT,
        },
    ];
    script
}

/// `q2333e`: the Prime Minister beaten, with the data's 15,000 EXP
/// (`rewardSummary`, Cosmic's too).
fn prime_minister_beaten() -> NpcScript {
    let mut script = NpcScript::new(
        VIOLETTA,
        vec![Page::Say(text(keys::VIOLETTA_PRIME_MINISTER_BEATEN))],
    );
    script.on_end = vec![
        Effect::CompleteQuest(events::STORY_OF_BETRAYAL),
        Effect::GainExp(15_000),
    ];
    script
}

/// `q2334s`, The Identity of the Princess: Violetta shows her face, which
/// completes the quest as it starts, with Cosmic's 1,000 EXP (the data has
/// none).
fn violettas_face() -> NpcScript {
    let mut script = NpcScript::new(VIOLETTA, vec![Page::Say(text(keys::VIOLETTA_FACE))]);
    script.on_end = vec![Effect::FinishQuest(2334), Effect::GainExp(1000)];
    script
}

/// `q2335s`, Eliminating the Rest: Violetta gives the key the Prime
/// Minister dropped (4032405), which opens the Security Room near
/// Skyscraper 3 (`go_secretroom`, whose door completes the quest), and the
/// quest with it; without a free ETC slot, neither.
fn violettas_key() -> NpcScript {
    let mut script = NpcScript::new(
        VIOLETTA,
        vec![
            Page::Say(text(keys::VIOLETTA_MINIONS)),
            Page::Ask {
                text: text(keys::VIOLETTA_KEY),
                on_yes: Vec::new(),
                on_no: None,
            },
            Page::Say(text(keys::VIOLETTA_KEY_LUCK)),
        ],
    );
    script.on_end = vec![
        Effect::GiveItem {
            item_id: SECURITY_ROOM_KEY,
            quantity: 1,
        },
        Effect::StartQuest(ELIMINATING_THE_REST),
    ];
    script.inventory_full = Some(text(keys::VIOLETTA_KEY_FULL));
    script
}

/// `q2342s`, The Recovered Royal Seal: for one still taking the seal to
/// the Mushking (2331) without it (the quest asks that none be held),
/// Violetta hands over the one the Prime Minister dropped, and the quest
/// completes as it starts (Cosmic `2342.js`). Anyone else is only told
/// that she keeps it.
fn violettas_seal(player: &Player) -> NpcScript {
    if player.quests.state(SEAL_OF_THE_EMPIRE) != QuestState::Started
        || player.inventory.count(ROYAL_SEAL) > 0
    {
        return NpcScript::new(VIOLETTA, vec![Page::Say(text(keys::VIOLETTA_SEAL_KEPT))]);
    }
    let mut script = NpcScript::new(VIOLETTA, vec![Page::Say(text(keys::VIOLETTA_SEAL))]);
    script.on_end = vec![
        Effect::FinishQuest(2342),
        Effect::GiveItem {
            item_id: ROYAL_SEAL,
            quantity: 1,
        },
    ];
    script.inventory_full = Some(text(keys::VIOLETTA_SEAL_FULL));
    script
}

/// Olson the Toy Soldier, on Eos Tower's 100th floor (221024400).
const OLSON: i32 = 2040002;
/// Mark the Toy Soldier, in the dollhouse room (922000010).
const MARK: i32 = 2040028;

/// Olson, to a player on Dollhouse (3230): the pendulum hides in the odd
/// dollhouse, and he takes them into the dollhouse room alone (Cosmic
/// `2040002`). Only one may be in it on a channel at a time; another is
/// told so. To anyone else he speaks as usual.
fn olson() -> NpcScript {
    let mut script = NpcScript::new(
        OLSON,
        vec![
            Page::Say(text(keys::DOLLHOUSE_HINT)),
            Page::Ask {
                text: text(keys::DOLLHOUSE_ENTER),
                on_yes: vec![Effect::StartEvent(EventKind::Dollhouse)],
                on_no: None,
            },
        ],
    );
    script.refused = Some(text(keys::DOLLHOUSE_BUSY));
    script
}

/// Mark, in the dollhouse room: he takes the pendulum from a player on
/// Dollhouse (3230), which completes it, and sends them back by Olson, which
/// ends their run (Cosmic `2040028`). The data gives nothing for it.
fn mark(player: &Player) -> NpcScript {
    let back = Effect::WarpToPortal(events::EOS_TOWER_100, events::EOS_TOWER_100_RETURN_ID);
    let found = player.quests.state(events::DOLLHOUSE_QUEST) == QuestState::Started
        && player.inventory.count(events::DOLLHOUSE_PENDULUM) > 0;
    let ask = |key| Page::Ask {
        text: text(key),
        on_yes: vec![back.clone()],
        on_no: None,
    };
    if !found {
        return NpcScript::new(MARK, vec![ask(keys::DOLLHOUSE_NOT_FOUND)]);
    }
    let thanks = Page::Act {
        text: text(keys::DOLLHOUSE_FOUND),
        effects: vec![
            Effect::TakeItem {
                item_id: events::DOLLHOUSE_PENDULUM,
                quantity: 1,
            },
            Effect::CompleteQuest(events::DOLLHOUSE_QUEST),
        ],
    };
    NpcScript::new(MARK, vec![thanks, ask(keys::DOLLHOUSE_BACK)])
}

/// Cheng the Assistant, in the Toy Factory (220020000) and at Sector 4's far
/// end (922000000).
const CHENG: i32 = 2040003;
/// The ten Machine Parts Cheng wants, and his thanks: one of four shoe
/// scrolls, drawn at random, and 2,700 EXP (Cosmic `3239.js`; the data gives
/// nothing).
const PARTS_WANTED: i16 = 10;
const PARTS_SCROLLS: [i32; 4] = [2040704, 2040705, 2040707, 2040708];
const PARTS_EXP: i32 = 2700;

/// Cheng (Cosmic `2040003`): in the factory, he takes a player on The
/// Missing Mechanical Parts (3239) alone into Sector 4; others get his
/// default speech. In Sector 4, he takes ten Machine Parts from a player on
/// it, which completes it, once they have a USE slot for the scroll, then
/// offers the way out to the Secret Passage, which ends their run.
fn cheng(player: &Player, rng: &mut impl Rng) -> Option<NpcScript> {
    let started = player.quests.state(events::PARTS_QUEST) == QuestState::Started;
    if player.stats.map_id != events::PARTS_ROOM {
        if !started {
            return None;
        }
        let mut script = NpcScript::new(
            CHENG,
            vec![Page::Ask {
                text: text(keys::MECHANICAL_PARTS_ENTER),
                on_yes: vec![Effect::StartEvent(EventKind::MechanicalParts)],
                on_no: None,
            }],
        );
        script.refused = Some(text(keys::MECHANICAL_PARTS_BUSY));
        return Some(script);
    }
    let leave = Page::Ask {
        text: text(keys::MECHANICAL_PARTS_LEAVE),
        on_yes: vec![Effect::Warp(events::PARTS_PASSAGE)],
        on_no: None,
    };
    if !started || player.inventory.count(events::MACHINE_PART) < i32::from(PARTS_WANTED) {
        return Some(NpcScript::new(CHENG, vec![leave]));
    }
    let scroll = *PARTS_SCROLLS.choose(rng)?;
    if !crate::items::fits_until(&player.inventory, scroll, 1, None) {
        let full = Page::Say(text(keys::MECHANICAL_PARTS_NO_ROOM));
        return Some(NpcScript::new(CHENG, vec![full, leave]));
    }
    let thanks = Page::Act {
        text: text(keys::MECHANICAL_PARTS_DONE),
        effects: vec![
            Effect::TakeItem {
                item_id: events::MACHINE_PART,
                quantity: PARTS_WANTED,
            },
            Effect::GiveItem {
                item_id: scroll,
                quantity: 1,
            },
            Effect::GainExp(PARTS_EXP),
            Effect::CompleteQuest(events::PARTS_QUEST),
        ],
    };
    Some(NpcScript::new(CHENG, vec![thanks, leave]))
}

/// The Spirit of Snow Statue, broken, in the Valley of Snowmen (211040102).
const SNOW_STATUE: i32 = 2020012;

/// `q3108s`, Snowman's Rage - Found a clue: the broken statue is the clue
/// Scadur wants, and the quest completes as it starts (Cosmic `3108.js`),
/// with the data's 1,000 EXP (`Act/1`).
fn snow_statue_clue() -> NpcScript {
    let mut script = NpcScript::new(SNOW_STATUE, vec![Page::Say(text(keys::SNOW_STATUE_CLUE))]);
    script.on_end = vec![Effect::FinishQuest(3108), Effect::GainExp(1000)];
    script
}

/// In Search for the Lost Memory, for each branch of explorers, the
/// Cygnus Knights and the Arans (Searching for Lost Memories, at Lilin):
/// (quest, its job instructor).
const LOST_MEMORIES: [(i16, i32); 7] = [
    (3523, 1022000),
    (3524, 1032001),
    (3525, 1012100),
    (3526, 1052001),
    (3527, 1090000),
    (3529, 1101002),
    (3539, LILIN),
];
/// The record the Memory Keeper's quest (3507) reads (`Check/1
/// infoNumber`), and what it reads once the memory is back (`infoex`).
const LOST_MEMORY_RECORD: i16 = 7081;
const LOST_MEMORY_BACK: &str = "1";

/// `q3523s`-`q3527s`, `q3529s` and `q3539s`: the job instructor of a player on the
/// Memory Keeper's quest (3507) gives back the lost memory, which sets
/// 3507's record so it can end, and the quest completes as it starts
/// (Cosmic `3523.js`). The data has no rewards.
fn lost_memory(quest: i16) -> Option<NpcScript> {
    let &(_, npc) = LOST_MEMORIES.iter().find(|&&(q, _)| q == quest)?;
    let mut script = NpcScript::new(npc, vec![Page::Say(text(keys::LOST_MEMORY_REGAINED))]);
    script.on_end = vec![
        Effect::FinishQuest(quest),
        Effect::SetRecord {
            record: LOST_MEMORY_RECORD,
            value: LOST_MEMORY_BACK,
        },
    ];
    Some(script)
}

/// The black shadow hidden in the Pig Farm (100020000) for "A Black Shadow
/// in the Pig Farm" (20706), and the four spots around it that hide
/// nothing; the black shadow in the hidden Magic Library
/// (`scripts::HIDDEN_MAGIC_LIBRARY`) and the library's four corners.
const PIG_FARM_SHADOW: i32 = 1012115;
const PIG_FARM_DECOY: i32 = 1012116;
const LIBRARY_SHADOW: i32 = 1032109;
const LIBRARY_CORNER: i32 = 1032110;
/// "A Black Shadow in the Pig Farm" and the record its completion reads
/// (`Check/1 infoNumber`).
const BLACK_SHADOW_IN_THE_PIG_FARM: i16 = 20706;
const PIG_FARM_SHADOW_RECORD: i16 = 20731;
/// The Blue Mushrooms the library's black shadow sets on the player, ten
/// on the ground below each point (Cosmic `1032109.js`).
const BLUE_MUSHROOM: i32 = 2220100;
const LIBRARY_AMBUSH: [(i16, i16); 3] = [(117, 183), (4, 183), (-109, 183)];
const LIBRARY_AMBUSH_EACH: u8 = 10;

/// A black shadow the Cygnus Knights hunt, or a spot that hides none.
/// While "A Black Shadow in the Pig Farm" is under way, the Pig Farm's
/// shadow is spotted, which sets the record its completion at 1103001
/// reads (Cosmic completes the quest there instead). While "Maybe it's
/// Grendel!" is under way and it isn't yet found, the library's shadow
/// sets its Blue Mushrooms on the player once read, and the record that
/// quest's completion reads. Otherwise there is nothing to see. The text
/// is unsourced.
fn black_shadow(npc: i32, player: &Player) -> NpcScript {
    let started = |quest| player.quests.state(quest) == QuestState::Started;
    let pages = match npc {
        PIG_FARM_SHADOW if started(BLACK_SHADOW_IN_THE_PIG_FARM) => vec![Page::Act {
            text: text(keys::BLACK_SHADOW_SPOTTED),
            effects: vec![Effect::SetRecord {
                record: PIG_FARM_SHADOW_RECORD,
                value: scripts::SHADOW_SPOTTED,
            }],
        }],
        LIBRARY_SHADOW
            if started(scripts::MAYBE_ITS_GRENDEL)
                && player.quests.info(scripts::LIBRARY_SHADOW_RECORD)
                    != Some(scripts::SHADOW_SPOTTED) =>
        {
            vec![
                Page::Say(text(keys::BLACK_SHADOW_AMBUSH)),
                Page::Done(vec![
                    Effect::SetRecord {
                        record: scripts::LIBRARY_SHADOW_RECORD,
                        value: scripts::SHADOW_SPOTTED,
                    },
                    Effect::BringOutMobs {
                        mob_id: BLUE_MUSHROOM,
                        at: &LIBRARY_AMBUSH,
                        each: LIBRARY_AMBUSH_EACH,
                    },
                ]),
            ]
        }
        _ => vec![Page::Say(text(keys::BLACK_SHADOW_NOTHING))],
    };
    NpcScript::new(npc, pages)
}

/// The Cave Wall of Evil Eye, the Puppeteer's door at the end of the Cave
/// of Evil Eye (105070300), which its `enterDollcave` portal opens until
/// the Puppeteer is beaten (Cosmic `PupeteerPassword`).
pub const PUPPETEER_DOOR: i32 = 1063011;
/// The door's password, as Find the Puppeteer! (20730) spells it out:
/// `#p1104000# is a genius #o9300285#!`, Francis's name and the
/// Puppeteer's.
const PUPPETEER_PASSWORD: &str = "Francis is a genius Puppeteer";
const PUPPETEER_PASSWORD_MAX: i16 = 40;
/// Behind the door, Francis's room, where Francis (1104000) catches the
/// intruder.
const FRANCIS_ROOM: i32 = 910510001;
const FRANCIS: i32 = 1104000;
/// The Puppeteer's Cave (21728), an Aran's search of the door, and the
/// record its completion reads ("0").
const PUPPETEERS_CAVE: i16 = 21728;
const PUPPETEERS_CAVE_RECORD: i16 = 21761;
const PUPPETEERS_CAVE_SEARCHED: &str = "0";

/// The Puppeteer's door. An Aran on The Puppeteer's Cave finds the way
/// blocked, which sets the record that quest's completion at 1061019
/// reads. Anyone else is asked for the password. The text is unsourced.
fn puppeteer_door(player: &Player) -> NpcScript {
    if player.quests.state(PUPPETEERS_CAVE) == QuestState::Started {
        return NpcScript::new(
            PUPPETEER_DOOR,
            vec![Page::Act {
                text: text(keys::PUPPETEER_DOOR_SEARCHED),
                effects: vec![Effect::SetRecord {
                    record: PUPPETEERS_CAVE_RECORD,
                    value: PUPPETEERS_CAVE_SEARCHED,
                }],
            }],
        );
    }
    NpcScript::new(
        PUPPETEER_DOOR,
        vec![Page::AskText {
            text: text(keys::PUPPETEER_DOOR_ASK),
            min: 1,
            max: PUPPETEER_PASSWORD_MAX,
            on_text: OnText::PuppeteerDoor {
                hunting: scripts::hunted_puppeteer(player).is_some(),
            },
        }],
    )
}

/// The right password, in any case and with or without its "!", lets one
/// hunting the Puppeteer into Francis's room; anyone else hears it's right
/// but the way stays shut, in pink.
fn puppeteer_door_opens(text_given: &str, hunting: bool) -> Vec<Page> {
    let given = text_given.trim();
    let given = given.strip_suffix('!').unwrap_or(given).trim_end();
    if !given.eq_ignore_ascii_case(PUPPETEER_PASSWORD) {
        return vec![Page::Say(text(keys::PUPPETEER_DOOR_WRONG))];
    }
    if !hunting {
        return vec![Page::Done(vec![Effect::Pink(text(
            keys::PUPPETEER_DOOR_SHUT,
        ))])];
    }
    vec![Page::Done(vec![
        Effect::PortalSound,
        Effect::WarpToPortal(FRANCIS_ROOM, 1),
    ])]
}

/// Francis catches the intruder and sends them on into the Puppeteer's
/// hideout, one player at a time (Cosmic `1104000.js` and its `Puppeteer`
/// event). The text is unsourced.
fn francis() -> NpcScript {
    let mut script = NpcScript::new(FRANCIS, vec![Page::Say(text(keys::FRANCIS_INTRUDER))]);
    script.on_end = vec![
        Effect::RequireEmptyMap(scripts::PUPPETEERS_HIDEOUT),
        Effect::WarpToPortal(scripts::PUPPETEERS_HIDEOUT, 1),
    ];
    script.refused = Some(text(keys::FRANCIS_HIDEOUT_BUSY));
    script
}

const FAKE_TRU: i32 = 1204007;
const TRU: i32 = 1002104;
const STRANGE_INFORMATION: i16 = 21733;
const POLEARM_MASTERY: i32 = 21100000;

/// `q21733s`: the Puppeteer calls an Aran to Lith Harbor's Info Shop in
/// Tru's name, and Gathering Some Strange Information (21733) starts.
fn fake_tru_calls() -> NpcScript {
    let mut script = NpcScript::new(FAKE_TRU, vec![Page::Say(text(keys::FAKE_TRU_CALL))]);
    script.on_end = vec![Effect::StartQuest(STRANGE_INFORMATION)];
    script
}

/// `q21733e`: the Puppeteer beaten (`scripts::unscripted_map_enter`), Tru
/// gives 8,000 EXP and Polearm Mastery, up to 20 (Cosmic `21733.js`).
fn tru_on_the_trap() -> NpcScript {
    let mut script = NpcScript::new(
        TRU,
        [keys::TRU_PUPPETEER_BEATEN, keys::TRU_POLEARM_MASTERY]
            .map(|key| Page::Say(text(key)))
            .to_vec(),
    );
    script.on_end = vec![
        Effect::CompleteQuest(STRANGE_INFORMATION),
        Effect::GainExp(8000),
        Effect::TeachSkill {
            skill_id: POLEARM_MASTERY,
            level: 0,
            master_level: 20,
        },
    ];
    script
}

const TRACES_OF_THE_PUPPETEER: i16 = 21734;
const SEAL_STONE_OF_VICTORIA: i16 = 21735;
const OFF_TO_ORBIS: i16 = 21736;
const COMBO_DRAIN: i32 = 21100005;
/// The Seal Stone of Victoria Island, for Lilin (1201000) to see.
const VICTORIA_SEAL_STONE: i32 = 4032323;

/// `q21734s`: Tru sends an Aran after the Puppeteer, down the passage
/// behind Sleepywood's cave (`enterDollWay`).
fn puppeteer_traces() -> NpcScript {
    let mut script = NpcScript::new(TRU, vec![Page::Say(text(keys::TRU_PUPPETEER_TRACES))]);
    script.on_end = vec![Effect::StartQuest(TRACES_OF_THE_PUPPETEER)];
    script
}

/// `q21734e`: the Puppeteer beaten in his arena
/// (`scripts::PUPPETEERS_ARENA`), Tru gives 12,500 EXP and Combo Drain, up
/// to 20 (Cosmic `21734.js`).
fn puppeteer_trail_beaten() -> NpcScript {
    let mut script = NpcScript::new(
        TRU,
        keys::TRU_COMBO_DRAIN
            .map(|key| Page::Say(text(key)))
            .to_vec(),
    );
    script.on_end = vec![
        Effect::CompleteQuest(TRACES_OF_THE_PUPPETEER),
        Effect::GainExp(12500),
        Effect::TeachSkill {
            skill_id: COMBO_DRAIN,
            level: 0,
            master_level: 20,
        },
    ];
    script
}

/// `q21735e`: Lilin takes the seal stone into safekeeping for 6,037 EXP
/// (Cosmic `21735.js`).
fn seal_stone_kept() -> NpcScript {
    let mut script = NpcScript::new(LILIN, vec![Page::Say(text(keys::LILIN_SEAL_STONE))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: VICTORIA_SEAL_STONE,
            quantity: 1,
        },
        Effect::GainExp(6037),
        Effect::CompleteQuest(SEAL_STONE_OF_VICTORIA),
    ];
    script
}

/// `q21735s`: Tru hands over the Seal Stone of Victoria Island to show
/// Lilin, unless one is held (Cosmic `21735.js`).
fn seal_stone_for_lilin(player: &Player) -> NpcScript {
    let mut script = NpcScript::new(
        TRU,
        keys::TRU_SEAL_STONE
            .map(|key| Page::Say(text(key)))
            .to_vec(),
    );
    if player.inventory.count(VICTORIA_SEAL_STONE) == 0 {
        script.on_end.push(Effect::GiveItem {
            item_id: VICTORIA_SEAL_STONE,
            quantity: 1,
        });
    }
    script
        .on_end
        .push(Effect::StartQuest(SEAL_STONE_OF_VICTORIA));
    script.inventory_full = Some(text(keys::TRU_SEAL_STONE_FULL));
    script
}

/// `q21736s`: Tru sends an Aran to Lisa (2012012) in Orbis, who ends it
/// with the data's 500 EXP (Cosmic `21736.js`).
fn off_to_orbis() -> NpcScript {
    let [news, ask, go] = keys::TRU_ORBIS;
    NpcScript::new(
        TRU,
        vec![
            Page::Say(text(news)),
            Page::Ask {
                text: text(ask),
                on_yes: vec![Effect::StartQuest(OFF_TO_ORBIS)],
                on_no: Some(text(keys::TRU_ORBIS_DECLINED)),
            },
            Page::Say(text(go)),
        ],
    )
}

const SEALED_GARDEN: i16 = 21738;
const TO_THE_SEALED_GARDEN: i16 = 21739;
const ORBIS_STONE_STOLEN: i16 = 21740;
const INVESTIGATING_MU_LUNG: i16 = 21741;
const THE_BLACK_SCROLL: i16 = 21742;
const LILIN: i32 = 1201000;
const MR_DO: i32 = 2090004;
const COMBO_SMASH: i32 = 21100004;
/// The Hanging Scroll, a raise item: Special Ink (4032342, from "The
/// Special Ink", 21743) brings its writing back in record 21763.
const HANGING_SCROLL: i32 = 4220151;

/// `q21738s`: Spiruna (2032001) tells an Aran of the garden the Black
/// Wings are after, and Sealed Garden starts; Spiruna ends it with the
/// data's 1,000 EXP (Cosmic `21738.js`).
fn sealed_garden() -> NpcScript {
    let [news, ask] = keys::SPIRUNA_SEALED_GARDEN;
    NpcScript::new(
        SPIRUNA,
        vec![
            Page::Say(text(news)),
            Page::Ask {
                text: text(ask),
                on_yes: vec![Effect::StartQuest(SEALED_GARDEN)],
                on_no: Some(text(keys::SPIRUNA_SEALED_GARDEN_DECLINED)),
            },
        ],
    )
}

/// `q21739e`: the garden's guardian beaten (`enterNepenthes`,
/// `sealGarden`), Spiruna gives 29,500 EXP (Cosmic `21739.js`).
fn sealed_garden_cleared() -> NpcScript {
    let mut script = NpcScript::new(SPIRUNA, vec![Page::Say(text(keys::SPIRUNA_GARDEN_CLEARED))]);
    script.on_end = vec![
        Effect::CompleteQuest(TO_THE_SEALED_GARDEN),
        Effect::GainExp(29500),
    ];
    script
}

/// `q21740s`: the Orbis seal stone stolen, Tru sends an Aran to Lilin
/// (Cosmic `21740.js`).
fn orbis_stone_stolen() -> NpcScript {
    let mut script = NpcScript::new(TRU, vec![Page::Say(text(keys::TRU_ORBIS_STONE_STOLEN))]);
    script.on_end = vec![Effect::StartQuest(ORBIS_STONE_STOLEN)];
    script
}

/// `q21740e`: Lilin teaches Combo Smash, up to 20, and nothing else
/// (Cosmic `21740.js`).
fn lilin_teaches_combo_smash() -> NpcScript {
    let mut script = NpcScript::new(
        LILIN,
        keys::LILIN_COMBO_SMASH
            .map(|key| Page::Say(text(key)))
            .to_vec(),
    );
    script.on_end = vec![
        Effect::CompleteQuest(ORBIS_STONE_STOLEN),
        Effect::TeachSkill {
            skill_id: COMBO_SMASH,
            level: 0,
            master_level: 20,
        },
    ];
    script
}

/// `q21741s`: Tru sends an Aran to Mr. Do in Mu Lung, who ends it with the
/// data's 500 EXP (Cosmic `21741.js`).
fn off_to_mu_lung() -> NpcScript {
    let [news, ask, keyword] = keys::TRU_MU_LUNG;
    NpcScript::new(
        TRU,
        vec![
            Page::Say(text(news)),
            Page::Ask {
                text: text(ask),
                on_yes: vec![Effect::StartQuest(INVESTIGATING_MU_LUNG)],
                on_no: Some(text(keys::TRU_MU_LUNG_DECLINED)),
            },
            Page::Say(text(keyword)),
        ],
    )
}

/// `q21742s`: Mr. Do owns up to soaking the Black Wings' Hanging Scroll and
/// hands it over to have its writing restored, unless one is held (Cosmic
/// `21742.js`). Its record grows by feeding it Special Ink in its raise
/// window (`world/raise.rs`).
fn black_scroll(player: &Player) -> NpcScript {
    let mut on_yes = Vec::new();
    if player.inventory.count(HANGING_SCROLL) == 0 {
        on_yes.push(Effect::GiveItem {
            item_id: HANGING_SCROLL,
            quantity: 1,
        });
    }
    on_yes.push(Effect::StartQuest(THE_BLACK_SCROLL));
    let [met, ask] = keys::MR_DO_BLACK_SCROLL;
    let [soaked, jin_jin] = keys::MR_DO_SCROLL_SOAKED;
    let mut script = NpcScript::new(
        MR_DO,
        vec![
            Page::Say(text(met)),
            Page::Ask {
                text: text(ask),
                on_yes,
                on_no: None,
            },
            Page::Say(text(soaked)),
            Page::Say(text(jin_jin)),
        ],
    );
    script.inventory_full = Some(text(keys::MR_DO_SCROLL_FULL));
    script
}

/// `q21742e`: the writing back (record 21763 at 800, `Check.img/21742/1`),
/// Mr. Do takes the scroll and gives 10,000 EXP. Cosmic also takes eight
/// Special Ink, which feeding the scroll has already used up.
fn black_scroll_read() -> NpcScript {
    let mut script = NpcScript::new(MR_DO, vec![Page::Say(text(keys::MR_DO_SCROLL_READ))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: HANGING_SCROLL,
            quantity: 1,
        },
        Effect::GainExp(10000),
        Effect::CompleteQuest(THE_BLACK_SCROLL),
    ];
    script
}

const MU_GONG: i32 = 2091007;
const MU_GONGS_TEST: i16 = 21746;
const SHADOW_KNIGHT_CAUGHT_EXP: i32 = 16000;
const STOLEN_SEAL_STONE_OF_MU_LUNG: i16 = 21748;
const FINAL_CHARGE: i32 = 21100002;
const BACK_TO_THE_PAST: i16 = 21749;
const ATHENA_PIERCE_OF_ELLIN: i32 = 2131000;
const SHE_REMEMBERS_ME: i16 = 21750;
const LOST_SEAL_STONE: i16 = 21753;
const ATHENA_PIERCE: i32 = 1012100;
const INFO_ON_ANOTHER_SEAL_STONE: i16 = 21754;
const ATHENAS_LETTER: i32 = 4032328;
const NEINHEART: i32 = 1101002;
const NEINHEART_THE_STRATEGIST: i16 = 21757;
const LETTER_TO_THE_EMPRESS: i32 = 4032330;

/// `q21746s`: Mu Gong tests an Aran against a clone in the arena, one
/// challenger at a time (Cosmic `21746.js`). The data ends it with 5,000
/// EXP; one who leaves before winning forfeits and starts again (its
/// `Say.img/21746/1` stop line).
fn mu_gongs_test() -> NpcScript {
    let mut script = NpcScript::new(MU_GONG, vec![Page::Say(text(keys::MU_GONG_TEST))]);
    script.on_end = vec![
        Effect::RequireEmptyMap(scripts::MU_GONGS_ARENA),
        Effect::StartQuest(MU_GONGS_TEST),
        Effect::WarpToPortal(scripts::MU_GONGS_ARENA, 0),
    ];
    script.refused = Some(text(keys::MU_GONG_ARENA_BUSY));
    script
}

/// `q21747s`: Mu Gong sends an Aran after the Shadow Knight in the Sealed
/// Shrine, with the pillar's password (Cosmic `21747.js`; the password is
/// `QuestInfo/21747/1`'s).
fn catch_the_shadow_knight() -> NpcScript {
    let [ask, password] = keys::MU_GONG_SHADOW_KNIGHT;
    NpcScript::new(
        MU_GONG,
        vec![
            Page::Ask {
                text: text(ask),
                on_yes: vec![Effect::StartQuest(scripts::CATCH_THE_SHADOW_KNIGHT)],
                on_no: None,
            },
            Page::Say(text(password)),
        ],
    )
}

/// `q21747e`: the Shadow Knight beaten but the stone taken, Mu Gong gives
/// 16,000 EXP (Cosmic `21747.js`).
fn shadow_knight_caught() -> NpcScript {
    let mut script = NpcScript::new(
        MU_GONG,
        keys::MU_GONG_STONE_TAKEN
            .map(|key| Page::Say(text(key)))
            .to_vec(),
    );
    script.on_end = vec![
        Effect::GainExp(SHADOW_KNIGHT_CAUGHT_EXP),
        Effect::CompleteQuest(scripts::CATCH_THE_SHADOW_KNIGHT),
    ];
    script
}

/// `q21748e`: Lilin hears of the Mu Lung seal stone and teaches Final
/// Charge, up to 30, its levels in Skill.img, with 20,000 EXP (Cosmic
/// `21748.js`).
fn lilin_teaches_final_charge() -> NpcScript {
    let mut script = NpcScript::new(
        LILIN,
        keys::LILIN_FINAL_CHARGE
            .map(|key| Page::Say(text(key)))
            .to_vec(),
    );
    script.on_end = vec![
        Effect::CompleteQuest(STOLEN_SEAL_STONE_OF_MU_LUNG),
        Effect::GainExp(20000),
        Effect::TeachSkill {
            skill_id: FINAL_CHARGE,
            level: 0,
            master_level: 30,
        },
    ];
    script
}

/// `q21749s`: Tru tells an Aran of Ludibrium's broken clocktower, a way
/// into the past. Cosmic completes it here; the data ends it at Tru with
/// 500 EXP (`Check.img/21749/1`).
fn back_to_the_past() -> NpcScript {
    let mut script = NpcScript::new(
        TRU,
        keys::TRU_BACK_TO_THE_PAST
            .map(|key| Page::Say(text(key)))
            .to_vec(),
    );
    script.on_end = vec![Effect::StartQuest(BACK_TO_THE_PAST)];
    script
}

/// `q21750e`: Athena Pierce of the past knows the Aran (Cosmic
/// `21750.js`). The quest gives nothing.
fn athena_remembers() -> NpcScript {
    let mut script = NpcScript::new(
        ATHENA_PIERCE_OF_ELLIN,
        keys::ATHENA_REMEMBERS
            .map(|key| Page::Say(text(key)))
            .to_vec(),
    );
    script.on_end = vec![Effect::CompleteQuest(SHE_REMEMBERS_ME)];
    script
}

/// `q21753s`: the stone gone with the Gentleman, Athena Pierce promises to
/// carry a letter to the Aran's own time (Cosmic `21753.js`). Tru ends it
/// with the data's 20,000 EXP.
fn athena_keeps_the_letter() -> NpcScript {
    let mut script = NpcScript::new(
        ATHENA_PIERCE_OF_ELLIN,
        keys::ATHENA_LETTER_PROMISE
            .map(|key| Page::Say(text(key)))
            .to_vec(),
    );
    script.on_end = vec![Effect::StartQuest(LOST_SEAL_STONE)];
    script
}

/// `q21754s`: Athena Pierce, in the Aran's own time, hands over the letter
/// kept all along, unless one is held (Cosmic `21754.js`). Tru ends it with the
/// data's 56,800 EXP and takes it.
fn athena_hands_over_the_letter(player: &Player) -> NpcScript {
    let mut script = NpcScript::new(ATHENA_PIERCE, vec![Page::Say(text(keys::ATHENA_LETTER))]);
    if player.inventory.count(ATHENAS_LETTER) == 0 {
        script.on_end.push(Effect::GiveItem {
            item_id: ATHENAS_LETTER,
            quantity: 1,
        });
    }
    script
        .on_end
        .push(Effect::StartQuest(INFO_ON_ANOTHER_SEAL_STONE));
    script.inventory_full = Some(text(keys::ATHENA_LETTER_FULL));
    script
}

/// `q21757e`: Neinheart takes Lilin's letter and gives 1,000 EXP (Cosmic
/// `21757.js`, `Act.img/21757/1`).
fn neinheart_takes_the_letter() -> NpcScript {
    let mut script = NpcScript::new(NEINHEART, vec![Page::Say(text(keys::NEINHEART_LETTER))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: LETTER_TO_THE_EMPRESS,
            quantity: 1,
        },
        Effect::GainExp(1000),
        Effect::CompleteQuest(NEINHEART_THE_STRATEGIST),
    ];
    script
}

const NEW_BEGINNINGS: i16 = 21700;
const TRAIN_OR_DIE_3: i16 = 21703;
const BABY_STEPS: i16 = 21704;
const FIRST_ASSIGNMENT_DONE: i16 = 21712;
const SECOND_ASSIGNMENT_DONE: i16 = 21716;
const PUPPETEERS_INVITATION: i16 = 21719;
const PUPPETEERS_WARNING: i16 = 21720;
const FOURTH_ASSIGNMENT_DONE: i16 = 21729;
const SUSPICIOUS_JOHN: i16 = 21766;
const WOODEN_BOX: i16 = 21767;
const PUO: i32 = 1202006;
const BRUCE: i32 = 1012111;
const ROWEN: i32 = 1032101;
/// The shadow that hands an Aran the Puppeteer's invitation (21719), from
/// the quest alert: it stands on no map.
const INVITATION_SHADOW: i32 = 1204000;
/// Francis in the Puppeteer's Cave (910510200), the cave's only way out.
const CAVE_FRANCIS: i32 = 1204001;
const PUPPETEERS_CAVE_MAP: i32 = 910510200;
/// Where Francis sends an Aran out: the Hunting Ground in the Deep Forest
/// II (105040200), by its `in00` (portal 10) to the Puppeteer's Hiding
/// Place.
const DEEP_FOREST_II: i32 = 105040200;
const DEEP_FOREST_II_IN00: u8 = 10;
/// The record "The Puppeteer's Warning" (21720) needs at "0" to start
/// (`Check.img/21720/0`), which Francis's warning sets.
pub const PUPPETEERS_WARNING_RECORD: i16 = 21760;
const ILJI: i32 = 1061019;
const TEO: i32 = 1002001;
const JOHNS_BOX: i32 = 1204033;
const PRACTICE_POLEARM: i32 = 1442077;
const SPECIAL_RIEN_RED_POTION: i32 = 2000022;
const SPECIAL_RIEN_BLUE_POTION: i32 = 2000023;
const COMBO_ABILITY: i32 = 21000000;
const POLEARM_BOOSTER: i32 = 21001003;
const ARTHRITIS_MEDICINE: i32 = 4032423;

/// `q21700s`: Lilin hands an Aran a Practice Polearm (`Act.img/21700/0`)
/// and sends them west of Rien to Puo, who ends it from the data (Cosmic
/// `21700.js` gives a plain Pole Arm instead).
fn new_beginnings() -> NpcScript {
    let [remember, how, ask, go] = keys::LILIN_NEW_BEGINNINGS;
    let mut script = NpcScript::new(
        LILIN,
        vec![
            Page::Say(text(remember)),
            Page::Reply(text(how)),
            Page::Ask {
                text: text(ask),
                on_yes: vec![
                    Effect::GiveItem {
                        item_id: PRACTICE_POLEARM,
                        quantity: 1,
                    },
                    Effect::StartQuest(NEW_BEGINNINGS),
                ],
                on_no: Some(text(keys::LILIN_NEW_BEGINNINGS_DECLINED)),
            },
            Page::Say(text(go)),
        ],
    );
    script.inventory_full = Some(text(keys::LILIN_NEW_BEGINNINGS_FULL));
    script
}

/// `q21703s`: Puo sends an Aran into the third training ground after 30
/// Target Pigs (Cosmic `21703.js`).
fn target_pigs() -> NpcScript {
    let [lesson, ask, go] = keys::PUO_TARGET_PIGS;
    NpcScript::new(
        PUO,
        vec![
            Page::Say(text(lesson)),
            Page::Ask {
                text: text(ask),
                on_yes: vec![Effect::StartQuest(TRAIN_OR_DIE_3)],
                on_no: Some(text(keys::PUO_TARGET_PIGS_DECLINED)),
            },
            Page::Say(text(go)),
        ],
    )
}

/// `q21703e`: the pigs beaten, Puo gives the data's 30 of each Special
/// Rien potion (`Act.img/21703/1`), and Cosmic's 2,800 EXP and Combo
/// Ability, up to 10, then sends the Aran back to Lilin.
fn combo_ability() -> NpcScript {
    let [proud, ask, remembered, lilin] = keys::PUO_COMBO_ABILITY;
    let mut script = NpcScript::new(
        PUO,
        vec![
            Page::Say(text(proud)),
            Page::Ask {
                text: text(ask),
                on_yes: vec![
                    Effect::GiveItem {
                        item_id: SPECIAL_RIEN_RED_POTION,
                        quantity: 30,
                    },
                    Effect::GiveItem {
                        item_id: SPECIAL_RIEN_BLUE_POTION,
                        quantity: 30,
                    },
                    Effect::GainExp(2800),
                    Effect::TeachSkill {
                        skill_id: COMBO_ABILITY,
                        level: 0,
                        master_level: 10,
                    },
                    Effect::CompleteQuest(TRAIN_OR_DIE_3),
                ],
                on_no: None,
            },
            Page::Reply(text(remembered)),
            Page::Say(text(lilin)),
        ],
    );
    script.inventory_full = Some(text(keys::PUO_COMBO_ABILITY_FULL));
    script
}

/// `q21704s`: Lilin hears of Combo Ability and gives Cosmic's 500 EXP; the
/// quest completes as it starts (`FINISHED_BY_START_SCRIPT`).
fn baby_steps() -> NpcScript {
    let [training, combo, body] = keys::LILIN_BABY_STEPS;
    let mut script = NpcScript::new(
        LILIN,
        vec![
            Page::Say(text(training)),
            Page::Reply(text(combo)),
            Page::Say(text(body)),
        ],
    );
    script.on_end = vec![Effect::FinishQuest(BABY_STEPS), Effect::GainExp(500)];
    script
}

/// An assignment's report to Tru, offered by `npc` (Cosmic `21712.js`,
/// `21716.js`): pages, a yes/no that starts `quest`, then a last word
/// and the Aran's thought.
fn assignment_done(npc: i32, quest: i16, [said, ask, after, report]: [&str; 4]) -> NpcScript {
    NpcScript::new(
        npc,
        vec![
            Page::Say(text(said)),
            Page::Ask {
                text: text(ask),
                on_yes: vec![Effect::StartQuest(quest)],
                on_no: Some(text(keys::ASSIGNMENT_DONE_DECLINED)),
            },
            Page::Say(text(after)),
            Page::Reply(text(report)),
        ],
    )
}

/// `q21719s`: a shadow brings an Aran the Puppeteer's invitation, and on
/// accepting, sends them into the Puppeteer's Cave, where Francis waits
/// (Cosmic `21719.js` completes it at once; here Francis does). The record
/// 21720 starts on is set here too, so an Aran who reaches Tru with the
/// invitation unfinished and ends it there can still go on.
fn puppeteers_invitation() -> NpcScript {
    let [found, who, ask] = keys::SHADOW_INVITATION;
    NpcScript::new(
        INVITATION_SHADOW,
        vec![
            Page::Say(text(found)),
            Page::Reply(text(who)),
            Page::Ask {
                text: text(ask),
                on_yes: vec![
                    Effect::SetRecord {
                        record: PUPPETEERS_WARNING_RECORD,
                        value: "0",
                    },
                    Effect::StartQuest(PUPPETEERS_INVITATION),
                    Effect::WarpToPortal(PUPPETEERS_CAVE_MAP, 0),
                ],
                on_no: Some(text(keys::SHADOW_INVITATION_DECLINED)),
            },
        ],
    )
}

/// Francis in the Puppeteer's Cave warns an Aran off the Black Wings'
/// business, which ends the invitation and sets the record "The
/// Puppeteer's Warning" (21720) starts on (again: the invitation set it
/// first), then sends them out to the Deep Forest (Cosmic `1204001.js`).
/// No source sets the record; this is where the warning is heard. Anyone
/// else is just sent out.
fn cave_francis(player: &Player) -> NpcScript {
    let out = Effect::WarpToPortal(DEEP_FOREST_II, DEEP_FOREST_II_IN00);
    if player.quests.state(PUPPETEERS_INVITATION) != QuestState::Started {
        let mut script =
            NpcScript::new(CAVE_FRANCIS, vec![Page::Say(text(keys::CAVE_FRANCIS_OUT))]);
        script.on_end = vec![out];
        return script;
    }
    let [warning, thought] = keys::CAVE_FRANCIS_WARNING;
    let mut script = NpcScript::new(
        CAVE_FRANCIS,
        vec![Page::Say(text(warning)), Page::Reply(text(thought))],
    );
    script.on_end = vec![
        Effect::SetRecord {
            record: PUPPETEERS_WARNING_RECORD,
            value: "0",
        },
        Effect::CompleteQuest(PUPPETEERS_INVITATION),
        out,
    ];
    script
}

/// `q21720e`: Lilin hears Tru's message of the Puppeteer and gives back
/// Polearm Booster, up to 20, with Cosmic's 3,900 EXP (`21720.js`).
fn polearm_booster() -> NpcScript {
    let [message, black_wings, prophecy, ask, remembered, train] = keys::LILIN_POLEARM_BOOSTER;
    NpcScript::new(
        LILIN,
        vec![
            Page::Say(text(message)),
            Page::Reply(text(black_wings)),
            Page::Say(text(prophecy)),
            Page::Ask {
                text: text(ask),
                on_yes: vec![
                    Effect::GainExp(3900),
                    Effect::TeachSkill {
                        skill_id: POLEARM_BOOSTER,
                        level: 0,
                        master_level: 20,
                    },
                    Effect::CompleteQuest(PUPPETEERS_WARNING),
                ],
                on_no: None,
            },
            Page::Reply(text(remembered)),
            Page::Say(text(train)),
        ],
    )
}

/// `q21729s`: Ilji sends an Aran to read what the Puppeteer scribbled on
/// the Mysterious Statue (21730), then to Tru (Cosmic `21729.js`).
fn fourth_assignment_done() -> NpcScript {
    let mut script = NpcScript::new(ILJI, vec![Page::Say(text(keys::ILJI_STATUE))]);
    script.on_end = vec![Effect::StartQuest(FOURTH_ASSIGNMENT_DONE)];
    script
}

/// `q21766s`: Teo asks an Aran to find out what John (20000) keeps in a
/// wooden box (Cosmic `21766.js`).
fn suspicious_john() -> NpcScript {
    let [favour, where_] = keys::TEO_SUSPICIOUS_JOHN;
    let mut script = NpcScript::new(TEO, vec![Page::Say(text(favour)), Page::Say(text(where_))]);
    script.on_end = vec![Effect::StartQuest(SUSPICIOUS_JOHN)];
    script
}

/// `q21766e`: the box's secret kept, the Aran tells Teo something else,
/// for Cosmic's 200 EXP.
fn john_cleared() -> NpcScript {
    let [ask, made_up] = keys::TEO_JOHN_CLEARED;
    let mut script = NpcScript::new(TEO, vec![Page::Say(text(ask)), Page::Reply(text(made_up))]);
    script.on_end = vec![Effect::GainExp(200), Effect::CompleteQuest(SUSPICIOUS_JOHN)];
    script
}

/// `q21767s`: John's box holds Arthritis Medicine, which the Aran takes to
/// John (who ends it from the data), unless one is held (Cosmic
/// `21767.js`).
fn wooden_box(player: &Player) -> NpcScript {
    let mut script = NpcScript::new(JOHNS_BOX, vec![Page::Reply(text(keys::JOHNS_BOX_MEDICINE))]);
    if player.inventory.count(ARTHRITIS_MEDICINE) == 0 {
        script.on_end.push(Effect::GiveItem {
            item_id: ARTHRITIS_MEDICINE,
            quantity: 1,
        });
    }
    script.on_end.push(Effect::StartQuest(WOODEN_BOX));
    script.inventory_full = Some(text(keys::JOHNS_BOX_FULL));
    script
}

// Pucci (1202007, in 140020110), Scadur in El Nath (2020007), the
// pack's wolf (1202008, in 140010210) and Maha in Rien (1201002), on an
// Aran's way to riding and raising a wolf.
const PUCCI: i32 = 1202007;
const SCADUR: i32 = 2020007;
const PACK_WOLF: i32 = 1202008;
const MAHA: i32 = 1201002;
const PUCCIS_REQUEST: i16 = 21600;
const WOLF_MOUNT: i16 = 21604;
const THE_WOLFS_EVOLUTION: i16 = 21618;
const WOLF_MOUNT_ITEM: i32 = 1902017;
const EVOLVED_WOLF_MOUNT: i32 = 1902018;

/// `q21600s`: Pucci sends an Aran to Nanuke for food for a wolf cub
/// (Cosmic `21600.js`); Nanuke ends it from the data.
fn puccis_request() -> NpcScript {
    let [strong, ask, nanuke] = keys::PUCCI_WOLF_REQUEST;
    NpcScript::new(
        PUCCI,
        vec![
            Page::Say(text(strong)),
            Page::Ask {
                text: text(ask),
                on_yes: vec![Effect::StartQuest(PUCCIS_REQUEST)],
                on_no: Some(text(keys::PUCCI_WOLF_REQUEST_DECLINED)),
            },
            Page::Say(text(nanuke)),
        ],
    )
}

/// `q21604s`: Scadur offers a saddle for the Aran's wolf for 50 hides
/// (Cosmic `21604.js`); the data's end gives it and the riding skill.
fn wolf_mount() -> NpcScript {
    let mut script = NpcScript::new(
        SCADUR,
        keys::SCADUR_WOLF_MOUNT
            .map(|key| Page::Say(text(key)))
            .to_vec(),
    );
    script.on_end = vec![Effect::StartQuest(WOLF_MOUNT)];
    script
}

/// `q21613s`: the pack's wolf wants its pup back and puts the Aran to the
/// test in the wolves' room, alone, against the wolves it summons (Cosmic
/// `21613.js`).
fn the_wolves_test() -> NpcScript {
    let [pup, friend, ask] = keys::WOLF_TEST;
    let mut script = NpcScript::new(
        PACK_WOLF,
        vec![
            Page::Say(text(pup)),
            Page::Reply(text(friend)),
            Page::Ask {
                text: text(ask),
                on_yes: vec![
                    Effect::RequireEmptyMap(scripts::WOLVES_TEST_ROOM),
                    Effect::StartQuest(scripts::THE_WOLVES_TEST),
                    Effect::WarpToPortal(scripts::WOLVES_TEST_ROOM, 0),
                ],
                on_no: Some(text(keys::WOLF_TEST_DECLINED)),
            },
        ],
    );
    script.refused = Some(text(keys::WOLF_TEST_BUSY));
    script
}

/// Whether the pack's wolf lets `player` take its test again: on it and
/// yet to come out of the room with every wolf down. Cosmic has no way
/// back in once time runs out.
fn retakes_the_wolves_test(player: &Player) -> bool {
    player.quests.state(scripts::THE_WOLVES_TEST) == QuestState::Started
        && player.quests.info(scripts::WOLVES_TEST_RECORD) != Some(scripts::WOLF_RUN_DONE)
}

fn the_wolves_test_again() -> NpcScript {
    let mut script = NpcScript::new(
        PACK_WOLF,
        vec![Page::Ask {
            text: text(keys::WOLF_TEST_AGAIN),
            on_yes: vec![
                Effect::RequireEmptyMap(scripts::WOLVES_TEST_ROOM),
                Effect::WarpToPortal(scripts::WOLVES_TEST_ROOM, 0),
            ],
            on_no: None,
        }],
    );
    script.refused = Some(text(keys::WOLF_TEST_BUSY));
    script
}

/// `q21618s`: Maha offers to wake the power in the Aran's wolf (Cosmic
/// `21618.js`).
fn the_wolfs_evolution() -> NpcScript {
    let [power, can_you, ask] = keys::MAHA_WOLF_EVOLUTION;
    NpcScript::new(
        MAHA,
        vec![
            Page::Say(text(power)),
            Page::Reply(text(can_you)),
            Page::Ask {
                text: text(ask),
                on_yes: vec![Effect::StartQuest(THE_WOLFS_EVOLUTION)],
                on_no: None,
            },
        ],
    )
}

/// `q21618e`: Maha turns the wolf into its grown form, once it is out of
/// its equipment slot (Cosmic `21618.js`).
fn the_wolf_evolved(player: &Player) -> NpcScript {
    if player.inventory.count(WOLF_MOUNT_ITEM) == 0 {
        return NpcScript::new(
            MAHA,
            vec![Page::Say(text(keys::MAHA_WOLF_EVOLUTION_UNEQUIP))],
        );
    }
    let mut script = NpcScript::new(MAHA, vec![Page::Say(text(keys::MAHA_WOLF_EVOLUTION_DONE))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: WOLF_MOUNT_ITEM,
            quantity: 1,
        },
        Effect::GiveItem {
            item_id: EVOLVED_WOLF_MOUNT,
            quantity: 1,
        },
        Effect::CompleteQuest(THE_WOLFS_EVOLUTION),
    ];
    script.inventory_full = Some(text(keys::MAHA_WOLF_EVOLUTION_FULL));
    script
}

// Russellon (2111009) in Alcadno's lab and the desk in its upper lab
// (2111015, `alcadno_potion`) where the pills were left, and Yulete
// (2112014) in the lab behind the dark pipe (926130000), in Magatia.
const RUSSELLON: i32 = 2111009;
const PILL_DESK: i32 = 2111015;
const LIFE_ALCHEMY: i16 = 3314;
const RUSSELLONS_PILLS: i32 = 2022198;
/// The disease the pills give (`specEx`: mob skill 125, Poison, level 7),
/// as a buff's skill id.
const PILL_POISON: i32 = 125 | 7 << 16;
const ALL_CURE_POTION: i32 = 2050004;
const LIFE_ALCHEMY_EXP: i32 = 12500;
/// 3314's `Act/1` items: 20 of one, weighted by its `prop`.
const LIFE_ALCHEMY_POTIONS: [(i32, u32); 6] = [
    (2022199, 1),
    (2022224, 5),
    (2022225, 5),
    (2022226, 5),
    (2022227, 5),
    (2022228, 5),
];
const LAB_YULETE: i32 = 2112014;
const YULETES_REWARD: i16 = 3382;
const ZENUMIST_MARBLE: i32 = 4001159;
const ALCADNO_MARBLE: i32 = 4001160;
const HORUS_EYE: i32 = 1122010;
const ROCK_OF_WISDOM: i32 = 2041212;

/// Whether the player is under the poison of Russellon's pills.
fn took_the_pill(player: &Player) -> bool {
    (player.buffs.active().iter())
        .any(|buff| buff.stat == game::BuffStat::Poison && buff.skill_id == PILL_POISON)
}

/// Whether the desk has a pill for the player: on "Life Alchemy, and the
/// Missing Alchemist" (3314), holding none and not under one.
fn takes_a_pill(player: &Player) -> bool {
    player.quests.state(LIFE_ALCHEMY) == QuestState::Started
        && player.inventory.count(RUSSELLONS_PILLS) == 0
        && !took_the_pill(player)
}

/// The desk in Alcadno's upper lab: the player takes one of Russellon's
/// pills, thinking so (Cosmic `2111015.js`).
fn pill_desk() -> NpcScript {
    let mut script = NpcScript::new(
        PILL_DESK,
        vec![Page::Act {
            text: text(keys::RUSSELLON_PILL_TAKEN),
            effects: vec![Effect::GiveItem {
                item_id: RUSSELLONS_PILLS,
                quantity: 1,
            }],
        }],
    );
    script.player_speaks = true;
    script.inventory_full = Some(text(keys::RUSSELLON_PILL_FULL));
    script
}

/// `q3314e`: Russellon sees the pill's work on the player, completes the
/// quest with its `Act/1` rewards, which a script's completion doesn't
/// give, and cures them (an All Cure Potion's effect); without it, sends
/// them back to take one (Cosmic `3314.js`).
fn life_alchemy_done(player: &Player, rng: &mut impl Rng) -> NpcScript {
    if !took_the_pill(player) {
        return NpcScript::new(
            RUSSELLON,
            vec![Page::Say(text(keys::RUSSELLON_PILL_NOT_TAKEN))],
        );
    }
    let &(item_id, _) = LIFE_ALCHEMY_POTIONS
        .choose_weighted(rng, |&(_, weight)| weight)
        .expect("weights");
    let mut script = NpcScript::new(RUSSELLON, vec![Page::Say(text(keys::RUSSELLON_PILL_DONE))]);
    script.on_end = vec![
        Effect::GiveItem {
            item_id,
            quantity: 20,
        },
        Effect::GainExp(LIFE_ALCHEMY_EXP),
        Effect::CompleteQuest(LIFE_ALCHEMY),
        Effect::ApplyItemBuff(ALL_CURE_POTION),
    ];
    script.inventory_full = Some(text(keys::RUSSELLON_REWARD_FULL));
    script
}

/// `q3382e`: Yulete makes Horus' Eye from 25 of each marble, for one who
/// has none, or else a Rock of Wisdom from 10 of each (Cosmic `3382.js`).
fn yuletes_reward(player: &Player) -> NpcScript {
    let held = |count: i16| {
        player.inventory.count(ZENUMIST_MARBLE) >= i32::from(count)
            && player.inventory.count(ALCADNO_MARBLE) >= i32::from(count)
    };
    let has_eye = player.inventory.count(HORUS_EYE) > 0
        || (player.inventory.items(InventoryType::Equipped))
            .any(|(_, item)| item.item_id == HORUS_EYE);
    let (key, count, item_id) = if held(25) && !has_eye {
        (keys::YULETE_MARBLES_EYE, 25, HORUS_EYE)
    } else if held(10) {
        (keys::YULETE_MARBLES_ROCK, 10, ROCK_OF_WISDOM)
    } else {
        return NpcScript::new(LAB_YULETE, vec![Page::Say(text(keys::YULETE_MARBLES_MORE))]);
    };
    let mut script = NpcScript::new(LAB_YULETE, vec![Page::Say(text(key))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: ZENUMIST_MARBLE,
            quantity: count,
        },
        Effect::TakeItem {
            item_id: ALCADNO_MARBLE,
            quantity: count,
        },
        Effect::GiveItem {
            item_id,
            quantity: 1,
        },
        Effect::CompleteQuest(YULETES_REWARD),
    ];
    script.inventory_full = Some(text(keys::YULETE_MARBLES_FULL));
    script
}

// Karcasa (2101013) in Ariant, who keeps the queen's silk for "Stealing
// Queen's Order of Silk" (3941), on the way to Jano's "Mission Complete!"
// (3940).
const KARCASA: i32 = 2101013;
const QUEENS_SILK_QUEST: i16 = 3941;
const QUEENS_SILK: i32 = 4031571;

/// `q3941s` and `q3941e`: Karcasa takes a player disguised by Tigun's
/// potion for Tigun, starts the quest and, at its end, hands over the
/// queen's silk (the data's `Act/1`, which a script's completion doesn't
/// give); anyone else is sent off (Cosmic `3941.js`).
fn karcasa_silk(player: &Player, end: bool) -> NpcScript {
    if !scripts::disguised_as_tigun(player) {
        return NpcScript::new(KARCASA, vec![Page::Say(text(keys::KARCASA_SILK_STRANGER))]);
    }
    if !end {
        let pages = vec![
            Page::Say(text(keys::KARCASA_SILK_TIGUN)),
            Page::Say(text(keys::KARCASA_SILK_HANDED)),
        ];
        let mut script = NpcScript::new(KARCASA, pages);
        script.on_end = vec![Effect::StartQuest(QUEENS_SILK_QUEST)];
        return script;
    }
    let mut script = NpcScript::new(KARCASA, vec![Page::Say(text(keys::KARCASA_SILK_GIVEN))]);
    script.on_end = vec![
        Effect::GiveItem {
            item_id: QUEENS_SILK,
            quantity: 1,
        },
        Effect::CompleteQuest(QUEENS_SILK_QUEST),
    ];
    script.inventory_full = Some(text(keys::KARCASA_SILK_FULL));
    script
}

/// Adonis (9201106) in El Nath, who tells of the Zakum Diamonds for "Lost
/// Spirits" (8255). Its sequel (8256, `q8256s`), the Liberation Ritual that
/// powers up an iTCG relic with a diamond, isn't in: nothing in v83 gives the
/// relics, and its recipe has no source.
const ADONIS: i32 = 9201106;
const LOST_SPIRITS: i16 = 8255;

/// `q8255s`: Adonis's tale, and the quest completes as it starts (Cosmic
/// `8255.js`), with the data's 500 EXP (`Act/1`).
fn adonis_lost_spirits() -> NpcScript {
    let pages = vec![
        Page::Say(text(keys::ADONIS_SPIRITS_TALE)),
        Page::Say(text(keys::ADONIS_SPIRITS_RITUAL)),
    ];
    let mut script = NpcScript::new(ADONIS, pages);
    script.on_end = vec![Effect::FinishQuest(LOST_SPIRITS), Effect::GainExp(500)];
    script
}

/// Swallow (2071006) in Korean Folk Town, and "The Seed That Swallow Lost"
/// (3609), which a player who lost Swallow's Magic Seed (4031225) before
/// giving it to Nolbu or Hongbu starts from the data. It has no completion
/// NPC (`COMPLETED_BY_NPC_SCRIPT`): talking to Swallow again while it is
/// started gives a new seed (the data's `Act/1`) and completes it. Cosmic
/// has no script for it.
const SWALLOW: i32 = 2071006;
const THE_SEED_SWALLOW_LOST: i16 = 3609;
const SWALLOWS_SEED: i32 = 4031225;

fn swallow_new_seed() -> NpcScript {
    let mut script = NpcScript::new(SWALLOW, vec![Page::Say(text(keys::SWALLOW_NEW_SEED))]);
    script.on_end = vec![
        Effect::GiveItem {
            item_id: SWALLOWS_SEED,
            quantity: 1,
        },
        Effect::CompleteQuest(THE_SEED_SWALLOW_LOST),
    ];
    script.inventory_full = Some(text(keys::SWALLOW_NEW_SEED_FULL));
    script
}

/// Pia (1012102) in Henesys, and "A rainbow snail shell that makes wishes
/// come true!?" (2156), which ends on Pia's `q2156e`: the data's `Check/1`
/// asks for nothing and its `Act/1` gives nothing. The script asks for the
/// Rainbow-colored Snail Shell Mano drops, takes it and gives the GMS v83
/// data's 7,500 EXP, 30,000 mesos and 3 fame, and Pia plays the shell's
/// effect (`act2156`). Cosmic has no script for it.
const PIA: i32 = 1012102;
const RAINBOW_SHELL_QUEST: i16 = 2156;
const RAINBOW_SNAIL_SHELL: i32 = 2210006;

fn pia_rainbow_shell(player: &Player) -> NpcScript {
    if player.inventory.count(RAINBOW_SNAIL_SHELL) == 0 {
        return NpcScript::new(PIA, vec![Page::Say(text(keys::PIA_SHELL_MISSING))]);
    }
    let mut script = NpcScript::new(PIA, vec![Page::Say(text(keys::PIA_SHELL_WISH))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: RAINBOW_SNAIL_SHELL,
            quantity: 1,
        },
        Effect::GainExp(7500),
        Effect::GainMeso(30000),
        Effect::GainFame(3),
        Effect::CompleteQuest(RAINBOW_SHELL_QUEST),
        Effect::NpcSpecialAction {
            npc_id: PIA,
            action: "act2156",
        },
    ];
    script
}

/// June (1052125) on Kerning Square's 7th floor (`Depart_topFloorIn`,
/// also opened by the `topFloor` portal beside her): one helping Blake (on
/// 2286, 2287 or 2288) goes up to the Spirit of Rock's floors alone, and
/// is told so when all eight are taken. The VIP Zone opens only through
/// Admission to the VIP Zone (2291), so asking after it just says how
/// (Cosmic `1052125`).
const JUNE: i32 = events::JUNE;
const BLAKES_REQUESTS: [i16; 3] = [2286, 2287, 2288];
/// Admission to the VIP Zone (2291) needs the VIP pass quest before it
/// (2290) and 10 VIP Invitations, at levels 35 to 50.
const VIP_PASS: i16 = 2290;
const VIP_ADMISSION: i16 = 2291;
const VIP_INVITATION: i32 = 4032521;
const VIP_INVITATIONS: i16 = 10;
const VIP_MAX_LEVEL: u8 = 50;

fn june(player: &Player) -> NpcScript {
    let helping = BLAKES_REQUESTS
        .iter()
        .any(|&quest| player.quests.state(quest) == QuestState::Started);
    let blake = match helping {
        true => Page::Done(vec![Effect::StartEvent(EventKind::RockSpirit)]),
        false => Page::Say(text(keys::JUNE_NOT_HELPING)),
    };
    let vip = if player.quests.state(VIP_PASS) != QuestState::Completed {
        keys::JUNE_NOT_VIP
    } else if player.stats.level > VIP_MAX_LEVEL {
        keys::JUNE_VIP_OVER_LEVEL
    } else {
        keys::JUNE_VIP_INVITATIONS
    };
    let mut script = NpcScript::new(
        JUNE,
        vec![Page::Menu {
            text: text(keys::JUNE_MENU),
            choices: vec![
                (text(keys::JUNE_HELPING_BLAKE), vec![blake]),
                (text(keys::JUNE_VIP), vec![Page::Say(text(vip))]),
            ],
        }],
    );
    script.refused = Some(text(keys::JUNE_BUSY));
    script
}

/// `q2291e`, Admission to the VIP Zone: June takes the 10 VIP Invitations
/// and takes the player up to the VIP Zone alone, which completes the
/// quest (its `Act/1` is empty); when all eight runs are taken, nothing is
/// taken and June says so (Cosmic `2291.js`).
fn vip_admission(player: &Player) -> NpcScript {
    if player.inventory.count(VIP_INVITATION) < i32::from(VIP_INVITATIONS) {
        return NpcScript::new(JUNE, vec![Page::Say(text(keys::JUNE_VIP_MISSING))]);
    }
    let mut script = NpcScript::new(JUNE, vec![Page::Say(text(keys::JUNE_VIP_WELCOME))]);
    script.on_end = vec![
        Effect::TakeItem {
            item_id: VIP_INVITATION,
            quantity: VIP_INVITATIONS,
        },
        Effect::CompleteQuest(VIP_ADMISSION),
        Effect::StartEvent(EventKind::RockSpiritVip),
    ];
    script.refused = Some(text(keys::JUNE_BUSY));
    script
}

/// The Sealed Shrine's Entrance pillar (2091009) between Mu Lung's
/// temple pillars: the password lets one who has yet to beat the Shadow
/// Knight in, one at a time (Cosmic `2091009.js`).
const SHRINE_PILLAR: i32 = 2091009;
const SHRINE_PASSWORD: &str = "Actions speak louder than words";
const SHRINE_PASSWORD_MAX: i16 = 40;

fn shrine_pillar(player: &Player) -> NpcScript {
    let quest = scripts::CATCH_THE_SHADOW_KNIGHT;
    let hunting = player.quests.state(quest) == QuestState::Started
        && player.quests.kills(quest).iter().all(|&count| count == 0);
    let mut script = NpcScript::new(
        SHRINE_PILLAR,
        vec![Page::AskText {
            text: text(keys::SHRINE_PILLAR_ASK),
            min: 1,
            max: SHRINE_PASSWORD_MAX,
            on_text: OnText::ShrinePillar { hunting },
        }],
    );
    script.refused = Some(text(keys::SHRINE_PILLAR_BUSY));
    script
}

/// The password, in any case and with or without a final ".", lets one
/// hunting the Shadow Knight into the shrine; anyone else hears it's right
/// but the way stays shut, in pink.
fn shrine_pillar_opens(text_given: &str, hunting: bool) -> Vec<Page> {
    let given = text_given.trim();
    let given = given.strip_suffix('.').unwrap_or(given).trim_end();
    if !given.eq_ignore_ascii_case(SHRINE_PASSWORD) {
        return vec![Page::Say(text(keys::SHRINE_PILLAR_WRONG))];
    }
    if !hunting {
        return vec![Page::Done(vec![Effect::Pink(text(
            keys::SHRINE_PILLAR_SHUT,
        ))])];
    }
    vec![Page::Done(vec![
        Effect::RequireEmptyMap(scripts::SEALED_SHRINE),
        Effect::PortalSound,
        Effect::WarpToPortal(scripts::SEALED_SHRINE, 0),
    ])]
}

/// The statues of the Puppeteer's Secret Passage (910510100): the one at
/// its end leads into his arena, one challenger at a time (Cosmic
/// `1063017.js`), and the one at its start back out to the cave
/// (`1063016.js`).
const PASSAGE_END_STATUE: i32 = 1063017;
const PASSAGE_EXIT_STATUE: i32 = 1063016;
const PASSAGE_WAY_OUT: i32 = 105040201;

fn passage_end_statue() -> NpcScript {
    let mut script = NpcScript::new(
        PASSAGE_END_STATUE,
        vec![Page::Ask {
            text: text(keys::PASSAGE_STATUE_ASK),
            on_yes: vec![
                Effect::RequireEmptyMap(scripts::PUPPETEERS_ARENA),
                Effect::WarpToPortal(scripts::PUPPETEERS_ARENA, 0),
            ],
            on_no: None,
        }],
    );
    script.refused = Some(text(keys::PASSAGE_STATUE_BUSY));
    script
}

fn passage_exit_statue() -> NpcScript {
    NpcScript::new(
        PASSAGE_EXIT_STATUE,
        vec![Page::Ask {
            text: text(keys::PASSAGE_EXIT_ASK),
            on_yes: vec![Effect::WarpToPortal(PASSAGE_WAY_OUT, 2)],
            on_no: None,
        }],
    )
}

/// Ghosthunter Bob, in the Clocktower's lobby.
const GHOSTHUNTER_BOB: i32 = 2041026;
const A_CUTE_BABY_BIRD: i16 = 3250;
/// The Timer's Egg, a raise item: Bird Feed grows it in record 7067.
const TIMERS_EGG: i32 = 4220046;

/// `q3250s`: Bob offers a Timer's Egg to raise (`Say.img/3250/0` has the
/// pages), unless one is already held.
fn baby_bird_offer(player: &Player) -> NpcScript {
    let mut on_yes = Vec::new();
    if player.inventory.count(TIMERS_EGG) == 0 {
        on_yes.push(Effect::GiveItem {
            item_id: TIMERS_EGG,
            quantity: 1,
        });
    }
    on_yes.push(Effect::StartQuest(A_CUTE_BABY_BIRD));
    let [feeding, tamed, ask] = keys::BOB_BABY_BIRD_OFFER;
    let [given, back] = keys::BOB_BABY_BIRD_GIVEN;
    let mut script = NpcScript::new(
        GHOSTHUNTER_BOB,
        vec![
            Page::Say(text(feeding)),
            Page::Say(text(tamed)),
            Page::Ask {
                text: text(ask),
                on_yes,
                on_no: Some(text(keys::BOB_BABY_BIRD_DECLINED)),
            },
            Page::Say(text(given)),
            Page::Say(text(back)),
        ],
    );
    script.inventory_full = Some(text(keys::BOB_BABY_BIRD_FULL));
    script
}

/// `q3250e`: the bird grown (record 7067 at 3,000, `Check.img/3250/1`),
/// Bob takes it back. The quest gives nothing (`Act.img/3250/1`).
fn baby_bird_grown() -> NpcScript {
    let mut script = NpcScript::new(
        GHOSTHUNTER_BOB,
        vec![Page::Say(text(keys::BOB_BABY_BIRD_GROWN))],
    );
    script.on_end = vec![
        Effect::TakeItem {
            item_id: TIMERS_EGG,
            quantity: 1,
        },
        Effect::CompleteQuest(A_CUTE_BABY_BIRD),
    ];
    script
}

/// Captain Al, in Lith Harbor (104000000) and Orbis Park (200000200).
const CAPTAIN_AL: i32 = 1002103;
/// The end scripts that read the Family window's numbers: Captain Al's
/// and Dalair's Outstanding Citizen ([`NpcScript::for_family_quest`]).
pub const FAMILY_QUEST_SCRIPTS: [&str; 4] = ["q2232e", "q2233e", "q2234e", "q29508e"];
/// Nine Spirit's Baby Dragon, in its nest (240040612).
const BABY_DRAGON: i32 = 2081011;
const NINE_SPIRIT_EGG: i32 = 4001094;
const DRAGON_STONE: i32 = 2041200;

/// `q3714s`, the Remnants of Horned Tail: the egg from Horned Tail hatches
/// on the nest and the baby dragon gives up a Dragon Stone, with Cosmic's
/// 42,000 EXP (the data has no rewards). The quest repeats (`interval 0`),
/// once for each egg, but not while a Dragon Stone is held, as Cosmic.
fn baby_dragon(player: &Player) -> NpcScript {
    if player.inventory.count(DRAGON_STONE) > 0 {
        return NpcScript::new(
            BABY_DRAGON,
            vec![Page::Say(text(keys::NINE_SPIRIT_STONE_HELD))],
        );
    }
    let mut script = NpcScript::new(
        BABY_DRAGON,
        vec![Page::Say(text(keys::NINE_SPIRIT_HATCHED))],
    );
    script.on_end = vec![
        Effect::FinishQuest(3714),
        Effect::TakeItem {
            item_id: NINE_SPIRIT_EGG,
            quantity: 1,
        },
        Effect::GiveItem {
            item_id: DRAGON_STONE,
            quantity: 1,
        },
        Effect::GainExp(42_000),
    ];
    script.inventory_full = Some(text(keys::NINE_SPIRIT_FULL));
    script
}

/// `q2216s`-`q2219s`: the informant at `index` tells what they know about
/// Dyle, which completes the quest as it starts, for 7,000 EXP (Cosmic's;
/// the data has none). The last of the four heard takes back JM's list.
fn informant(player: &Player, index: usize) -> Option<NpcScript> {
    let (quest, npc) = INFORMANTS[index];
    let mut script = NpcScript::new(npc, vec![Page::Say(text(keys::DYLE_INFORMANTS[index]))]);
    script.on_end = vec![Effect::FinishQuest(quest), Effect::GainExp(7000)];
    let rest_heard = INFORMANTS
        .iter()
        .filter(|&&(other, _)| other != quest)
        .all(|&(other, _)| player.quests.state(other) == QuestState::Completed);
    if rest_heard && player.inventory.count(INFORMANT_LIST) > 0 {
        script.on_end.push(Effect::TakeItem {
            item_id: INFORMANT_LIST,
            quantity: 1,
        });
    }
    Some(script)
}

/// `q8185e`, `q4659e` and `q8189e`: Garnox spends a Rock of Evolution and
/// his fee to evolve the first pet out (the one the quest checked) into a
/// kind picked by weight, and the quest completes; 4659 also takes its
/// Cheap Batteries.
fn pet_evolution(player: &Player, quest: i16) -> NpcScript {
    let say = |key| NpcScript::new(GARNOX, vec![Page::Say(text(key))]);
    let Some(item) = player.summoned_pet(0) else {
        return say(keys::PET_EVOLUTION_NO_PET);
    };
    let (Some(serial), Some(pet)) = (item.cash_serial, item.pet.as_ref()) else {
        return say(keys::PET_EVOLUTION_NO_PET);
    };
    let evolutions = world::rock_evolutions(item.item_id);
    let Some(into) = world::roll_evolution(&evolutions, &mut rand::thread_rng()) else {
        return say(keys::PET_EVOLUTION_NO_PET);
    };
    let days = items::info(into).map_or(0, |info| info.pet_life);
    if days <= 0 {
        return say(keys::PET_EVOLUTION_NO_PET);
    }
    if player.meso < EVOLUTION_FEE {
        return say(keys::PET_EVOLUTION_NO_MESOS);
    }
    let name = if pet.name == items::pet_name(item.item_id) {
        items::pet_name(into)
    } else {
        pet.name.clone()
    };
    let says = |key| content::text().get(key, &[("name", &name), ("kind", &items::pet_name(into))]);
    let mut on_yes = vec![
        Effect::TakeItem {
            item_id: world::ROCK_OF_EVOLUTION,
            quantity: 1,
        },
        Effect::SpendMeso(EVOLUTION_FEE),
    ];
    if quest == ROBO_UPGRADE_QUEST {
        on_yes.push(Effect::TakeItem {
            item_id: CHEAP_BATTERY,
            quantity: 50,
        });
    }
    on_yes.push(Effect::EvolvePet { serial, into, days });
    on_yes.push(Effect::CompleteQuest(quest));
    let mut script = NpcScript::new(
        GARNOX,
        vec![
            Page::Ask {
                text: content::text().get(keys::PET_EVOLUTION_ASK, &[("name", &pet.name)]),
                on_yes,
                on_no: Some(text(keys::PET_EVOLUTION_DECLINE)),
            },
            Page::Say(says(keys::PET_EVOLUTION_DONE)),
        ],
    );
    script.refused = Some(text(keys::PET_EVOLUTION_REFUSED));
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

/// `q1028s`, To Lith Harbor!: Lucas has asked Shanks to take the player,
/// who carries Lucas's message for Olaf, so Shanks sails to Lith Harbor for
/// free and the quest starts. A no stays (`QuestInfo` 1028; no Cosmic
/// script). The data's start also asks for the Wizet Plain Suit
/// (1042003), a GM item, which shelves the quest for players. The ask is
/// unsourced.
fn shanks_sails_for_lucas() -> NpcScript {
    let mut script = NpcScript::new(
        SHANKS,
        vec![Page::Ask {
            text: text(keys::SHANKS_FOR_LUCAS),
            on_yes: Vec::new(),
            on_no: Some(text(keys::SHANKS_DECLINE)),
        }],
    );
    script.on_end = vec![Effect::StartQuest(1028), Effect::Warp(LITH_HARBOR)];
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

/// What Lenario's script needs to know of the player: their guild, its
/// alliance, and whether they lead their party.
#[derive(Clone, Copy, Debug, Default)]
pub struct AllianceView<'a> {
    pub guild: Option<&'a Guild>,
    pub alliance: Option<&'a Alliance>,
    pub party_leader: bool,
}

/// What a wedding assistant needs besides the player: the couple's booked
/// wedding, if any, the soonest wedding at the assistant's venue the
/// player is a guest of, the player's channel, and the time.
pub struct WeddingView<'a> {
    pub reservation: Option<&'a Reservation>,
    pub guest_of: Option<&'a Reservation>,
    pub channel: u8,
    pub now: SystemTime,
}

/// Lenario, beside Heracle, founds, grows, and disbands alliances for
/// guild masters (the HeavenMS script's menu).
fn lenario(player: &Player, view: AllianceView) -> NpcScript {
    let cid = player.stats.id;
    if !view.guild.is_some_and(|guild| guild.is_master(cid)) {
        return NpcScript::new(LENARIO, vec![Page::Say(text(keys::LENARIO_NOT_MASTER))]);
    }
    let master = view.alliance.filter(|alliance| alliance.is_master(cid));
    let choices = vec![
        (
            text(keys::LENARIO_INFO),
            vec![Page::Say(text(keys::LENARIO_INFO_SAY))],
        ),
        (
            text(keys::LENARIO_HOW),
            vec![Page::Say(text(keys::LENARIO_HOW_SAY))],
        ),
        (text(keys::LENARIO_CREATE), create_alliance(player, &view)),
        (text(keys::LENARIO_GROW), grow_alliance(player, master)),
        (text(keys::LENARIO_DISBAND), vec![disband_alliance(master)]),
    ];
    NpcScript::new(
        LENARIO,
        vec![Page::Menu {
            text: text(keys::LENARIO_MENU),
            choices,
        }],
    )
}

/// The party's leader, a guild master in no alliance, pays, then names it.
/// Like the HeavenMS script it asks first and only then says the player
/// can't pay.
fn create_alliance(player: &Player, view: &AllianceView) -> Vec<Page> {
    if view.guild.is_some_and(|guild| guild.alliance_id != 0) {
        return vec![Page::Say(text(keys::LENARIO_IN_ALLIANCE))];
    }
    if !view.party_leader {
        return vec![Page::Say(text(keys::LENARIO_NOT_LEADER))];
    }
    let price = alliance::CREATE_COST.to_string();
    let vars = [("price", price.as_str())];
    let next = if player.meso < alliance::CREATE_COST {
        Page::Say(content::text().get(keys::LENARIO_CREATE_NO_MESOS, &vars))
    } else {
        name_alliance(false)
    };
    vec![
        Page::Ask {
            text: content::text().get(keys::LENARIO_CREATE_ASK, &vars),
            on_yes: Vec::new(),
            on_no: Some(text(keys::LENARIO_DECLINE)),
        },
        next,
    ]
}

/// The prompt for the alliance's name; `again` after one that can't be
/// used.
fn name_alliance(again: bool) -> Page {
    Page::AskText {
        text: text(if again {
            keys::LENARIO_NAME_AGAIN
        } else {
            keys::LENARIO_NAME
        }),
        min: *alliance::NAME_LEN.start(),
        max: *alliance::NAME_LEN.end(),
        on_text: OnText::NameAlliance,
    }
}

/// Lenario asks whether the name is right, and asks for another if it
/// can't be one.
fn confirm_alliance_name(name: &str) -> Vec<Page> {
    if !alliance::name_is_valid(name) {
        return vec![name_alliance(true)];
    }
    vec![Page::Ask {
        text: content::text().get(keys::LENARIO_NAME_CONFIRM, &[("name", name)]),
        on_yes: vec![Effect::CreateAlliance {
            name: name.to_string(),
        }],
        on_no: Some(text(keys::LENARIO_DECLINE)),
    }]
}

/// Room for one more guild, for the alliance's master.
fn grow_alliance(player: &Player, master: Option<&Alliance>) -> Vec<Page> {
    let Some(master) = master else {
        return vec![Page::Say(text(keys::LENARIO_GROW_NOT_MASTER))];
    };
    if master.capacity >= alliance::MAX_CAPACITY {
        return vec![Page::Say(text(keys::LENARIO_GROW_FULL))];
    }
    let price = alliance::GROW_COST.to_string();
    let capacity = (master.capacity + 1).to_string();
    let vars = [("price", price.as_str()), ("capacity", capacity.as_str())];
    let can_pay = player.meso >= alliance::GROW_COST;
    let on_yes = if can_pay {
        vec![Effect::GrowAlliance {
            from: master.capacity,
        }]
    } else {
        Vec::new()
    };
    let mut pages = vec![Page::Ask {
        text: content::text().get(keys::LENARIO_GROW_ASK, &vars),
        on_yes,
        on_no: Some(text(keys::LENARIO_DECLINE)),
    }];
    if !can_pay {
        pages.push(Page::Say(
            content::text().get(keys::LENARIO_GROW_NO_MESOS, &vars),
        ));
    }
    pages
}

fn disband_alliance(master: Option<&Alliance>) -> Page {
    match master {
        Some(_) => Page::Ask {
            text: text(keys::LENARIO_DISBAND_ASK),
            on_yes: vec![Effect::DisbandAlliance],
            on_no: Some(text(keys::LENARIO_DECLINE)),
        },
        None => Page::Say(text(keys::LENARIO_DISBAND_NOT_MASTER)),
    }
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
        (route.ticket, route.price),
        player.meso >= route.price,
        ask,
        keys::SHIPS_SELL_DECLINE,
        keys::SHIPS_SELL_SHORT,
    )
}

/// A `ticket` for `price`, asked about with `ask`: no says `decline`, and
/// a player who can't buy it hears `short` after yes.
fn sell(
    (ticket, price): (i32, i32),
    can_pay: bool,
    ask: String,
    decline: &str,
    short: &str,
) -> Vec<Page> {
    let on_yes = if can_pay {
        vec![
            Effect::SpendMeso(price),
            Effect::GiveItem {
                item_id: ticket,
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
        let price = price.to_string();
        pages.push(Page::Say(content::text().get(short, &[("price", &price)])));
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
    // A yes that comes after boarding closed (`RequireBoarding`).
    let mut script = NpcScript::new(npc_id, vec![page]);
    script.refused = Some(text(keys::SHIPS_BOARD_CLOSING));
    script
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

/// `NLC_Move`: a subway gate, as of route time `t`.
fn subway_gate(npc_id: i32, route: &Route, player: &Player, t: i64) -> NpcScript {
    let tickets = SUBWAY_LINES
        .iter()
        .find(|line| line.route.platform == route.platform)
        .map_or([route.ticket; 2], SubwayLine::tickets);
    let says = GateText {
        ask: keys::SUBWAY_BOARD_ASK,
        decline: keys::SUBWAY_BOARD_DECLINE,
        no_ticket: keys::SUBWAY_BOARD_NO_TICKET,
        closing: keys::SUBWAY_BOARD_CLOSING,
        full: keys::SUBWAY_BOARD_FULL,
    };
    let mut pages = gate(route, &tickets, player, t, &says);
    if npc_id == KERNING_CITY_GATE {
        pages = vec![Page::Menu {
            text: text(keys::LINE3_GATE_MENU),
            choices: vec![
                (
                    text(keys::LINE3_GATE_SQUARE),
                    vec![Page::Done(vec![Effect::WarpToPortal(
                        rides::KERNING_TO_SQUARE.map,
                        0,
                    )])],
                ),
                (text(keys::LINE3_GATE_SITE), construction_site(player)),
                (text(keys::LINE3_GATE_SUBWAY), pages),
            ],
        }];
    }
    // A yes that comes after boarding closed (`RequireBoarding`).
    let mut script = NpcScript::new(npc_id, pages);
    script.refused = Some(text(says.closing));
    script
}

/// The Kerning City gate's way into Line 3's construction sites: a choice
/// of the site tickets held, taking the one chosen.
fn construction_site(player: &Player) -> Vec<Page> {
    let choices: Vec<_> = SITE_TICKETS
        .iter()
        .zip(SITE_ENTRANCES)
        .filter(|&(&ticket, _)| player.inventory.count(ticket) > 0)
        .map(|(&ticket, map)| {
            let effects = vec![
                Effect::TakeItem {
                    item_id: ticket,
                    quantity: 1,
                },
                Effect::WarpToPortal(map, crate::scripts::named(map, "st00")),
            ];
            (format!("#t{ticket}#"), vec![Page::Done(effects)])
        })
        .collect();
    if choices.is_empty() {
        return vec![Page::Say(text(keys::LINE3_GATE_NO_TICKET))];
    }
    vec![Page::Menu {
        text: text(keys::LINE3_GATE_TICKETS),
        choices,
    }]
}

/// Jake at the ticketing booth: a ticket to each construction site whose
/// quest the player has begun, in order of the sites.
fn jake(player: &Player) -> NpcScript {
    let price = SITE_TICKET_PRICE.to_string();
    let price = [("price", price.as_str())];
    let mut pages = vec![Page::Say(text(keys::LINE3_JAKE_HELLO))];
    let sites = SITE_QUESTS
        .iter()
        .filter(|&&quest| player.quests.state(quest) != QuestState::NotStarted)
        .count();
    if sites > 0 {
        let buy = |ticket: i32| {
            if player.meso < SITE_TICKET_PRICE {
                Page::Say(content::text().get(keys::LINE3_JAKE_SHORT, &price))
            } else {
                Page::Done(vec![
                    Effect::SpendMeso(SITE_TICKET_PRICE),
                    Effect::GiveItem {
                        item_id: ticket,
                        quantity: 1,
                    },
                ])
            }
        };
        pages.push(Page::Menu {
            text: content::text().get(keys::LINE3_JAKE_MENU, &price),
            choices: SITE_TICKETS[..sites]
                .iter()
                .map(|&ticket| (format!("#t{ticket}#"), vec![buy(ticket)]))
                .collect(),
        });
    }
    let mut script = NpcScript::new(JAKE, pages);
    script.inventory_full = Some(text(keys::LINE3_JAKE_FULL));
    script
}

/// The way out of a construction site, back to the ticketing booth; a no
/// gets a word.
fn site_exit() -> NpcScript {
    NpcScript::new(
        SITE_EXIT,
        vec![
            Page::Say(text(keys::LINE3_EXIT_SAY)),
            Page::Ask {
                text: text(keys::LINE3_EXIT_ASK),
                on_yes: vec![Effect::WarpToPortal(SUBWAY_TICKETING, 0)],
                on_no: Some(text(keys::LINE3_EXIT_STAY)),
            },
        ],
    )
}

/// Whether character `slot` of a started info quest's string reads `from`.
fn info_reads(player: &Player, quest: i16, slot: u8, from: u8) -> bool {
    (player.quests.info(quest)).and_then(|info| info.as_bytes().get(usize::from(slot)).copied())
        == Some(from)
}

/// The oasis, the secret wall, and Magatia's wall each move their quest
/// on by one character as the player looks; otherwise they are silent.
fn info_setter(
    (npc, quest, slot, from, to): (i32, i16, u8, u8, u8),
    player: &Player,
) -> Option<NpcScript> {
    if !info_reads(player, quest, slot, from) {
        return None;
    }
    let set = vec![Effect::SetQuestChar {
        quest,
        slot,
        from,
        to,
    }];
    let pages = match npc {
        MAGATIA_WALL => vec![
            Page::Ask {
                text: text(keys::QUEST_INFO_WALL_ASK),
                on_yes: set,
                on_no: None,
            },
            Page::Say(text(keys::QUEST_INFO_WALL_READ)),
        ],
        PALACE_OASIS => vec![Page::Act {
            text: text(keys::QUEST_INFO_OASIS),
            effects: set,
        }],
        _ => vec![Page::Act {
            text: text(keys::QUEST_INFO_SECRET_WALL),
            effects: set,
        }],
    };
    Some(NpcScript::new(npc, pages))
}

/// Tangyoon, while Find Fresh Milk is under way: an empty bottle, and off
/// to the stable.
fn tangyoon() -> NpcScript {
    let mut script = NpcScript::new(
        TANGYOON,
        vec![
            Page::Say(text(keys::MILK_TANGYOON[0])),
            Page::Say(text(keys::MILK_TANGYOON[1])),
            Page::Done(vec![
                Effect::GiveItem {
                    item_id: MILK_BOTTLES[0],
                    quantity: 1,
                },
                Effect::Warp(STABLE),
            ]),
        ],
    );
    script.inventory_full = Some(text(keys::MILK_TANGYOON_FULL));
    script
}

/// A mother cow fills the first bottle held that isn't full by a third,
/// unless she was the last to be milked.
fn mother_cow(npc: i32, cow: u16, player: &Player) -> NpcScript {
    let last = player.quests.kills(FRESH_MILK).first() == Some(&cow);
    let held = MILK_BOTTLES[..3]
        .iter()
        .position(|&bottle| player.inventory.count(bottle) > 0);
    let page = match held {
        _ if last => Page::Say(text(keys::MILK_COW_LAST)),
        Some(stage) => Page::Act {
            text: text(keys::MILK_COW_FILLS[stage]),
            effects: vec![
                Effect::TakeItem {
                    item_id: MILK_BOTTLES[stage],
                    quantity: 1,
                },
                Effect::GiveItem {
                    item_id: MILK_BOTTLES[stage + 1],
                    quantity: 1,
                },
                Effect::QuestRecord {
                    quest_id: FRESH_MILK,
                    count: cow,
                },
            ],
        },
        None => Page::Say(text(keys::MILK_COW_NO_BOTTLE)),
    };
    let mut script = NpcScript::new(npc, vec![page]);
    script.inventory_full = Some(text(keys::MILK_COW_FULL));
    script
}

/// A calf drinks a bottle with any milk in it back to empty.
fn calf(npc: i32, player: &Player) -> NpcScript {
    let held = |bottle: &i32| player.inventory.count(*bottle) > 0;
    let page = if held(&MILK_BOTTLES[0]) {
        Page::Say(text(keys::MILK_CALF_EMPTY))
    } else if let Some(&bottle) = MILK_BOTTLES[1..].iter().find(|bottle| held(bottle)) {
        Page::Act {
            text: text(keys::MILK_CALF_DRINKS),
            effects: vec![
                Effect::TakeItem {
                    item_id: bottle,
                    quantity: 1,
                },
                Effect::GiveItem {
                    item_id: MILK_BOTTLES[0],
                    quantity: 1,
                },
            ],
        }
    } else {
        Page::Say(text(keys::MILK_CALF_NO_BOTTLE))
    };
    NpcScript::new(npc, vec![page])
}

/// The Shiny Stone marks Shining Stone once while it is under way;
/// otherwise it says nothing of its own.
fn shiny_stone(player: &Player) -> Option<NpcScript> {
    let started = player.quests.state(SHINING_STONE) == QuestState::Started;
    if !started || player.quests.info(SHINING_STONE) == Some(SHINING_STONE_TOUCHED) {
        return None;
    }
    Some(NpcScript::new(
        SHINY_STONE,
        vec![Page::Act {
            text: text(keys::QUEST_INFO_SHINY_STONE),
            effects: vec![Effect::SetQuestInfo {
                quest: SHINING_STONE,
                value: SHINING_STONE_TOUCHED.to_owned(),
            }],
        }],
    ))
}

/// A Shaman Rock takes a charm while its character of How to Shoo Away the
/// Evil's string is still '0'; otherwise it says nothing of its own.
/// `nth` tells apart the same rock twice on a map.
pub fn shaman_rock(npc_id: i32, nth: u8, player: &Player) -> Option<NpcScript> {
    let &(.., slot) = SHAMAN_ROCKS
        .iter()
        .find(|&&(map, rock, which, _)| (map, rock, which) == (player.stats.map_id, npc_id, nth))?;
    if !info_reads(player, SHOO_AWAY_THE_EVIL, slot, b'0')
        || player.inventory.count(SHAMAN_CHARM) == 0
    {
        return None;
    }
    Some(NpcScript::new(
        npc_id,
        vec![Page::Act {
            text: text(keys::QUEST_INFO_SHAMAN_ROCK),
            effects: vec![
                Effect::SetQuestChar {
                    quest: SHOO_AWAY_THE_EVIL,
                    slot,
                    from: b'0',
                    to: b'1',
                },
                Effect::TakeItem {
                    item_id: SHAMAN_CHARM,
                    quantity: 1,
                },
            ],
        }],
    ))
}

/// A meteorite gives its sample without a word while the quest is under
/// way, if its slot still reads '0', or again once every slot reads '1'
/// and the player holds fewer than six.
fn meteorite(npc: i32, player: &Player) -> Option<NpcScript> {
    let slot = u8::try_from(npc - METEORITES.start()).ok()?;
    let give = Effect::GiveItem {
        item_id: METEORITE_SAMPLE,
        quantity: 1,
    };
    let effects = if info_reads(player, METEORITE_SAMPLES, slot, b'0') {
        vec![
            Effect::SetQuestChar {
                quest: METEORITE_SAMPLES,
                slot,
                from: b'0',
                to: b'1',
            },
            give,
        ]
    } else if player.quests.info(METEORITE_SAMPLES) == Some("111111")
        && player.inventory.count(METEORITE_SAMPLE) < 6
    {
        vec![give]
    } else {
        return None;
    };
    let mut script = NpcScript::new(npc, vec![Page::Done(effects)]);
    script.inventory_full = Some(text(keys::METEORITE_FULL));
    Some(script)
}

/// A string of Eliza's harp sounds its note, and while Eliza's Eternal
/// Sleep is under way counts it toward the tune or, if it is the wrong
/// one, starts the tune over.
fn harp_string(npc: i32, player: &Player) -> Option<NpcScript> {
    let string = usize::try_from(npc - HARP_STRINGS.start()).ok()?;
    let mut effects = vec![Effect::SoundToMap(format!("orbis/{}", HARP_SOUNDS[string]))];
    let played = Some(player)
        .filter(|player| player.quests.state(ELIZAS_ETERNAL_SLEEP) == QuestState::Started)
        .and_then(|player| {
            player
                .quests
                .info(ELIZAS_ETERNAL_SLEEP)?
                .parse::<usize>()
                .ok()
        })
        .filter(|&played| played < TWINKLE_TWINKLE.len());
    if let Some(played) = played {
        let set = |played: usize| Effect::SetQuestInfo {
            quest: ELIZAS_ETERNAL_SLEEP,
            value: played.to_string(),
        };
        if TWINKLE_TWINKLE[played] != HARP_NOTES[string] {
            effects.extend([
                set(0),
                Effect::Pink(text(keys::QUEST_INFO_HARP_MISSED)),
                Effect::FieldEffect("quest/party/wrong_kor".into()),
                Effect::Sound("Party1/Failed".into()),
            ]);
        } else {
            effects.push(set(played + 1));
            if played + 1 == TWINKLE_TWINKLE.len() {
                effects.extend([
                    Effect::Pink(text(keys::QUEST_INFO_HARP_PLAYED)),
                    Effect::FieldEffect("quest/party/clear".into()),
                    Effect::Sound("Party1/Clear".into()),
                ]);
            }
        }
    }
    Some(NpcScript::new(npc, vec![Page::Done(effects)]))
}

/// A circle of the Magic Pentagram, while its quest is under way and not
/// yet done: the next in turn counts, the last taking the stones and
/// saying so; any other starts the count over without a word.
fn pentagram_circle(npc: i32, player: &Player) -> Option<NpcScript> {
    if player.quests.state(MAGIC_PENTAGRAM) != QuestState::Started {
        return None;
    }
    let touched = player.quests.info(MAGIC_PENTAGRAM)?;
    let circle = npc - PENTAGRAM_CIRCLES.start();
    let last = *PENTAGRAM_CIRCLES.end() - PENTAGRAM_CIRCLES.start();
    if touched == (last + 1).to_string() {
        return None;
    }
    let set = |touched: i32| Effect::SetQuestInfo {
        quest: MAGIC_PENTAGRAM,
        value: touched.to_string(),
    };
    let stones = MAGIC_STONES
        .iter()
        .all(|&stone| player.inventory.count(stone) > 0);
    let page = if touched != circle.to_string() || circle == last && !stones {
        Page::Done(vec![set(0)])
    } else if circle == last {
        let mut effects = vec![set(circle + 1)];
        effects.extend(MAGIC_STONES.map(|item_id| Effect::TakeItem {
            item_id,
            quantity: 1,
        }));
        Page::Act {
            text: text(keys::QUEST_INFO_PENTAGRAM),
            effects,
        }
    } else {
        Page::Done(vec![set(circle + 1)])
    };
    Some(NpcScript::new(npc, vec![page]))
}

/// A house in Ariant takes its delivery without a word, if it is still
/// due and the player holds one.
fn ariant_delivery(
    (npc, quest, item, slot): (i32, i16, i32, u8),
    player: &Player,
) -> Option<NpcScript> {
    if !info_reads(player, quest, slot, b'2') || player.inventory.count(item) == 0 {
        return None;
    }
    Some(NpcScript::new(
        npc,
        vec![Page::Done(vec![
            Effect::SetQuestChar {
                quest,
                slot,
                from: b'2',
                to: b'3',
            },
            Effect::TakeItem {
                item_id: item,
                quantity: 1,
            },
        ])],
    ))
}

/// An item maker: pick a kind, then an item of it; the maker lists what it
/// takes, and a yes trades all of it for the item at once.
fn item_maker(npc_id: i32, hello: &str, kinds: &[(&str, &[Recipe])]) -> NpcScript {
    let choices = kinds
        .iter()
        .map(|&(label, recipes)| {
            let pages = recipes.iter().map(|recipe| make(recipe, None)).collect();
            (text(label), vec![recipe_menu(recipes, pages)])
        })
        .collect();
    let menu = Page::Menu {
        text: text(hello),
        choices,
    };
    with_maker_words(NpcScript::new(npc_id, vec![menu]))
}

/// The shoes, gloves and weapons makers who take a stimulator too: what
/// one is, then each kind of `plain` items, then each kind of `stimulated`
/// ones. A stimulator of the item's kind varies its stats as a drop's are,
/// or loses everything one time in `STIMULATOR_LOSS` (rolled as the
/// conversation opens), when the maker says `lost`.
fn stimulator_choices(
    explained: &str,
    lost: &'static str,
    plain: &[(&str, &[Recipe])],
    stimulated: &[(&str, &[Recipe])],
    rng: &mut impl Rng,
) -> Vec<(String, Vec<Page>)> {
    let mut choices = vec![(text(keys::MAKER_MOS_WHAT), vec![Page::Say(text(explained))])];
    for &(label, recipes) in plain {
        let pages = recipes.iter().map(|recipe| make(recipe, None)).collect();
        choices.push((text(label), vec![recipe_menu(recipes, pages)]));
    }
    for &(label, recipes) in stimulated {
        let pages = recipes
            .iter()
            .map(|recipe| {
                let stimulator = item_makers::stimulator(recipe.item).map(|item| {
                    let loses = rng.gen_range(0, item_makers::STIMULATOR_LOSS) == 0;
                    (item, loses.then_some(lost))
                });
                make(recipe, stimulator)
            })
            .collect();
        choices.push((text(label), vec![recipe_menu(recipes, pages)]));
    }
    choices
}

/// Sarah, Tara or Rydole: a maker with stimulators (`stimulator_choices`).
fn stimulating_maker(
    npc_id: i32,
    hello: &str,
    plain: &[(&str, &[Recipe])],
    stimulated: &[(&str, &[Recipe])],
    rng: &mut impl Rng,
) -> NpcScript {
    let menu = Page::Menu {
        text: text(hello),
        choices: stimulator_choices(
            keys::MAKER_STIMULATOR,
            keys::MAKER_STIMULATOR_LOST,
            plain,
            stimulated,
            rng,
        ),
    };
    with_maker_words(NpcScript::new(npc_id, vec![menu]))
}

/// Mos: a weapon of each class, or the same with a stimulator
/// (`stimulator_choices`). He also makes Cornian's Dagger for the way to
/// Moira.
fn mos(player: &Player, rng: &mut impl Rng) -> NpcScript {
    let plain = [
        (keys::MAKER_MOS_WARRIOR, item_makers::MOS_WARRIOR),
        (keys::MAKER_MOS_BOWMAN, item_makers::MOS_BOWMAN),
        (keys::MAKER_MOS_MAGICIAN, item_makers::MOS_MAGICIAN),
        (keys::MAKER_MOS_THIEF, item_makers::MOS_THIEF),
        (keys::MAKER_MOS_PIRATE, item_makers::MOS_PIRATE),
    ];
    let stimulated = [
        (keys::MAKER_MOS_WARRIOR_STIMULATED, item_makers::MOS_WARRIOR),
        (keys::MAKER_MOS_BOWMAN_STIMULATED, item_makers::MOS_BOWMAN),
        (
            keys::MAKER_MOS_MAGICIAN_STIMULATED,
            item_makers::MOS_MAGICIAN,
        ),
        (keys::MAKER_MOS_THIEF_STIMULATED, item_makers::MOS_THIEF),
        (keys::MAKER_MOS_PIRATE_STIMULATED, item_makers::MOS_PIRATE),
    ];
    let mut choices = stimulator_choices(
        keys::MAKER_MOS_STIMULATOR,
        keys::MAKER_MOS_LOST,
        &plain,
        &stimulated,
        rng,
    );
    if CORNIAN_QUESTS
        .iter()
        .any(|&quest| player.quests.state(quest) == QuestState::Started)
    {
        let dagger = item_makers::CORNIANS_DAGGER;
        let mut pages = vec![Page::Say(text(keys::MAKER_MOS_DAGGER))];
        pages.extend(make(&dagger, None));
        choices.push((format!("#t{}#", dagger.item), pages));
    }
    let menu = Page::Menu {
        text: text(keys::MAKER_MOS_HELLO),
        choices,
    };
    with_maker_words(NpcScript::new(MOS, vec![menu]))
}

/// The menu of one kind's items, each leading to its `pages`.
fn recipe_menu(recipes: &[Recipe], pages: Vec<Vec<Page>>) -> Page {
    Page::Menu {
        text: text(keys::MAKER_PICK),
        choices: recipes
            .iter()
            .map(|recipe| format!("#t{}#", recipe.item))
            .zip(pages)
            .collect(),
    }
}

/// Ask to make `recipe`, listing what it takes, then say it is done. With
/// a `stimulator` (its item, and what the maker says if the item is lost),
/// it takes the stimulator too.
fn make(recipe: &Recipe, stimulator: Option<(i32, Option<&'static str>)>) -> Vec<Page> {
    let materials: Vec<(i32, i16)> = recipe
        .materials
        .iter()
        .copied()
        .chain(stimulator.map(|(item, _)| (item, 1)))
        .collect();
    let item = recipe.item.to_string();
    let mut ask = content::text().get(keys::MAKER_ASK, &[("item", item.as_str())]);
    for (item, quantity) in &materials {
        ask.push_str(&format!("\r\n#i{item}# {quantity} #t{item}#"));
    }
    let meso = recipe.cost.to_string();
    let meso = content::text().get(keys::MAKER_MESO, &[("meso", meso.as_str())]);
    ask.push_str(&format!("\r\n#i{MESO_ICON}# {meso}"));
    let mut on_yes = vec![Effect::SpendMeso(recipe.cost)];
    on_yes.extend(
        materials
            .iter()
            .map(|&(item_id, quantity)| Effect::TakeItem { item_id, quantity }),
    );
    let done = match stimulator {
        Some((_, Some(lost))) => lost,
        Some((_, None)) => {
            on_yes.push(Effect::GiveVariedItem(recipe.item));
            keys::MAKER_DONE
        }
        None => {
            on_yes.push(Effect::GiveItem {
                item_id: recipe.item,
                quantity: 1,
            });
            keys::MAKER_DONE
        }
    };
    vec![
        Page::Ask {
            text: ask,
            on_yes,
            on_no: None,
        },
        Page::Say(text(done)),
    ]
}

/// What a maker says when the materials or mesos are short, or the item
/// doesn't fit.
fn with_maker_words(mut script: NpcScript) -> NpcScript {
    script.refused = Some(text(keys::MAKER_SHORT));
    script.inventory_full = Some(text(keys::MAKER_FULL));
    script
}

/// What a gate says: before boarding, after a no, without a ticket, once
/// boarding has closed, and between trips.
struct GateText {
    ask: &'static str,
    decline: &'static str,
    no_ticket: &'static str,
    closing: &'static str,
    full: &'static str,
}

/// A gate that asks first and only then looks for one of `tickets`,
/// taking the first held, then shows the player to the waiting room while
/// `route` boards, as of route time `t`.
fn gate(route: &Route, tickets: &[i32], player: &Player, t: i64, says: &GateText) -> Vec<Page> {
    match route.timetable.state(t) {
        State::Wait => {
            let held = tickets
                .iter()
                .find(|&&ticket| player.inventory.count(ticket) > 0);
            let has_ticket = held.is_some();
            let on_yes = if let Some(&ticket) = held {
                vec![
                    Effect::RequireBoarding(route.platform),
                    Effect::TakeItem {
                        item_id: ticket,
                        quantity: 1,
                    },
                    Effect::Warp(route.waiting_room),
                ]
            } else {
                Vec::new()
            };
            let mut pages = vec![Page::Ask {
                text: text(says.ask),
                on_yes,
                on_no: Some(text(says.decline)),
            }];
            if !has_ticket {
                pages.push(Page::Say(text(says.no_ticket)));
            }
            pages
        }
        State::Start => vec![Page::Say(text(says.closing))],
        _ => vec![Page::Say(text(says.full))],
    }
}

/// An airport's booth: one menu to buy the ticket for its plane, which
/// flies to `place`, or to go in and board it, as of route time `t`. A
/// player who already holds a ticket can't buy another (Cosmic).
fn plane_booth(npc_id: i32, route: &Route, place: &str, player: &Player, t: i64) -> NpcScript {
    let choice = |key| content::text().get(key, &[("place", place)]);
    let can_pay = player.meso >= route.price && player.inventory.count(route.ticket) == 0;
    let buy = sell(
        (route.ticket, route.price),
        can_pay,
        ticket_text(keys::PLANE_SELL_ASK, route),
        keys::PLANE_SELL_DECLINE,
        keys::PLANE_SELL_SHORT,
    );
    let says = GateText {
        ask: keys::PLANE_BOARD_ASK,
        decline: keys::PLANE_BOARD_DECLINE,
        no_ticket: keys::PLANE_BOARD_NO_TICKET,
        closing: keys::PLANE_BOARD_CLOSED,
        full: keys::PLANE_BOARD_CLOSED,
    };
    let mut script = NpcScript::new(
        npc_id,
        vec![Page::Menu {
            text: choice(keys::PLANE_BOOTH_MENU),
            choices: vec![
                (choice(keys::PLANE_BOOTH_BUY), buy),
                (
                    text(keys::PLANE_BOOTH_BOARD),
                    gate(route, &[route.ticket], player, t, &says),
                ),
            ],
        }],
    );
    script.inventory_full = Some(text(keys::PLANE_SELL_FULL));
    script
}

/// A pilot: in a departure room, lets the player back out to the airport
/// without the ticket; on a plane, says it lands soon.
fn pilot(npc_id: i32, player: &Player) -> Option<NpcScript> {
    let map_id = player.stats.map_id;
    if let Some(route) = conti::riding_on(map_id) {
        let (_, _, place) = PLANE_BOOTHS
            .iter()
            .find(|(_, plane, _)| plane.deck == route.deck)?;
        let says = content::text().get(keys::PLANE_RIDE, &[("place", place)]);
        return Some(NpcScript::new(npc_id, vec![Page::Say(says)]));
    }
    let route = conti::waiting_in(map_id)?;
    Some(NpcScript::new(
        npc_id,
        vec![Page::Ask {
            text: text(keys::PLANE_LEAVE_ASK),
            on_yes: vec![Effect::Warp(route.platform)],
            on_no: Some(text(keys::PLANE_LEAVE_DECLINE)),
        }],
    ))
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
        let fare = line.fare(player.stats.level);
        let (term, price, ticket) = (
            route.timetable.term.to_string(),
            fare.1.to_string(),
            fare.0.to_string(),
        );
        let vars = [
            ("term", term.as_str()),
            ("price", &price),
            ("ticket", &ticket),
        ];
        let ask = content::text().get(line.ask, &vars);
        let pages = sell(
            fare,
            player.meso >= fare.1,
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
        let price = fare.1.to_string();
        script.inventory_full =
            Some(content::text().get(keys::SUBWAY_SELL_SHORT, &[("price", &price)]));
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

/// Tory (Cosmic `1012112`): the party's leader may take the party up to
/// Primrose Hill, and anyone may hear how it is played.
fn tory(entry: Entry) -> NpcScript {
    let enter = match entry {
        Entry::NoParty => Page::Say(text(keys::HPQ_TORY_NO_PARTY)),
        Entry::NotLeader => Page::Say(text(keys::HPQ_TORY_NOT_LEADER)),
        Entry::NotEligible => Page::Say(hpq_terms(keys::HPQ_TORY_NOT_ELIGIBLE)),
        Entry::Busy => Page::Say(text(keys::HPQ_TORY_BUSY)),
        Entry::Ready => Page::Ask {
            text: text(keys::HPQ_TORY_ASK),
            on_yes: vec![Effect::StartEvent(EventKind::Hpq)],
            on_no: Some(text(keys::HPQ_TORY_DECLINE)),
        },
    };
    let about = keys::HPQ_TORY_ABOUT_PAGES
        .iter()
        .map(|&key| Page::Say(text(key)))
        .collect();
    let mut script = NpcScript::new(
        TORY,
        vec![Page::Menu {
            text: hpq_terms(keys::HPQ_TORY_MENU),
            choices: vec![
                (text(keys::HPQ_TORY_ENTER), vec![enter]),
                (text(keys::HPQ_TORY_ABOUT), about),
            ],
        }],
    );
    // Someone left or moved since she checked, or another party went up.
    script.refused = Some(hpq_terms(keys::HPQ_TORY_NOT_ELIGIBLE));
    script
}

/// The Red Sign (Cosmic `2040034`): the party's leader takes the party in
/// at once, and anyone may read what the quest is.
fn red_sign(entry: Entry) -> NpcScript {
    let enter = match entry {
        Entry::NoParty => Page::Say(text(keys::LPQ_SIGN_NO_PARTY)),
        Entry::NotLeader => Page::Say(text(keys::LPQ_SIGN_NOT_LEADER)),
        Entry::NotEligible => Page::Say(event_terms(&events::LPQ, keys::LPQ_SIGN_NOT_ELIGIBLE)),
        Entry::Busy => Page::Say(text(keys::LPQ_SIGN_BUSY)),
        Entry::Ready => Page::Done(vec![Effect::StartEvent(EventKind::Lpq)]),
    };
    let mut script = NpcScript::new(
        RED_SIGN,
        vec![Page::Menu {
            text: event_terms(&events::LPQ, keys::LPQ_SIGN_MENU),
            choices: vec![
                (text(keys::LPQ_SIGN_ENTER), vec![enter]),
                (
                    text(keys::LPQ_SIGN_ABOUT),
                    vec![Page::Say(text(keys::LPQ_SIGN_INFO))],
                ),
            ],
        }],
    );
    // Someone left or moved since it checked, or another party went in.
    script.refused = Some(event_terms(&events::LPQ, keys::LPQ_SIGN_NOT_ELIGIBLE));
    script
}

/// Wonky (Cosmic `2013000`) in Orbis: the party's leader takes the party
/// into the Tower of Goddess at once, and anyone may read what the quest
/// is. Cosmic's party search and prize lines are not offered.
fn wonky(entry: Entry) -> NpcScript {
    let enter = match entry {
        Entry::NoParty => Page::Say(text(keys::OPQ_WONKY_NO_PARTY)),
        Entry::NotLeader => Page::Say(text(keys::OPQ_WONKY_NOT_LEADER)),
        Entry::NotEligible => Page::Say(event_terms(&events::OPQ, keys::OPQ_WONKY_NOT_ELIGIBLE)),
        Entry::Busy => Page::Say(text(keys::OPQ_WONKY_BUSY)),
        Entry::Ready => Page::Done(vec![Effect::StartEvent(EventKind::Opq)]),
    };
    let mut script = NpcScript::new(
        WONKY,
        vec![Page::Menu {
            text: event_terms(&events::OPQ, keys::OPQ_WONKY_MENU),
            choices: vec![
                (text(keys::OPQ_WONKY_ENTER), vec![enter]),
                (
                    text(keys::OPQ_WONKY_ABOUT),
                    vec![Page::Say(text(keys::OPQ_WONKY_INFO))],
                ),
            ],
        }],
    );
    // Someone left or moved since he checked, or another party went in.
    script.refused = Some(event_terms(&events::OPQ, keys::OPQ_WONKY_NOT_ELIGIBLE));
    script
}

/// `key`, with who may play `event`.
fn event_terms(event: &events::Event, key: &str) -> String {
    let (team, levels) = (&event.team, &event.levels);
    content::text().get(
        key,
        &[
            ("least", &event.least().to_string()),
            ("most", &team.end().to_string()),
            ("low", &levels.start().to_string()),
            ("high", &levels.end().to_string()),
        ],
    )
}

/// `key`, with who may play Henesys's party quest.
fn hpq_terms(key: &str) -> String {
    let (team, levels) = (&events::HPQ.team, &events::HPQ.levels);
    content::text().get(
        key,
        &[
            ("least", &team.start().to_string()),
            ("most", &team.end().to_string()),
            ("low", &levels.start().to_string()),
        ],
    )
}

/// The rice cakes Growlie wants for the party quest's one stage.
const CAKES_WANTED: i16 = 10;

/// Growlie (Cosmic `1012114`) on Primrose Hill: what the place is, the way
/// out, and for the party's leader until the party has `won`, the hand-in
/// of 10 rice cakes, which wins the party quest.
fn growlie(player: &Player, leader: bool, won: bool) -> Option<NpcScript> {
    if player.stats.map_id != events::HPQ.entry {
        return None;
    }
    let about = keys::HPQ_GROWLIE_ABOUT_PAGES
        .iter()
        .map(|&key| Page::Say(text(key)))
        .collect();
    let leave = vec![Page::Ask {
        text: text(keys::HPQ_GROWLIE_LEAVE_ASK),
        on_yes: vec![Effect::Warp(events::HPQ.exit)],
        on_no: Some(text(keys::HPQ_GROWLIE_STAY)),
    }];
    let mut choices = vec![(text(keys::HPQ_GROWLIE_ABOUT), about)];
    let n = CAKES_WANTED.to_string();
    let short = content::text().get(keys::HPQ_GROWLIE_CAKES_SHORT, &[("n", &n)]);
    if leader && !won {
        let held = player.inventory.count(events::RICE_CAKE);
        let cakes = if held >= i32::from(CAKES_WANTED) {
            Page::Act {
                text: text(keys::HPQ_GROWLIE_CAKES_OK),
                effects: vec![
                    Effect::TakeItem {
                        item_id: events::RICE_CAKE,
                        quantity: CAKES_WANTED,
                    },
                    Effect::EventCommand(EventCommand::ClearStage(1)),
                ],
            }
        } else {
            Page::Say(short.clone())
        };
        choices.push((text(keys::HPQ_GROWLIE_CAKES), vec![cakes]));
    }
    choices.push((text(keys::HPQ_GROWLIE_LEAVE), leave));
    let mut script = NpcScript::new(
        GROWLIE,
        vec![Page::Menu {
            text: text(keys::HPQ_GROWLIE_MENU),
            choices,
        }],
    );
    // The cakes dropped since the menu opened.
    script.refused = Some(short);
    Some(script)
}

/// Tommy (Cosmic `1012113`): where the party that won Primrose Hill comes
/// out, he tells of Pig Town and its leader may take the party there; in
/// Pig Town he lets players out to its exit; at Primrose Hill's way out he
/// sends players back to Henesys Park.
fn tommy(player: &Player, leader: bool, won: bool) -> Option<NpcScript> {
    let map_id = player.stats.map_id;
    let bonus = events::HPQ.bonus.as_ref()?;
    let pages = if map_id == events::HPQ.exit {
        vec![Page::Ask {
            text: text(keys::HPQ_TOMMY_HOME),
            on_yes: vec![Effect::Warp(events::HPQ.lobby)],
            on_no: None,
        }]
    } else if Some(map_id) == events::HPQ.clear_map {
        let go = if leader && won {
            Page::Ask {
                text: text(keys::HPQ_TOMMY_PIGS_ASK),
                on_yes: vec![Effect::EventCommand(EventCommand::StartBonus)],
                on_no: None,
            }
        } else {
            Page::Say(text(keys::HPQ_TOMMY_PIGS_LEADER))
        };
        vec![Page::Say(text(keys::HPQ_TOMMY_PIGS)), go]
    } else if map_id == bonus.map {
        vec![Page::Ask {
            text: text(keys::HPQ_TOMMY_PIGS_LEAVE),
            on_yes: vec![Effect::Warp(bonus.exit)],
            on_no: None,
        }]
    } else {
        return None;
    };
    Some(NpcScript::new(TOMMY, pages))
}

/// Tory (Cosmic `1012112`) where the party that won Primrose Hill comes out,
/// and at Pig Town's exit: she gives each member who comes to her the
/// reward and sends them home, so nobody gets it twice. Without the win
/// she only sends them home.
fn tory_after(player: &Player, won: bool) -> Option<NpcScript> {
    let map_id = player.stats.map_id;
    let bonus = events::HPQ.bonus.as_ref()?;
    if Some(map_id) != events::HPQ.clear_map && map_id != bonus.exit {
        return None;
    }
    let page = if won {
        Page::Ask {
            text: text(keys::HPQ_TORY_REWARD_ASK),
            on_yes: vec![
                Effect::GiveItem {
                    item_id: events::HPQ_REWARD,
                    quantity: 1,
                },
                Effect::Warp(events::HPQ.lobby),
            ],
            on_no: None,
        }
    } else {
        Page::Ask {
            text: text(keys::HPQ_TORY_HOME_ASK),
            on_yes: vec![Effect::Warp(events::HPQ.lobby)],
            on_no: None,
        }
    };
    let mut script = NpcScript::new(TORY, vec![page]);
    script.inventory_full = Some(text(keys::HPQ_TORY_REWARD_FULL));
    Some(script)
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
        Cloto::Won => return cloto_reward(player, &mut rand::thread_rng()),
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

/// Cloto on the last stage once it is won (Cosmic `9020001`,
/// `giveEventReward`) gives each member who comes to her one of
/// `KPQ_REWARDS` and sends them on to the bonus stage, so nobody gets it
/// twice. A member without room for any of the rewards gets nothing yet.
fn cloto_reward(player: &Player, rng: &mut impl Rng) -> NpcScript {
    let reward = events::KPQ_REWARDS.choose(rng);
    let last = match reward {
        Some(&(item_id, quantity)) if has_reward_slot(&player.inventory) => {
            let portal = crate::scripts::named(events::KPQ_BONUS, "st00");
            Page::Done(vec![
                Effect::GiveItem { item_id, quantity },
                Effect::WarpToPortal(events::KPQ_BONUS, portal),
            ])
        }
        _ => Page::Say(text(keys::KPQ_CLOTO_S5_FULL)),
    };
    let pages = vec![Page::Say(text(keys::KPQ_CLOTO_S5_WON)), last];
    let mut script = NpcScript::new(CLOTO, pages);
    script.inventory_full = Some(text(keys::KPQ_CLOTO_S5_FULL));
    script
}

/// Whether `inventory` has a free slot in each of the Equip, Use and Etc
/// tabs. Cosmic's `hasRewardSlot` checks the tabs its reward list comes in,
/// which for Kerning City's and Ludibrium's lists are these three.
fn has_reward_slot(inventory: &Inventory) -> bool {
    [InventoryType::Equip, InventoryType::Use, InventoryType::Etc]
        .into_iter()
        .all(|kind| inventory.items(kind).count() < usize::from(inventory.slot_limit(kind)))
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
/// On the bonus stage she asks only whether the player is ready to go.
fn nella(player: &Player) -> Option<NpcScript> {
    let map_id = player.stats.map_id;
    let (key, to) = if map_id == events::KPQ.exit {
        (keys::KPQ_NELLA_HOME, events::KPQ.lobby)
    } else if map_id == events::KPQ_BONUS {
        (keys::KPQ_NELLA_BONUS, events::KPQ.exit)
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

/// A Ludibrium balloon (Cosmic `2040036`-`2040044`): the leader first
/// hears what its stage asks, then hands in what it takes, which clears the
/// stage; the others are sent to fetch them.
fn balloon(hand_in: &HandIn, view: Balloon, player: &Player) -> NpcScript {
    if hand_in.npc == EAK {
        return eak_hand_in(hand_in, view, player);
    }
    let item = hand_in.item.to_string();
    let n = hand_in.count.to_string();
    let get = |key| content::text().get(key, &[("item", &item), ("n", &n)]);
    let short = get(keys::LPQ_BALLOON_SHORT);
    let page = match view {
        Balloon::Cleared => Page::Say(text(keys::LPQ_BALLOON_HURRY)),
        Balloon::NotLeader => Page::Say(text(keys::LPQ_BALLOON_NOT_LEADER)),
        Balloon::Intro => Page::Say(get(lpq_balloon_intro(hand_in.stage))),
        Balloon::Ready if player.inventory.count(hand_in.item) >= i32::from(hand_in.count) => {
            Page::Act {
                text: text(keys::LPQ_BALLOON_DONE),
                effects: vec![
                    Effect::TakeItem {
                        item_id: hand_in.item,
                        quantity: hand_in.count,
                    },
                    Effect::EventCommand(EventCommand::ClearStage(hand_in.stage)),
                ],
            }
        }
        Balloon::Ready | Balloon::Answer { .. } => Page::Say(short.clone()),
    };
    let mut script = NpcScript::new(hand_in.npc, vec![page]);
    // The items dropped since the page opened.
    script.refused = Some(short);
    script
}

/// Eak in the walkway and the lounge (Cosmic `2013001`): he takes what the
/// leader gathered for a piece of the statue, which clears the room.
/// Cosmic lets the walkway's be handed in twice; here a cleared room only
/// sends the team on.
fn eak_hand_in(hand_in: &HandIn, view: Balloon, player: &Player) -> NpcScript {
    let item = hand_in.item.to_string();
    let n = hand_in.count.to_string();
    let piece = hand_in.prize.unwrap_or_default().to_string();
    let vars = [
        ("item", item.as_str()),
        ("n", n.as_str()),
        ("piece", piece.as_str()),
    ];
    let get = |key| content::text().get(key, &vars);
    let short = get(keys::OPQ_EAK_HAND_IN);
    let page = match view {
        Balloon::Cleared => Page::Say(text(keys::OPQ_EAK_ROOM_CLEARED)),
        Balloon::NotLeader => Page::Say(text(keys::OPQ_EAK_LEADER_ONLY)),
        Balloon::Ready if player.inventory.count(hand_in.item) >= i32::from(hand_in.count) => {
            let mut effects = vec![Effect::TakeItem {
                item_id: hand_in.item,
                quantity: hand_in.count,
            }];
            effects.extend(hand_in.prize.map(|item_id| Effect::GiveItem {
                item_id,
                quantity: 1,
            }));
            effects.push(Effect::EventCommand(EventCommand::ClearStage(
                hand_in.stage,
            )));
            Page::Act {
                text: get(keys::OPQ_EAK_PIECE),
                effects,
            }
        }
        Balloon::Intro | Balloon::Ready | Balloon::Answer { .. } => Page::Say(short.clone()),
    };
    let mut script = NpcScript::new(EAK, vec![page]);
    script.refused = Some(short);
    script.inventory_full = Some(text(keys::OPQ_EAK_FULL));
    script
}

/// Ellin at the maze's end (Cosmic `2133001`): whoever finds her clears
/// the maze, and she takes the team on to the fourth stage.
fn ellin_maze_end() -> NpcScript {
    NpcScript::new(
        ELLIN_GUIDE,
        vec![
            Page::Act {
                text: text(keys::ELLIN_GUIDE_MAZE_END),
                effects: vec![Effect::EventCommand(EventCommand::ClearStage(3))],
            },
            Page::Done(vec![Effect::EventCommand(EventCommand::MoveTeam(
                events::ELLIN_STAGES[3],
            ))]),
        ],
    )
}

/// The Sprite on the forest's fifth stage (Cosmic `2133004`): the leader
/// showing it the Purple Stone of Magic, which they keep for the altar, is
/// taken on with the whole team to the last stage; anyone else may leave.
fn ellin_sprite(player: &Player, leader: bool) -> NpcScript {
    if !leader || player.inventory.count(events::PURPLE_STONE) == 0 {
        return ask_warp(ELLIN_SPRITE, keys::ELLIN_SPRITE_LEAVE, events::ELLIN.exit);
    }
    NpcScript::new(
        ELLIN_SPRITE,
        vec![
            Page::Say(text(keys::ELLIN_SPRITE_TO_ALTAR)),
            Page::Done(vec![Effect::EventCommand(EventCommand::MoveTeam(
                events::ELLIN_STAGES[5],
            ))]),
        ],
    )
}

/// Ellin on the forest's fourth stage: the leader with the Monster Marbles
/// hands them over, which takes the team on; anyone else holding them is
/// sent to the leader. Otherwise she first tells a member what to do (if
/// they haven't `heard` her), then, while Poison Flowers are left, gives
/// one with no Purification Marbles ten, and asks one with some whether to
/// leave, as she does once all are caught.
fn ellin_flowers(leads: bool, heard: bool, flowers: usize, player: &Player) -> NpcScript {
    use events::{
        MONSTER_MARBLE, MONSTER_MARBLES_WANTED, PURIFICATION_MARBLE, PURIFICATION_MARBLES_GIVEN,
    };
    let counted = |key, n: i16| content::text().get(key, &[("n", &n.to_string())]);
    let leave = |key| Page::Ask {
        text: counted(key, MONSTER_MARBLES_WANTED),
        on_yes: vec![Effect::WarpToPortal(events::ELLIN.exit, 0)],
        on_no: None,
    };
    if player.inventory.count(MONSTER_MARBLE) >= i32::from(MONSTER_MARBLES_WANTED) {
        if !leads {
            return NpcScript::new(
                ELLIN_GUIDE,
                vec![Page::Say(text(keys::ELLIN_GUIDE_TO_LEADER))],
            );
        }
        let hand_in = vec![
            Effect::TakeItem {
                item_id: MONSTER_MARBLE,
                quantity: MONSTER_MARBLES_WANTED,
            },
            Effect::EventCommand(EventCommand::MoveTeam(events::ELLIN_STAGES[4])),
        ];
        let pages = vec![
            Page::Say(text(keys::ELLIN_GUIDE_HAND_IN)),
            Page::Done(hand_in),
        ];
        let mut script = NpcScript::new(ELLIN_GUIDE, pages);
        // The marbles dropped since the page opened.
        script.refused = Some(counted(keys::ELLIN_GUIDE_FLOWERS, MONSTER_MARBLES_WANTED));
        return script;
    }
    let mut pages = Vec::new();
    if !heard {
        pages.push(Page::Say(counted(
            keys::ELLIN_GUIDE_FLOWERS,
            MONSTER_MARBLES_WANTED,
        )));
    }
    pages.push(if flowers == 0 {
        leave(keys::ELLIN_GUIDE_ALL_CAUGHT)
    } else if player.inventory.count(PURIFICATION_MARBLE) > 0 {
        leave(keys::ELLIN_GUIDE_FLOWERS_LEAVE)
    } else {
        Page::Act {
            text: counted(keys::ELLIN_GUIDE_MARBLES, PURIFICATION_MARBLES_GIVEN),
            effects: vec![Effect::GiveItem {
                item_id: PURIFICATION_MARBLE,
                quantity: PURIFICATION_MARBLES_GIVEN,
            }],
        }
    });
    let mut script = NpcScript::new(ELLIN_GUIDE, pages);
    script.inventory_full = Some(text(keys::ELLIN_GUIDE_NO_ROOM));
    script
}

/// What Ludibrium's balloon on `stage` asks of the party.
fn lpq_balloon_intro(stage: u8) -> &'static str {
    match stage {
        1 => keys::LPQ_BALLOON_INTRO_1,
        2 => keys::LPQ_BALLOON_INTRO_2,
        3 => keys::LPQ_BALLOON_INTRO_3,
        4 => keys::LPQ_BALLOON_INTRO_4,
        5 => keys::LPQ_BALLOON_INTRO_5,
        7 => keys::LPQ_BALLOON_INTRO_7,
        _ => keys::LPQ_BALLOON_INTRO_9,
    }
}

/// Sgt. Anderson (Cosmic `2040047`): on the party quest's maps he lets a
/// player out, who can't come back; at the way out he sends them on down
/// to Eos Tower.
fn anderson(player: &Player) -> Option<NpcScript> {
    let map_id = player.stats.map_id;
    let pages = if map_id == events::LPQ.exit {
        vec![
            Page::Say(text(keys::LPQ_ANDERSON_HOME)),
            Page::Done(vec![Effect::Warp(events::LPQ.lobby)]),
        ]
    } else if events::LPQ.holds(map_id) {
        vec![Page::Ask {
            text: text(keys::LPQ_ANDERSON_LEAVE_ASK),
            on_yes: vec![Effect::Warp(events::LPQ.exit)],
            on_no: None,
        }]
    } else {
        return None;
    };
    Some(NpcScript::new(ANDERSON, pages))
}

/// Wonky (Cosmic `2013000`) on the Orbis party quest's cloud stage lets a
/// player out, who can't come back.
fn wonky_inside(player: &Player) -> Option<NpcScript> {
    if player.stats.map_id != events::OPQ.entry {
        return None;
    }
    let page = Page::Ask {
        text: text(keys::OPQ_WONKY_LEAVE_ASK),
        on_yes: vec![Effect::Warp(events::OPQ.exit)],
        on_no: None,
    };
    Some(NpcScript::new(WONKY, vec![page]))
}

/// Eak (Cosmic `2013001`) at the Orbis party quest's way out sends players
/// back to Orbis. His lines on its stages come with them.
fn eak(player: &Player) -> Option<NpcScript> {
    if player.stats.map_id != events::OPQ.exit {
        return None;
    }
    let pages = vec![
        Page::Say(text(keys::OPQ_EAK_HOME)),
        Page::Done(vec![Effect::Warp(events::OPQ.lobby)]),
    ];
    Some(NpcScript::new(EAK, pages))
}

/// Eak in the Tower of Goddess. On the cloud stage he sends anyone but the
/// leader up to its top (`st00`), and the leader too once it is cleared;
/// before then the leader hears of Minerva and clears it, which takes the
/// whole team there (`OPQ.clear_portals`). Cosmic clears first and tells
/// after, but the warp would close his dialog here, so he tells first. On
/// the hub only the leader hears him.
fn opq_eak(map_id: i32, leader: bool, cleared: bool) -> Option<NpcScript> {
    let cloud = events::OPQ.entry;
    let pages = match map_id {
        _ if map_id == cloud && (!leader || cleared) => {
            let portal = crate::scripts::named(cloud, "st00");
            vec![Page::Done(vec![Effect::WarpToPortal(cloud, portal)])]
        }
        _ if map_id == cloud => vec![
            Page::Say(text(keys::OPQ_EAK_CLOUD)),
            Page::Done(vec![Effect::EventCommand(EventCommand::ClearStage(1))]),
        ],
        events::OPQ_HUB | events::OPQ_STORAGE if !leader => {
            vec![Page::Say(text(keys::OPQ_EAK_LEADER_ONLY))]
        }
        events::OPQ_HUB => vec![Page::Say(text(keys::OPQ_EAK_GATHER))],
        events::OPQ_GARDEN => vec![Page::Say(text(keys::OPQ_EAK_GARDEN))],
        // With every guard gone he takes the leader alone on to the
        // treasure room, without a word; until then he says what to do.
        events::OPQ_STORAGE => {
            let to = events::OPQ_BONUS;
            let portal = crate::scripts::named(to, "st00");
            let mut script = NpcScript::new(
                EAK,
                vec![Page::Done(vec![
                    Effect::RequireNoMobs,
                    Effect::WarpToPortal(to, portal),
                ])],
            );
            script.refused = Some(text(keys::OPQ_EAK_STORAGE));
            return Some(script);
        }
        _ => return None,
    };
    Some(NpcScript::new(EAK, pages))
}

/// Eak to the leader on the hub, as Minerva's statue stands (Cosmic
/// `2013001`, 920010100): with all six pieces in it he takes the team to
/// the garden, without a word, until it is cleared.
fn opq_hub(statue: Statue) -> NpcScript {
    let page = match statue {
        Statue::Gathering => Page::Say(text(keys::OPQ_EAK_GATHER)),
        Statue::Garden => Page::Done(vec![Effect::EventCommand(EventCommand::MoveTeam(
            events::OPQ_GARDEN,
        ))]),
        Statue::GrassOfLife => Page::Say(text(keys::OPQ_EAK_GRASS)),
        Statue::Won => Page::Say(text(keys::OPQ_EAK_MINERVA)),
    };
    NpcScript::new(EAK, vec![page])
}

/// Eak in the storage room (Cosmic `2013001`, `statusStg2`): lest the
/// Salion stage stick, once no monster or drop is left in the room he
/// gives the statue's second piece himself and clears the room, as its
/// last spot would. Any member may ask.
fn opq_salion(bare: bool, cleared: bool) -> NpcScript {
    let page = if cleared {
        Page::Say(text(keys::OPQ_EAK_ROOM_CLEARED))
    } else if bare {
        Page::Act {
            text: text(keys::OPQ_EAK_SALION_FOUND),
            effects: vec![
                Effect::GiveItem {
                    item_id: events::OPQ_SALION_PIECE,
                    quantity: 1,
                },
                Effect::EventCommand(EventCommand::ClearStage(events::OPQ_SALION_STAGE)),
            ],
        }
    } else {
        Page::Say(text(keys::OPQ_EAK_SALION_FIND))
    };
    let mut script = NpcScript::new(EAK, vec![page]);
    script.inventory_full = Some(text(keys::OPQ_EAK_FULL));
    script
}

/// Eak in the music room (Cosmic `2013001`, `statusStg3`): he asks for the
/// day's record until the music box plays it, then brings out the stone
/// box (`stone3`, its fade-in) and clears the room. Any member may ask.
fn opq_music(played: bool, cleared: bool) -> NpcScript {
    let page = if cleared {
        Page::Say(text(keys::OPQ_EAK_MUSIC_DONE))
    } else if played {
        stone_box_out(
            keys::OPQ_EAK_MUSIC_PLAYED,
            "stone3",
            events::OPQ_MUSIC_STAGE,
        )
    } else {
        Page::Say(text(keys::OPQ_EAK_MUSIC_ASK))
    };
    NpcScript::new(EAK, vec![page])
}

/// Eak in the sealed room (Cosmic `2013001`, `statusStg4`): with three of
/// the team on its platforms as he wants (`events::head_count`), he brings
/// out the stone box at its top (`stone4`) and clears the room; otherwise
/// he hints how many platforms are right. Any member may ask.
fn opq_head_count(count: Option<HeadCount>) -> NpcScript {
    let page = match count {
        None => Page::Say(text(keys::OPQ_EAK_HEADS_DONE)),
        Some(HeadCount::Few) => Page::Say(text(keys::OPQ_EAK_HEADS_FEW)),
        Some(HeadCount::Wrong { right: 0 }) => Page::Say(text(keys::OPQ_EAK_HEADS_ALL_WRONG)),
        Some(HeadCount::Wrong { .. }) => Page::Say(text(keys::OPQ_EAK_HEADS_ONE_RIGHT)),
        Some(HeadCount::Right) => stone_box_out(
            keys::OPQ_EAK_HEADS_RIGHT,
            "stone4",
            events::OPQ_HEAD_COUNT_STAGE,
        ),
    };
    NpcScript::new(EAK, vec![page])
}

/// Eak in the tall room (Cosmic `2013001`, `statusStg6`): with the two
/// levers up that he wants (`events::lever_check`), he brings out the
/// stone box beside him (`stone6`) and clears the room; with two others,
/// he says whether one of them is right. Any member may ask.
fn opq_levers(check: Option<LeverCheck>) -> NpcScript {
    let page = match check {
        None => Page::Say(text(keys::OPQ_EAK_LEVERS_DONE)),
        Some(LeverCheck::Few) => Page::Say(text(keys::OPQ_EAK_LEVERS_ASK)),
        Some(LeverCheck::Wrong { one_right: true }) => {
            Page::Say(text(keys::OPQ_EAK_LEVERS_ONE_RIGHT))
        }
        Some(LeverCheck::Wrong { one_right: false }) => {
            Page::Say(text(keys::OPQ_EAK_LEVERS_NONE_RIGHT))
        }
        Some(LeverCheck::Right) => stone_box_out(
            keys::OPQ_EAK_LEVERS_RIGHT,
            "stone6",
            events::OPQ_LEVER_STAGE,
        ),
    };
    NpcScript::new(EAK, vec![page])
}

/// Eak's line `key` as he brings out the stone box `name` (Cosmic
/// `forceHitReactor`) and clears `stage`.
fn stone_box_out(key: &str, name: &'static str, stage: u8) -> Page {
    Page::Act {
        text: text(key),
        effects: vec![
            Effect::SetReactorState { name, state: 1 },
            Effect::EventCommand(EventCommand::ClearStage(stage)),
        ],
    }
}

/// The Pink Balloon (Cosmic `2040045`) lets players off Ludibrium's bonus
/// stage early, to the reward.
fn pink_balloon(player: &Player) -> Option<NpcScript> {
    let bonus = events::LPQ.bonus.as_ref()?;
    if player.stats.map_id != bonus.map {
        return None;
    }
    let portal = crate::scripts::named(bonus.exit, "st00");
    let page = Page::Ask {
        text: text(keys::LPQ_PINK_LEAVE_ASK),
        on_yes: vec![Effect::WarpToPortal(bonus.exit, portal)],
        on_no: Some(text(keys::LPQ_PINK_STAY)),
    };
    Some(NpcScript::new(PINK_BALLOON, vec![page]))
}

/// Arturo (Cosmic `2040035`, `giveEventReward`) gives each member of the
/// team that won Ludibrium's party quest one of `LPQ_REWARDS` and sends
/// them back to Eos Tower, so nobody gets it twice. A member without room
/// for any of the rewards (`has_reward_slot`) gets nothing yet.
fn arturo(player: &Player, rng: &mut impl Rng) -> Option<NpcScript> {
    let bonus = events::LPQ.bonus.as_ref()?;
    if player.stats.map_id != bonus.exit {
        return None;
    }
    let last = if has_reward_slot(&player.inventory) {
        let &(item_id, quantity) = events::LPQ_REWARDS.choose(rng)?;
        Page::Done(vec![
            Effect::GiveItem { item_id, quantity },
            Effect::Warp(events::LPQ.lobby),
        ])
    } else {
        Page::Say(text(keys::LPQ_ARTURO_FULL))
    };
    let pages = vec![Page::Say(text(keys::LPQ_ARTURO_THANKS)), last];
    let mut script = NpcScript::new(ARTURO, pages);
    script.inventory_full = Some(text(keys::LPQ_ARTURO_FULL));
    Some(script)
}

/// Minerva (Cosmic `2013002`), once her statue is whole: on the hub she
/// asks each member of the team whether to go, and sends those who will
/// to her prize room alone; there she gives each one of `OPQ_REWARDS` and
/// sends them back to Orbis, so nobody gets it twice. A member without
/// room for any of the prizes (`has_reward_slot`) gets nothing yet.
fn minerva(player: &Player, rng: &mut impl Rng) -> Option<NpcScript> {
    let pages = match player.stats.map_id {
        events::OPQ_HUB => vec![Page::Ask {
            text: text(keys::OPQ_MINERVA_ASK),
            on_yes: vec![Effect::WarpToPortal(events::OPQ_PRIZE_ROOM, 0)],
            on_no: None,
        }],
        // The leader, in the treasure room she came out in.
        events::OPQ_BONUS => vec![Page::Ask {
            text: text(keys::OPQ_MINERVA_BONUS_ASK),
            on_yes: vec![Effect::WarpToPortal(events::OPQ_PRIZE_ROOM, 0)],
            on_no: None,
        }],
        events::OPQ_PRIZE_ROOM => {
            let last = if has_reward_slot(&player.inventory) {
                let &(item_id, quantity) = events::OPQ_REWARDS.choose(rng)?;
                Page::Done(vec![
                    Effect::GiveItem { item_id, quantity },
                    Effect::WarpToPortal(events::OPQ.lobby, 0),
                ])
            } else {
                Page::Say(text(keys::OPQ_MINERVA_FULL))
            };
            vec![Page::Say(text(keys::OPQ_MINERVA_THANKS)), last]
        }
        _ => return None,
    };
    let mut script = NpcScript::new(MINERVA, pages);
    script.inventory_full = Some(text(keys::OPQ_MINERVA_FULL));
    Some(script)
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

/// Corba (Cosmic's `2082003`): one choice, for anyone, that makes the
/// player a dragon and sends them flying (report a014).
fn corba() -> NpcScript {
    NpcScript::new(
        CORBA,
        vec![Page::Menu {
            text: text(keys::CORBA_MENU),
            choices: vec![(
                text(keys::CORBA_CHOICE),
                vec![Page::Done(vec![
                    Effect::ApplyItemBuff(MINI_DRACO),
                    Effect::WarpToPortal(FLIGHT_TO_TEMPLE, 0),
                ])],
            )],
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
    let on_yes = vec![Effect::SpendMeso(fare), warp];
    gated(ask, decline, short, player.meso >= fare, on_yes)
}

/// A question asked with `ask`: yes applies `on_yes` if `allowed`, and
/// otherwise says `refusal` after yes instead; no says `decline` (if any).
fn gated(
    ask: String,
    decline: Option<&str>,
    refusal: &str,
    allowed: bool,
    on_yes: Vec<Effect>,
) -> Vec<Page> {
    let mut pages = vec![Page::Ask {
        text: ask,
        on_yes: if allowed { on_yes } else { Vec::new() },
        on_no: decline.map(text),
    }];
    if !allowed {
        pages.push(Page::Say(text(refusal)));
    }
    pages
}

/// `world_trip`: Spinel's tour from where the player stands, or at the
/// Mushroom Shrine the way back.
fn spinel(player: &Player) -> NpcScript {
    let here = player.stats.map_id;
    if here == MUSHROOM_SHRINE {
        return spinel_back(player);
    }
    let (to, fare) = if here == BOAT_QUAY_TOWN {
        (TREND_ZONE_METROPOLIS, MALAYSIA_FARE)
    } else {
        (MUSHROOM_SHRINE, WORLD_TOUR_FARE)
    };
    let (map, price) = (to.to_string(), fare.to_string());
    let map = [("map", map.as_str())];
    let mut trip = vec![Page::Say(
        content::text().get(keys::WORLD_TRIP_DESCRIBE, &map),
    )];
    if player.meso >= fare {
        trip.push(Page::Say(text(keys::WORLD_TRIP_MORE)));
        trip.push(Page::Done(vec![
            Effect::SpendMeso(fare),
            Effect::SaveLocation(WORLD_TOUR_RETURN, Some(here)),
            Effect::WarpToPortal(to, 0),
        ]));
    } else {
        trip.push(Page::End(text(keys::WORLD_TRIP_SHORT)));
    }
    let pages = vec![
        Page::Say(content::text().get(keys::WORLD_TRIP_INTRO, &[("price", &price)])),
        Page::Menu {
            text: content::text().get(keys::WORLD_TRIP_OFFER, &map),
            choices: vec![(content::text().get(keys::WORLD_TRIP_CHOICE, &map), trip)],
        },
    ];
    let mut script = NpcScript::new(SPINEL, pages);
    script.refused = Some(text(keys::WORLD_TRIP_SHORT));
    script
}

/// Spinel at the Mushroom Shrine: back to where the tour began, else to
/// Lith Harbor, or stay.
fn spinel_back(player: &Player) -> NpcScript {
    let back = saved_location(player, WORLD_TOUR_RETURN, LITH_HARBOR);
    let map = back.to_string();
    let map = [("map", map.as_str())];
    let home = vec![
        Page::Say(text(keys::WORLD_TRIP_BACK_GO)),
        Page::Done(vec![
            Effect::SaveLocation(WORLD_TOUR_RETURN, None),
            Effect::Warp(back),
        ]),
    ];
    let stay = vec![Page::Say(text(keys::WORLD_TRIP_BACK_STAY))];
    NpcScript::new(
        SPINEL,
        vec![Page::Menu {
            text: content::text().get(keys::WORLD_TRIP_BACK_MENU, &map),
            choices: vec![
                (content::text().get(keys::WORLD_TRIP_BACK_YES, &map), home),
                (text(keys::WORLD_TRIP_BACK_NO), stay),
            ],
        }],
    )
}

/// The map saved under `kind`, else `fallback`.
fn saved_location(player: &Player, kind: i16, fallback: i32) -> i32 {
    player
        .saved_locations
        .get(&kind)
        .copied()
        .unwrap_or(fallback)
}

/// Out of a GM's event, back to where an Event Assistant took the player
/// from (`EVENT_RETURN`, else Henesys): Pietro and Pietra after the game,
/// and Harry during it, who gives the player's place back. Pietro gives
/// the winner a Scroll of Secrets on the way, and keeps them until it fits.
fn gm_event_exit(npc_id: i32, player: &Player) -> Option<NpcScript> {
    let back = saved_location(player, EVENT_RETURN, HENESYS.0);
    let go = vec![Effect::SaveLocation(EVENT_RETURN, None), Effect::Warp(back)];
    let page = match npc_id {
        PIETRO => {
            let mut prize = vec![Effect::GiveItem {
                item_id: SCROLL_OF_SECRETS,
                quantity: 1,
            }];
            prize.extend(go);
            let pages = vec![Page::Say(text(keys::GM_EVENT_PIETRO)), Page::Done(prize)];
            let mut script = NpcScript::new(PIETRO, pages);
            script.inventory_full = Some(text(keys::GM_EVENT_PIETRO_FULL));
            return Some(script);
        }
        PIETRA => Page::Say(text(keys::GM_EVENT_PIETRA)),
        HARRY => {
            let mut leave = vec![Effect::LeaveGmEvent {
                map_id: player.stats.map_id,
            }];
            leave.extend(go);
            let ask = Page::Ask {
                text: text(keys::GM_EVENT_HARRY_ASK),
                on_yes: leave,
                on_no: None,
            };
            return Some(NpcScript::new(HARRY, vec![ask]));
        }
        _ => return None,
    };
    Some(NpcScript::new(npc_id, vec![page, Page::Done(go)]))
}

/// `florina2`: to Florina Beach for `fare` or the VIP ticket, saving where
/// the trip began.
fn florina_guide(npc_id: i32, fare: i32, player: &Player) -> NpcScript {
    let price = fare.to_string();
    let price = [("price", price.as_str())];
    let (beach, portal) = FLORINA_BEACH;
    let go = [
        Effect::SaveLocation(FLORINA_RETURN, Some(player.stats.map_id)),
        Effect::WarpToPortal(beach, portal),
    ];
    let mut paid = vec![Effect::SpendMeso(fare)];
    paid.extend(go.clone());
    let pay = gated(
        content::text().get(keys::FLORINA_PAY_ASK, &price),
        Some(keys::FLORINA_DECLINE),
        keys::FLORINA_SHORT,
        player.meso >= fare,
        paid,
    );
    let ticket = gated(
        text(keys::FLORINA_TICKET_ASK),
        Some(keys::FLORINA_DECLINE),
        keys::FLORINA_NO_TICKET,
        holds(player, VIP_TICKET_TO_FLORINA),
        go.to_vec(),
    );
    let what = vec![
        Page::Say(text(keys::FLORINA_WHAT_1)),
        Page::Say(text(keys::FLORINA_WHAT_2)),
    ];
    let mut script = NpcScript::new(
        npc_id,
        vec![Page::Menu {
            text: content::text().get(keys::FLORINA_MENU, &price),
            choices: vec![
                (content::text().get(keys::FLORINA_PAY, &price), pay),
                (text(keys::FLORINA_TICKET), ticket),
                (text(keys::FLORINA_WHAT), what),
            ],
        }],
    );
    script.refused = Some(text(keys::FLORINA_SHORT));
    script
}

/// `florina1`: Pison takes the player back where their trip to Florina
/// Beach began, else to Lith Harbor.
fn pison(player: &Player) -> NpcScript {
    let back = saved_location(player, FLORINA_RETURN, LITH_HARBOR);
    let map = back.to_string();
    let map = [("map", map.as_str())];
    NpcScript::new(
        PISON,
        vec![
            Page::Say(content::text().get(keys::FLORINA_LEAVE, &map)),
            Page::Ask {
                text: content::text().get(keys::FLORINA_LEAVE_ASK, &map),
                on_yes: vec![
                    Effect::SaveLocation(FLORINA_RETURN, None),
                    Effect::Warp(back),
                ],
                on_no: Some(content::text().get(keys::FLORINA_LEAVE_DECLINE, &map)),
            },
        ],
    )
}

/// Thomas Swift, free from Henesys to Amoria and back. `None` elsewhere.
fn thomas_swift(player: &Player) -> Option<NpcScript> {
    let (ask, (to, portal)) = match player.stats.map_id {
        id if id == HENESYS.0 => (keys::AMORIA_TO_AMORIA, AMORIA),
        id if id == AMORIA.0 => (keys::AMORIA_TO_HENESYS, HENESYS),
        _ => return None,
    };
    Some(NpcScript::new(
        THOMAS_SWIFT,
        vec![
            Page::Ask {
                text: text(ask),
                on_yes: Vec::new(),
                on_no: Some(text(keys::AMORIA_DECLINE)),
            },
            Page::Say(text(keys::AMORIA_BYE)),
            Page::Done(vec![Effect::WarpToPortal(to, portal)]),
        ],
    ))
}

/// `EngageRing`: Moony makes a ring box for a blessed groom, or throws
/// away the boxes the player holds.
fn moony(player: &Player) -> NpcScript {
    let boxes: Vec<(i32, i32)> = RING_BOXES
        .iter()
        .map(|&(ring_box, ..)| (ring_box, player.inventory.count(ring_box)))
        .filter(|&(_, count)| count > 0)
        .collect();
    let make = if marked(player, BLESSED) {
        if !boxes.is_empty() {
            vec![Page::Say(text(keys::MOONY_HAVE_BOX))]
        } else if player.stats.gender != 0 {
            vec![Page::Say(text(keys::MOONY_GROOM_ONLY))]
        } else {
            vec![Page::Menu {
                text: text(keys::MOONY_CHOOSE),
                choices: RING_BOXES
                    .iter()
                    .map(|&ring| (format!("#t{}#", ring.0), ring_order(ring, player)))
                    .collect(),
            }]
        }
    } else if marked(player, BLESSING_SOUGHT) {
        vec![Page::Say(text(keys::MOONY_SEEKING))]
    } else {
        vec![
            Page::Say(text(keys::MOONY_NEED_BLESSING)),
            Page::Done(vec![Effect::Mark(BLESSING_SOUGHT)]),
        ]
    };
    let discard = if boxes.is_empty() {
        vec![Page::Say(text(keys::MOONY_NO_BOX))]
    } else {
        let effects = boxes
            .iter()
            .map(|&(item_id, count)| Effect::TakeItem {
                item_id,
                quantity: count as i16,
            })
            .collect();
        vec![Page::Act {
            text: text(keys::MOONY_DISCARDED),
            effects,
        }]
    };
    let mut script = NpcScript::new(
        MOONY,
        vec![Page::Menu {
            text: text(keys::MOONY_MENU),
            choices: vec![
                (text(keys::MOONY_MAKE), make),
                (text(keys::MOONY_DISCARD), discard),
            ],
        }],
    );
    script.inventory_full = Some(text(keys::MOONY_FULL));
    script.refused = Some(text(keys::MOONY_NO_STONES));
    script
}

/// Moony's question for one ring: the stones and the fee, and on yes the
/// box.
fn ring_order((ring_box, stone, fee): (i32, i32, i32), player: &Player) -> Vec<Page> {
    let (box_id, stone_id, diamond, price) = (
        ring_box.to_string(),
        stone.to_string(),
        DIAMOND.to_string(),
        fee.to_string(),
    );
    let ask = content::text().get(
        keys::MOONY_ASK,
        &[
            ("box", box_id.as_str()),
            ("stone", stone_id.as_str()),
            ("diamond", diamond.as_str()),
            ("price", price.as_str()),
        ],
    );
    let refusal = if player.meso < fee {
        Some(keys::MOONY_SHORT)
    } else if !holds(player, stone) || !holds(player, DIAMOND) {
        Some(keys::MOONY_NO_STONES)
    } else {
        None
    };
    let on_yes = vec![
        Effect::SpendMeso(fee),
        Effect::TakeItem {
            item_id: stone,
            quantity: 1,
        },
        Effect::TakeItem {
            item_id: DIAMOND,
            quantity: 1,
        },
        Effect::GiveItem {
            item_id: ring_box,
            quantity: 1,
        },
    ];
    let mut pages = gated(
        ask,
        Some(keys::MOONY_DECLINE),
        refusal.unwrap_or(keys::MOONY_NO_STONES),
        refusal.is_none(),
        on_yes,
    );
    if refusal.is_none() {
        pages.push(Page::Say(text(keys::MOONY_DONE)));
    }
    pages
}

/// `MeetPare`: Mom and Dad ask the player sent by Moony for four Proofs of
/// Love, and bless them for the proofs. `None` (their own lines) otherwise.
fn mom_and_dad(player: &Player) -> Option<NpcScript> {
    if !marked(player, BLESSING_SOUGHT) {
        return None;
    }
    if !marked(player, PROOFS_ASKED) {
        let mut script = NpcScript::new(
            MOM_AND_DAD,
            vec![
                Page::Say(text(keys::PARENTS_ASK_1)),
                Page::Say(text(keys::PARENTS_ASK_2)),
                Page::Say(text(keys::PARENTS_ASK_3)),
            ],
        );
        script.on_end = vec![Effect::Mark(PROOFS_ASKED)];
        return Some(script);
    }
    let proofs: Vec<(i32, i32)> = (0..NANAS.len() as i32)
        .map(|i| (PROOF_OF_LOVE + i, player.inventory.count(PROOF_OF_LOVE + i)))
        .filter(|&(_, count)| count > 0)
        .collect();
    if proofs.len() < PROOFS_WANTED {
        return Some(NpcScript::new(
            MOM_AND_DAD,
            vec![Page::Say(text(keys::PARENTS_REMIND))],
        ));
    }
    let mut effects = vec![
        Effect::Unmark(BLESSING_SOUGHT),
        Effect::Unmark(PROOFS_ASKED),
        Effect::Mark(BLESSED),
        Effect::GainExp(BLESSING_EXP),
    ];
    for i in 0..NANAS.len() as i16 {
        effects.extend([
            Effect::Unmark(NANA_ASKED + i),
            Effect::Unmark(NANA_DONE + i),
        ]);
    }
    effects.extend(proofs.iter().map(|&(item_id, count)| Effect::TakeItem {
        item_id,
        quantity: count as i16,
    }));
    let mut script = NpcScript::new(
        MOM_AND_DAD,
        vec![
            Page::Say(text(keys::PARENTS_PROUD)),
            Page::Act {
                text: text(keys::PARENTS_BLESS),
                effects,
            },
        ],
    );
    script.refused = Some(text(keys::PARENTS_REMIND));
    Some(script)
}

/// `ProofHene` and the rest: while the blessing is sought, a Nana asks for
/// 50 of her town's drop and gives her Proof of Love for them, the first
/// time with EXP. `None` (her own lines) otherwise, and in towns without a
/// task, as the Kerning City Nana in Magatia and Singapore.
fn nana(npc_id: i32, player: &Player) -> Option<NpcScript> {
    if !marked(player, BLESSING_SOUGHT) {
        return None;
    }
    let here = player.stats.map_id;
    let place = NANA_TASKS.iter().position(|&(town, ..)| town == here)?;
    let (_, drop, exp) = NANA_TASKS[place];
    let proof = PROOF_OF_LOVE + place as i32;
    let (asked, done) = (NANA_ASKED + place as i16, NANA_DONE + place as i16);
    let drop_id = drop.to_string();
    let drop_arg = [("item", drop_id.as_str())];
    let say = |key: &str| content::text().get(key, &drop_arg);
    let enough = player.inventory.count(drop) >= i32::from(NANA_TASK_COUNT);
    let mut trade = vec![
        Effect::TakeItem {
            item_id: drop,
            quantity: NANA_TASK_COUNT,
        },
        Effect::GiveItem {
            item_id: proof,
            quantity: 1,
        },
    ];
    let pages = if holds(player, proof) {
        vec![Page::Say(say(keys::NANA_HAVE_PROOF))]
    } else if marked(player, done) {
        // Again, for the drops alone.
        vec![
            Page::Ask {
                text: say(keys::NANA_AGAIN),
                on_yes: if enough { trade } else { Vec::new() },
                on_no: None,
            },
            Page::Say(say(if enough {
                keys::NANA_THANKS
            } else {
                keys::NANA_NEED
            })),
        ]
    } else if marked(player, asked) {
        if enough {
            trade.extend([
                Effect::GainExp(exp),
                Effect::Unmark(asked),
                Effect::Mark(done),
            ]);
            vec![Page::Act {
                text: say(keys::NANA_THANKS),
                effects: trade,
            }]
        } else {
            vec![Page::Say(say(keys::NANA_NEED))]
        }
    } else {
        vec![
            Page::Ask {
                text: say(keys::NANA_FAVOUR),
                on_yes: vec![Effect::Mark(asked)],
                on_no: Some(say(keys::NANA_DECLINE)),
            },
            Page::Say(say(keys::NANA_TASK)),
        ]
    };
    let mut script = NpcScript::new(npc_id, pages);
    script.inventory_full = Some(say(keys::NANA_FULL));
    script.refused = Some(say(keys::NANA_NEED));
    Some(script)
}

/// The venue a wedding assistant books, if `npc_id` is one.
pub fn wedding_venue(npc_id: i32) -> Option<Venue> {
    match npc_id {
        NICOLE => Some(Venue::Cathedral),
        BONNIE => Some(Venue::Chapel),
        _ => None,
    }
}

fn wedding_assistant_of(venue: Venue) -> i32 {
    match venue {
        Venue::Cathedral => NICOLE,
        Venue::Chapel => BONNIE,
    }
}

/// The venue whose weddings `npc_id` begins in Amoria, if it is one who
/// does.
pub fn wedding_starter_venue(npc_id: i32) -> Option<Venue> {
    match npc_id {
        WAYNE => Some(Venue::Chapel),
        HIGH_PRIEST => Some(Venue::Cathedral),
        _ => None,
    }
}

fn wedding_starter_of(venue: Venue) -> i32 {
    match venue {
        Venue::Chapel => WAYNE,
        Venue::Cathedral => HIGH_PRIEST,
    }
}

/// The venue whose altar `npc_id` marries couples at, if a priest.
pub fn wedding_priest_venue(npc_id: i32) -> Option<Venue> {
    match npc_id {
        CHAPEL_PRIEST => Some(Venue::Chapel),
        HIGH_PRIEST => Some(Venue::Cathedral),
        _ => None,
    }
}

fn wedding_priest_of(venue: Venue) -> i32 {
    match venue {
        Venue::Chapel => CHAPEL_PRIEST,
        Venue::Cathedral => HIGH_PRIEST,
    }
}

/// The venue whose altar `npc_id` sees weddings out of, if one who does.
pub fn wedding_staff_venue(npc_id: i32) -> Option<Venue> {
    match npc_id {
        JACKIE => Some(Venue::Chapel),
        NANCY => Some(Venue::Cathedral),
        _ => None,
    }
}

/// `chapel`'s text, or `cathedral`'s at the Cathedral.
fn venue_text(venue: Venue, chapel: &str, cathedral: &str) -> String {
    text(match venue {
        Venue::Chapel => chapel,
        Venue::Cathedral => cathedral,
    })
}

/// The wedding starter of `venue` (Wayne for the Chapel, the High Priest
/// for the Cathedral) begins the
/// couple's wedding booked there on this channel once its hour has come,
/// with wedding clothes at hand; their partner must be beside them, which
/// the directory checks (report b071).
fn wedding_starter(
    venue: Venue,
    player: &Player,
    reservation: Option<&Reservation>,
    channel: u8,
    now: SystemTime,
) -> NpcScript {
    let engaged = player
        .marriage
        .as_ref()
        .is_some_and(|record| record.status == marriage::ENGAGED);
    let booked = reservation.filter(|reservation| engaged && reservation.venue == venue);
    let page = match booked {
        None if engaged => Page::Say(venue_text(
            venue,
            keys::WEDDING_STARTER_NOT_BOOKED,
            keys::WEDDING_CATHEDRAL_STARTER_NOT_BOOKED,
        )),
        None => Page::Say(venue_text(
            venue,
            keys::WEDDING_STARTER_INTRO,
            keys::WEDDING_CATHEDRAL_STARTER_INTRO,
        )),
        Some(reservation) if reservation.channel != channel => Page::Say(wedding_text(
            keys::WEDDING_STARTER_CHANNEL,
            reservation,
            now,
        )),
        Some(reservation) if reservation.inviting(now) => {
            Page::Say(wedding_text(keys::WEDDING_STARTER_EARLY, reservation, now))
        }
        Some(_) if !wedding::has_clothes(player) => Page::Say(text(keys::WEDDING_STARTER_CLOTHES)),
        Some(_) => Page::Ask {
            text: venue_text(
                venue,
                keys::WEDDING_STARTER_ASK,
                keys::WEDDING_CATHEDRAL_STARTER_ASK,
            ),
            on_yes: vec![Effect::StartWedding { venue }],
            on_no: None,
        },
    };
    NpcScript::new(wedding_starter_of(venue), vec![page])
}

/// The priest at the altar asks each of the couple for their vows, once
/// they wear wedding clothes, hold their half of the engagement and have
/// room for the ring (report b071). The marriage comes with the second.
fn wedding_priest(venue: Venue, player: &Player, stage: Option<WeddingStage>) -> NpcScript {
    let say = |key| Page::Say(text(key));
    let married = || {
        Page::Say(venue_text(
            venue,
            keys::WEDDING_PRIEST_MARRIED,
            keys::WEDDING_CATHEDRAL_PRIEST_MARRIED,
        ))
    };
    let record = player.marriage.as_ref();
    let token = record.is_some_and(|record| {
        let (empty_box, ring) = marriage::tokens(record.item_id);
        holds(player, empty_box) || holds(player, ring)
    });
    let page = match stage {
        None => say(keys::WEDDING_PRIEST_INTRO),
        Some(WeddingStage::Gathering) => say(keys::WEDDING_STAFF_WAIT),
        Some(WeddingStage::Married) => married(),
        Some(WeddingStage::Ceremony) if !wedding::dressed(player) => {
            say(keys::WEDDING_PRIEST_CLOTHES)
        }
        Some(WeddingStage::Ceremony) if !token => say(keys::WEDDING_PRIEST_TOKEN),
        Some(WeddingStage::Ceremony) => {
            let ring = record.map_or(0, |record| record.ring_id());
            if items::fits_until(&player.inventory, ring, 1, None) {
                let partner = record
                    .map(|record| {
                        if record.groom_id == player.stats.id {
                            record.bride_name.as_str()
                        } else {
                            record.groom_name.as_str()
                        }
                    })
                    .unwrap_or_default();
                Page::Ask {
                    text: content::text().get(keys::WEDDING_PRIEST_ASK, &[("partner", partner)]),
                    on_yes: vec![Effect::Vows],
                    on_no: None,
                }
            } else {
                say(keys::WEDDING_PRIEST_ROOM)
            }
        }
    };
    NpcScript::new(wedding_priest_of(venue), vec![page])
}

/// `key`'s text about the booked wedding: its ticket, the minutes left at
/// `now`, and its channel as the player counts them.
pub fn wedding_text(key: &str, reservation: &Reservation, now: SystemTime) -> String {
    let ticket = reservation.ticket().to_string();
    let minutes = reservation.minutes_left(now).to_string();
    let channel = (u16::from(reservation.channel) + 1).to_string();
    content::text().get(
        key,
        &[
            ("ticket", ticket.as_str()),
            ("minutes", minutes.as_str()),
            ("channel", channel.as_str()),
        ],
    )
}

/// What the partner's side refused a booking for, in the assistant's
/// words.
pub fn partner_refusal_text(refusal: PartnerRefusal) -> String {
    text(match refusal {
        PartnerRefusal::Offline => keys::WEDDING_PARTNER_OFFLINE,
        PartnerRefusal::Elsewhere => keys::WEDDING_PARTNER_ELSEWHERE,
        PartnerRefusal::HasRing => keys::WEDDING_PARTNER_HAS_RING,
        PartnerRefusal::NoRoom => keys::WEDDING_PARTNER_NO_ROOM,
    })
}

/// Whether the player has room for a booking's invitations, which expire
/// and so take a slot of their own.
pub fn invitations_fit(player: &Player, venue: Venue) -> bool {
    invitations_fit_n(player, venue, wedding::INVITATIONS)
}

/// Nicole (`cathedral`) and Bonnie (`vegas`) in Amoria tell how a wedding
/// is prepared, and book an engaged couple's for a ticket from the Cash
/// tab, the premium one first (report b066). The partner's side is checked
/// when the booking is made.
fn wedding_assistant(venue: Venue, player: &Player, view: WeddingView) -> NpcScript {
    let (ticket, premium) = (venue.ticket(false), venue.ticket(true));
    let (ticket_id, premium_id) = (ticket.to_string(), premium.to_string());
    let tickets = [
        ("ticket", ticket_id.as_str()),
        ("premium", premium_id.as_str()),
    ];
    let prepare = vec![Page::Say(
        content::text().get(keys::WEDDING_PREPARE, &tickets),
    )];
    let engaged = player
        .marriage
        .as_ref()
        .is_some_and(|record| record.status == marriage::ENGAGED);
    let refusal = if !engaged {
        Some(text(keys::WEDDING_NO_RING))
    } else if let Some(reservation) = view.reservation {
        Some(wedding_text(keys::WEDDING_BOOKED, reservation, view.now))
    } else if wedding::has_wedding_ring(player) {
        Some(text(keys::WEDDING_HAS_RING))
    } else if !invitations_fit(player, venue) {
        Some(text(keys::WEDDING_NO_ROOM))
    } else if !holds(player, ticket) && !holds(player, premium) {
        Some(content::text().get(keys::WEDDING_NO_TICKET, &tickets))
    } else {
        None
    };
    let reserve = match refusal {
        Some(refusal) => vec![Page::Say(refusal)],
        None => vec![Page::Done(vec![Effect::BookWedding {
            venue,
            premium: holds(player, premium),
        }])],
    };
    let mut choices = vec![
        (text(keys::WEDDING_CHOICE_PREPARE), prepare),
        (text(keys::WEDDING_CHOICE_RESERVE), reserve),
        (
            text(keys::WEDDING_CHOICE_GUEST),
            wedding_guest(venue, player, &view),
        ),
    ];
    if holds(player, wedding::INVITATION_TICKET) {
        let own = view.reservation.filter(|_| engaged);
        choices.push((
            text(keys::WEDDING_CHOICE_EXTRA),
            extra_invitations(venue, player, own, view.now),
        ));
    }
    NpcScript::new(
        wedding_assistant_of(venue),
        vec![Page::Menu {
            text: text(keys::WEDDING_MENU),
            choices,
        }],
    )
}

/// A guest asking to go in: invited, with the invitation, to a wedding at
/// the venue that is due on this channel. Whether the couple have begun it
/// and still gather is the wedding's to say; guests are only reminded of
/// the clothes (report b071 Q1).
fn wedding_guest(venue: Venue, player: &Player, view: &WeddingView) -> Vec<Page> {
    let say = match view.guest_of {
        Some(wedding) if holds(player, venue.received_invitation()) => {
            if wedding.inviting(view.now) {
                wedding_text(keys::WEDDING_GUEST_EARLY, wedding, view.now)
            } else if wedding.channel != view.channel {
                wedding_text(keys::WEDDING_GUEST_CHANNEL, wedding, view.now)
            } else {
                let key = if wedding::has_clothes(player) {
                    keys::WEDDING_GUEST_ASK
                } else {
                    keys::WEDDING_GUEST_ASK_CLOTHES
                };
                return vec![Page::Ask {
                    text: text(key),
                    on_yes: vec![Effect::EnterWedding { venue }],
                    on_no: None,
                }];
            }
        }
        _ => text(keys::WEDDING_GUEST_NOT_INVITED),
    };
    vec![Page::Say(say)]
}

/// What the assistant of `venue` says to a guest the wedding didn't let
/// in.
pub fn guest_refusal_text(refusal: GuestRefusal) -> String {
    text(match refusal {
        GuestRefusal::NotStarted => keys::WEDDING_GUEST_WAIT,
        GuestRefusal::Late => keys::WEDDING_GUEST_LATE,
        GuestRefusal::Full => keys::WEDDING_GUEST_FULL,
    })
}

/// Whether the player has room for `quantity` invitations to `venue`'s
/// wedding, which expire and so take a slot of their own unless held.
fn invitations_fit_n(player: &Player, venue: Venue, quantity: i16) -> bool {
    let until = Some(SystemTime::UNIX_EPOCH);
    items::fits_until(&player.inventory, venue.invitation(), quantity, until)
}

/// A Wedding Invitation Ticket traded for invitations to the couple's
/// wedding at the venue, before it starts.
fn extra_invitations(
    venue: Venue,
    player: &Player,
    own: Option<&Reservation>,
    now: SystemTime,
) -> Vec<Page> {
    let refusal = match own.filter(|reservation| reservation.venue == venue) {
        None => Some(text(keys::WEDDING_EXTRA_NOT_BOOKED)),
        Some(reservation) if !reservation.inviting(now) => Some(text(keys::WEDDING_EXTRA_STARTED)),
        Some(_) if !invitations_fit_n(player, venue, wedding::EXTRA_INVITATIONS) => {
            Some(text(keys::WEDDING_NO_ROOM))
        }
        Some(_) => None,
    };
    match refusal {
        Some(refusal) => vec![Page::Say(refusal)],
        None => vec![Page::Done(vec![Effect::ExtraInvitations { venue }])],
    }
}

/// A choice of a trip to `to` for `fare`, which on a yes pays and `warp`s.
fn fare_choice(to: i32, fare: i32, warp: Effect, player: &Player) -> (String, Vec<Page>) {
    let (map, price) = (to.to_string(), fare.to_string());
    let args = [("map", map.as_str()), ("price", price.as_str())];
    let ask = content::text().get(keys::FERRY_ASK, &args);
    let pages = ride(
        ask,
        Some(keys::FERRY_DECLINE),
        keys::FERRY_SHORT,
        fare,
        warp,
        player,
    );
    (content::text().get(keys::FERRY_CHOICE, &args), pages)
}

/// A ferry's pilot: aboard for the fare, and the ride map's timer does the
/// rest.
fn ferry(npc_id: i32, ride: rides::Ride, fare: i32, player: &Player) -> NpcScript {
    let warp = Effect::WarpToPortal(ride.map, 0);
    let choices = vec![fare_choice(ride.to.0, fare, warp, player)];
    let mut script = NpcScript::new(
        npc_id,
        vec![Page::Menu {
            text: text(keys::FERRY_INTRO),
            choices,
        }],
    );
    script.refused = Some(text(keys::FERRY_SHORT));
    script
}

/// The crane from where the player stands. `None` elsewhere.
fn crane(player: &Player) -> Option<NpcScript> {
    let aboard = |ride: rides::Ride| Effect::WarpToPortal(ride.map, 0);
    let to_herb_town = Effect::WarpToPortal(HERB_TOWN, 0);
    let pages = match player.stats.map_id {
        CRANE_PLATFORM => vec![Page::Menu {
            text: text(keys::FERRY_INTRO),
            choices: vec![fare_choice(
                MU_LUNG_TEMPLE,
                CRANE_FARE,
                aboard(rides::ORBIS_TO_MU_LUNG),
                player,
            )],
        }],
        MU_LUNG_TEMPLE => vec![Page::Menu {
            text: text(keys::FERRY_INTRO),
            choices: vec![
                fare_choice(
                    CRANE_PLATFORM,
                    CRANE_FARE,
                    aboard(rides::MU_LUNG_TO_ORBIS),
                    player,
                ),
                fare_choice(HERB_TOWN, HERB_TOWN_FARE, to_herb_town, player),
            ],
        }],
        // Just the question, with no menu.
        HERB_TOWN => {
            let back = Effect::WarpToPortal(MU_LUNG_TEMPLE, 0);
            fare_choice(MU_LUNG_TEMPLE, HERB_TOWN_FARE, back, player).1
        }
        _ => return None,
    };
    let mut script = NpcScript::new(CRANE, pages);
    script.refused = Some(text(keys::FERRY_SHORT));
    Some(script)
}

/// Audrey's rides around Malaysia from where the player stands. `None`
/// elsewhere.
fn audrey(player: &Player) -> Option<NpcScript> {
    let here = player.stats.map_id;
    let ride = |(to, portal): (i32, u8), fare: i32| {
        let (map, price) = (to.to_string(), fare.to_string());
        let args = [("map", map.as_str()), ("price", price.as_str())];
        let mut go = vec![Effect::SpendMeso(fare)];
        if here == MALAYSIA_CBD {
            go.push(Effect::SaveLocation(WORLD_TOUR_RETURN, Some(here)));
        }
        go.push(Effect::WarpToPortal(to, portal));
        let ask = content::text().get(keys::MALAYSIA_ASK, &args);
        let pages = gated(
            ask,
            Some(keys::MALAYSIA_DECLINE),
            keys::MALAYSIA_SHORT,
            player.meso >= fare,
            go,
        );
        (content::text().get(keys::MALAYSIA_CHOICE, &args), pages)
    };
    let (intro, choices) = match here {
        MALAYSIA_CBD => (
            keys::MALAYSIA_INTRO_CBD,
            vec![ride(CBD_TO_TREND_ZONE, CBD_FARE)],
        ),
        TREND_ZONE_METROPOLIS => {
            let back = saved_location(player, WORLD_TOUR_RETURN, BOAT_QUAY_TOWN);
            // Only the Malaysian towns are saved on the way here; anywhere
            // else arrives at its first spawn point.
            let warp = if back == MALAYSIA_CBD || back == BOAT_QUAY_TOWN {
                Effect::WarpToPortal(back, MALAYSIA_HOME_PORTAL)
            } else {
                Effect::Warp(back)
            };
            let home = vec![
                Page::Say(text(keys::MALAYSIA_HOME_GO)),
                Page::Done(vec![Effect::SaveLocation(WORLD_TOUR_RETURN, None), warp]),
            ];
            let map = back.to_string();
            let label = content::text().get(keys::MALAYSIA_HOME, &[("map", map.as_str())]);
            (
                keys::MALAYSIA_INTRO,
                vec![ride(TO_KAMPUNG_VILLAGE, MALAYSIA_FARE), (label, home)],
            )
        }
        KAMPUNG_VILLAGE => (
            keys::MALAYSIA_INTRO,
            vec![ride(BACK_TO_TREND_ZONE, MALAYSIA_FARE)],
        ),
        _ => return None,
    };
    let mut script = NpcScript::new(
        AUDREY,
        vec![Page::Menu {
            text: text(intro),
            choices,
        }],
    );
    script.refused = Some(text(keys::MALAYSIA_SHORT));
    Some(script)
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
        let stats = &player.stats;
        // Any job of the chief's class from level 50, Aran and Cygnus
        // included (upstream), may ask for Zakum.
        script.pages = if stats.job / 100 % 10 * 100 == line.job && stats.level >= ZAKUM_LEVEL {
            zakum_approval(player)
        } else {
            vec![Page::Say(say("chief.other"))]
        };
        return script;
    }
    let job = player.stats.job + 1;
    // Past the Holy Stone, the chief only speaks of the advance; before,
    // a second job may pick the test or Zakum.
    let advancing = marked(player, HOLY_STONE_ASSIGNED) && holds(player, NECKLACE_OF_WISDOM);
    let pages = if advancing {
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
        vec![Page::Act {
            text: say("chief.strength"),
            effects: vec![
                Effect::TakeItem {
                    item_id: NECKLACE_OF_STRENGTH,
                    quantity: 1,
                },
                Effect::Mark(HOLY_STONE_ASSIGNED),
            ],
        }]
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
    script.pages = if advancing {
        pages
    } else {
        vec![Page::Menu {
            text: text(keys::ZAKUM_CHIEF_MENU),
            choices: vec![
                (text(keys::ZAKUM_CHIEF_THIRD_JOB), pages),
                (text(keys::ZAKUM_CHIEF_ZAKUM), zakum_approval(player)),
            ],
        }]
    };
    script
}

/// The lowest level a chief approves for Zakum, and Adobis warns of the
/// trials (upstream).
const ZAKUM_LEVEL: u8 = 50;
pub const ADOBIS: i32 = 2030008;
pub const AURA: i32 = 2032002;
/// In the Room of Tragedy, 280090000, where those who die or come back to
/// the game inside the Dead Mine land.
const ALI: i32 = 2030011;
/// NPCs with no speech in String.nx and nothing else to do, and their one
/// line: Wolf Spirit Ryko in the Hall of Bowmen, Adonis in El Nath, the
/// Document Roll in Chloe's house, Mu Lung's Practice Chart, Sharenian's
/// Knight Armor and Sharen III's Will, and the Sunstone, Moonstone, Enigma
/// and Hollowed Ground graves.
const ONE_LINERS: &[(i32, &str)] = &[
    (9010021, keys::LINE_RYKO),
    (9201106, keys::LINE_ADONIS),
    (2040031, keys::LINE_DOCUMENT_ROLL),
    (2096000, keys::LINE_PRACTICE_CHART),
    (9040012, keys::LINE_KNIGHT_ARMOR),
    (9040007, keys::LINE_SHAREN_WILL),
    (9201071, keys::LINE_GRAVE_LIGHT),
    (9201072, keys::LINE_GRAVE_LIGHT),
    (9201073, keys::LINE_ENIGMA_TOMBSTONE),
    (9201084, keys::LINE_WORN_TOMBSTONE),
];
const LIRA: i32 = 2032003;
const AMON: i32 = 2030010;
/// From the Dead Mine (stage 1), for the Breath of Lava.
const FIRE_ORE: i32 = 4031061;
/// Lira's, past the Breath of Lava (stage 2).
const BREATH_OF_FIRE: i32 = 4031062;
const GOLD_TOOTH: i32 = 4000082;
const GOLD_TEETH_TO_FORGE: i16 = 30;
pub(crate) const EYE_OF_FIRE: i32 = 4001017;
/// Where Adobis stands, and the gate to Zakum.
const DOOR_TO_ZAKUM: i32 = 211042300;
const BREATH_OF_LAVA: i32 = 280020000;
const ZAKUM_ALTAR: i32 = 280030000;
/// Where Amon takes players off the altar: the expedition's exit in
/// upstream's ZakumBattle event (its Amon script says the Door to Zakum).
pub(crate) const ENTRANCE_TO_ZAKUM: i32 = 211042400;

pub(crate) fn zakum_approved(player: &Player) -> bool {
    marked(player, ZAKUM_APPROVED)
}

/// A chief's approval for Zakum, given once and repeated after.
fn zakum_approval(player: &Player) -> Vec<Page> {
    let approved = text(keys::ZAKUM_CHIEF_APPROVED);
    if zakum_approved(player) {
        vec![Page::Say(approved)]
    } else {
        vec![Page::Act {
            text: approved,
            effects: vec![Effect::Mark(ZAKUM_APPROVED)],
        }]
    }
}

/// Adobis at the Door to Zakum: a warning until a chief approves, then
/// the trials. The party's leader takes it into the Dead Mine (stage 1),
/// whose prize, a Piece of Fire Ore, the Breath of Lava takes, and the forge turns both trials' items and 30
/// Gold Teeth into five Eyes of Fire.
fn adobis(entry: Entry, player: &Player) -> NpcScript {
    let mut script = NpcScript::new(ADOBIS, Vec::new());
    if !zakum_approved(player) {
        let key = if player.stats.level >= ZAKUM_LEVEL {
            keys::ZAKUM_ADOBIS_UNAPPROVED
        } else {
            keys::ZAKUM_ADOBIS_WARNING
        };
        script.pages = vec![Page::Say(text(key))];
        return script;
    }
    let (ore, breath) = (holds(player, FIRE_ORE), holds(player, BREATH_OF_FIRE));
    let lava = if ore && !breath {
        Page::Ask {
            text: text(keys::ZAKUM_ADOBIS_LAVA_ASK),
            on_yes: vec![Effect::WarpToPortal(BREATH_OF_LAVA, 0)],
            on_no: None,
        }
    } else if breath {
        Page::End(text(keys::ZAKUM_ADOBIS_LAVA_HAVE))
    } else {
        Page::End(text(keys::ZAKUM_ADOBIS_LAVA_EARLIER))
    };
    let take = |item_id, quantity| Effect::TakeItem { item_id, quantity };
    let forge = if !(ore && breath) {
        Page::End(text(keys::ZAKUM_ADOBIS_FORGE_LACK))
    } else if player.inventory.count(GOLD_TOOTH) < i32::from(GOLD_TEETH_TO_FORGE) {
        Page::End(text(keys::ZAKUM_ADOBIS_FORGE_SHORT))
    } else {
        Page::Act {
            text: text(keys::ZAKUM_ADOBIS_FORGE_DONE),
            effects: vec![
                take(FIRE_ORE, 1),
                take(BREATH_OF_FIRE, 1),
                take(GOLD_TOOTH, GOLD_TEETH_TO_FORGE),
                Effect::GiveItem {
                    item_id: EYE_OF_FIRE,
                    quantity: 5,
                },
                Effect::Mark(ZAKUM_TRIALS_DONE),
            ],
        }
    };
    let mine = match entry {
        Entry::NoParty => Page::End(text(keys::ZAKUM_ADOBIS_MINE_NO_PARTY)),
        Entry::NotLeader => Page::End(text(keys::ZAKUM_ADOBIS_MINE_NOT_LEADER)),
        Entry::NotEligible => Page::End(event_terms(
            &events::ZPQ,
            keys::ZAKUM_ADOBIS_MINE_NOT_ELIGIBLE,
        )),
        Entry::Busy => Page::End(text(keys::ZAKUM_ADOBIS_MINE_BUSY)),
        Entry::Ready => Page::Done(vec![Effect::StartEvent(EventKind::Zpq)]),
    };
    script.pages = vec![Page::Menu {
        text: text(keys::ZAKUM_ADOBIS_MENU),
        choices: vec![
            (text(keys::ZAKUM_ADOBIS_MINE_CHOICE), vec![mine]),
            (text(keys::ZAKUM_ADOBIS_LAVA_CHOICE), vec![lava]),
            (text(keys::ZAKUM_ADOBIS_FORGE_CHOICE), vec![forge]),
        ],
    }];
    script.inventory_full = Some(text(keys::ZAKUM_ADOBIS_FORGE_FULL));
    // Someone left or moved since he checked, another party went in, or
    // the forge's items went.
    script.refused = Some(text(keys::ZAKUM_ADOBIS_REFUSED));
    script
}

/// Adobis again (2030013), at the Entrance to Zakum.
pub const ZAKUM_RECRUITER: i32 = 2030013;

/// The Mark of the Squad (2083004), at the Entrance to Horntail's Cave.
pub const HORNTAIL_RECRUITER: i32 = 2083004;

/// The NPC who gathers a kind of expedition.
#[derive(Clone, Copy, Debug)]
pub struct Recruiter {
    pub npc: i32,
    /// The item one needs to lead one, if any, and what the NPC says to
    /// one without.
    pub needs: Option<(i32, &'static str)>,
}

/// Who gathers `kind`'s expeditions, if a squad goes to it.
pub fn recruiter(kind: EventKind) -> Option<Recruiter> {
    match kind {
        EventKind::Zakum => Some(Recruiter {
            npc: ZAKUM_RECRUITER,
            needs: Some((EYE_OF_FIRE, keys::ZAKUM_SQUAD_NO_EYE)),
        }),
        EventKind::Horntail => Some(Recruiter {
            npc: HORNTAIL_RECRUITER,
            needs: None,
        }),
        _ => None,
    }
}

/// The NPC who gathers the channel's expedition of `kind`
/// (`crate::squads`; Cosmic `2030013`, `2083004`): one of the levels, with
/// the item it needs if any (Zakum's Eye of Fire), opens it, others of the
/// levels join, and its leader turns members out, takes it in, or calls it
/// off. While it runs, those on its roster may go back in.
fn squad(kind: EventKind, view: SquadView, player: &Player) -> Option<NpcScript> {
    let Recruiter { npc, needs } = recruiter(kind)?;
    let says = squads::text(kind)?;
    let event = kind.event();
    let squad = |command| Effect::Squad { kind, command };
    let named = |key: &str, name: &str| content::text().get(key, &[("name", name)]);
    let pages = match view {
        _ if !event.levels.contains(&player.stats.level) => {
            vec![Page::Say(event_terms(event, says.level))]
        }
        SquadView::None => match needs.filter(|&(item, _)| !holds(player, item)) {
            Some((_, without)) => vec![Page::Say(text(without))],
            None => vec![
                Page::Ask {
                    text: event_terms(event, says.ask),
                    on_yes: vec![squad(SquadCommand::Open)],
                    on_no: None,
                },
                Page::Say(text(keys::SQUAD_OPENED)),
            ],
        },
        SquadView::Leading(others) => {
            let count = (others.len() + 1).to_string();
            let expel = if others.is_empty() {
                Page::End(text(keys::SQUAD_ALONE))
            } else {
                Page::Menu {
                    text: text(keys::SQUAD_LIST),
                    choices: others
                        .iter()
                        .map(|(id, name)| {
                            let act = Page::Act {
                                text: named(keys::SQUAD_EXPELLED, name),
                                effects: vec![squad(SquadCommand::Expel(*id))],
                            };
                            (name.clone(), vec![act])
                        })
                        .collect(),
                }
            };
            let least = event.least();
            let start = if others.len() + 1 < least {
                let least = least.to_string();
                Page::End(content::text().get(keys::SQUAD_TOO_FEW, &[("least", &least)]))
            } else {
                Page::Done(vec![squad(SquadCommand::Start)])
            };
            let stop = Page::Act {
                text: text(keys::SQUAD_STOPPED),
                effects: vec![squad(SquadCommand::Stop)],
            };
            vec![Page::Menu {
                text: content::text().get(keys::SQUAD_MENU, &[("count", &count)]),
                choices: vec![
                    (text(keys::SQUAD_EXPEL_CHOICE), vec![expel]),
                    (text(says.start_choice), vec![start]),
                    (text(keys::SQUAD_STOP_CHOICE), vec![stop]),
                ],
            }]
        }
        SquadView::Registered { leader } => {
            vec![Page::Say(named(keys::SQUAD_REGISTERED, &leader))]
        }
        SquadView::Joinable => vec![
            Page::Ask {
                text: text(says.join_ask),
                on_yes: vec![squad(SquadCommand::Join)],
                on_no: None,
            },
            Page::Say(text(keys::SQUAD_JOINED)),
        ],
        SquadView::Banned { leader } => vec![Page::Say(named(keys::SQUAD_BANNED, &leader))],
        SquadView::Full => vec![Page::Say(text(keys::SQUAD_FULL))],
        SquadView::Rejoin => vec![Page::Ask {
            text: text(says.rejoin_ask),
            on_yes: vec![squad(SquadCommand::Rejoin)],
            on_no: None,
        }],
        SquadView::Taken => vec![Page::Say(text(says.taken))],
    };
    let mut script = NpcScript::new(npc, pages);
    // The squad changed since it looked: another opened one, it filled,
    // started, or ended, or the player left the map.
    script.refused = Some(text(keys::SQUAD_REFUSED));
    Some(script)
}

/// Aura (Cosmic `2032002`). Before the win: what the mine asks, the
/// leader's hand-in of the Fire Ore, which with 30 documents wins more
/// (`EventCommand::WinWithDocuments`), and a way out for anyone. After it,
/// each member's share once: a piece of the Fire Ore, and with the
/// documents five of the mine's return scrolls.
fn aura(view: Aura, player: &Player) -> NpcScript {
    let take = |item_id, quantity| Effect::TakeItem { item_id, quantity };
    let mut script = NpcScript::new(AURA, Vec::new());
    match view {
        Aura::Playing { leader } => {
            let documents = player.inventory.count(events::ZPQ_DOCUMENT)
                >= i32::from(events::ZPQ_DOCUMENTS_WANTED);
            let hand_in = if !leader {
                Page::End(text(keys::ZAKUM_AURA_LEADER_ONLY))
            } else if !holds(player, events::FIRE_ORE) {
                Page::End(text(keys::ZAKUM_AURA_NEED_ORE))
            } else if documents {
                Page::Ask {
                    text: text(keys::ZAKUM_AURA_ASK_DOCUMENTS),
                    on_yes: vec![
                        take(events::FIRE_ORE, 1),
                        take(events::ZPQ_DOCUMENT, events::ZPQ_DOCUMENTS_WANTED),
                        Effect::EventCommand(EventCommand::WinWithDocuments),
                    ],
                    on_no: None,
                }
            } else {
                Page::Ask {
                    text: text(keys::ZAKUM_AURA_ASK),
                    on_yes: vec![
                        take(events::FIRE_ORE, 1),
                        Effect::EventCommand(EventCommand::ClearStage(1)),
                    ],
                    on_no: None,
                }
            };
            let out = Page::Ask {
                text: text(keys::ZAKUM_AURA_OUT_ASK),
                on_yes: vec![Effect::Warp(events::ZPQ.exit)],
                on_no: None,
            };
            script.pages = vec![Page::Menu {
                text: text(keys::ZAKUM_AURA_MENU),
                choices: vec![
                    (
                        text(keys::ZAKUM_AURA_ABOUT_CHOICE),
                        vec![Page::End(text(keys::ZAKUM_AURA_ABOUT))],
                    ),
                    (text(keys::ZAKUM_AURA_HAND_IN_CHOICE), vec![hand_in]),
                    (text(keys::ZAKUM_AURA_OUT_CHOICE), vec![out]),
                ],
            }];
            // The ore or the documents went before the yes.
            script.refused = Some(text(keys::ZAKUM_AURA_NEED_ORE));
        }
        Aura::Won { taken: true, .. } => {
            script.pages = vec![
                Page::Say(text(keys::ZAKUM_AURA_WON)),
                Page::End(text(keys::ZAKUM_AURA_PRIZE_TAKEN)),
            ];
        }
        Aura::Won { documents, .. } => {
            let mut share = vec![Effect::GiveItem {
                item_id: events::FIRE_ORE_PIECE,
                quantity: 1,
            }];
            if documents {
                let (item_id, quantity) = events::DEAD_MINE_SCROLL;
                share.insert(0, Effect::GiveItem { item_id, quantity });
            }
            share.push(Effect::EventCommand(EventCommand::PrizeTaken));
            script.pages = vec![Page::Say(text(keys::ZAKUM_AURA_WON)), Page::Done(share)];
            script.inventory_full = Some(text(keys::ZAKUM_AURA_PRIZE_FULL));
        }
    }
    script
}

/// Ali (Cosmic `2030011`) in the Room of Tragedy sends players back to the
/// Door to Zakum; leaving the Dead Mine already took its items.
fn ali() -> NpcScript {
    NpcScript::new(
        ALI,
        vec![
            Page::Say(text(keys::ZAKUM_ALI_BYE)),
            Page::Done(vec![Effect::Warp(events::ZPQ.exit)]),
        ],
    )
}

/// Lira at the far end of the Breath of Lava: the Breath of Fire and EXP,
/// and back to the Door to Zakum.
fn lira() -> NpcScript {
    let mut script = NpcScript::new(
        LIRA,
        vec![
            Page::Say(text(keys::ZAKUM_LIRA_REWARD)),
            Page::Say(text(keys::ZAKUM_LIRA_GO)),
        ],
    );
    script.on_end = vec![
        Effect::GiveItem {
            item_id: BREATH_OF_FIRE,
            quantity: 1,
        },
        Effect::GainExp(10_000),
        Effect::Warp(DOOR_TO_ZAKUM),
    ];
    script.inventory_full = Some(text(keys::ZAKUM_LIRA_FULL));
    script
}

/// Amon, a way out of the Breath of Lava, and of Zakum's altar.
fn amon(player: &Player) -> NpcScript {
    let out = if player.stats.map_id == ZAKUM_ALTAR {
        ENTRANCE_TO_ZAKUM
    } else {
        DOOR_TO_ZAKUM
    };
    NpcScript::new(
        AMON,
        vec![Page::Ask {
            text: text(keys::ZAKUM_AMON_LEAVE),
            on_yes: vec![Effect::Warp(out)],
            on_no: None,
        }],
    )
}

/// The Tombstone at Deep Inside the Clocktower, by the way to Papulatus.
const TOMBSTONE: i32 = 2041024;
/// The Machine Apparatus at the Origin of Clocktower, the way out of the
/// fight with Papulatus.
const MACHINE_APPARATUS: i32 = 2041025;

fn machine_apparatus() -> NpcScript {
    NpcScript::new(
        MACHINE_APPARATUS,
        vec![Page::Ask {
            text: text(keys::PAPULATUS_MACHINE_LEAVE),
            on_yes: vec![Effect::Warp(events::CLOCKTOWER_DEPTHS)],
            on_no: None,
        }],
    )
}

/// Keroben, at the Cave of Life's entrance (240040700), lets by only the
/// Dragon Squad: those turned to dragons by the Dragon Elixir (2210003,
/// quest 7301's reward) or holding its certificate (4001086, quest 7302's).
const KEROBEN: i32 = 2081005;
/// The Encrypted Slate of the Squad, at the Cave Entrance (240050000).
const SQUAD_SLATE: i32 = 2083000;
/// The Crystal of Roots, the way out of the cave's depths.
const CRYSTAL_OF_ROOTS: i32 = 2083002;
const DRAGON_ELIXIR: i32 = 2210003;
const DRAGON_SQUAD_CERTIFICATE: i32 = 4001086;
const POWER_ELIXIR: i32 = 2000005;
/// What Keroben asks for ten Power Elixirs.
const KEROBEN_ELIXIRS: (i16, i32) = (10, 100_000);
const HORNTAIL_LEVEL: u8 = 100;
const CAVE_OF_LIFE_ENTRANCE: i32 = 240040700;
pub(crate) const CAVE_ENTRANCE: i32 = 240050000;
pub(crate) const HORNTAIL_CAVE_ENTRANCE: i32 = 240050400;
/// Cave : The Road in Between, from which `hontale_out1` leads back to the
/// Entrance to Horntail's Cave.
pub(crate) const ROAD_IN_BETWEEN: i32 = 240050600;

fn keroben(player: &Player) -> NpcScript {
    let dragon = player
        .buffs
        .active()
        .iter()
        .any(|buff| buff.skill_id == -DRAGON_ELIXIR);
    if !dragon && !holds(player, DRAGON_SQUAD_CERTIFICATE) {
        return NpcScript::new(KEROBEN, vec![Page::Say(text(keys::HORNTAIL_KEROBEN_AWAY))]);
    }
    let (quantity, price) = KEROBEN_ELIXIRS;
    let buy = if player.meso >= price {
        Page::Act {
            text: text(keys::HORNTAIL_KEROBEN_BOUGHT),
            effects: vec![
                Effect::SpendMeso(price),
                Effect::GiveItem {
                    item_id: POWER_ELIXIR,
                    quantity,
                },
            ],
        }
    } else {
        Page::End(text(keys::HORNTAIL_KEROBEN_SHORT))
    };
    let enter = if player.stats.level >= HORNTAIL_LEVEL {
        Page::Done(vec![Effect::WarpToPortal(CAVE_ENTRANCE, 0)])
    } else {
        Page::End(text(keys::HORNTAIL_KEROBEN_LOW_LEVEL))
    };
    let mut script = NpcScript::new(
        KEROBEN,
        vec![Page::Menu {
            text: text(keys::HORNTAIL_KEROBEN_MENU),
            choices: vec![
                (text(keys::HORNTAIL_KEROBEN_BUY_CHOICE), vec![buy]),
                (text(keys::HORNTAIL_KEROBEN_ENTER_CHOICE), vec![enter]),
            ],
        }],
    );
    script.inventory_full = Some(text(keys::HORNTAIL_KEROBEN_FULL));
    script.refused = Some(text(keys::HORNTAIL_KEROBEN_SHORT));
    script
}

/// The slate lets the squad's certificate holders on to the Entrance to
/// Horntail's Cave. (Cosmic may also let in those holding the three
/// bosses' proofs, with solo expeditions on; that is off by default.)
fn squad_slate(player: &Player) -> NpcScript {
    let page = if holds(player, DRAGON_SQUAD_CERTIFICATE) {
        Page::Ask {
            text: text(keys::HORNTAIL_SLATE_ASK),
            on_yes: vec![Effect::WarpToPortal(HORNTAIL_CAVE_ENTRANCE, 0)],
            on_no: None,
        }
    } else {
        Page::Say(text(keys::HORNTAIL_SLATE_SEALED))
    };
    NpcScript::new(SQUAD_SLATE, vec![page])
}

/// The gatekeeper of Horntail's Trial Grounds (Cosmic `2083001`), on the
/// Cave Entrance, on their first map, and on their two last ones.
pub const HTPQ_KEEPER: i32 = 2083001;

/// The gatekeeper at the Cave Entrance: in with the party, or what the
/// Trial Grounds are. (Cosmic's Party Search toggle is left out.)
fn htpq_gatekeeper(entry: Entry) -> NpcScript {
    let enter = match entry {
        Entry::NoParty => Page::Say(text(keys::HTPQ_KEEPER_NO_PARTY)),
        Entry::NotLeader => Page::Say(text(keys::HTPQ_KEEPER_NOT_LEADER)),
        Entry::NotEligible => Page::Say(event_terms(&events::HTPQ, keys::HTPQ_KEEPER_NOT_ELIGIBLE)),
        Entry::Busy => Page::Say(text(keys::HTPQ_KEEPER_BUSY)),
        Entry::Ready => Page::Done(vec![Effect::StartEvent(EventKind::Htpq)]),
    };
    let mut script = NpcScript::new(
        HTPQ_KEEPER,
        vec![Page::Menu {
            text: event_terms(&events::HTPQ, keys::HTPQ_KEEPER_MENU),
            choices: vec![
                (text(keys::HTPQ_KEEPER_ENTER), vec![enter]),
                (
                    text(keys::HTPQ_KEEPER_ABOUT),
                    vec![Page::Say(text(keys::HTPQ_KEEPER_INFO))],
                ),
            ],
        }],
    );
    // Someone left or moved since the gatekeeper checked, or another party
    // went in.
    script.refused = Some(event_terms(&events::HTPQ, keys::HTPQ_KEEPER_NOT_ELIGIBLE));
    script
}

/// The gatekeeper inside the Trial Grounds hears only the leader. On the
/// first map the five Crystal Keys take the team on to the lamp's room;
/// on either last map the Blue Key and six Red Keys win the quest. Either
/// way the keys go at once, without a word.
fn htpq_keeper(player: &Player, leader: bool) -> Option<NpcScript> {
    let map_id = player.stats.map_id;
    let (wanted, done, short): (Vec<(i32, i16)>, _, _) = match map_id {
        events::HTPQ_ENTRY => (
            events::CRYSTAL_KEYS.iter().map(|&key| (key, 1)).collect(),
            EventCommand::MoveTeam(events::HTPQ_LAMP),
            keys::HTPQ_KEEPER_KEYS_SHORT,
        ),
        events::HTPQ_LIGHT | events::HTPQ_DARK => (
            vec![
                (events::BLUE_CRYSTAL_KEY, 1),
                (events::RED_CRYSTAL_KEY, events::RED_KEYS_WANTED),
            ],
            EventCommand::ClearStage(events::HTPQ.last_stage()),
            keys::HTPQ_KEEPER_LAST_KEYS_SHORT,
        ),
        _ => return None,
    };
    let held =
        |&(item_id, quantity): &(i32, i16)| player.inventory.count(item_id) >= i32::from(quantity);
    let page = if !leader {
        Page::Say(text(keys::HTPQ_KEEPER_LEADER_ONLY))
    } else if wanted.iter().all(held) {
        let mut effects: Vec<_> = wanted
            .iter()
            .map(|&(item_id, quantity)| Effect::TakeItem { item_id, quantity })
            .collect();
        effects.push(Effect::EventCommand(done));
        Page::Done(effects)
    } else {
        Page::Say(text(short))
    };
    let mut script = NpcScript::new(HTPQ_KEEPER, vec![page]);
    // A key dropped since the gatekeeper looked.
    script.refused = Some(text(short));
    Some(script)
}

/// The Crystal of Roots asks, then takes those past the Entrance to
/// Horntail's Cave to the Road in Between, and others back out to the
/// Cave of Life's entrance.
fn crystal_of_roots(player: &Player) -> NpcScript {
    let out = if player.stats.map_id > HORNTAIL_CAVE_ENTRANCE {
        Effect::WarpToPortal(ROAD_IN_BETWEEN, 0)
    } else {
        Effect::WarpToPortal(
            CAVE_OF_LIFE_ENTRANCE,
            crate::scripts::named(CAVE_OF_LIFE_ENTRANCE, "out00"),
        )
    };
    NpcScript::new(
        CRYSTAL_OF_ROOTS,
        vec![Page::Ask {
            text: text(keys::HORNTAIL_CRYSTAL_LEAVE),
            on_yes: vec![out],
            on_no: None,
        }],
    )
}

/// A magic rock in Orbis Tower or Eos Tower: one of its scroll takes the
/// player to another rock of the tower, by the `go00` portal beside it in
/// Eos Tower.
struct TowerRock {
    npc: i32,
    scroll: i32,
    to: &'static [(i32, &'static str)],
}

/// From the quests 3038 (Orbis) and 3205 (Eos).
const ORBIS_ROCK_SCROLL: i32 = 4001019;
const EOS_ROCK_SCROLL: i32 = 4001020;
const TOWER_ROCKS: [TowerRock; 6] = [
    // Orbis Magic Spot, at the top of Orbis Tower; El Nath Magic Spot, at
    // its foot.
    TowerRock {
        npc: 2012014,
        scroll: ORBIS_ROCK_SCROLL,
        to: &[(200082100, "sp")],
    },
    TowerRock {
        npc: 2012015,
        scroll: ORBIS_ROCK_SCROLL,
        to: &[(200080200, "sp")],
    },
    // The First to Fourth Eos Rocks, on the 100th, 71st, 41st and 1st
    // floors.
    TowerRock {
        npc: 2040024,
        scroll: EOS_ROCK_SCROLL,
        to: &[(221022900, "go00")],
    },
    TowerRock {
        npc: 2040025,
        scroll: EOS_ROCK_SCROLL,
        to: &[(221024400, "go00"), (221021700, "go00")],
    },
    TowerRock {
        npc: 2040026,
        scroll: EOS_ROCK_SCROLL,
        to: &[(221022900, "go00"), (221020000, "go00")],
    },
    TowerRock {
        npc: 2040027,
        scroll: EOS_ROCK_SCROLL,
        to: &[(221021700, "go00")],
    },
];

/// Nothing without the scroll; with it, a choice of floors where there
/// are two, and a yes or no.
fn tower_rock(rock: &TowerRock, player: &Player) -> NpcScript {
    let item = rock.scroll.to_string();
    let item = ("item", item.as_str());
    if !holds(player, rock.scroll) {
        let none = content::text().get(keys::TOWER_ROCK_NONE, &[item]);
        return NpcScript::new(rock.npc, vec![Page::Say(none)]);
    }
    let go = |&(map, portal): &(i32, &str)| {
        let floor = map.to_string();
        Page::Ask {
            text: content::text().get(keys::TOWER_ROCK_ASK, &[item, ("map", &floor)]),
            on_yes: vec![
                Effect::TakeItem {
                    item_id: rock.scroll,
                    quantity: 1,
                },
                Effect::WarpToPortal(map, crate::scripts::named(map, portal)),
            ],
            on_no: None,
        }
    };
    let page = match rock.to {
        [only] => go(only),
        floors => Page::Menu {
            text: content::text().get(keys::TOWER_ROCK_MENU, &[item]),
            choices: floors
                .iter()
                .map(|floor| (format!("#m{}#", floor.0), vec![go(floor)]))
                .collect(),
        },
    };
    NpcScript::new(rock.npc, vec![page])
}

/// A pile at the end of a Forest of Patience, or the box at the end of a
/// construction site. A player on its quest who lacks the quest's items
/// finds them there; anyone else finds a prize, and in Ellinia's forest
/// everyone does. Then it sends the player out, with no words, or says
/// `full` if there is no room.
struct PatiencePile {
    npc: i32,
    quest: i16,
    item: i32,
    count: i16,
    always_prize: bool,
    /// Item, quantity and weight. Clothes for the other gender are left
    /// out.
    prizes: &'static [(i32, i16, u32)],
    out: i32,
    full: &'static str,
}

/// The piles, from Cosmic's scripts (ids, quantities, weights).
const PATIENCE_PILES: [PatiencePile; 8] = [
    // The pile of flowers and the pile of herbs in Ellinia's forest, for
    // Sabitrama's quests 2050 and 2051.
    PatiencePile {
        npc: 1043000,
        quest: 2050,
        item: 4031020,
        count: 1,
        always_prize: true,
        prizes: &[
            (1040052, 1, 10),
            (1040054, 1, 10),
            (1040130, 1, 10),
            (1041143, 1, 15),
            (1042013, 1, 10),
            (1042022, 1, 10),
            (1042034, 1, 10),
            (1042084, 1, 10),
            (1042098, 1, 10),
            (1042117, 1, 10),
            (1702002, 1, 5),
            (1702015, 1, 5),
        ],
        out: ELLINIA,
        full: keys::PATIENCE_FULL,
    },
    PatiencePile {
        npc: 1043001,
        quest: 2051,
        item: 4031032,
        count: 1,
        always_prize: true,
        prizes: &[
            (1060041, 1, 10),
            (1060048, 1, 10),
            (1060116, 1, 10),
            (1061113, 1, 10),
            (1061130, 1, 10),
            (1061139, 1, 10),
            (1062009, 1, 10),
            (1062017, 1, 10),
            (1062024, 1, 10),
            (1062056, 1, 10),
            (1062061, 1, 10),
            (1702045, 1, 5),
            (1702114, 1, 5),
        ],
        out: ELLINIA,
        full: keys::PATIENCE_FULL,
    },
    // The pink, blue and white flowers at the ends of the Deep Forest's
    // steps, for John's quests 2052 to 2054.
    PatiencePile {
        npc: 1063000,
        quest: 2052,
        item: 4031025,
        count: 10,
        always_prize: false,
        prizes: &[
            (4010000, 3, 1),
            (4010001, 3, 1),
            (4010002, 3, 1),
            (4010003, 3, 1),
            (4010004, 3, 1),
            (4010005, 3, 1),
        ],
        out: STATUE_FOREST,
        full: keys::PATIENCE_FULL,
    },
    PatiencePile {
        npc: 1063001,
        quest: 2053,
        item: 4031026,
        count: 20,
        always_prize: false,
        prizes: &[(4020000, 4, 1), (4020002, 4, 1), (4020006, 4, 1)],
        out: STATUE_FOREST,
        full: keys::PATIENCE_FULL,
    },
    PatiencePile {
        npc: 1063002,
        quest: 2054,
        item: 4031028,
        count: 30,
        always_prize: false,
        prizes: &[(4010006, 4, 1), (4010007, 4, 1), (4020007, 4, 1)],
        out: STATUE_FOREST,
        full: keys::PATIENCE_FULL,
    },
    // The boxes at the ends of Line 3's construction sites B1 to B3, for
    // Shumi's quests 2055 to 2057.
    PatiencePile {
        npc: 1052008,
        quest: 2055,
        item: 4031039,
        count: 1,
        always_prize: false,
        prizes: &[
            (4020000, 1, 1),
            (4020001, 1, 1),
            (4020002, 1, 1),
            (4020003, 1, 1),
            (4020004, 1, 1),
        ],
        out: SUBWAY_TICKETING,
        full: keys::LINE3_CHEST_FULL,
    },
    PatiencePile {
        npc: 1052009,
        quest: 2056,
        item: 4031040,
        count: 1,
        always_prize: false,
        prizes: &[
            (4020005, 1, 1),
            (4020006, 1, 1),
            (4020007, 1, 1),
            (4020008, 1, 1),
            (4010000, 1, 1),
        ],
        out: SUBWAY_TICKETING,
        full: keys::LINE3_CHEST_FULL,
    },
    PatiencePile {
        npc: 1052010,
        quest: 2057,
        item: 4031041,
        count: 1,
        always_prize: false,
        prizes: &[
            (4010001, 1, 1),
            (4010002, 1, 1),
            (4010003, 1, 1),
            (4010004, 1, 1),
            (4010005, 1, 1),
            (4010006, 1, 1),
            (4010007, 1, 1),
        ],
        out: SUBWAY_TICKETING,
        full: keys::LINE3_CHEST_FULL,
    },
];

fn patience_pile(pile: &PatiencePile, player: &Player, rng: &mut impl Rng) -> NpcScript {
    let on_quest = player.quests.state(pile.quest) == QuestState::Started
        && player.inventory.count(pile.item) < i32::from(pile.count);
    let mut effects = Vec::new();
    if on_quest {
        effects.push(Effect::GiveItem {
            item_id: pile.item,
            quantity: pile.count,
        });
    }
    if pile.always_prize || !on_quest {
        // An equip's thousands digit is its gender: 0, 1, or 2 for both.
        let fits = |item: i32| {
            let gender = item / 1000 % 10;
            item / 1_000_000 != 1 || gender == 2 || gender == i32::from(player.stats.gender)
        };
        let prizes: Vec<_> = pile.prizes.iter().filter(|prize| fits(prize.0)).collect();
        let total: u32 = prizes.iter().map(|prize| prize.2).sum();
        let mut pick = rng.gen_range(0, total.max(1));
        for &&(item_id, quantity, weight) in &prizes {
            if pick < weight {
                effects.push(Effect::GiveItem { item_id, quantity });
                break;
            }
            pick -= weight;
        }
    }
    effects.push(Effect::WarpToPortal(pile.out, 0));
    let mut script = NpcScript::new(pile.npc, vec![Page::Done(effects)]);
    script.inventory_full = Some(text(pile.full));
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

/// Lord Jonathan lets a Corsair with "Jonathan's Court" started into his
/// test, one at a time, until the real Bart is found; then the quest
/// completes with him. Cosmic asks a riddle first, whose text no source
/// gives (QUESTIONS a118).
fn lord_jonathan(player: &Player) -> NpcScript {
    use crate::reactors::{BART_FOUND, BART_RECORD, BART_ROOM, JONATHANS_COURT};
    let testing = player.quests.state(JONATHANS_COURT) == QuestState::Started
        && player.quests.info(BART_RECORD) != Some(BART_FOUND);
    if !testing {
        return NpcScript::new(LORD_JONATHAN, vec![Page::Say(text(keys::JONATHAN_HELLO))]);
    }
    let mut script = NpcScript::new(
        LORD_JONATHAN,
        vec![Page::Ask {
            text: text(keys::JONATHAN_TEST_ASK),
            on_yes: vec![
                Effect::RequireEmptyMap(BART_ROOM),
                Effect::WarpToPortal(BART_ROOM, 0),
            ],
            on_no: None,
        }],
    );
    script.refused = Some(text(keys::JONATHAN_TEST_BUSY));
    script
}

/// The Insignificant Being takes a Bowmaster or Marksman of level 120 on
/// 6107 or 6108 straight into the Junior Balrog's other world, one at a
/// time, where 6108 counts the kills.
fn insignificant_being(player: &Player) -> NpcScript {
    use crate::scripts::BALROG_WORLD;
    let say = |key| NpcScript::new(INSIGNIFICANT_BEING, vec![Page::Say(text(key))]);
    let on_quest = BALROG_WORLD_QUESTS
        .iter()
        .any(|&quest| player.quests.state(quest) == QuestState::Started);
    if !on_quest {
        return say(keys::BALROG_WORLD_NOT_ALLOWED);
    }
    if !BALROG_WORLD_JOBS.contains(&player.stats.job) {
        return say(keys::BALROG_WORLD_JOB);
    }
    if player.stats.level < 120 {
        return say(keys::BALROG_WORLD_LEVEL);
    }
    let mut script = NpcScript::new(
        INSIGNIFICANT_BEING,
        vec![Page::Done(vec![
            Effect::RequireEmptyMap(BALROG_WORLD),
            Effect::WarpToPortal(BALROG_WORLD, 0),
        ])],
    );
    script.refused = Some(text(keys::BALROG_WORLD_BUSY));
    script
}

/// `npc` asks `key`, and a yes takes the player to `to`'s first spawn point
/// (Cosmic `1092008`, `1095000`, and `2095000` past Save Delli!).
/// 2042002 in a Monster Carnival result room (Cosmic `2042002`): the
/// battle just fought, graded by the player's side's CP in all, and its
/// EXP on the way back to the Carnival Field. With no battle to grade, as
/// after a reconnect, the player just goes back.
fn carnival_grade(player: &Player) -> NpcScript {
    let back = Effect::WarpToPortal(carnival::CARNIVAL_LOBBY, 0);
    let Some((won, points)) = player.carnival_result else {
        return NpcScript::new(CARNIVAL_GUIDE, vec![Page::Done(vec![back])]);
    };
    let grade = carnival::grade(points);
    let key = if won {
        keys::CARNIVAL_GUIDE_WON
    } else {
        keys::CARNIVAL_GUIDE_LOST
    };
    let letter = carnival::grade_letter(grade).to_string();
    let said = content::text().get(key, &[("grade", &letter)]);
    let mut script = NpcScript::new(CARNIVAL_GUIDE, vec![Page::Say(said)]);
    script
        .on_end
        .extend([Effect::GainExp(carnival::result_exp(won, grade)), back]);
    script
}

fn ask_warp(npc: i32, key: &str, to: i32) -> NpcScript {
    NpcScript::new(
        npc,
        vec![Page::Ask {
            text: text(key),
            on_yes: vec![Effect::WarpToPortal(to, 0)],
            on_no: None,
        }],
    )
}

/// Delli (Cosmic `2095000`) where the party gathers: the leader takes the
/// party in to protect Willi, and anyone may hear what they must do.
fn delli(entry: Entry) -> NpcScript {
    let enter = match entry {
        Entry::NoParty => Page::Say(text(keys::DELLI_NO_PARTY)),
        Entry::NotLeader => Page::Say(text(keys::DELLI_NOT_LEADER)),
        Entry::NotEligible => Page::Say(event_terms(&events::DELLI, keys::DELLI_NOT_ELIGIBLE)),
        Entry::Busy => Page::Say(text(keys::DELLI_BUSY)),
        Entry::Ready => Page::Done(vec![Effect::StartEvent(EventKind::Delli)]),
    };
    let mut script = NpcScript::new(
        DELLI,
        vec![Page::Menu {
            text: text(keys::DELLI_MENU),
            choices: vec![
                (text(keys::DELLI_ENTER), vec![enter]),
                (
                    text(keys::DELLI_ABOUT),
                    vec![Page::Say(text(keys::DELLI_INFO))],
                ),
            ],
        }],
    );
    // Someone left or moved since Delli checked, or another party went in.
    script.refused = Some(event_terms(&events::DELLI, keys::DELLI_NOT_ELIGIBLE));
    script
}

/// Tylus (Cosmic `2020008`, while "Protecting Tylus" is started) in the
/// Chief's Residence: the leader takes the party in to protect him, and
/// anyone may hear what they must do.
fn tylus(entry: Entry) -> NpcScript {
    let enter = match entry {
        Entry::NoParty => Page::Say(text(keys::TYLUS_NO_PARTY)),
        Entry::NotLeader => Page::Say(text(keys::TYLUS_NOT_LEADER)),
        Entry::NotEligible => Page::Say(event_terms(&events::TYLUS, keys::TYLUS_NOT_ELIGIBLE)),
        Entry::Busy => Page::Say(text(keys::TYLUS_BUSY)),
        Entry::Ready => Page::Done(vec![Effect::StartEvent(EventKind::Tylus)]),
    };
    let mut script = NpcScript::new(
        TYLUS,
        vec![Page::Menu {
            text: text(keys::TYLUS_MENU),
            choices: vec![
                (text(keys::TYLUS_ENTER), vec![enter]),
                (
                    text(keys::TYLUS_ABOUT),
                    vec![Page::Say(text(keys::TYLUS_INFO))],
                ),
            ],
        }],
    );
    // Someone left or moved since Tylus checked, or another party went in.
    script.refused = Some(event_terms(&events::TYLUS, keys::TYLUS_NOT_ELIGIBLE));
    script
}

/// Ellin (Cosmic `2133000`) in the Deep Fairy Forest: the leader takes the
/// party into the Forest of Poison Haze, anyone may hear what it holds,
/// and `player` may trade Altaire Fragments for earrings.
fn ellin(entry: Entry, player: &Player) -> NpcScript {
    let enter = match entry {
        Entry::NoParty => Page::Say(text(keys::ELLIN_NO_PARTY)),
        Entry::NotLeader => Page::Say(text(keys::ELLIN_NOT_LEADER)),
        Entry::NotEligible => Page::Say(event_terms(&events::ELLIN, keys::ELLIN_NOT_ELIGIBLE)),
        Entry::Busy => Page::Say(text(keys::ELLIN_BUSY)),
        Entry::Ready => Page::Done(vec![Effect::StartEvent(EventKind::Ellin)]),
    };
    let mut script = NpcScript::new(
        ELLIN,
        vec![Page::Menu {
            text: text(keys::ELLIN_MENU),
            choices: vec![
                (text(keys::ELLIN_ENTER), vec![enter]),
                (
                    text(keys::ELLIN_ABOUT),
                    vec![Page::Say(text(keys::ELLIN_INFO))],
                ),
                (text(keys::ELLIN_PRIZE), vec![ellin_prizes(player)]),
            ],
        }],
    );
    // Someone left or moved since Ellin checked, or another party went in.
    script.refused = Some(event_terms(&events::ELLIN, keys::ELLIN_NOT_ELIGIBLE));
    script.inventory_full = Some(text(keys::ELLIN_PRIZE_NO_ROOM));
    script
}

/// What Romeo or Juliet says that differs between them.
struct MagatiaKeys {
    menu: &'static str,
    info: &'static str,
    retreat: &'static str,
}

fn magatia_keys(side: MagatiaSide) -> MagatiaKeys {
    match side {
        MagatiaSide::Zenumist => MagatiaKeys {
            menu: keys::MAGATIA_ROMEO_MENU,
            info: keys::MAGATIA_ROMEO_INFO,
            retreat: keys::MAGATIA_ROMEO_RETREAT,
        },
        MagatiaSide::Alcadno => MagatiaKeys {
            menu: keys::MAGATIA_JULIET_MENU,
            info: keys::MAGATIA_JULIET_INFO,
            retreat: keys::MAGATIA_JULIET_RETREAT,
        },
    }
}

/// Who starts `side`'s party quest: Romeo or Juliet.
pub fn magatia_host(side: MagatiaSide) -> i32 {
    match side {
        MagatiaSide::Zenumist => ROMEO,
        MagatiaSide::Alcadno => JULIET,
    }
}

/// The side whose library `npc` is one of the books of, if it is one.
pub fn magatia_book_side(npc: i32) -> Option<MagatiaSide> {
    match npc {
        ZENUMIST_BOOK => Some(MagatiaSide::Zenumist),
        ALCADNO_BOOK => Some(MagatiaSide::Alcadno),
        _ => None,
    }
}

/// A Magatia library's book (Cosmic `2112007`, `2112013`), which says
/// what it held as the player takes it. The switch clears the stage and
/// shows the library's `d00` lit for everyone there.
fn magatia_book(npc_id: i32, find: Option<Find>) -> NpcScript {
    let side = magatia_book_side(npc_id).unwrap_or(MagatiaSide::Zenumist);
    let act = |key: &str, args: &[(&str, &str)], effects| Page::Act {
        text: content::text().get(key, args),
        effects,
    };
    let page = match find {
        None => Page::Say(text(keys::MAGATIA_BOOK_NOTHING)),
        Some(Find::Mesos) => act(
            keys::MAGATIA_BOOK_MESOS,
            &[("mesos", &events::MAGATIA_BOOK_MESOS.to_string())],
            vec![Effect::GainMeso(events::MAGATIA_BOOK_MESOS)],
        ),
        Some(Find::Exp) => act(
            keys::MAGATIA_BOOK_EXP,
            &[("exp", &events::MAGATIA_BOOK_EXP.to_string())],
            vec![Effect::GainExp(events::MAGATIA_BOOK_EXP)],
        ),
        Some(Find::Letter) => act(
            keys::MAGATIA_BOOK_LETTER,
            &[],
            vec![Effect::GiveItem {
                item_id: side.letter(),
                quantity: 1,
            }],
        ),
        Some(Find::Pass) => act(
            keys::MAGATIA_BOOK_PASS,
            &[],
            vec![
                Effect::SetReactorState {
                    name: MAGATIA_SWITCH,
                    state: 1,
                },
                Effect::EventCommand(EventCommand::ClearStage(1)),
            ],
        ),
    };
    let mut script = NpcScript::new(npc_id, vec![page]);
    script.inventory_full = Some(text(keys::MAGATIA_BOOK_LETTER_FULL));
    script
}

/// Yulete in `side`'s office.
pub fn yulete(side: MagatiaSide) -> i32 {
    match side {
        MagatiaSide::Zenumist => YULETE,
        MagatiaSide::Alcadno => YULETE_ALCADNO,
    }
}

/// The hub's Romeo or Juliet on `side`.
pub fn magatia_hub_host(side: MagatiaSide) -> i32 {
    match side {
        MagatiaSide::Zenumist => ROMEO_HUB,
        MagatiaSide::Alcadno => JULIET_HUB,
    }
}

/// Who waits in `map_id` from the first arrival in a run's copy of it,
/// and where (Cosmic `MagatiaPQ_Z.afterSetup`'s `spawnNpc`): Yulete in the
/// boss's room, his traces in theirs, and the one who thanks the party in
/// the last room.
pub fn magatia_waiting(map_id: i32) -> Option<(i32, (i16, i16))> {
    let side = MagatiaSide::of(map_id)?;
    if map_id == side.stages()[6] {
        Some((yulete(side), events::LAB_YULETE_AT))
    } else if map_id == side.traces_room() {
        Some((traces(side), events::MAGATIA_THANKS_AT))
    } else if map_id == side.thanks_room() {
        Some((MAGATIA_THANKS, events::MAGATIA_THANKS_AT))
    } else {
        None
    }
}

/// Yulete's traces on `side`.
pub fn traces(side: MagatiaSide) -> i32 {
    match side {
        MagatiaSide::Zenumist => YULETE_TRACES,
        MagatiaSide::Alcadno => YULETE_TRACES_ALCADNO,
    }
}

/// Romeo (Cosmic `2112006`) or Juliet (`2112005`) in the boss's room once
/// it is won. If the escort fainted they thank the party and send each
/// member on to the last room; otherwise they give each the side's gift
/// (`MagatiaSide::gift`), which needs room, and send them on: to the last
/// room if Yulete was not caught, so the Frankenroid was the plain one,
/// or else past his traces first. Cosmic's Alcadno script never sends
/// anyone straight on; both sides do as its Zenumist script does.
fn magatia_saved(side: MagatiaSide, escort_failed: bool, caught: bool) -> NpcScript {
    let npc = magatia_hub_host(side);
    let zenumist = side == MagatiaSide::Zenumist;
    if escort_failed {
        let key = match zenumist {
            true => keys::MAGATIA_ROMEO_SAVED_HURT,
            false => keys::MAGATIA_JULIET_SAVED_HURT,
        };
        return NpcScript::new(
            npc,
            vec![
                Page::Say(text(key)),
                Page::Done(vec![Effect::WarpToPortal(side.thanks_room(), 0)]),
            ],
        );
    }
    let key = match zenumist {
        true => keys::MAGATIA_ROMEO_SAVED,
        false => keys::MAGATIA_JULIET_SAVED,
    };
    let next = match caught {
        false => side.thanks_room(),
        true => side.traces_room(),
    };
    let mut script = NpcScript::new(
        npc,
        vec![
            Page::Say(text(key)),
            Page::Say(text(keys::MAGATIA_GIFT)),
            Page::Done(vec![
                Effect::GiveItem {
                    item_id: side.gift(),
                    quantity: 1,
                },
                Effect::WarpToPortal(next, 0),
            ]),
        ],
    );
    script.inventory_full = Some(text(keys::MAGATIA_GIFT_FULL));
    script
}

/// Yulete's traces (Cosmic `2112001`, `2112011`): beaten, he is cheered
/// up, says he is sorry, and sends the member on to the last room, the
/// way into his lab open to them from now on (`YULETE_LAB_OPENED`).
fn yulete_traces(side: MagatiaSide) -> NpcScript {
    let on = vec![
        Page::Say(text(keys::MAGATIA_TRACES_SORRY)),
        Page::Done(vec![
            Effect::Mark(YULETE_LAB_OPENED),
            Effect::WarpToPortal(side.thanks_room(), 0),
        ]),
    ];
    NpcScript::new(
        traces(side),
        vec![Page::Menu {
            text: text(keys::MAGATIA_TRACES_DEFEATED),
            choices: vec![(text(keys::MAGATIA_TRACES_CHEER), on)],
        }],
    )
}

/// Set to "1" once the player has seen a Romeo and Juliet run through:
/// "Yulete's Request" (3365) and "Romeo's Present" (3369) wait for it.
pub const MAGATIA_WON: i16 = 7072;

/// The one who thanks the party in the last room (Cosmic `2112018`,
/// `giveEventReward`): their thanks, by whether the escort fainted, then
/// one of `MAGATIA_REWARDS`, the run marked won (`MAGATIA_WON`) and the
/// way home to the side's lobby. A
/// member without a free slot in each kind of inventory the rewards come
/// in gets nothing yet, and stays.
fn magatia_thanks(
    side: MagatiaSide,
    player: &Player,
    escort_failed: bool,
    rng: &mut impl Rng,
) -> NpcScript {
    let key = match escort_failed {
        true => keys::MAGATIA_THANKS_JAIL,
        false => keys::MAGATIA_THANKS_REHAB,
    };
    let lobby = EventKind::Magatia(side).event().lobby;
    let room = events::MAGATIA_REWARDS.iter().all(|&(item_id, _)| {
        InventoryType::for_item(item_id).is_none_or(|kind| {
            player.inventory.items(kind).count() < usize::from(player.inventory.slot_limit(kind))
        })
    });
    let last = match events::MAGATIA_REWARDS.choose(rng).filter(|_| room) {
        Some(&(item_id, quantity)) => Page::Done(vec![
            Effect::SetRecord {
                record: MAGATIA_WON,
                value: "1",
            },
            Effect::GiveItem { item_id, quantity },
            Effect::Warp(lobby),
        ]),
        None => Page::Say(text(keys::MAGATIA_THANKS_FULL)),
    };
    let mut script = NpcScript::new(MAGATIA_THANKS, vec![Page::Say(text(key)), last]);
    script.inventory_full = Some(text(keys::MAGATIA_THANKS_FULL));
    script
}

/// Romeo (Cosmic `2112006`) or Juliet (`2112005`) in their side's hub of
/// files: the side's letter from the library shocks them, which everyone
/// hears; both files, from the leader, clear the stage, send the hub's
/// monsters away and open its way on, which they open again once it shuts.
/// Without either, they ask whether the member would rather retreat.
fn magatia_hub(
    side: MagatiaSide,
    player: &Player,
    leader: bool,
    cleared: bool,
    door_shut: bool,
) -> NpcScript {
    let npc = magatia_hub_host(side);
    let event = EventKind::Magatia(side).event();
    let zenumist = side == MagatiaSide::Zenumist;
    let say = |key: &str| NpcScript::new(npc, vec![Page::Say(text(key))]);
    let act = |key: &str, effects: Vec<Effect>| {
        NpcScript::new(
            npc,
            vec![Page::Act {
                text: text(key),
                effects,
            }],
        )
    };
    let has = |item_id: i32| player.inventory.count(item_id) > 0;
    if has(side.letter()) {
        return act(
            keys::MAGATIA_LETTER,
            vec![
                Effect::TakeItem {
                    item_id: side.letter(),
                    quantity: 1,
                },
                Effect::EventCommand(EventCommand::Shocked),
            ],
        );
    }
    if cleared {
        return match door_shut {
            true => act(
                keys::MAGATIA_OPEN_DOOR,
                vec![Effect::OpenReactor(side.hub_door())],
            ),
            false if zenumist => say(keys::MAGATIA_ROMEO_HURRY),
            false => say(keys::MAGATIA_JULIET_HURRY),
        };
    }
    if events::MAGATIA_FILES.into_iter().all(has) {
        if !leader {
            return say(keys::MAGATIA_FILES_LEADER);
        }
        let mut effects: Vec<Effect> = (events::MAGATIA_FILES.into_iter())
            .map(|item_id| Effect::TakeItem {
                item_id,
                quantity: 1,
            })
            .collect();
        effects.extend([
            Effect::EventCommand(EventCommand::ClearStage(events::MAGATIA_FILES_STAGE)),
            Effect::ClearMobs,
            Effect::OpenReactor(side.hub_door()),
        ]);
        return act(keys::MAGATIA_FILES, effects);
    }
    ask_warp(npc, magatia_keys(side).retreat, event.exit)
}

/// The reactor on each library that the switch lights (Map.nx `d00`,
/// Reactor.nx 2618005).
const MAGATIA_SWITCH: &str = "d00";

/// The side whose party quest `npc` (Romeo or Juliet) starts.
fn magatia_side(npc: i32) -> MagatiaSide {
    if npc == JULIET {
        MagatiaSide::Alcadno
    } else {
        MagatiaSide::Zenumist
    }
}

/// Romeo (Cosmic `2112004`) or Juliet (`2112003`) in their room: the
/// leader takes the party into their side's party quest, and anyone may
/// hear about it. Cosmic's Party Search toggle has no counterpart here, as
/// at Guon.
fn magatia(side: MagatiaSide, entry: Entry) -> NpcScript {
    let kind = EventKind::Magatia(side);
    let event = kind.event();
    let enter = match entry {
        Entry::NoParty => Page::Say(text(keys::MAGATIA_NO_PARTY)),
        Entry::NotLeader => Page::Say(text(keys::MAGATIA_NOT_LEADER)),
        Entry::NotEligible => Page::Say(event_terms(event, keys::MAGATIA_NOT_ELIGIBLE)),
        Entry::Busy => Page::Say(text(keys::MAGATIA_BUSY)),
        Entry::Ready => Page::Done(vec![Effect::StartEvent(kind)]),
    };
    let said = magatia_keys(side);
    let mut script = NpcScript::new(
        magatia_host(side),
        vec![Page::Menu {
            text: text(said.menu),
            choices: vec![
                (text(keys::MAGATIA_ENTER), vec![enter]),
                (text(keys::MAGATIA_ABOUT), vec![Page::Say(text(said.info))]),
            ],
        }],
    );
    // Someone left or moved since they checked, or another party went in.
    script.refused = Some(event_terms(event, keys::MAGATIA_NOT_ELIGIBLE));
    script
}

/// Guon (Cosmic `2094000`) Over the Pirate Ship: the leader takes the
/// party aboard, and anyone may hear what waits there. Cosmic's Party
/// Search toggle has no counterpart here (`v83ref/lanes/QUESTIONS.md`
/// a128 6).
fn guon(entry: Entry) -> NpcScript {
    let enter = match entry {
        Entry::NoParty => Page::Say(text(keys::PIRATE_NO_PARTY)),
        Entry::NotLeader => Page::Say(text(keys::PIRATE_NOT_LEADER)),
        Entry::NotEligible => Page::Say(event_terms(&events::PIRATE, keys::PIRATE_NOT_ELIGIBLE)),
        Entry::Busy => Page::Say(text(keys::PIRATE_BUSY)),
        Entry::Ready => Page::Done(vec![Effect::StartEvent(EventKind::Pirate)]),
    };
    let mut script = NpcScript::new(
        GUON,
        vec![Page::Menu {
            text: text(keys::PIRATE_MENU),
            choices: vec![
                (text(keys::PIRATE_ENTER), vec![enter]),
                (
                    text(keys::PIRATE_ABOUT),
                    vec![Page::Say(text(keys::PIRATE_INFO))],
                ),
            ],
        }],
    );
    // Someone left or moved since Guon checked, or another party went in.
    script.refused = Some(event_terms(&events::PIRATE, keys::PIRATE_NOT_ELIGIBLE));
    script
}

/// Duarte (Cosmic `2103013`'s commented-out script) in Nett's Pyramid's
/// lobby: anyone may hear about the Pyramid and the treasures in Pharaoh
/// Yeti's Tomb, and a player of level 40 or more may go in alone or as the
/// leader of a party, in a mode of their choice, Hell's alone above level
/// 60, or into the Tomb with a gem, or have the medal. Cosmic's Find a
/// Party is left out.
fn duarte(entries: &[(EventKind, Entry)], player: &Player) -> NpcScript {
    let level = player.stats.level;
    let run = |kind: EventKind| {
        let event = kind.event();
        let entry = entries
            .iter()
            .find(|(of, _)| *of == kind)
            .map_or(Entry::Busy, |(_, entry)| *entry);
        let page = match entry {
            _ if level < *events::PYRAMID_LEVELS.start() => {
                event_terms(event, keys::PYRAMID_DUARTE_LOW_LEVEL)
            }
            _ if !event.levels.contains(&level) => {
                event_terms(&events::PYRAMID[0], keys::PYRAMID_DUARTE_ONLY_HELL)
            }
            Entry::Ready => return Page::Done(vec![Effect::StartEvent(kind)]),
            Entry::Busy => text(keys::PYRAMID_DUARTE_BUSY),
            Entry::NoParty => text(keys::PYRAMID_DUARTE_NO_PARTY),
            Entry::NotLeader => text(keys::PYRAMID_DUARTE_NOT_LEADER),
            Entry::NotEligible => event_terms(event, keys::PYRAMID_DUARTE_PARTY_TERMS),
        };
        Page::Say(page)
    };
    let modes = |kind: fn(PyramidMode) -> EventKind| Page::Menu {
        text: text(keys::PYRAMID_DUARTE_MODE),
        choices: PyramidMode::ALL
            .iter()
            .map(|&mode| {
                let icon = format!("#i{}#", events::PYRAMID_MODE_ICON + mode.index());
                (icon, vec![run(kind(mode))])
            })
            .collect(),
    };
    let enter = Page::Menu {
        text: text(keys::PYRAMID_DUARTE_HOW),
        choices: vec![
            (
                text(keys::PYRAMID_DUARTE_ALONE),
                vec![modes(EventKind::Pyramid)],
            ),
            (
                text(keys::PYRAMID_DUARTE_PARTY),
                vec![modes(EventKind::PyramidParty)],
            ),
        ],
    };
    let mut script = NpcScript::new(
        DUARTE,
        vec![Page::Menu {
            text: text(keys::PYRAMID_DUARTE_MENU),
            choices: vec![
                (
                    text(keys::PYRAMID_DUARTE_ABOUT),
                    vec![
                        Page::Say(text(keys::PYRAMID_DUARTE_INFO)),
                        Page::Say(text(keys::PYRAMID_DUARTE_RULES)),
                        Page::Say(text(keys::PYRAMID_DUARTE_LUCK)),
                    ],
                ),
                (text(keys::PYRAMID_DUARTE_ENTER), vec![enter]),
                (text(keys::PYRAMID_DUARTE_TOMB), vec![duarte_tomb(player)]),
                (
                    text(keys::PYRAMID_DUARTE_TREASURES),
                    vec![Page::Say(text(keys::PYRAMID_DUARTE_TREASURES_INFO))],
                ),
                (text(keys::PYRAMID_DUARTE_MEDAL), vec![duarte_medal(player)]),
            ],
        }],
    );
    // Someone left or moved since Duarte checked, or the last room went.
    script.refused = Some(text(keys::PYRAMID_DUARTE_REFUSED));
    script
}

/// Spiegelmann on the Carnival Field (Cosmic `2042000`): what the Monster
/// Carnival is, and for a party's leader of the right level, its rooms,
/// each with how many it takes. Picking one the party may take sends it
/// in. Cosmic's Carnival 2 and Maple Coin trades aren't in.
fn spiegelmann(
    entries: &[(EventKind, Entry)],
    rooms: &[carnival::Room],
    player: &Player,
) -> NpcScript {
    let entry = |kind: EventKind| {
        entries
            .iter()
            .find(|(of, _)| *of == kind)
            .map_or(Entry::Busy, |(_, entry)| *entry)
    };
    let room = |room: u8| {
        let kind = EventKind::Carnival(room);
        // A room in use may be waiting for a challenge: the directory
        // decides when the leader picks it.
        let page = match entry(kind) {
            Entry::Ready | Entry::Busy => Page::Done(vec![Effect::CarnivalRoom(room)]),
            Entry::NoParty => Page::Say(text(keys::CARNIVAL_SPIEGELMANN_NO_PARTY)),
            Entry::NotLeader => Page::Say(text(keys::CARNIVAL_SPIEGELMANN_NOT_LEADER)),
            Entry::NotEligible => {
                Page::Say(event_terms(kind.event(), keys::CARNIVAL_SPIEGELMANN_TERMS))
            }
        };
        let number = room.to_string();
        let view = rooms.get(usize::from(room) - 1);
        let label = match view.copied().unwrap_or(carnival::Room::Free) {
            carnival::Room::Free => content::text().get(
                keys::CARNIVAL_SPIEGELMANN_ROOM,
                &[
                    ("room", &number),
                    ("least", &kind.event().least().to_string()),
                    ("most", &kind.event().team.end().to_string()),
                ],
            ),
            carnival::Room::Waiting { size, level } => content::text().get(
                keys::CARNIVAL_SPIEGELMANN_WAITING,
                &[
                    ("room", &number),
                    ("size", &size.to_string()),
                    ("level", &level.to_string()),
                ],
            ),
            carnival::Room::Battle => {
                content::text().get(keys::CARNIVAL_SPIEGELMANN_TAKEN, &[("room", &number)])
            }
        };
        (label, vec![page])
    };
    let first = entry(EventKind::Carnival(1));
    let enter = match first {
        _ if !carnival::CARNIVAL_LEVELS.contains(&player.stats.level) => Page::Say(event_terms(
            &events::CARNIVAL[0],
            keys::CARNIVAL_SPIEGELMANN_LEVELS,
        )),
        Entry::NoParty => Page::Say(text(keys::CARNIVAL_SPIEGELMANN_NO_PARTY)),
        Entry::NotLeader => Page::Say(text(keys::CARNIVAL_SPIEGELMANN_NOT_LEADER)),
        _ => Page::Menu {
            text: text(keys::CARNIVAL_SPIEGELMANN_ROOMS),
            choices: (1..=carnival::CARNIVAL_ROOMS).map(room).collect(),
        },
    };
    let mut script = NpcScript::new(
        SPIEGELMANN,
        vec![Page::Menu {
            text: text(keys::CARNIVAL_SPIEGELMANN_MENU),
            choices: vec![
                (text(keys::CARNIVAL_SPIEGELMANN_ENTER), vec![enter]),
                (
                    text(keys::CARNIVAL_SPIEGELMANN_ABOUT),
                    vec![
                        Page::Say(text(keys::CARNIVAL_SPIEGELMANN_INFO)),
                        Page::Say(text(keys::CARNIVAL_SPIEGELMANN_RULES)),
                    ],
                ),
            ],
        }],
    );
    // Someone left or moved since he checked, or the room was taken.
    script.refused = Some(text(keys::CARNIVAL_SPIEGELMANN_BUSY));
    script
}

/// Spiegelmann in town (Cosmic `2042000`): he takes a player of levels 30
/// to 50 to the Carnival Field, keeping where they were for its way out.
fn spiegelmann_away(player: &Player) -> NpcScript {
    let page = if carnival::CARNIVAL_LEVELS.contains(&player.stats.level) {
        Page::Ask {
            text: text(keys::CARNIVAL_SPIEGELMANN_GO),
            on_yes: vec![
                Effect::SaveLocation(CARNIVAL_RETURN, Some(player.stats.map_id)),
                Effect::WarpToPortal(carnival::CARNIVAL_LOBBY, 0),
            ],
            on_no: None,
        }
    } else {
        Page::Say(event_terms(
            &events::CARNIVAL[0],
            keys::CARNIVAL_SPIEGELMANN_LEVELS,
        ))
    };
    NpcScript::new(SPIEGELMANN, vec![page])
}

/// Duarte's way into Pharaoh Yeti's Tomb (Cosmic's menu item 3, unfinished
/// there): the player picks a mode's gem, which he takes for a room of that
/// mode's Tomb, from level 40.
fn duarte_tomb(player: &Player) -> Page {
    let choices = PyramidMode::ALL
        .iter()
        .map(|&mode| {
            let gem = events::PYRAMID_GEM + mode.index();
            let kind = EventKind::PyramidTomb(mode);
            let page = if player.stats.level < *events::PYRAMID_LEVELS.start() {
                Page::Say(event_terms(kind.event(), keys::PYRAMID_DUARTE_LOW_LEVEL))
            } else if player.inventory.count(gem) == 0 {
                Page::Say(text(keys::PYRAMID_DUARTE_TOMB_NO_GEM))
            } else {
                let take = Effect::TakeItem {
                    item_id: gem,
                    quantity: 1,
                };
                Page::Done(vec![take, Effect::StartEvent(kind)])
            };
            (format!("#i{gem}# #t{gem}#"), vec![page])
        })
        .collect();
    Page::Menu {
        text: text(keys::PYRAMID_DUARTE_TOMB_GEM),
        choices,
    }
}

/// Duarte's Protector of Pharaoh medal, for 50,000 kills in the Pyramid
/// (Cosmic's menu item 5, a stub there): one each, which completes quest
/// 29932 if it was started. A medal lost may be had again.
fn duarte_medal(player: &Player) -> Page {
    use crate::pyramid::{protector_kills, PROTECTOR_MEDAL, PROTECTOR_NEEDED, PROTECTOR_QUEST};
    let kills = protector_kills(player);
    let held = player.inventory.count(PROTECTOR_MEDAL) > 0
        || (player.inventory.items(InventoryType::Equipped))
            .any(|(_, item)| item.item_id == PROTECTOR_MEDAL);
    if held {
        return Page::Say(text(keys::PYRAMID_DUARTE_MEDAL_HAVE));
    }
    if kills < PROTECTOR_NEEDED {
        let (kills, left) = (kills.to_string(), (PROTECTOR_NEEDED - kills).to_string());
        let params = [("kills", kills.as_str()), ("left", left.as_str())];
        return Page::Say(content::text().get(keys::PYRAMID_DUARTE_MEDAL_MORE, &params));
    }
    if player.inventory.free(InventoryType::Equip) == 0 {
        return Page::Say(text(keys::PYRAMID_DUARTE_MEDAL_ROOM));
    }
    let mut effects = vec![Effect::GiveItem {
        item_id: PROTECTOR_MEDAL,
        quantity: 1,
    }];
    if player.quests.state(PROTECTOR_QUEST) == QuestState::Started {
        effects.push(Effect::CompleteQuest(PROTECTOR_QUEST));
    }
    Page::Act {
        text: text(keys::PYRAMID_DUARTE_MEDAL_GIVEN),
        effects,
    }
}

/// Duarte on a Pyramid stage: the player may give the run up and go back
/// to the lobby (Cosmic `2103013`).
fn duarte_forfeit() -> NpcScript {
    NpcScript::new(
        DUARTE,
        vec![Page::Ask {
            text: text(keys::PYRAMID_DUARTE_FORFEIT_ASK),
            on_yes: vec![Effect::Warp(events::PYRAMID_LOBBY)],
            on_no: None,
        }],
    )
}

/// Duarte after a run: on a result map he sends the player on to the
/// mode's room in the Shades, and there back to the lobby (Cosmic
/// `2103013`). There, after a run through all five stages, he offers the
/// mode's Tomb now or its gem for later. None elsewhere.
fn duarte_after(player: &Player) -> Option<NpcScript> {
    let map_id = player.stats.map_id;
    let mode = |first: i32| {
        let mode = map_id - first;
        (0..PyramidMode::ALL.len() as i32)
            .contains(&mode)
            .then_some(mode)
    };
    let (ask, to) = if let Some(mode) = mode(events::PYRAMID_RESULT) {
        (
            keys::PYRAMID_DUARTE_RESULT_ASK,
            events::PYRAMID_SHADES + mode,
        )
    } else if let Some(mode) = mode(events::PYRAMID_SHADES) {
        if player.pyramid_gem == Some(mode) {
            return Some(duarte_shades(player, mode));
        }
        (keys::PYRAMID_DUARTE_SHADES_ASK, events::PYRAMID_LOBBY)
    } else {
        return None;
    };
    Some(NpcScript::new(
        DUARTE,
        vec![Page::Ask {
            text: text(ask),
            on_yes: vec![Effect::Warp(to)],
            on_no: None,
        }],
    ))
}

/// Duarte in mode `mode`'s room in the Shades, after a run through all its
/// stages: into a room of its Tomb at once, or its gem, which needs a free
/// Etc slot, and back to the lobby.
fn duarte_shades(player: &Player, mode: i32) -> NpcScript {
    let gem = events::PYRAMID_GEM + mode;
    let kind = EventKind::PyramidTomb(PyramidMode::ALL[mode as usize]);
    let later = if player.inventory.free(InventoryType::Etc) == 0 {
        Page::Say(text(keys::PYRAMID_DUARTE_SHADES_NO_ROOM))
    } else {
        Page::Act {
            text: text(keys::PYRAMID_DUARTE_SHADES_GEM),
            effects: vec![
                Effect::GiveItem {
                    item_id: gem,
                    quantity: 1,
                },
                Effect::Warp(events::PYRAMID_LOBBY),
            ],
        }
    };
    let mut script = NpcScript::new(
        DUARTE,
        vec![Page::Menu {
            text: text(keys::PYRAMID_DUARTE_SHADES_PASSED),
            choices: vec![
                (
                    text(keys::PYRAMID_DUARTE_SHADES_NOW),
                    vec![Page::Done(vec![Effect::StartEvent(kind)])],
                ),
                (text(keys::PYRAMID_DUARTE_SHADES_LATER), vec![later]),
            ],
        }],
    );
    // Every room of the mode's Tomb is taken.
    script.refused = Some(text(keys::PYRAMID_DUARTE_TOMB_FULL));
    script
}

/// So Gong (Cosmic `2091005`) in the Dojo's hall: a challenger of level 25
/// or more may take it on alone, once past his trial, or as the leader of
/// a party whose `party` entry it is, buy a belt, reset their training
/// points or try for a medal, and anyone may hear what it is.
fn so_gong(entry: Entry, party: Entry, player: &Player) -> NpcScript {
    if player.stats.level < *events::DOJO.levels.start() {
        return NpcScript::new(SO_GONG, vec![Page::Say(text(keys::DOJO_SO_GONG_LOW_LEVEL))]);
    }
    let alone = match entry {
        // A first-timer must beat So Gong himself first, in his trial
        // (Cosmic's `getFinishedDojoTutorial`), whose own start finds
        // whether a room is free.
        Entry::Ready | Entry::Busy if !player.dojo.tutorial_done => Page::Decide {
            text: text(keys::DOJO_SO_GONG_TRIAL_ASK),
            on_yes: vec![Page::Done(vec![Effect::StartEvent(EventKind::DojoTrial)])],
            on_no: vec![Page::Say(text(keys::DOJO_SO_GONG_TRIAL_NO))],
        },
        // The run starts after a recorded floor (`event_started`), and a
        // no erases the record.
        Entry::Ready if player.dojo.stage > 0 => {
            let round = (player.dojo.stage / 6 * 5).to_string();
            Page::Decide {
                text: content::text().get(keys::DOJO_SO_GONG_RESUME, &[("round", &round)]),
                on_yes: vec![Page::Done(vec![Effect::StartEvent(EventKind::Dojo)])],
                on_no: vec![Page::Done(vec![Effect::DojoStage(0)])],
            }
        }
        Entry::Ready => Page::Done(vec![Effect::StartEvent(EventKind::Dojo)]),
        Entry::Busy => Page::Say(text(keys::DOJO_SO_GONG_BUSY)),
        _ => Page::Say(text(keys::DOJO_SO_GONG_LOW_LEVEL)),
    };
    // Those of the party in the hall come along; a team too far apart in
    // levels isn't eligible (`DOJO_PARTY.spread`).
    let party = match party {
        Entry::Ready => Page::Done(vec![Effect::StartEvent(EventKind::DojoParty)]),
        Entry::Busy => Page::Say(text(keys::DOJO_SO_GONG_BUSY)),
        Entry::NotEligible => Page::Say(text(keys::DOJO_SO_GONG_PARTY_SPREAD)),
        Entry::NoParty | Entry::NotLeader => Page::Say(text(keys::DOJO_SO_GONG_PARTY_LEADER)),
    };
    let points = player.dojo.points;
    let reset = Page::Decide {
        text: text(keys::DOJO_SO_GONG_RESET_ASK),
        on_yes: vec![Page::Act {
            text: text(keys::DOJO_SO_GONG_RESET_DONE),
            effects: vec![Effect::DojoPoints(-points)],
        }],
        on_no: vec![Page::Say(text(keys::DOJO_SO_GONG_RESET_NO))],
    };
    let mut script = NpcScript::new(
        SO_GONG,
        vec![Page::Menu {
            text: text(keys::DOJO_SO_GONG_MENU),
            choices: vec![
                (text(keys::DOJO_SO_GONG_ALONE), vec![alone]),
                (text(keys::DOJO_SO_GONG_PARTY), vec![party]),
                (text(keys::DOJO_SO_GONG_BELT), vec![so_gong_belts(player)]),
                (text(keys::DOJO_SO_GONG_RESET), vec![reset]),
                (text(keys::DOJO_SO_GONG_MEDAL), vec![so_gong_medal(player)]),
                (
                    text(keys::DOJO_SO_GONG_ABOUT),
                    vec![Page::Say(text(keys::DOJO_SO_GONG_INFO))],
                ),
            ],
        }],
    );
    // Every Dojo filled since So Gong checked.
    script.refused = Some(text(keys::DOJO_SO_GONG_BUSY));
    script.inventory_full = Some(text(keys::DOJO_SO_GONG_ROOM));
    script
}

/// So Gong's belts (Cosmic `2091005`), for `player`, who needs room in
/// their EQUIP inventory to be shown them. Each is for a challenger over
/// its level with its price in training points, who hands in the belt
/// before it, unequipped; otherwise he says what it takes.
fn so_gong_belts(player: &Player) -> Page {
    use game::dojo::{BELTS, BELT_LEVELS, BELT_PRICES};
    if player.inventory.free(InventoryType::Equip) == 0 {
        return Page::Say(text(keys::DOJO_SO_GONG_ROOM));
    }
    let item = |key, id: i32| content::text().get(key, &[("belt", &id.to_string())]);
    let in_bag = |id| player.inventory.count(id) > 0;
    let owns = |id| in_bag(id) || player.inventory.equipped_ids().any(|(_, held)| held == id);
    let points = player.dojo.points;
    let choices = (0..BELTS.len())
        .map(|i| {
            let belt = BELTS[i];
            let label = item(
                if owns(belt) {
                    keys::DOJO_SO_GONG_BELT_HELD
                } else {
                    keys::DOJO_SO_GONG_BELT_CHOICE
                },
                belt,
            );
            let before = i.checked_sub(1).map(|j| BELTS[j]);
            let (level, price) = (BELT_LEVELS[i], BELT_PRICES[i]);
            let handed = before.is_none_or(in_bag);
            let page = if before.is_none_or(owns)
                && handed
                && points >= price
                && player.stats.level > level
            {
                let mut effects: Vec<_> = before
                    .map(|item_id| Effect::TakeItem {
                        item_id,
                        quantity: 1,
                    })
                    .into_iter()
                    .collect();
                effects.extend([
                    Effect::GiveItem {
                        item_id: belt,
                        quantity: 1,
                    },
                    Effect::DojoPoints(-price),
                ]);
                Page::Act {
                    text: item(keys::DOJO_SO_GONG_BELT_GIVEN, belt),
                    effects,
                }
            } else {
                let mut lacking = Vec::new();
                if !handed {
                    lacking.push(text(keys::DOJO_SO_GONG_BELT_LACKS_BELT));
                }
                if price > points {
                    let more = (price - points).to_string();
                    lacking.push(
                        content::text()
                            .get(keys::DOJO_SO_GONG_BELT_LACKS_POINTS, &[("points", &more)]),
                    );
                }
                let lacking = match lacking.join(&text(keys::DOJO_SO_GONG_BELT_AND)) {
                    lacking if lacking.is_empty() => lacking,
                    lacking => content::text()
                        .get(keys::DOJO_SO_GONG_BELT_LACKING, &[("lacking", &lacking)]),
                };
                let before =
                    before.map_or(String::new(), |id| item(keys::DOJO_SO_GONG_BELT_BEFORE, id));
                let (belt, level, price) = (belt.to_string(), level.to_string(), price.to_string());
                Page::Say(content::text().get(
                    keys::DOJO_SO_GONG_BELT_NEEDS,
                    &[
                        ("belt", &belt),
                        ("before", &before),
                        ("level", &level),
                        ("points", &price),
                        ("lacking", &lacking),
                    ],
                ))
            };
            (label, vec![page])
        })
        .collect();
    let points = points.to_string();
    Page::Menu {
        text: content::text().get(keys::DOJO_SO_GONG_BELTS, &[("points", &points)]),
        choices,
    }
}

/// So Gong's medals (Cosmic `2091005`), for `player`. Medal `n` is for
/// beating round `n`'s boss 100 times and going on from its floor
/// (`scripts::dojo_next`); he takes a first try-out, then hands each over
/// once won and sets the next.
fn so_gong_medal(player: &Player) -> Page {
    use game::dojo::{medal, MEDAL_KILLS, ROUNDS};
    let (stage, kills) = (player.dojo.vanquisher_stage, player.dojo.vanquisher_kills);
    let named = |key, extra: &[(&str, &str)]| {
        let medal = medal(stage.max(1)).to_string();
        let mut params = vec![("medal", medal.as_str())];
        params.extend_from_slice(extra);
        content::text().get(key, &params)
    };
    if stage <= 0 {
        return Page::Decide {
            text: named(keys::DOJO_SO_GONG_MEDAL_TRY, &[]),
            on_yes: vec![Page::Done(vec![Effect::DojoMedal { stage: 1, kills: 0 }])],
            on_no: vec![Page::Say(text(keys::DOJO_SO_GONG_MEDAL_NO))],
        };
    }
    if stage > ROUNDS {
        return Page::Say(text(keys::DOJO_SO_GONG_MEDAL_ALL));
    }
    if kills < MEDAL_KILLS {
        let left = (MEDAL_KILLS - kills).to_string();
        return Page::Say(named(keys::DOJO_SO_GONG_MEDAL_MORE, &[("kills", &left)]));
    }
    if player.inventory.free(InventoryType::Equip) == 0 {
        return Page::Say(text(keys::DOJO_SO_GONG_MEDAL_ROOM));
    }
    Page::Act {
        text: named(keys::DOJO_SO_GONG_MEDAL_GIVEN, &[]),
        effects: vec![
            Effect::GiveItem {
                item_id: medal(stage),
                quantity: 1,
            },
            Effect::DojoMedal {
                stage: stage + 1,
                kills: 0,
            },
        ],
    }
}

/// So Gong (Cosmic `2091005`) to `player` on a Dojo floor. On a rest floor
/// the challenger may go on to the next, which erases a recorded score,
/// leave by the exit's `st00`, or record their score there; on a fighting
/// floor they may give up, out to the exit with a pink line.
///
/// On the party Dojo's there is no record, and the `leader` going on
/// takes everyone on the rest floor along; anyone else goes alone.
fn so_gong_on_floor(player: &Player, leader: bool) -> NpcScript {
    let map_id = player.stats.map_id;
    let party = events::DOJO_PARTY.holds(map_id);
    let floor = events::dojo_floor(map_id).unwrap_or(1);
    if events::rest_floor(floor) {
        let on = match party {
            true if leader => Effect::EventCommand(EventCommand::DojoOn(floor)),
            true => Effect::WarpToPortal(events::DOJO_PARTY.stage_map(floor + 1), 0),
            false => Effect::WarpToPortal(events::DOJO.stage_map(floor + 1), 0),
        };
        let record = if player.dojo.stage == floor {
            Page::Say(text(keys::DOJO_SO_GONG_RECORD_KEPT))
        } else {
            Page::Act {
                text: text(keys::DOJO_SO_GONG_RECORDED),
                effects: vec![Effect::DojoStage(floor)],
            }
        };
        let mut choices = vec![
            (
                text(keys::DOJO_SO_GONG_CONTINUE),
                vec![Page::Done(vec![Effect::DojoStage(0), on])],
            ),
            (
                text(keys::DOJO_SO_GONG_LEAVE),
                vec![Page::Ask {
                    text: text(keys::DOJO_SO_GONG_LEAVE_ASK),
                    on_yes: vec![Effect::WarpToPortal(events::DOJO_EXIT, 1)],
                    on_no: None,
                }],
            ),
        ];
        if !party {
            choices.push((
                text(keys::DOJO_SO_GONG_RECORD),
                vec![Page::Decide {
                    text: text(keys::DOJO_SO_GONG_RECORD_ASK),
                    on_yes: vec![record],
                    on_no: vec![Page::Say(text(keys::DOJO_SO_GONG_RECORD_NO))],
                }],
            ));
        }
        return NpcScript::new(
            SO_GONG,
            vec![Page::Menu {
                text: text(keys::DOJO_SO_GONG_REST),
                choices,
            }],
        );
    }
    NpcScript::new(
        SO_GONG,
        vec![Page::Ask {
            text: text(keys::DOJO_SO_GONG_QUIT),
            on_yes: vec![
                Effect::WarpToPortal(events::DOJO_EXIT, 0),
                Effect::Pink(text(keys::DOJO_SO_GONG_QUIT_DONE)),
            ],
            on_no: Some(text(keys::DOJO_SO_GONG_QUIT_NO)),
        }],
    )
}

/// The notice board in Mu Lung (Cosmic `2091006`): touching it takes the
/// player to the Dojo's entrance, saving where they were for its way out
/// (`scripts::portal`'s `dojang_exit`), and reading it shows its notice.
fn dojo_board(player: &Player) -> NpcScript {
    NpcScript::new(
        DOJO_BOARD,
        vec![Page::Menu {
            text: text(keys::DOJO_BOARD_MENU),
            choices: vec![
                (
                    text(keys::DOJO_BOARD_CHALLENGE),
                    vec![Page::Ask {
                        text: text(keys::DOJO_BOARD_ASK),
                        on_yes: vec![
                            Effect::SaveLocation(DOJO_RETURN, Some(player.stats.map_id)),
                            Effect::WarpToPortal(events::DOJO_ENTRANCE, events::DOJO_BOARD_PORTAL),
                        ],
                        on_no: Some(text(keys::DOJO_BOARD_DECLINE)),
                    }],
                ),
                (
                    text(keys::DOJO_BOARD_READ),
                    vec![
                        Page::Say(text(keys::DOJO_BOARD_NOTICE_0)),
                        Page::Say(text(keys::DOJO_BOARD_NOTICE_1)),
                    ],
                ),
            ],
        }],
    )
}

/// The Dimensional Mirror (Cosmic `9010022`): a slide menu of the places
/// open at the player's level, each saving the town for the way back
/// before going. Cosmic's Ariant Coliseum (0), second Carnival (3) and
/// Construction Site (6) have no event here, so they aren't offered; its
/// Pyramid (5), stubbed there, is.
fn dimensional_mirror(player: &Player) -> NpcScript {
    let level = player.stats.level;
    let here = player.stats.map_id;
    let go = |kind, map, portal| {
        vec![Page::Done(vec![
            Effect::SaveLocation(kind, Some(here)),
            Effect::WarpToPortal(map, portal),
        ])]
    };
    let places = [
        (
            1,
            level >= 25,
            keys::MIRROR_DOJO,
            go(DOJO_RETURN, events::DOJO_ENTRANCE, MIRROR_DOJO_PORTAL),
        ),
        (
            2,
            carnival::CARNIVAL_LEVELS.contains(&level),
            keys::MIRROR_CARNIVAL,
            go(
                CARNIVAL_RETURN,
                carnival::CARNIVAL_LOBBY,
                MIRROR_CARNIVAL_PORTAL,
            ),
        ),
        (
            5,
            level >= 40,
            keys::MIRROR_PYRAMID,
            go(PYRAMID_RETURN, events::PYRAMID_LOBBY, MIRROR_PYRAMID_PORTAL),
        ),
    ];
    let mut choices: Vec<_> = places
        .into_iter()
        .filter(|&(_, open, ..)| open)
        .map(|(id, _, key, pages)| (id, text(key), pages))
        .collect();
    if choices.is_empty() {
        choices.push((-1, text(keys::MIRROR_NOWHERE), Vec::new()));
    }
    NpcScript::new(DIMENSIONAL_MIRROR, vec![Page::Slide { set: 0, choices }])
}

/// Guon (Cosmic `2094002`) on the ship's map `map_id`: anyone but the
/// leader may give up and go back Over the Pirate Ship; the leader hears
/// what the map asks.
fn guon_aboard(player: &Player, leader: bool, marks: u8, bare: bool) -> NpcScript {
    if !leader {
        return ask_warp(GUON_ABOARD, keys::PIRATE_GUON_LEAVE, events::PIRATE_LOBBY);
    }
    let map_id = player.stats.map_id;
    let stage = |n: usize| events::PIRATE_STAGES[n];
    let key = match map_id {
        _ if map_id == stage(1) => return guon_marks(player, marks),
        // In a bellflower room beaten bare, he shows its chest (Cosmic
        // `setReactorState`); again is no matter.
        _ if bare && (map_id == stage(2) + 1 || map_id == stage(3) + 1) => {
            let name = crate::reactors::PIRATE_CHEST_NAMES[usize::from(map_id == stage(3) + 1)];
            let page = Page::Act {
                text: text(keys::PIRATE_GUON_CHEST),
                effects: vec![Effect::SetReactorState { name, state: 1 }],
            };
            return NpcScript::new(GUON_ABOARD, vec![page]);
        }
        _ if map_id == stage(2) || map_id == stage(3) => keys::PIRATE_GUON_DECK,
        _ if map_id == stage(2) + 1 || map_id == stage(3) + 1 => keys::PIRATE_GUON_BONUS,
        _ if map_id == stage(2) + 2 || map_id == stage(3) + 2 => keys::PIRATE_GUON_CREW,
        _ if map_id == stage(4) => keys::PIRATE_GUON_DOORS,
        _ if map_id == stage(5) && bare => keys::PIRATE_GUON_SAVED,
        _ if map_id == stage(5) => keys::PIRATE_GUON_BOSS,
        _ => keys::PIRATE_GUON_BOXES,
    };
    NpcScript::new(GUON_ABOARD, vec![Page::Say(text(key))])
}

/// Wu Yang, freed on the Pirate Ship's last stage: the leader hearing him
/// out wins the run, which takes the team to the clear room; anyone else
/// is sent to the leader (Cosmic `2094001`). Monsters may still be about.
fn wu_yang_freed(leader: bool) -> NpcScript {
    if !leader {
        return NpcScript::new(WU_YANG, vec![Page::Say(text(keys::PIRATE_WU_WAIT))]);
    }
    let stage = events::PIRATE.last_stage();
    NpcScript::new(
        WU_YANG,
        vec![
            Page::Say(text(keys::PIRATE_WU_THANKS)),
            Page::Done(vec![Effect::EventCommand(EventCommand::ClearStage(stage))]),
        ],
    )
}

/// Wu Yang in the Pirate Ship's clear room (Cosmic `2094001`): a feather
/// (`HPQ_REWARD`) and the way back to Herb Town, or the next of the Lord
/// Pirate's Hats for twenty feathers and the hat before.
fn wu_yang_clear(player: &Player) -> NpcScript {
    use events::{HPQ_REWARD, PIRATE_HATS, PIRATE_HAT_FEATHERS};
    let room = |kind: InventoryType| {
        player.inventory.items(kind).count() < usize::from(player.inventory.slot_limit(kind))
    };
    let out = if items::fits_until(&player.inventory, HPQ_REWARD, 1, None) {
        Page::Done(vec![
            Effect::GiveItem {
                item_id: HPQ_REWARD,
                quantity: 1,
            },
            Effect::WarpToPortal(events::PIRATE_LOBBY, 0),
        ])
    } else {
        Page::Say(text(keys::PIRATE_WU_ROOM_ETC))
    };
    let need = content::text().get(
        keys::PIRATE_WU_NEED_FEATHERS,
        &[
            ("n", &PIRATE_HAT_FEATHERS.to_string()),
            ("item", &HPQ_REWARD.to_string()),
        ],
    );
    let held = PIRATE_HATS
        .iter()
        .rposition(|&hat| player.inventory.count(hat) > 0);
    let hat = match held {
        Some(n) if n + 1 == PIRATE_HATS.len() => Page::Say(text(keys::PIRATE_WU_BEST)),
        _ if player.inventory.count(HPQ_REWARD) < i32::from(PIRATE_HAT_FEATHERS) => {
            Page::Say(need.clone())
        }
        _ if !room(InventoryType::Equip) => Page::Say(text(keys::PIRATE_WU_ROOM_EQUIP)),
        _ => {
            let next = held.map_or(0, |n| n + 1);
            let old = held.map(|n| Effect::TakeItem {
                item_id: PIRATE_HATS[n],
                quantity: 1,
            });
            let effects = old
                .into_iter()
                .chain([
                    Effect::TakeItem {
                        item_id: HPQ_REWARD,
                        quantity: PIRATE_HAT_FEATHERS,
                    },
                    Effect::GiveItem {
                        item_id: PIRATE_HATS[next],
                        quantity: 1,
                    },
                ])
                .collect();
            Page::Act {
                text: text(keys::PIRATE_WU_HAT_GIVEN),
                effects,
            }
        }
    };
    let menu = Page::Menu {
        text: text(keys::PIRATE_WU_MENU),
        choices: vec![
            (text(keys::PIRATE_WU_OUT), vec![out]),
            (text(keys::PIRATE_WU_HAT), vec![hat]),
        ],
    };
    let mut script = NpcScript::new(WU_YANG, vec![menu]);
    script.inventory_full = Some(text(keys::PIRATE_WU_ROOM_ETC));
    script.refused = Some(need);
    script
}

/// Guon to the leader on the second stage, whose team has handed him
/// `marks` Marks: he takes twenty of the next in turn, if they hold them,
/// and asks for the one after, or says what he wants (Cosmic `2094002`,
/// `stage2`). The event checks the turn again (`HandInMarks`).
fn guon_marks(player: &Player, marks: u8) -> NpcScript {
    // What he wants with `n` Marks in.
    const WANTS: [&str; 4] = [
        keys::PIRATE_GUON_MARKS,
        keys::PIRATE_GUON_RISING,
        keys::PIRATE_GUON_VETERAN,
        keys::PIRATE_GUON_MARKS_IN,
    ];
    let n = usize::from(marks).min(events::PIRATE_MARKS.len());
    let wanted = events::PIRATE_MARKS_WANTED;
    let page = match events::PIRATE_MARKS.get(n) {
        Some(&(mark, _)) if player.inventory.count(mark) >= i32::from(wanted) => Page::Act {
            text: text(WANTS[n + 1]),
            effects: vec![
                Effect::TakeItem {
                    item_id: mark,
                    quantity: wanted,
                },
                Effect::EventCommand(EventCommand::HandInMarks(marks + 1)),
            ],
        },
        _ => Page::Say(text(WANTS[n])),
    };
    NpcScript::new(GUON_ABOARD, vec![page])
}

/// The Altaire Earrings, then Glittering, then Brilliant (Character.nx
/// Accessory).
const ALTAIRE_EARRINGS: [i32; 3] = [1_032_060, 1_032_061, 1_032_072];

/// Ellin's prizes (Cosmic `2133000`, "reclaim a prize"): ten Altaire
/// Fragments make Altaire Earrings, and ten more each make a pair in
/// `player`'s bag the next, while they hold none of that pair yet; the
/// worn ones don't count (Cosmic `haveItem`). Cosmic's rule: Quest.nx has
/// no such trade (`v83ref/lanes/QUESTIONS.md` a125 5).
fn ellin_prizes(player: &Player) -> Page {
    let has = |item_id| player.inventory.count(item_id) > 0;
    let fragments = player.inventory.count(events::ALTAIRE_FRAGMENT) >= 10;
    let steps = [
        (
            keys::ELLIN_PRIZE_ALTAIRE,
            None,
            keys::ELLIN_PRIZE_NO_FRAGMENTS,
        ),
        (
            keys::ELLIN_PRIZE_GLITTERING,
            Some(ALTAIRE_EARRINGS[0]),
            keys::ELLIN_PRIZE_NO_ALTAIRE,
        ),
        (
            keys::ELLIN_PRIZE_BRILLIANT,
            Some(ALTAIRE_EARRINGS[1]),
            keys::ELLIN_PRIZE_NO_GLITTERING,
        ),
    ];
    let choices = (steps.into_iter().zip(ALTAIRE_EARRINGS))
        .map(|((label, from, refusal), to)| {
            let page = if fragments && from.map_or(true, has) && !has(to) {
                let mut effects: Vec<Effect> = (from.into_iter())
                    .map(|item_id| Effect::TakeItem {
                        item_id,
                        quantity: 1,
                    })
                    .collect();
                effects.extend([
                    Effect::TakeItem {
                        item_id: events::ALTAIRE_FRAGMENT,
                        quantity: 10,
                    },
                    Effect::GiveItem {
                        item_id: to,
                        quantity: 1,
                    },
                ]);
                Page::Done(effects)
            } else {
                Page::Say(text(refusal))
            };
            (text(label), vec![page])
        })
        .collect();
    Page::Menu {
        text: text(keys::ELLIN_PRIZE_MENU),
        choices,
    }
}

/// Rolly (Cosmic `9103001`) in Ludibrium: the leader takes the party into
/// the Ludibrium Maze, and anyone may hear its rules. Cosmic's Party
/// Search toggle is left out.
fn rolly(entry: Entry) -> NpcScript {
    let maze = &events::LUDIMAZE;
    let enter = match entry {
        Entry::NoParty => Page::Say(text(keys::LUDIMAZE_NO_PARTY)),
        Entry::NotLeader => Page::Say(text(keys::LUDIMAZE_NOT_LEADER)),
        Entry::NotEligible => Page::Say(event_terms(maze, keys::LUDIMAZE_NOT_ELIGIBLE)),
        Entry::Busy => Page::Say(text(keys::LUDIMAZE_BUSY)),
        Entry::Ready => Page::Done(vec![Effect::StartEvent(EventKind::LudiMaze)]),
    };
    let mut script = NpcScript::new(
        ROLLY,
        vec![Page::Menu {
            text: event_terms(maze, keys::LUDIMAZE_MENU),
            choices: vec![
                (text(keys::LUDIMAZE_ENTER), vec![enter]),
                (
                    text(keys::LUDIMAZE_ABOUT),
                    vec![Page::Say(text(keys::LUDIMAZE_INFO))],
                ),
            ],
        }],
    );
    // Someone left or moved since Rolly checked, or another party went in.
    script.refused = Some(event_terms(maze, keys::LUDIMAZE_NOT_ELIGIBLE));
    script
}

/// Pietri (Cosmic `9103000`) in the maze's goal room: only the leader,
/// with the whole team there and at least `MAZE_COUPONS_WANTED` coupons,
/// hands them all in, which wins the maze for `MAZE_COUPON_EXP` each to
/// every member (`EventCommand::WinWithCoupons`).
fn pietri(leader: bool, won: bool, together: bool, player: &Player) -> NpcScript {
    let coupons = player.inventory.count(events::MAZE_COUPON);
    let short = || {
        let n = events::MAZE_COUPONS_WANTED.to_string();
        let item = events::MAZE_COUPON.to_string();
        content::text().get(keys::LUDIMAZE_PIETRI_SHORT, &[("n", &n), ("item", &item)])
    };
    let page = if won {
        Page::Say(text(keys::LUDIMAZE_PIETRI_DONE))
    } else if !leader {
        Page::Say(text(keys::LUDIMAZE_PIETRI_NOT_LEADER))
    } else if !together {
        Page::Say(text(keys::LUDIMAZE_PIETRI_WAIT))
    } else if coupons < events::MAZE_COUPONS_WANTED {
        Page::Say(short())
    } else {
        let quantity = i16::try_from(coupons).unwrap_or(i16::MAX);
        let (n, item) = (quantity.to_string(), events::MAZE_COUPON.to_string());
        let each = (i32::from(quantity) * events::MAZE_COUPON_EXP).to_string();
        Page::Ask {
            text: content::text().get(
                keys::LUDIMAZE_PIETRI_ASK,
                &[("n", &n), ("item", &item), ("exp", &each)],
            ),
            on_yes: vec![
                Effect::TakeItem {
                    item_id: events::MAZE_COUPON,
                    quantity,
                },
                Effect::EventCommand(EventCommand::WinWithCoupons(i32::from(quantity))),
            ],
            on_no: None,
        }
    };
    let mut script = NpcScript::new(PIETRI, vec![page]);
    // The coupons went before the yes.
    script.refused = Some(short());
    script
}

/// Rolly (Cosmic `9103002`, `giveEventReward`) in the clear room asks each
/// member of the team that won the Ludibrium Maze whether to take their
/// present, gives them one of `MAZE_REWARDS` and sends them to the way
/// out, so nobody gets it twice. A member without room for any of the
/// rewards (`has_reward_slot`) gets nothing yet, and stays.
fn maze_present(player: &Player, rng: &mut impl Rng) -> Option<NpcScript> {
    if player.stats.map_id != events::MAZE_CLEAR {
        return None;
    }
    let last = if has_reward_slot(&player.inventory) {
        let &(item_id, quantity) = events::MAZE_REWARDS.choose(rng)?;
        Page::Done(vec![
            Effect::GiveItem { item_id, quantity },
            Effect::Warp(events::MAZE_EXIT),
        ])
    } else {
        Page::Say(text(keys::LUDIMAZE_PRESENT_FULL))
    };
    let ask = Page::Ask {
        text: text(keys::LUDIMAZE_PRESENT_ASK),
        on_yes: Vec::new(),
        on_no: None,
    };
    let mut script = NpcScript::new(ROLLY_PRESENT, vec![ask, last]);
    script.inventory_full = Some(text(keys::LUDIMAZE_PRESENT_FULL));
    Some(script)
}

/// Papulatus in the past sends a visitor back to their own time. A
/// Buccaneer he has taken up on "A New Revelation" (6363) goes back with
/// the record its completion reads.
fn past_papulatus(player: &Player) -> NpcScript {
    use crate::scripts::{NEW_REVELATION, SENT_BACK, SENT_BACK_RECORD};
    let mut script = NpcScript::new(
        PAST_PAPULATUS,
        vec![Page::Say(text(keys::PAST_PAPULATUS_SEND_BACK))],
    );
    let sending = player.quests.state(NEW_REVELATION) == QuestState::Started
        && player.quests.info(SENT_BACK_RECORD) != Some(SENT_BACK);
    script.on_end = sending
        .then_some(Effect::SetRecord {
            record: SENT_BACK_RECORD,
            value: SENT_BACK,
        })
        .into_iter()
        .chain([Effect::Warp(crate::events::CLOCKTOWER_DEPTHS)])
        .collect();
    script
}

/// Kyrin's 4th-job tests (`scripts::KYRIN_TESTS`). In town she offers a
/// Pirate on one, its record not yet set, the way into its room, one at a
/// time; past it, she marks the test passed and sends them back to town,
/// where the client completes the quest with her. `None` otherwise.
fn kyrin_fourth_job_test(player: &Player) -> Option<NpcScript> {
    use crate::scripts::{KYRIN_PASSED, KYRIN_PASSED_ROOM, KYRIN_TESTS};
    let on_test = KYRIN_TESTS.iter().find(|test| {
        player.quests.state(test.quest) == QuestState::Started
            && player.quests.info(test.record) != Some(KYRIN_PASSED)
    });
    if player.stats.map_id == KYRIN_PASSED_ROOM {
        let mut script = NpcScript::new(KYRIN, vec![Page::Say(text(keys::KYRIN_TEST_PASSED))]);
        script.on_end = on_test
            .map(|test| Effect::SetRecord {
                record: test.record,
                value: KYRIN_PASSED,
            })
            .into_iter()
            .chain([Effect::WarpToPortal(KYRIN_MAP, 0)])
            .collect();
        return Some(script);
    }
    let test = on_test.filter(|_| player.stats.map_id == KYRIN_MAP)?;
    let mut script = NpcScript::new(
        KYRIN,
        vec![Page::Ask {
            text: text(keys::KYRIN_TEST_ASK),
            on_yes: vec![
                Effect::RequireEmptyMap(test.room),
                Effect::WarpToPortal(test.room, 0),
            ],
            on_no: None,
        }],
    );
    script.refused = Some(text(keys::KYRIN_TEST_BUSY));
    Some(script)
}

/// The clone's room, Lord Jonathan's test, Kyrin's, the Junior Balrog's
/// other world, the Field of Wolves and the wolves' test room: the time a
/// challenger has in it, which the map itself doesn't set.
pub fn time_limit(map_id: i32) -> Option<u32> {
    if map_id == crate::scripts::BALROG_WORLD {
        return Some(BALROG_WORLD_TIME_LIMIT);
    }
    if map_id == crate::reactors::BART_ROOM {
        return Some(BART_TIME_LIMIT);
    }
    if crate::scripts::KYRIN_TESTS
        .iter()
        .any(|test| test.room == map_id)
    {
        return Some(crate::scripts::kyrin_time_limit());
    }
    if [scripts::FIELD_OF_WOLVES, scripts::WOLVES_TEST_ROOM].contains(&map_id) {
        return Some(WOLF_RUN_TIME_LIMIT);
    }
    is_clone_room(map_id).then_some(CLONE_TIME_LIMIT)
}

/// An Aran's time in the Field of Wolves or the wolves' test room
/// (Cosmic's `Aran_2ndmount` and `Aran_3rdmount` events).
const WOLF_RUN_TIME_LIMIT: u32 = 3 * 60;

fn is_clone_room(map_id: i32) -> bool {
    THIRD_JOBS.iter().any(|line| line.boss_map == map_id)
}

/// Whether the field puts its mobs and reactors back when the last player
/// leaves, as a clone's room does for the next challenger, a mini-dungeon's
/// copy for the next party, and a fourth job's solo room for the next
/// player.
pub fn resets_when_empty(map_id: i32) -> bool {
    is_clone_room(map_id)
        || crate::dungeons::of_copy(map_id).is_some()
        || crate::scripts::SOLO_MAPS.contains(&map_id)
}

/// Gachapon machine `npc_id`'s word that a bag is full, if one is. A
/// prize may go to any bag; rolling with one full would turn some prizes
/// away with the ticket kept: a free roll, which also tells which bag the
/// prize was for.
fn gachapon_bag_full(npc_id: i32, player: &Player) -> Option<NpcScript> {
    let inventory = &player.inventory;
    gachapon::BAGS
        .iter()
        .any(|&bag| inventory.free(bag) == 0)
        .then(|| NpcScript::new(npc_id, vec![Page::Say(text(keys::GACHAPON_FULL))]))
}

/// A Gachapon machine in `town` (report c058 Q6). To a player with a
/// ticket it offers the `prize` already rolled, named once it is theirs;
/// to anyone else it explains itself.
fn gachapon_machine(npc_id: i32, town: &'static str, prize: Option<Prize>) -> NpcScript {
    let Some(prize) = prize else {
        let choice = |label, answer| (text(label), vec![Page::Say(text(answer))]);
        return NpcScript::new(
            npc_id,
            vec![Page::Menu {
                text: content::text().get(keys::GACHAPON_MENU, &[("town", town)]),
                choices: vec![
                    choice(keys::GACHAPON_WHAT_CHOICE, keys::GACHAPON_WHAT),
                    choice(keys::GACHAPON_WHERE_CHOICE, keys::GACHAPON_WHERE),
                ],
            }],
        );
    };
    let item = prize.item_id.to_string();
    let count = prize.quantity.to_string();
    let mut script = NpcScript::new(
        npc_id,
        vec![
            Page::Ask {
                text: content::text().get(keys::GACHAPON_ASK, &[("town", town)]),
                on_yes: vec![Effect::Gachapon {
                    ticket: gachapon::TICKET,
                    item_id: prize.item_id,
                    quantity: prize.quantity,
                    town,
                    announced: prize.announced(),
                }],
                on_no: None,
            },
            Page::Say(
                content::text().get(keys::GACHAPON_WON, &[("item", &item), ("count", &count)]),
            ),
        ],
    );
    script.inventory_full = Some(text(keys::GACHAPON_FULL));
    script.refused = Some(text(keys::GACHAPON_NO_TICKET));
    script
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

/// Spiruna refines Dark Crystal Ore into Dark Crystals, as many as asked,
/// for those who finished her quest.
fn spiruna(player: &Player) -> NpcScript {
    if player.quests.state(SPIRUNA_QUEST) != QuestState::Completed {
        return NpcScript::new(SPIRUNA, vec![Page::Say(text(keys::SPIRUNA_OTHER))]);
    }
    let pages = vec![
        Page::Ask {
            text: text(keys::SPIRUNA_ASK),
            on_yes: Vec::new(),
            on_no: None,
        },
        how_many(&refiners::SPIRUNA_DARK_CRYSTAL, OnNumber::Spiruna),
    ];
    let mut script = NpcScript::new(SPIRUNA, pages);
    script.refused = Some(text(keys::SPIRUNA_SHORT));
    script.inventory_full = Some(text(keys::THIRD_JOB_INVENTORY_FULL));
    script
}

/// One kind of thing a refiner offers, under its label.
enum Offer {
    /// Refined in as many batches as asked.
    Batches(&'static str, &'static [Refine]),
    /// Made a bundle at a time, as arrows are.
    Bundles(&'static str, &'static [Refine]),
    /// Equipment made or upgraded one at a time.
    Equips(&'static str, &'static [Recipe]),
    /// Chris's Steel Plates from Iron Hog's Metal Hoofs.
    MetalHoof,
}

/// A refiner: who, their greeting (a yes or no first, with the goodbye
/// for a no, if `goodbye`), the share of the price they charge, and what
/// they offer.
struct Refiner {
    npc: i32,
    hello: &'static str,
    goodbye: Option<&'static str>,
    percent: i32,
    offers: &'static [Offer],
}

const MR_THUNDER: i32 = 1022003;
const CHRIS: i32 = 1052003;
const VOGEN: i32 = 2020000;
const PI: i32 = 2040016;
const MUHAMAD: i32 = 2100001;
const MR_SMITH: i32 = 1022004;
const VICIOUS: i32 = 1012002;
const JM: i32 = 1052002;

const THUNDER: Refiner = Refiner {
    npc: MR_THUNDER,
    hello: keys::REFINER_THUNDER_HELLO,
    goodbye: None,
    percent: 100,
    offers: &[
        Offer::Batches(keys::REFINER_MINERAL, refiners::MINERALS),
        Offer::Batches(keys::REFINER_JEWEL, refiners::JEWELS),
        Offer::Equips(keys::REFINER_THUNDER_HELMETS, refiners::THUNDER_HELMETS),
        Offer::Equips(keys::REFINER_THUNDER_SHIELDS, refiners::THUNDER_SHIELDS),
    ],
};

const CHRIS_REFINER: Refiner = Refiner {
    npc: CHRIS,
    hello: keys::REFINER_CHRIS_HELLO,
    goodbye: None,
    percent: 100,
    offers: &[
        Offer::Batches(keys::REFINER_MINERAL, refiners::MINERALS),
        Offer::Batches(keys::REFINER_JEWEL, refiners::JEWELS),
        Offer::MetalHoof,
        Offer::Equips(keys::REFINER_UPGRADE_CLAW, refiners::CHRIS_CLAWS),
    ],
};

const VOGEN_REFINER: Refiner = Refiner {
    npc: VOGEN,
    hello: keys::REFINER_VOGEN_HELLO,
    goodbye: None,
    percent: 100,
    offers: &[
        Offer::Batches(keys::REFINER_MINERAL, refiners::MINERALS),
        Offer::Batches(keys::REFINER_JEWEL, refiners::JEWELS),
        Offer::Batches(keys::REFINER_ROCK, refiners::ROCKS),
        Offer::Batches(keys::REFINER_CRYSTAL, refiners::CRYSTALS_AND_DARK),
        Offer::Batches(keys::REFINER_MATERIALS, refiners::MATERIALS),
        Offer::Bundles(keys::REFINER_ARROWS, refiners::ARROWS),
    ],
};

/// Pi and Muhamad charge 90% of the others' prices (Cosmic).
const PI_REFINER: Refiner = Refiner {
    npc: PI,
    hello: keys::REFINER_PI_HELLO,
    goodbye: None,
    percent: 90,
    offers: &[
        Offer::Batches(keys::REFINER_MINERAL, refiners::MINERALS),
        Offer::Batches(keys::REFINER_JEWEL, refiners::JEWELS),
        Offer::Batches(keys::REFINER_ROCK, refiners::ROCKS),
        Offer::Batches(keys::REFINER_CRYSTAL, refiners::CRYSTALS),
        Offer::Batches(keys::REFINER_MATERIALS, refiners::MATERIALS),
        Offer::Bundles(keys::REFINER_ARROWS, refiners::ARROWS),
    ],
};

const MUHAMAD_REFINER: Refiner = Refiner {
    npc: MUHAMAD,
    hello: keys::REFINER_MUHAMAD_HELLO,
    goodbye: Some(keys::REFINER_MUHAMAD_BYE),
    percent: 90,
    offers: &[
        Offer::Batches(keys::REFINER_MINERAL, refiners::MINERALS_AND_LIDIUM),
        Offer::Batches(keys::REFINER_JEWEL, refiners::JEWELS),
        Offer::Batches(keys::REFINER_CRYSTAL, refiners::CRYSTALS),
    ],
};

const SMITH_REFINER: Refiner = Refiner {
    npc: MR_SMITH,
    hello: keys::REFINER_SMITH_HELLO,
    goodbye: None,
    percent: 100,
    offers: &[
        Offer::Equips(keys::REFINER_MAKE_GLOVE, refiners::SMITH_GLOVES),
        Offer::Equips(keys::REFINER_UPGRADE_GLOVE, refiners::SMITH_GLOVE_UPGRADES),
        Offer::Batches(keys::REFINER_MATERIALS, refiners::MATERIALS),
    ],
};

const VICIOUS_REFINER: Refiner = Refiner {
    npc: VICIOUS,
    hello: keys::REFINER_VICIOUS_HELLO,
    goodbye: None,
    percent: 100,
    offers: &[
        Offer::Equips(keys::REFINER_BOW, refiners::VICIOUS_BOWS),
        Offer::Equips(keys::REFINER_CROSSBOW, refiners::VICIOUS_CROSSBOWS),
        Offer::Equips(keys::REFINER_MAKE_GLOVE, refiners::VICIOUS_GLOVES),
        Offer::Equips(
            keys::REFINER_UPGRADE_GLOVE,
            refiners::VICIOUS_GLOVE_UPGRADES,
        ),
        Offer::Batches(keys::REFINER_MATERIALS, refiners::MATERIALS),
        Offer::Bundles(keys::REFINER_ARROWS, refiners::ARROWS),
    ],
};

const JM_REFINER: Refiner = Refiner {
    npc: JM,
    hello: keys::REFINER_JM_HELLO,
    goodbye: None,
    percent: 100,
    offers: &[
        Offer::Equips(keys::REFINER_MAKE_GLOVE, refiners::JM_GLOVES),
        Offer::Equips(keys::REFINER_UPGRADE_GLOVE, refiners::JM_GLOVE_UPGRADES),
        Offer::Equips(keys::REFINER_MAKE_CLAW, refiners::JM_CLAWS),
        Offer::Equips(keys::REFINER_UPGRADE_CLAW, refiners::JM_CLAW_UPGRADES),
        Offer::Batches(keys::REFINER_MATERIALS, refiners::MATERIALS),
    ],
};

/// A refiner's menu of offers, each a menu of its items.
fn refiner(refiner: &Refiner) -> NpcScript {
    let percent = refiner.percent;
    let choices = refiner
        .offers
        .iter()
        .map(|offer| match *offer {
            Offer::Batches(label, refines) => (
                text(label),
                vec![refine_menu(refines, |refine| {
                    vec![how_many(refine, OnNumber::Refine { refine, percent })]
                })],
            ),
            Offer::Bundles(label, refines) => (
                text(label),
                vec![refine_menu(refines, |refine| {
                    refine_batches(refine, percent, 1)
                })],
            ),
            Offer::Equips(label, recipes) => {
                let pages = recipes.iter().map(|recipe| make(recipe, None)).collect();
                (text(label), vec![recipe_menu(recipes, pages)])
            }
            Offer::MetalHoof => {
                let refine = &refiners::METAL_HOOF;
                (
                    text(keys::REFINER_CHRIS_HOOF),
                    vec![
                        Page::Ask {
                            text: text(keys::REFINER_CHRIS_HOOF_ASK),
                            on_yes: Vec::new(),
                            on_no: None,
                        },
                        how_many(refine, OnNumber::Refine { refine, percent }),
                    ],
                )
            }
        })
        .collect();
    let pages = match refiner.goodbye {
        Some(goodbye) => vec![
            Page::Ask {
                text: text(refiner.hello),
                on_yes: Vec::new(),
                on_no: Some(text(goodbye)),
            },
            Page::Menu {
                text: text(keys::REFINER_WHAT),
                choices,
            },
        ],
        None => vec![Page::Menu {
            text: text(refiner.hello),
            choices,
        }],
    };
    with_maker_words(NpcScript::new(refiner.npc, pages))
}

/// The menu of `refines`, each leading to its `pages`. Two ways of making
/// the same item are told apart by their first material.
fn refine_menu(refines: &'static [Refine], pages: impl Fn(&'static Refine) -> Vec<Page>) -> Page {
    Page::Menu {
        text: text(keys::MAKER_PICK),
        choices: refines
            .iter()
            .map(|refine| {
                let mut label = format!("#t{}#", refine.item);
                if refine.yields > 1 {
                    label.push_str(&format!(" x{}", refine.yields));
                }
                if refines
                    .iter()
                    .filter(|other| other.item == refine.item)
                    .count()
                    > 1
                {
                    label.push_str(&format!(" (#t{}#)", refine.materials[0].0));
                }
                (label, pages(refine))
            })
            .collect(),
    }
}

/// Ask how many batches of `refine` to make, from one to a hundred.
fn how_many(refine: &Refine, on_number: OnNumber) -> Page {
    let item = refine.item.to_string();
    Page::AskNumber {
        text: content::text().get(keys::REFINER_HOW_MANY, &[("item", item.as_str())]),
        default: 1,
        min: 1,
        max: refiners::MOST_AT_ONCE,
        on_number,
    }
}

/// Ask to make `batches` of `refine` at `percent` of its price, listing
/// what they take, then say it is done.
fn refine_batches(refine: &Refine, percent: i32, batches: i32) -> Vec<Page> {
    let item = refine.item.to_string();
    let count = (i32::from(refine.yields) * batches).to_string();
    let mut ask = content::text().get(
        keys::REFINER_ASK,
        &[("count", count.as_str()), ("item", item.as_str())],
    );
    for &(item, quantity) in refine.materials {
        let quantity = times(i32::from(quantity), batches);
        ask.push_str(&format!("\r\n#i{item}# {quantity} #t{item}#"));
    }
    let meso = refine.meso(batches, percent);
    if meso > 0 {
        let meso = meso.to_string();
        let meso = content::text().get(keys::MAKER_MESO, &[("meso", meso.as_str())]);
        ask.push_str(&format!("\r\n#i{MESO_ICON}# {meso}"));
    }
    vec![
        Page::Ask {
            text: ask,
            on_yes: refining(refine, percent, batches),
            on_no: None,
        },
        Page::Say(text(keys::MAKER_DONE)),
    ]
}

/// Making `batches` of `refine` at `percent` of its price: the item is
/// given first, so a full inventory is told before short mesos, and those
/// before missing materials; nothing applies unless all of it does.
fn refining(refine: &Refine, percent: i32, batches: i32) -> Vec<Effect> {
    let mut effects = vec![Effect::GiveItem {
        item_id: refine.item,
        quantity: times(i32::from(refine.yields), batches),
    }];
    let meso = refine.meso(batches, percent);
    if meso > 0 {
        effects.push(Effect::SpendMeso(meso));
    }
    effects.extend(
        refine
            .materials
            .iter()
            .map(|&(item_id, quantity)| Effect::TakeItem {
                item_id,
                quantity: times(i32::from(quantity), batches),
            }),
    );
    effects
}

/// `quantity` `batches` times over, as an item count; the refiners' and
/// traders' bounds keep it in range.
fn times(quantity: i32, batches: i32) -> i16 {
    i16::try_from(quantity * batches).unwrap_or(i16::MAX)
}

/// An Altaire camper offers a Perfect Pitch for each hundred of their
/// item the player holds, up to three hundred at once.
fn pitch_trader(npc: i32, player: &Player) -> NpcScript {
    let input = refiners::PITCH_TRADERS
        .iter()
        .find(|&&(trader, _)| trader == npc)
        .map_or(0, |&(_, input)| input);
    let item = input.to_string();
    let say = |key| content::text().get(key, &[("item", item.as_str())]);
    let most = refiners::pitches(player.inventory.count(input));
    let trade = if most == 0 {
        vec![Page::Say(say(keys::ALTAIRE_SHORT))]
    } else {
        vec![Page::AskNumber {
            text: text(keys::ALTAIRE_HOW_MANY),
            default: most,
            min: 1,
            max: most,
            on_number: OnNumber::Pitches { input },
        }]
    };
    let menu = Page::Menu {
        text: say(keys::ALTAIRE_HELLO),
        choices: vec![(say(keys::ALTAIRE_TRADE), trade)],
    };
    let mut script = NpcScript::new(npc, vec![menu]);
    script.refused = Some(say(keys::ALTAIRE_SHORT));
    script.inventory_full = Some(text(keys::ALTAIRE_FULL));
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

/// A stylist's menu of services (`hair_*`, `face_*`, `skin_*`), if `npc` is
/// one. A service with nothing to offer the player is left out.
fn stylist(npc: i32, player: &Player, rng: &mut impl Rng) -> Option<NpcScript> {
    stylists::services(npc).next()?;
    let choices: Vec<_> = stylists::services(npc)
        .filter_map(|service| {
            let label = content::text().get(
                stylist_label(service.look),
                &[("item", service.coupon.to_string().as_str())],
            );
            Some((label, stylist_pages(service, player, rng)?))
        })
        .collect();
    // The client can't show an empty menu.
    if choices.is_empty() {
        return None;
    }
    Some(NpcScript::new(
        npc,
        vec![Page::Menu {
            text: text(keys::STYLIST_MENU),
            choices,
        }],
    ))
}

fn stylist_label(look: Look) -> &'static str {
    match look {
        Look::Cut => keys::STYLIST_CUT,
        Look::Dye => keys::STYLIST_DYE,
        Look::Face => keys::STYLIST_FACE,
        Look::Lens => keys::STYLIST_LENS,
        Look::OneTimeLens => keys::STYLIST_ONE_TIME_LENS,
        Look::Skin => keys::STYLIST_SKIN,
    }
}

/// A choice of the looks on offer, or a yes to a rolled one; then what
/// the stylist says after, or to a player without the coupon. `None` if
/// there is nothing to offer.
fn stylist_pages(service: &Service, player: &Player, rng: &mut impl Rng) -> Option<Vec<Page>> {
    let looks = service.looks(player);
    if looks.is_empty() {
        return (service.look == Look::OneTimeLens)
            .then(|| vec![Page::Say(text(keys::STYLIST_NO_LENS))]);
    }
    let payment = service.payment(player);
    let item = service.coupon.to_string();
    let say = |key: &str| content::text().get(key, &[("item", item.as_str())]);
    let effects = |(style, lens): (i32, Option<i32>)| match payment {
        Some(coupon) => coupon
            .or(lens)
            .map(|item_id| Effect::TakeItem {
                item_id,
                quantity: 1,
            })
            .into_iter()
            .chain([Effect::SetStyle(style)])
            .collect(),
        None => Vec::new(),
    };
    let after = Page::Say(say(if payment.is_some() {
        keys::STYLIST_DONE
    } else {
        keys::STYLIST_NO_COUPON
    }));
    let ask = match service.pick {
        Pick::Choose => Page::Style {
            text: say(if service.look == Look::OneTimeLens {
                keys::STYLIST_CHOOSE_LENS
            } else {
                keys::STYLIST_CHOOSE
            }),
            choices: looks
                .iter()
                .map(|&(style, lens)| (style, effects((style, lens))))
                .collect(),
        },
        Pick::Random => Page::Ask {
            text: say(keys::STYLIST_RANDOM),
            on_yes: effects(*looks.choose(rng)?),
            on_no: None,
        },
    };
    Some(vec![ask, after])
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
    text: Option<String>,
) -> Option<ScriptInput> {
    match (prompt, message_type, response) {
        (Prompt::Text(_), 0, 1) => Some(ScriptInput::Next),
        (Prompt::Text(page), 0, 0) if page.has_previous => Some(ScriptInput::Previous),
        (Prompt::YesNo { .. }, 1, 1) => Some(ScriptInput::Yes),
        (Prompt::YesNo { .. }, 1, 0) => Some(ScriptInput::No),
        (Prompt::Selection { .. }, 4, 1) => selection.map(ScriptInput::Select),
        (Prompt::Style { .. }, 7, 1) => selection.map(ScriptInput::Select),
        (Prompt::Slide { .. }, 14, 1) => selection.map(ScriptInput::Select),
        (Prompt::AskText { .. }, 2, 1) => text.map(ScriptInput::Text),
        // The client checks the range too, but only as typed.
        (Prompt::AskNumber { min, max, .. }, 3, 1) => selection
            .filter(|number| (min..=max).contains(&number))
            .map(ScriptInput::Number),
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
            Prompt::AskText { .. } => 2,
            Prompt::AskNumber { .. } => 3,
            Prompt::Slide { .. } => 14,
        })
    }

    fn page_text(step: &Step) -> String {
        match &step.prompt {
            Some(Prompt::Text(page)) => page.text.clone(),
            other => panic!("text page expected, got {other:?}"),
        }
    }

    fn question(step: &Step) -> String {
        match &step.prompt {
            Some(Prompt::YesNo { text, .. }) => text.clone(),
            other => panic!("yes/no expected, got {other:?}"),
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
    fn a_gachapon_machine_trades_a_ticket_for_its_prize() {
        let mut player = player(50);
        crate::items::give(&mut player.inventory, gachapon::TICKET, 1).unwrap();
        let mut script = NpcScript::for_npc(9100100, &player).unwrap();
        let ask = script.begin();
        assert_eq!(kind(&ask), Some(1));
        let Some(Effect::Gachapon {
            ticket,
            item_id,
            quantity,
            town,
            announced,
        }) = script
            .respond(ScriptInput::Yes)
            .unwrap()
            .effects
            .first()
            .cloned()
        else {
            panic!("the yes trades the ticket");
        };
        assert_eq!(town, "Henesys");
        assert_eq!(ticket, gachapon::TICKET);
        assert!(crate::items::item_name(item_id).is_some());
        assert!(quantity > 0);
        let _ = announced;

        // The same prize, by hand, to name it after the trade.
        let prize = Prize {
            item_id: 2_049_100,
            quantity: 1,
            tier: game_data::server_content::GachaponTier::Rare,
        };
        let mut script = gachapon_machine(9100101, "Ellinia", Some(prize));
        let yes = script.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            yes.effects,
            [Effect::Gachapon {
                ticket: gachapon::TICKET,
                item_id: 2_049_100,
                quantity: 1,
                town: "Ellinia",
                announced: true,
            }]
        );
        assert!(page_text(&yes).contains("2049100"));
        assert_eq!(buttons(&yes), (false, false));
        assert_eq!(script.respond(ScriptInput::Next), Some(Step::default()));
        // A full inventory, or a ticket gone since, is told.
        let mut script = gachapon_machine(9100101, "Ellinia", Some(prize));
        script.respond(ScriptInput::Yes);
        let full = script.inventory_full().unwrap();
        assert!(full.effects.is_empty());
        assert_eq!(page_text(&full), text(keys::GACHAPON_FULL));
        let mut script = gachapon_machine(9100101, "Ellinia", Some(prize));
        script.respond(ScriptInput::Yes);
        assert_eq!(
            page_text(&script.refused().unwrap()),
            text(keys::GACHAPON_NO_TICKET)
        );
    }

    #[test]
    fn a_gachapon_machine_wants_room_in_every_bag_before_it_rolls() {
        let mut player = player(50);
        crate::items::give(&mut player.inventory, gachapon::TICKET, 1).unwrap();
        for slot in 1..=player.inventory.slot_limit(InventoryType::Setup) as i16 {
            player
                .inventory
                .insert(InventoryType::Setup, slot, game::Item::stack(3_010_000, 1));
        }
        let mut script = NpcScript::for_npc(9100100, &player).unwrap();
        let full = script.begin();
        assert_eq!(kind(&full), Some(0));
        assert_eq!(page_text(&full), text(keys::GACHAPON_FULL));
        assert!(full.effects.is_empty());
    }

    #[test]
    fn a_remote_ticket_plays_the_machine_at_once() {
        let mut player = player(50);
        assert!(NpcScript::remote_gachapon(9100109, &player).is_none());
        // A Gachapon Ticket is not a remote one.
        crate::items::give(&mut player.inventory, gachapon::TICKET, 1).unwrap();
        assert!(NpcScript::remote_gachapon(9100109, &player).is_none());
        crate::items::give(&mut player.inventory, gachapon::REMOTE_TICKET, 1).unwrap();
        assert!(NpcScript::remote_gachapon(9100108, &player).is_none());

        let mut script = NpcScript::remote_gachapon(9100109, &player).unwrap();
        let won = script.begin();
        assert_eq!(kind(&won), Some(0));
        let [Effect::Gachapon {
            ticket,
            item_id,
            town,
            ..
        }] = won.effects[..]
        else {
            panic!("{:?}", won.effects);
        };
        assert_eq!((ticket, town), (gachapon::REMOTE_TICKET, "New Leaf City"));
        assert!(page_text(&won).contains(&item_id.to_string()));
        assert_eq!(buttons(&won), (false, false));
        assert_eq!(script.respond(ScriptInput::Next), Some(Step::default()));
        assert_eq!(
            page_text(&script.inventory_full().unwrap()),
            text(keys::GACHAPON_FULL)
        );

        // A full bag keeps the ticket.
        for slot in 1..=player.inventory.slot_limit(InventoryType::Etc) as i16 {
            player
                .inventory
                .insert(InventoryType::Etc, slot, game::Item::stack(4_000_000, 1));
        }
        let full = NpcScript::remote_gachapon(9100109, &player)
            .unwrap()
            .begin();
        assert_eq!(page_text(&full), text(keys::GACHAPON_FULL));
        assert!(full.effects.is_empty());
    }

    #[test]
    fn a_gachapon_machine_explains_itself_without_a_ticket() {
        let mut script = NpcScript::for_npc(9100117, &player(50)).unwrap();
        assert_eq!(kind(&script.begin()), Some(4));
        for (choice, answer) in [(0, keys::GACHAPON_WHAT), (1, keys::GACHAPON_WHERE)] {
            let mut script = script.clone();
            let page = script.respond(ScriptInput::Select(choice)).unwrap();
            assert!(page.effects.is_empty());
            assert_eq!(page_text(&page), text(answer));
        }
        assert!(script.respond(ScriptInput::Select(2)).is_none());
        // A machine Map.nx never places stays silent.
        assert!(NpcScript::for_npc(9100108, &player(50)).is_none());
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
    fn every_quest_script_the_catalog_supports_exists() {
        for script in game_data::quests::SCRIPTS {
            assert!(
                NpcScript::for_quest(script, &player(50))
                    .or_else(|| {
                        NpcScript::for_family_quest(script, &player(50), &FamilyInfo::default())
                    })
                    .is_some(),
                "{script} has no script"
            );
        }
    }

    /// The effects at the end of a two-page quest script.
    fn ending(script: &str, player: &Player) -> (i32, Vec<Effect>) {
        let mut script = NpcScript::for_quest(script, player).unwrap();
        let npc = script.npc_id();
        assert_eq!(buttons(&script.begin()), (false, true));
        assert_eq!(
            buttons(&script.respond(ScriptInput::Next).unwrap()),
            (true, true)
        );
        let end = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(end.prompt, None);
        (npc, end.effects)
    }

    /// What a one-page script applies once read.
    fn read_through(mut script: NpcScript) -> (i32, Vec<Effect>) {
        let npc = script.npc_id();
        assert!(script.begin().prompt.is_some());
        let end = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(end.prompt, None);
        (npc, end.effects)
    }

    #[test]
    fn the_trash_can_holds_jms_list_from_5_to_8_pm() {
        let list = Effect::GiveItem {
            item_id: INFORMANT_LIST,
            quantity: 1,
        };
        for hour in [16, 20, 0] {
            for lost in [false, true] {
                let script = trash_can(&traveller(30, 5000, false), lost, hour);
                assert_eq!(read_through(script), (TRASH_CAN, vec![]), "{hour}h");
            }
        }
        for hour in [17, 19] {
            let found = trash_can(&traveller(30, 0, false), false, hour);
            assert_eq!(
                read_through(found),
                (
                    TRASH_CAN,
                    vec![
                        Effect::CompleteQuest(2214),
                        list.clone(),
                        Effect::GainExp(20_000)
                    ]
                )
            );
            assert!(trash_can(&traveller(30, 0, false), false, hour)
                .inventory_full()
                .is_some());
        }
        // A lost list costs the fee, which must be held.
        let short = trash_can(&traveller(30, INFORMANT_LIST_FEE - 1, false), true, 18);
        assert_eq!(read_through(short), (TRASH_CAN, vec![]));
        let paid = trash_can(&traveller(30, INFORMANT_LIST_FEE, false), true, 18);
        assert_eq!(
            read_through(paid),
            (
                TRASH_CAN,
                vec![
                    Effect::SpendMeso(INFORMANT_LIST_FEE),
                    Effect::CompleteQuest(2215),
                    list
                ]
            )
        );
    }

    #[test]
    fn each_informant_completes_their_quest_and_the_last_takes_the_list() {
        let heard = |quest: i16| vec![Effect::FinishQuest(quest), Effect::GainExp(7000)];
        let holding = |done: &[i16]| {
            let mut player = with_quests(&done.iter().map(|&q| (q, true)).collect::<Vec<_>>());
            player.inventory.insert(
                game::InventoryType::Etc,
                1,
                game::Item::stack(INFORMANT_LIST, 1),
            );
            player
        };
        for (script, index) in [("q2216s", 0), ("q2217s", 1), ("q2218s", 2), ("q2219s", 3)] {
            let (quest, npc) = INFORMANTS[index];
            let first = NpcScript::for_quest(script, &holding(&[])).unwrap();
            assert_eq!(read_through(first), (npc, heard(quest)));
            let others: Vec<i16> = INFORMANTS
                .iter()
                .map(|&(other, _)| other)
                .filter(|&other| other != quest)
                .collect();
            let mut last = heard(quest);
            last.push(Effect::TakeItem {
                item_id: INFORMANT_LIST,
                quantity: 1,
            });
            let script = NpcScript::for_quest(script, &holding(&others)).unwrap();
            assert_eq!(read_through(script), (npc, last));
        }
        // Two still to hear: the list stays.
        let early = NpcScript::for_quest("q2219s", &holding(&[2216, 2217])).unwrap();
        assert_eq!(read_through(early), (1052006, heard(2219)));
    }

    #[test]
    fn reef_thanks_the_player_with_eight_fame() {
        let script = NpcScript::for_quest("q2228s", &with_quests(&[(2227, true)])).unwrap();
        assert_eq!(
            read_through(script),
            (REEF, vec![Effect::FinishQuest(2228), Effect::GainFame(8)])
        );
    }

    #[test]
    fn mu_young_starts_2238_and_sends_the_player_to_jr_balrog() {
        let player = with_quests(&[(2237, true)]);
        let script = NpcScript::for_quest("q2238s", &player).unwrap();
        let step = script.begin();
        assert!(step.prompt.is_none());
        assert_eq!(step.effects, [Effect::StartQuest(2238)]);
        let mut script = NpcScript::for_quest("q2245s", &player).unwrap();
        assert_eq!(script.npc_id(), MU_YOUNG);
        let step = script.begin();
        assert!(step.prompt.is_none());
        assert_eq!(
            step.effects,
            [
                Effect::FinishQuest(2245),
                Effect::StartEvent(EventKind::Tristan)
            ]
        );
        assert!(script.refused().is_some(), "another is in the room");
    }

    #[test]
    fn violetta_is_found_and_names_the_prime_minister() {
        let player = with_quests(&[(2331, false)]);
        let quest = |script| NpcScript::for_quest(script, &player).unwrap();
        let step = quest("q2332s").begin();
        assert!(step.prompt.is_none(), "the quest alert starts it silently");
        assert_eq!(step.effects, [Effect::StartQuest(events::WHERES_VIOLETTA)]);
        assert_eq!(
            read_through(quest("q2332e")),
            (
                VIOLETTA,
                vec![
                    Effect::CompleteQuest(events::WHERES_VIOLETTA),
                    Effect::GainExp(4400)
                ]
            )
        );
        let mut betrayed = quest("q2333s");
        assert!(betrayed.begin().prompt.is_some());
        let named = betrayed.respond(ScriptInput::Yes).unwrap();
        assert!(named.prompt.is_some() && named.effects.is_empty());
        let end = betrayed.respond(ScriptInput::Next).unwrap();
        assert_eq!(
            end.effects,
            [
                Effect::StartQuest(events::STORY_OF_BETRAYAL),
                Effect::BringOutMob {
                    mob_id: events::PRIME_MINISTER,
                    at: events::PRIME_MINISTER_AT,
                },
            ]
        );
        let mut declined = quest("q2333s");
        declined.begin();
        let no = declined.respond(ScriptInput::No);
        assert!(no.is_none_or(|step| step.effects.is_empty()));
        assert_eq!(
            read_through(quest("q2333e")),
            (
                VIOLETTA,
                vec![
                    Effect::CompleteQuest(events::STORY_OF_BETRAYAL),
                    Effect::GainExp(15_000)
                ]
            )
        );
        assert_eq!(
            read_through(quest("q2334s")),
            (
                VIOLETTA,
                vec![Effect::FinishQuest(2334), Effect::GainExp(1000)]
            )
        );
    }

    #[test]
    fn violetta_keeps_the_royal_seal_for_whoever_left_it() {
        let seal = Effect::GiveItem {
            item_id: ROYAL_SEAL,
            quantity: 1,
        };
        let taking_it = with_quests(&[(2331, false)]);
        let script = NpcScript::for_quest("q2342s", &taking_it).unwrap();
        assert_eq!(
            read_through(script),
            (VIOLETTA, vec![Effect::FinishQuest(2342), seal])
        );
        let mut script = NpcScript::for_quest("q2342s", &taking_it).unwrap();
        assert!(
            script.inventory_full().is_some(),
            "said with no room for it"
        );
        let mut holding = taking_it.clone();
        holding.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(ROYAL_SEAL, 1),
        );
        for player in [holding, with_quests(&[]), with_quests(&[(2331, true)])] {
            let script = NpcScript::for_quest("q2342s", &player).unwrap();
            assert_eq!(read_through(script), (VIOLETTA, Vec::new()), "only told");
        }
    }

    #[test]
    fn violetta_gives_the_security_room_key_with_eliminating_the_rest() {
        let quest = || NpcScript::for_quest("q2335s", &with_quests(&[])).unwrap();
        let mut script = quest();
        assert!(script.begin().prompt.is_some());
        let asked = script.respond(ScriptInput::Next).unwrap();
        assert!(asked.prompt.is_some() && asked.effects.is_empty());
        let luck = script.respond(ScriptInput::Yes).unwrap();
        assert!(luck.prompt.is_some() && luck.effects.is_empty());
        let end = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(end.prompt, None);
        assert_eq!(
            end.effects,
            [
                Effect::GiveItem {
                    item_id: SECURITY_ROOM_KEY,
                    quantity: 1
                },
                Effect::StartQuest(ELIMINATING_THE_REST),
            ]
        );
        assert!(quest().inventory_full().is_some(), "said with no room");
        let mut declined = quest();
        declined.begin();
        declined.respond(ScriptInput::Next);
        let no = declined.respond(ScriptInput::No);
        assert!(no.is_none_or(|step| step.effects.is_empty()));
    }

    #[test]
    fn the_domestic_secretary_sends_the_player_to_look_into_the_barrier_and_the_walls() {
        for (name, quest, decline) in [
            (
                "q2314s",
                EXPLORING_MUSHROOM_FOREST,
                Some(keys::MC_FOREST_DECLINE),
            ),
            ("q2322s", OVER_THE_CASTLE_WALLS, None),
        ] {
            let script = || NpcScript::for_quest(name, &with_quests(&[])).unwrap();
            let mut yes = script();
            assert!(
                matches!(yes.begin().prompt, Some(Prompt::YesNo { npc_id, .. }) if npc_id == DOMESTIC_SECRETARY)
            );
            let go = yes.respond(ScriptInput::Yes).unwrap();
            assert!(go.prompt.is_some() && go.effects.is_empty());
            let end = yes.respond(ScriptInput::Next).unwrap();
            assert_eq!(
                (end.prompt, end.effects),
                (None, vec![Effect::StartQuest(quest)])
            );

            let mut no = script();
            no.begin();
            let said = no.respond(ScriptInput::No).unwrap();
            assert!(said.effects.is_empty());
            let said = said.prompt.map(|prompt| match prompt {
                Prompt::Text(page) => page.text,
                other => panic!("{other:?}"),
            });
            assert_eq!(said, decline.map(text), "{name}");
        }
    }

    #[test]
    fn a_thought_is_the_players_own() {
        let mut thought = NpcScript::thought("Hmm.".into());
        let Some(Prompt::Text(page)) = thought.begin().prompt else {
            panic!("a page");
        };
        assert!(page.player_speaks && !page.has_next && page.text == "Hmm.");
        let done = thought.respond(ScriptInput::Next).unwrap();
        assert_eq!(done, Step::default());
    }

    #[test]
    fn each_job_instructor_gives_back_the_lost_memory() {
        for (quest, npc) in LOST_MEMORIES {
            let script = NpcScript::for_quest(&format!("q{quest}s"), &player(50)).unwrap();
            assert_eq!(
                read_through(script),
                (
                    npc,
                    vec![
                        Effect::FinishQuest(quest),
                        Effect::SetRecord {
                            record: 7081,
                            value: "1"
                        }
                    ]
                )
            );
        }
    }

    #[test]
    fn the_black_shadows_are_found_only_on_their_quests() {
        let on = |quests: &[(i16, QuestState)]| {
            let mut player = traveller(30, 0, false);
            for &(quest_id, state) in quests {
                let status = match state {
                    QuestState::NotStarted => continue,
                    QuestState::Started => QuestStatus::Started {
                        kills: Vec::new(),
                        info: None,
                    },
                    QuestState::Completed => QuestStatus::Completed {
                        at: std::time::SystemTime::UNIX_EPOCH,
                    },
                };
                player.quests.upsert(game::QuestRecord { quest_id, status });
            }
            player
        };
        let spotted = |record| Effect::SetRecord { record, value: "1" };
        let said = |step: Step| match step.prompt {
            Some(Prompt::Text(page)) => page.text,
            other => panic!("{other:?}"),
        };
        let nothing = |npc, player: &Player| {
            let script = NpcScript::for_npc(npc, player).unwrap();
            assert_eq!(said(script.begin()), text(keys::BLACK_SHADOW_NOTHING));
            assert_eq!(read_through(script), (npc, vec![]));
        };

        // The Pig Farm's shadow, spotted at once.
        let hunting = on(&[(BLACK_SHADOW_IN_THE_PIG_FARM, QuestState::Started)]);
        let step = NpcScript::for_npc(PIG_FARM_SHADOW, &hunting)
            .unwrap()
            .begin();
        assert_eq!(step.effects, [spotted(20731)]);
        assert_eq!(said(step), text(keys::BLACK_SHADOW_SPOTTED));
        nothing(PIG_FARM_DECOY, &hunting);
        for state in [QuestState::NotStarted, QuestState::Completed] {
            nothing(
                PIG_FARM_SHADOW,
                &on(&[(BLACK_SHADOW_IN_THE_PIG_FARM, state)]),
            );
        }

        // The library's, which sets its Blue Mushrooms on the player once read.
        let searching = on(&[(scripts::MAYBE_ITS_GRENDEL, QuestState::Started)]);
        let script = NpcScript::for_npc(LIBRARY_SHADOW, &searching).unwrap();
        assert_eq!(said(script.begin()), text(keys::BLACK_SHADOW_AMBUSH));
        assert_eq!(
            read_through(script),
            (
                LIBRARY_SHADOW,
                vec![
                    spotted(20732),
                    Effect::BringOutMobs {
                        mob_id: 2220100,
                        at: &[(117, 183), (4, 183), (-109, 183)],
                        each: 10,
                    }
                ]
            )
        );
        nothing(LIBRARY_CORNER, &searching);
        let mut found = searching.clone();
        found.quests.set_record(20732, "1");
        nothing(LIBRARY_SHADOW, &found);
        nothing(LIBRARY_SHADOW, &on(&[]));
    }

    #[test]
    fn the_puppeteer_calls_an_aran_in_trus_name_and_tru_rewards_his_fall() {
        let aran = traveller(31, 0, false);
        assert_eq!(
            read_through(NpcScript::for_quest("q21733s", &aran).unwrap()),
            (FAKE_TRU, vec![Effect::StartQuest(STRANGE_INFORMATION)])
        );
        let mut tru = NpcScript::for_quest("q21733e", &aran).unwrap();
        assert_eq!(tru.npc_id(), TRU);
        assert!(tru.begin().prompt.is_some());
        assert!(tru.respond(ScriptInput::Next).unwrap().prompt.is_some());
        assert_eq!(
            tru.respond(ScriptInput::Next).unwrap().effects,
            [
                Effect::CompleteQuest(STRANGE_INFORMATION),
                Effect::GainExp(8000),
                Effect::TeachSkill {
                    skill_id: POLEARM_MASTERY,
                    level: 0,
                    master_level: 20,
                },
            ]
        );
    }

    #[test]
    fn mu_gong_tests_an_aran_and_sends_it_after_the_shadow_knight() {
        let aran = traveller(57, 0, false);
        let test = NpcScript::for_quest("q21746s", &aran).unwrap();
        assert_eq!(test.refused, Some(text(keys::MU_GONG_ARENA_BUSY)));
        assert_eq!(
            yes_through(test),
            (
                MU_GONG,
                vec![
                    Effect::RequireEmptyMap(scripts::MU_GONGS_ARENA),
                    Effect::StartQuest(MU_GONGS_TEST),
                    Effect::WarpToPortal(scripts::MU_GONGS_ARENA, 0),
                ]
            )
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21747s", &aran).unwrap()),
            (
                MU_GONG,
                vec![Effect::StartQuest(scripts::CATCH_THE_SHADOW_KNIGHT)]
            )
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21747e", &aran).unwrap()),
            (
                MU_GONG,
                vec![
                    Effect::GainExp(16000),
                    Effect::CompleteQuest(scripts::CATCH_THE_SHADOW_KNIGHT),
                ]
            )
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21748e", &aran).unwrap()),
            (
                LILIN,
                vec![
                    Effect::CompleteQuest(STOLEN_SEAL_STONE_OF_MU_LUNG),
                    Effect::GainExp(20000),
                    Effect::TeachSkill {
                        skill_id: FINAL_CHARGE,
                        level: 0,
                        master_level: 30,
                    },
                ]
            )
        );
    }

    #[test]
    fn the_shrine_pillar_lets_one_hunting_the_shadow_knight_in() {
        let on = |quests: &[(i16, QuestState)]| {
            let mut player = traveller(57, 0, false);
            for &(quest_id, state) in quests {
                let status = match state {
                    QuestState::Started => QuestStatus::Started {
                        kills: Vec::new(),
                        info: None,
                    },
                    _ => QuestStatus::Completed {
                        at: std::time::SystemTime::UNIX_EPOCH,
                    },
                };
                player.quests.upsert(game::QuestRecord { quest_id, status });
            }
            player
        };
        let answer = |player: &Player, given: &str| {
            let mut pillar = NpcScript::for_npc(SHRINE_PILLAR, player).unwrap();
            assert_eq!(pillar.refused, Some(text(keys::SHRINE_PILLAR_BUSY)));
            assert!(matches!(
                pillar.begin().prompt,
                Some(Prompt::AskText { .. })
            ));
            pillar.respond(ScriptInput::Text(given.into())).unwrap()
        };
        let through = [
            Effect::RequireEmptyMap(scripts::SEALED_SHRINE),
            Effect::PortalSound,
            Effect::WarpToPortal(scripts::SEALED_SHRINE, 0),
        ];
        let shut = [Effect::Pink(text(keys::SHRINE_PILLAR_SHUT))];

        let hunting = on(&[(scripts::CATCH_THE_SHADOW_KNIGHT, QuestState::Started)]);
        for given in [
            "Actions speak louder than words",
            "actions speak louder than words.",
            " Actions speak louder than words . ",
        ] {
            assert_eq!(answer(&hunting, given).effects, through, "{given}");
        }
        let wrong = answer(&hunting, "Words speak louder than actions");
        assert_eq!(page_text(&wrong), text(keys::SHRINE_PILLAR_WRONG));
        assert!(wrong.effects.is_empty());
        let mut beaten = hunting.clone();
        *beaten
            .quests
            .kills_mut(scripts::CATCH_THE_SHADOW_KNIGHT)
            .unwrap() = vec![1];
        assert_eq!(
            answer(&beaten, "Actions speak louder than words").effects,
            shut
        );
        assert_eq!(
            answer(&on(&[]), "Actions speak louder than words").effects,
            shut
        );
    }

    #[test]
    fn an_aran_goes_back_to_ellin_and_athena_keeps_a_letter_for_it() {
        let aran = traveller(63, 0, false);
        assert_eq!(
            yes_through(NpcScript::for_quest("q21749s", &aran).unwrap()),
            (TRU, vec![Effect::StartQuest(BACK_TO_THE_PAST)])
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21750e", &aran).unwrap()),
            (
                ATHENA_PIERCE_OF_ELLIN,
                vec![Effect::CompleteQuest(SHE_REMEMBERS_ME)]
            )
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21753s", &aran).unwrap()),
            (
                ATHENA_PIERCE_OF_ELLIN,
                vec![Effect::StartQuest(LOST_SEAL_STONE)]
            )
        );

        let athena = NpcScript::for_quest("q21754s", &aran).unwrap();
        assert_eq!(athena.inventory_full, Some(text(keys::ATHENA_LETTER_FULL)));
        assert_eq!(
            yes_through(athena),
            (
                ATHENA_PIERCE,
                vec![
                    Effect::GiveItem {
                        item_id: ATHENAS_LETTER,
                        quantity: 1,
                    },
                    Effect::StartQuest(INFO_ON_ANOTHER_SEAL_STONE),
                ]
            )
        );
        let mut holding = aran.clone();
        holding.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(ATHENAS_LETTER, 1),
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21754s", &holding).unwrap()).1,
            [Effect::StartQuest(INFO_ON_ANOTHER_SEAL_STONE)]
        );

        assert_eq!(
            yes_through(NpcScript::for_quest("q21757e", &aran).unwrap()),
            (
                NEINHEART,
                vec![
                    Effect::TakeItem {
                        item_id: LETTER_TO_THE_EMPRESS,
                        quantity: 1,
                    },
                    Effect::GainExp(1000),
                    Effect::CompleteQuest(NEINHEART_THE_STRATEGIST),
                ]
            )
        );
    }

    #[test]
    fn an_aran_fills_the_gaps_from_lilins_polearm_to_johns_box() {
        let aran = traveller(10, 0, false);
        let lilin = NpcScript::for_quest("q21700s", &aran).unwrap();
        assert_eq!(
            lilin.inventory_full,
            Some(text(keys::LILIN_NEW_BEGINNINGS_FULL))
        );
        assert_eq!(
            yes_through(lilin),
            (
                LILIN,
                vec![
                    Effect::GiveItem {
                        item_id: PRACTICE_POLEARM,
                        quantity: 1,
                    },
                    Effect::StartQuest(NEW_BEGINNINGS),
                ]
            )
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21703s", &aran).unwrap()),
            (PUO, vec![Effect::StartQuest(TRAIN_OR_DIE_3)])
        );
        let mut puo = NpcScript::for_quest("q21703s", &aran).unwrap();
        puo.begin();
        puo.respond(ScriptInput::Next).unwrap();
        let declined = puo.respond(ScriptInput::No).unwrap();
        assert_eq!(page_text(&declined), text(keys::PUO_TARGET_PIGS_DECLINED));
        assert!(declined.effects.is_empty());

        let puo = NpcScript::for_quest("q21703e", &aran).unwrap();
        assert_eq!(puo.inventory_full, Some(text(keys::PUO_COMBO_ABILITY_FULL)));
        assert_eq!(
            yes_through(puo),
            (
                PUO,
                vec![
                    Effect::GiveItem {
                        item_id: SPECIAL_RIEN_RED_POTION,
                        quantity: 30,
                    },
                    Effect::GiveItem {
                        item_id: SPECIAL_RIEN_BLUE_POTION,
                        quantity: 30,
                    },
                    Effect::GainExp(2800),
                    Effect::TeachSkill {
                        skill_id: COMBO_ABILITY,
                        level: 0,
                        master_level: 10,
                    },
                    Effect::CompleteQuest(TRAIN_OR_DIE_3),
                ]
            )
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21704s", &aran).unwrap()),
            (
                LILIN,
                vec![Effect::FinishQuest(BABY_STEPS), Effect::GainExp(500)]
            )
        );

        for (name, npc, quest) in [
            ("q21712s", BRUCE, FIRST_ASSIGNMENT_DONE),
            ("q21716s", ROWEN, SECOND_ASSIGNMENT_DONE),
        ] {
            assert_eq!(
                yes_through(NpcScript::for_quest(name, &aran).unwrap()),
                (npc, vec![Effect::StartQuest(quest)]),
                "{name}"
            );
        }
        assert_eq!(
            yes_through(NpcScript::for_quest("q21729s", &aran).unwrap()),
            (ILJI, vec![Effect::StartQuest(FOURTH_ASSIGNMENT_DONE)])
        );

        assert_eq!(
            yes_through(NpcScript::for_quest("q21720e", &aran).unwrap()),
            (
                LILIN,
                vec![
                    Effect::GainExp(3900),
                    Effect::TeachSkill {
                        skill_id: POLEARM_BOOSTER,
                        level: 0,
                        master_level: 20,
                    },
                    Effect::CompleteQuest(PUPPETEERS_WARNING),
                ]
            )
        );

        assert_eq!(
            yes_through(NpcScript::for_quest("q21766s", &aran).unwrap()),
            (TEO, vec![Effect::StartQuest(SUSPICIOUS_JOHN)])
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21766e", &aran).unwrap()),
            (
                TEO,
                vec![Effect::GainExp(200), Effect::CompleteQuest(SUSPICIOUS_JOHN)]
            )
        );
        let bx = NpcScript::for_quest("q21767s", &aran).unwrap();
        assert_eq!(bx.inventory_full, Some(text(keys::JOHNS_BOX_FULL)));
        assert_eq!(
            read_through(bx),
            (
                JOHNS_BOX,
                vec![
                    Effect::GiveItem {
                        item_id: ARTHRITIS_MEDICINE,
                        quantity: 1,
                    },
                    Effect::StartQuest(WOODEN_BOX),
                ]
            )
        );
        let mut holding = aran.clone();
        holding.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(ARTHRITIS_MEDICINE, 1),
        );
        assert_eq!(
            read_through(NpcScript::for_quest("q21767s", &holding).unwrap()).1,
            [Effect::StartQuest(WOODEN_BOX)]
        );
    }

    #[test]
    fn the_shadow_brings_an_aran_to_francis_who_warns_it_and_sends_it_back() {
        let aran = traveller(30, 0, false);
        assert_eq!(
            yes_through(NpcScript::for_quest("q21719s", &aran).unwrap()),
            (
                INVITATION_SHADOW,
                vec![
                    Effect::SetRecord {
                        record: PUPPETEERS_WARNING_RECORD,
                        value: "0",
                    },
                    Effect::StartQuest(PUPPETEERS_INVITATION),
                    Effect::WarpToPortal(PUPPETEERS_CAVE_MAP, 0),
                ]
            )
        );

        let out = Effect::WarpToPortal(DEEP_FOREST_II, DEEP_FOREST_II_IN00);
        let stray = NpcScript::for_npc(CAVE_FRANCIS, &aran).unwrap();
        assert_eq!(
            page_text(&stray.clone().begin()),
            text(keys::CAVE_FRANCIS_OUT)
        );
        assert_eq!(read_through(stray), (CAVE_FRANCIS, vec![out.clone()]));

        let mut invited = aran.clone();
        invited.quests.upsert(game::QuestRecord {
            quest_id: PUPPETEERS_INVITATION,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        assert_eq!(
            yes_through(NpcScript::for_npc(CAVE_FRANCIS, &invited).unwrap()),
            (
                CAVE_FRANCIS,
                vec![
                    Effect::SetRecord {
                        record: PUPPETEERS_WARNING_RECORD,
                        value: "0",
                    },
                    Effect::CompleteQuest(PUPPETEERS_INVITATION),
                    out,
                ]
            )
        );
    }

    #[test]
    fn an_aran_earns_a_wolf_from_pucci_scadur_the_pack_and_maha() {
        let aran = traveller(200, 0, false);
        assert_eq!(
            yes_through(NpcScript::for_quest("q21600s", &aran).unwrap()),
            (PUCCI, vec![Effect::StartQuest(PUCCIS_REQUEST)])
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21604s", &aran).unwrap()),
            (SCADUR, vec![Effect::StartQuest(WOLF_MOUNT)])
        );

        let into_the_room = [
            Effect::RequireEmptyMap(scripts::WOLVES_TEST_ROOM),
            Effect::WarpToPortal(scripts::WOLVES_TEST_ROOM, 0),
        ];
        let test = NpcScript::for_quest("q21613s", &aran).unwrap();
        assert_eq!(test.refused, Some(text(keys::WOLF_TEST_BUSY)));
        assert_eq!(
            yes_through(test),
            (
                PACK_WOLF,
                vec![
                    into_the_room[0].clone(),
                    Effect::StartQuest(scripts::THE_WOLVES_TEST),
                    into_the_room[1].clone(),
                ]
            )
        );
        // On the test and yet to pass it, the wolf lets the Aran back in;
        // otherwise it has nothing to say.
        assert!(NpcScript::for_npc(PACK_WOLF, &aran).is_none());
        let mut testing = aran.clone();
        testing.quests.upsert(game::QuestRecord {
            quest_id: scripts::THE_WOLVES_TEST,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        let again = NpcScript::for_npc(PACK_WOLF, &testing).unwrap();
        assert_eq!(again.refused, Some(text(keys::WOLF_TEST_BUSY)));
        assert_eq!(yes_through(again), (PACK_WOLF, into_the_room.to_vec()));
        testing
            .quests
            .set_record(scripts::WOLVES_TEST_RECORD, scripts::WOLF_RUN_DONE);
        assert!(NpcScript::for_npc(PACK_WOLF, &testing).is_none());

        assert_eq!(
            yes_through(NpcScript::for_quest("q21618s", &aran).unwrap()),
            (MAHA, vec![Effect::StartQuest(THE_WOLFS_EVOLUTION)])
        );
        // The wolf has to be out of its slot, in the Equip inventory.
        let riding = NpcScript::for_quest("q21618e", &aran).unwrap();
        assert_eq!(
            page_text(&riding.clone().begin()),
            text(keys::MAHA_WOLF_EVOLUTION_UNEQUIP)
        );
        assert_eq!(read_through(riding), (MAHA, Vec::new()));
        let mut holding = aran.clone();
        holding.inventory.insert(
            game::InventoryType::Equip,
            1,
            game::Item::stack(WOLF_MOUNT_ITEM, 1),
        );
        let evolution = NpcScript::for_quest("q21618e", &holding).unwrap();
        assert!(evolution.inventory_full.is_some());
        assert_eq!(
            read_through(evolution),
            (
                MAHA,
                vec![
                    Effect::TakeItem {
                        item_id: WOLF_MOUNT_ITEM,
                        quantity: 1,
                    },
                    Effect::GiveItem {
                        item_id: EVOLVED_WOLF_MOUNT,
                        quantity: 1,
                    },
                    Effect::CompleteQuest(THE_WOLFS_EVOLUTION),
                ]
            )
        );

        for room in [scripts::FIELD_OF_WOLVES, scripts::WOLVES_TEST_ROOM] {
            assert_eq!(time_limit(room), Some(3 * 60));
            assert!(resets_when_empty(room));
        }
    }

    #[test]
    fn russellons_pill_poisons_its_taker_and_yulete_trades_marbles() {
        // The desk has a pill only for one on 3314, holding none and not
        // under one.
        let mut alchemist = traveller(70, 0, false);
        assert!(NpcScript::for_npc(PILL_DESK, &alchemist).is_none());
        alchemist.quests.upsert(game::QuestRecord {
            quest_id: LIFE_ALCHEMY,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        let desk = NpcScript::for_npc(PILL_DESK, &alchemist).unwrap();
        assert!(desk.player_speaks && desk.inventory_full.is_some());
        let pill = Effect::GiveItem {
            item_id: RUSSELLONS_PILLS,
            quantity: 1,
        };
        assert_eq!(yes_through(desk), (PILL_DESK, vec![pill]));
        let mut holding = alchemist.clone();
        holding.inventory.insert(
            game::InventoryType::Use,
            1,
            game::Item::stack(RUSSELLONS_PILLS, 1),
        );
        assert!(NpcScript::for_npc(PILL_DESK, &holding).is_none());

        // Russellon wants to see the pill's work before ending the quest.
        let unwell = NpcScript::for_quest("q3314e", &alchemist).unwrap();
        assert_eq!(
            page_text(&unwell.clone().begin()),
            text(keys::RUSSELLON_PILL_NOT_TAKEN)
        );
        assert_eq!(read_through(unwell), (RUSSELLON, Vec::new()));
        let mut poisoned = alchemist.clone();
        poisoned.buffs.give(game::Buff::new(
            PILL_POISON,
            game::BuffStat::Poison,
            50,
            std::time::Instant::now(),
            std::time::Duration::from_secs(300),
        ));
        assert!(NpcScript::for_npc(PILL_DESK, &poisoned).is_none());
        let done = NpcScript::for_quest("q3314e", &poisoned).unwrap();
        assert!(done.inventory_full.is_some());
        let (RUSSELLON, effects) = read_through(done) else {
            panic!();
        };
        let [Effect::GiveItem {
            item_id,
            quantity: 20,
        }, Effect::GainExp(LIFE_ALCHEMY_EXP), Effect::CompleteQuest(LIFE_ALCHEMY), Effect::ApplyItemBuff(ALL_CURE_POTION)] =
            effects[..]
        else {
            panic!("{effects:?}");
        };
        assert!(LIFE_ALCHEMY_POTIONS.iter().any(|&(id, _)| id == item_id));

        // Yulete: Horus' Eye for 25 of each marble, a Rock of Wisdom for
        // 10, or a word to bring more.
        let with_marbles = |count: i16| {
            let mut player = traveller(70, 0, false);
            for (slot, marble) in [(1, ZENUMIST_MARBLE), (2, ALCADNO_MARBLE)] {
                player.inventory.insert(
                    game::InventoryType::Etc,
                    slot,
                    game::Item::stack(marble, count),
                );
            }
            player
        };
        let trade = |count: i16, item_id: i32| {
            vec![
                Effect::TakeItem {
                    item_id: ZENUMIST_MARBLE,
                    quantity: count,
                },
                Effect::TakeItem {
                    item_id: ALCADNO_MARBLE,
                    quantity: count,
                },
                Effect::GiveItem {
                    item_id,
                    quantity: 1,
                },
                Effect::CompleteQuest(YULETES_REWARD),
            ]
        };
        let few = NpcScript::for_quest("q3382e", &with_marbles(9)).unwrap();
        assert_eq!(
            page_text(&few.clone().begin()),
            text(keys::YULETE_MARBLES_MORE)
        );
        assert_eq!(read_through(few), (LAB_YULETE, Vec::new()));
        let rock = NpcScript::for_quest("q3382e", &with_marbles(24)).unwrap();
        assert!(rock.inventory_full.is_some());
        assert_eq!(read_through(rock), (LAB_YULETE, trade(10, ROCK_OF_WISDOM)));
        assert_eq!(
            read_through(NpcScript::for_quest("q3382e", &with_marbles(25)).unwrap()),
            (LAB_YULETE, trade(25, HORUS_EYE))
        );
        // Wearing the eye already, the player gets the rock.
        let mut wearing = with_marbles(25);
        wearing.inventory.insert(
            game::InventoryType::Equipped,
            -17,
            game::Item::stack(HORUS_EYE, 1),
        );
        assert_eq!(
            read_through(NpcScript::for_quest("q3382e", &wearing).unwrap()),
            (LAB_YULETE, trade(10, ROCK_OF_WISDOM))
        );
    }

    #[test]
    fn karcasa_hands_the_queens_silk_only_to_tigun() {
        // Undisguised, Karcasa sends the player off at either end.
        let stranger = traveller(29, 0, false);
        for name in ["q3941s", "q3941e"] {
            let script = NpcScript::for_quest(name, &stranger).unwrap();
            assert_eq!(read_through(script), (KARCASA, vec![]));
        }
        // Disguised as Tigun, the player is let in on it, then given the silk.
        let mut tigun = stranger.clone();
        let now = std::time::Instant::now();
        let lasts = std::time::Duration::from_secs(60);
        let morph = game::buff::Buff::new(-2210005, game::buff::BuffStat::Morph, 1, now, lasts);
        tigun.buffs.give(morph);
        assert_eq!(
            yes_through(NpcScript::for_quest("q3941s", &tigun).unwrap()),
            (KARCASA, vec![Effect::StartQuest(QUEENS_SILK_QUEST)])
        );
        let end = NpcScript::for_quest("q3941e", &tigun).unwrap();
        assert!(end.inventory_full.is_some());
        assert_eq!(
            read_through(end),
            (
                KARCASA,
                vec![
                    Effect::GiveItem {
                        item_id: QUEENS_SILK,
                        quantity: 1
                    },
                    Effect::CompleteQuest(QUEENS_SILK_QUEST)
                ]
            )
        );
    }

    #[test]
    fn adonis_tells_of_the_lost_spirits_and_swallow_finds_a_new_seed() {
        let traveller = traveller(40, 0, false);
        assert_eq!(
            yes_through(NpcScript::for_quest("q8255s", &traveller).unwrap()),
            (
                ADONIS,
                vec![Effect::FinishQuest(LOST_SPIRITS), Effect::GainExp(500)]
            )
        );

        // Swallow has a new seed only for one on 3609.
        assert!(NpcScript::for_npc(SWALLOW, &traveller).is_none());
        let mut seedless = traveller.clone();
        seedless.quests.upsert(game::QuestRecord {
            quest_id: THE_SEED_SWALLOW_LOST,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        let swallow = NpcScript::for_npc(SWALLOW, &seedless).unwrap();
        assert!(swallow.inventory_full.is_some());
        assert_eq!(
            read_through(swallow),
            (
                SWALLOW,
                vec![
                    Effect::GiveItem {
                        item_id: SWALLOWS_SEED,
                        quantity: 1
                    },
                    Effect::CompleteQuest(THE_SEED_SWALLOW_LOST)
                ]
            )
        );
    }

    #[test]
    fn pia_takes_the_rainbow_snail_shell_and_plays_its_effect() {
        let traveller = traveller(30, 0, false);
        let without = NpcScript::for_quest("q2156e", &traveller).unwrap();
        assert_eq!(
            page_text(&without.clone().begin()),
            text(keys::PIA_SHELL_MISSING)
        );
        assert_eq!(read_through(without), (PIA, Vec::new()));

        let mut holding = traveller.clone();
        holding.inventory.insert(
            game::InventoryType::Use,
            1,
            game::Item::stack(RAINBOW_SNAIL_SHELL, 1),
        );
        let with = NpcScript::for_quest("q2156e", &holding).unwrap();
        assert_eq!(page_text(&with.clone().begin()), text(keys::PIA_SHELL_WISH));
        assert_eq!(
            read_through(with),
            (
                PIA,
                vec![
                    Effect::TakeItem {
                        item_id: RAINBOW_SNAIL_SHELL,
                        quantity: 1
                    },
                    Effect::GainExp(7500),
                    Effect::GainMeso(30000),
                    Effect::GainFame(3),
                    Effect::CompleteQuest(RAINBOW_SHELL_QUEST),
                    Effect::NpcSpecialAction {
                        npc_id: PIA,
                        action: "act2156"
                    },
                ]
            )
        );
    }

    /// Every effect of `script` read to its end, answering yes to each ask.
    fn yes_through(mut script: NpcScript) -> (i32, Vec<Effect>) {
        let npc = script.npc_id();
        let mut step = script.begin();
        let mut effects = Vec::new();
        loop {
            effects.append(&mut step.effects);
            let input = match kind(&step) {
                Some(1) => ScriptInput::Yes,
                Some(_) => ScriptInput::Next,
                None => return (npc, effects),
            };
            step = script.respond(input).unwrap();
        }
    }

    #[test]
    fn tru_sends_an_aran_after_the_puppeteer_then_to_lilin_and_orbis() {
        let aran = traveller(37, 0, false);
        assert_eq!(
            yes_through(NpcScript::for_quest("q21734s", &aran).unwrap()),
            (TRU, vec![Effect::StartQuest(TRACES_OF_THE_PUPPETEER)])
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21734e", &aran).unwrap()),
            (
                TRU,
                vec![
                    Effect::CompleteQuest(TRACES_OF_THE_PUPPETEER),
                    Effect::GainExp(12500),
                    Effect::TeachSkill {
                        skill_id: COMBO_DRAIN,
                        level: 0,
                        master_level: 20,
                    },
                ]
            )
        );

        let stone = NpcScript::for_quest("q21735s", &aran).unwrap();
        assert!(stone.inventory_full.is_some());
        assert_eq!(
            yes_through(stone),
            (
                TRU,
                vec![
                    Effect::GiveItem {
                        item_id: VICTORIA_SEAL_STONE,
                        quantity: 1
                    },
                    Effect::StartQuest(SEAL_STONE_OF_VICTORIA),
                ]
            )
        );
        let mut holder = aran.clone();
        holder.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(VICTORIA_SEAL_STONE, 1),
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21735s", &holder).unwrap()).1,
            [Effect::StartQuest(SEAL_STONE_OF_VICTORIA)]
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21735e", &holder).unwrap()),
            (
                LILIN,
                vec![
                    Effect::TakeItem {
                        item_id: VICTORIA_SEAL_STONE,
                        quantity: 1
                    },
                    Effect::GainExp(6037),
                    Effect::CompleteQuest(SEAL_STONE_OF_VICTORIA),
                ]
            )
        );

        let orbis = || {
            let mut tru = NpcScript::for_quest("q21736s", &aran).unwrap();
            tru.begin();
            assert_eq!(kind(&tru.respond(ScriptInput::Next).unwrap()), Some(1));
            tru
        };
        assert_eq!(
            orbis().respond(ScriptInput::Yes).unwrap().effects,
            [Effect::StartQuest(OFF_TO_ORBIS)]
        );
        assert!(orbis().respond(ScriptInput::No).unwrap().effects.is_empty());
    }

    #[test]
    fn an_aran_goes_from_the_sealed_garden_to_mr_dos_black_scroll() {
        let aran = traveller(45, 0, false);
        let quest = |name| yes_through(NpcScript::for_quest(name, &aran).unwrap());
        assert_eq!(
            quest("q21738s"),
            (SPIRUNA, vec![Effect::StartQuest(SEALED_GARDEN)])
        );
        let mut spiruna = NpcScript::for_quest("q21738s", &aran).unwrap();
        spiruna.begin();
        assert_eq!(kind(&spiruna.respond(ScriptInput::Next).unwrap()), Some(1));
        assert!(spiruna.respond(ScriptInput::No).unwrap().effects.is_empty());
        assert_eq!(
            quest("q21739e"),
            (
                SPIRUNA,
                vec![
                    Effect::CompleteQuest(TO_THE_SEALED_GARDEN),
                    Effect::GainExp(29500)
                ]
            )
        );
        assert_eq!(
            quest("q21740s"),
            (TRU, vec![Effect::StartQuest(ORBIS_STONE_STOLEN)])
        );
        assert_eq!(
            quest("q21740e"),
            (
                LILIN,
                vec![
                    Effect::CompleteQuest(ORBIS_STONE_STOLEN),
                    Effect::TeachSkill {
                        skill_id: COMBO_SMASH,
                        level: 0,
                        master_level: 20,
                    },
                ]
            )
        );
        assert_eq!(
            quest("q21741s"),
            (TRU, vec![Effect::StartQuest(INVESTIGATING_MU_LUNG)])
        );

        let scroll = NpcScript::for_quest("q21742s", &aran).unwrap();
        assert!(scroll.inventory_full.is_some());
        assert_eq!(
            yes_through(scroll),
            (
                MR_DO,
                vec![
                    Effect::GiveItem {
                        item_id: HANGING_SCROLL,
                        quantity: 1
                    },
                    Effect::StartQuest(THE_BLACK_SCROLL),
                ]
            )
        );
        let mut holder = aran.clone();
        holder.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(HANGING_SCROLL, 1),
        );
        assert_eq!(
            yes_through(NpcScript::for_quest("q21742s", &holder).unwrap()).1,
            [Effect::StartQuest(THE_BLACK_SCROLL)]
        );
        assert_eq!(
            quest("q21742e"),
            (
                MR_DO,
                vec![
                    Effect::TakeItem {
                        item_id: HANGING_SCROLL,
                        quantity: 1
                    },
                    Effect::GainExp(10000),
                    Effect::CompleteQuest(THE_BLACK_SCROLL),
                ]
            )
        );
    }

    #[test]
    fn the_passages_statues_lead_into_the_puppeteers_arena_and_out() {
        let aran = traveller(37, 0, false);
        let mut end = NpcScript::for_npc(PASSAGE_END_STATUE, &aran).unwrap();
        assert!(end.refused.is_some());
        assert_eq!(kind(&end.begin()), Some(1));
        assert_eq!(
            end.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::RequireEmptyMap(scripts::PUPPETEERS_ARENA),
                Effect::WarpToPortal(scripts::PUPPETEERS_ARENA, 0),
            ]
        );
        let mut exit = NpcScript::for_npc(PASSAGE_EXIT_STATUE, &aran).unwrap();
        assert_eq!(kind(&exit.begin()), Some(1));
        assert_eq!(
            exit.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::WarpToPortal(PASSAGE_WAY_OUT, 2)]
        );
    }

    #[test]
    fn bob_hands_out_a_timers_egg_and_takes_it_back_grown() {
        let offer = |player: &Player| {
            let mut bob = NpcScript::for_quest("q3250s", player).unwrap();
            assert_eq!(bob.npc_id(), GHOSTHUNTER_BOB);
            bob.begin();
            bob.respond(ScriptInput::Next).unwrap();
            assert_eq!(kind(&bob.respond(ScriptInput::Next).unwrap()), Some(1));
            bob
        };
        let mut bob = offer(&player(70));
        assert_eq!(
            bob.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::GiveItem {
                    item_id: TIMERS_EGG,
                    quantity: 1
                },
                Effect::StartQuest(A_CUTE_BABY_BIRD),
            ]
        );
        // One already held is not given again.
        let mut holder = player(70);
        holder.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(TIMERS_EGG, 1),
        );
        assert_eq!(
            offer(&holder).respond(ScriptInput::Yes).unwrap().effects,
            [Effect::StartQuest(A_CUTE_BABY_BIRD)]
        );
        assert!(offer(&player(70))
            .respond(ScriptInput::No)
            .unwrap()
            .effects
            .is_empty());

        assert_eq!(
            read_through(NpcScript::for_quest("q3250e", &holder).unwrap()),
            (
                GHOSTHUNTER_BOB,
                vec![
                    Effect::TakeItem {
                        item_id: TIMERS_EGG,
                        quantity: 1
                    },
                    Effect::CompleteQuest(A_CUTE_BABY_BIRD),
                ]
            )
        );
    }

    #[test]
    fn the_puppeteers_door_lets_his_hunters_through_on_the_password() {
        let on = |quests: &[(i16, QuestState)]| {
            let mut player = traveller(30, 0, false);
            for &(quest_id, state) in quests {
                let status = match state {
                    QuestState::Started => QuestStatus::Started {
                        kills: Vec::new(),
                        info: None,
                    },
                    _ => QuestStatus::Completed {
                        at: std::time::SystemTime::UNIX_EPOCH,
                    },
                };
                player.quests.upsert(game::QuestRecord { quest_id, status });
            }
            player
        };
        let answer = |player: &Player, given: &str| {
            let mut door = NpcScript::for_npc(PUPPETEER_DOOR, player).unwrap();
            assert!(matches!(door.begin().prompt, Some(Prompt::AskText { .. })));
            door.respond(ScriptInput::Text(given.into())).unwrap()
        };
        let through = [Effect::PortalSound, Effect::WarpToPortal(FRANCIS_ROOM, 1)];

        for quest in [20730, 21731] {
            let hunting = on(&[(quest, QuestState::Started)]);
            for given in [
                "Francis is a genius Puppeteer!",
                "francis is a genius puppeteer",
                " Francis is a genius Puppeteer ! ",
            ] {
                assert_eq!(answer(&hunting, given).effects, through, "{given}");
            }
            let wrong = answer(&hunting, "Puppeteer is a genius Francis!");
            assert_eq!(page_text(&wrong), text(keys::PUPPETEER_DOOR_WRONG));
            assert!(wrong.effects.is_empty());
            // Beaten, the way stays shut.
            let mut beaten = hunting.clone();
            *beaten.quests.kills_mut(quest).unwrap() = vec![1];
            assert_eq!(
                answer(&beaten, "Francis is a genius Puppeteer!").effects,
                [Effect::Pink(text(keys::PUPPETEER_DOOR_SHUT))]
            );
        }
        assert_eq!(
            answer(&on(&[]), "Francis is a genius Puppeteer!").effects,
            [Effect::Pink(text(keys::PUPPETEER_DOOR_SHUT))]
        );

        // An Aran on The Puppeteer's Cave finds the way barred.
        let searching = on(&[(PUPPETEERS_CAVE, QuestState::Started)]);
        let step = NpcScript::for_npc(PUPPETEER_DOOR, &searching)
            .unwrap()
            .begin();
        assert_eq!(page_text(&step), text(keys::PUPPETEER_DOOR_SEARCHED));
        assert_eq!(
            step.effects,
            [Effect::SetRecord {
                record: 21761,
                value: "0"
            }]
        );

        // Francis sends the intruder on into the hideout, if no one is in.
        let francis = NpcScript::for_npc(FRANCIS, &on(&[])).unwrap();
        assert_eq!(page_text(&francis.begin()), text(keys::FRANCIS_INTRUDER));
        assert_eq!(
            read_through(francis),
            (
                FRANCIS,
                vec![
                    Effect::RequireEmptyMap(scripts::PUPPETEERS_HIDEOUT),
                    Effect::WarpToPortal(scripts::PUPPETEERS_HIDEOUT, 1),
                ]
            )
        );
    }

    #[test]
    fn each_job_instructor_writes_a_letter_to_the_mushking_empire() {
        let said = |step: Step| match step.prompt {
            Some(Prompt::Text(page)) => (page.text, page.player_speaks),
            other => panic!("{other:?}"),
        };
        let letter = Effect::GiveItem {
            item_id: KINGDOM_LETTER,
            quantity: 1,
        };
        for (quest, instructor) in KINGDOM_LETTERS {
            let name = format!("q{quest}s");
            let script = || NpcScript::for_quest(&name, &player(50)).unwrap();
            let mut yes = script();
            assert!(
                matches!(yes.begin().prompt, Some(Prompt::YesNo { npc_id, .. }) if npc_id == instructor)
            );
            let kingdom = yes.respond(ScriptInput::Yes).unwrap();
            assert!(kingdom.effects.is_empty());
            let written = yes.respond(ScriptInput::Next).unwrap();
            assert_eq!(written.effects, [letter.clone(), Effect::StartQuest(quest)]);
            let ask = yes.respond(ScriptInput::Next).unwrap();
            assert!(matches!(ask.prompt, Some(Prompt::YesNo { .. })) && ask.effects.is_empty());
            let go = yes.respond(ScriptInput::Yes).unwrap();
            assert_eq!(
                (go.prompt, go.effects),
                (None, vec![Effect::Warp(MUSHROOM_OPENING)])
            );
            let full = script().inventory_full().unwrap();
            assert_eq!(said(full).0, text(keys::MC_LETTER_FULL));

            let mut route = script();
            route.begin();
            route.respond(ScriptInput::Yes);
            route.respond(ScriptInput::Next);
            route.respond(ScriptInput::Next);
            let told = route.respond(ScriptInput::No).unwrap();
            assert!(told.effects.is_empty());
            assert_eq!(said(told).0, text(keys::MC_LETTER_ROUTE));

            let mut later = script();
            later.begin();
            let no = later.respond(ScriptInput::No).unwrap();
            assert!(no.effects.is_empty());
            assert_eq!(said(no).0, text(keys::MC_LETTER_LATER));
        }

        // One still holding a letter from a quest given up keeps it.
        let mut holding = player(50);
        holding
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(KINGDOM_LETTER, 1));
        let mut script = NpcScript::for_quest("q2300s", &holding).unwrap();
        script.begin();
        script.respond(ScriptInput::Yes);
        let written = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(written.effects, [Effect::StartQuest(2300)]);

        // Without room the instructor only says so.
        let mut full = player(50);
        for slot in 1..=full.inventory.slot_limit(InventoryType::Etc) as i16 {
            full.inventory
                .insert(InventoryType::Etc, slot, game::Item::stack(4_000_000, 1));
        }
        let mut script = NpcScript::for_quest("q2300s", &full).unwrap();
        script.begin();
        let told = script.respond(ScriptInput::Yes).unwrap();
        assert!(told.effects.is_empty());
        assert_eq!(said(told).0, text(keys::MC_LETTER_FULL));
        assert_eq!(script.respond(ScriptInput::Next), Some(Step::default()));
    }

    #[test]
    fn the_head_patrol_officer_reads_the_letter_and_the_player_answers() {
        for (quest, _) in KINGDOM_LETTERS {
            let mut script = NpcScript::for_quest(&format!("q{quest}e"), &player(50)).unwrap();
            let speakers: Vec<bool> = [
                script.begin(),
                script.respond(ScriptInput::Next).unwrap(),
                script.respond(ScriptInput::Next).unwrap(),
            ]
            .into_iter()
            .map(|step| match step.prompt {
                Some(Prompt::Text(page)) => {
                    assert_eq!(page.npc_id, HEAD_SECURITY_OFFICER);
                    assert!(step.effects.is_empty());
                    page.player_speaks
                }
                other => panic!("{other:?}"),
            })
            .collect();
            assert_eq!(speakers, [false, true, false]);
            let end = script.respond(ScriptInput::Next).unwrap();
            assert_eq!(
                (end.prompt, end.effects),
                (
                    None,
                    vec![
                        Effect::TakeItem {
                            item_id: KINGDOM_LETTER,
                            quantity: 1
                        },
                        Effect::CompleteQuest(quest),
                        Effect::GainExp(1200),
                    ]
                )
            );
        }
        assert_eq!(NpcScript::for_quest("q2305e", &player(50)), None, "Cygnus");
    }

    #[test]
    fn james_is_found_and_then_escapes_in_the_disguise() {
        let speakers = |script: &mut NpcScript, pages: usize| -> Vec<bool> {
            (0..pages)
                .map(|page| {
                    let step = if page == 0 {
                        script.begin()
                    } else {
                        script.respond(ScriptInput::Next).unwrap()
                    };
                    assert!(step.effects.is_empty());
                    match step.prompt {
                        Some(Prompt::Text(page)) => {
                            assert_eq!(page.npc_id, JAMES);
                            page.player_speaks
                        }
                        other => panic!("{other:?}"),
                    }
                })
                .collect()
        };
        let mut found = NpcScript::for_quest("q2325e", &player(50)).unwrap();
        assert_eq!(speakers(&mut found, 3), [false, true, false]);
        assert_eq!(
            found.respond(ScriptInput::Next).unwrap().effects,
            [Effect::CompleteQuest(JAMES_FOUND), Effect::GainExp(6000)]
        );

        let mut escapes = NpcScript::for_quest("q2327s", &player(50)).unwrap();
        assert_eq!(speakers(&mut escapes, 2), [false, false]);
        assert_eq!(
            escapes.respond(ScriptInput::Next).unwrap().effects,
            [Effect::StartQuest(JAMES_ESCAPES)]
        );
    }

    #[test]
    fn scarrs_gives_a_spare_killer_mushroom_spore() {
        let script = || NpcScript::for_quest("q2338s", &player(50)).unwrap();
        assert_eq!(
            read_through(script()),
            (
                SCARRS,
                vec![
                    Effect::GiveItem {
                        item_id: KILLER_MUSHROOM_SPORE,
                        quantity: 1
                    },
                    Effect::FinishQuest(SPORES_AGAIN),
                ]
            )
        );
        let Some(Prompt::Text(full)) = script().inventory_full().unwrap().prompt else {
            panic!("told");
        };
        assert_eq!(full.text, text(keys::SCARRS_SPORE_FULL));
    }

    #[test]
    fn the_rememberer_takes_the_recording_charms_and_points_to_karcasa() {
        let script = |name| NpcScript::for_quest(name, &player(50)).unwrap();
        assert_eq!(
            read_through(script("q2251e")),
            (
                REMEMBERER,
                vec![
                    Effect::TakeItem {
                        item_id: RECORDING_CHARM,
                        quantity: 20
                    },
                    Effect::CompleteQuest(ZOMBIE_SIGNAL_3),
                    Effect::GainExp(8000),
                ]
            )
        );
        assert_eq!(
            read_through(script("q2254s")),
            (REMEMBERER, vec![Effect::StartQuest(KARCASA_OF_THE_DESERT)])
        );
    }

    #[test]
    fn the_camel_cab_talks_about_the_mushroom_castle_out_of_the_meerkats_hearing() {
        let at = |map| {
            let mut player = player(50);
            player.stats.map_id = map;
            player
        };
        let script = |name, map| NpcScript::for_quest(name, &at(map)).unwrap();
        for (name, effect) in [
            ("q2257e", Effect::CompleteQuest(KARCASA_RELENTS)),
            ("q2258e", Effect::CompleteQuest(MEERKATS)),
            ("q2259s", Effect::StartQuest(SCORPIONS)),
            ("q2260s", Effect::StartQuest(TO_THE_CASTLE)),
            ("q2260e", Effect::CompleteQuest(TO_THE_CASTLE)),
        ] {
            assert_eq!(
                read_through(script(name, OUTSIDE_ARIANT)),
                (CAMEL_CAB, vec![effect]),
                "{name}"
            );
        }

        // 2258 asks first; no is the end of it.
        let mut meerkats = script("q2258s", OUTSIDE_ARIANT);
        assert_eq!(kind(&meerkats.begin()), Some(1));
        let no = meerkats.respond(ScriptInput::No).unwrap();
        assert_eq!((no.prompt, no.effects), (None, vec![]));
        let mut meerkats = script("q2258s", OUTSIDE_ARIANT);
        meerkats.begin();
        let go = meerkats.respond(ScriptInput::Yes).unwrap();
        assert!(go.prompt.is_some() && go.effects.is_empty());
        assert_eq!(
            meerkats.respond(ScriptInput::Next).unwrap().effects,
            [Effect::StartQuest(MEERKATS)]
        );

        // 2259 ends only at Sahel 1.
        assert_eq!(
            read_through(script("q2259e", OUTSIDE_ARIANT)),
            (CAMEL_CAB, vec![])
        );
        assert_eq!(
            read_through(script("q2259e", 260020700)),
            (CAMEL_CAB, vec![Effect::CompleteQuest(SCORPIONS)])
        );
    }

    #[test]
    fn dr_kim_mr_ku_and_muhamad_reward_their_quests() {
        assert_eq!(
            read_through(NpcScript::for_quest("q3452e", &player(50)).unwrap()),
            (
                DR_KIM,
                vec![
                    Effect::TakeItem {
                        item_id: BLOCTOPUS_KEY_CHAIN,
                        quantity: 1
                    },
                    Effect::GiveItem {
                        item_id: MANA_ELIXIR_PILL,
                        quantity: 50
                    },
                    Effect::GainExp(8000),
                    Effect::CompleteQuest(BLOCKTOPUS_ALIEN),
                ]
            )
        );

        // The most of a tier held is taken.
        for (held, taken, given, exp) in [
            (1, 1, vec![(2000000, 1)], 10),
            (99, 50, vec![(2020007, 50)], 10000),
            (499, 100, vec![], 45000),
            (999, 600, vec![(2020013, 50)], 54000),
            (1200, 1000, vec![(2040501, 1), (2000005, 50)], 54000),
        ] {
            let mut holding = player(50);
            holding.inventory.insert(
                game::InventoryType::Etc,
                1,
                game::Item::stack(BELLFLOWER, held),
            );
            let mut expected = vec![Effect::TakeItem {
                item_id: BELLFLOWER,
                quantity: taken,
            }];
            expected.extend(
                given
                    .into_iter()
                    .map(|(item_id, quantity)| Effect::GiveItem { item_id, quantity }),
            );
            expected.extend([
                Effect::GainExp(exp),
                Effect::CompleteQuest(LACKING_INGREDIENTS),
            ]);
            let script = NpcScript::for_quest("q3833e", &holding).unwrap();
            assert_eq!(read_through(script), (MR_KU, expected), "{held}");
        }

        let mut muhamad = NpcScript::for_quest("q3953e", &player(50)).unwrap();
        assert_eq!(muhamad.npc_id(), MUHAMAD);
        assert_eq!(kind(&muhamad.begin()), Some(4));
        for _ in 0..3 {
            assert_eq!(
                kind(&muhamad.respond(ScriptInput::Select(0)).unwrap()),
                Some(4)
            );
        }
        let convinced = muhamad.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(page_text(&convinced), text(keys::MUHAMAD_CONVINCED));
        assert_eq!(
            convinced.effects,
            [
                Effect::CompleteQuest(CONVINCE_MUHAMAD),
                Effect::TakeItem {
                    item_id: LIDIUM,
                    quantity: 1
                },
                Effect::GainExp(20000),
            ]
        );
    }

    #[test]
    fn a_bush_gives_glasses_and_abel_takes_any_pair() {
        let looking = |held: Option<i32>| {
            let mut player = traveller(20, 0, false);
            player.quests.upsert(game::QuestRecord {
                quest_id: FIND_MY_GLASSES,
                status: QuestStatus::Started {
                    kills: Vec::new(),
                    info: None,
                },
            });
            if let Some(glasses) = held {
                player
                    .inventory
                    .insert(game::InventoryType::Etc, 1, game::Item::stack(glasses, 1));
            }
            player
        };
        use rand::SeedableRng;
        let rng = &mut rand::rngs::StdRng::seed_from_u64(7);
        let mut found = std::collections::BTreeSet::new();
        for _ in 0..30 {
            let (npc, effects) = read_through(nautilus_bush(1094004, &looking(None), rng));
            assert_eq!(npc, 1094004);
            let [Effect::GiveItem {
                item_id,
                quantity: 1,
            }] = effects[..]
            else {
                panic!("{effects:?}");
            };
            found.insert(item_id);
        }
        assert_eq!(
            found.into_iter().collect::<Vec<_>>(),
            [ABELS_GLASSES, OTHER_GLASSES[0], OTHER_GLASSES[1]]
        );
        // One pair at a time; and without the quest the bush is a bush.
        let searched = nautilus_bush(1094004, &looking(Some(OTHER_GLASSES[1])), rng);
        assert_eq!(read_through(searched).1, []);
        assert!(NpcScript::for_npc(1094004, &traveller(20, 0, false)).is_none());

        let thanks = |glasses, exp, scrolls| {
            vec![
                Effect::TakeItem {
                    item_id: glasses,
                    quantity: 1,
                },
                Effect::GainExp(exp),
                Effect::GiveItem {
                    item_id: NAUTILUS_RETURN,
                    quantity: scrolls,
                },
                Effect::CompleteQuest(FIND_MY_GLASSES),
                Effect::NpcSpecialAction {
                    npc_id: ABEL,
                    action: "quest",
                },
            ]
        };
        let own = NpcScript::for_quest("q2186e", &looking(Some(ABELS_GLASSES))).unwrap();
        assert_eq!(read_through(own), (ABEL, thanks(ABELS_GLASSES, 1700, 10)));
        let other = NpcScript::for_npc(ABEL, &looking(Some(OTHER_GLASSES[0]))).unwrap();
        assert!(other.inventory_full.is_some());
        assert_eq!(
            read_through(other),
            (ABEL, thanks(OTHER_GLASSES[0], 1000, 5))
        );
        // His own pair goes through the quest's end instead.
        assert!(NpcScript::for_npc(ABEL, &looking(Some(ABELS_GLASSES))).is_none());
        assert!(NpcScript::for_npc(ABEL, &looking(None)).is_none());
    }

    #[test]
    fn eleska_takes_moppies_box_and_manji_sends_the_player_off() {
        for (name, quest, box_id) in [
            ("q2124e", SAND_CREW_SUPPLY, MOPPIES_BOX),
            ("q2126e", SAND_CREW_SUPPLY_AGAIN, MOPPIES_BOX_AGAIN),
        ] {
            let script = NpcScript::for_quest(name, &player(20)).unwrap();
            assert_eq!(
                read_through(script),
                (
                    ELESKA,
                    vec![
                        Effect::TakeItem {
                            item_id: box_id,
                            quantity: 1
                        },
                        Effect::CompleteQuest(quest)
                    ]
                )
            );
        }
        let script = NpcScript::for_quest("q2127e", &player(20)).unwrap();
        assert_eq!(
            read_through(script),
            (MANJI, vec![Effect::CompleteQuest(TO_THE_DESERT)])
        );
    }

    #[test]
    fn each_teller_of_the_rumor_completes_its_quest() {
        for (quest, npc, exp) in [
            (2148, 1020000, 10),
            (2149, MANJI, 10),
            (2150, 1022007, 10),
            (2151, 1022000, 10),
            (2152, 1032104, 15),
        ] {
            let script = NpcScript::for_quest(&format!("q{quest}s"), &player(35)).unwrap();
            assert_eq!(
                read_through(script),
                (npc, vec![Effect::FinishQuest(quest), Effect::GainExp(exp)])
            );
        }
    }

    #[test]
    fn tienk_completes_and_rho_starts_the_last_song() {
        let script = NpcScript::for_quest("q2197e", &player(5)).unwrap();
        assert_eq!(
            read_through(script),
            (TIENK, vec![Effect::CompleteQuest(TIENKS_GREETING)])
        );
        let script = NpcScript::for_quest("q2293s", &player(40)).unwrap();
        assert_eq!(
            read_through(script),
            (MAESTRO_RHO, vec![Effect::StartQuest(THE_LAST_SONG)])
        );
    }

    #[test]
    fn new_leaf_city_starts_ask_then_start_their_quest() {
        for (quest, npc) in [
            (8221, 9201051),
            (8223, 9201098),
            (8224, TAGGRIN),
            (8226, TAGGRIN),
            (8232, LITA_LAWLESS),
            (8238, LITA_LAWLESS),
        ] {
            let name = format!("q{quest}s");
            let mut script = NpcScript::for_quest(&name, &player(80)).unwrap();
            assert_eq!(script.npc_id(), npc);
            script.begin();
            assert!(script.respond(ScriptInput::Yes).unwrap().prompt.is_some());
            let end = script.respond(ScriptInput::Next).unwrap();
            assert_eq!(end.effects, [Effect::StartQuest(quest)], "{quest}");

            let mut script = NpcScript::for_quest(&name, &player(80)).unwrap();
            script.begin();
            assert!(script.respond(ScriptInput::No).unwrap().prompt.is_some());
            let end = script.respond(ScriptInput::Next).unwrap();
            assert!(end.effects.is_empty(), "{quest}");
        }
        assert!(NpcScript::for_quest("q8220s", &player(80)).is_none());
    }

    #[test]
    fn jack_john_lukan_and_elpam_end_their_quests() {
        let holding = |held: &[(game::InventoryType, i32, i16)]| {
            let mut player = player(80);
            for (slot, &(kind, item, count)) in held.iter().enumerate() {
                player
                    .inventory
                    .insert(kind, slot as i16 + 1, game::Item::stack(item, count));
            }
            player
        };
        let etc = game::InventoryType::Etc;
        let take = |item_id, quantity| Effect::TakeItem { item_id, quantity };

        let script = NpcScript::for_quest("q8219e", &player(80)).unwrap();
        let mut script = script;
        script.begin();
        let end = {
            script.respond(ScriptInput::Next).unwrap();
            script.respond(ScriptInput::Next).unwrap()
        };
        assert_eq!(
            end.effects,
            [
                give_one(PHANTOM_FOREST_MAP),
                Effect::GainExp(175_000),
                Effect::CompleteQuest(8219)
            ]
        );

        // Lukan wants all ten badges.
        let few = holding(&[(etc, STORMBREAKER_BADGE, 9)]);
        let (_, effects) = read_through(NpcScript::for_quest("q8222e", &few).unwrap());
        assert!(effects.is_empty());
        let ten = holding(&[(etc, STORMBREAKER_BADGE, 10)]);
        assert_eq!(
            read_through(NpcScript::for_quest("q8222e", &ten).unwrap()),
            (
                LUKAN,
                vec![take(STORMBREAKER_BADGE, 10), Effect::CompleteQuest(8222)]
            )
        );

        // The letter is given only when it isn't held.
        for (quest, npc, letter) in [
            (8227, JACK, CODED_COMMUNIQUE),
            (8228, JOHN_BARRICADE, CODED_COMMUNIQUE),
            (8229, JOHN_BARRICADE, TRANSLATED_COMMUNIQUE),
        ] {
            let name = format!("q{quest}s");
            for held in [false, true] {
                let player = if held {
                    holding(&[(etc, letter, 1)])
                } else {
                    player(80)
                };
                let mut script = NpcScript::for_quest(&name, &player).unwrap();
                assert_eq!(script.npc_id(), npc);
                script.begin();
                let mut step = script.respond(ScriptInput::Yes).unwrap();
                if step.prompt.is_some() {
                    step = script.respond(ScriptInput::Next).unwrap();
                }
                let mut want = Vec::new();
                if !held {
                    want.push(give_one(letter));
                }
                want.push(Effect::StartQuest(quest));
                assert_eq!(step.effects, want, "{quest} {held}");
            }
        }

        let (_, effects) = read_through(NpcScript::for_quest("q8228e", &player(80)).unwrap());
        assert!(effects.is_empty());
        let coded = holding(&[(etc, CODED_COMMUNIQUE, 1)]);
        assert_eq!(
            read_through(NpcScript::for_quest("q8228e", &coded).unwrap()),
            (
                ELPAM_GORLAB,
                vec![take(CODED_COMMUNIQUE, 1), Effect::CompleteQuest(8228)]
            )
        );
        let translated = holding(&[(etc, TRANSLATED_COMMUNIQUE, 1)]);
        assert_eq!(
            read_through(NpcScript::for_quest("q8229e", &translated).unwrap()),
            (
                JACK,
                vec![
                    take(TRANSLATED_COMMUNIQUE, 1),
                    Effect::GainExp(50_000),
                    Effect::CompleteQuest(8229)
                ]
            )
        );

        // Jack keeps the keystone with the player.
        let keystone = holding(&[(game::InventoryType::Setup, CRIMSONWOOD_KEYSTONE, 1)]);
        assert_eq!(
            read_through(NpcScript::for_quest("q8230e", &keystone).unwrap()),
            (JACK, vec![Effect::CompleteQuest(8230)])
        );
        let (_, effects) = read_through(NpcScript::for_quest("q8230e", &player(80)).unwrap());
        assert!(effects.is_empty());
    }

    #[test]
    fn adventurer_titles_start_silently_and_dalair_gives_the_medals() {
        for (quest, title, medal) in ADVENTURER_TITLES {
            let mut start = NpcScript::for_quest(&format!("q{quest}s"), &player(30)).unwrap();
            let step = start.begin();
            assert_eq!(step.prompt, None, "{quest}");
            assert_eq!(step.effects[0], Effect::StartQuest(quest));
            assert!(
                matches!(&step.effects[1], Effect::InfoText(line) if line.contains(title)),
                "{quest}"
            );

            let end = NpcScript::for_quest(&format!("q{quest}e"), &player(30)).unwrap();
            assert!(end.inventory_full.is_some());
            assert_eq!(
                read_through(end),
                (DALAIR, vec![give_one(medal), Effect::CompleteQuest(quest)]),
            );
        }
        assert!(NpcScript::for_quest("q29904s", &player(30)).is_none());
    }

    #[test]
    fn a_medal_held_finishes_its_title_quest_silently() {
        for (quest, title, npc) in MEDAL_TITLES {
            let mut start = NpcScript::for_quest(&format!("q{quest}s"), &player(30)).unwrap();
            assert_eq!(start.npc_id(), npc);
            let step = start.begin();
            assert_eq!(step.prompt, None, "{quest}");
            assert_eq!(step.effects[0], Effect::FinishQuest(quest));
            assert!(
                matches!(&step.effects[1], Effect::InfoText(line) if line.contains(title)),
                "{quest}"
            );
        }
        assert!(NpcScript::for_quest("q29904s", &player(30)).is_none());
    }

    #[test]
    fn celebrity_keeps_the_starting_fame_and_ends_at_a_thousand_more() {
        let mut fan = player(50);
        fan.stats.fame = 25;
        let (npc, effects) = read_through(NpcScript::for_quest("q29002s", &fan).unwrap());
        assert_eq!(npc, DALAIR);
        assert_eq!(
            effects,
            [
                Effect::StartQuest(CELEBRITY),
                Effect::SetQuestEx {
                    quest: CELEBRITY,
                    key: "popS",
                    value: "25".into(),
                },
                Effect::SetQuestEx {
                    quest: CELEBRITY,
                    key: "popG",
                    value: "1000".into(),
                },
            ]
        );

        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_000);
        let ends_at = |date: SystemTime| {
            let mut started = fan.clone();
            started.quest_ex.set(CELEBRITY, "popS", 25);
            started.quest_ex.set(CELEBRITY, "popG", 1000);
            started.quests.upsert(game::QuestRecord {
                quest_id: CELEBRITY,
                status: QuestStatus::Started {
                    kills: Vec::new(),
                    info: Some(game::quest::end_date(date)),
                },
            });
            started
        };
        let in_time = ends_at(now + std::time::Duration::from_secs(60));
        let short = celebrity_end(&in_time, now);
        assert_eq!(
            page_text(&short.clone().begin()),
            content::text().get(
                keys::CELEBRITY_NOT_YET,
                &[("gained", "0"), ("goal", "1000")]
            )
        );
        assert_eq!(read_through(short), (DALAIR, vec![]));

        let mut famous = in_time.clone();
        famous.stats.fame = 1025;
        let done = celebrity_end(&famous, now);
        assert!(done.inventory_full.is_some());
        assert_eq!(
            read_through(done),
            (
                DALAIR,
                vec![give_one(CELEBRITY_MEDAL), Effect::CompleteQuest(CELEBRITY)]
            )
        );

        let mut late = ends_at(now - std::time::Duration::from_secs(1));
        late.stats.fame = 1025;
        let too_late = celebrity_end(&late, now);
        assert_eq!(
            page_text(&too_late.clone().begin()),
            text(keys::CELEBRITY_TOO_LATE)
        );
        assert_eq!(read_through(too_late), (DALAIR, vec![]));
    }

    #[test]
    fn veteran_hunter_ends_with_its_medal_at_a_million_hunted() {
        use crate::quests::VETERAN_HUNTER;
        let hunter = player(50);
        let (npc, effects) = read_through(NpcScript::for_quest("q29400s", &hunter).unwrap());
        assert_eq!(npc, DALAIR);
        assert_eq!(
            effects,
            [
                Effect::StartQuest(VETERAN_HUNTER),
                Effect::SetQuestEx {
                    quest: VETERAN_HUNTER,
                    key: "mon",
                    value: "0".into(),
                },
                Effect::SetQuestEx {
                    quest: VETERAN_HUNTER,
                    key: "mg",
                    value: "1000000".into(),
                },
            ]
        );

        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_000);
        let hunted = |count: u32, date: SystemTime| {
            let mut started = hunter.clone();
            started.quest_ex.set(VETERAN_HUNTER, "mon", count);
            started
                .quest_ex
                .set(VETERAN_HUNTER, "mg", VETERAN_HUNTER_GOAL);
            started.quests.upsert(game::QuestRecord {
                quest_id: VETERAN_HUNTER,
                status: QuestStatus::Started {
                    kills: Vec::new(),
                    info: Some(game::quest::end_date(date)),
                },
            });
            started
        };
        let in_time = now + std::time::Duration::from_secs(60);
        let short = veteran_hunter_end(&hunted(999_999, in_time), now);
        assert_eq!(
            page_text(&short.clone().begin()),
            content::text().get(
                keys::VETERAN_HUNTER_NOT_YET,
                &[("hunted", "999999"), ("goal", "1000000")]
            )
        );
        assert_eq!(read_through(short), (DALAIR, vec![]));

        let done = veteran_hunter_end(&hunted(1_000_000, in_time), now);
        assert!(done.inventory_full.is_some());
        assert_eq!(
            read_through(done),
            (
                DALAIR,
                vec![
                    give_one(VETERAN_HUNTER_MEDAL),
                    Effect::CompleteQuest(VETERAN_HUNTER)
                ]
            )
        );

        let late = now - std::time::Duration::from_secs(1);
        let too_late = veteran_hunter_end(&hunted(1_000_000, late), now);
        assert_eq!(
            page_text(&too_late.clone().begin()),
            text(keys::VETERAN_HUNTER_TOO_LATE)
        );
        assert_eq!(read_through(too_late), (DALAIR, vec![]));
    }

    #[test]
    fn knight_titles_start_silently_at_cygnus() {
        for (quest, title) in KNIGHT_TITLES {
            let mut start = NpcScript::for_quest(&format!("q{quest}s"), &player(30)).unwrap();
            assert_eq!(start.npc_id(), CYGNUS);
            let step = start.begin();
            assert_eq!(step.prompt, None, "{quest}");
            assert_eq!(step.effects[0], Effect::StartQuest(quest));
            assert!(
                matches!(&step.effects[1], Effect::InfoText(line) if line.contains(title)),
                "{quest}"
            );
        }
    }

    #[test]
    fn dalair_gives_the_outstanding_citizen_medal_to_the_married_guilded_senior() {
        let family = |juniors| FamilyInfo {
            juniors,
            ..FamilyInfo::default()
        };
        let citizen = |married, guilded| {
            let mut player = guilder(50, 0, guilded);
            player.marriage = Some(game::marriage::MarriageRecord {
                id: 1,
                groom_id: player.stats.id,
                bride_id: player.stats.id + 1,
                status: if married {
                    game::marriage::MARRIED
                } else {
                    game::marriage::ENGAGED
                },
                item_id: 2240000,
                groom_name: player.stats.name.clone(),
                bride_name: "Bride".into(),
            });
            player
        };
        let ends = |player: &Player, juniors| {
            read_through(NpcScript::for_family_quest("q29508e", player, &family(juniors)).unwrap())
        };
        let not_yet = (DALAIR, vec![]);
        assert_eq!(ends(&citizen(true, true), 0), not_yet);
        assert_eq!(ends(&citizen(false, true), 1), not_yet);
        assert_eq!(ends(&citizen(true, false), 1), not_yet);
        assert_eq!(ends(&guilder(50, 0, true), 1), not_yet);
        let end = NpcScript::for_family_quest("q29508e", &citizen(true, true), &family(1)).unwrap();
        assert!(end.inventory_full.is_some());
        assert_eq!(
            read_through(end),
            (
                DALAIR,
                vec![
                    give_one(OUTSTANDING_CITIZEN_MEDAL),
                    Effect::CompleteQuest(OUTSTANDING_CITIZEN)
                ]
            ),
        );
    }

    #[test]
    fn shanks_sails_for_lucas_and_starts_to_lith_harbor() {
        let mut script = NpcScript::for_quest("q1028s", &player(5)).unwrap();
        assert!(script.begin().prompt.is_some());
        let end = script.respond(ScriptInput::Yes).unwrap();
        assert_eq!(end.prompt, None);
        assert_eq!(
            end.effects,
            [Effect::StartQuest(1028), Effect::Warp(LITH_HARBOR)]
        );

        let mut script = NpcScript::for_quest("q1028s", &player(5)).unwrap();
        script.begin();
        let no = script.respond(ScriptInput::No).unwrap();
        assert!(no.prompt.is_some() && no.effects.is_empty());
        assert_eq!(script.respond(ScriptInput::Next).unwrap().effects, []);
    }

    #[test]
    fn the_faulty_translator_gives_only_noise() {
        let script = NpcScript::for_quest("q3455s", &player(50)).unwrap();
        assert_eq!(
            read_through(script),
            (
                ALIEN_GRAY,
                vec![
                    Effect::TakeItem {
                        item_id: FAULTY_TRANSLATOR,
                        quantity: 1
                    },
                    Effect::StartQuest(GRAYS_GARBLED)
                ]
            )
        );
    }

    #[test]
    fn the_sorcerer_sees_the_potion_worked() {
        let script = NpcScript::for_quest("q3514e", &player(50)).unwrap();
        assert_eq!(
            read_through(script),
            (
                SORCERER,
                vec![
                    Effect::GainExp(891_500),
                    Effect::CompleteQuest(SELLS_EMOTIONS)
                ]
            )
        );
    }

    #[test]
    fn ardin_sends_the_player_to_face_the_other_self() {
        let room = crate::scripts::ARDINS_ROOM;
        assert!(resets_when_empty(room));
        let mut no = NpcScript::for_quest("q3933s", &player(50)).unwrap();
        assert_eq!(no.npc_id(), ARDIN);
        no.begin();
        assert_eq!(kind(&no.respond(ScriptInput::Next).unwrap()), Some(1));
        let declined = no.respond(ScriptInput::No).unwrap();
        assert_eq!((declined.prompt, declined.effects), (None, vec![]));
        let mut yes = NpcScript::for_quest("q3933s", &player(50)).unwrap();
        yes.begin();
        yes.respond(ScriptInput::Next).unwrap();
        let go = yes.respond(ScriptInput::Yes).unwrap();
        assert_eq!(page_text(&go), text(keys::ARDIN_TEST_GO));
        assert_eq!(
            yes.respond(ScriptInput::Next).unwrap().effects,
            [
                Effect::RequireEmptyMap(room),
                Effect::StartQuest(ARDINS_SAND_BANDITS),
                Effect::WarpToPortal(room, 1),
            ]
        );
        assert_eq!(yes.refused, Some(text(keys::ARDIN_TEST_BUSY)));
        // Portal 1 is the room's st00, beside Ardin's other self.
        let Ok(maps) = crate::game_data::get() else {
            return;
        };
        let field = maps.field(room).unwrap();
        assert_eq!(field.portal_by_id(1).unwrap().name, "st00");
        assert_eq!(field.mob_spawns[0].mob_id, 9100013);
        assert_eq!(field.mob_spawns[0].mob_time, -1);
        // Its way out names a portal Ardin's house lacks: the default spawn.
        assert!(crate::reactors::lost_link(room, "st00"));
        let out = field.portal_by_name("st00").unwrap();
        let house = maps.field(out.to_map.unwrap()).unwrap();
        assert!(house.resolve_spawn_portal(&out.to_name).is_none());
    }

    #[test]
    fn han_the_broker_takes_two_of_a_jewel_ore_for_a_societys_test() {
        let none = NpcScript::for_quest("q3301e", &player(50)).unwrap();
        assert_eq!(read_through(none), (HAN_THE_BROKER, vec![]));

        let mut holding = player(50);
        holding
            .inventory
            .insert(game::InventoryType::Etc, 1, game::Item::stack(4020000, 1));
        holding
            .inventory
            .insert(game::InventoryType::Etc, 2, game::Item::stack(4020002, 2));
        holding
            .inventory
            .insert(game::InventoryType::Etc, 3, game::Item::stack(4020008, 5));
        for (name, quest) in [("q3301e", ZENUMIST_TEST), ("q3303e", ALCADNO_TEST)] {
            let mut script = NpcScript::for_quest(name, &holding).unwrap();
            let Some(Prompt::Selection { text, .. }) = script.begin().prompt else {
                panic!("a choice of ores");
            };
            // Only those held two of.
            assert!(!text.contains("#t4020000#") && text.contains("#t4020002#"));
            let taken = script.respond(ScriptInput::Select(1)).unwrap();
            assert_eq!(
                taken.effects,
                [
                    Effect::TakeItem {
                        item_id: 4020008,
                        quantity: 2
                    },
                    Effect::CompleteQuest(quest)
                ],
                "{name}"
            );
            assert!(script.refused().is_some(), "no ores, no deal");
        }
    }

    #[test]
    fn carson_and_maed_make_a_lost_cape_again() {
        for (name, npc, quest) in [
            ("q3305s", CARSON, ZENUMIST_CAPE_AGAIN),
            ("q3306s", MAED, ALCADNO_CAPE_AGAIN),
        ] {
            let script = || NpcScript::for_quest(name, &player(50)).unwrap();
            let mut no = script();
            no.begin();
            assert_eq!(kind(&no.respond(ScriptInput::Next).unwrap()), Some(1));
            let declined = no.respond(ScriptInput::No).unwrap();
            assert_eq!((declined.prompt, declined.effects), (None, vec![]));
            let mut yes = script();
            assert_eq!(yes.npc_id(), npc);
            yes.begin();
            yes.respond(ScriptInput::Next).unwrap();
            let go = yes.respond(ScriptInput::Yes).unwrap();
            assert!(go.prompt.is_some() && go.effects.is_empty());
            assert_eq!(
                yes.respond(ScriptInput::Next).unwrap().effects,
                [Effect::StartQuest(quest)],
                "{name}"
            );
        }
    }

    #[test]
    fn parwen_and_de_lang_start_their_quests_in_dran_s_lab() {
        let yes_to_all = |name: &str| {
            let mut script = NpcScript::for_quest(name, &player(50)).unwrap();
            let mut step = script.begin();
            loop {
                let input = match kind(&step) {
                    Some(1) => ScriptInput::Yes,
                    _ => ScriptInput::Next,
                };
                if !step.effects.is_empty() {
                    return (script.npc_id(), step.effects);
                }
                step = script.respond(input).unwrap();
            }
        };
        let to_the_lab = |quest| vec![Effect::StartQuest(quest), Effect::Warp(DRANS_LAB)];
        assert_eq!(
            yes_to_all("q3320s"),
            (PARWEN, to_the_lab(WHAT_PARWEN_KNOWS))
        );
        assert_eq!(
            yes_to_all("q3321s"),
            (DE_LANG, vec![Effect::StartQuest(THE_MISSING_ALCHEMIST)])
        );
        assert_eq!(
            yes_to_all("q3353s"),
            (PARWEN, to_the_lab(WHAT_DE_LANG_WANTS))
        );
        assert_eq!(
            yes_to_all("q3354s"),
            (DE_LANG, vec![Effect::StartQuest(DE_LANGS_POTION)])
        );
        // Once What Parwen Knows is taken, Parwen sends the player back.
        assert_eq!(NpcScript::for_npc(PARWEN, &with_quests(&[])), None);
        for completed in [false, true] {
            let parwen =
                NpcScript::for_npc(PARWEN, &with_quests(&[(WHAT_PARWEN_KNOWS, completed)]))
                    .unwrap();
            assert_eq!(parwen.begin().effects, [Effect::Warp(DRANS_LAB)]);
        }
    }

    #[test]
    fn parwen_s_password_opens_the_secret_passage() {
        use rand::SeedableRng;
        let rng = &mut rand::rngs::StdRng::seed_from_u64(3);
        let mut parwen = verifying_the_password(rng);
        parwen.begin();
        let ask = parwen.respond(ScriptInput::Next).unwrap();
        assert_eq!(kind(&ask), Some(1));
        let code = parwen.respond(ScriptInput::Yes).unwrap();
        let [_, Effect::SetQuestInfo { quest, value }] =
            &parwen.respond(ScriptInput::Next).unwrap().effects[..]
        else {
            panic!("no password");
        };
        assert_eq!(*quest, scripts::VERIFYING_THE_PASSWORD);
        assert_eq!(value.len(), PASSWORD_LEN);
        assert!(value.bytes().all(|b| PASSWORD_CHARS.contains(&b)));
        assert!(page_text(&code).contains(value.as_str()), "told once");
        let mut no = verifying_the_password(rng);
        no.begin();
        no.respond(ScriptInput::Next).unwrap();
        let hurry = no.respond(ScriptInput::No).unwrap();
        assert_eq!(page_text(&hurry), text(keys::PARWEN_PASSWORD_HURRY));
        assert!(hurry.effects.is_empty());

        let mut player = with_quests(&[]);
        assert_eq!(NpcScript::for_npc(SECRET_PASSAGE, &player), None);
        player.stats.map_id = 261010000;
        player.quests.upsert(game::quest::QuestRecord {
            quest_id: scripts::VERIFYING_THE_PASSWORD,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: Some(value.clone()),
            },
        });
        let door = || NpcScript::for_npc(SECRET_PASSAGE, &player).unwrap();
        let mut wrong = door();
        wrong.begin();
        let said = wrong
            .respond(ScriptInput::Text("0000000000".into()))
            .unwrap();
        assert_eq!(page_text(&said), text(keys::SECRET_PASSAGE_WRONG));
        assert!(said.effects.is_empty());
        let mut right = door();
        right.begin();
        let opened = right.respond(ScriptInput::Text(value.clone())).unwrap();
        assert_eq!(
            opened.effects,
            [
                Effect::SetQuestInfo {
                    quest: scripts::VERIFYING_THE_PASSWORD,
                    value: "1".into(),
                },
                Effect::PortalSound,
                scripts::through_secret_door(&player),
            ]
        );
        // Given, the door no longer asks.
        player.quests.set_info(scripts::VERIFYING_THE_PASSWORD, "1");
        assert_eq!(NpcScript::for_npc(SECRET_PASSAGE, &player), None);
    }

    #[test]
    fn olson_takes_a_player_on_dollhouse_into_the_room() {
        assert_eq!(NpcScript::for_npc(OLSON, &with_quests(&[])), None);
        assert_eq!(
            NpcScript::for_npc(OLSON, &with_quests(&[(3230, true)])),
            None
        );
        let mut script = NpcScript::for_npc(OLSON, &with_quests(&[(3230, false)])).unwrap();
        assert!(script.begin().effects.is_empty(), "the hint first");
        let ask = script.respond(ScriptInput::Next).unwrap();
        assert!(ask.effects.is_empty());
        let go = script.respond(ScriptInput::Yes).unwrap();
        assert_eq!(go.effects, [Effect::StartEvent(EventKind::Dollhouse)]);
        assert!(script.refused().is_some(), "told when it is taken");
    }

    #[test]
    fn june_takes_one_helping_blake_up_to_the_spirit_of_rock() {
        let pick = |player: &Player, choice: i32| {
            let mut script = NpcScript::for_npc(JUNE, player).unwrap();
            assert!(script.begin().effects.is_empty(), "the menu first");
            let step = script.respond(ScriptInput::Select(choice)).unwrap();
            (script, step)
        };
        for quest in BLAKES_REQUESTS {
            let (mut script, go) = pick(&with_quests(&[(quest, false)]), 0);
            assert_eq!(go.effects, [Effect::StartEvent(EventKind::RockSpirit)]);
            assert!(script.refused().is_some(), "told when all are taken");
        }
        let (_, no) = pick(&with_quests(&[(2286, true)]), 0);
        assert!(no.effects.is_empty());
        assert_eq!(page_text(&no), text(keys::JUNE_NOT_HELPING));

        // The VIP Zone is 2291's to open; June only says how.
        let vip = |player: &Player| page_text(&pick(player, 1).1);
        assert_eq!(vip(&with_quests(&[])), text(keys::JUNE_NOT_VIP));
        assert_eq!(
            vip(&with_quests(&[(VIP_PASS, true)])),
            text(keys::JUNE_VIP_OVER_LEVEL)
        );
        let mut young = with_quests(&[(VIP_PASS, true)]);
        young.stats.level = 50;
        assert_eq!(vip(&young), text(keys::JUNE_VIP_INVITATIONS));
    }

    #[test]
    fn june_takes_ten_vip_invitations_into_the_vip_zone() {
        let mut player = with_quests(&[(VIP_ADMISSION, false)]);
        player.stats.level = 40;
        let give = |player: &mut Player, count: i16| {
            player.inventory.insert(
                game::InventoryType::Etc,
                1,
                game::Item::stack(VIP_INVITATION, count),
            );
        };
        give(&mut player, 9);
        let short = NpcScript::for_quest("q2291e", &player).unwrap();
        assert_eq!(
            page_text(&short.clone().begin()),
            text(keys::JUNE_VIP_MISSING)
        );
        assert_eq!(read_through(short), (JUNE, Vec::new()));

        give(&mut player, 10);
        let mut script = NpcScript::for_quest("q2291e", &player).unwrap();
        assert_eq!(page_text(&script.begin()), text(keys::JUNE_VIP_WELCOME));
        let go = script.respond(ScriptInput::Next).unwrap();
        assert_eq!(
            go.effects,
            [
                Effect::TakeItem {
                    item_id: VIP_INVITATION,
                    quantity: 10
                },
                Effect::CompleteQuest(VIP_ADMISSION),
                Effect::StartEvent(EventKind::RockSpiritVip),
            ]
        );
        assert!(script.refused().is_some(), "told when all are taken");
    }

    #[test]
    fn mark_takes_the_pendulum_and_sends_the_player_back() {
        let back = Effect::WarpToPortal(events::EOS_TOWER_100, events::EOS_TOWER_100_RETURN_ID);
        let mut on_it = with_quests(&[(3230, false)]);
        for player in [on_it.clone(), with_quests(&[])] {
            let mut script = NpcScript::for_npc(MARK, &player).unwrap();
            assert!(script.begin().effects.is_empty(), "nothing taken");
            let yes = script.respond(ScriptInput::Yes).unwrap();
            assert_eq!(yes.effects, [back.clone()]);
        }
        on_it.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(events::DOLLHOUSE_PENDULUM, 1),
        );
        let mut script = NpcScript::for_npc(MARK, &on_it).unwrap();
        assert_eq!(
            script.begin().effects,
            [
                Effect::TakeItem {
                    item_id: events::DOLLHOUSE_PENDULUM,
                    quantity: 1
                },
                Effect::CompleteQuest(3230)
            ]
        );
        let ask = script.respond(ScriptInput::Next).unwrap();
        assert!(ask.effects.is_empty());
        let yes = script.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.effects, [back]);
    }

    #[test]
    fn cheng_sends_a_player_on_3239_in_and_takes_ten_parts_for_a_scroll() {
        // In the factory, only a player on it is taken in.
        assert!(NpcScript::for_npc(CHENG, &with_quests(&[])).is_none());
        let mut on_it = with_quests(&[(3239, false)]);
        on_it.stats.map_id = events::TOY_FACTORY;
        let mut script = NpcScript::for_npc(CHENG, &on_it).unwrap();
        assert!(script.begin().effects.is_empty());
        let yes = script.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            yes.effects,
            [Effect::StartEvent(EventKind::MechanicalParts)]
        );
        assert_eq!(script.refused, Some(text(keys::MECHANICAL_PARTS_BUSY)));

        // In Sector 4, with fewer than ten parts, only the way out.
        let out = Effect::Warp(events::PARTS_PASSAGE);
        on_it.stats.map_id = events::PARTS_ROOM;
        let parts = |count| game::Item::stack(events::MACHINE_PART, count);
        on_it
            .inventory
            .insert(game::InventoryType::Etc, 1, parts(9));
        let mut script = NpcScript::for_npc(CHENG, &on_it).unwrap();
        assert!(script.begin().effects.is_empty());
        assert_eq!(
            script.respond(ScriptInput::Yes).unwrap().effects,
            [out.clone()]
        );

        // Ten: the parts for a scroll and 2,700 EXP, then the way out.
        on_it
            .inventory
            .insert(game::InventoryType::Etc, 1, parts(10));
        let mut script = NpcScript::for_npc(CHENG, &on_it).unwrap();
        let effects = script.begin().effects;
        let Effect::GiveItem {
            item_id,
            quantity: 1,
        } = effects[1]
        else {
            panic!("{effects:?}");
        };
        assert!(PARTS_SCROLLS.contains(&item_id));
        assert_eq!(
            effects,
            [
                Effect::TakeItem {
                    item_id: events::MACHINE_PART,
                    quantity: 10
                },
                Effect::GiveItem {
                    item_id,
                    quantity: 1
                },
                Effect::GainExp(2700),
                Effect::CompleteQuest(3239)
            ]
        );
        assert!(script
            .respond(ScriptInput::Next)
            .unwrap()
            .effects
            .is_empty());
        assert_eq!(
            script.respond(ScriptInput::Yes).unwrap().effects,
            [out.clone()]
        );

        // Not with a full USE inventory.
        on_it.inventory.set_slot_limit(game::InventoryType::Use, 1);
        on_it
            .inventory
            .insert(game::InventoryType::Use, 1, game::Item::stack(2_000_000, 1));
        let mut script = NpcScript::for_npc(CHENG, &on_it).unwrap();
        let page = script.begin();
        assert!(page.effects.is_empty());
        assert_eq!(page_text(&page), text(keys::MECHANICAL_PARTS_NO_ROOM));
        script.respond(ScriptInput::Next).unwrap();
        assert_eq!(script.respond(ScriptInput::Yes).unwrap().effects, [out]);
    }

    #[test]
    fn the_broken_snow_statue_is_the_clue() {
        let script = NpcScript::for_quest("q3108s", &player(50)).unwrap();
        assert_eq!(
            read_through(script),
            (
                SNOW_STATUE,
                vec![Effect::FinishQuest(3108), Effect::GainExp(1000)]
            )
        );
    }

    #[test]
    fn the_baby_dragon_trades_each_egg_for_a_dragon_stone() {
        let mut player = with_quests(&[(3706, true)]);
        player.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(NINE_SPIRIT_EGG, 2),
        );
        assert!(NpcScript::for_quest("q3714s", &player)
            .unwrap()
            .inventory_full()
            .is_some());
        let hatched = NpcScript::for_quest("q3714s", &player).unwrap();
        assert_eq!(
            read_through(hatched),
            (
                BABY_DRAGON,
                vec![
                    Effect::FinishQuest(3714),
                    Effect::TakeItem {
                        item_id: NINE_SPIRIT_EGG,
                        quantity: 1
                    },
                    Effect::GiveItem {
                        item_id: DRAGON_STONE,
                        quantity: 1
                    },
                    Effect::GainExp(42_000),
                ]
            )
        );
        // Not while a stone is held.
        player.inventory.insert(
            game::InventoryType::Use,
            1,
            game::Item::stack(DRAGON_STONE, 1),
        );
        let held = NpcScript::for_quest("q3714s", &player).unwrap();
        assert_eq!(read_through(held), (BABY_DRAGON, vec![]));
    }

    #[test]
    fn captain_al_ends_each_lesson_by_the_family_window() {
        let family = |juniors, reputation, total_reputation| FamilyInfo {
            juniors,
            reputation,
            total_reputation,
            ..FamilyInfo::default()
        };
        let ends = |script, family: &FamilyInfo| {
            read_through(NpcScript::for_family_quest(script, &player(50), family).unwrap())
        };
        let done = |quest| (CAPTAIN_AL, vec![Effect::CompleteQuest(quest)]);
        let not_yet = (CAPTAIN_AL, vec![]);
        // A junior taken, for 3,000 EXP.
        assert_eq!(ends("q2232e", &family(0, 0, 0)), not_yet);
        assert_eq!(
            ends("q2232e", &family(1, 0, 0)),
            (
                CAPTAIN_AL,
                vec![Effect::CompleteQuest(2232), Effect::GainExp(3000)]
            )
        );
        // 1,000 Rep earned, spent or not.
        assert_eq!(ends("q2233e", &family(1, 999, 999)), not_yet);
        assert_eq!(ends("q2233e", &family(1, 0, 1000)), done(2233));
        // 2,000 earned, under 500 left to spend.
        assert_eq!(ends("q2234e", &family(1, 499, 1999)), not_yet);
        assert_eq!(ends("q2234e", &family(1, 500, 2000)), not_yet);
        assert_eq!(ends("q2234e", &family(1, 499, 2000)), done(2234));
        for script in FAMILY_QUEST_SCRIPTS {
            assert!(NpcScript::for_quest(script, &player(50)).is_none());
        }
        assert!(NpcScript::for_family_quest("q2214e", &player(50), &family(1, 0, 0)).is_none());
    }

    #[test]
    fn a_maker_class_ends_with_its_teacher() {
        for (script, npc, quest) in [
            ("q6030e", CARSON, 6030),
            ("q6031e", HUGHES, 6031),
            ("q6032e", MEREN, 6032),
        ] {
            assert_eq!(
                ending(script, &player(50)),
                (npc, vec![Effect::CompleteQuest(quest)])
            );
        }
    }

    #[test]
    fn bartos_teaches_follow_the_lead_for_a_pet_snack() {
        let mut aran = player(50);
        aran.stats.job = 2110;
        let mut snackless = NpcScript::for_quest("q4647e", &aran).unwrap();
        let step = snackless.begin();
        assert!(step.effects.is_empty());
        assert_eq!(buttons(&step), (false, false));
        assert_eq!(snackless.respond(ScriptInput::Next), Some(Step::default()));
        aran.inventory
            .insert(InventoryType::Cash, 1, game::Item::stack(PET_SNACK, 1));
        let lesson = |skill_id| {
            (
                BARTOS,
                vec![
                    Effect::TakeItem {
                        item_id: PET_SNACK,
                        quantity: 1,
                    },
                    Effect::CompleteQuest(PET_SECRET_QUEST),
                    Effect::TeachSkill {
                        skill_id,
                        level: 1,
                        master_level: 1,
                    },
                ],
            )
        };
        assert_eq!(ending("q4647e", &aran), lesson(20000024));
        aran.stats.job = 1210;
        assert_eq!(ending("q4647e", &aran), lesson(10000018));
        aran.stats.job = 411;
        assert_eq!(ending("q4647e", &aran), lesson(8));
    }

    #[test]
    fn the_hidden_note_is_found_once_on_bartoss_test() {
        let mut tester = player(50);
        let first = |player: &Player| {
            let script = NpcScript::for_npc(PET_TEST_NOTE_SPOT, player).unwrap();
            let step = script.begin();
            (kind(&step), script)
        };
        // Off the test: only grass.
        assert_eq!(first(&tester).0, Some(0));
        tester.quests.upsert(game::QuestRecord {
            quest_id: PET_TEST_QUEST,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        let (kind_on_test, mut script) = first(&tester);
        assert_eq!(kind_on_test, Some(1));
        assert_eq!(
            script.respond(ScriptInput::Yes).unwrap().effects,
            vec![Effect::GiveItem {
                item_id: HIDDEN_NOTE,
                quantity: 1,
            }]
        );
        assert!(script.inventory_full().is_some());
        tester
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(HIDDEN_NOTE, 1));
        assert_eq!(first(&tester).0, Some(0));
    }

    #[test]
    fn meren_teaches_the_makers_higher_levels_by_job() {
        let mut aran = player(50);
        aran.stats.job = 2110;
        assert_eq!(
            ending("q6033e", &aran),
            (
                MEREN,
                vec![
                    Effect::CompleteQuest(6033),
                    Effect::TeachSkill {
                        skill_id: 20001007,
                        level: 2,
                        master_level: 3,
                    },
                    Effect::GainExp(230_000),
                ]
            )
        );
        let mut warrior = player(50);
        warrior.stats.job = 111;
        assert_eq!(
            ending("q6036e", &warrior),
            (
                MEREN,
                vec![
                    Effect::TakeItem {
                        item_id: GOLD_ANVIL,
                        quantity: 1,
                    },
                    Effect::CompleteQuest(6036),
                    Effect::TeachSkill {
                        skill_id: 1007,
                        level: 3,
                        master_level: 3,
                    },
                    Effect::GainExp(300_000),
                ]
            )
        );
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
                alliance_id: 0,
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
            alliance_rank: 5,
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
        // A yes after boarding closed is refused with the closing text.
        assert!(cherry.refused().is_some());
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

        for (choice, passage) in [
            (0, ORBIS_TUNNEL),
            (2, 200000130),
            (3, 200000140),
            (4, 200000150),
            (5, 200000160),
        ] {
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
        // Kerning City's gate asks where to first: the train to Kerning
        // Square boards at once, and the subway is third.
        let mut gate = subway_gate(KERNING_CITY_GATE, &conti::TO_NLC, &holder, minute(2));
        gate.begin();
        assert_eq!(
            gate.respond(ScriptInput::Select(0)).unwrap().effects,
            [Effect::WarpToPortal(rides::KERNING_TO_SQUARE.map, 0)]
        );
        let mut gate = subway_gate(KERNING_CITY_GATE, &conti::TO_NLC, &holder, minute(2));
        assert_eq!(kind(&gate.begin()), Some(4));
        assert_eq!(
            kind(&gate.respond(ScriptInput::Select(2)).unwrap()),
            Some(1)
        );
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
        // A yes after boarding closed is refused with the closing text.
        assert!(gate.refused().is_some());
        // Either line's gate takes its Basic ticket first.
        for (npc, route, basic) in [
            (1052007, &conti::TO_NLC, 4031710),
            (9201068, &conti::TO_KERNING_CITY, 4031712),
        ] {
            let mut both = traveller(10, 0, false);
            for ticket in [route.ticket, basic] {
                both.inventory
                    .insert(game::InventoryType::Etc, 1, game::Item::stack(ticket, 1));
            }
            let mut gate = subway_gate(npc, route, &both, minute(2));
            gate.begin();
            if npc == KERNING_CITY_GATE {
                gate.respond(ScriptInput::Select(2));
            }
            assert!(gate
                .respond(ScriptInput::Yes)
                .unwrap()
                .effects
                .contains(&Effect::TakeItem {
                    item_id: basic,
                    quantity: 1
                }));
        }
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
            let mut gate = subway_gate(KERNING_CITY_GATE, &conti::TO_NLC, &holder, t);
            gate.begin();
            let closed = gate.respond(ScriptInput::Select(2)).unwrap();
            assert_eq!(buttons(&closed), (false, false));
        }

        // Bell sells the ticket for the line leaving her platform: the Basic
        // one to level 10 and under, the Regular one past it.
        for (level, platform, ticket, price) in [
            (11, 103000100, 4031711, 5000),
            (11, 600010001, 4031713, 5000),
            (10, 103000100, 4031710, 1000),
            (10, 600010001, 4031712, 1000),
        ] {
            let mut buyer = traveller(level, price, false);
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
                    Effect::SpendMeso(price),
                    Effect::GiveItem {
                        item_id: ticket,
                        quantity: 1
                    }
                ]
            );
            assert!(bell.inventory_full().is_some());
        }
        // One short of the Basic fare is told so.
        let mut short = traveller(10, 999, false);
        short.stats.map_id = 103000100;
        let mut bell = NpcScript::for_npc(BELL, &short).unwrap();
        bell.begin();
        bell.respond(ScriptInput::Select(0)).unwrap();
        let step = bell.respond(ScriptInput::Yes).unwrap();
        assert!(step.effects.is_empty());
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
    fn the_airport_booths_sell_one_ticket_and_board_its_holders() {
        const HOUR: i64 = 3_600_000;
        // A plane leaves on the hour and every 6 minutes; boarding is open
        // from 5 to 1 minutes before.
        let minute = |m: i64| 7 * HOUR + m * 60_000;
        let (irene, route) = (9270041, &conti::TO_CHANGI);
        let booth = |player: &Player, t| plane_booth(irene, route, "Singapore", player, t);
        let buyer = rider(0, 5000, 103000000);
        let mut sold = booth(&buyer, minute(2));
        assert_eq!(kind(&sold.begin()), Some(4));
        assert_eq!(
            kind(&sold.respond(ScriptInput::Select(0)).unwrap()),
            Some(1)
        );
        assert_eq!(
            sold.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::SpendMeso(5000),
                Effect::GiveItem {
                    item_id: 4031731,
                    quantity: 1
                }
            ]
        );
        assert!(sold.inventory_full().is_some());
        // Not to one short of the fare, nor to one who holds a ticket.
        let mut holder = buyer.clone();
        holder.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(route.ticket, 1),
        );
        for player in [&rider(0, 4999, 103000000), &holder] {
            let mut refused = booth(player, minute(2));
            refused.begin();
            refused.respond(ScriptInput::Select(0)).unwrap();
            let step = refused.respond(ScriptInput::Yes).unwrap();
            assert!(step.effects.is_empty());
            assert_eq!(buttons(&step), (false, false));
        }
        // A holder boards while the plane boards.
        let mut board = booth(&holder, minute(2));
        board.begin();
        assert_eq!(
            kind(&board.respond(ScriptInput::Select(1)).unwrap()),
            Some(1)
        );
        assert_eq!(
            board.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::RequireBoarding(103000000),
                Effect::TakeItem {
                    item_id: 4031731,
                    quantity: 1
                },
                Effect::Warp(540010100),
            ]
        );
        for t in [minute(5), minute(6)] {
            let mut late = booth(&holder, t);
            late.begin();
            let step = late.respond(ScriptInput::Select(1)).unwrap();
            assert_eq!(buttons(&step), (false, false));
        }
    }

    #[test]
    fn the_pilots_let_passengers_out_and_say_the_plane_lands_soon() {
        for (pilot, room, airport) in [
            (9270017, 540010100, 103000000),
            (9270018, 540010001, 540010000),
        ] {
            let mut out = NpcScript::for_npc(pilot, &rider(0, 0, room)).unwrap();
            assert_eq!(kind(&out.begin()), Some(1));
            assert_eq!(
                out.respond(ScriptInput::Yes).unwrap().effects,
                [Effect::Warp(airport)]
            );
        }
        let mut says = Vec::new();
        for plane in [540010101, 540010002] {
            let aboard = NpcScript::for_npc(9270018, &rider(0, 0, plane)).unwrap();
            let step = aboard.begin();
            assert_eq!(buttons(&step), (false, false));
            says.push(page_text(&step));
        }
        assert_ne!(says[0], says[1]);
        assert!(NpcScript::for_npc(9270018, &rider(0, 0, 540010000)).is_none());
    }

    #[test]
    fn spinel_tours_japan_and_malaysia_and_brings_the_traveller_back() {
        // From a town to the Mushroom Shrine, or from Boat Quay to Malaysia:
        // the fare is paid and the town saved as the last page turns.
        for (from, to, fare) in [
            (100000000, 800000000, 3000),
            (240000000, 800000000, 3000),
            (541000000, 550000000, 10000),
        ] {
            let mut tour = NpcScript::for_npc(SPINEL, &rider(100, fare, from)).unwrap();
            assert_eq!(kind(&tour.begin()), Some(0));
            assert_eq!(kind(&tour.respond(ScriptInput::Next).unwrap()), Some(4));
            assert_eq!(
                kind(&tour.respond(ScriptInput::Select(0)).unwrap()),
                Some(0)
            );
            let last = tour.respond(ScriptInput::Next).unwrap();
            assert!(last.effects.is_empty());
            assert_eq!(
                tour.respond(ScriptInput::Next).unwrap().effects,
                [
                    Effect::SpendMeso(fare),
                    Effect::SaveLocation(WORLD_TOUR_RETURN, Some(from)),
                    Effect::WarpToPortal(to, 0),
                ]
            );
            assert!(tour.refused.is_some());
            // One meso short, the tour ends after the description.
            let mut short = NpcScript::for_npc(SPINEL, &rider(100, fare - 1, from)).unwrap();
            short.begin();
            short.respond(ScriptInput::Next).unwrap();
            short.respond(ScriptInput::Select(0)).unwrap();
            let step = short.respond(ScriptInput::Next).unwrap();
            assert!(step.effects.is_empty());
            assert_eq!(buttons(&step), (false, false));
        }
        // At the shrine, back where the tour began, else to Lith Harbor.
        let mut toured = rider(100, 0, 800000000);
        toured.saved_locations.insert(WORLD_TOUR_RETURN, 220000000);
        for (player, back) in [(&toured, 220000000), (&rider(100, 0, 800000000), 104000000)] {
            let mut home = NpcScript::for_npc(SPINEL, player).unwrap();
            assert_eq!(kind(&home.begin()), Some(4));
            assert!(home
                .respond(ScriptInput::Select(0))
                .unwrap()
                .effects
                .is_empty());
            assert_eq!(
                home.respond(ScriptInput::Next).unwrap().effects,
                [
                    Effect::SaveLocation(WORLD_TOUR_RETURN, None),
                    Effect::Warp(back),
                ]
            );
            let mut stay = NpcScript::for_npc(SPINEL, player).unwrap();
            stay.begin();
            let step = stay.respond(ScriptInput::Select(1)).unwrap();
            assert!(step.effects.is_empty());
            assert!(stay.respond(ScriptInput::Next).unwrap().effects.is_empty());
        }
    }

    #[test]
    fn the_florina_guides_take_a_fare_or_show_the_ticket_and_pison_sees_players_home() {
        let take = |guide, player: &Player| {
            let mut trip = NpcScript::for_npc(guide, player).unwrap();
            assert_eq!(kind(&trip.begin()), Some(4));
            assert_eq!(
                kind(&trip.respond(ScriptInput::Select(0)).unwrap()),
                Some(1)
            );
            trip.respond(ScriptInput::Yes).unwrap().effects
        };
        let go = |from| {
            [
                Effect::SaveLocation(FLORINA_RETURN, Some(from)),
                Effect::WarpToPortal(110000000, 4),
            ]
        };
        for (guide, fare, from) in [
            (1002002, 1500, 104000000),
            (2010005, 2000, 200000000),
            (2040048, 1800, 220000000),
        ] {
            let mut paid = vec![Effect::SpendMeso(fare)];
            paid.extend(go(from));
            assert_eq!(take(guide, &rider(100, fare, from)), paid);
            assert!(take(guide, &rider(100, fare - 1, from)).is_empty());
        }
        // The VIP ticket is shown, not taken; without it nothing happens.
        let mut holder = rider(100, 0, 104000000);
        holder.inventory.insert(
            game::InventoryType::Etc,
            1,
            game::Item::stack(VIP_TICKET_TO_FLORINA, 1),
        );
        for (player, effects) in [
            (&holder, go(104000000).to_vec()),
            (&rider(100, 0, 104000000), Vec::new()),
        ] {
            let mut trip = NpcScript::for_npc(1002002, player).unwrap();
            trip.begin();
            assert_eq!(
                kind(&trip.respond(ScriptInput::Select(1)).unwrap()),
                Some(1)
            );
            assert_eq!(trip.respond(ScriptInput::Yes).unwrap().effects, effects);
        }
        let mut what = NpcScript::for_npc(1002002, &holder).unwrap();
        what.begin();
        assert_eq!(
            kind(&what.respond(ScriptInput::Select(2)).unwrap()),
            Some(0)
        );
        assert!(what.respond(ScriptInput::Next).unwrap().effects.is_empty());

        // Pison: back where the trip began, else to Lith Harbor.
        let mut came = rider(100, 0, 110000000);
        came.saved_locations.insert(FLORINA_RETURN, 200000000);
        for (player, back) in [(&came, 200000000), (&rider(100, 0, 110000000), 104000000)] {
            let mut home = NpcScript::for_npc(PISON, player).unwrap();
            assert_eq!(kind(&home.begin()), Some(0));
            assert_eq!(kind(&home.respond(ScriptInput::Next).unwrap()), Some(1));
            assert_eq!(
                home.respond(ScriptInput::Yes).unwrap().effects,
                [
                    Effect::SaveLocation(FLORINA_RETURN, None),
                    Effect::Warp(back)
                ]
            );
        }
    }

    #[test]
    fn pietro_pietra_and_harry_send_players_back_from_a_gms_event() {
        let back = |player: &Player, npc| {
            let mut exit = NpcScript::for_npc(npc, player).unwrap();
            exit.begin();
            exit.respond(match npc {
                HARRY => ScriptInput::Yes,
                _ => ScriptInput::Next,
            })
            .unwrap()
            .effects
        };
        let go = |to| vec![Effect::SaveLocation(EVENT_RETURN, None), Effect::Warp(to)];
        let mut came = rider(100, 0, 109050000);
        came.saved_locations.insert(EVENT_RETURN, 60000);
        // Pietro gives the winner the prize on the way.
        let mut prize = vec![Effect::GiveItem {
            item_id: SCROLL_OF_SECRETS,
            quantity: 1,
        }];
        prize.extend(go(60000));
        assert_eq!(back(&came, PIETRO), prize);
        came.stats.map_id = 109050001;
        assert_eq!(back(&came, PIETRA), go(60000));
        // Without a saved map, Henesys.
        assert_eq!(back(&rider(100, 0, 109050001), PIETRA), go(HENESYS.0));
        // Harry, during the game, gives the place back first.
        came.stats.map_id = 109080000;
        let mut leave = vec![Effect::LeaveGmEvent { map_id: 109080000 }];
        leave.extend(go(60000));
        assert_eq!(back(&came, HARRY), leave);
    }

    #[test]
    fn thomas_swift_walks_between_henesys_and_amoria_for_free() {
        for (from, to) in [(100000000, (680000000, 0)), (680000000, (100000000, 5))] {
            let mut walk = NpcScript::for_npc(THOMAS_SWIFT, &rider(0, 0, from)).unwrap();
            assert_eq!(kind(&walk.begin()), Some(1));
            assert!(walk.respond(ScriptInput::Yes).unwrap().effects.is_empty());
            assert_eq!(
                walk.respond(ScriptInput::Next).unwrap().effects,
                [Effect::WarpToPortal(to.0, to.1)]
            );
        }
        assert!(NpcScript::for_npc(THOMAS_SWIFT, &rider(0, 0, 101000000)).is_none());
    }

    #[test]
    fn the_ferries_and_the_crane_take_a_fare_and_board_the_ride() {
        // (npc, where it stands, choice, fare, where a yes warps, where the
        // choice says it goes)
        for (npc, from, choice, fare, warp, to) in [
            (1100008, 200000161, Some(0), 1000, (200090020, 0), 130000210),
            (1100004, 130000210, Some(0), 1000, (200090021, 0), 200000161),
            (1100007, 101000400, Some(0), 1000, (200090030, 0), 130000210),
            (1100003, 130000210, Some(0), 1000, (200090031, 0), 101000400),
            (1200004, 104000000, Some(0), 800, (200090060, 0), 140020300),
            (1200003, 140020300, Some(0), 800, (200090070, 0), 104000000),
            (CRANE, 200000141, Some(0), 1500, (200090300, 0), 250000100),
            (CRANE, 250000100, Some(0), 1500, (200090310, 0), 200000141),
            (CRANE, 250000100, Some(1), 500, (251000000, 0), 251000000),
            (CRANE, 251000000, None, 500, (250000100, 0), 250000100),
        ] {
            let start = |meso| {
                let mut ride = NpcScript::for_npc(npc, &rider(0, meso, from)).unwrap();
                let first = ride.begin();
                let ask = match choice {
                    Some(choice) => {
                        assert_eq!(kind(&first), Some(4));
                        assert!(menu_text(&first).contains(&format!("#m{to}#")));
                        ride.respond(ScriptInput::Select(choice)).unwrap()
                    }
                    None => first,
                };
                assert_eq!(kind(&ask), Some(1));
                ride
            };
            let mut ride = start(fare);
            assert!(ride.refused.is_some());
            assert_eq!(
                ride.respond(ScriptInput::Yes).unwrap().effects,
                [
                    Effect::SpendMeso(fare),
                    Effect::WarpToPortal(warp.0, warp.1)
                ]
            );
            let mut short = start(fare - 1);
            assert!(short.respond(ScriptInput::Yes).unwrap().effects.is_empty());
            let mut no = start(fare);
            assert!(no.respond(ScriptInput::No).unwrap().effects.is_empty());
        }
        assert!(NpcScript::for_npc(CRANE, &rider(0, 0, 100000000)).is_none());
    }

    #[test]
    fn audrey_rides_around_malaysia_and_sees_the_tour_home() {
        // A paid ride asks first; from the CBD it saves where the tour began.
        for (from, to, fare) in [
            (540000000, (550000000, 0), 42000),
            (550000000, (551000000, 2), 10000),
            (551000000, (550000000, 4), 10000),
        ] {
            let start = |meso| {
                let mut ride = NpcScript::for_npc(AUDREY, &rider(0, meso, from)).unwrap();
                assert_eq!(kind(&ride.begin()), Some(4));
                assert_eq!(
                    kind(&ride.respond(ScriptInput::Select(0)).unwrap()),
                    Some(1)
                );
                ride
            };
            let mut paid = vec![Effect::SpendMeso(fare)];
            if from == 540000000 {
                paid.push(Effect::SaveLocation(WORLD_TOUR_RETURN, Some(from)));
            }
            paid.push(Effect::WarpToPortal(to.0, to.1));
            let mut ride = start(fare);
            assert_eq!(ride.respond(ScriptInput::Yes).unwrap().effects, paid);
            assert!(ride.refused.is_some());
            // One meso short, or saying no, goes nowhere.
            let mut short = start(fare - 1);
            assert!(short.respond(ScriptInput::Yes).unwrap().effects.is_empty());
            let mut no = start(fare);
            assert!(no.respond(ScriptInput::No).unwrap().effects.is_empty());
        }
        // From Trend Zone, free back where the tour began, else Boat Quay.
        let mut toured = rider(0, 0, 550000000);
        toured.saved_locations.insert(WORLD_TOUR_RETURN, 540000000);
        let mut astray = rider(0, 0, 550000000);
        astray.saved_locations.insert(WORLD_TOUR_RETURN, 100000000);
        for (player, warp) in [
            (&toured, Effect::WarpToPortal(540000000, 4)),
            (&rider(0, 0, 550000000), Effect::WarpToPortal(541000000, 4)),
            (&astray, Effect::Warp(100000000)),
        ] {
            let mut home = NpcScript::for_npc(AUDREY, player).unwrap();
            home.begin();
            let step = home.respond(ScriptInput::Select(1)).unwrap();
            assert_eq!(kind(&step), Some(0));
            assert!(step.effects.is_empty());
            assert_eq!(
                home.respond(ScriptInput::Next).unwrap().effects,
                [Effect::SaveLocation(WORLD_TOUR_RETURN, None), warp]
            );
        }
        assert!(NpcScript::for_npc(AUDREY, &rider(0, 0, 541000000)).is_none());
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
        // Both say how many it takes.
        assert!(
            short.contains("10") && clear.contains("10"),
            "{short} {clear}"
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
    fn tory_takes_a_ready_party_up_and_tells_anyone_how_it_is_played() {
        let mut ready = NpcScript::tory(Entry::Ready);
        assert_eq!(kind(&ready.begin()), Some(4));
        let ask = ready.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(kind(&ask), Some(1));
        assert_eq!(
            ready.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::StartEvent(EventKind::Hpq)]
        );
        for entry in [
            Entry::NoParty,
            Entry::NotLeader,
            Entry::NotEligible,
            Entry::Busy,
        ] {
            let mut tory = NpcScript::tory(entry);
            tory.begin();
            let said = tory.respond(ScriptInput::Select(0)).unwrap();
            assert_eq!(kind(&said), Some(0));
            assert!(tory.respond(ScriptInput::Next).unwrap().effects.is_empty());
        }
        // How it is played: three pages, whatever the party.
        let mut tory = NpcScript::tory(Entry::NoParty);
        tory.begin();
        tory.respond(ScriptInput::Select(1)).unwrap();
        tory.respond(ScriptInput::Next).unwrap();
        let last = tory.respond(ScriptInput::Next).unwrap();
        assert!(last.prompt.is_some());
        assert!(tory.respond(ScriptInput::Next).unwrap().prompt.is_none());
    }

    #[test]
    fn growlie_and_tommy_let_players_off_primrose_hill_then_home() {
        let mut growlie = NpcScript::for_npc(GROWLIE, &rider(100, 0, 910010000)).unwrap();
        assert_eq!(kind(&growlie.begin()), Some(4));
        growlie.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(
            growlie.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::Warp(910010300)]
        );
        let mut tommy = NpcScript::for_npc(TOMMY, &rider(100, 0, 910010300)).unwrap();
        assert_eq!(kind(&tommy.begin()), Some(1));
        assert_eq!(
            tommy.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::Warp(100000200)]
        );
        assert_eq!(NpcScript::for_npc(GROWLIE, &rider(100, 0, 100000200)), None);
        assert_eq!(NpcScript::for_npc(TOMMY, &rider(100, 0, 910010000)), None);
    }

    #[test]
    fn nella_lets_players_out_of_the_party_quest_then_home() {
        let mut asked = vec![];
        for (from, to) in [
            (103000801, 103000890),
            (events::KPQ_BONUS, 103000890),
            (103000890, 103000000),
        ] {
            let mut nella = NpcScript::for_npc(NELLA, &rider(100, 0, from)).unwrap();
            let page = nella.begin();
            assert_eq!(kind(&page), Some(1));
            asked.push(page.prompt);
            assert_eq!(
                nella.respond(ScriptInput::Yes).unwrap().effects,
                [Effect::Warp(to)]
            );
        }
        // The bonus stage's question is its own.
        assert!(asked[1] != asked[0] && asked[1] != asked[2], "{asked:?}");
        assert_eq!(NpcScript::for_npc(NELLA, &rider(100, 0, 103000000)), None);
    }

    #[test]
    fn cloto_gives_each_winner_a_reward_and_sends_them_to_the_bonus() {
        let mut player = rider(100, 0, events::KPQ.stage_map(5));
        use rand::SeedableRng;
        let rng = &mut rand::rngs::StdRng::seed_from_u64(7);
        let won = NpcScript::cloto(Cloto::Won, &player).begin();
        assert!(page_text(&won).contains(&text(keys::KPQ_CLOTO_S5_WON)));
        let mut script = cloto_reward(&player, rng);
        assert_eq!(kind(&script.begin()), Some(0));
        let effects = script.respond(ScriptInput::Next).unwrap().effects;
        let [Effect::GiveItem { item_id, quantity }, Effect::WarpToPortal(bonus, _)] = effects[..]
        else {
            panic!("{effects:?}");
        };
        assert!(events::KPQ_REWARDS.contains(&(item_id, quantity)));
        assert_eq!(bonus, events::KPQ_BONUS);
        // Should the give fail all the same, she says so and keeps them.
        let refused = script.inventory_full().unwrap();
        assert!(page_text(&refused).contains(&text(keys::KPQ_CLOTO_S5_FULL)));

        // A full Etc tab: nothing yet, whatever the reward would be.
        let inventory = &mut player.inventory;
        inventory.set_slot_limit(InventoryType::Etc, 1);
        inventory.insert(InventoryType::Etc, 1, game::Item::stack(4000000, 1));
        let mut script = cloto_reward(&player, rng);
        script.begin();
        let full = script.respond(ScriptInput::Next).unwrap();
        assert!(full.effects.is_empty());
        assert!(page_text(&full).contains(&text(keys::KPQ_CLOTO_S5_FULL)));
    }

    #[test]
    fn the_red_sign_takes_a_ready_party_in_and_tells_anyone_what_it_is() {
        let mut ready = NpcScript::red_sign(Entry::Ready);
        assert_eq!(kind(&ready.begin()), Some(4));
        let step = ready.respond(ScriptInput::Select(0)).unwrap();
        assert!(step.prompt.is_none());
        assert_eq!(step.effects, [Effect::StartEvent(EventKind::Lpq)]);
        assert!(ready.refused().is_some());
        for entry in [
            Entry::NoParty,
            Entry::NotLeader,
            Entry::NotEligible,
            Entry::Busy,
        ] {
            let mut sign = NpcScript::red_sign(entry);
            sign.begin();
            let said = sign.respond(ScriptInput::Select(0)).unwrap();
            assert_eq!(kind(&said), Some(0));
            assert!(sign.respond(ScriptInput::Next).unwrap().effects.is_empty());
        }
        let mut sign = NpcScript::red_sign(Entry::NoParty);
        sign.begin();
        assert_eq!(
            kind(&sign.respond(ScriptInput::Select(1)).unwrap()),
            Some(0)
        );
        assert!(sign.respond(ScriptInput::Next).unwrap().prompt.is_none());
    }

    #[test]
    fn wonky_takes_a_ready_party_into_the_tower_and_lets_players_out() {
        let mut ready = NpcScript::wonky(Entry::Ready);
        assert_eq!(kind(&ready.begin()), Some(4));
        let step = ready.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(step.effects, [Effect::StartEvent(EventKind::Opq)]);
        assert!(ready.refused().is_some());
        for entry in [
            Entry::NoParty,
            Entry::NotLeader,
            Entry::NotEligible,
            Entry::Busy,
        ] {
            let mut wonky = NpcScript::wonky(entry);
            wonky.begin();
            assert_eq!(
                kind(&wonky.respond(ScriptInput::Select(0)).unwrap()),
                Some(0)
            );
        }

        // On the cloud stage he lets a player out; Eak sends them home.
        let inside = rider(100, 0, events::OPQ.entry);
        let mut out = NpcScript::for_npc(WONKY, &inside).unwrap();
        assert_eq!(kind(&out.begin()), Some(1));
        let effects = out.respond(ScriptInput::Yes).unwrap().effects;
        assert_eq!(effects, [Effect::Warp(events::OPQ.exit)]);
        let left = rider(100, 0, events::OPQ.exit);
        assert!(NpcScript::for_npc(WONKY, &left).is_none());
        let mut eak = NpcScript::for_npc(EAK, &left).unwrap();
        assert_eq!(kind(&eak.begin()), Some(0));
        let effects = eak.respond(ScriptInput::Next).unwrap().effects;
        assert_eq!(effects, [Effect::Warp(events::OPQ.lobby)]);
        assert!(NpcScript::for_npc(EAK, &inside).is_none());
    }

    #[test]
    fn eak_tells_the_leader_of_minerva_then_clears_the_cloud_stage() {
        let cloud = events::OPQ.entry;
        let mut leader = NpcScript::opq_eak(cloud, true, false).unwrap();
        let step = leader.begin();
        assert_eq!(
            (kind(&step), page_text(&step)),
            (Some(0), text(keys::OPQ_EAK_CLOUD))
        );
        let effects = leader.respond(ScriptInput::Next).unwrap().effects;
        assert_eq!(effects, [Effect::EventCommand(EventCommand::ClearStage(1))]);
        // The others, and the leader after, go up to the top.
        let top = crate::scripts::named(cloud, "st00");
        for (leader, cleared) in [(false, false), (false, true), (true, true)] {
            let step = NpcScript::opq_eak(cloud, leader, cleared).unwrap().begin();
            assert!(step.prompt.is_none());
            assert_eq!(step.effects, [Effect::WarpToPortal(cloud, top)]);
        }
        let said = |leader| {
            page_text(
                &NpcScript::opq_eak(events::OPQ_HUB, leader, true)
                    .unwrap()
                    .begin(),
            )
        };
        assert_eq!(said(true), text(keys::OPQ_EAK_GATHER));
        assert_eq!(said(false), text(keys::OPQ_EAK_LEADER_ONLY));
        assert!(NpcScript::opq_eak(920_010_200, true, true).is_none());
        let garden = NpcScript::opq_eak(events::OPQ_GARDEN, false, true).unwrap();
        assert_eq!(page_text(&garden.begin()), text(keys::OPQ_EAK_GARDEN));
    }

    #[test]
    fn eak_takes_the_leader_alone_on_from_the_storage_once_its_guards_are_gone() {
        let storage = |leader| NpcScript::opq_eak(events::OPQ_STORAGE, leader, true).unwrap();
        let other = storage(false).begin();
        assert_eq!(page_text(&other), text(keys::OPQ_EAK_LEADER_ONLY));
        let mut leader = storage(true);
        let step = leader.begin();
        assert!(step.prompt.is_none());
        let st00 = crate::scripts::named(events::OPQ_BONUS, "st00");
        assert_eq!(
            step.effects,
            [
                Effect::RequireNoMobs,
                Effect::WarpToPortal(events::OPQ_BONUS, st00)
            ]
        );
        // While a guard is left he says so instead.
        let said = leader.refused().unwrap();
        assert_eq!(page_text(&said), text(keys::OPQ_EAK_STORAGE));
    }

    #[test]
    fn eak_takes_the_team_to_the_garden_once_the_statue_has_its_pieces() {
        let hub = |statue| NpcScript::opq_hub(statue).begin();
        assert_eq!(
            page_text(&hub(Statue::Gathering)),
            text(keys::OPQ_EAK_GATHER)
        );
        let garden = hub(Statue::Garden);
        assert!(garden.prompt.is_none());
        assert_eq!(
            garden.effects,
            [Effect::EventCommand(EventCommand::MoveTeam(
                events::OPQ_GARDEN
            ))]
        );
        assert_eq!(
            page_text(&hub(Statue::GrassOfLife)),
            text(keys::OPQ_EAK_GRASS)
        );
        assert_eq!(page_text(&hub(Statue::Won)), text(keys::OPQ_EAK_MINERVA));
    }

    #[test]
    fn eak_gives_the_second_piece_himself_once_the_storage_room_is_bare() {
        let said = |script: NpcScript| {
            let step = script.begin();
            (page_text(&step), step.effects)
        };
        assert_eq!(
            said(NpcScript::opq_salion(false, false)),
            (text(keys::OPQ_EAK_SALION_FIND), vec![])
        );
        assert_eq!(
            said(NpcScript::opq_salion(true, false)),
            (
                text(keys::OPQ_EAK_SALION_FOUND),
                vec![
                    Effect::GiveItem {
                        item_id: events::OPQ_SALION_PIECE,
                        quantity: 1,
                    },
                    Effect::EventCommand(EventCommand::ClearStage(events::OPQ_SALION_STAGE)),
                ]
            )
        );
        for bare in [false, true] {
            assert_eq!(
                said(NpcScript::opq_salion(bare, true)),
                (text(keys::OPQ_EAK_ROOM_CLEARED), vec![])
            );
        }
        let mut full = NpcScript::opq_salion(true, false);
        full.begin();
        assert_eq!(
            page_text(&full.inventory_full().unwrap()),
            text(keys::OPQ_EAK_FULL)
        );
    }

    #[test]
    fn eak_brings_out_a_stone_box_once_the_music_plays_or_the_heads_are_right() {
        let said = |script: NpcScript| {
            let step = script.begin();
            (page_text(&step), step.effects)
        };
        let box_out = |name, stage| {
            vec![
                Effect::SetReactorState { name, state: 1 },
                Effect::EventCommand(EventCommand::ClearStage(stage)),
            ]
        };
        assert_eq!(
            said(NpcScript::opq_music(false, false)),
            (text(keys::OPQ_EAK_MUSIC_ASK), vec![])
        );
        assert_eq!(
            said(NpcScript::opq_music(true, false)),
            (
                text(keys::OPQ_EAK_MUSIC_PLAYED),
                box_out("stone3", events::OPQ_MUSIC_STAGE)
            )
        );
        assert_eq!(
            said(NpcScript::opq_music(true, true)),
            (text(keys::OPQ_EAK_MUSIC_DONE), vec![])
        );
        let heads = |count| said(NpcScript::opq_head_count(count));
        assert_eq!(
            heads(Some(HeadCount::Right)),
            (
                text(keys::OPQ_EAK_HEADS_RIGHT),
                box_out("stone4", events::OPQ_HEAD_COUNT_STAGE)
            )
        );
        for (count, key) in [
            (Some(HeadCount::Few), keys::OPQ_EAK_HEADS_FEW),
            (
                Some(HeadCount::Wrong { right: 1 }),
                keys::OPQ_EAK_HEADS_ONE_RIGHT,
            ),
            (
                Some(HeadCount::Wrong { right: 0 }),
                keys::OPQ_EAK_HEADS_ALL_WRONG,
            ),
            (None, keys::OPQ_EAK_HEADS_DONE),
        ] {
            assert_eq!(heads(count), (text(key), vec![]));
        }
        let levers = |check| said(NpcScript::opq_levers(check));
        assert_eq!(
            levers(Some(LeverCheck::Right)),
            (
                text(keys::OPQ_EAK_LEVERS_RIGHT),
                box_out("stone6", events::OPQ_LEVER_STAGE)
            )
        );
        for (check, key) in [
            (Some(LeverCheck::Few), keys::OPQ_EAK_LEVERS_ASK),
            (
                Some(LeverCheck::Wrong { one_right: true }),
                keys::OPQ_EAK_LEVERS_ONE_RIGHT,
            ),
            (
                Some(LeverCheck::Wrong { one_right: false }),
                keys::OPQ_EAK_LEVERS_NONE_RIGHT,
            ),
            (None, keys::OPQ_EAK_LEVERS_DONE),
        ] {
            assert_eq!(levers(check), (text(key), vec![]));
        }
    }

    #[test]
    fn the_blue_balloon_clears_stage_8_only_for_five_on_the_right_crates() {
        let said = |view, stood| {
            let step = NpcScript::blue_balloon(8, view, stood).begin();
            (step.effects.clone(), page_text(&step))
        };
        let answer = Balloon::Answer {
            combo: 0b1_0010_1111,
        };
        let (effects, _) = said(answer, Some(Standing::Right));
        assert_eq!(effects, [Effect::EventCommand(EventCommand::ClearStage(8))]);
        for (view, stood, key) in [
            (answer, Some(Standing::Wrong), keys::LPQ_BALLOON_WRONG),
            (answer, Some(Standing::Few), keys::LPQ_BALLOON_FEW),
            (Balloon::Intro, None, keys::LPQ_BALLOON_INTRO_8),
            (Balloon::NotLeader, None, keys::LPQ_BALLOON_NOT_LEADER),
            (Balloon::Cleared, None, keys::LPQ_BALLOON_HURRY),
        ] {
            let (effects, said) = said(view, stood);
            assert!(effects.is_empty(), "{view:?}");
            assert!(said.contains(&text(key)), "{view:?}: {said}");
        }
    }

    #[test]
    fn the_pink_balloon_lets_players_off_the_bonus_stage_early() {
        let bonus = events::LPQ.bonus.as_ref().unwrap();
        let player = rider(100, 0, bonus.map);
        let ask = || NpcScript::in_event(PINK_BALLOON, &player, false, true).unwrap();
        let mut pink = ask();
        assert_eq!(kind(&pink.begin()), Some(1));
        let effects = pink.respond(ScriptInput::Yes).unwrap().effects;
        assert!(
            matches!(effects[..], [Effect::WarpToPortal(map, _)] if map == bonus.exit),
            "{effects:?}"
        );
        let stay = ask().respond(ScriptInput::No).unwrap();
        assert!(page_text(&stay).contains(&text(keys::LPQ_PINK_STAY)));
        assert!(stay.effects.is_empty());
        let elsewhere = rider(100, 0, bonus.exit);
        assert!(NpcScript::in_event(PINK_BALLOON, &elsewhere, false, true).is_none());
    }

    #[test]
    fn arturo_gives_each_winner_a_reward_with_room_for_any_of_them() {
        let reward_map = events::LPQ.bonus.as_ref().unwrap().exit;
        let mut player = rider(100, 0, reward_map);
        use rand::SeedableRng;
        let rng = &mut rand::rngs::StdRng::seed_from_u64(7);
        let mut script = arturo(&player, rng).unwrap();
        assert_eq!(kind(&script.begin()), Some(0));
        let effects = script.respond(ScriptInput::Next).unwrap().effects;
        let [Effect::GiveItem { item_id, quantity }, Effect::Warp(home)] = effects[..] else {
            panic!("{effects:?}");
        };
        assert!(events::LPQ_REWARDS.contains(&(item_id, quantity)));
        assert_eq!(home, events::LPQ.lobby);
        // Should the give fail all the same, he says so and keeps them.
        let refused = script.inventory_full().unwrap();
        assert!(page_text(&refused).contains(&text(keys::LPQ_ARTURO_FULL)));
        assert!(NpcScript::in_event(ARTURO, &player, false, false).is_none());

        // A full Use tab: nothing yet, whatever the reward would be.
        let inventory = &mut player.inventory;
        inventory.set_slot_limit(InventoryType::Use, 1);
        inventory.insert(InventoryType::Use, 1, game::Item::stack(2000000, 1));
        let mut script = arturo(&player, rng).unwrap();
        script.begin();
        let full = script.respond(ScriptInput::Next).unwrap();
        assert!(full.effects.is_empty());
        assert!(page_text(&full).contains(&text(keys::LPQ_ARTURO_FULL)));
    }

    #[test]
    fn minerva_sends_each_winner_to_her_prize_room_then_gives_a_prize() {
        let mut player = rider(60, 0, events::OPQ_HUB);
        assert!(NpcScript::in_event(MINERVA, &player, true, false).is_none());
        let mut ask = NpcScript::in_event(MINERVA, &player, false, true).unwrap();
        assert_eq!(question(&ask.begin()), text(keys::OPQ_MINERVA_ASK));
        let to = events::OPQ_PRIZE_ROOM;
        let went = ask.respond(ScriptInput::Yes).unwrap().effects;
        assert_eq!(went, [Effect::WarpToPortal(to, 0)]);
        let mut ask = NpcScript::in_event(MINERVA, &player, false, true).unwrap();
        ask.begin();
        assert!(ask.respond(ScriptInput::No).unwrap().effects.is_empty());

        // In the treasure room she sends the leader on there too.
        player.stats.map_id = events::OPQ_BONUS;
        let mut ask = NpcScript::in_event(MINERVA, &player, true, true).unwrap();
        let asked = ask.begin();
        assert_eq!(question(&asked), text(keys::OPQ_MINERVA_BONUS_ASK));
        let went = ask.respond(ScriptInput::Yes).unwrap().effects;
        assert_eq!(went, [Effect::WarpToPortal(to, 0)]);

        player.stats.map_id = to;
        use rand::SeedableRng;
        let rng = &mut rand::rngs::StdRng::seed_from_u64(7);
        let mut script = minerva(&player, rng).unwrap();
        assert_eq!(page_text(&script.begin()), text(keys::OPQ_MINERVA_THANKS));
        let effects = script.respond(ScriptInput::Next).unwrap().effects;
        let [Effect::GiveItem { item_id, quantity }, Effect::WarpToPortal(home, 0)] = effects[..]
        else {
            panic!("{effects:?}");
        };
        assert!(events::OPQ_REWARDS.contains(&(item_id, quantity)));
        assert_eq!(home, events::OPQ.lobby);

        // A full Etc tab: nothing yet, whatever the prize would be.
        let inventory = &mut player.inventory;
        inventory.set_slot_limit(InventoryType::Etc, 1);
        inventory.insert(InventoryType::Etc, 1, game::Item::stack(4000000, 1));
        let mut script = minerva(&player, rng).unwrap();
        script.begin();
        let full = script.respond(ScriptInput::Next).unwrap();
        assert!(full.effects.is_empty());
        assert!(page_text(&full).contains(&text(keys::OPQ_MINERVA_FULL)));
        player.stats.map_id = events::OPQ.lobby;
        assert!(minerva(&player, rng).is_none());
    }

    #[test]
    fn anderson_lets_players_out_of_the_dimensional_crack_then_down() {
        for from in [922010100, 922010201, 922010900] {
            let mut anderson = NpcScript::for_npc(ANDERSON, &rider(100, 0, from)).unwrap();
            assert_eq!(kind(&anderson.begin()), Some(1));
            assert_eq!(
                anderson.respond(ScriptInput::Yes).unwrap().effects,
                [Effect::Warp(922010000)]
            );
        }
        let mut anderson = NpcScript::for_npc(ANDERSON, &rider(100, 0, 922010000)).unwrap();
        assert_eq!(kind(&anderson.begin()), Some(0));
        assert_eq!(
            anderson.respond(ScriptInput::Next).unwrap().effects,
            [Effect::Warp(221024500)]
        );
        assert_eq!(
            NpcScript::for_npc(ANDERSON, &rider(100, 0, 221024500)),
            None
        );
    }

    #[test]
    fn a_balloon_takes_its_stage_s_passes_from_a_leader_who_heard_it_out() {
        let first = events::LPQ.hand_in(1).unwrap();
        let holding = |quantity| hunter(&[(events::LPQ_PASS, quantity)], &[]);
        let says = |view, passes| {
            let page = NpcScript::balloon(first, view, &holding(passes)).begin();
            assert_eq!(kind(&page), Some(0));
            (page_text(&page), page.effects)
        };
        let item = events::LPQ_PASS.to_string();
        let get = |key| content::text().get(key, &[("item", &item), ("n", "25")]);
        for (view, key) in [
            (Balloon::Cleared, keys::LPQ_BALLOON_HURRY),
            (Balloon::NotLeader, keys::LPQ_BALLOON_NOT_LEADER),
            (Balloon::Intro, keys::LPQ_BALLOON_INTRO_1),
            (Balloon::Ready, keys::LPQ_BALLOON_SHORT),
        ] {
            let (said, effects) = says(view, 24);
            assert!(said.contains(&get(key)), "{said}");
            assert!(effects.is_empty());
        }
        let (said, effects) = says(Balloon::Ready, 25);
        assert!(said.contains(&text(keys::LPQ_BALLOON_DONE)), "{said}");
        assert_eq!(
            effects,
            [
                Effect::TakeItem {
                    item_id: events::LPQ_PASS,
                    quantity: 25
                },
                Effect::EventCommand(EventCommand::ClearStage(1))
            ]
        );
        let mut script = NpcScript::balloon(first, Balloon::Ready, &holding(25));
        assert_eq!(script.npc_id, 2040036);
        assert!(script.refused().is_some());
        for hand_in in events::LPQ.hand_ins {
            assert!(lpq_balloon_intro(hand_in.stage).ends_with(&hand_in.stage.to_string()));
        }
    }

    #[test]
    fn eak_trades_a_rooms_gatherings_for_a_piece_of_the_statue_at_once() {
        let lounge = events::OPQ.hand_in(6).unwrap();
        let holding = |quantity| hunter(&[(4_001_052, quantity)], &[]);
        let says = |view, quantity| {
            let page = NpcScript::balloon(lounge, view, &holding(quantity)).begin();
            assert_eq!(kind(&page), Some(0));
            (page_text(&page), page.effects)
        };
        let short = content::text().get(keys::OPQ_EAK_HAND_IN, &[("item", "4001052"), ("n", "40")]);
        for (view, said) in [
            (Balloon::Cleared, text(keys::OPQ_EAK_ROOM_CLEARED)),
            (Balloon::NotLeader, text(keys::OPQ_EAK_LEADER_ONLY)),
            (Balloon::Ready, short),
        ] {
            let (page, effects) = says(view, 39);
            assert!(page.contains(&said), "{page}");
            assert!(effects.is_empty());
        }
        let (said, effects) = says(Balloon::Ready, 40);
        let piece = content::text().get(keys::OPQ_EAK_PIECE, &[("piece", "4001048")]);
        assert!(said.contains(&piece), "{said}");
        assert_eq!(
            effects,
            [
                Effect::TakeItem {
                    item_id: 4_001_052,
                    quantity: 40
                },
                Effect::GiveItem {
                    item_id: 4_001_048,
                    quantity: 1
                },
                Effect::EventCommand(EventCommand::ClearStage(6))
            ]
        );
        let mut script = NpcScript::balloon(lounge, Balloon::Ready, &holding(40));
        assert_eq!(script.npc_id, EAK);
        assert!(script.refused().is_some());
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
                QuestStatus::Started {
                    kills: Vec::new(),
                    info: None,
                }
            };
            player.quests.upsert(game::QuestRecord { quest_id, status });
        }
        player
    }

    #[test]
    fn corba_makes_anyone_a_dragon_and_flies_them_off() {
        let mut corba = NpcScript::for_npc(CORBA, &with_quests(&[])).unwrap();
        corba.begin();
        assert_eq!(
            corba.respond(ScriptInput::Select(0)),
            Some(Step {
                prompt: None,
                effects: vec![
                    Effect::ApplyItemBuff(MINI_DRACO),
                    Effect::WarpToPortal(FLIGHT_TO_TEMPLE, 0)
                ],
            })
        );
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
    fn victoria_explains_the_cathedral_wedding() {
        let mut victoria = NpcScript::for_npc(VICTORIA, &with_quests(&[])).unwrap();
        let page = victoria.begin();
        assert_eq!(page_text(&page), text(keys::WEDDING_VICTORIA));
        assert_eq!(victoria.respond(ScriptInput::Next), Some(Step::default()));
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
        let started = || QuestStatus::Started {
            kills: Vec::new(),
            info: None,
        };

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

    /// Lenario for a guild master, with the menu entry `choice` picked.
    fn lenario_picks(player: &Player, view: AllianceView, choice: i32) -> (NpcScript, Step) {
        let mut lenario = NpcScript::lenario(player, view);
        assert_eq!(kind(&lenario.begin()), Some(4));
        let step = lenario.respond(ScriptInput::Select(choice)).unwrap();
        (lenario, step)
    }

    /// An alliance of the `testers` guild and another, with `master` its
    /// master and room for `capacity` guilds.
    fn union(master: i32, capacity: u8) -> Alliance {
        let mut own = testers(master, 10);
        own.alliance_id = 7;
        own.members[0].alliance_rank = alliance::MASTER_RANK;
        let mut other = testers(master + 10, 10);
        other.id = 2;
        other.alliance_id = 7;
        other.members[0].alliance_rank = alliance::GUILD_MASTER_RANK;
        Alliance {
            id: 7,
            name: "Union".into(),
            titles: alliance::DEFAULT_TITLES.map(String::from),
            capacity,
            notice: String::new(),
            guilds: vec![own, other],
        }
    }

    #[test]
    fn lenario_speaks_only_to_guild_masters() {
        let player = guilder(10, 0, true);
        let id = player.stats.id;
        for guild in [Some(testers(id - 1, 10)), None] {
            let view = AllianceView {
                guild: guild.as_ref(),
                ..Default::default()
            };
            let lenario = NpcScript::lenario(&player, view);
            assert_eq!(kind(&lenario.begin()), Some(0));
        }
    }

    #[test]
    fn lenario_founds_an_alliance_for_a_party_leader_who_pays_and_names_it() {
        let player = guilder(10, alliance::CREATE_COST, true);
        let guild = testers(player.stats.id, 10);
        let view = AllianceView {
            guild: Some(&guild),
            alliance: None,
            party_leader: true,
        };
        let (mut lenario, step) = lenario_picks(&player, view, 2);
        assert_eq!(kind(&step), Some(1));
        let name = lenario.respond(ScriptInput::Yes).unwrap();
        assert!(name.effects.is_empty());
        let Some(Prompt::AskText { min, max, .. }) = name.prompt else {
            panic!("a text prompt expected, got {:?}", name.prompt);
        };
        assert_eq!((min, max), (4, 12));
        // A name that can't be one asks again.
        let again = lenario.respond(ScriptInput::Text("a b".into())).unwrap();
        assert_eq!(kind(&again), Some(2));
        let confirm = lenario.respond(ScriptInput::Text("Union".into())).unwrap();
        assert_eq!(kind(&confirm), Some(1));
        let yes = lenario.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.prompt, None);
        assert_eq!(
            yes.effects,
            [Effect::CreateAlliance {
                name: "Union".into()
            }]
        );

        // Short of the price: asked, then told.
        let poor = guilder(10, alliance::CREATE_COST - 1, true);
        let (mut lenario, _) = lenario_picks(&poor, view, 2);
        let yes = lenario.respond(ScriptInput::Yes).unwrap();
        says_one_page(lenario, &yes);

        // Not the party's leader, or already in an alliance.
        let mut allied = testers(player.stats.id, 10);
        allied.alliance_id = 7;
        for view in [
            AllianceView {
                party_leader: false,
                ..view
            },
            AllianceView {
                guild: Some(&allied),
                ..view
            },
        ] {
            let (lenario, step) = lenario_picks(&player, view, 2);
            says_one_page(lenario, &step);
        }
    }

    #[test]
    fn lenario_grows_and_disbands_only_the_masters_alliance() {
        let player = guilder(10, alliance::GROW_COST, true);
        let id = player.stats.id;
        let own = union(id, 2);
        let view = AllianceView {
            guild: Some(&own.guilds[0]),
            alliance: Some(&own),
            party_leader: false,
        };
        let (mut lenario, _) = lenario_picks(&player, view, 3);
        let yes = lenario.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.effects, [Effect::GrowAlliance { from: 2 }]);
        let (mut lenario, _) = lenario_picks(&player, view, 4);
        let yes = lenario.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.effects, [Effect::DisbandAlliance]);

        // Full, or the other guild's master.
        let full = union(id, alliance::MAX_CAPACITY);
        let (lenario, step) = lenario_picks(
            &player,
            AllianceView {
                alliance: Some(&full),
                ..view
            },
            3,
        );
        says_one_page(lenario, &step);
        let other = union(id - 10, 2);
        let view = AllianceView {
            guild: Some(&other.guilds[1]),
            alliance: Some(&other),
            party_leader: false,
        };
        for choice in [3, 4] {
            let (lenario, step) = lenario_picks(&player, view, choice);
            says_one_page(lenario, &step);
        }
    }

    #[test]
    fn answers_must_fit_the_prompt() {
        let page = Prompt::Text(DialoguePage {
            npc_id: 1,
            text: String::new(),
            has_previous: false,
            has_next: true,
            player_speaks: false,
        });
        assert_eq!(input(&page, 0, 1, None, None), Some(ScriptInput::Next));
        assert_eq!(input(&page, 0, 0, None, None), None, "no previous page");
        assert_eq!(input(&page, 1, 1, None, None), None);
        let menu = Prompt::Selection {
            npc_id: 1,
            text: String::new(),
        };
        assert_eq!(
            input(&menu, 4, 1, Some(3), None),
            Some(ScriptInput::Select(3))
        );
        assert_eq!(input(&menu, 4, -1, None, None), None);
        let styles = Prompt::Style {
            npc_id: 1,
            text: String::new(),
            styles: vec![30000],
        };
        assert_eq!(
            input(&styles, 7, 1, Some(0), None),
            Some(ScriptInput::Select(0))
        );
        assert_eq!(input(&styles, 7, 0, None, None), None, "cancelled");
        assert_eq!(input(&styles, 4, 1, Some(0), None), None);
        let ask = Prompt::AskText {
            npc_id: 1,
            text: String::new(),
            min: 4,
            max: 12,
        };
        assert_eq!(
            input(&ask, 2, 1, None, Some("Union".into())),
            Some(ScriptInput::Text("Union".into()))
        );
        assert_eq!(input(&ask, 2, 0, None, None), None, "cancelled");
        let number = Prompt::AskNumber {
            npc_id: 1,
            text: String::new(),
            default: 1,
            min: 1,
            max: 100,
        };
        assert_eq!(
            input(&number, 3, 1, Some(100), None),
            Some(ScriptInput::Number(100))
        );
        assert_eq!(input(&number, 3, 1, Some(0), None), None, "below");
        assert_eq!(input(&number, 3, 1, Some(101), None), None, "above");
        assert_eq!(input(&number, 3, 0, None, None), None, "cancelled");
    }

    fn styles(step: &Step) -> Vec<i32> {
        match &step.prompt {
            Some(Prompt::Style { styles, .. }) => styles.clone(),
            other => panic!("styles expected, got {other:?}"),
        }
    }

    const NATALIE: i32 = 1012103;
    const HAIRCUT_COUPON: i32 = 5150001;

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
    fn dr_feeble_rolls_a_face_on_a_yes() {
        const COUPON: i32 = 5152000;
        let mut man = player(50);
        man.stats.gender = 0;
        man.inventory
            .insert(InventoryType::Cash, 1, game::Item::stack(COUPON, 1));
        let mut feeble = NpcScript::for_npc(1052005, &man).unwrap();
        assert_eq!(kind(&feeble.begin()), Some(4));
        let ask = feeble.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(kind(&ask), Some(1));
        let done = feeble.respond(ScriptInput::Yes).unwrap();
        let faces = stylists::services(1052005).next().unwrap().looks(&man);
        match &done.effects[..] {
            [Effect::TakeItem {
                item_id: COUPON,
                quantity: 1,
            }, Effect::SetStyle(face)] => assert!(faces.contains(&(*face, None))),
            other => panic!("a coupon for a face expected, got {other:?}"),
        }
        assert_eq!(kind(&done), Some(0));
    }

    #[test]
    fn a_lens_shop_says_so_to_a_player_without_one_time_lenses() {
        let mut shop = NpcScript::for_npc(9200100, &player(50)).unwrap();
        assert_eq!(kind(&shop.begin()), Some(4));
        let none = shop.respond(ScriptInput::Select(2)).unwrap();
        assert_eq!(page_text(&none), text(keys::STYLIST_NO_LENS));
        assert!(none.effects.is_empty());
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

    #[test]
    fn garnox_evolves_the_first_pet_out_with_a_rock() {
        if crate::items::data().is_err() {
            return;
        }
        let mut owner = player(50);
        owner.meso = EVOLUTION_FEE;
        let evolve = |owner: &Player, script| {
            let mut garnox = NpcScript::for_quest(script, owner).unwrap();
            assert_eq!(garnox.npc_id(), GARNOX);
            let step = garnox.begin();
            if kind(&step) != Some(1) {
                return None;
            }
            let done = garnox.respond(ScriptInput::Yes).unwrap();
            Some(done.effects)
        };
        assert_eq!(evolve(&owner, "q8185e"), None, "no pet out");

        let mut dragon = game::Item::stack(5000029, 1);
        dragon.cash_serial = Some(80);
        dragon.pet = Some(game::pet::PetData::new("Baby Dragon"));
        owner.inventory.insert(InventoryType::Cash, 1, dragon);
        owner.pets = vec![80];
        let effects = evolve(&owner, "q8185e").unwrap();
        let [Effect::TakeItem {
            item_id: world::ROCK_OF_EVOLUTION,
            quantity: 1,
        }, Effect::SpendMeso(EVOLUTION_FEE), Effect::EvolvePet {
            serial: 80,
            into,
            days: 90,
        }, Effect::CompleteQuest(8185)] = effects[..]
        else {
            panic!("{effects:?}");
        };
        assert!((5000030..=5000033).contains(&into), "{into}");

        // A grown dragon becomes another one.
        let grown = |owner: &mut Player, item_id| {
            let mut item = owner.inventory.get(InventoryType::Cash, 1).unwrap().clone();
            item.item_id = item_id;
            owner.inventory.insert(InventoryType::Cash, 1, item);
        };
        for _ in 0..20 {
            grown(&mut owner, 5000031);
            let effects = evolve(&owner, "q8189e").unwrap();
            let Some(Effect::EvolvePet { into, .. }) = effects.get(2) else {
                panic!("{effects:?}");
            };
            assert!([5000030, 5000032, 5000033].contains(into), "{into}");
            assert_eq!(effects[3], Effect::CompleteQuest(8189));
        }

        // The Baby Robo's upgrade takes its batteries too.
        grown(&mut owner, 5000048);
        let effects = evolve(&owner, "q4659e").unwrap();
        assert_eq!(
            effects[2],
            Effect::TakeItem {
                item_id: CHEAP_BATTERY,
                quantity: 50
            }
        );
        let Effect::EvolvePet { into, .. } = effects[3] else {
            panic!("{effects:?}");
        };
        assert!((5000049..=5000053).contains(&into), "{into}");

        owner.meso = EVOLUTION_FEE - 1;
        assert_eq!(evolve(&owner, "q4659e"), None, "the fee is short");
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

        // Rene only trains her own line, from level 70; younger, she
        // approves them for Zakum.
        let mut young = hunter(&[], &[]);
        young.stats.level = 69;
        assert_eq!(
            NpcScript::for_npc(RENE, &young).unwrap().begin().effects,
            [Effect::Mark(ZAKUM_APPROVED)]
        );
        let mut tylus = NpcScript::for_npc(2020008, &hunter(&[], &[])).unwrap();
        assert_eq!(kind(&tylus.begin()), Some(0));
        assert_eq!(tylus.respond(ScriptInput::Next).unwrap().effects, []);

        // She assigns the test, and Athena sends the player to the door.
        let mut rene = NpcScript::for_npc(RENE, &hunter(&[], &[])).unwrap();
        assert_eq!(kind(&rene.begin()), Some(4));
        rene.respond(ScriptInput::Select(0)).unwrap();
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
        let mut rene = NpcScript::for_npc(RENE, &sent).unwrap();
        assert_eq!(
            buttons(&rene.respond(ScriptInput::Select(0)).unwrap()),
            (false, false)
        );

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
            rene.respond(ScriptInput::Select(0)).unwrap().effects,
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
    fn tower_rocks_take_a_scroll_to_another_floor() {
        const FIRST_EOS_ROCK: i32 = 2040024;
        const THIRD_EOS_ROCK: i32 = 2040026;
        let none = NpcScript::for_npc(FIRST_EOS_ROCK, &hunter(&[], &[])).unwrap();
        assert_eq!(
            page_text(&none.begin()),
            text(keys::TOWER_ROCK_NONE).replace("{item}", "4001020")
        );
        let scroll = hunter(&[(EOS_ROCK_SCROLL, 1)], &[]);
        let take = Effect::TakeItem {
            item_id: EOS_ROCK_SCROLL,
            quantity: 1,
        };
        // One way down from the top, by the go00 portal beside the rock.
        let mut first = NpcScript::for_npc(FIRST_EOS_ROCK, &scroll).unwrap();
        assert_eq!(kind(&first.begin()), Some(1));
        assert_eq!(
            first.respond(ScriptInput::Yes).unwrap().effects,
            [take.clone(), Effect::WarpToPortal(221022900, 3)]
        );
        // Two ways from the middle: up or down.
        let mut third = NpcScript::for_npc(THIRD_EOS_ROCK, &scroll).unwrap();
        assert_eq!(kind(&third.begin()), Some(4));
        assert_eq!(
            kind(&third.respond(ScriptInput::Select(1)).unwrap()),
            Some(1)
        );
        assert_eq!(
            third.respond(ScriptInput::Yes).unwrap().effects,
            [take, Effect::WarpToPortal(221020000, 4)]
        );
        let mut third = NpcScript::for_npc(THIRD_EOS_ROCK, &scroll).unwrap();
        third.begin();
        third.respond(ScriptInput::Select(0)).unwrap();
        assert!(third
            .respond(ScriptInput::No)
            .is_none_or(|step| step.effects.is_empty()));
        // Orbis Tower's rocks want the Orbis scroll.
        let orbis = hunter(&[(ORBIS_ROCK_SCROLL, 1)], &[]);
        let mut spot = NpcScript::for_npc(2012015, &orbis).unwrap();
        spot.begin();
        assert_eq!(
            spot.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::TakeItem {
                    item_id: ORBIS_ROCK_SCROLL,
                    quantity: 1
                },
                Effect::WarpToPortal(200080200, 0)
            ]
        );
        let eos_at_orbis = NpcScript::for_npc(2012015, &scroll).unwrap();
        assert_eq!(kind(&eos_at_orbis.begin()), Some(0));
    }

    #[test]
    fn patience_piles_give_the_quest_items_or_a_prize_and_send_the_player_out() {
        use rand::SeedableRng;
        let rng = &mut rand::rngs::StdRng::seed_from_u64(7);
        let [flowers, _, pink, ..] = &PATIENCE_PILES;
        let found = |pile, player: &Player, rng: &mut _| {
            let script = patience_pile(pile, player, rng);
            assert_eq!(script.npc_id, pile.npc);
            script.begin().effects
        };
        let gift = |item_id, quantity| Effect::GiveItem { item_id, quantity };
        // On John's quest, the pink flowers' ten instead of a prize.
        let on_quest = with_quests(&[(2052, false)]);
        assert_eq!(
            found(pink, &on_quest, rng),
            [gift(4031025, 10), Effect::WarpToPortal(105040300, 0)]
        );
        let mut done = on_quest.clone();
        done.inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(4031025, 10));
        let prize = found(pink, &done, rng);
        assert!(
            matches!(prize[..], [Effect::GiveItem { item_id, quantity: 3 }, _]
            if (4010000..=4010005).contains(&item_id))
        );
        // In Ellinia's forest, the quest item and a prize too.
        let effects = found(flowers, &with_quests(&[(2050, false)]), rng);
        assert_eq!(effects[0], gift(4031020, 1));
        assert_eq!(effects[2], Effect::WarpToPortal(ELLINIA, 0));
        // Clothes come for the player's gender, or for both.
        let mut woman = with_quests(&[]);
        woman.stats.gender = 1;
        for _ in 0..100 {
            let effects = found(flowers, &woman, rng);
            let [Effect::GiveItem { item_id, .. }, _] = effects[..] else {
                panic!("{effects:?}");
            };
            assert_ne!(item_id / 1000 % 10, 0, "{item_id}");
        }
        // Every prize is an item we know.
        for pile in &PATIENCE_PILES {
            for &(item_id, ..) in pile.prizes {
                assert!(crate::items::info(item_id).is_some(), "{item_id}");
            }
        }
    }

    #[test]
    fn line3_sells_site_tickets_and_takes_their_holders_in_and_out() {
        let menu = |step: Step| match step.prompt {
            Some(Prompt::Selection { text, .. }) => text,
            other => panic!("menu expected, got {other:?}"),
        };
        // Jake sells a ticket for each of Shumi's quests begun.
        let mut buyer = with_quests(&[(2055, true), (2056, false)]);
        buyer.meso = SITE_TICKET_PRICE;
        let mut jake = NpcScript::for_npc(JAKE, &buyer).unwrap();
        assert_eq!(kind(&jake.begin()), Some(0));
        let tickets = menu(jake.respond(ScriptInput::Next).unwrap());
        assert!(tickets.contains("#t4031037#") && !tickets.contains("#t4031038#"));
        assert_eq!(
            jake.respond(ScriptInput::Select(1)).unwrap().effects,
            [
                Effect::SpendMeso(SITE_TICKET_PRICE),
                Effect::GiveItem {
                    item_id: 4031037,
                    quantity: 1
                }
            ]
        );
        assert!(jake.inventory_full().is_some());
        // Short of the price, a word and no ticket.
        buyer.meso = SITE_TICKET_PRICE - 1;
        let mut jake = NpcScript::for_npc(JAKE, &buyer).unwrap();
        jake.begin();
        jake.respond(ScriptInput::Next);
        let short = jake.respond(ScriptInput::Select(0)).unwrap();
        assert!(short.effects.is_empty());
        assert_eq!(kind(&short), Some(0));
        // Before the quests, only the greeting.
        let mut jake = NpcScript::for_npc(JAKE, &with_quests(&[])).unwrap();
        jake.begin();
        assert!(jake
            .respond(ScriptInput::Next)
            .is_none_or(|step| step.prompt.is_none()));

        // The gate's second choice offers the site tickets held.
        let mut holder = traveller(30, 0, false);
        for (slot, ticket) in [(1, 4031036), (2, 4031038)] {
            holder
                .inventory
                .insert(InventoryType::Etc, slot, game::Item::stack(ticket, 1));
        }
        let mut gate = NpcScript::for_npc(KERNING_CITY_GATE, &holder).unwrap();
        gate.begin();
        let held = menu(gate.respond(ScriptInput::Select(1)).unwrap());
        assert!(held.contains("#t4031036#") && !held.contains("#t4031037#"));
        let b3 = 103000906;
        assert_eq!(
            gate.respond(ScriptInput::Select(1)).unwrap().effects,
            [
                Effect::TakeItem {
                    item_id: 4031038,
                    quantity: 1
                },
                Effect::WarpToPortal(b3, crate::scripts::named(b3, "st00")),
            ]
        );
        // Without one, a word.
        let mut gate = NpcScript::for_npc(KERNING_CITY_GATE, &traveller(30, 0, false)).unwrap();
        gate.begin();
        let none = gate.respond(ScriptInput::Select(1)).unwrap();
        assert!(none.effects.is_empty());
        assert_eq!(kind(&none), Some(0));

        // The box at B2's end: Shumi's money on her quest, else an ore.
        use rand::SeedableRng;
        let rng = &mut rand::rngs::StdRng::seed_from_u64(7);
        let b2 = PATIENCE_PILES
            .iter()
            .find(|pile| pile.npc == 1052009)
            .unwrap();
        assert_eq!(
            patience_pile(b2, &with_quests(&[(2056, false)]), rng)
                .begin()
                .effects,
            [
                Effect::GiveItem {
                    item_id: 4031040,
                    quantity: 1
                },
                Effect::WarpToPortal(SUBWAY_TICKETING, 0)
            ]
        );
        let ore = patience_pile(b2, &with_quests(&[]), rng).begin().effects;
        assert!(
            matches!(ore[..], [Effect::GiveItem { item_id, quantity: 1 }, _]
            if b2.prizes.iter().any(|prize| prize.0 == item_id))
        );

        // The way out asks, then goes back to the ticketing booth.
        let mut exit = NpcScript::for_npc(SITE_EXIT, &holder).unwrap();
        assert_eq!(kind(&exit.begin()), Some(0));
        assert_eq!(kind(&exit.respond(ScriptInput::Next).unwrap()), Some(1));
        assert_eq!(
            exit.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::WarpToPortal(SUBWAY_TICKETING, 0)]
        );
    }

    #[test]
    fn adobis_lets_a_ready_party_into_the_dead_mine_and_aura_takes_the_ore() {
        let approved = hunter(&[], &[ZAKUM_APPROVED]);
        let mine = |entry| {
            let mut adobis = NpcScript::adobis(entry, &approved);
            adobis.begin();
            adobis.respond(ScriptInput::Select(0)).unwrap()
        };
        assert_eq!(
            mine(Entry::Ready).effects,
            [Effect::StartEvent(EventKind::Zpq)]
        );
        assert_eq!(
            page_text(&mine(Entry::Busy)),
            text(keys::ZAKUM_ADOBIS_MINE_BUSY)
        );
        assert!(page_text(&mine(Entry::NotEligible)).contains("50"));
        // The unapproved hear the warning, party or not.
        let warned = NpcScript::adobis(Entry::Ready, &hunter(&[], &[]));
        assert_eq!(
            page_text(&warned.begin()),
            text(keys::ZAKUM_ADOBIS_UNAPPROVED)
        );

        // Aura: only the leader hands in, and only with the Fire Ore.
        let hand_in = |leader, items: &[(i32, i16)]| {
            let mut aura = NpcScript::aura(Aura::Playing { leader }, &hunter(items, &[]));
            assert_eq!(kind(&aura.begin()), Some(4));
            let step = aura.respond(ScriptInput::Select(1)).unwrap();
            (aura, step)
        };
        let ore = (events::FIRE_ORE, 1);
        let says = |step: &Step| page_text(step);
        assert_eq!(
            says(&hand_in(false, &[ore]).1),
            text(keys::ZAKUM_AURA_LEADER_ONLY)
        );
        assert_eq!(says(&hand_in(true, &[]).1), text(keys::ZAKUM_AURA_NEED_ORE));
        let take = |item_id, quantity| Effect::TakeItem { item_id, quantity };
        let (mut aura, step) = hand_in(true, &[ore, (events::ZPQ_DOCUMENT, 29)]);
        assert_eq!(question(&step), text(keys::ZAKUM_AURA_ASK));
        assert_eq!(
            aura.respond(ScriptInput::Yes).unwrap().effects,
            [
                take(events::FIRE_ORE, 1),
                Effect::EventCommand(EventCommand::ClearStage(1))
            ]
        );
        let (mut aura, step) = hand_in(true, &[ore, (events::ZPQ_DOCUMENT, 30)]);
        assert_eq!(question(&step), text(keys::ZAKUM_AURA_ASK_DOCUMENTS));
        assert_eq!(
            aura.respond(ScriptInput::Yes).unwrap().effects,
            [
                take(events::FIRE_ORE, 1),
                take(events::ZPQ_DOCUMENT, 30),
                Effect::EventCommand(EventCommand::WinWithDocuments)
            ]
        );
        assert!(aura.refused().is_some());
        // Anyone may leave.
        let mut aura = NpcScript::aura(Aura::Playing { leader: false }, &hunter(&[], &[]));
        aura.begin();
        aura.respond(ScriptInput::Select(2)).unwrap();
        assert_eq!(
            aura.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::Warp(events::ZPQ.exit)]
        );

        // After the win, each member's share, once.
        let share = |documents, taken| {
            let view = Aura::Won { documents, taken };
            let mut aura = NpcScript::aura(view, &hunter(&[], &[]));
            assert_eq!(page_text(&aura.begin()), text(keys::ZAKUM_AURA_WON));
            let step = aura.respond(ScriptInput::Next).unwrap();
            (aura, step)
        };
        let give = |item_id, quantity| Effect::GiveItem { item_id, quantity };
        let (mut aura, step) = share(false, false);
        assert_eq!(
            step.effects,
            [
                give(events::FIRE_ORE_PIECE, 1),
                Effect::EventCommand(EventCommand::PrizeTaken)
            ]
        );
        assert!(aura.inventory_full().is_some());
        let (scroll, scrolls) = events::DEAD_MINE_SCROLL;
        assert_eq!(
            share(true, false).1.effects,
            [
                give(scroll, scrolls),
                give(events::FIRE_ORE_PIECE, 1),
                Effect::EventCommand(EventCommand::PrizeTaken)
            ]
        );
        let (_, again) = share(true, true);
        assert!(again.effects.is_empty());
        assert_eq!(page_text(&again), text(keys::ZAKUM_AURA_PRIZE_TAKEN));

        // Ali sends whoever fell in the mine back to the Door to Zakum.
        let mut ali = NpcScript::for_npc(ALI, &hunter(&[], &[])).unwrap();
        ali.begin();
        assert_eq!(
            ali.respond(ScriptInput::Next).unwrap().effects,
            [Effect::Warp(DOOR_TO_ZAKUM)]
        );
    }

    #[test]
    fn the_gatekeeper_lets_a_ready_party_into_the_trial_grounds_and_takes_its_keys() {
        let mut ready = NpcScript::htpq_gatekeeper(Entry::Ready);
        assert_eq!(kind(&ready.begin()), Some(4));
        let step = ready.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(step.effects, [Effect::StartEvent(EventKind::Htpq)]);
        assert!(ready.refused().is_some());
        for entry in [
            Entry::NoParty,
            Entry::NotLeader,
            Entry::NotEligible,
            Entry::Busy,
        ] {
            let mut keeper = NpcScript::htpq_gatekeeper(entry);
            keeper.begin();
            let step = keeper.respond(ScriptInput::Select(0)).unwrap();
            assert_eq!((kind(&step), step.effects), (Some(0), vec![]));
        }

        // Inside, on the first map, only the leader with all five Crystal
        // Keys is heard: the keys go, and the team goes on to the lamp.
        let first = |items: &[(i32, i16)]| {
            let mut player = hunter(items, &[]);
            player.stats.map_id = events::HTPQ_ENTRY;
            player
        };
        let five: Vec<_> = events::CRYSTAL_KEYS.iter().map(|&key| (key, 1)).collect();
        let step = |player: &Player, leader| {
            NpcScript::in_event(HTPQ_KEEPER, player, leader, false)
                .unwrap()
                .begin()
        };
        let mut taken: Vec<_> = events::CRYSTAL_KEYS
            .iter()
            .map(|&item_id| Effect::TakeItem {
                item_id,
                quantity: 1,
            })
            .collect();
        taken.push(Effect::EventCommand(EventCommand::MoveTeam(
            events::HTPQ_LAMP,
        )));
        let handed = step(&first(&five), true);
        assert!(handed.prompt.is_none());
        assert_eq!(handed.effects, taken);
        assert_eq!(
            page_text(&step(&first(&five), false)),
            text(keys::HTPQ_KEEPER_LEADER_ONLY)
        );
        assert_eq!(
            page_text(&step(&first(&five[1..]), true)),
            text(keys::HTPQ_KEEPER_KEYS_SHORT)
        );

        // On either last map, the Blue Key and six Red Keys win it.
        let last = |map, reds| {
            let mut player = hunter(
                &[
                    (events::BLUE_CRYSTAL_KEY, 1),
                    (events::RED_CRYSTAL_KEY, reds),
                ],
                &[],
            );
            player.stats.map_id = map;
            player
        };
        for map in [events::HTPQ_LIGHT, events::HTPQ_DARK] {
            assert_eq!(
                step(&last(map, 6), true).effects,
                [
                    Effect::TakeItem {
                        item_id: events::BLUE_CRYSTAL_KEY,
                        quantity: 1
                    },
                    Effect::TakeItem {
                        item_id: events::RED_CRYSTAL_KEY,
                        quantity: 6
                    },
                    Effect::EventCommand(EventCommand::ClearStage(6)),
                ]
            );
            assert_eq!(
                page_text(&step(&last(map, 5), true)),
                text(keys::HTPQ_KEEPER_LAST_KEYS_SHORT)
            );
        }
        // Nothing to say elsewhere, or once won.
        assert!(
            NpcScript::in_event(HTPQ_KEEPER, &last(events::HTPQ_LAMP, 6), true, false).is_none()
        );
        assert!(NpcScript::in_event(HTPQ_KEEPER, &first(&five), true, true).is_none());
    }

    #[test]
    fn one_line_npcs_say_their_line_and_finish() {
        for &(npc, key) in ONE_LINERS {
            let mut script = NpcScript::for_npc(npc, &player(10)).unwrap();
            let page = script.begin();
            assert_eq!(kind(&page), Some(0), "{npc}");
            assert_eq!(page_text(&page), text(key), "{npc}");
            let after = script.respond(ScriptInput::Next).unwrap();
            assert!(after.prompt.is_none() && after.effects.is_empty(), "{npc}");
        }
    }

    #[test]
    fn the_tombstone_talks_and_the_machine_takes_the_party_out_of_papulatus() {
        let player = player(50);
        let tombstone = NpcScript::for_npc(TOMBSTONE, &player).unwrap();
        assert_eq!(kind(&tombstone.begin()), Some(0));

        let mut machine = NpcScript::for_npc(MACHINE_APPARATUS, &player).unwrap();
        assert_eq!(kind(&machine.begin()), Some(1));
        assert_eq!(
            machine.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::Warp(events::CLOCKTOWER_DEPTHS)]
        );
    }

    #[test]
    fn keroben_and_the_slate_let_the_dragon_squad_on_toward_horntail() {
        let squad = |level, meso| {
            let mut player = hunter(&[(DRAGON_SQUAD_CERTIFICATE, 1)], &[]);
            (player.stats.level, player.meso) = (level, meso);
            player
        };
        let said =
            |npc, player: &Player| page_text(&NpcScript::for_npc(npc, player).unwrap().begin());

        // Keroben turns away all but dragons and certificate holders.
        assert_eq!(
            said(KEROBEN, &hunter(&[], &[])),
            text(keys::HORNTAIL_KEROBEN_AWAY)
        );
        let mut dragon = hunter(&[], &[]);
        dragon.stats.level = 100;
        let now = std::time::Instant::now();
        let lasts = std::time::Duration::from_secs(60);
        let morph =
            game::buff::Buff::new(-DRAGON_ELIXIR, game::buff::BuffStat::Morph, 1, now, lasts);
        dragon.buffs.give(morph);
        let menu = NpcScript::for_npc(KEROBEN, &dragon).unwrap().begin();
        assert_eq!(kind(&menu), Some(4));

        // He sells ten Power Elixirs, and lets those of level 100 in.
        let mut keroben = NpcScript::for_npc(KEROBEN, &squad(100, 100_000)).unwrap();
        keroben.begin();
        assert_eq!(
            keroben.respond(ScriptInput::Select(0)).unwrap().effects,
            [
                Effect::SpendMeso(100_000),
                Effect::GiveItem {
                    item_id: POWER_ELIXIR,
                    quantity: 10
                }
            ]
        );
        let mut keroben = NpcScript::for_npc(KEROBEN, &squad(99, 99_999)).unwrap();
        keroben.begin();
        let short = keroben.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(page_text(&short), text(keys::HORNTAIL_KEROBEN_SHORT));
        let mut keroben = NpcScript::for_npc(KEROBEN, &squad(99, 0)).unwrap();
        keroben.begin();
        let low = keroben.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(page_text(&low), text(keys::HORNTAIL_KEROBEN_LOW_LEVEL));
        let mut keroben = NpcScript::for_npc(KEROBEN, &squad(100, 0)).unwrap();
        keroben.begin();
        assert_eq!(
            keroben.respond(ScriptInput::Select(1)).unwrap().effects,
            [Effect::WarpToPortal(CAVE_ENTRANCE, 0)]
        );

        // The slate opens to the certificate alone.
        assert_eq!(
            said(SQUAD_SLATE, &dragon),
            text(keys::HORNTAIL_SLATE_SEALED)
        );
        let mut slate = NpcScript::for_npc(SQUAD_SLATE, &squad(100, 0)).unwrap();
        slate.begin();
        assert_eq!(
            slate.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::WarpToPortal(HORNTAIL_CAVE_ENTRANCE, 0)]
        );

        // The Crystal of Roots takes those past the entrance to the Road in
        // Between, and others out of the cave.
        let leave = |map_id| {
            let mut player = squad(100, 0);
            player.stats.map_id = map_id;
            let mut crystal = NpcScript::for_npc(CRYSTAL_OF_ROOTS, &player).unwrap();
            crystal.begin();
            crystal.respond(ScriptInput::Yes).unwrap().effects
        };
        assert_eq!(leave(240060200), [Effect::WarpToPortal(ROAD_IN_BETWEEN, 0)]);
        let out00 = crate::scripts::named(CAVE_OF_LIFE_ENTRANCE, "out00");
        assert_eq!(
            leave(HORNTAIL_CAVE_ENTRANCE),
            [Effect::WarpToPortal(CAVE_OF_LIFE_ENTRANCE, out00)]
        );
    }

    #[test]
    fn the_mark_of_the_squad_gathers_horntails_expeditions() {
        let at = |level| {
            let mut player = hunter(&[], &[]);
            player.stats.level = level;
            player
        };
        let gather = |kind, level| NpcScript::squad(kind, SquadView::None, &at(level)).unwrap();
        let low = gather(EventKind::Horntail, 99);
        assert_eq!(low.npc_id, HORNTAIL_RECRUITER);
        let level = event_terms(&events::HORNTAIL, keys::HORNTAIL_SQUAD_LEVEL);
        assert_eq!(page_text(&low.begin()), level);
        // Any of level 100 may lead one; Zakum's wants an Eye of Fire.
        let mut open = gather(EventKind::Horntail, 100);
        let ask = event_terms(&events::HORNTAIL, keys::HORNTAIL_SQUAD_ASK);
        assert_eq!(question(&open.begin()), ask);
        assert_eq!(
            open.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::Squad {
                kind: EventKind::Horntail,
                command: SquadCommand::Open
            }]
        );
        assert_eq!(
            page_text(&gather(EventKind::Zakum, 100).begin()),
            text(keys::ZAKUM_SQUAD_NO_EYE)
        );
        let taken = NpcScript::squad(EventKind::Horntail, SquadView::Taken, &at(100));
        assert_eq!(
            page_text(&taken.unwrap().begin()),
            text(keys::HORNTAIL_SQUAD_TAKEN)
        );
        assert!(NpcScript::squad(EventKind::Kpq, SquadView::None, &at(100)).is_none());
        for kind in EventKind::ALL {
            assert_eq!(kind.event().squad, recruiter(kind).is_some(), "{kind:?}");
        }
    }

    #[test]
    fn a_chief_approves_zakum_and_adobis_holds_its_trials() {
        const RENE: i32 = 2020010;
        // The chief's second choice approves, once.
        let mut rene = NpcScript::for_npc(RENE, &hunter(&[], &[])).unwrap();
        assert_eq!(
            rene.respond(ScriptInput::Select(1)).unwrap().effects,
            [Effect::Mark(ZAKUM_APPROVED)]
        );
        // Holding the Necklace of Strength, the Zakum choice keeps it.
        let mut rene =
            NpcScript::for_npc(RENE, &hunter(&[(NECKLACE_OF_STRENGTH, 1)], &[])).unwrap();
        assert_eq!(
            rene.respond(ScriptInput::Select(1)).unwrap().effects,
            [Effect::Mark(ZAKUM_APPROVED)]
        );
        let mut rene = NpcScript::for_npc(RENE, &hunter(&[], &[ZAKUM_APPROVED])).unwrap();
        let again = rene.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(page_text(&again), text(keys::ZAKUM_CHIEF_APPROVED));
        assert!(again.effects.is_empty());
        // A third job of the line goes straight to it; under 50, nothing.
        let mut ranger = hunter(&[], &[]);
        ranger.stats.job = 311;
        let rene = NpcScript::for_npc(RENE, &ranger).unwrap();
        assert_eq!(rene.begin().effects, [Effect::Mark(ZAKUM_APPROVED)]);
        ranger.stats.level = 49;
        let rene = NpcScript::for_npc(RENE, &ranger).unwrap();
        assert!(rene.begin().effects.is_empty());

        // Adobis warns the unapproved.
        let adobis = |player: &Player| NpcScript::adobis(Entry::NoParty, player);
        let says = |player: &Player| page_text(&adobis(player).begin());
        assert_eq!(says(&hunter(&[], &[])), text(keys::ZAKUM_ADOBIS_UNAPPROVED));
        let mut young = hunter(&[], &[]);
        young.stats.level = 49;
        assert_eq!(says(&young), text(keys::ZAKUM_ADOBIS_WARNING));

        // The Breath of Lava takes the Dead Mine's Fire Ore.
        let choose = |player: &Player, choice| {
            let mut adobis = adobis(player);
            assert_eq!(kind(&adobis.begin()), Some(4));
            let step = adobis.respond(ScriptInput::Select(choice)).unwrap();
            (adobis, step)
        };
        let lava = |items: &[(i32, i16)]| choose(&hunter(items, &[ZAKUM_APPROVED]), 1);
        assert_eq!(
            page_text(&lava(&[]).1),
            text(keys::ZAKUM_ADOBIS_LAVA_EARLIER)
        );
        assert_eq!(
            page_text(&lava(&[(FIRE_ORE, 1), (BREATH_OF_FIRE, 1)]).1),
            text(keys::ZAKUM_ADOBIS_LAVA_HAVE)
        );
        let (mut adobis_lava, step) = lava(&[(FIRE_ORE, 1)]);
        assert_eq!(kind(&step), Some(1));
        assert_eq!(
            adobis_lava.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::WarpToPortal(BREATH_OF_LAVA, 0)]
        );

        // Lira gives the Breath of Fire and sends the player back.
        let mut lira = NpcScript::for_npc(LIRA, &hunter(&[], &[ZAKUM_APPROVED])).unwrap();
        lira.respond(ScriptInput::Next).unwrap();
        assert_eq!(
            lira.respond(ScriptInput::Next).unwrap().effects,
            [
                Effect::GiveItem {
                    item_id: BREATH_OF_FIRE,
                    quantity: 1
                },
                Effect::GainExp(10_000),
                Effect::Warp(DOOR_TO_ZAKUM),
            ]
        );

        // The forge wants both trials' items and 30 Gold Teeth.
        let forge = |items: &[(i32, i16)]| choose(&hunter(items, &[ZAKUM_APPROVED]), 2).1;
        let trials = [(FIRE_ORE, 1), (BREATH_OF_FIRE, 1), (GOLD_TOOTH, 29)];
        assert_eq!(
            page_text(&forge(&trials[1..])),
            text(keys::ZAKUM_ADOBIS_FORGE_LACK)
        );
        assert_eq!(
            page_text(&forge(&trials)),
            text(keys::ZAKUM_ADOBIS_FORGE_SHORT)
        );
        let take = |item_id, quantity| Effect::TakeItem { item_id, quantity };
        assert_eq!(
            forge(&[(FIRE_ORE, 1), (BREATH_OF_FIRE, 1), (GOLD_TOOTH, 30)]).effects,
            [
                take(FIRE_ORE, 1),
                take(BREATH_OF_FIRE, 1),
                take(GOLD_TOOTH, 30),
                Effect::GiveItem {
                    item_id: EYE_OF_FIRE,
                    quantity: 5
                },
                Effect::Mark(ZAKUM_TRIALS_DONE),
            ]
        );

        // Amon lets the player out.
        let mut out = hunter(&[], &[]);
        for (map, to) in [
            (BREATH_OF_LAVA, DOOR_TO_ZAKUM),
            (ZAKUM_ALTAR, ENTRANCE_TO_ZAKUM),
        ] {
            out.stats.map_id = map;
            let mut amon = NpcScript::for_npc(AMON, &out).unwrap();
            assert_eq!(kind(&amon.begin()), Some(1));
            assert_eq!(
                amon.respond(ScriptInput::Yes).unwrap().effects,
                [Effect::Warp(to)]
            );
        }

        // The gate wants the approval, the trials and an Eye of Fire.
        let gate = |items: &[(i32, i16)], markers: &[i16]| {
            crate::scripts::portal("Zakum05", &hunter(items, markers))
                .unwrap()
                .effects
        };
        let pink = |key| vec![Effect::Pink(text(key))];
        assert_eq!(gate(&[], &[]), pink(keys::ZAKUM_GATE_UNAPPROVED));
        assert_eq!(
            gate(&[(EYE_OF_FIRE, 1)], &[ZAKUM_APPROVED]),
            pink(keys::ZAKUM_GATE_TRIALS)
        );
        let done = [ZAKUM_APPROVED, ZAKUM_TRIALS_DONE];
        assert_eq!(gate(&[], &done), pink(keys::ZAKUM_GATE_NO_EYE));
        assert!(matches!(
            gate(&[(EYE_OF_FIRE, 1)], &done)[..],
            [
                Effect::ShutByReactor {
                    name: "gate",
                    state: 1
                },
                Effect::PortalSound,
                Effect::WarpToPortal(ENTRANCE_TO_ZAKUM, _)
            ]
        ));
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
        const ORE: i32 = 4004004;
        let player = hunter(&[(ORE, 30)], &[]);
        let spiruna = NpcScript::for_npc(SPIRUNA, &player).unwrap();
        assert_eq!(kind(&spiruna.begin()), Some(0));
        let mut player = player;
        player.quests.upsert(game::QuestRecord {
            quest_id: SPIRUNA_QUEST,
            status: QuestStatus::Completed {
                at: std::time::SystemTime::UNIX_EPOCH,
            },
        });
        let mut spiruna = NpcScript::for_npc(SPIRUNA, &player).unwrap();
        let step = spiruna.respond(ScriptInput::Yes).unwrap();
        assert_eq!(how_many_bounds(&step), (1, 1, 100));
        assert_eq!(
            spiruna.respond(ScriptInput::Number(3)).unwrap().effects,
            [
                Effect::GiveItem {
                    item_id: DARK_CRYSTAL,
                    quantity: 3
                },
                Effect::SpendMeso(1_500_000),
                Effect::TakeItem {
                    item_id: ORE,
                    quantity: 30
                },
            ]
        );
        assert_eq!(
            page_text(&spiruna.refused().unwrap()),
            text(keys::SPIRUNA_SHORT)
        );
    }

    /// A quantity prompt's default, least and most.
    fn how_many_bounds(step: &Step) -> (i32, i32, i32) {
        match step.prompt {
            Some(Prompt::AskNumber {
                default, min, max, ..
            }) => (default, min, max),
            ref other => panic!("number expected, got {other:?}"),
        }
    }

    #[test]
    fn refiners_refine_as_many_batches_as_asked() {
        // Mr. Thunder: mineral ore, then Steel Plates, five batches.
        let mut thunder = NpcScript::for_npc(MR_THUNDER, &player(30)).unwrap();
        assert!(menu_text(&thunder.begin()).contains("#L3#"));
        thunder.respond(ScriptInput::Select(0)).unwrap();
        let step = thunder.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(how_many_bounds(&step), (1, 1, 100));
        let ask = question(&thunder.respond(ScriptInput::Number(5)).unwrap());
        assert!(ask.contains("#i4010001# 50 #t4010001#"), "{ask}");
        assert!(ask.contains("1500"), "{ask}");
        assert_eq!(
            thunder.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::GiveItem {
                    item_id: 4011001,
                    quantity: 5
                },
                Effect::SpendMeso(1500),
                Effect::TakeItem {
                    item_id: 4010001,
                    quantity: 50
                },
            ]
        );

        // Pi charges 90%, and makes Screws fifteen a batch for nothing.
        let mut pi = NpcScript::for_npc(PI, &player(30)).unwrap();
        pi.respond(ScriptInput::Select(0)).unwrap();
        pi.respond(ScriptInput::Select(6)).unwrap();
        pi.respond(ScriptInput::Number(2)).unwrap();
        assert_eq!(
            pi.respond(ScriptInput::Yes).unwrap().effects[1],
            Effect::SpendMeso(1440)
        );
        let mut pi = NpcScript::for_npc(PI, &player(30)).unwrap();
        pi.respond(ScriptInput::Select(4)).unwrap();
        let menu = menu_text(&pi.begin()).to_owned();
        assert!(menu.contains("#t4003001# (#t4000003#)"), "{menu}");
        assert!(menu.contains("#t4003001# (#t4000018#)"), "{menu}");
        assert!(menu.contains("#t4003000# x15"), "{menu}");
        pi.respond(ScriptInput::Select(2)).unwrap();
        let ask = question(&pi.respond(ScriptInput::Number(2)).unwrap());
        assert!(ask.contains(" 30 "), "{ask}");
        assert!(!ask.contains(&MESO_ICON.to_string()), "{ask}");
        assert_eq!(
            pi.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::GiveItem {
                    item_id: 4003000,
                    quantity: 30
                },
                Effect::TakeItem {
                    item_id: 4011000,
                    quantity: 2
                },
                Effect::TakeItem {
                    item_id: 4011001,
                    quantity: 2
                },
            ]
        );

        // Vogen's arrows come a bundle at a time, and his Dark Crystals
        // cost a million each.
        let mut vogen = NpcScript::for_npc(VOGEN, &player(30)).unwrap();
        vogen.respond(ScriptInput::Select(5)).unwrap();
        assert_eq!(
            kind(&vogen.respond(ScriptInput::Select(2)).unwrap()),
            Some(1)
        );
        assert_eq!(
            vogen.respond(ScriptInput::Yes).unwrap().effects[0],
            Effect::GiveItem {
                item_id: 2060001,
                quantity: 900
            }
        );
        let mut vogen = NpcScript::for_npc(VOGEN, &player(30)).unwrap();
        vogen.respond(ScriptInput::Select(3)).unwrap();
        vogen.respond(ScriptInput::Select(4)).unwrap();
        vogen.respond(ScriptInput::Number(100)).unwrap();
        assert_eq!(
            vogen.respond(ScriptInput::Yes).unwrap().effects[1],
            Effect::SpendMeso(100_000_000)
        );
        assert!(with_maker_words(NpcScript::new(VOGEN, Vec::new()))
            .refused
            .is_some());
    }

    #[test]
    fn muhamad_asks_first_and_refines_lidium() {
        let mut muhamad = NpcScript::for_npc(MUHAMAD, &player(30)).unwrap();
        assert_eq!(kind(&muhamad.begin()), Some(1));
        assert_eq!(
            page_text(&muhamad.respond(ScriptInput::No).unwrap()),
            text(keys::REFINER_MUHAMAD_BYE)
        );
        let mut muhamad = NpcScript::for_npc(MUHAMAD, &player(30)).unwrap();
        let menu = menu_text(&muhamad.respond(ScriptInput::Yes).unwrap()).to_owned();
        assert!(menu.contains("#L2#") && !menu.contains("#L3#"), "{menu}");
        muhamad.respond(ScriptInput::Select(0)).unwrap();
        muhamad.respond(ScriptInput::Select(7)).unwrap();
        muhamad.respond(ScriptInput::Number(1)).unwrap();
        assert_eq!(
            muhamad.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::GiveItem {
                    item_id: 4011008,
                    quantity: 1
                },
                Effect::SpendMeso(270),
                Effect::TakeItem {
                    item_id: 4010007,
                    quantity: 10
                },
            ]
        );
    }

    #[test]
    fn thunder_and_chris_upgrade_equipment_one_at_a_time() {
        let mut thunder = NpcScript::for_npc(MR_THUNDER, &player(30)).unwrap();
        thunder.respond(ScriptInput::Select(2)).unwrap();
        assert_eq!(
            kind(&thunder.respond(ScriptInput::Select(0)).unwrap()),
            Some(1)
        );
        assert_eq!(
            thunder.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::SpendMeso(500),
                Effect::TakeItem {
                    item_id: 1002001,
                    quantity: 1
                },
                Effect::TakeItem {
                    item_id: 4011002,
                    quantity: 1
                },
                Effect::GiveItem {
                    item_id: 1002042,
                    quantity: 1
                },
            ]
        );

        // Chris's Metal Hoofs: a word first, then how many.
        let mut chris = NpcScript::for_npc(CHRIS, &player(30)).unwrap();
        assert_eq!(
            kind(&chris.respond(ScriptInput::Select(2)).unwrap()),
            Some(1)
        );
        let step = chris.respond(ScriptInput::Yes).unwrap();
        assert_eq!(how_many_bounds(&step), (1, 1, 100));
        chris.respond(ScriptInput::Number(2)).unwrap();
        assert_eq!(
            chris.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::GiveItem {
                    item_id: 4011001,
                    quantity: 2
                },
                Effect::SpendMeso(2000),
                Effect::TakeItem {
                    item_id: 4000039,
                    quantity: 200
                },
            ]
        );
    }

    #[test]
    fn smith_vicious_and_jm_make_equipment_and_materials() {
        // Vicious: a War Bow from Processed Wood and Snail Shells.
        let mut vicious = NpcScript::for_npc(VICIOUS, &player(30)).unwrap();
        assert!(menu_text(&vicious.begin()).contains("#L5#"));
        vicious.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(
            kind(&vicious.respond(ScriptInput::Select(0)).unwrap()),
            Some(1)
        );
        assert_eq!(
            vicious.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::SpendMeso(800),
                Effect::TakeItem {
                    item_id: 4003001,
                    quantity: 5
                },
                Effect::TakeItem {
                    item_id: 4000000,
                    quantity: 30
                },
                Effect::GiveItem {
                    item_id: 1452002,
                    quantity: 1
                },
            ]
        );

        // Mr. Smith: Processed Wood from Firewood, in batches.
        let mut smith = NpcScript::for_npc(MR_SMITH, &player(30)).unwrap();
        smith.respond(ScriptInput::Select(2)).unwrap();
        smith.respond(ScriptInput::Select(1)).unwrap();
        smith.respond(ScriptInput::Number(4)).unwrap();
        assert_eq!(
            smith.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::GiveItem {
                    item_id: 4003001,
                    quantity: 4
                },
                Effect::TakeItem {
                    item_id: 4000018,
                    quantity: 20
                },
            ]
        );

        // JM: the Meba takes a Garnier.
        let mut jm = NpcScript::for_npc(JM, &player(30)).unwrap();
        jm.respond(ScriptInput::Select(2)).unwrap();
        jm.respond(ScriptInput::Select(2)).unwrap();
        let effects = jm.respond(ScriptInput::Yes).unwrap().effects;
        assert_eq!(
            effects[1],
            Effect::TakeItem {
                item_id: 1472000,
                quantity: 1
            }
        );
        assert_eq!(
            effects.last(),
            Some(&Effect::GiveItem {
                item_id: 1472007,
                quantity: 1
            })
        );
    }

    #[test]
    fn altaire_campers_trade_each_hundred_for_a_perfect_pitch() {
        const PERZEN: i32 = 2131001;
        const RUBBLE: i32 = 4000439;
        let mut perzen = NpcScript::for_npc(PERZEN, &hunter(&[(RUBBLE, 99)], &[])).unwrap();
        perzen.begin();
        assert_eq!(
            page_text(&perzen.respond(ScriptInput::Select(0)).unwrap()),
            content::text().get(keys::ALTAIRE_SHORT, &[("item", "4000439")])
        );
        let player = hunter(&[(RUBBLE, 200), (RUBBLE, 50)], &[]);
        let mut perzen = NpcScript::for_npc(PERZEN, &player).unwrap();
        let step = perzen.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(how_many_bounds(&step), (2, 1, 2));
        assert_eq!(
            perzen.respond(ScriptInput::Number(2)).unwrap().effects,
            [
                Effect::TakeItem {
                    item_id: RUBBLE,
                    quantity: 200
                },
                Effect::GiveItem {
                    item_id: refiners::PERFECT_PITCH,
                    quantity: 2
                },
            ]
        );
    }

    fn engaging(markers: &[i16], gender: u8, meso: i32, items: &[(i32, i16)]) -> Player {
        let mut player = traveller(30, meso, false);
        player.stats.gender = gender;
        for &quest_id in markers {
            let status = QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            };
            player.quests.upsert(game::QuestRecord { quest_id, status });
        }
        for (slot, &(item_id, count)) in items.iter().enumerate() {
            let tab = if item_id / 1_000_000 == 2 {
                game::InventoryType::Use
            } else {
                game::InventoryType::Etc
            };
            player
                .inventory
                .insert(tab, slot as i16 + 1, game::Item::stack(item_id, count));
        }
        player
    }

    #[test]
    fn moony_sends_a_groom_for_the_blessing_then_makes_his_ring_box() {
        let make = |player: &Player| {
            let mut moony = NpcScript::for_npc(MOONY, player).unwrap();
            assert_eq!(kind(&moony.begin()), Some(4));
            let step = moony.respond(ScriptInput::Select(0)).unwrap();
            (moony, step)
        };

        // First the blessing, then a word while it is sought.
        let (mut moony, step) = make(&engaging(&[], 0, 0, &[]));
        assert_eq!(page_text(&step), text(keys::MOONY_NEED_BLESSING));
        assert_eq!(
            moony.respond(ScriptInput::Next).unwrap().effects,
            [Effect::Mark(BLESSING_SOUGHT)]
        );
        let (_, step) = make(&engaging(&[BLESSING_SOUGHT], 0, 0, &[]));
        assert_eq!(page_text(&step), text(keys::MOONY_SEEKING));

        // Blessed, the groom picks a ring and pays its stones and fee.
        let stones = [(4021009, 1), (DIAMOND, 1)];
        let groom = engaging(&[BLESSED], 0, 20_000, &stones);
        let (mut moony, step) = make(&groom);
        assert_eq!(menu_text(&step).matches("#L").count(), 4);
        let ask = moony.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(kind(&ask), Some(1));
        let made = moony.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            made.effects,
            [
                Effect::SpendMeso(20_000),
                Effect::TakeItem {
                    item_id: 4021009,
                    quantity: 1
                },
                Effect::TakeItem {
                    item_id: DIAMOND,
                    quantity: 1
                },
                Effect::GiveItem {
                    item_id: 2240001,
                    quantity: 1
                },
            ]
        );
        assert_eq!(page_text(&made), text(keys::MOONY_DONE));

        // Short of the fee, or of a stone, nothing changes hands.
        let refused = |player: &Player, ring: i32| {
            let (mut moony, _) = make(player);
            moony.respond(ScriptInput::Select(ring)).unwrap();
            let step = moony.respond(ScriptInput::Yes).unwrap();
            assert_eq!(step.effects, []);
            page_text(&step)
        };
        let poor = engaging(&[BLESSED], 0, 19_999, &stones);
        assert_eq!(refused(&poor, 1), text(keys::MOONY_SHORT));
        assert_eq!(refused(&groom, 2), text(keys::MOONY_NO_STONES));
        let no_diamond = engaging(&[BLESSED], 0, 20_000, &stones[..1]);
        assert_eq!(refused(&no_diamond, 1), text(keys::MOONY_NO_STONES));

        // One box at a time, and only for a groom.
        let holding = engaging(&[BLESSED], 0, 20_000, &[(2240003, 1)]);
        assert_eq!(page_text(&make(&holding).1), text(keys::MOONY_HAVE_BOX));
        let bride = engaging(&[BLESSED], 1, 20_000, &stones);
        assert_eq!(page_text(&make(&bride).1), text(keys::MOONY_GROOM_ONLY));
    }

    #[test]
    fn moony_throws_away_every_ring_box_held() {
        let discard = |player: &Player| {
            let mut moony = NpcScript::for_npc(MOONY, player).unwrap();
            moony.begin();
            moony.respond(ScriptInput::Select(1)).unwrap()
        };
        let none = discard(&engaging(&[], 0, 0, &[]));
        assert_eq!(page_text(&none), text(keys::MOONY_NO_BOX));
        assert_eq!(none.effects, []);
        let boxes = discard(&engaging(&[], 0, 0, &[(2240000, 1), (2240002, 2)]));
        assert_eq!(page_text(&boxes), text(keys::MOONY_DISCARDED));
        assert_eq!(
            boxes.effects,
            [
                Effect::TakeItem {
                    item_id: 2240000,
                    quantity: 1
                },
                Effect::TakeItem {
                    item_id: 2240002,
                    quantity: 2
                },
            ]
        );
    }

    /// An engaged bride in Amoria holding `items`, by tab.
    fn bride(items: &[(InventoryType, i32)]) -> Player {
        let mut player = traveller(30, 0, false);
        player.stats.map_id = AMORIA.0;
        player.stats.gender = 1;
        player.marriage = Some(game::marriage::MarriageRecord {
            id: 5,
            groom_id: 2,
            bride_id: player.stats.id,
            status: marriage::ENGAGED,
            item_id: 2240000,
            groom_name: "Groom".into(),
            bride_name: "Bride".into(),
        });
        for (slot, &(tab, item_id)) in items.iter().enumerate() {
            let item = game::Item::stack(item_id, 1);
            player.inventory.insert(tab, slot as i16 + 1, item);
        }
        player
    }

    #[test]
    fn the_wedding_assistants_book_an_engaged_couple_for_a_ticket() {
        if crate::items::data().is_err() {
            return;
        }
        let now = SystemTime::UNIX_EPOCH;
        let ask = |npc_id, player: &Player, reservation: Option<&Reservation>, choice| {
            let view = WeddingView {
                reservation,
                guest_of: None,
                channel: 0,
                now,
            };
            let mut script = NpcScript::wedding_assistant(npc_id, player, view).unwrap();
            assert_eq!(kind(&script.begin()), Some(4));
            script.respond(ScriptInput::Select(choice)).unwrap()
        };
        let reserve = |npc_id, player: &Player| ask(npc_id, player, None, 1);

        // In their lobbies, other lines.
        let mut lobby = bride(&[]);
        lobby.stats.map_id = 680000100;
        let view = WeddingView {
            reservation: None,
            guest_of: None,
            channel: 0,
            now,
        };
        assert!(NpcScript::wedding_assistant(BONNIE, &lobby, view).is_none());

        let prepare = ask(NICOLE, &bride(&[]), None, 0);
        assert!(page_text(&prepare).contains("#t5251000#"));
        assert!(page_text(&prepare).contains("#t5251003#"));

        let mut single = bride(&[]);
        single.marriage = None;
        assert_eq!(
            page_text(&reserve(NICOLE, &single)),
            text(keys::WEDDING_NO_RING)
        );
        let mut married = bride(&[]);
        married.marriage.as_mut().unwrap().status = marriage::MARRIED;
        assert_eq!(
            page_text(&reserve(NICOLE, &married)),
            text(keys::WEDDING_NO_RING)
        );

        // Booked already, at either venue: when.
        let booked = Reservation {
            marriage_id: 5,
            venue: Venue::Chapel,
            premium: true,
            channel: 1,
            starts_at: now + std::time::Duration::from_secs(90 * 60),
        };
        let ticket = [(InventoryType::Cash, 5251000)];
        let when = page_text(&ask(NICOLE, &bride(&ticket), Some(&booked), 1));
        assert_eq!(when, wedding_text(keys::WEDDING_BOOKED, &booked, now));
        assert!(when.contains("#t5251002#") && when.contains("90") && when.contains('2'));

        let ring = bride(&[(InventoryType::Equip, 1112803), ticket[0]]);
        assert_eq!(
            page_text(&reserve(NICOLE, &ring)),
            text(keys::WEDDING_HAS_RING)
        );
        let mut worn = bride(&ticket);
        worn.inventory
            .insert(InventoryType::Equipped, -12, game::Item::stack(1112809, 1));
        assert_eq!(
            page_text(&reserve(NICOLE, &worn)),
            text(keys::WEDDING_HAS_RING)
        );

        let mut full = bride(&[(InventoryType::Etc, 4000000), ticket[0]]);
        full.inventory_mut().set_slot_limit(InventoryType::Etc, 1);
        assert_eq!(
            page_text(&reserve(NICOLE, &full)),
            text(keys::WEDDING_NO_ROOM)
        );
        // Invitations expire, so they never join a stack already held.
        let mut stack = bride(&[(InventoryType::Etc, 4031395), ticket[0]]);
        stack.inventory_mut().set_slot_limit(InventoryType::Etc, 1);
        assert_eq!(
            page_text(&reserve(NICOLE, &stack)),
            text(keys::WEDDING_NO_ROOM)
        );

        // A Chapel ticket books nothing at the Cathedral.
        let chapel = bride(&[(InventoryType::Cash, 5251001)]);
        let none = page_text(&reserve(NICOLE, &chapel));
        assert!(none.contains("#t5251000#") && none.contains("#t5251003#"));

        let book = reserve(NICOLE, &bride(&ticket));
        assert_eq!(kind(&book), None);
        let cathedral = Effect::BookWedding {
            venue: Venue::Cathedral,
            premium: false,
        };
        assert_eq!(book.effects, [cathedral]);
        // The premium ticket first.
        let both = bride(&[
            (InventoryType::Cash, 5251001),
            (InventoryType::Cash, 5251002),
        ]);
        let chapel = Effect::BookWedding {
            venue: Venue::Chapel,
            premium: true,
        };
        assert_eq!(reserve(BONNIE, &both).effects, [chapel]);
    }

    #[test]
    fn the_wedding_staff_see_guests_out_and_the_married_couple_off() {
        let mut leave = NpcScript::wedding_staff(BONNIE, WeddingStage::Gathering, true, true);
        assert_eq!(kind(&leave.begin()), Some(1));
        let yes = leave.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.effects, [Effect::LeaveWedding]);
        let wait = NpcScript::wedding_staff(JACKIE, WeddingStage::Ceremony, false, true);
        assert_eq!(page_text(&wait.begin()), text(keys::WEDDING_STAFF_WAIT));
        let mut end = NpcScript::wedding_staff(JACKIE, WeddingStage::Married, false, false);
        assert_eq!(question(&end.begin()), text(keys::WEDDING_STAFF_ASK));
        let yes = end.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.effects, [Effect::EndWedding]);
        let mut party = NpcScript::wedding_staff(NANCY, WeddingStage::Married, false, true);
        assert_eq!(
            question(&party.begin()),
            text(keys::WEDDING_STAFF_PARTY_ASK)
        );
        let yes = party.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.effects, [Effect::EndWedding]);
    }

    #[test]
    fn the_party_room_has_two_ways_out() {
        let party = |npc_id, map_id, keys| NpcScript::wedding_party(npc_id, map_id, keys);
        let mut quit = party(TRAVIS, wedding::PARTY_ROOM, 0).unwrap();
        assert_eq!(question(&quit.begin()), text(keys::WEDDING_PARTY_QUIT_ASK));
        let yes = quit.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.effects, [Effect::Warp(680000000)]);
        let mut out = party(WEDDING_ROBIN, wedding::PARTY_ROOM, 0).unwrap();
        out.begin();
        let ask = out.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(question(&ask), text(keys::WEDDING_PARTY_EXIT_ASK));
        let yes = out.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.effects, [Effect::Warp(wedding::WEDDING_EXIT)]);
        assert!(party(JACKIE, wedding::PARTY_ROOM, 0).is_none());
        assert!(party(TRAVIS, wedding::HUNTING_GROUND, 0).is_none());
        assert!(party(WEDDING_ROBIN, wedding::WEDDING_EXIT, 0).is_none());
    }

    #[test]
    fn robin_leads_to_the_hunting_ground_and_seven_keys_to_the_boxes() {
        let party = |map_id, keys| NpcScript::wedding_party(WEDDING_ROBIN, map_id, keys).unwrap();
        let mut hunt = party(wedding::PARTY_ROOM, 0);
        hunt.begin();
        let step = hunt.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(step.effects, [Effect::Warp(wedding::HUNTING_GROUND)]);
        assert!(step.prompt.is_none());

        let mut short = party(wedding::HUNTING_GROUND, 6);
        short.begin();
        let step = short.respond(ScriptInput::Select(0)).unwrap();
        assert!(step.effects.is_empty());
        assert_eq!(page_text(&step), text(keys::WEDDING_ROBIN_NO_KEYS));

        let mut boxes = party(wedding::HUNTING_GROUND, 7);
        boxes.begin();
        let step = boxes.respond(ScriptInput::Select(0)).unwrap();
        let take = Effect::TakeItem {
            item_id: wedding::HEART_KEY,
            quantity: 7,
        };
        assert_eq!(step.effects, [take, Effect::Warp(wedding::BOX_ROOM)]);
        // Keys gone by the answer: the take fails and Robin says why.
        let refused = boxes.refused().unwrap();
        assert_eq!(page_text(&refused), text(keys::WEDDING_ROBIN_NO_KEYS));

        let mut back = party(wedding::BOX_ROOM, 0);
        assert_eq!(question(&back.begin()), text(keys::WEDDING_ROBIN_BACK_ASK));
        let yes = back.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.effects, [Effect::Warp(wedding::HUNTING_GROUND)]);
    }

    #[test]
    fn the_greeters_take_the_couples_wishlists_once() {
        let chapel = Venue::Chapel.hall();
        let mut ask = NpcScript::wedding_greeter(TRAVIS, chapel, true, false).unwrap();
        assert_eq!(question(&ask.begin()), text(keys::WEDDING_WISHLIST_ASK));
        let yes = ask.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.effects, [Effect::OpenWishlist]);
        assert!(yes.prompt.is_none());
        let altar = Venue::Cathedral.altar();
        let sent = NpcScript::wedding_greeter(DEBBIE, altar, true, true).unwrap();
        assert_eq!(page_text(&sent.begin()), text(keys::WEDDING_WISHLIST_SENT));
        let guest = NpcScript::wedding_greeter(DEBBIE, altar, false, false).unwrap();
        assert_eq!(
            page_text(&guest.begin()),
            text(keys::WEDDING_WISHLIST_GUEST)
        );
        // Travis keeps to the party room's way back there, and each greeter
        // to their own venue.
        assert!(NpcScript::wedding_greeter(TRAVIS, wedding::PARTY_ROOM, true, false).is_none());
        assert!(NpcScript::wedding_greeter(TRAVIS, altar, true, false).is_none());
        assert!(NpcScript::wedding_greeter(PILA, chapel, true, false).is_none());
    }

    #[test]
    fn pila_opens_a_wishlist_in_a_wedding_and_the_gifts_outside() {
        for (choice, bride) in [(0, false), (1, true)] {
            let mut give = NpcScript::pila(true, true);
            give.begin();
            let step = give.respond(ScriptInput::Select(choice)).unwrap();
            assert_eq!(step.effects, [Effect::OpenRegistry { bride }]);
            assert!(step.prompt.is_none());
        }
        let mut take = NpcScript::pila(false, true);
        assert_eq!(question(&take.begin()), text(keys::WEDDING_PILA_GIFTS));
        let yes = take.respond(ScriptInput::Yes).unwrap();
        assert_eq!(yes.effects, [Effect::OpenGifts]);
        let hello = NpcScript::pila(false, false);
        assert_eq!(page_text(&hello.begin()), text(keys::WEDDING_PILA_HELLO));
    }

    #[test]
    fn the_wedding_assistants_answer_guests_and_make_more_invitations() {
        if crate::items::data().is_err() {
            return;
        }
        let now = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(600 * 60);
        let wedding = |venue, minutes: u64| Reservation {
            marriage_id: 5,
            venue,
            premium: false,
            channel: 0,
            starts_at: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(minutes * 60),
        };
        let choose = |npc_id, player: &Player, view: WeddingView, choice| {
            let mut script = NpcScript::wedding_assistant(npc_id, player, view).unwrap();
            script.begin();
            script.respond(ScriptInput::Select(choice)).unwrap()
        };
        let as_guest = |player: &Player, guest_of: Option<&Reservation>| {
            let view = WeddingView {
                reservation: None,
                guest_of,
                channel: 0,
                now,
            };
            page_text(&choose(NICOLE, player, view, 2))
        };

        // A guest needs both the invitation and a place on the list.
        let later = wedding(Venue::Cathedral, 630);
        let invitation = [(InventoryType::Etc, 4031407)];
        let not_invited = text(keys::WEDDING_GUEST_NOT_INVITED);
        assert_eq!(as_guest(&bride(&[]), Some(&later)), not_invited);
        assert_eq!(as_guest(&bride(&invitation), None), not_invited);
        let early = as_guest(&bride(&invitation), Some(&later));
        assert_eq!(early, wedding_text(keys::WEDDING_GUEST_EARLY, &later, now));
        // Once it is due on their channel, whether to go in, with a word
        // on the clothes for a guest without any.
        let due = wedding(Venue::Cathedral, 595);
        let enter = |player: &Player, guest_of: &Reservation| {
            let view = WeddingView {
                reservation: None,
                guest_of: Some(guest_of),
                channel: 0,
                now,
            };
            choose(NICOLE, player, view, 2)
        };
        let question = |step: &Step| match &step.prompt {
            Some(Prompt::YesNo { text, .. }) => text.clone(),
            other => panic!("question expected, got {other:?}"),
        };
        let ask = enter(&bride(&invitation), &due);
        assert_eq!(question(&ask), text(keys::WEDDING_GUEST_ASK_CLOTHES));
        let dressed = bride(&[invitation[0], (InventoryType::Equip, 1051150)]);
        let ask = enter(&dressed, &due);
        assert_eq!(question(&ask), text(keys::WEDDING_GUEST_ASK));
        let mut script = NpcScript::wedding_assistant(
            NICOLE,
            &dressed,
            WeddingView {
                reservation: None,
                guest_of: Some(&due),
                channel: 0,
                now,
            },
        )
        .unwrap();
        script.begin();
        script.respond(ScriptInput::Select(2)).unwrap();
        let yes = script.respond(ScriptInput::Yes).unwrap();
        let cathedral = Effect::EnterWedding {
            venue: Venue::Cathedral,
        };
        assert_eq!(yes.effects, [cathedral]);
        let elsewhere = Reservation { channel: 2, ..due };
        let channel = page_text(&enter(&bride(&invitation), &elsewhere));
        assert_eq!(
            channel,
            wedding_text(keys::WEDDING_GUEST_CHANNEL, &elsewhere, now)
        );

        // More invitations only with the ticket.
        fn own(reservation: Option<&Reservation>, now: SystemTime) -> WeddingView<'_> {
            WeddingView {
                reservation,
                guest_of: None,
                channel: 0,
                now,
            }
        }
        let plain = NpcScript::wedding_assistant(NICOLE, &bride(&[]), own(None, now)).unwrap();
        let menu = format!("{:?}", plain.begin());
        assert!(!menu.contains(&text(keys::WEDDING_CHOICE_EXTRA)));
        let ticket = [(InventoryType::Cash, 5251100)];
        let extra = |player: &Player, reservation| {
            page_text(&choose(NICOLE, player, own(reservation, now), 3))
        };
        assert_eq!(
            extra(&bride(&ticket), None),
            text(keys::WEDDING_EXTRA_NOT_BOOKED)
        );
        let chapel = wedding(Venue::Chapel, 630);
        assert_eq!(
            extra(&bride(&ticket), Some(&chapel)),
            text(keys::WEDDING_EXTRA_NOT_BOOKED)
        );
        assert_eq!(
            extra(&bride(&ticket), Some(&due)),
            text(keys::WEDDING_EXTRA_STARTED)
        );
        let mut full = bride(&[(InventoryType::Etc, 4000000), ticket[0]]);
        full.inventory_mut().set_slot_limit(InventoryType::Etc, 1);
        assert_eq!(extra(&full, Some(&later)), text(keys::WEDDING_NO_ROOM));
        let made = choose(NICOLE, &bride(&ticket), own(Some(&later), now), 3);
        let effect = Effect::ExtraInvitations {
            venue: Venue::Cathedral,
        };
        assert_eq!(made.effects, [effect]);
    }

    #[test]
    fn mom_and_dad_bless_for_four_different_proofs_of_love() {
        // Their own lines until Moony sends the player.
        assert!(NpcScript::for_npc(MOM_AND_DAD, &engaging(&[], 0, 0, &[])).is_none());

        let mut asking =
            NpcScript::for_npc(MOM_AND_DAD, &engaging(&[BLESSING_SOUGHT], 0, 0, &[])).unwrap();
        assert_eq!(page_text(&asking.begin()), text(keys::PARENTS_ASK_1));
        asking.respond(ScriptInput::Next).unwrap();
        let last = asking.respond(ScriptInput::Next).unwrap();
        assert_eq!(page_text(&last), text(keys::PARENTS_ASK_3));
        assert_eq!(
            asking.respond(ScriptInput::Next).unwrap().effects,
            [Effect::Mark(PROOFS_ASKED)]
        );

        // Two of one proof count once.
        let asked = [BLESSING_SOUGHT, PROOFS_ASKED];
        let three = [(4031367, 2), (4031368, 1), (4031370, 1)];
        let waiting = NpcScript::for_npc(MOM_AND_DAD, &engaging(&asked, 0, 0, &three)).unwrap();
        assert_eq!(page_text(&waiting.begin()), text(keys::PARENTS_REMIND));

        let four = [(4031367, 2), (4031368, 1), (4031370, 1), (4031372, 1)];
        let mut proud = NpcScript::for_npc(MOM_AND_DAD, &engaging(&asked, 0, 0, &four)).unwrap();
        assert_eq!(page_text(&proud.begin()), text(keys::PARENTS_PROUD));
        let blessed = proud.respond(ScriptInput::Next).unwrap();
        assert_eq!(page_text(&blessed), text(keys::PARENTS_BLESS));
        let effects = &blessed.effects;
        assert_eq!(
            effects[..4],
            [
                Effect::Unmark(BLESSING_SOUGHT),
                Effect::Unmark(PROOFS_ASKED),
                Effect::Mark(BLESSED),
                Effect::GainExp(BLESSING_EXP),
            ]
        );
        for i in 0..6 {
            assert!(effects.contains(&Effect::Unmark(NANA_ASKED + i)));
            assert!(effects.contains(&Effect::Unmark(NANA_DONE + i)));
        }
        let taken: Vec<_> = effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::TakeItem { item_id, quantity } => Some((*item_id, *quantity)),
                _ => None,
            })
            .collect();
        assert_eq!(taken, four);
    }

    #[test]
    fn each_nana_trades_her_proof_of_love_for_fifty_of_her_towns_drop() {
        let sought = |more: &[i16], map: i32, items: &[(i32, i16)]| {
            let mut player = engaging(&[&[BLESSING_SOUGHT][..], more].concat(), 0, 0, items);
            player.stats.map_id = map;
            player
        };
        for (i, &(town, drop, exp)) in NANA_TASKS.iter().enumerate() {
            let (npc, proof) = (NANAS[i], PROOF_OF_LOVE + i as i32);
            let (asked, done) = (NANA_ASKED + i as i16, NANA_DONE + i as i16);
            let say = |key: &str| content::text().get(key, &[("item", &drop.to_string())]);
            let trade = [
                Effect::TakeItem {
                    item_id: drop,
                    quantity: 50,
                },
                Effect::GiveItem {
                    item_id: proof,
                    quantity: 1,
                },
            ];

            // Her own lines before Moony's errand.
            assert!(NpcScript::for_npc(npc, &engaging(&[], 0, 0, &[])).is_none());

            // The favour: on yes, her drop named.
            let mut nana = NpcScript::for_npc(npc, &sought(&[], town, &[])).unwrap();
            assert_eq!(kind(&nana.begin()), Some(1));
            let task = nana.respond(ScriptInput::Yes).unwrap();
            assert_eq!(task.effects, [Effect::Mark(asked)]);
            assert_eq!(page_text(&task), say(keys::NANA_TASK));

            // Short, a reminder; with 50, the proof and the EXP.
            let short = sought(&[asked], town, &[(drop, 49)]);
            let nana = NpcScript::for_npc(npc, &short).unwrap();
            assert_eq!(page_text(&nana.begin()), say(keys::NANA_NEED));
            let ready = sought(&[asked], town, &[(drop, 50)]);
            let thanks = NpcScript::for_npc(npc, &ready).unwrap().begin();
            assert_eq!(page_text(&thanks), say(keys::NANA_THANKS));
            let mut paid = trade.to_vec();
            paid.extend([
                Effect::GainExp(exp),
                Effect::Unmark(asked),
                Effect::Mark(done),
            ]);
            assert_eq!(thanks.effects, paid);

            // Holding her proof, nothing more; done and without it, another
            // for the drops alone.
            let holding = sought(&[done], town, &[(proof, 1), (drop, 50)]);
            let nana = NpcScript::for_npc(npc, &holding).unwrap();
            assert_eq!(page_text(&nana.begin()), say(keys::NANA_HAVE_PROOF));
            let again = sought(&[done], town, &[(drop, 50)]);
            let mut nana = NpcScript::for_npc(npc, &again).unwrap();
            assert_eq!(kind(&nana.begin()), Some(1));
            assert_eq!(nana.respond(ScriptInput::Yes).unwrap().effects, trade);
            let poor = sought(&[done], town, &[]);
            let mut nana = NpcScript::for_npc(npc, &poor).unwrap();
            nana.begin();
            let refused = nana.respond(ScriptInput::Yes).unwrap();
            assert_eq!(refused.effects, []);
            assert_eq!(page_text(&refused), say(keys::NANA_NEED));
        }

        // A Nana in a town without a task keeps her own lines.
        let elsewhere = sought(&[], 260000000, &[]);
        assert!(NpcScript::for_npc(NANAS[1], &elsewhere).is_none());
    }

    #[test]
    fn info_quests_move_on_one_character_per_npc() {
        let with_info = |quest_id, info: &str| {
            let mut player = traveller(30, 0, false);
            player.quests.upsert(game::QuestRecord {
                quest_id,
                status: QuestStatus::Started {
                    kills: Vec::new(),
                    info: Some(info.into()),
                },
            });
            player
        };
        // House 4 (slot 1) takes its delivery without a word.
        let mut sejan = with_info(3929, "3222");
        assert!(NpcScript::for_npc(2103005, &sejan).is_none());
        sejan
            .inventory
            .insert(game::InventoryType::Etc, 1, game::Item::stack(4031580, 4));
        let house = NpcScript::for_npc(2103005, &sejan).unwrap();
        let step = house.begin();
        assert_eq!(step.prompt, None);
        assert_eq!(
            step.effects,
            [
                Effect::SetQuestChar {
                    quest: 3929,
                    slot: 1,
                    from: b'2',
                    to: b'3'
                },
                Effect::TakeItem {
                    item_id: 4031580,
                    quantity: 1
                }
            ]
        );
        // House 1 has had its delivery; quest 3926's cupboards aren't due.
        assert!(NpcScript::for_npc(2103003, &sejan).is_none());
        assert!(NpcScript::for_npc(2103009, &sejan).is_none());

        // The oasis refreshes once; Magatia's wall asks first.
        let oasis = NpcScript::for_npc(PALACE_OASIS, &with_info(3900, "4")).unwrap();
        assert!(matches!(
            &oasis.begin().effects[..],
            [Effect::SetQuestChar { quest: 3900, .. }]
        ));
        assert!(NpcScript::for_npc(PALACE_OASIS, &with_info(3900, "5")).is_none());
        let mut wall = NpcScript::for_npc(MAGATIA_WALL, &with_info(3311, "4")).unwrap();
        assert!(matches!(wall.begin().prompt, Some(Prompt::YesNo { .. })));
        let read = wall.respond(ScriptInput::Yes).unwrap();
        assert_eq!(read.effects.len(), 1);
        assert!(NpcScript::for_npc(SECRET_WALL, &traveller(30, 0, false)).is_none());
    }

    #[test]
    fn each_meteorite_gives_one_sample_for_its_own_character() {
        let field = |info: &str, samples: i16| {
            let mut player = traveller(44, 0, false);
            player.quests.upsert(game::QuestRecord {
                quest_id: METEORITE_SAMPLES,
                status: QuestStatus::Started {
                    kills: Vec::new(),
                    info: Some(info.into()),
                },
            });
            if samples > 0 {
                player.inventory.insert(
                    game::InventoryType::Etc,
                    1,
                    game::Item::stack(METEORITE_SAMPLE, samples),
                );
            }
            player
        };
        let give = Effect::GiveItem {
            item_id: METEORITE_SAMPLE,
            quantity: 1,
        };
        // The fourth meteorite is the fourth character.
        let script = NpcScript::for_npc(2050017, &field("110000", 2)).unwrap();
        assert_eq!(script.inventory_full, Some(text(keys::METEORITE_FULL)));
        let step = script.begin();
        assert_eq!(step.prompt, None);
        assert_eq!(
            step.effects,
            [
                Effect::SetQuestChar {
                    quest: METEORITE_SAMPLES,
                    slot: 3,
                    from: b'0',
                    to: b'1'
                },
                give.clone()
            ]
        );
        // One already had says nothing, nor any without the quest.
        assert!(NpcScript::for_npc(2050015, &field("110000", 2)).is_none());
        assert!(NpcScript::for_npc(2050014, &traveller(44, 0, false)).is_none());
        // With all six had, a lost sample is given again, up to six.
        let again = NpcScript::for_npc(2050019, &field("111111", 5)).unwrap();
        assert_eq!(again.begin().effects, [give]);
        assert!(NpcScript::for_npc(2050019, &field("111111", 6)).is_none());
    }

    /// A player with info quest `quest` under way, its string `info`.
    fn on_info_quest(quest: i16, info: &str) -> Player {
        let mut player = traveller(85, 0, false);
        player.quests.upsert(game::QuestRecord {
            quest_id: quest,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: Some(info.into()),
            },
        });
        player
    }

    #[test]
    fn elizas_harp_counts_the_tune_and_starts_over_on_a_wrong_note() {
        let play = |npc: i32, info: &str| {
            let player = on_info_quest(ELIZAS_ETERNAL_SLEEP, info);
            NpcScript::for_npc(npc, &player).unwrap().begin().effects
        };
        let set = |value: &str| Effect::SetQuestInfo {
            quest: ELIZAS_ETERNAL_SLEEP,
            value: value.into(),
        };
        // The third note is a G, the fifth string.
        assert_eq!(
            play(2012031, "2"),
            [Effect::SoundToMap("orbis/sol".into()), set("3")]
        );
        assert_eq!(
            play(2012027, "2"),
            [
                Effect::SoundToMap("orbis/do".into()),
                set("0"),
                Effect::Pink(text(keys::QUEST_INFO_HARP_MISSED)),
                Effect::FieldEffect("quest/party/wrong_kor".into()),
                Effect::Sound("Party1/Failed".into()),
            ]
        );
        // The last note, a C, ends it.
        let last = play(2012027, "41");
        assert_eq!(last[1], set("42"));
        assert_eq!(last[4], Effect::Sound("Party1/Clear".into()));
        // Played through, or off the quest, a string only sounds.
        for player in [
            on_info_quest(ELIZAS_ETERNAL_SLEEP, "42"),
            traveller(85, 0, false),
        ] {
            let step = NpcScript::for_npc(2012033, &player).unwrap().begin();
            assert_eq!(step.effects, [Effect::SoundToMap("orbis/si".into())]);
        }
        // The tune's 42 notes, all on the harp.
        assert!(TWINKLE_TWINKLE.iter().all(|note| HARP_NOTES.contains(note)));
    }

    #[test]
    fn the_pentagram_circles_count_in_turn_and_the_last_takes_the_stones() {
        let set = |value: &str| Effect::SetQuestInfo {
            quest: MAGIC_PENTAGRAM,
            value: value.into(),
        };
        let touch = |npc: i32, player: &Player| NpcScript::for_npc(npc, player).unwrap().begin();
        assert_eq!(
            touch(2111021, &on_info_quest(MAGIC_PENTAGRAM, "1")).effects,
            [set("2")]
        );
        // Out of turn: over again.
        assert_eq!(
            touch(2111020, &on_info_quest(MAGIC_PENTAGRAM, "2")).effects,
            [set("0")]
        );
        // The last without the stones starts over; with them it takes them.
        let mut player = on_info_quest(MAGIC_PENTAGRAM, "3");
        assert_eq!(touch(2111023, &player).effects, [set("0")]);
        for (slot, stone) in (1..).zip(MAGIC_STONES) {
            player
                .inventory
                .insert(game::InventoryType::Etc, slot, game::Item::stack(stone, 1));
        }
        let step = touch(2111023, &player);
        assert_eq!(page_text(&step), text(keys::QUEST_INFO_PENTAGRAM));
        assert_eq!(step.effects.len(), 4);
        assert_eq!(step.effects[0], set("4"));
        // Done, or off the quest, they say nothing of their own.
        assert!(NpcScript::for_npc(2111020, &on_info_quest(MAGIC_PENTAGRAM, "4")).is_none());
        assert!(NpcScript::for_npc(2111020, &traveller(85, 0, false)).is_none());
    }

    #[test]
    fn each_shaman_rock_takes_one_charm_for_its_own_character() {
        let swamp = |info: &str, charms: i16| {
            let mut player = traveller(50, 0, false);
            player.stats.map_id = 105090000;
            player.quests.upsert(game::QuestRecord {
                quest_id: SHOO_AWAY_THE_EVIL,
                status: QuestStatus::Started {
                    kills: Vec::new(),
                    info: Some(info.into()),
                },
            });
            if charms > 0 {
                player.inventory.insert(
                    game::InventoryType::Etc,
                    1,
                    game::Item::stack(SHAMAN_CHARM, charms),
                );
            }
            player
        };
        let player = swamp("111000", 3);
        // The Swamp's second rock is the fifth character.
        let rock = shaman_rock(1063013, 1, &player).unwrap();
        let step = rock.begin();
        assert_eq!(page_text(&step), text(keys::QUEST_INFO_SHAMAN_ROCK));
        assert_eq!(
            step.effects,
            [
                Effect::SetQuestChar {
                    quest: SHOO_AWAY_THE_EVIL,
                    slot: 4,
                    from: b'0',
                    to: b'1'
                },
                Effect::TakeItem {
                    item_id: SHAMAN_CHARM,
                    quantity: 1
                }
            ]
        );
        assert!(matches!(
            &shaman_rock(1063013, 0, &player).unwrap().begin().effects[0],
            Effect::SetQuestChar { slot: 3, .. }
        ));
        // A rock already charmed, one without a charm held, or the other
        // rock's id here say nothing of their own.
        assert!(shaman_rock(1063013, 0, &swamp("111100", 2)).is_none());
        assert!(shaman_rock(1063013, 1, &swamp("111000", 0)).is_none());
        assert!(shaman_rock(1063012, 0, &player).is_none());
        let mut elsewhere = swamp("000000", 6);
        elsewhere.stats.map_id = 105050200;
        assert!(matches!(
            &shaman_rock(1063012, 0, &elsewhere).unwrap().begin().effects[0],
            Effect::SetQuestChar { slot: 0, .. }
        ));
    }

    #[test]
    fn the_shiny_stone_marks_shining_stone_once() {
        let on = |info: &str| {
            let mut player = traveller(30, 0, false);
            player.quests.upsert(game::QuestRecord {
                quest_id: SHINING_STONE,
                status: QuestStatus::Started {
                    kills: Vec::new(),
                    info: Some(info.into()),
                },
            });
            player
        };
        let step = NpcScript::for_npc(SHINY_STONE, &on("")).unwrap().begin();
        assert_eq!(page_text(&step), text(keys::QUEST_INFO_SHINY_STONE));
        assert_eq!(
            step.effects,
            [Effect::SetQuestInfo {
                quest: SHINING_STONE,
                value: "5".into()
            }]
        );
        // Once marked, or off the quest, it says nothing of its own.
        assert!(NpcScript::for_npc(SHINY_STONE, &on("5")).is_none());
        assert!(NpcScript::for_npc(SHINY_STONE, &traveller(30, 0, false)).is_none());
    }

    #[test]
    fn item_makers_trade_materials_and_mesos_for_their_items() {
        use rand::SeedableRng;
        let menu = |step: Step| match step.prompt {
            Some(Prompt::Selection { text, .. }) => text,
            other => panic!("menu expected, got {other:?}"),
        };
        let ask = |step: Step| match step.prompt {
            Some(Prompt::YesNo { text, .. }) => text,
            other => panic!("question expected, got {other:?}"),
        };
        let take = |item_id, quantity| Effect::TakeItem { item_id, quantity };
        // Serryl's first knuckler: 20 Leather and 1,000 mesos.
        let player = traveller(30, 0, false);
        let mut serryl = NpcScript::for_npc(SERRYL, &player).unwrap();
        assert_eq!(menu(serryl.begin()).matches("#L").count(), 3);
        assert!(menu(serryl.respond(ScriptInput::Select(0)).unwrap()).contains("#t1482007#"));
        let asked = ask(serryl.respond(ScriptInput::Select(0)).unwrap());
        assert!(asked.contains("#t1482001#") && asked.contains("#i4000021# 20 #t4000021#"));
        assert!(asked.contains("#i4031138# ") && asked.contains("1000"));
        let made = serryl.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            made.effects,
            [
                Effect::SpendMeso(1000),
                take(4000021, 20),
                Effect::GiveItem {
                    item_id: 1482001,
                    quantity: 1
                }
            ]
        );
        assert_eq!(kind(&made), Some(0));
        assert!(serryl.refused().is_some());
        // Neve upgrades a Bronze Husk to a Mithril Husk.
        let mut neve = NpcScript::for_npc(NEVE, &player).unwrap();
        assert_eq!(menu(neve.begin()).matches("#L").count(), 4);
        neve.respond(ScriptInput::Select(0));
        neve.respond(ScriptInput::Select(1));
        let made = neve.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            made.effects[..2],
            [Effect::SpendMeso(90000), take(1082103, 1)]
        );
        assert!(neve.inventory_full().is_some());

        // Mos: what a stimulator is, five classes, the same with one, and
        // the dagger only on the way to Moira.
        let mos = NpcScript::for_npc(MOS, &player).unwrap();
        assert_eq!(menu(mos.begin()).matches("#L").count(), 11);
        let rescuer = with_quests(&[(7301, false)]);
        let mut mos = NpcScript::for_npc(MOS, &rescuer).unwrap();
        assert!(menu(mos.begin()).contains("#L11##b#t4001078#"));
        assert_eq!(
            kind(&mos.respond(ScriptInput::Select(11)).unwrap()),
            Some(0)
        );
        assert!(ask(mos.respond(ScriptInput::Next).unwrap()).contains("#i4001079# 1 "));
        // A stimulated Dragon Carabella takes a one-handed sword stimulator
        // and is lost, or made with its stats varied.
        let stimulated = |mut rng: &mut dyn rand::RngCore| {
            let mut mos = super::mos(&player, &mut rng);
            mos.begin();
            mos.respond(ScriptInput::Select(6));
            assert!(ask(mos.respond(ScriptInput::Select(0)).unwrap()).contains("#i4130002# 1 "));
            mos.respond(ScriptInput::Yes).unwrap()
        };
        let lost = stimulated(&mut rand::rngs::mock::StepRng::new(0, 0));
        assert!(lost.effects.contains(&take(4130002, 1)));
        assert!(!lost
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::GiveItem { .. } | Effect::GiveVariedItem(_))));
        assert_eq!(page_text(&lost), text(keys::MAKER_MOS_LOST));
        let made = (0..100)
            .map(|seed| stimulated(&mut rand::rngs::StdRng::seed_from_u64(seed)))
            .find(|step| step.effects.contains(&Effect::GiveVariedItem(1302059)))
            .unwrap();
        assert_eq!(page_text(&made), text(keys::MAKER_DONE));
    }

    #[test]
    fn tangyoons_cows_fill_a_bottle_a_third_at_a_time() {
        let give = |item_id| Effect::GiveItem {
            item_id,
            quantity: 1,
        };
        let take = |item_id| Effect::TakeItem {
            item_id,
            quantity: 1,
        };
        // Tangyoon gives an empty bottle and sends the player to the
        // stable, only while Find Fresh Milk is under way.
        let mut player = with_quests(&[(FRESH_MILK, false)]);
        let mut tangyoon = NpcScript::for_npc(TANGYOON, &player).unwrap();
        assert_eq!(page_text(&tangyoon.begin()), text(keys::MILK_TANGYOON[0]));
        tangyoon.respond(ScriptInput::Next).unwrap();
        let sent = tangyoon.respond(ScriptInput::Next).unwrap();
        assert_eq!(sent.effects, [give(MILK_BOTTLES[0]), Effect::Warp(STABLE)]);
        let idle = NpcScript::for_npc(TANGYOON, &traveller(70, 0, false));
        assert!(idle.is_none_or(
            |other| !other.begin().effects.contains(&give(MILK_BOTTLES[0]))
                && other.begin().prompt.is_none_or(|prompt| !matches!(
                    prompt,
                    Prompt::Text(page) if page.text == text(keys::MILK_TANGYOON[0])
                ))
        ));

        // With no bottle, the cows and calves have nothing to say about it.
        let (first, second) = (MOTHER_COWS[0], MOTHER_COWS[1]);
        let said = |npc, player: &Player| {
            let script = NpcScript::for_npc(npc, player).unwrap();
            let step = script.begin();
            (page_text(&step), step.effects)
        };
        assert_eq!(said(first.0, &player).0, text(keys::MILK_COW_NO_BOTTLE));
        assert_eq!(said(CALVES[0], &player).0, text(keys::MILK_CALF_NO_BOTTLE));

        // A cow fills the empty bottle by a third and is remembered.
        player
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(MILK_BOTTLES[0], 1));
        assert_eq!(said(CALVES[1], &player).0, text(keys::MILK_CALF_EMPTY));
        let (page, effects) = said(first.0, &player);
        assert_eq!(page, text(keys::MILK_COW_FILLS[0]));
        assert_eq!(
            effects,
            [
                take(MILK_BOTTLES[0]),
                give(MILK_BOTTLES[1]),
                Effect::QuestRecord {
                    quest_id: FRESH_MILK,
                    count: first.1,
                },
            ]
        );
        // The same cow twice running gives nothing; the other one fills on.
        player.quests.upsert(game::QuestRecord {
            quest_id: FRESH_MILK,
            status: QuestStatus::Started {
                kills: vec![first.1],
                info: None,
            },
        });
        player.inventory = traveller(70, 0, false).inventory;
        player
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(MILK_BOTTLES[2], 1));
        let (page, effects) = said(first.0, &player);
        assert_eq!(page, text(keys::MILK_COW_LAST));
        assert!(effects.is_empty());
        let (page, effects) = said(second.0, &player);
        assert_eq!(page, text(keys::MILK_COW_FILLS[2]));
        assert_eq!(
            effects,
            [
                take(MILK_BOTTLES[2]),
                give(MILK_BOTTLES[3]),
                Effect::QuestRecord {
                    quest_id: FRESH_MILK,
                    count: second.1,
                },
            ]
        );
        // A calf drinks it all.
        let (page, effects) = said(CALVES[0], &player);
        assert_eq!(page, text(keys::MILK_CALF_DRINKS));
        assert_eq!(effects, [take(MILK_BOTTLES[2]), give(MILK_BOTTLES[0])]);
    }

    #[test]
    fn victoria_el_nath_and_ludibrium_makers_make_their_items() {
        let menu = |step: Step| match step.prompt {
            Some(Prompt::Selection { text, .. }) => text,
            other => panic!("menu expected, got {other:?}"),
        };
        let ask = |step: Step| match step.prompt {
            Some(Prompt::YesNo { text, .. }) => text,
            other => panic!("question expected, got {other:?}"),
        };
        let take = |item_id, quantity| Effect::TakeItem { item_id, quantity };
        let player = traveller(30, 0, false);
        // Francois upgrades a Bronze Pride to a Steel Pride.
        let mut francois = NpcScript::for_npc(FRANCOIS, &player).unwrap();
        assert_eq!(menu(francois.begin()).matches("#L").count(), 5);
        francois.respond(ScriptInput::Select(2));
        let made = francois.respond(ScriptInput::Select(0)).unwrap();
        assert!(ask(made).contains("#i1002064# 1 #t1002064#"));
        let made = francois.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            made.effects,
            [
                Effect::SpendMeso(40000),
                take(1002064, 1),
                take(4011001, 3),
                Effect::GiveItem {
                    item_id: 1002065,
                    quantity: 1
                }
            ]
        );
        for npc in [CHRISHRAMA, GORDON] {
            let mut maker = NpcScript::for_npc(npc, &player).unwrap();
            assert_eq!(menu(maker.begin()).matches("#L").count(), 4);
            assert!(maker.refused().is_some());
        }

        // Sarah, Tara and Rydole: what a stimulator is, four classes, and
        // four with a stimulator.
        let mut sarah = NpcScript::for_npc(SARAH, &player).unwrap();
        assert_eq!(menu(sarah.begin()).matches("#L").count(), 9);
        assert_eq!(
            page_text(&sarah.respond(ScriptInput::Select(0)).unwrap()),
            text(keys::MAKER_STIMULATOR)
        );
        // Her stimulator upgrades a Bronze Missel to a Steel Missel.
        let mut sarah = NpcScript::for_npc(SARAH, &player).unwrap();
        sarah.begin();
        sarah.respond(ScriptInput::Select(5));
        let asked = ask(sarah.respond(ScriptInput::Select(0)).unwrap());
        assert!(asked.contains("#t1082005#") && asked.contains("#i1082007# 1 "));
        assert!(asked.contains("#i4130000# 1 ") && asked.contains("18000"));
        // Tara's shoes take a shoes stimulator.
        let mut tara = NpcScript::for_npc(TARA, &player).unwrap();
        tara.begin();
        tara.respond(ScriptInput::Select(5));
        assert!(ask(tara.respond(ScriptInput::Select(0)).unwrap()).contains("#i4130001# 1 "));
        // Rydole's thieves' stimulator menu also upgrades claws.
        let mut rydole = NpcScript::for_npc(RYDOLE, &player).unwrap();
        rydole.begin();
        assert_eq!(
            menu(rydole.respond(ScriptInput::Select(8)).unwrap())
                .matches("#L")
                .count(),
            16
        );
        let asked = ask(rydole.respond(ScriptInput::Select(6)).unwrap());
        assert!(asked.contains("#t1472009#") && asked.contains("#i1472008# 1 "));
        assert!(asked.contains("#i4131013# 1 ") && asked.contains("#i4130015# 1 "));
        // A lost item takes everything, the stimulator too, and gives nothing.
        let mut lost = stimulating_maker(
            TARA,
            keys::MAKER_TARA_HELLO,
            &[],
            &[(
                keys::MAKER_SHOES_WARRIOR_STIMULATED,
                item_makers::TARA_WARRIOR,
            )],
            &mut rand::rngs::mock::StepRng::new(0, 0),
        );
        lost.begin();
        lost.respond(ScriptInput::Select(1));
        lost.respond(ScriptInput::Select(0));
        let lost = lost.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            lost.effects,
            [
                Effect::SpendMeso(18000),
                take(4021003, 4),
                take(4011001, 2),
                take(4000021, 45),
                take(4003000, 15),
                take(4130001, 1)
            ]
        );
        assert_eq!(page_text(&lost), text(keys::MAKER_STIMULATOR_LOST));
    }

    #[test]
    fn lord_jonathan_sends_a_corsair_on_his_test_until_bart_is_found() {
        use crate::reactors::{BART_FOUND, BART_RECORD, BART_ROOM, JONATHANS_COURT};
        let mut corsair = player(50);
        let first = |player: &Player| {
            let script = NpcScript::for_npc(LORD_JONATHAN, player).unwrap();
            let step = script.begin();
            (kind(&step), script)
        };
        // Off the quest he only greets.
        assert_eq!(first(&corsair).0, Some(0));

        corsair.quests.upsert(game::QuestRecord {
            quest_id: JONATHANS_COURT,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        let (kind_on_test, mut script) = first(&corsair);
        assert_eq!(kind_on_test, Some(1));
        assert_eq!(
            script.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::RequireEmptyMap(BART_ROOM),
                Effect::WarpToPortal(BART_ROOM, 0),
            ]
        );
        assert_eq!(kind(&script.refused().unwrap()), Some(0));
        assert_eq!(time_limit(BART_ROOM), Some(2 * 60));
        assert!(crate::scripts::SOLO_MAPS.contains(&BART_ROOM));

        // Once Bart is found, the test is over.
        assert!(corsair.quests.set_record(BART_RECORD, BART_FOUND));
        assert_eq!(first(&corsair).0, Some(0));
    }

    #[test]
    fn kyrin_sends_a_pirate_on_her_test_and_marks_it_passed_past_it() {
        use crate::scripts::{KYRIN_PASSED, KYRIN_PASSED_ROOM, KYRIN_TESTS};
        let test = &KYRIN_TESTS[1];
        let mut outlaw = player(50);
        (outlaw.stats.job, outlaw.stats.level, outlaw.stats.map_id) = (522, 120, KYRIN_MAP);
        // Off the quest, her other scripts answer.
        assert!(kyrin_fourth_job_test(&outlaw).is_none());

        outlaw.quests.upsert(game::QuestRecord {
            quest_id: test.quest,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        let mut script = NpcScript::for_npc(KYRIN, &outlaw).unwrap();
        assert_eq!(kind(&script.begin()), Some(1));
        assert_eq!(
            script.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::RequireEmptyMap(test.room),
                Effect::WarpToPortal(test.room, 0),
            ]
        );
        assert_eq!(kind(&script.refused().unwrap()), Some(0));
        assert_eq!(time_limit(test.room), Some(4 * 60));
        assert!(resets_when_empty(test.room));

        // Past the room she marks it passed and sends the Pirate to town.
        outlaw.stats.map_id = KYRIN_PASSED_ROOM;
        let mut script = NpcScript::for_npc(KYRIN, &outlaw).unwrap();
        assert_eq!(kind(&script.begin()), Some(0));
        assert_eq!(
            script.respond(ScriptInput::Next).unwrap().effects,
            [
                Effect::SetRecord {
                    record: test.record,
                    value: KYRIN_PASSED,
                },
                Effect::WarpToPortal(KYRIN_MAP, 0),
            ]
        );

        // Passed, she has no test to give.
        assert!(outlaw.quests.set_record(test.record, KYRIN_PASSED));
        outlaw.stats.map_id = KYRIN_MAP;
        assert!(kyrin_fourth_job_test(&outlaw).is_none());
    }

    #[test]
    fn the_insignificant_being_takes_a_bowman_on_the_quest_into_the_other_world() {
        use crate::scripts::BALROG_WORLD;
        let says = |player: &Player| {
            page_text(
                &NpcScript::for_npc(INSIGNIFICANT_BEING, player)
                    .unwrap()
                    .begin(),
            )
        };
        let mut bowman = player(50);
        (bowman.stats.job, bowman.stats.level) = (322, 120);
        assert_eq!(says(&bowman), text(keys::BALROG_WORLD_NOT_ALLOWED));

        bowman.quests.upsert(game::QuestRecord {
            quest_id: 6108,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        let mut warrior = bowman.clone();
        warrior.stats.job = 112;
        assert_eq!(says(&warrior), text(keys::BALROG_WORLD_JOB));
        let mut low = bowman.clone();
        low.stats.level = 119;
        assert_eq!(says(&low), text(keys::BALROG_WORLD_LEVEL));

        // In at once, one at a time.
        let mut script = NpcScript::for_npc(INSIGNIFICANT_BEING, &bowman).unwrap();
        let step = script.begin();
        assert!(step.prompt.is_none());
        assert_eq!(
            step.effects,
            [
                Effect::RequireEmptyMap(BALROG_WORLD),
                Effect::WarpToPortal(BALROG_WORLD, 0),
            ]
        );
        assert_eq!(
            page_text(&script.refused().unwrap()),
            text(keys::BALROG_WORLD_BUSY)
        );
        assert_eq!(time_limit(BALROG_WORLD), Some(20 * 60));
        assert!(resets_when_empty(BALROG_WORLD));
    }

    #[test]
    fn the_past_papulatus_sends_a_buccaneer_on_the_revelation_back_with_its_record() {
        use crate::events::CLOCKTOWER_DEPTHS;
        use crate::scripts::{NEW_REVELATION, SENT_BACK, SENT_BACK_RECORD};
        let ends = |player: &Player| {
            let mut script = NpcScript::for_npc(PAST_PAPULATUS, player).unwrap();
            assert_eq!(
                page_text(&script.begin()),
                text(keys::PAST_PAPULATUS_SEND_BACK)
            );
            script.respond(ScriptInput::Next).unwrap().effects
        };
        let mut buccaneer = player(50);
        (buccaneer.stats.job, buccaneer.stats.level) = (512, 120);
        assert_eq!(ends(&buccaneer), [Effect::Warp(CLOCKTOWER_DEPTHS)]);

        buccaneer.quests.upsert(game::QuestRecord {
            quest_id: NEW_REVELATION,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        assert_eq!(
            ends(&buccaneer),
            [
                Effect::SetRecord {
                    record: SENT_BACK_RECORD,
                    value: SENT_BACK,
                },
                Effect::Warp(CLOCKTOWER_DEPTHS),
            ]
        );
        assert!(buccaneer.quests.set_record(SENT_BACK_RECORD, SENT_BACK));
        assert_eq!(ends(&buccaneer), [Effect::Warp(CLOCKTOWER_DEPTHS)]);
    }

    #[test]
    fn shulynch_sends_a_corsair_after_delli_who_takes_the_party_in_and_home() {
        use crate::events::{DELLI_LOBBY, NAUTILUS_HIDEOUT};
        let asks = |npc: i32, player: &Player, key: &str, to: i32| {
            let mut script = NpcScript::for_npc(npc, player).unwrap();
            assert_eq!(question(&script.begin()), text(key));
            let step = script.respond(ScriptInput::Yes).unwrap();
            assert_eq!(step.effects, [Effect::WarpToPortal(to, 0)]);
        };
        let mut corsair = player(50);
        (corsair.stats.job, corsair.stats.level) = (522, 120);
        // Without the quest Shulynch says only its own lines.
        assert!(NpcScript::for_npc(SHULYNCH, &corsair).is_none());
        corsair.quests.upsert(game::QuestRecord {
            quest_id: DELLI_QUEST,
            status: QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        asks(
            SHULYNCH,
            &corsair,
            keys::DELLI_SHULYNCH_ASK,
            LOOKING_FOR_DELLI,
        );
        asks(
            SHULYNCH_ON_THE_WAY,
            &corsair,
            keys::DELLI_SHULYNCH_LEAVE,
            TRAINING_ROOM,
        );
        corsair.stats.map_id = NAUTILUS_HIDEOUT;
        asks(DELLI, &corsair, keys::DELLI_HOME_ASK, TRAINING_ROOM);

        // In the lobby, the leader takes the party in.
        corsair.stats.map_id = DELLI_LOBBY;
        let mut ready = NpcScript::delli(Entry::Ready);
        assert_eq!(kind(&ready.begin()), Some(4));
        let step = ready.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(step.effects, [Effect::StartEvent(EventKind::Delli)]);
        assert!(ready.refused().is_some());
        for (entry, key) in [
            (Entry::NoParty, keys::DELLI_NO_PARTY),
            (Entry::NotLeader, keys::DELLI_NOT_LEADER),
            (Entry::Busy, keys::DELLI_BUSY),
        ] {
            let mut delli = NpcScript::delli(entry);
            delli.begin();
            let step = delli.respond(ScriptInput::Select(0)).unwrap();
            assert_eq!(page_text(&step), text(key));
        }
        let mut delli = NpcScript::delli(Entry::NotEligible);
        delli.begin();
        let step = delli.respond(ScriptInput::Select(0)).unwrap();
        assert!(page_text(&step).contains("120"), "{step:?}");
        let mut delli = NpcScript::delli(Entry::Ready);
        delli.begin();
        let step = delli.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(page_text(&step), text(keys::DELLI_INFO));
    }

    #[test]
    fn ellin_takes_the_party_into_the_forest_and_lets_a_member_out() {
        let mut ready = NpcScript::ellin(Entry::Ready, &player(50));
        assert_eq!(kind(&ready.begin()), Some(4));
        let step = ready.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(step.effects, [Effect::StartEvent(EventKind::Ellin)]);
        assert!(ready.refused().is_some());
        for (entry, key) in [
            (Entry::NoParty, keys::ELLIN_NO_PARTY),
            (Entry::NotLeader, keys::ELLIN_NOT_LEADER),
            (Entry::Busy, keys::ELLIN_BUSY),
        ] {
            let mut ellin = NpcScript::ellin(entry, &player(50));
            ellin.begin();
            let step = ellin.respond(ScriptInput::Select(0)).unwrap();
            assert_eq!(page_text(&step), text(key));
        }
        let mut ellin = NpcScript::ellin(Entry::NotEligible, &player(50));
        ellin.begin();
        let step = ellin.respond(ScriptInput::Select(0)).unwrap();
        assert!(page_text(&step).contains("55"), "{step:?}");
        let mut ellin = NpcScript::ellin(Entry::Ready, &player(50));
        ellin.begin();
        let step = ellin.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(page_text(&step), text(keys::ELLIN_INFO));

        // In the forest, the other Ellin lets one out to the prize map.
        let mut member = player(50);
        assert!(NpcScript::for_npc(ELLIN_GUIDE, &member).is_none());
        member.stats.map_id = events::ELLIN_STAGES[0];
        let mut guide = NpcScript::for_npc(ELLIN_GUIDE, &member).unwrap();
        assert_eq!(question(&guide.begin()), text(keys::ELLIN_GUIDE_LEAVE));
        let step = guide.respond(ScriptInput::Yes).unwrap();
        assert_eq!(step.effects, [Effect::WarpToPortal(events::ELLIN_EXIT, 0)]);
        // At the entrance she only welcomes them; at the maze's end she
        // clears it and takes the team on; on the fourth stage she has the
        // flowers' marbles.
        member.stats.map_id = events::ELLIN_ENTRY;
        let guide = NpcScript::for_npc(ELLIN_GUIDE, &member).unwrap();
        assert_eq!(page_text(&guide.begin()), text(keys::ELLIN_GUIDE_WELCOME));
        member.stats.map_id = events::ELLIN_STAGES[2];
        let mut guide = NpcScript::for_npc(ELLIN_GUIDE, &member).unwrap();
        let step = guide.begin();
        assert_eq!(page_text(&step), text(keys::ELLIN_GUIDE_MAZE_END));
        let command = |command| Effect::EventCommand(command);
        assert_eq!(step.effects, [command(EventCommand::ClearStage(3))]);
        let step = guide.respond(ScriptInput::Next).unwrap();
        let on = command(EventCommand::MoveTeam(events::ELLIN_STAGES[3]));
        assert_eq!((step.effects, step.prompt), (vec![on], None));
        member.stats.map_id = events::ELLIN_STAGES[3];
        assert!(NpcScript::for_npc(ELLIN_GUIDE, &member).is_none());
        // So do the maze's mileposts.
        member.stats.map_id = events::ELLIN_STAGES[2];
        let mut post = NpcScript::for_npc(ELLIN_MILEPOST, &member).unwrap();
        assert_eq!(question(&post.begin()), text(keys::ELLIN_MILEPOST_LEAVE));
        let step = post.respond(ScriptInput::Yes).unwrap();
        assert_eq!(step.effects, [Effect::WarpToPortal(events::ELLIN_EXIT, 0)]);
    }

    #[test]
    fn the_carnival_guide_grades_the_battle_and_pays_its_exp() {
        let mut member = player(50);
        member.stats.map_id = carnival::win_map(1);
        let back = Effect::WarpToPortal(carnival::CARNIVAL_LOBBY, 0);
        // With no battle to grade, straight back.
        let mut guide = NpcScript::for_npc(CARNIVAL_GUIDE, &member).unwrap();
        assert_eq!(guide.begin().effects, [back.clone()]);
        member.carnival_result = Some((true, 120));
        let mut guide = NpcScript::for_npc(CARNIVAL_GUIDE, &member).unwrap();
        let said = content::text().get(keys::CARNIVAL_GUIDE_WON, &[("grade", "B")]);
        assert_eq!(page_text(&guide.begin()), said);
        let step = guide.respond(ScriptInput::Next).unwrap();
        assert_eq!(step.effects, [Effect::GainExp(25_500), back.clone()]);
        // A draw or a loss grades in the losers' room.
        member.stats.map_id = carnival::lose_map(1);
        member.carnival_result = Some((false, 0));
        let mut guide = NpcScript::for_npc(CARNIVAL_GUIDE, &member).unwrap();
        let said = content::text().get(keys::CARNIVAL_GUIDE_LOST, &[("grade", "D")]);
        assert_eq!(page_text(&guide.begin()), said);
        let step = guide.respond(ScriptInput::Next).unwrap();
        assert_eq!(step.effects, [Effect::GainExp(2_500), back]);
    }

    #[test]
    fn spiegelmann_shows_the_rooms_and_puts_challenges() {
        let entries: Vec<_> = (1..=6)
            .map(|room| (EventKind::Carnival(room), Entry::Busy))
            .collect();
        let mut leader = player(50);
        leader.stats.level = 40;
        leader.stats.map_id = carnival::CARNIVAL_LOBBY;
        let rooms = [
            carnival::Room::Waiting { size: 2, level: 41 },
            carnival::Room::Battle,
        ];
        let mut script = NpcScript::spiegelmann(&entries, &rooms, &leader);
        script.begin();
        let Some(Prompt::Selection { text: menu, .. }) =
            script.respond(ScriptInput::Select(0)).unwrap().prompt
        else {
            panic!("room list expected");
        };
        let label = |key, params: &[(&str, &str)]| content::text().get(key, params);
        let waiting = [("room", "1"), ("size", "2"), ("level", "41")];
        assert!(menu.contains(&label(keys::CARNIVAL_SPIEGELMANN_WAITING, &waiting)));
        assert!(menu.contains(&label(keys::CARNIVAL_SPIEGELMANN_TAKEN, &[("room", "2")])));
        let free = [("room", "3"), ("least", "2"), ("most", "4")];
        assert!(menu.contains(&label(keys::CARNIVAL_SPIEGELMANN_ROOM, &free)));
        // A room in use is picked like a free one; the directory decides.
        assert_eq!(
            script.respond(ScriptInput::Select(0)).unwrap().effects,
            [Effect::CarnivalRoom(1)]
        );
        let said = |refused| page_text(&NpcScript::spiegelmann_picked(1, refused).begin());
        assert_eq!(said(None), text(keys::CARNIVAL_SPIEGELMANN_SENT));
        let unequal = Some(carnival::Refusal::Unequal);
        assert_eq!(said(unequal), text(keys::CARNIVAL_SPIEGELMANN_UNEQUAL));

        // The waiting leader decides.
        let challengers = [carnival::Challenger {
            name: "Blue".into(),
            level: 33,
        }];
        let ask = |accept| {
            let mut script = NpcScript::carnival_challenge(7, &challengers);
            let asked = question(&script.begin());
            let input = if accept {
                ScriptInput::Yes
            } else {
                ScriptInput::No
            };
            (asked, script.respond(input).unwrap().effects)
        };
        let (asked, effects) = ask(true);
        assert!(asked.contains("#bBlue#k, level 33"), "{asked}");
        let answered = |accept| Effect::CarnivalAnswer {
            challenger: 7,
            accept,
        };
        assert_eq!(effects, [answered(true)]);
        assert_eq!(ask(false).1, [answered(false)]);

        // In the waiting room, one waits or goes back to the field.
        leader.stats.map_id = carnival::room_map(4);
        let mut wait = NpcScript::for_npc(CARNIVAL_ROOM_NPC, &leader).unwrap();
        assert_eq!(page_text(&wait.begin()), text(keys::CARNIVAL_ROOM_WAIT));
        let mut leave = NpcScript::for_npc(CARNIVAL_ROOM_EXIT, &leader).unwrap();
        assert_eq!(question(&leave.begin()), text(keys::CARNIVAL_ROOM_LEAVE));
        assert_eq!(
            leave.respond(ScriptInput::Yes).unwrap().effects,
            [Effect::WarpToPortal(carnival::CARNIVAL_LOBBY, 0)]
        );
    }

    #[test]
    fn spiegelmann_sends_a_leader_to_a_room_and_others_to_the_field() {
        let entries = |entry| {
            (1..=6)
                .map(|room| (EventKind::Carnival(room), entry))
                .collect::<Vec<_>>()
        };
        let at = |level, map_id| {
            let mut player = player(50);
            player.stats.level = level;
            player.stats.map_id = map_id;
            player
        };
        let lobby = carnival::CARNIVAL_LOBBY;
        // Join, then the room.
        let enter = |entry, level| {
            let mut script = NpcScript::spiegelmann(&entries(entry), &[], &at(level, lobby));
            assert_eq!(kind(&script.begin()), Some(4));
            script.respond(ScriptInput::Select(0)).unwrap()
        };
        let mut script = NpcScript::spiegelmann(&entries(Entry::Ready), &[], &at(35, lobby));
        script.begin();
        assert_eq!(
            kind(&script.respond(ScriptInput::Select(0)).unwrap()),
            Some(4)
        );
        assert_eq!(
            script.respond(ScriptInput::Select(4)).unwrap().effects,
            [Effect::CarnivalRoom(5)]
        );
        let levels = event_terms(&events::CARNIVAL[0], keys::CARNIVAL_SPIEGELMANN_LEVELS);
        assert_eq!(page_text(&enter(Entry::Ready, 51)), levels);
        assert_eq!(page_text(&enter(Entry::Ready, 29)), levels);
        let no_party = text(keys::CARNIVAL_SPIEGELMANN_NO_PARTY);
        assert_eq!(page_text(&enter(Entry::NoParty, 40)), no_party);
        let not_leader = text(keys::CARNIVAL_SPIEGELMANN_NOT_LEADER);
        assert_eq!(page_text(&enter(Entry::NotLeader, 40)), not_leader);
        let mut script = NpcScript::spiegelmann(&entries(Entry::NotEligible), &[], &at(40, lobby));
        script.begin();
        script.respond(ScriptInput::Select(0)).unwrap();
        let terms = event_terms(
            EventKind::Carnival(6).event(),
            keys::CARNIVAL_SPIEGELMANN_TERMS,
        );
        assert_eq!(
            page_text(&script.respond(ScriptInput::Select(5)).unwrap()),
            terms
        );

        // In town, he offers the way to the field and keeps the way back.
        let mut script = NpcScript::for_npc(SPIEGELMANN, &at(30, 220000000)).unwrap();
        assert_eq!(kind(&script.begin()), Some(1));
        assert_eq!(
            script.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::SaveLocation(CARNIVAL_RETURN, Some(220000000)),
                Effect::WarpToPortal(lobby, 0),
            ]
        );
        let mut script = NpcScript::for_npc(SPIEGELMANN, &at(60, 220000000)).unwrap();
        assert_eq!(page_text(&script.begin()), levels);
    }

    #[test]
    fn duarte_takes_a_player_into_the_pyramid_in_the_mode_they_choose() {
        let entries = |entry| {
            events::pyramid_kinds()
                .map(|kind| (kind, entry))
                .collect::<Vec<_>>()
        };
        let at = |level| {
            let mut player = player(50);
            player.stats.level = level;
            player.stats.map_id = events::PYRAMID_LOBBY;
            player
        };
        // Lobby menu, alone or with a party, then the mode.
        let choose = |entry, level, party: i32, mode: i32| {
            let mut duarte = NpcScript::duarte(&entries(entry), &at(level));
            assert_eq!(kind(&duarte.begin()), Some(4));
            duarte.respond(ScriptInput::Select(1)).unwrap();
            duarte.respond(ScriptInput::Select(party)).unwrap();
            duarte.respond(ScriptInput::Select(mode)).unwrap()
        };
        let hard = EventKind::Pyramid(PyramidMode::Hard);
        assert_eq!(
            choose(Entry::Ready, 45, 0, 2).effects,
            [Effect::StartEvent(hard)]
        );
        let hell = EventKind::PyramidParty(PyramidMode::Hell);
        assert_eq!(
            choose(Entry::Ready, 70, 1, 3).effects,
            [Effect::StartEvent(hell)]
        );
        let low = content::text().get(keys::PYRAMID_DUARTE_LOW_LEVEL, &[("low", "40")]);
        assert_eq!(page_text(&choose(Entry::Ready, 39, 0, 3)), low);
        let only_hell = content::text().get(keys::PYRAMID_DUARTE_ONLY_HELL, &[("high", "60")]);
        assert_eq!(page_text(&choose(Entry::Ready, 61, 0, 0)), only_hell);
        let busy = text(keys::PYRAMID_DUARTE_BUSY);
        assert_eq!(page_text(&choose(Entry::Busy, 45, 0, 0)), busy);
        let no_party = text(keys::PYRAMID_DUARTE_NO_PARTY);
        assert_eq!(page_text(&choose(Entry::NoParty, 45, 1, 0)), no_party);
        let not_leader = text(keys::PYRAMID_DUARTE_NOT_LEADER);
        assert_eq!(page_text(&choose(Entry::NotLeader, 45, 1, 0)), not_leader);
        let terms = event_terms(
            EventKind::PyramidParty(PyramidMode::Easy).event(),
            keys::PYRAMID_DUARTE_PARTY_TERMS,
        );
        assert_eq!(page_text(&choose(Entry::NotEligible, 45, 1, 0)), terms);
        let mut ready = NpcScript::duarte(&entries(Entry::Ready), &at(45));
        let refused = ready.refused().unwrap();
        assert_eq!(page_text(&refused), text(keys::PYRAMID_DUARTE_REFUSED));
        // The about pages.
        let mut about = NpcScript::duarte(&entries(Entry::Ready), &at(45));
        about.begin();
        let step = about.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(page_text(&step), text(keys::PYRAMID_DUARTE_INFO));

        // On a stage he lets the player give up, back to the lobby.
        let mut player = at(45);
        player.stats.map_id = EventKind::Pyramid(PyramidMode::Easy).event().stage_map(3);
        let mut forfeit = NpcScript::in_event(DUARTE, &player, true, false).unwrap();
        assert_eq!(
            question(&forfeit.begin()),
            text(keys::PYRAMID_DUARTE_FORFEIT_ASK)
        );
        let step = forfeit.respond(ScriptInput::Yes).unwrap();
        assert_eq!(step.effects, [Effect::Warp(events::PYRAMID_LOBBY)]);
        // After the run: from the mode's result map on to its room in the
        // Shades, and from there back to the lobby.
        player.stats.map_id = events::PYRAMID_RESULT + 2;
        let mut result = NpcScript::for_npc(DUARTE, &player).unwrap();
        assert_eq!(
            question(&result.begin()),
            text(keys::PYRAMID_DUARTE_RESULT_ASK)
        );
        let step = result.respond(ScriptInput::Yes).unwrap();
        assert_eq!(step.effects, [Effect::Warp(events::PYRAMID_SHADES + 2)]);
        player.stats.map_id = events::PYRAMID_SHADES + 2;
        let mut shades = NpcScript::for_npc(DUARTE, &player).unwrap();
        assert_eq!(
            question(&shades.begin()),
            text(keys::PYRAMID_DUARTE_SHADES_ASK)
        );
        let step = shades.respond(ScriptInput::Yes).unwrap();
        assert_eq!(step.effects, [Effect::Warp(events::PYRAMID_LOBBY)]);
    }

    #[test]
    fn duarte_gives_a_gem_for_pharaoh_yetis_tomb_and_takes_it_back() {
        let hard = EventKind::PyramidTomb(PyramidMode::Hard);
        let sapphire = events::PYRAMID_GEM;
        let topaz = events::PYRAMID_GEM + 3;
        // In the lobby: the gem chosen, if held, for a room of its mode.
        let tomb = |player: &Player, gem: i32| {
            let mut duarte = NpcScript::duarte(&[], player);
            duarte.begin();
            duarte.respond(ScriptInput::Select(2)).unwrap();
            duarte.respond(ScriptInput::Select(gem - sapphire)).unwrap()
        };
        let mut player = player(50);
        player.stats.level = 45;
        player.stats.map_id = events::PYRAMID_LOBBY;
        assert_eq!(
            page_text(&tomb(&player, topaz)),
            text(keys::PYRAMID_DUARTE_TOMB_NO_GEM)
        );
        player
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(topaz, 2));
        let take = Effect::TakeItem {
            item_id: topaz,
            quantity: 1,
        };
        let hell = Effect::StartEvent(EventKind::PyramidTomb(PyramidMode::Hell));
        assert_eq!(tomb(&player, topaz).effects, [take, hell]);
        let mut low = player.clone();
        low.stats.level = 39;
        let terms = event_terms(hard.event(), keys::PYRAMID_DUARTE_LOW_LEVEL);
        assert_eq!(page_text(&tomb(&low, topaz)), terms);

        // In Hard's room of the Shades after a run through it: the Tomb at
        // once, or its gem and the lobby.
        player.stats.map_id = events::PYRAMID_SHADES + 2;
        player.pyramid_gem = Some(2);
        let shades = |player: &Player, choice: i32| {
            let mut duarte = NpcScript::for_npc(DUARTE, player).unwrap();
            duarte.begin();
            (duarte.respond(ScriptInput::Select(choice)).unwrap(), duarte)
        };
        let (now, mut script) = shades(&player, 0);
        assert_eq!(now.effects, [Effect::StartEvent(hard)]);
        let refused = script.refused().unwrap();
        assert_eq!(page_text(&refused), text(keys::PYRAMID_DUARTE_TOMB_FULL));
        let (later, _) = shades(&player, 1);
        assert_eq!(page_text(&later), text(keys::PYRAMID_DUARTE_SHADES_GEM));
        let gem = Effect::GiveItem {
            item_id: sapphire + 2,
            quantity: 1,
        };
        assert_eq!(later.effects, [gem, Effect::Warp(events::PYRAMID_LOBBY)]);
        let mut full = player.clone();
        let room = full.inventory.free(InventoryType::Etc);
        for slot in 1..=(room as i16 + 1) {
            full.inventory
                .insert(InventoryType::Etc, slot, game::Item::stack(4_000_000, 1));
        }
        let (later, _) = shades(&full, 1);
        assert_eq!(page_text(&later), text(keys::PYRAMID_DUARTE_SHADES_NO_ROOM));
        // Another mode's run, or none, gives nothing here.
        player.pyramid_gem = Some(1);
        let mut back = NpcScript::for_npc(DUARTE, &player).unwrap();
        assert_eq!(
            question(&back.begin()),
            text(keys::PYRAMID_DUARTE_SHADES_ASK)
        );
    }

    #[test]
    fn duarte_gives_the_protector_of_pharaoh_medal_for_50000_pyramid_kills() {
        use crate::pyramid::{PROTECTOR_KILLS, PROTECTOR_MEDAL, PROTECTOR_QUEST};
        let equip = |item_id| game::Item::equipment(item_id, game::EquipStats::default());
        let medal = |player: &Player| {
            let entries: Vec<_> = events::pyramid_kinds()
                .map(|kind| (kind, Entry::Ready))
                .collect();
            let mut duarte = NpcScript::duarte(&entries, player);
            duarte.begin();
            duarte.respond(ScriptInput::Select(4)).unwrap()
        };
        let mut hunter = player(50);
        hunter.stats.map_id = events::PYRAMID_LOBBY;
        hunter.quests.set_record(PROTECTOR_KILLS, "49999");
        let more = [("kills", "49999"), ("left", "1")];
        assert_eq!(
            page_text(&medal(&hunter)),
            content::text().get(keys::PYRAMID_DUARTE_MEDAL_MORE, &more)
        );
        hunter.quests.set_record(PROTECTOR_KILLS, "50000");
        let mut full = hunter.clone();
        let room = full.inventory.free(InventoryType::Equip);
        for slot in 1..=(room as i16 + 1) {
            full.inventory
                .insert(InventoryType::Equip, slot, equip(1_002_000));
        }
        assert_eq!(
            page_text(&medal(&full)),
            text(keys::PYRAMID_DUARTE_MEDAL_ROOM)
        );
        let give = Effect::GiveItem {
            item_id: PROTECTOR_MEDAL,
            quantity: 1,
        };
        let step = medal(&hunter);
        assert_eq!(page_text(&step), text(keys::PYRAMID_DUARTE_MEDAL_GIVEN));
        assert_eq!(step.effects, [give.clone()]);
        // Started at Dalair, the quest completes with it.
        hunter.quests.upsert(game::QuestRecord {
            quest_id: PROTECTOR_QUEST,
            status: game::QuestStatus::Started {
                kills: Vec::new(),
                info: None,
            },
        });
        let step = medal(&hunter);
        assert_eq!(step.effects, [give, Effect::CompleteQuest(PROTECTOR_QUEST)]);
        // One each.
        hunter
            .inventory
            .insert(InventoryType::Equip, 1, equip(PROTECTOR_MEDAL));
        assert_eq!(
            page_text(&medal(&hunter)),
            text(keys::PYRAMID_DUARTE_MEDAL_HAVE)
        );
    }

    #[test]
    fn so_gong_tries_a_first_timer_himself_before_the_dojo() {
        let mut first = player(50);
        first.stats.level = 25;
        for entry in [Entry::Ready, Entry::Busy] {
            let mut hall = NpcScript::so_gong(entry, Entry::NoParty, &first);
            hall.begin();
            let step = hall.respond(ScriptInput::Select(0)).unwrap();
            assert_eq!(question(&step), text(keys::DOJO_SO_GONG_TRIAL_ASK));
            let step = hall.respond(ScriptInput::Yes).unwrap();
            assert_eq!(step.effects, [Effect::StartEvent(EventKind::DojoTrial)]);
        }
        let mut wary = NpcScript::so_gong(Entry::Ready, Entry::NoParty, &first);
        wary.begin();
        wary.respond(ScriptInput::Select(0)).unwrap();
        let step = wary.respond(ScriptInput::No).unwrap();
        assert_eq!(page_text(&step), text(keys::DOJO_SO_GONG_TRIAL_NO));
        assert!(step.effects.is_empty());
    }

    #[test]
    fn so_gong_takes_a_challenger_in_alone_and_lets_them_give_up() {
        let at = |level| {
            let mut player = player(50);
            player.stats.level = level;
            player.dojo.tutorial_done = true;
            player
        };
        let mut low = NpcScript::so_gong(Entry::Ready, Entry::NoParty, &at(24));
        assert_eq!(page_text(&low.begin()), text(keys::DOJO_SO_GONG_LOW_LEVEL));
        let mut ready = NpcScript::so_gong(Entry::Ready, Entry::NoParty, &at(25));
        assert_eq!(kind(&ready.begin()), Some(4));
        let step = ready.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(step.effects, [Effect::StartEvent(EventKind::Dojo)]);
        let refused = ready.refused().unwrap();
        assert_eq!(page_text(&refused), text(keys::DOJO_SO_GONG_BUSY));
        let mut busy = NpcScript::so_gong(Entry::Busy, Entry::NoParty, &at(25));
        busy.begin();
        let step = busy.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(page_text(&step), text(keys::DOJO_SO_GONG_BUSY));
        let mut about = NpcScript::so_gong(Entry::Ready, Entry::NoParty, &at(25));
        about.begin();
        let step = about.respond(ScriptInput::Select(5)).unwrap();
        assert_eq!(page_text(&step), text(keys::DOJO_SO_GONG_INFO));

        // On a fighting floor he lets them give up.
        let mut challenger = at(30);
        challenger.stats.map_id = events::DOJO_FIRST + 200;
        let mut quit = NpcScript::in_event(SO_GONG, &challenger, true, false).unwrap();
        assert_eq!(question(&quit.begin()), text(keys::DOJO_SO_GONG_QUIT));
        let step = quit.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            step.effects,
            [
                Effect::WarpToPortal(events::DOJO_EXIT, 0),
                Effect::Pink(text(keys::DOJO_SO_GONG_QUIT_DONE)),
            ]
        );
        let mut stay = NpcScript::in_event(SO_GONG, &challenger, true, false).unwrap();
        stay.begin();
        let step = stay.respond(ScriptInput::No).unwrap();
        assert_eq!(page_text(&step), text(keys::DOJO_SO_GONG_QUIT_NO));

        // On a rest floor they go on to the next, or out by the exit's st00.
        challenger.stats.map_id = events::DOJO_FIRST + 500;
        let mut rest = NpcScript::in_event(SO_GONG, &challenger, true, false).unwrap();
        assert_eq!(kind(&rest.begin()), Some(4));
        let step = rest.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(
            step.effects,
            [
                Effect::DojoStage(0),
                Effect::WarpToPortal(events::DOJO_FIRST + 600, 0)
            ]
        );
        let mut leave = NpcScript::in_event(SO_GONG, &challenger, true, false).unwrap();
        leave.begin();
        leave.respond(ScriptInput::Select(1)).unwrap();
        let step = leave.respond(ScriptInput::Yes).unwrap();
        assert_eq!(step.effects, [Effect::WarpToPortal(events::DOJO_EXIT, 1)]);
    }

    #[test]
    fn so_gong_takes_a_party_in_with_its_leader_and_moves_it_on_together() {
        let mut leader = player(50);
        leader.stats.level = 30;
        leader.dojo.tutorial_done = true;
        let party = |entry| {
            let mut gong = NpcScript::so_gong(Entry::Ready, entry, &leader);
            gong.begin();
            gong.respond(ScriptInput::Select(1)).unwrap()
        };
        assert_eq!(
            party(Entry::Ready).effects,
            [Effect::StartEvent(EventKind::DojoParty)]
        );
        let spread = text(keys::DOJO_SO_GONG_PARTY_SPREAD);
        assert_eq!(page_text(&party(Entry::NotEligible)), spread);
        let lead = text(keys::DOJO_SO_GONG_PARTY_LEADER);
        assert_eq!(page_text(&party(Entry::NoParty)), lead);
        assert_eq!(page_text(&party(Entry::NotLeader)), lead);
        let busy = text(keys::DOJO_SO_GONG_BUSY);
        assert_eq!(page_text(&party(Entry::Busy)), busy);

        // On a party rest floor the leader takes everyone on; a member goes
        // alone, and there is no score to record.
        leader.stats.map_id = events::DOJO_PARTY.stage_map(6);
        let on = |leads| {
            let mut rest = NpcScript::in_event(SO_GONG, &leader, leads, false).unwrap();
            rest.begin();
            rest.respond(ScriptInput::Select(0)).unwrap().effects
        };
        assert_eq!(
            on(true),
            [
                Effect::DojoStage(0),
                Effect::EventCommand(EventCommand::DojoOn(6))
            ]
        );
        assert_eq!(
            on(false),
            [
                Effect::DojoStage(0),
                Effect::WarpToPortal(events::DOJO_PARTY.stage_map(7), 0)
            ]
        );
        let rest = NpcScript::in_event(SO_GONG, &leader, true, false).unwrap();
        let Some(Prompt::Selection { text: menu, .. }) = rest.begin().prompt else {
            panic!("a menu expected");
        };
        assert!(!menu.contains(&text(keys::DOJO_SO_GONG_RECORD)), "{menu}");

        // On a party fighting floor he lets them give up, as alone.
        leader.stats.map_id = events::DOJO_PARTY.stage_map(2);
        let quit = NpcScript::in_event(SO_GONG, &leader, true, false).unwrap();
        assert_eq!(question(&quit.begin()), text(keys::DOJO_SO_GONG_QUIT));
    }

    #[test]
    fn so_gong_records_a_rest_floors_score_and_starts_after_it_next_time() {
        let mut challenger = player(50);
        challenger.stats.level = 30;
        challenger.stats.map_id = events::DOJO.stage_map(12);
        challenger.dojo.tutorial_done = true;
        let mut rest = NpcScript::in_event(SO_GONG, &challenger, true, false).unwrap();
        rest.begin();
        let step = rest.respond(ScriptInput::Select(2)).unwrap();
        assert_eq!(question(&step), text(keys::DOJO_SO_GONG_RECORD_ASK));
        let step = rest.respond(ScriptInput::Yes).unwrap();
        assert_eq!(page_text(&step), text(keys::DOJO_SO_GONG_RECORDED));
        assert_eq!(step.effects, [Effect::DojoStage(12)]);
        let mut wavers = NpcScript::in_event(SO_GONG, &challenger, true, false).unwrap();
        wavers.begin();
        wavers.respond(ScriptInput::Select(2)).unwrap();
        let step = wavers.respond(ScriptInput::No).unwrap();
        assert_eq!(page_text(&step), text(keys::DOJO_SO_GONG_RECORD_NO));
        assert!(step.effects.is_empty());
        challenger.dojo.stage = 12;
        let mut again = NpcScript::in_event(SO_GONG, &challenger, true, false).unwrap();
        again.begin();
        again.respond(ScriptInput::Select(2)).unwrap();
        let step = again.respond(ScriptInput::Yes).unwrap();
        assert_eq!(page_text(&step), text(keys::DOJO_SO_GONG_RECORD_KEPT));
        assert!(step.effects.is_empty());

        // In the hall he offers the round reached, or erases it.
        challenger.stats.map_id = events::DOJO.lobby;
        let mut hall = NpcScript::so_gong(Entry::Ready, Entry::NoParty, &challenger);
        hall.begin();
        let step = hall.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(
            question(&step),
            content::text().get(keys::DOJO_SO_GONG_RESUME, &[("round", "10")])
        );
        let step = hall.respond(ScriptInput::Yes).unwrap();
        assert_eq!(step.effects, [Effect::StartEvent(EventKind::Dojo)]);
        let mut erase = NpcScript::so_gong(Entry::Ready, Entry::NoParty, &challenger);
        erase.begin();
        erase.respond(ScriptInput::Select(0)).unwrap();
        let step = erase.respond(ScriptInput::No).unwrap();
        assert_eq!(step.effects, [Effect::DojoStage(0)]);
        assert!(step.prompt.is_none());
    }

    #[test]
    fn so_gong_sells_belts_for_training_points_resets_them_and_hands_out_medals() {
        use game::dojo::{BELTS, MEDAL_KILLS};
        let mut challenger = player(50);
        challenger.stats.level = 40;
        challenger.dojo.points = 2_500;
        let equip = |item_id| game::Item::equipment(item_id, game::EquipStats::default());
        let choose = |player: &Player, choice: i32, belt: i32| {
            let mut gong = NpcScript::so_gong(Entry::Ready, Entry::NoParty, player);
            gong.begin();
            gong.respond(ScriptInput::Select(choice)).unwrap();
            gong.respond(ScriptInput::Select(belt)).unwrap()
        };
        let give = |item_id| Effect::GiveItem {
            item_id,
            quantity: 1,
        };
        let take = |item_id| Effect::TakeItem {
            item_id,
            quantity: 1,
        };
        let mut gong = NpcScript::so_gong(Entry::Ready, Entry::NoParty, &challenger);
        gong.begin();
        let menu = gong.respond(ScriptInput::Select(2)).unwrap();
        let Some(Prompt::Selection { text: menu, .. }) = menu.prompt else {
            panic!("belts expected, got {menu:?}");
        };
        let points = content::text().get(keys::DOJO_SO_GONG_BELTS, &[("points", "2500")]);
        assert!(menu.starts_with(&points), "{menu}");
        let white = choose(&challenger, 2, 0);
        assert_eq!(white.effects, [give(BELTS[0]), Effect::DojoPoints(-200)]);

        // The yellow belt takes the white one, unequipped.
        let needs = |lacking: &str| {
            let (belt, before) = (BELTS[1].to_string(), BELTS[0].to_string());
            let before = content::text().get(keys::DOJO_SO_GONG_BELT_BEFORE, &[("belt", &before)]);
            content::text().get(
                keys::DOJO_SO_GONG_BELT_NEEDS,
                &[
                    ("belt", &belt),
                    ("before", &before),
                    ("level", "35"),
                    ("points", "1800"),
                    ("lacking", lacking),
                ],
            )
        };
        let lacks = content::text().get(
            keys::DOJO_SO_GONG_BELT_LACKING,
            &[("lacking", &text(keys::DOJO_SO_GONG_BELT_LACKS_BELT))],
        );
        assert_eq!(page_text(&choose(&challenger, 2, 1)), needs(&lacks));
        let mut wearing = challenger.clone();
        wearing
            .inventory
            .insert(InventoryType::Equipped, -50, equip(BELTS[0]));
        assert_eq!(page_text(&choose(&wearing, 2, 1)), needs(&lacks));
        challenger
            .inventory
            .insert(InventoryType::Equip, 1, equip(BELTS[0]));
        let yellow = choose(&challenger, 2, 1);
        assert_eq!(
            page_text(&yellow),
            content::text().get(keys::DOJO_SO_GONG_BELT_GIVEN, &[("belt", "1132001")])
        );
        assert_eq!(
            yellow.effects,
            [take(BELTS[0]), give(BELTS[1]), Effect::DojoPoints(-1800)]
        );
        challenger.stats.level = 35;
        assert_eq!(page_text(&choose(&challenger, 2, 1)), needs(""));

        // A full EQUIP inventory shows no belts.
        let mut full = challenger.clone();
        let room = full.inventory.free(InventoryType::Equip);
        for slot in 2..=(room as i16 + 1) {
            full.inventory
                .insert(InventoryType::Equip, slot, equip(1_002_000));
        }
        let mut gong = NpcScript::so_gong(Entry::Ready, Entry::NoParty, &full);
        gong.begin();
        let step = gong.respond(ScriptInput::Select(2)).unwrap();
        assert_eq!(page_text(&step), text(keys::DOJO_SO_GONG_ROOM));

        // The points can be reset.
        let mut reset = NpcScript::so_gong(Entry::Ready, Entry::NoParty, &challenger);
        reset.begin();
        let step = reset.respond(ScriptInput::Select(3)).unwrap();
        assert_eq!(question(&step), text(keys::DOJO_SO_GONG_RESET_ASK));
        let step = reset.respond(ScriptInput::Yes).unwrap();
        assert_eq!(page_text(&step), text(keys::DOJO_SO_GONG_RESET_DONE));
        assert_eq!(step.effects, [Effect::DojoPoints(-2_500)]);

        // A first medal is tried for, then handed over after 100 wins.
        let medal = |player: &Player| {
            let mut gong = NpcScript::so_gong(Entry::Ready, Entry::NoParty, player);
            gong.begin();
            gong.respond(ScriptInput::Select(4)).unwrap()
        };
        let mano = [("medal", "1142033")];
        let step = medal(&challenger);
        assert_eq!(
            question(&step),
            content::text().get(keys::DOJO_SO_GONG_MEDAL_TRY, &mano)
        );
        let mut gong = NpcScript::so_gong(Entry::Ready, Entry::NoParty, &challenger);
        gong.begin();
        gong.respond(ScriptInput::Select(4)).unwrap();
        let step = gong.respond(ScriptInput::Yes).unwrap();
        assert_eq!(step.effects, [Effect::DojoMedal { stage: 1, kills: 0 }]);
        challenger.dojo.vanquisher_stage = 1;
        challenger.dojo.vanquisher_kills = MEDAL_KILLS - 1;
        assert_eq!(
            page_text(&medal(&challenger)),
            content::text().get(
                keys::DOJO_SO_GONG_MEDAL_MORE,
                &[("medal", "1142033"), ("kills", "1")]
            )
        );
        challenger.dojo.vanquisher_kills = MEDAL_KILLS;
        full.dojo = challenger.dojo;
        assert_eq!(
            page_text(&medal(&full)),
            text(keys::DOJO_SO_GONG_MEDAL_ROOM)
        );
        let step = medal(&challenger);
        assert_eq!(
            page_text(&step),
            content::text().get(keys::DOJO_SO_GONG_MEDAL_GIVEN, &mano)
        );
        assert_eq!(
            step.effects,
            [give(1142033), Effect::DojoMedal { stage: 2, kills: 0 }]
        );
        challenger.dojo.vanquisher_stage = game::dojo::ROUNDS + 1;
        assert_eq!(
            page_text(&medal(&challenger)),
            text(keys::DOJO_SO_GONG_MEDAL_ALL)
        );
    }

    #[test]
    fn the_dojo_board_saves_where_it_was_touched() {
        let mut reader = player(50);
        reader.stats.map_id = events::DOJO_BOARD_MAP;
        let mut board = NpcScript::for_npc(DOJO_BOARD, &reader).unwrap();
        assert_eq!(kind(&board.begin()), Some(4));
        let step = board.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(question(&step), text(keys::DOJO_BOARD_ASK));
        let step = board.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            step.effects,
            [
                Effect::SaveLocation(DOJO_RETURN, Some(events::DOJO_BOARD_MAP)),
                Effect::WarpToPortal(events::DOJO_ENTRANCE, events::DOJO_BOARD_PORTAL),
            ]
        );
        let mut read = NpcScript::for_npc(DOJO_BOARD, &reader).unwrap();
        read.begin();
        let step = read.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(page_text(&step), text(keys::DOJO_BOARD_NOTICE_0));
        let step = read.respond(ScriptInput::Next).unwrap();
        assert_eq!(page_text(&step), text(keys::DOJO_BOARD_NOTICE_1));
    }

    fn slide(step: &Step) -> String {
        match &step.prompt {
            Some(Prompt::Slide { set: 0, menu, .. }) => menu.clone(),
            other => panic!("slide menu expected, got {other:?}"),
        }
    }

    #[test]
    fn the_dimensional_mirror_offers_what_the_level_allows() {
        let menu = |level: u8| {
            let mut traveller = player(50);
            traveller.stats.level = level;
            let mirror = NpcScript::for_npc(DIMENSIONAL_MIRROR, &traveller).unwrap();
            slide(&mirror.begin())
        };
        let [dojo, carnival, pyramid] = [
            keys::MIRROR_DOJO,
            keys::MIRROR_CARNIVAL,
            keys::MIRROR_PYRAMID,
        ]
        .map(text);
        let nowhere = format!("#-1#{}", text(keys::MIRROR_NOWHERE));
        assert_eq!(menu(19), nowhere);
        assert_eq!(menu(24), nowhere);
        assert_eq!(menu(25), format!("#1#{dojo}"));
        assert_eq!(menu(30), format!("#1#{dojo}#2#{carnival}"));
        assert_eq!(menu(40), format!("#1#{dojo}#2#{carnival}#5#{pyramid}"));
        assert_eq!(menu(51), format!("#1#{dojo}#5#{pyramid}"));
    }

    #[test]
    fn the_dimensional_mirror_keeps_the_town_for_the_way_back() {
        let mut traveller = player(50);
        traveller.stats.level = 45;
        traveller.stats.map_id = 100000000;
        let pick = |id| {
            let mut mirror = NpcScript::for_npc(DIMENSIONAL_MIRROR, &traveller).unwrap();
            assert_eq!(kind(&mirror.begin()), Some(14));
            mirror.respond(ScriptInput::Select(id))
        };
        let went = |kind, map, portal| {
            Some(Step {
                prompt: None,
                effects: vec![
                    Effect::SaveLocation(kind, Some(100000000)),
                    Effect::WarpToPortal(map, portal),
                ],
            })
        };
        assert_eq!(pick(1), went(DOJO_RETURN, events::DOJO_ENTRANCE, 0));
        assert_eq!(pick(2), went(CARNIVAL_RETURN, carnival::CARNIVAL_LOBBY, 3));
        assert_eq!(pick(5), went(PYRAMID_RETURN, events::PYRAMID_LOBBY, 4));
        // Nothing else is on the menu, the nowhere entry included.
        for id in [-1, 0, 3, 6] {
            assert_eq!(pick(id), None);
        }
        let prompt = Prompt::Slide {
            npc_id: DIMENSIONAL_MIRROR,
            set: 0,
            menu: String::new(),
        };
        assert_eq!(
            input(&prompt, 14, 1, Some(5), None),
            Some(ScriptInput::Select(5))
        );
        assert_eq!(input(&prompt, 14, 0, None, None), None);
        assert_eq!(input(&prompt, 4, 1, Some(5), None), None);
    }

    #[test]
    fn romeo_and_juliet_take_their_sides_in_and_let_a_member_out() {
        for (side, host, menu, info, retreat) in [
            (
                MagatiaSide::Zenumist,
                ROMEO,
                keys::MAGATIA_ROMEO_MENU,
                keys::MAGATIA_ROMEO_INFO,
                keys::MAGATIA_ROMEO_RETREAT,
            ),
            (
                MagatiaSide::Alcadno,
                JULIET,
                keys::MAGATIA_JULIET_MENU,
                keys::MAGATIA_JULIET_INFO,
                keys::MAGATIA_JULIET_RETREAT,
            ),
        ] {
            let started = EventKind::Magatia(side);
            let event = started.event();
            assert_eq!(magatia_host(side), host);
            assert_eq!(magatia_side(host), side);
            let mut ready = NpcScript::magatia(side, Entry::Ready);
            assert_eq!(ready.npc_id, host);
            let menu_page = ready.begin();
            assert_eq!(kind(&menu_page), Some(4));
            assert!(format!("{menu_page:?}").contains(&text(menu)));
            let step = ready.respond(ScriptInput::Select(0)).unwrap();
            assert_eq!(step.effects, [Effect::StartEvent(started)]);
            assert!(ready.refused().is_some());
            for (entry, key) in [
                (Entry::NoParty, keys::MAGATIA_NO_PARTY),
                (Entry::NotLeader, keys::MAGATIA_NOT_LEADER),
                (Entry::Busy, keys::MAGATIA_BUSY),
            ] {
                let mut asked = NpcScript::magatia(side, entry);
                asked.begin();
                let step = asked.respond(ScriptInput::Select(0)).unwrap();
                assert_eq!(page_text(&step), text(key));
            }
            let mut asked = NpcScript::magatia(side, Entry::NotEligible);
            asked.begin();
            let step = asked.respond(ScriptInput::Select(0)).unwrap();
            assert!(page_text(&step).contains("85"), "{step:?}");
            let mut asked = NpcScript::magatia(side, Entry::Ready);
            asked.begin();
            let step = asked.respond(ScriptInput::Select(1)).unwrap();
            assert_eq!(page_text(&step), text(info));

            // Inside, a member may retreat to the exit; in the lobby or on
            // the exit there is nothing more to say.
            let mut member = player(75);
            member.stats.map_id = event.entry;
            let mut inside = NpcScript::for_npc(host, &member).unwrap();
            assert_eq!(question(&inside.begin()), text(retreat));
            let step = inside.respond(ScriptInput::Yes).unwrap();
            assert_eq!(step.effects, [Effect::WarpToPortal(event.exit, 0)]);
            for map in [event.lobby, event.exit] {
                member.stats.map_id = map;
                assert!(NpcScript::for_npc(host, &member).is_none(), "{map}");
            }
        }
    }

    #[test]
    fn a_magatia_book_gives_what_it_held_and_the_switch_clears_the_stage() {
        for (npc, side, letter) in [
            (ZENUMIST_BOOK, MagatiaSide::Zenumist, 4_001_131),
            (ALCADNO_BOOK, MagatiaSide::Alcadno, 4_001_130),
        ] {
            assert_eq!(magatia_book_side(npc), Some(side));
            let found = |find| NpcScript::magatia_book(npc, find).begin();
            let empty = found(None);
            assert_eq!(page_text(&empty), text(keys::MAGATIA_BOOK_NOTHING));
            assert!(empty.effects.is_empty());
            let mesos = found(Some(Find::Mesos));
            assert!(page_text(&mesos).contains("500"), "{mesos:?}");
            assert_eq!(mesos.effects, [Effect::GainMeso(500)]);
            let exp = found(Some(Find::Exp));
            assert!(page_text(&exp).contains("500"), "{exp:?}");
            assert_eq!(exp.effects, [Effect::GainExp(500)]);
            let letter_found = found(Some(Find::Letter));
            assert_eq!(page_text(&letter_found), text(keys::MAGATIA_BOOK_LETTER));
            assert_eq!(
                letter_found.effects,
                [Effect::GiveItem {
                    item_id: letter,
                    quantity: 1
                }]
            );
            let mut full = NpcScript::magatia_book(npc, Some(Find::Letter));
            full.begin();
            let step = full.inventory_full().unwrap();
            assert_eq!(page_text(&step), text(keys::MAGATIA_BOOK_LETTER_FULL));
            let pass = found(Some(Find::Pass));
            assert_eq!(page_text(&pass), text(keys::MAGATIA_BOOK_PASS));
            assert_eq!(
                pass.effects,
                [
                    Effect::SetReactorState {
                        name: "d00",
                        state: 1
                    },
                    Effect::EventCommand(EventCommand::ClearStage(1))
                ]
            );
        }
        assert_eq!(magatia_book_side(ROMEO), None);
    }

    #[test]
    fn the_hubs_romeo_and_juliet_read_the_letter_and_take_the_files() {
        for (side, host, letter, hurry, retreat) in [
            (
                MagatiaSide::Zenumist,
                ROMEO_HUB,
                4_001_131,
                keys::MAGATIA_ROMEO_HURRY,
                keys::MAGATIA_ROMEO_RETREAT,
            ),
            (
                MagatiaSide::Alcadno,
                JULIET_HUB,
                4_001_130,
                keys::MAGATIA_JULIET_HURRY,
                keys::MAGATIA_JULIET_RETREAT,
            ),
        ] {
            assert_eq!(magatia_hub_host(side), host);
            let event = EventKind::Magatia(side).event();
            let holding = |items: &[i32]| {
                let mut member = player(50);
                member.stats.map_id = side.stages()[3];
                for (slot, &item_id) in (1..).zip(items) {
                    (member.inventory).insert(
                        InventoryType::Etc,
                        slot,
                        game::Item::stack(item_id, 1),
                    );
                }
                member
            };
            let hub = |member: &Player, leader, cleared, shut| {
                let script = NpcScript::magatia_hub(side, member, leader, cleared, shut);
                assert_eq!(script.npc_id, host);
                script.begin()
            };
            let door = Effect::OpenReactor(side.hub_door());

            // The letter first, for anyone, once it is held.
            let read = hub(
                &holding(&[letter, 4_001_134, 4_001_135]),
                false,
                false,
                false,
            );
            assert_eq!(page_text(&read), text(keys::MAGATIA_LETTER));
            assert_eq!(
                read.effects,
                [
                    Effect::TakeItem {
                        item_id: letter,
                        quantity: 1
                    },
                    Effect::EventCommand(EventCommand::Shocked)
                ]
            );

            // Both files from the leader clear the stage; a member is sent
            // to the leader, and one file is not enough.
            let files = holding(&[4_001_134, 4_001_135]);
            let taken = hub(&files, true, false, false);
            assert_eq!(page_text(&taken), text(keys::MAGATIA_FILES));
            assert_eq!(
                taken.effects,
                [
                    Effect::TakeItem {
                        item_id: 4_001_134,
                        quantity: 1
                    },
                    Effect::TakeItem {
                        item_id: 4_001_135,
                        quantity: 1
                    },
                    Effect::EventCommand(EventCommand::ClearStage(4)),
                    Effect::ClearMobs,
                    door.clone()
                ]
            );
            let member = hub(&files, false, false, false);
            assert_eq!(page_text(&member), text(keys::MAGATIA_FILES_LEADER));
            assert!(member.effects.is_empty());
            for (items, leader) in [(&[4_001_134][..], true), (&[], false)] {
                let mut asked = NpcScript::magatia_hub(side, &holding(items), leader, false, false);
                assert_eq!(question(&asked.begin()), text(retreat));
                let step = asked.respond(ScriptInput::Yes).unwrap();
                assert_eq!(step.effects, [Effect::WarpToPortal(event.exit, 0)]);
            }

            // Once cleared, the way on opens again when it has shut.
            let opened = hub(&holding(&[]), false, true, true);
            assert_eq!(page_text(&opened), text(keys::MAGATIA_OPEN_DOOR));
            assert_eq!(opened.effects, [door]);
            let open = hub(&holding(&[]), false, true, false);
            assert_eq!(page_text(&open), text(hurry));
            assert!(open.effects.is_empty());
        }
    }

    #[test]
    fn after_the_frankenroid_the_party_is_thanked_and_rewarded_on_the_way_home() {
        for side in MagatiaSide::ALL {
            let stages = side.stages();
            let (traces_room, thanks_room) = (side.traces_room(), side.thanks_room());
            assert_eq!(traces_room, stages[0] + 500);
            assert_eq!(thanks_room, stages[0] + 600);
            assert_eq!(
                magatia_waiting(stages[6]),
                Some((yulete(side), events::LAB_YULETE_AT))
            );
            assert_eq!(
                magatia_waiting(traces_room),
                Some((traces(side), events::MAGATIA_THANKS_AT))
            );
            assert_eq!(
                magatia_waiting(thanks_room),
                Some((MAGATIA_THANKS, events::MAGATIA_THANKS_AT))
            );
            assert_eq!(magatia_waiting(stages[5]), None);

            // Saved, Romeo or Juliet gives the gift and sends the member
            // on: past Yulete's traces if he was caught, the mighty one
            // having come out.
            for (caught, next) in [(false, thanks_room), (true, traces_room)] {
                let mut saved = NpcScript::magatia_saved(side, false, caught);
                assert_eq!(saved.npc_id(), magatia_hub_host(side));
                saved.begin();
                let gift = saved.respond(ScriptInput::Next).unwrap();
                assert_eq!(page_text(&gift), text(keys::MAGATIA_GIFT));
                let given = saved.respond(ScriptInput::Next).unwrap();
                assert_eq!(
                    given.effects,
                    [
                        Effect::GiveItem {
                            item_id: side.gift(),
                            quantity: 1
                        },
                        Effect::WarpToPortal(next, 0)
                    ]
                );
                let full = saved.inventory_full().unwrap();
                assert_eq!(page_text(&full), text(keys::MAGATIA_GIFT_FULL));
                assert!(full.effects.is_empty());
            }
            // With the escort fainted, there is no gift.
            let mut hurt = NpcScript::magatia_saved(side, true, true);
            hurt.begin();
            let on = hurt.respond(ScriptInput::Next).unwrap();
            assert_eq!(on.effects, [Effect::WarpToPortal(thanks_room, 0)]);

            let mut traced = NpcScript::yulete_traces(side);
            let asked = traced.begin();
            let asked = format!("{:?}", asked.prompt);
            assert!(
                asked.contains(&text(keys::MAGATIA_TRACES_DEFEATED)),
                "{asked}"
            );
            assert!(asked.contains(&text(keys::MAGATIA_TRACES_CHEER)), "{asked}");
            let sorry = traced.respond(ScriptInput::Select(0)).unwrap();
            assert_eq!(page_text(&sorry), text(keys::MAGATIA_TRACES_SORRY));
            let on = traced.respond(ScriptInput::Next).unwrap();
            assert_eq!(
                on.effects,
                [
                    Effect::Mark(YULETE_LAB_OPENED),
                    Effect::WarpToPortal(thanks_room, 0)
                ]
            );

            // The thanks, then the run marked won, one of the rewards and
            // the way home.
            let lobby = EventKind::Magatia(side).event().lobby;
            let mut rng = rand::thread_rng();
            for (escort_failed, key) in [
                (false, keys::MAGATIA_THANKS_REHAB),
                (true, keys::MAGATIA_THANKS_JAIL),
            ] {
                let mut member = player(50);
                member.stats.map_id = thanks_room;
                let mut thanks = NpcScript::magatia_thanks(side, &member, escort_failed, &mut rng);
                assert_eq!(thanks.npc_id(), MAGATIA_THANKS);
                assert_eq!(page_text(&thanks.begin()), text(key));
                let given = thanks.respond(ScriptInput::Next).unwrap();
                let [Effect::SetRecord {
                    record: MAGATIA_WON,
                    value: "1",
                }, Effect::GiveItem { item_id, quantity }, Effect::Warp(to)] = given.effects[..]
                else {
                    panic!("{given:?}");
                };
                assert!(events::MAGATIA_REWARDS.contains(&(item_id, quantity)));
                assert_eq!(to, lobby);
            }
            // A full Use inventory gets nothing yet.
            let mut member = player(50);
            let slots = member.inventory.slot_limit(InventoryType::Use);
            for slot in 1..=i16::from(slots) {
                (member.inventory).insert(
                    InventoryType::Use,
                    slot,
                    game::Item::stack(2_000_000, 1),
                );
            }
            let mut thanks = NpcScript::magatia_thanks(side, &member, false, &mut rng);
            thanks.begin();
            let full = thanks.respond(ScriptInput::Next).unwrap();
            assert_eq!(page_text(&full), text(keys::MAGATIA_THANKS_FULL));
            assert!(full.effects.is_empty());
        }
    }

    #[test]
    fn guon_takes_the_party_aboard_and_lets_a_member_off() {
        let mut ready = NpcScript::guon(Entry::Ready);
        assert_eq!(kind(&ready.begin()), Some(4));
        let step = ready.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(step.effects, [Effect::StartEvent(EventKind::Pirate)]);
        assert!(ready.refused().is_some());
        for (entry, key) in [
            (Entry::NoParty, keys::PIRATE_NO_PARTY),
            (Entry::NotLeader, keys::PIRATE_NOT_LEADER),
            (Entry::Busy, keys::PIRATE_BUSY),
        ] {
            let mut guon = NpcScript::guon(entry);
            guon.begin();
            let step = guon.respond(ScriptInput::Select(0)).unwrap();
            assert_eq!(page_text(&step), text(key));
        }
        let mut guon = NpcScript::guon(Entry::NotEligible);
        guon.begin();
        let step = guon.respond(ScriptInput::Select(0)).unwrap();
        assert!(page_text(&step).contains("100"), "{step:?}");
        let mut guon = NpcScript::guon(Entry::Ready);
        guon.begin();
        let step = guon.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(page_text(&step), text(keys::PIRATE_INFO));

        // Aboard, a member may go back; the leader hears what the map asks.
        let mut member = player(60);
        member.stats.map_id = events::PIRATE_STAGES[2];
        let mut aboard = NpcScript::guon_aboard(&member, false, 0, false);
        assert_eq!(question(&aboard.begin()), text(keys::PIRATE_GUON_LEAVE));
        let step = aboard.respond(ScriptInput::Yes).unwrap();
        assert_eq!(
            step.effects,
            [Effect::WarpToPortal(events::PIRATE_LOBBY, 0)]
        );
        for (map, key) in [
            (events::PIRATE_STAGES[0], keys::PIRATE_GUON_BOXES),
            (events::PIRATE_STAGES[1], keys::PIRATE_GUON_MARKS),
            (events::PIRATE_STAGES[3], keys::PIRATE_GUON_DECK),
            (925_100_201, keys::PIRATE_GUON_BONUS),
            (925_100_302, keys::PIRATE_GUON_CREW),
            (events::PIRATE_STAGES[4], keys::PIRATE_GUON_DOORS),
            (events::PIRATE_STAGES[5], keys::PIRATE_GUON_BOSS),
        ] {
            member.stats.map_id = map;
            let aboard = NpcScript::guon_aboard(&member, true, 0, false);
            assert_eq!(page_text(&aboard.begin()), text(key), "{map}");
        }
        // Beaten bare, a bellflower room shows its chest; a deck doesn't.
        for (map, name) in [(925_100_201, "treasure1"), (925_100_301, "treasure2")] {
            member.stats.map_id = map;
            let step = NpcScript::guon_aboard(&member, true, 0, true).begin();
            assert_eq!(page_text(&step), text(keys::PIRATE_GUON_CHEST), "{map}");
            assert_eq!(step.effects, [Effect::SetReactorState { name, state: 1 }]);
            let member_asked = NpcScript::guon_aboard(&member, false, 0, true).begin();
            assert!(member_asked.effects.is_empty());
        }
        member.stats.map_id = events::PIRATE_STAGES[2];
        let deck = NpcScript::guon_aboard(&member, true, 0, true).begin();
        assert_eq!(page_text(&deck), text(keys::PIRATE_GUON_DECK));
        // On the way out, he sends anyone back at once.
        assert!(NpcScript::for_npc(GUON_ABOARD, &member).is_none());
        member.stats.map_id = events::PIRATE_EXIT;
        let out = NpcScript::for_npc(GUON_ABOARD, &member).unwrap();
        let step = out.begin();
        assert_eq!(
            step.effects,
            [Effect::WarpToPortal(events::PIRATE_LOBBY, 0)]
        );
    }

    #[test]
    fn wu_yang_wins_the_ship_for_its_leader_and_makes_hats_after() {
        use events::{HPQ_REWARD, PIRATE_HATS};
        // Freed, he sends a member to the leader, and the leader's hearing
        // him out wins the run.
        let mut member = player(60);
        member.stats.map_id = events::PIRATE_STAGES[5];
        let mut freed = NpcScript::in_event(WU_YANG, &member, false, false).unwrap();
        let step = freed.begin();
        assert_eq!(page_text(&step), text(keys::PIRATE_WU_WAIT));
        assert!(freed.respond(ScriptInput::Next).unwrap().effects.is_empty());
        let mut freed = NpcScript::in_event(WU_YANG, &member, true, false).unwrap();
        assert_eq!(page_text(&freed.begin()), text(keys::PIRATE_WU_THANKS));
        let step = freed.respond(ScriptInput::Next).unwrap();
        assert_eq!(
            step.effects,
            [Effect::EventCommand(EventCommand::ClearStage(6))]
        );
        assert!(NpcScript::in_event(WU_YANG, &member, true, true).is_none());
        // Guon, with the ship bare, thanks the leader.
        let guon = NpcScript::guon_aboard(&member, true, 0, true).begin();
        assert_eq!(page_text(&guon), text(keys::PIRATE_GUON_SAVED));

        // In the clear room: a feather and the way home.
        member.stats.map_id = events::PIRATE_CLEAR;
        assert!(NpcScript::in_event(WU_YANG, &member, true, false).is_none());
        let choose = |player: &Player, choice| {
            let mut script = NpcScript::in_event(WU_YANG, player, false, true).unwrap();
            assert_eq!(kind(&script.begin()), Some(4));
            script.respond(ScriptInput::Select(choice)).unwrap()
        };
        assert_eq!(
            choose(&member, 0).effects,
            [
                Effect::GiveItem {
                    item_id: HPQ_REWARD,
                    quantity: 1
                },
                Effect::WarpToPortal(events::PIRATE_LOBBY, 0)
            ]
        );
        // Too few feathers for a hat.
        let need = choose(&member, 1);
        assert!(need.effects.is_empty());
        assert!(page_text(&need).contains("20"), "{need:?}");
        // Twenty make the first hat, and twenty more and it the second.
        let feathers = game::Item::stack(HPQ_REWARD, 20);
        member.inventory.insert(InventoryType::Etc, 1, feathers);
        let take = |item_id, quantity| Effect::TakeItem { item_id, quantity };
        let give = |item_id| Effect::GiveItem {
            item_id,
            quantity: 1,
        };
        let first = choose(&member, 1);
        assert_eq!(page_text(&first), text(keys::PIRATE_WU_HAT_GIVEN));
        assert_eq!(first.effects, [take(HPQ_REWARD, 20), give(PIRATE_HATS[0])]);
        let equip = |item_id| game::Item::equipment(item_id, game::EquipStats::default());
        member
            .inventory
            .insert(InventoryType::Equip, 1, equip(PIRATE_HATS[0]));
        assert_eq!(
            choose(&member, 1).effects,
            [
                take(PIRATE_HATS[0], 1),
                take(HPQ_REWARD, 20),
                give(PIRATE_HATS[1])
            ]
        );
        // With the second, there is no better.
        member
            .inventory
            .insert(InventoryType::Equip, 2, equip(PIRATE_HATS[1]));
        let best = choose(&member, 1);
        assert_eq!(page_text(&best), text(keys::PIRATE_WU_BEST));
        assert!(best.effects.is_empty());

        // A full Etc tab still takes a feather on its stack.
        let mut full = player(60);
        full.stats.map_id = events::PIRATE_CLEAR;
        let inventory = &mut full.inventory;
        inventory.set_slot_limit(InventoryType::Etc, 1);
        inventory.insert(InventoryType::Etc, 1, game::Item::stack(HPQ_REWARD, 20));
        assert_eq!(choose(&full, 0).effects.len(), 2);
        // Full tabs: he says so and gives nothing.
        let inventory = &mut full.inventory;
        inventory.insert(InventoryType::Etc, 1, game::Item::stack(4_000_000, 1));
        inventory.set_slot_limit(InventoryType::Equip, 1);
        inventory.insert(InventoryType::Equip, 1, equip(1_002_000));
        let out = choose(&full, 0);
        assert_eq!(page_text(&out), text(keys::PIRATE_WU_ROOM_ETC));
        assert!(out.effects.is_empty());
        let inventory = &mut full.inventory;
        inventory.insert(InventoryType::Etc, 1, game::Item::stack(HPQ_REWARD, 20));
        let hat = choose(&full, 1);
        assert_eq!(page_text(&hat), text(keys::PIRATE_WU_ROOM_EQUIP));
        assert!(hat.effects.is_empty());
    }

    #[test]
    fn guon_takes_twenty_of_each_mark_in_turn_from_the_leader() {
        let mut leader = player(60);
        leader.stats.map_id = events::PIRATE_STAGES[1];
        let wants = [
            keys::PIRATE_GUON_MARKS,
            keys::PIRATE_GUON_RISING,
            keys::PIRATE_GUON_VETERAN,
            keys::PIRATE_GUON_MARKS_IN,
        ];
        // Short of the Mark in turn, he says which he wants; others
        // don't count.
        for (slot, &(mark, _)) in events::PIRATE_MARKS.iter().enumerate() {
            leader.inventory.insert(
                InventoryType::Etc,
                slot as i16 + 1,
                game::Item::stack(mark, 19),
            );
        }
        for (marks, key) in wants.into_iter().enumerate() {
            let guon = NpcScript::guon_aboard(&leader, true, marks as u8, false);
            let step = guon.begin();
            assert_eq!(page_text(&step), text(key), "{marks}");
            assert!(step.effects.is_empty(), "{marks}");
        }
        // With twenty, he takes them and asks for the next.
        for (slot, &(mark, _)) in events::PIRATE_MARKS.iter().enumerate() {
            leader.inventory.insert(
                InventoryType::Etc,
                slot as i16 + 1,
                game::Item::stack(mark, 20),
            );
        }
        for (n, &(mark, _)) in events::PIRATE_MARKS.iter().enumerate() {
            let step = NpcScript::guon_aboard(&leader, true, n as u8, false).begin();
            assert_eq!(page_text(&step), text(wants[n + 1]), "{n}");
            assert_eq!(
                step.effects,
                [
                    Effect::TakeItem {
                        item_id: mark,
                        quantity: 20
                    },
                    Effect::EventCommand(EventCommand::HandInMarks(n as u8 + 1)),
                ]
            );
        }
        let done = NpcScript::guon_aboard(&leader, true, 3, false).begin();
        assert_eq!(page_text(&done), text(keys::PIRATE_GUON_MARKS_IN));
        assert!(done.effects.is_empty());
        // A member is only asked whether to leave.
        let member = NpcScript::guon_aboard(&leader, false, 0, false);
        assert_eq!(question(&member.begin()), text(keys::PIRATE_GUON_LEAVE));
    }

    #[test]
    fn ellin_trades_ten_fragments_for_each_pair_of_earrings_in_turn() {
        let prize = |player: &Player, choice| {
            let mut ellin = NpcScript::ellin(Entry::Ready, player);
            ellin.begin();
            let menu = ellin.respond(ScriptInput::Select(2)).unwrap();
            assert_eq!(kind(&menu), Some(4));
            ellin.respond(ScriptInput::Select(choice)).unwrap()
        };
        let take = |item_id, quantity| Effect::TakeItem { item_id, quantity };
        let give = |item_id| Effect::GiveItem {
            item_id,
            quantity: 1,
        };
        let mut player = player(50);
        for (choice, key) in [
            (0, keys::ELLIN_PRIZE_NO_FRAGMENTS),
            (1, keys::ELLIN_PRIZE_NO_ALTAIRE),
            (2, keys::ELLIN_PRIZE_NO_GLITTERING),
        ] {
            assert_eq!(page_text(&prize(&player, choice)), text(key));
        }
        player.inventory.insert(
            InventoryType::Etc,
            1,
            game::Item::stack(events::ALTAIRE_FRAGMENT, 10),
        );
        let fragments = take(events::ALTAIRE_FRAGMENT, 10);
        assert_eq!(
            prize(&player, 0).effects,
            [fragments.clone(), give(ALTAIRE_EARRINGS[0])]
        );
        assert_eq!(
            page_text(&prize(&player, 1)),
            text(keys::ELLIN_PRIZE_NO_ALTAIRE)
        );
        // With the first pair in the bag, the next step, but not the first.
        let equip = |item_id| game::Item::equipment(item_id, game::EquipStats::default());
        player
            .inventory
            .insert(InventoryType::Equip, 1, equip(ALTAIRE_EARRINGS[0]));
        assert_eq!(
            page_text(&prize(&player, 0)),
            text(keys::ELLIN_PRIZE_NO_FRAGMENTS)
        );
        assert_eq!(
            prize(&player, 1).effects,
            [
                take(ALTAIRE_EARRINGS[0], 1),
                fragments.clone(),
                give(ALTAIRE_EARRINGS[1])
            ]
        );
        player
            .inventory
            .insert(InventoryType::Equip, 2, equip(ALTAIRE_EARRINGS[1]));
        assert_eq!(
            page_text(&prize(&player, 1)),
            text(keys::ELLIN_PRIZE_NO_ALTAIRE)
        );
        assert_eq!(
            prize(&player, 2).effects,
            [
                take(ALTAIRE_EARRINGS[1], 1),
                fragments,
                give(ALTAIRE_EARRINGS[2])
            ]
        );
        let ellin = NpcScript::ellin(Entry::Ready, &player);
        assert_eq!(ellin.inventory_full, Some(text(keys::ELLIN_PRIZE_NO_ROOM)));
    }

    #[test]
    fn the_sprite_takes_the_leader_with_the_stone_on_to_the_altar() {
        let leave = Effect::WarpToPortal(events::ELLIN_EXIT, 0);
        let mut member = player(50);
        member.stats.map_id = events::ELLIN_STAGES[4];
        // Without the stone, or not leading, a member may only leave.
        for leader in [true, false] {
            let mut sprite = NpcScript::in_event(ELLIN_SPRITE, &member, leader, false).unwrap();
            assert_eq!(question(&sprite.begin()), text(keys::ELLIN_SPRITE_LEAVE));
            let step = sprite.respond(ScriptInput::Yes).unwrap();
            assert_eq!(step.effects, [leave.clone()]);
        }
        member.inventory.insert(
            InventoryType::Etc,
            1,
            game::Item::stack(events::PURPLE_STONE, 1),
        );
        let mut sprite = NpcScript::in_event(ELLIN_SPRITE, &member, false, false).unwrap();
        assert_eq!(question(&sprite.begin()), text(keys::ELLIN_SPRITE_LEAVE));
        // The leader with it is taken on, keeping it.
        let mut sprite = NpcScript::in_event(ELLIN_SPRITE, &member, true, false).unwrap();
        let step = sprite.begin();
        assert_eq!(page_text(&step), text(keys::ELLIN_SPRITE_TO_ALTAR));
        assert!(step.effects.is_empty());
        let step = sprite.respond(ScriptInput::Next).unwrap();
        let on = Effect::EventCommand(EventCommand::MoveTeam(events::ELLIN_STAGES[5]));
        assert_eq!((step.effects, step.prompt), (vec![on], None));
        // Elsewhere it has nothing to say.
        member.stats.map_id = events::ELLIN_STAGES[5];
        assert!(NpcScript::in_event(ELLIN_SPRITE, &member, true, false).is_none());
    }

    #[test]
    fn ellin_gives_out_purification_marbles_and_takes_the_leaders_monster_marbles() {
        use events::{MONSTER_MARBLE, PURIFICATION_MARBLE};
        let counted = |key, n: &str| content::text().get(key, &[("n", n)]);
        let leave = Effect::WarpToPortal(events::ELLIN_EXIT, 0);
        // First she tells a member what to do, then gives them ten marbles.
        let mut member = player(50);
        let mut ellin = NpcScript::ellin_flowers(false, false, 20, &member);
        let step = ellin.begin();
        assert_eq!(page_text(&step), counted(keys::ELLIN_GUIDE_FLOWERS, "20"));
        assert!(step.effects.is_empty());
        let step = ellin.respond(ScriptInput::Next).unwrap();
        assert_eq!(page_text(&step), counted(keys::ELLIN_GUIDE_MARBLES, "10"));
        let give = Effect::GiveItem {
            item_id: PURIFICATION_MARBLE,
            quantity: 10,
        };
        assert_eq!(step.effects, [give]);
        let full = ellin.inventory_full().unwrap();
        assert_eq!(page_text(&full), text(keys::ELLIN_GUIDE_NO_ROOM));
        // Once heard, one with marbles may leave instead.
        let ellin = NpcScript::ellin_flowers(true, true, 20, &member);
        assert_eq!(
            page_text(&ellin.begin()),
            counted(keys::ELLIN_GUIDE_MARBLES, "10")
        );
        member.inventory.insert(
            InventoryType::Use,
            1,
            game::Item::stack(PURIFICATION_MARBLE, 3),
        );
        let mut ellin = NpcScript::ellin_flowers(true, true, 20, &member);
        let ask = counted(keys::ELLIN_GUIDE_FLOWERS_LEAVE, "20");
        assert_eq!(question(&ellin.begin()), ask);
        assert_eq!(
            ellin.respond(ScriptInput::Yes).unwrap().effects,
            [leave.clone()]
        );
        // With every flower caught, she only asks.
        let mut ellin = NpcScript::ellin_flowers(false, true, 0, &player(50));
        let ask = counted(keys::ELLIN_GUIDE_ALL_CAUGHT, "20");
        assert_eq!(question(&ellin.begin()), ask);
        assert_eq!(ellin.respond(ScriptInput::Yes).unwrap().effects, [leave]);
        // Nineteen Monster Marbles aren't enough; twenty are the leader's to
        // hand in.
        member
            .inventory
            .insert(InventoryType::Etc, 1, game::Item::stack(MONSTER_MARBLE, 19));
        let ellin = NpcScript::ellin_flowers(true, true, 0, &member);
        let ask = counted(keys::ELLIN_GUIDE_ALL_CAUGHT, "20");
        assert_eq!(question(&ellin.begin()), ask);
        member
            .inventory
            .insert(InventoryType::Etc, 2, game::Item::stack(MONSTER_MARBLE, 1));
        let ellin = NpcScript::ellin_flowers(false, false, 0, &member);
        assert_eq!(page_text(&ellin.begin()), text(keys::ELLIN_GUIDE_TO_LEADER));
        let mut ellin = NpcScript::ellin_flowers(true, false, 0, &member);
        let step = ellin.begin();
        assert_eq!(page_text(&step), text(keys::ELLIN_GUIDE_HAND_IN));
        assert!(step.effects.is_empty());
        let step = ellin.respond(ScriptInput::Next).unwrap();
        let take = Effect::TakeItem {
            item_id: MONSTER_MARBLE,
            quantity: 20,
        };
        let on = Effect::EventCommand(EventCommand::MoveTeam(events::ELLIN_STAGES[4]));
        assert_eq!((step.effects, step.prompt), (vec![take, on], None));
        let short = ellin.refused().unwrap();
        assert_eq!(page_text(&short), counted(keys::ELLIN_GUIDE_FLOWERS, "20"));
    }

    #[test]
    fn tylus_takes_the_party_in_and_thanks_them_past_the_room() {
        let mut ready = NpcScript::tylus(Entry::Ready);
        assert_eq!(kind(&ready.begin()), Some(4));
        let step = ready.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(step.effects, [Effect::StartEvent(EventKind::Tylus)]);
        assert!(ready.refused().is_some());
        for (entry, key) in [
            (Entry::NoParty, keys::TYLUS_NO_PARTY),
            (Entry::NotLeader, keys::TYLUS_NOT_LEADER),
            (Entry::Busy, keys::TYLUS_BUSY),
        ] {
            let mut tylus = NpcScript::tylus(entry);
            tylus.begin();
            let step = tylus.respond(ScriptInput::Select(0)).unwrap();
            assert_eq!(page_text(&step), text(key));
        }
        let mut tylus = NpcScript::tylus(Entry::NotEligible);
        tylus.begin();
        let step = tylus.respond(ScriptInput::Select(0)).unwrap();
        assert!(page_text(&step).contains("80"), "{step:?}");
        let mut tylus = NpcScript::tylus(Entry::Ready);
        tylus.begin();
        let step = tylus.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(page_text(&step), text(keys::TYLUS_INFO));

        // Past the room, he thanks them and sends them to El Nath.
        let mut warrior = player(50);
        assert!(NpcScript::for_npc(TYLUS_GRATEFUL, &warrior).is_none());
        warrior.stats.map_id = events::TYLUS_THANKS;
        let mut thanks = NpcScript::for_npc(TYLUS_GRATEFUL, &warrior).unwrap();
        assert_eq!(page_text(&thanks.begin()), text(keys::TYLUS_THANKS));
        let step = thanks.respond(ScriptInput::Next).unwrap();
        assert_eq!(
            step.effects,
            [Effect::WarpToPortal(EL_NATH, EL_NATH_FROM_RESIDENCE)]
        );
    }

    #[test]
    fn rolly_takes_the_party_into_the_maze_and_everyone_home() {
        let mut ready = NpcScript::rolly(Entry::Ready);
        assert_eq!(kind(&ready.begin()), Some(4));
        let step = ready.respond(ScriptInput::Select(0)).unwrap();
        assert_eq!(step.effects, [Effect::StartEvent(EventKind::LudiMaze)]);
        assert!(ready.refused().is_some());
        for (entry, key) in [
            (Entry::NoParty, keys::LUDIMAZE_NO_PARTY),
            (Entry::NotLeader, keys::LUDIMAZE_NOT_LEADER),
            (Entry::Busy, keys::LUDIMAZE_BUSY),
        ] {
            let mut rolly = NpcScript::rolly(entry);
            rolly.begin();
            let step = rolly.respond(ScriptInput::Select(0)).unwrap();
            assert_eq!(page_text(&step), text(key));
        }
        let mut rolly = NpcScript::rolly(Entry::NotEligible);
        rolly.begin();
        let step = rolly.respond(ScriptInput::Select(0)).unwrap();
        assert!(page_text(&step).contains("51 to 70"), "{step:?}");
        let mut rolly = NpcScript::rolly(Entry::Ready);
        rolly.begin();
        let step = rolly.respond(ScriptInput::Select(1)).unwrap();
        assert_eq!(page_text(&step), text(keys::LUDIMAZE_INFO));

        // On the way out Rolly sends everyone back to Ludibrium; anywhere
        // else he has only his own lines.
        let mut out = player(50);
        assert!(NpcScript::for_npc(ROLLY_HOME, &out).is_none());
        out.stats.map_id = events::MAZE_EXIT;
        let mut home = NpcScript::for_npc(ROLLY_HOME, &out).unwrap();
        assert_eq!(page_text(&home.begin()), text(keys::LUDIMAZE_HOME));
        let step = home.respond(ScriptInput::Next).unwrap();
        assert_eq!(step.effects, [Effect::WarpToPortal(events::LUDIBRIUM, 0)]);
    }

    #[test]
    fn pietri_takes_all_of_the_leaders_coupons_from_thirty_with_everyone_there() {
        let coupons = |n| hunter(&[(events::MAZE_COUPON, n)], &[]);
        let says = |leader, won, together, n| {
            page_text(&NpcScript::pietri(leader, won, together, &coupons(n)).begin())
        };
        assert_eq!(
            says(false, false, true, 40),
            text(keys::LUDIMAZE_PIETRI_NOT_LEADER)
        );
        assert_eq!(
            says(true, false, false, 40),
            text(keys::LUDIMAZE_PIETRI_WAIT)
        );
        assert_eq!(says(true, true, true, 40), text(keys::LUDIMAZE_PIETRI_DONE));
        assert!(says(true, false, true, 29).contains("30"));
        let mut pietri = NpcScript::pietri(true, false, true, &coupons(42));
        let ask = pietri.begin();
        assert_eq!(kind(&ask), Some(1));
        assert!(question(&ask).contains("42") && question(&ask).contains("2100"));
        assert_eq!(
            pietri.respond(ScriptInput::Yes).unwrap().effects,
            [
                Effect::TakeItem {
                    item_id: events::MAZE_COUPON,
                    quantity: 42
                },
                Effect::EventCommand(EventCommand::WinWithCoupons(42))
            ]
        );
        assert!(page_text(&pietri.refused().unwrap()).contains("30"));
    }

    #[test]
    fn rolly_gives_each_winner_a_present_with_room_for_any_of_them() {
        let mut player = rider(100, 0, events::MAZE_CLEAR);
        use rand::SeedableRng;
        let rng = &mut rand::rngs::StdRng::seed_from_u64(7);
        let mut script = NpcScript::in_event(ROLLY_PRESENT, &player, false, true).unwrap();
        assert_eq!(kind(&script.begin()), Some(1));
        let effects = script.respond(ScriptInput::Yes).unwrap().effects;
        let [Effect::GiveItem { item_id, quantity }, Effect::Warp(out)] = effects[..] else {
            panic!("{effects:?}");
        };
        assert!(events::MAZE_REWARDS.contains(&(item_id, quantity)));
        assert_eq!(out, events::MAZE_EXIT);
        // Should the give fail all the same, he says so and keeps them.
        let refused = script.inventory_full().unwrap();
        assert!(page_text(&refused).contains(&text(keys::LUDIMAZE_PRESENT_FULL)));
        assert!(NpcScript::in_event(ROLLY_PRESENT, &player, false, false).is_none());
        let goal = rider(100, 0, events::MAZE_GOAL);
        assert!(maze_present(&goal, rng).is_none());

        // A full Etc tab: nothing yet, whatever the present would be.
        let inventory = &mut player.inventory;
        inventory.set_slot_limit(InventoryType::Etc, 1);
        inventory.insert(InventoryType::Etc, 1, game::Item::stack(4000000, 1));
        let mut script = maze_present(&player, rng).unwrap();
        script.begin();
        let full = script.respond(ScriptInput::Yes).unwrap();
        assert!(full.effects.is_empty());
        assert_eq!(page_text(&full), text(keys::LUDIMAZE_PRESENT_FULL));
    }
}
