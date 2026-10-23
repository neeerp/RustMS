//! Native NPC and quest scripts. Each is a list of pages with effects on its
//! answers; the runtime applies the effects (and persists them with any
//! quest change) before showing the next page. Text comes from the content
//! pack (`crate::content`), with committed placeholders. Lua is deferred;
//! these keep the effect-returning shape a scripting runtime would have.

use crate::content;
use crate::conti::{self, Route, State};
use crate::dialogue::{DialoguePage, Prompt};
use crate::events::{
    self, Balloon, Cloto, Entry, EventCommand, EventKind, HandIn, HeadCount, LeverCheck, Standing,
    Statue,
};
use crate::gachapon::{self, Prize};
use crate::items;
use crate::rides;
use crate::scripts::Effect;
use game::alliance::{self, Alliance};
use game::character::{FLORINA_RETURN, WORLD_TOUR_RETURN};
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
/// Heracle, at the guild headquarters. The runtime loads the player's
/// guild for his script (`NpcScript::heracle`).
pub const HERACLE: i32 = 2010007;
/// Lea, beside him, who makes guild emblems; her script needs the guild
/// too (`NpcScript::lea`).
pub const LEA: i32 = 2010008;
/// Lenario, beside Heracle, who founds alliances; his script needs the
/// guild, its alliance, and the party (`NpcScript::lenario`).
pub const LENARIO: i32 = 2010009;
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
/// offers Kerning Square and the Construction Site, left out until those
/// run) and New Leaf City's (`NLC_Move`).
const SUBWAY_GATES: [(i32, &Route); 2] = [
    (1052007, &conti::TO_NLC),
    (9201068, &conti::TO_KERNING_CITY),
];
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
/// Moony has sent the player for their parents' blessing.
const BLESSING_SOUGHT: i16 = 32300;
/// Mom and Dad have asked for the Proofs of Love.
const PROOFS_ASKED: i16 = 32301;
/// Mom and Dad have given their blessing; Moony makes rings.
const BLESSED: i16 = 32302;
/// A Nana's favour, by her place in `NANA_TASKS`: asked, and done once.
const NANA_ASKED: i16 = 32310;
const NANA_DONE: i16 = 32320;
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptInput {
    Next,
    Previous,
    Yes,
    No,
    Select(i32),
    Text(String),
}

/// What a script does with a line of text the player typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnText {
    /// Lenario confirms the name for the new alliance.
    NameAlliance,
}

impl OnText {
    /// The pages that follow the text.
    fn pages(self, text: &str) -> Vec<Page> {
        match self {
            OnText::NameAlliance => confirm_alliance_name(text),
        }
    }
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
    /// A line of text from `min` to `max` bytes long (`Prompt::AskText`);
    /// what `on_text` makes of it replaces the rest of the script.
    AskText {
        text: String,
        min: i16,
        max: i16,
        on_text: OnText,
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
    /// Ludibrium, Minerva's in the Tower of Goddess.
    pub fn in_event(npc_id: i32, player: &Player, leader: bool, won: bool) -> Option<Self> {
        match npc_id {
            GROWLIE => growlie(player, leader, won),
            TOMMY => tommy(player, leader, won),
            TORY => tory_after(player, won),
            PINK_BALLOON => pink_balloon(player),
            ARTURO if won => arturo(player, &mut rand::thread_rng()),
            MINERVA if won => minerva(player, &mut rand::thread_rng()),
            _ => None,
        }
    }

    /// Eak (Cosmic `2013001`) to a player in the Orbis party quest on
    /// `map_id`, who may be its `leader`, whose team may have `cleared` the
    /// cloud stage. None elsewhere.
    pub fn opq_eak(map_id: i32, leader: bool, cleared: bool) -> Option<Self> {
        opq_eak(map_id, leader, cleared)
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
            // A prize may go to any bag. Rolling with one full would turn
            // some prizes away with the ticket kept: a free roll, which
            // also tells which bag the prize was for.
            if gachapon::BAGS.iter().any(|&bag| inventory.free(bag) == 0) {
                return Some(NpcScript::new(
                    npc_id,
                    vec![Page::Say(text(keys::GACHAPON_FULL))],
                ));
            }
            let prize = gachapon::roll(npc_id, &mut gachapon::rng());
            return Some(gachapon_machine(npc_id, town, prize));
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
            SPINEL => Some(spinel(player)),
            PISON => Some(pison(player)),
            THOMAS_SWIFT => thomas_swift(player),
            MOONY => Some(moony(player)),
            MOM_AND_DAD => mom_and_dad(player),
            id if NANAS.contains(&id) => nana(id, player),
            AUDREY => audrey(player),
            CRANE => crane(player),
            NATALIE => Some(natalie(player)),
            NELLA => nella(player),
            ANDERSON => anderson(player),
            WONKY => wonky_inside(player),
            EAK => eak(player),
            GROWLIE => growlie(player, false, false),
            TOMMY => tommy(player, false, false),
            TORY => tory_after(player, false),
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
            "q6030e" => Some(maker_class(CARSON, keys::MAKER_ALCHEMY, 6030)),
            "q6031e" => Some(maker_class(HUGHES, keys::MAKER_SCIENCE, 6031)),
            "q6032e" => Some(maker_class(MEREN, keys::MAKER_PRACTICE, 6032)),
            "q6033e" => Some(maker_lesson(player, keys::MAKER_CRYSTAL_TEST, 6033)),
            "q6036e" => Some(maker_lesson(player, keys::MAKER_GOLD_ANVIL, 6036)),
            "q4647e" => Some(pet_secret(player)),
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
            (Page::AskText { on_text, .. }, ScriptInput::Text(text)) => {
                self.pages = on_text.pages(&text);
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
            Page::AskText { text, min, max, .. } => Prompt::AskText {
                npc_id,
                text: text.clone(),
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
    // A yes that comes after boarding closed (`RequireBoarding`).
    let mut script = NpcScript::new(npc_id, gate(route, &tickets, player, t, &says));
    script.refused = Some(text(says.closing));
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
            ("least", &team.start().to_string()),
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
    is_clone_room(map_id).then_some(CLONE_TIME_LIMIT)
}

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
    text: Option<String>,
) -> Option<ScriptInput> {
    match (prompt, message_type, response) {
        (Prompt::Text(_), 0, 1) => Some(ScriptInput::Next),
        (Prompt::Text(page), 0, 0) if page.has_previous => Some(ScriptInput::Previous),
        (Prompt::YesNo { .. }, 1, 1) => Some(ScriptInput::Yes),
        (Prompt::YesNo { .. }, 1, 0) => Some(ScriptInput::No),
        (Prompt::Selection { .. }, 4, 1) => selection.map(ScriptInput::Select),
        (Prompt::Style { .. }, 7, 1) => selection.map(ScriptInput::Select),
        (Prompt::AskText { .. }, 2, 1) => text.map(ScriptInput::Text),
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
                NpcScript::for_quest(script, &player(50)).is_some(),
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
            status: QuestStatus::Started { kills: Vec::new() },
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
            let gate = subway_gate(1052007, &conti::TO_NLC, &holder, t);
            assert_eq!(buttons(&gate.begin()), (false, false));
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

    fn engaging(markers: &[i16], gender: u8, meso: i32, items: &[(i32, i16)]) -> Player {
        let mut player = traveller(30, meso, false);
        player.stats.gender = gender;
        for &quest_id in markers {
            let status = QuestStatus::Started { kills: Vec::new() };
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
}
