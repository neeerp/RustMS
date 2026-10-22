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
    pub const HERACLE_MENU: &str = "heracle.menu";
    pub const HERACLE_CREATE: &str = "heracle.choice.create";
    pub const HERACLE_DISBAND: &str = "heracle.choice.disband";
    pub const HERACLE_GROW: &str = "heracle.choice.grow";
    pub const HERACLE_DISBAND_NOT_MASTER: &str = "heracle.disband.not_master";
    pub const HERACLE_DISBAND_ASK: &str = "heracle.disband.ask";
    pub const HERACLE_GROW_NOT_MASTER: &str = "heracle.grow.not_master";
    pub const HERACLE_GROW_FULL: &str = "heracle.grow.full";
    /// `{price}` and the `{capacity}` it grows to.
    pub const HERACLE_GROW_ASK: &str = "heracle.grow.ask";
    /// `{price}` and `{capacity}`, as for the question.
    pub const HERACLE_GROW_NO_MESOS: &str = "heracle.grow.no_mesos";

    /// Lea, who makes guild emblems beside Heracle; no licensed script has
    /// her text either.
    pub const LEA_MENU: &str = "lea.menu";
    pub const LEA_EMBLEM: &str = "lea.choice.emblem";
    pub const LEA_NOT_MASTER: &str = "lea.not_master";
    /// `{price}`.
    pub const LEA_ASK: &str = "lea.ask";
    pub const LEA_DECLINE: &str = "lea.decline";
    /// The popup for an emblem sent without its `{price}`.
    pub const LEA_NO_MESOS: &str = "lea.no_mesos";

    /// Lenario, who founds alliances beside Heracle, and the pink lines
    /// alliance members read; no licensed script has their text.
    pub const LENARIO_NOT_MASTER: &str = "lenario.not_master";
    pub const LENARIO_MENU: &str = "lenario.menu";
    pub const LENARIO_INFO: &str = "lenario.choice.info";
    pub const LENARIO_HOW: &str = "lenario.choice.how";
    pub const LENARIO_CREATE: &str = "lenario.choice.create";
    pub const LENARIO_GROW: &str = "lenario.choice.grow";
    pub const LENARIO_DISBAND: &str = "lenario.choice.disband";
    pub const LENARIO_INFO_SAY: &str = "lenario.info";
    pub const LENARIO_HOW_SAY: &str = "lenario.how";
    pub const LENARIO_IN_ALLIANCE: &str = "lenario.in_alliance";
    pub const LENARIO_NOT_LEADER: &str = "lenario.not_leader";
    /// `{price}`.
    pub const LENARIO_CREATE_ASK: &str = "lenario.create.ask";
    /// `{price}`.
    pub const LENARIO_CREATE_NO_MESOS: &str = "lenario.create.no_mesos";
    pub const LENARIO_NAME: &str = "lenario.name";
    pub const LENARIO_NAME_AGAIN: &str = "lenario.name.again";
    /// The `{name}` typed.
    pub const LENARIO_NAME_CONFIRM: &str = "lenario.name.confirm";
    pub const LENARIO_CREATE_PARTY: &str = "lenario.create.party";
    /// The new alliance's `{name}`.
    pub const LENARIO_CREATED: &str = "lenario.created";
    pub const LENARIO_GROW_NOT_MASTER: &str = "lenario.grow.not_master";
    pub const LENARIO_GROW_FULL: &str = "lenario.grow.full";
    /// `{price}` and the `{capacity}` it grows to.
    pub const LENARIO_GROW_ASK: &str = "lenario.grow.ask";
    /// `{price}` and `{capacity}`, as for the question.
    pub const LENARIO_GROW_NO_MESOS: &str = "lenario.grow.no_mesos";
    /// `{capacity}`.
    pub const LENARIO_GROWN: &str = "lenario.grown";
    pub const LENARIO_DISBAND_NOT_MASTER: &str = "lenario.disband.not_master";
    pub const LENARIO_DISBAND_ASK: &str = "lenario.disband.ask";
    pub const LENARIO_DISBANDED: &str = "lenario.disbanded";
    pub const LENARIO_DECLINE: &str = "lenario.decline";
    pub const LENARIO_FAILED: &str = "lenario.failed";
    /// `{guild}`, as typed.
    pub const ALLIANCE_NO_GUILD: &str = "alliance.invite.no_guild";
    /// `{guild}`.
    pub const ALLIANCE_IN_ALLIANCE: &str = "alliance.invite.in_alliance";
    /// `{guild}`.
    pub const ALLIANCE_OFFLINE: &str = "alliance.invite.offline";
    /// `{guild}`.
    pub const ALLIANCE_BUSY: &str = "alliance.invite.busy";
    pub const ALLIANCE_FULL: &str = "alliance.invite.full";
    /// `{guild}`.
    pub const ALLIANCE_DENIED: &str = "alliance.invite.denied";
    /// `{guild}`.
    pub const ALLIANCE_LEFT: &str = "alliance.left";
    /// `{guild}`.
    pub const ALLIANCE_EXPELLED: &str = "alliance.expelled";
    /// The new master's `{name}`.
    pub const ALLIANCE_MASTER: &str = "alliance.master";
    /// `{name}` and their new `{title}`.
    pub const ALLIANCE_RANK: &str = "alliance.rank";

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

    /// The Regular Cabs (`taxi1`-`taxi6`): a greeting, a menu of the other
    /// towns with their fares (opened, for a beginner, by the beginners'
    /// discount), a yes or no, and the lines for no and for too few mesos.
    pub const TAXI_INTRO: &str = "taxi.intro";
    pub const TAXI_DISCOUNT: &str = "taxi.discount";
    pub const TAXI_MENU: &str = "taxi.menu";
    pub const TAXI_CHOICE: &str = "taxi.choice";
    pub const TAXI_ASK: &str = "taxi.ask";
    pub const TAXI_DECLINE: &str = "taxi.decline";
    pub const TAXI_SHORT: &str = "taxi.short";

    /// The VIP Cabs (`mTaxi`) to Ant Tunnel Park; a beginner's yes or no
    /// opens with the Regular Cabs' discount line.
    pub const VIP_CAB_INTRO: &str = "vip_cab.intro";
    pub const VIP_CAB_ASK: &str = "vip_cab.ask";
    pub const VIP_CAB_DECLINE: &str = "vip_cab.decline";
    pub const VIP_CAB_SHORT: &str = "vip_cab.short";

    /// Ossyria's Danger Zone Taxi (`ossyria_taxi`).
    pub const DANGER_ZONE_TAXI_INTRO: &str = "danger_zone_taxi.intro";
    pub const DANGER_ZONE_TAXI_ASK: &str = "danger_zone_taxi.ask";
    pub const DANGER_ZONE_TAXI_DECLINE: &str = "danger_zone_taxi.decline";
    pub const DANGER_ZONE_TAXI_SHORT: &str = "danger_zone_taxi.short";

    /// The Camel Cab between Ariant and Magatia (`nihal_taxi`).
    pub const CAMEL_CAB_ASK: &str = "camel_cab.ask";
    pub const CAMEL_CAB_DECLINE: &str = "camel_cab.decline";
    pub const CAMEL_CAB_SHORT: &str = "camel_cab.short";

    /// The subway between Kerning City and New Leaf City: the ticket gates
    /// (`NLC_Move`), and Bell (`NLC_ticketing`), who sells tickets at each
    /// station (`to_nlc`, `to_kerning_city`: the menu, its one choice, and
    /// the ask with `{term}`, `{price}` and `{ticket}`) and lets riders back
    /// out of the waiting rooms.
    pub const SUBWAY_BOARD_ASK: &str = "subway.board.ask";
    pub const SUBWAY_BOARD_DECLINE: &str = "subway.board.decline";
    pub const SUBWAY_BOARD_NO_TICKET: &str = "subway.board.no_ticket";
    pub const SUBWAY_BOARD_CLOSING: &str = "subway.board.closing";
    pub const SUBWAY_BOARD_FULL: &str = "subway.board.full";
    pub const SUBWAY_TO_NLC_MENU: &str = "subway.to_nlc.menu";
    pub const SUBWAY_TO_NLC_CHOICE: &str = "subway.to_nlc.choice";
    pub const SUBWAY_TO_NLC_ASK: &str = "subway.to_nlc.ask";
    pub const SUBWAY_TO_NLC_LEAVE: &str = "subway.to_nlc.leave";
    pub const SUBWAY_TO_KERNING_CITY_MENU: &str = "subway.to_kerning_city.menu";
    pub const SUBWAY_TO_KERNING_CITY_CHOICE: &str = "subway.to_kerning_city.choice";
    pub const SUBWAY_TO_KERNING_CITY_ASK: &str = "subway.to_kerning_city.ask";
    pub const SUBWAY_TO_KERNING_CITY_LEAVE: &str = "subway.to_kerning_city.leave";
    pub const SUBWAY_SELL_DECLINE: &str = "subway.sell.decline";
    pub const SUBWAY_SELL_SHORT: &str = "subway.sell.short";
    pub const SUBWAY_LEAVE_DECLINE: &str = "subway.leave.decline";
    pub const PLANE_BOOTH_MENU: &str = "plane.booth.menu";
    pub const PLANE_BOOTH_BUY: &str = "plane.booth.buy";
    pub const PLANE_BOOTH_BOARD: &str = "plane.booth.board";
    pub const PLANE_SELL_ASK: &str = "plane.sell.ask";
    pub const PLANE_SELL_DECLINE: &str = "plane.sell.decline";
    pub const PLANE_SELL_SHORT: &str = "plane.sell.short";
    pub const PLANE_SELL_FULL: &str = "plane.sell.full";
    pub const PLANE_BOARD_ASK: &str = "plane.board.ask";
    pub const PLANE_BOARD_DECLINE: &str = "plane.board.decline";
    pub const PLANE_BOARD_NO_TICKET: &str = "plane.board.no_ticket";
    pub const PLANE_BOARD_CLOSED: &str = "plane.board.closed";
    pub const PLANE_LEAVE_ASK: &str = "plane.leave.ask";
    pub const PLANE_LEAVE_DECLINE: &str = "plane.leave.decline";
    pub const PLANE_RIDE: &str = "plane.ride";

    /// Spinel's world tour, the Florina Beach guides and Pison, and Thomas
    /// Swift between Henesys and Amoria.
    pub const WORLD_TRIP_INTRO: &str = "world_trip.intro";
    pub const WORLD_TRIP_OFFER: &str = "world_trip.offer";
    pub const WORLD_TRIP_CHOICE: &str = "world_trip.choice";
    pub const WORLD_TRIP_DESCRIBE: &str = "world_trip.describe";
    pub const WORLD_TRIP_MORE: &str = "world_trip.more";
    pub const WORLD_TRIP_SHORT: &str = "world_trip.short";
    pub const WORLD_TRIP_BACK_MENU: &str = "world_trip.back.menu";
    pub const WORLD_TRIP_BACK_YES: &str = "world_trip.back.yes";
    pub const WORLD_TRIP_BACK_NO: &str = "world_trip.back.no";
    pub const WORLD_TRIP_BACK_GO: &str = "world_trip.back.go";
    pub const WORLD_TRIP_BACK_STAY: &str = "world_trip.back.stay";
    pub const FLORINA_MENU: &str = "florina.menu";
    pub const FLORINA_PAY: &str = "florina.pay";
    pub const FLORINA_TICKET: &str = "florina.ticket";
    pub const FLORINA_WHAT: &str = "florina.what";
    pub const FLORINA_PAY_ASK: &str = "florina.pay_ask";
    pub const FLORINA_TICKET_ASK: &str = "florina.ticket_ask";
    pub const FLORINA_DECLINE: &str = "florina.decline";
    pub const FLORINA_SHORT: &str = "florina.short";
    pub const FLORINA_NO_TICKET: &str = "florina.no_ticket";
    pub const FLORINA_WHAT_1: &str = "florina.what_1";
    pub const FLORINA_WHAT_2: &str = "florina.what_2";
    pub const FLORINA_LEAVE: &str = "florina.leave";
    pub const FLORINA_LEAVE_ASK: &str = "florina.leave_ask";
    pub const FLORINA_LEAVE_DECLINE: &str = "florina.leave_decline";
    pub const AMORIA_TO_AMORIA: &str = "amoria.to_amoria";
    pub const AMORIA_TO_HENESYS: &str = "amoria.to_henesys";
    pub const AMORIA_DECLINE: &str = "amoria.decline";
    pub const AMORIA_BYE: &str = "amoria.bye";

    /// Audrey's rides around Malaysia: a choice and its ask with `{map}`
    /// and `{price}`, and the way home with `{map}`.
    pub const MALAYSIA_INTRO_CBD: &str = "malaysia.intro_cbd";
    pub const MALAYSIA_INTRO: &str = "malaysia.intro";
    pub const MALAYSIA_CHOICE: &str = "malaysia.choice";
    pub const MALAYSIA_ASK: &str = "malaysia.ask";
    pub const MALAYSIA_DECLINE: &str = "malaysia.decline";
    pub const MALAYSIA_SHORT: &str = "malaysia.short";
    pub const MALAYSIA_HOME: &str = "malaysia.home";
    pub const MALAYSIA_HOME_GO: &str = "malaysia.home_go";

    /// The Ereve and Rien ferries' pilots and the crane between Orbis, Mu
    /// Lung and Herb Town: a choice and its ask with `{map}` and `{price}`.
    pub const FERRY_INTRO: &str = "ferry.intro";
    pub const FERRY_CHOICE: &str = "ferry.choice";
    pub const FERRY_ASK: &str = "ferry.ask";
    pub const FERRY_DECLINE: &str = "ferry.decline";
    pub const FERRY_SHORT: &str = "ferry.short";

    /// The NLC Taxi between New Leaf City and the Haunted House's mansion
    /// grounds.
    pub const NLC_TAXI_ASK: &str = "nlc_taxi.ask";
    pub const NLC_TAXI_DECLINE: &str = "nlc_taxi.decline";
    pub const NLC_TAXI_SHORT: &str = "nlc_taxi.short";

    /// Kerning City's party quest: Lakelis, who takes a party in (the
    /// terms with `{least}`, `{most}`, `{low}` and `{high}`), and Nella,
    /// who lets players out and back to town.
    pub const KPQ_LAKELIS_NO_PARTY: &str = "kpq.lakelis.no_party";
    pub const KPQ_LAKELIS_NOT_LEADER: &str = "kpq.lakelis.not_leader";
    pub const KPQ_LAKELIS_NOT_ELIGIBLE: &str = "kpq.lakelis.not_eligible";
    pub const KPQ_LAKELIS_BUSY: &str = "kpq.lakelis.busy";
    pub const KPQ_LAKELIS_ASK: &str = "kpq.lakelis.ask";
    pub const KPQ_LAKELIS_DECLINE: &str = "kpq.lakelis.decline";
    pub const KPQ_NELLA_LEAVE: &str = "kpq.nella.leave";
    pub const KPQ_NELLA_HOME: &str = "kpq.nella.home";
    pub const KPQ_CLOTO_Q1: &str = "kpq.cloto.q1";
    pub const KPQ_CLOTO_Q2: &str = "kpq.cloto.q2";
    pub const KPQ_CLOTO_Q3: &str = "kpq.cloto.q3";
    pub const KPQ_CLOTO_Q4: &str = "kpq.cloto.q4";
    pub const KPQ_CLOTO_Q5: &str = "kpq.cloto.q5";
    pub const KPQ_CLOTO_Q6: &str = "kpq.cloto.q6";
    pub const KPQ_CLOTO_Q7: &str = "kpq.cloto.q7";
    pub const KPQ_CLOTO_RIGHT: &str = "kpq.cloto.right";
    pub const KPQ_CLOTO_WRONG: &str = "kpq.cloto.wrong";
    pub const KPQ_CLOTO_THANKS: &str = "kpq.cloto.thanks";
    pub const KPQ_CLOTO_CLEARED: &str = "kpq.cloto.cleared";
    pub const KPQ_CLOTO_LEADER_CLEAR: &str = "kpq.cloto.leader_clear";
    pub const KPQ_CLOTO_LEADER_SHORT: &str = "kpq.cloto.leader_short";
    pub const KPQ_CLOTO_FULL: &str = "kpq.cloto.full";
    pub const KPQ_CLOTO_REFUSED: &str = "kpq.cloto.refused";
    pub const KPQ_CLOTO_FOLLOW: &str = "kpq.cloto.follow";
    pub const KPQ_CLOTO_S2_INTRO: &str = "kpq.cloto.s2.intro";
    pub const KPQ_CLOTO_S3_INTRO: &str = "kpq.cloto.s3.intro";
    pub const KPQ_CLOTO_S4_INTRO: &str = "kpq.cloto.s4.intro";
    pub const KPQ_CLOTO_AREAS_WRONG: &str = "kpq.cloto.areas_wrong";
    pub const KPQ_CLOTO_AREAS_RIGHT: &str = "kpq.cloto.areas_right";
    pub const KPQ_CLOTO_S5_SHORT: &str = "kpq.cloto.s5.short";
    pub const KPQ_CLOTO_S5_CLEAR: &str = "kpq.cloto.s5.clear";
    pub const KPQ_CLOTO_S5_GATHER: &str = "kpq.cloto.s5.gather";
    pub const KPQ_CLOTO_S5_WON: &str = "kpq.cloto.s5.won";

    /// Henesys's party quest: Tory, who takes a party in from Henesys Park
    /// (the terms with `{least}`, `{most}` and `{low}`) and tells how it is
    /// played; Growlie on Primrose Hill, who lets players out; and Tommy,
    /// who sends them home from the way out.
    pub const HPQ_TORY_MENU: &str = "hpq.tory.menu";
    pub const HPQ_TORY_ENTER: &str = "hpq.tory.enter";
    pub const HPQ_TORY_ABOUT: &str = "hpq.tory.about";
    pub const HPQ_TORY_ABOUT_PAGES: [&str; 3] =
        ["hpq.tory.about.0", "hpq.tory.about.1", "hpq.tory.about.2"];
    pub const HPQ_TORY_NO_PARTY: &str = "hpq.tory.no_party";
    pub const HPQ_TORY_NOT_LEADER: &str = "hpq.tory.not_leader";
    pub const HPQ_TORY_NOT_ELIGIBLE: &str = "hpq.tory.not_eligible";
    pub const HPQ_TORY_BUSY: &str = "hpq.tory.busy";
    pub const HPQ_TORY_ASK: &str = "hpq.tory.ask";
    pub const HPQ_TORY_DECLINE: &str = "hpq.tory.decline";
    pub const HPQ_GROWLIE_MENU: &str = "hpq.growlie.menu";
    pub const HPQ_GROWLIE_ABOUT: &str = "hpq.growlie.about";
    pub const HPQ_GROWLIE_ABOUT_PAGES: [&str; 2] = ["hpq.growlie.about.0", "hpq.growlie.about.1"];
    pub const HPQ_GROWLIE_LEAVE: &str = "hpq.growlie.leave";
    pub const HPQ_GROWLIE_LEAVE_ASK: &str = "hpq.growlie.leave.ask";
    pub const HPQ_GROWLIE_STAY: &str = "hpq.growlie.stay";
    pub const HPQ_TOMMY_HOME: &str = "hpq.tommy.home";
    /// Told on Primrose Hill as the Moon Bunny comes out (a light-blue line).
    pub const HPQ_MOON_BUNNY_OUT: &str = "hpq.moon.bunny_out";
    /// The weather that blows then, telling the party what to do.
    pub const HPQ_MOON_WEATHER: &str = "hpq.moon.weather";
    /// Told as the Moon Bunny drops a rice cake, with the `count` so far.
    pub const HPQ_BUNNY_CAKE: &str = "hpq.bunny.cake";
    /// Told on every sixth hit mobs land on the Moon Bunny.
    pub const HPQ_BUNNY_SICK: &str = "hpq.bunny.sick";
    /// Told as the Moon Bunny, out of HP, goes home.
    pub const HPQ_BUNNY_GONE: &str = "hpq.bunny.gone";
    /// Told (in pink) as the party is sent off for losing the Moon Bunny.
    pub const HPQ_BUNNY_LOST: &str = "hpq.bunny.lost";
    pub const JEFF_INTRO: &str = "jeff.intro";
    pub const JEFF_ASK: &str = "jeff.ask";
    pub const JEFF_DECLINE: &str = "jeff.decline";
    pub const JEFF_TOO_WEAK: &str = "jeff.too_weak";
    /// Mushroom Castle's portals: `TD_MC_first`, `obstacle`, `gotocastle`.
    pub const MC_FIRST_BLOCKED: &str = "portal.mc_first.blocked";
    pub const MC_OBSTACLE_BLOCKED: &str = "portal.mc_obstacle.blocked";
    pub const MC_OBSTACLE_USED: &str = "portal.mc_obstacle.used";
    pub const MC_CASTLE_BLOCKED: &str = "portal.mc_castle.blocked";
    /// Gated portals of a004's section 3: the Temple of Time's lanes
    /// (`timeQuest`), Ereve's Training Forest (`enterDisguise0`), Magatia's
    /// secret doors (`secretDoor`), Ariant's palace (`ariant_castle`)
    /// and the black witches' cave (`enterWitch`).
    pub const TIME_LANE_LOST: &str = "portal.time_lane.lost";
    pub const DISGUISE_NO_PERMIT: &str = "portal.disguise.no_permit";
    pub const SECRET_DOOR_LOCKED: &str = "portal.secret_door.locked";
    pub const ARIANT_PALACE_NO_PASS: &str = "portal.ariant_palace.no_pass";
    pub const WITCH_CAVE_CREEPY: &str = "portal.witch_cave.creepy";
    /// The Cursed Forest (`curseforest`): not on Arwen's quests, and by day.
    pub const CURSED_FOREST_CLOSED: &str = "portal.cursed_forest.closed";
    pub const CURSED_FOREST_DAYTIME: &str = "portal.cursed_forest.daytime";
    /// Rien's way out (`rienTutor8`) during an Aran's fitness training.
    pub const RIEN_TUTOR_FITNESS_1: &str = "portal.rien_tutor.fitness1";
    pub const RIEN_TUTOR_FITNESS_2: &str = "portal.rien_tutor.fitness2";
    pub const RIEN_TUTOR_FITNESS_3: &str = "portal.rien_tutor.fitness3";
    /// The Helios Tower elevator (`elevator`) while the car is away.
    pub const ELEVATOR_AWAY: &str = "portal.elevator.away";
    /// Ludibrium's Time Control Room into Ellin Forest (`move_elin`).
    pub const ELLIN_GATE_PASSING: &str = "portal.ellin_gate.passing";
    /// The Bottom of the Temple's shortcut up (`OutPerrion_1`).
    pub const TEMPLE_SHORTCUT: &str = "portal.temple.shortcut";
    /// The Dangerous Forest's way on (`rienTutor7`) during the Aran tutorial.
    pub const RIEN_TUTOR_TO_TOWN: &str = "portal.rien_tutor.to_town";
    /// The training grounds (`enterGym`) without one of Puo's lessons.
    pub const TRAINING_GROUND_NO_LESSON: &str = "portal.training_ground.no_lesson";
    /// The Mirror Cave (`rienCaveEnter`) before the Aran's quests lead there.
    pub const MIRROR_CAVE_BLOCKED: &str = "portal.mirror_cave.blocked";
    /// The Nine Spirit's Nest (`dragonNest`) before quest 3706 is done.
    pub const NINE_SPIRIT_NEST_BLOCKED: &str = "portal.nine_spirit_nest.blocked";
    /// The Mushroom Castle's Security Room (`go_secretroom`) without its key.
    pub const SECURITY_ROOM_LOCKED: &str = "portal.security_room.locked";
    /// Utah's Pig Farm (`q2073`) without Camila's errand.
    pub const PIG_FARM_PRIVATE: &str = "portal.pig_farm.private";
    /// The way past the Cave of Evil Eye (`enterDollWay`) before it's open.
    pub const DOLL_WAY_BLOCKED: &str = "portal.doll_way.blocked";
    /// Ariant's Sand Bandits' hideout (`ariant_Agit`) to anyone else.
    pub const SAND_BANDITS_ONLY: &str = "portal.sand_bandits.members_only";
    /// The Treasure Room of Queen (`skyrom`) while someone is in it.
    pub const TREASURE_ROOM_OCCUPIED: &str = "portal.treasure_room.occupied";
    pub const SOLO_MAP_OCCUPIED: &str = "portal.solo_map.occupied";
    /// The Berserk room's way up to the statues (`s4berserk_move`) while a
    /// monster is left.
    pub const MONSTERS_LEFT: &str = "portal.s4berserk_move.monsters_left";
    /// An explorer medal's count went up (`{name}`, `{count}`, `{needed}`):
    /// the floating message and the info line.
    pub const MEDAL_PROGRESS: &str = "medal.explorer.progress";
    pub const MEDAL_PROGRESS_INFO: &str = "medal.explorer.progress_info";
    /// It reached what the medal needs (`{name}`).
    pub const MEDAL_EARNED: &str = "medal.explorer.earned";
    pub const MEDAL_EARNED_INFO: &str = "medal.explorer.earned_info";
    /// The Master of Disguise's forests (`enterDisguise1` to `5`): someone
    /// else is searching, or he has already been found.
    pub const DISGUISE_FOREST_OCCUPIED: &str = "portal.disguise.occupied";
    pub const DISGUISE_ALREADY_FOUND: &str = "portal.disguise.already_found";
    /// Ways a quest has yet to open (`tristanEnter`, `s4resur_enter`,
    /// `s4rush`).
    pub const MYSTERIOUS_FORCE: &str = "portal.mysterious_force";
    /// The Ellinia-Leafre shortcut (`minar_elli`), which spends a Magic
    /// Seed.
    pub const MAGIC_SEED_SPENT: &str = "portal.minar_elli.spent";
    pub const MAGIC_SEED_NEEDED: &str = "portal.minar_elli.no_seed";
    /// The Nautilus-Omega Sector link (`enter_earth00`, `enter_earth01`)
    /// without a Warp Card.
    pub const WARP_CARD_NEEDED: &str = "portal.earth.no_card";
    /// Ariant's palace rooms (`ariant_queens`) turning out a player not in
    /// Tigun's disguise.
    pub const PALACE_INTRUDER: &str = "portal.ariant_palace.intruder";
    /// The original Maple Island's instruction balloons (`advice00` to
    /// `advice09`, `adviceMap`).
    pub const ADVICE_MOVE: &str = "portal.advice.move";
    pub const ADVICE_HEENA: &str = "portal.advice.heena";
    pub const ADVICE_JUMP: &str = "portal.advice.jump";
    pub const ADVICE_CLIMB: &str = "portal.advice.climb";
    pub const ADVICE_SERA: &str = "portal.advice.sera";
    pub const ADVICE_QUEST_WINDOW: &str = "portal.advice.quest_window";
    pub const ADVICE_PORTAL: &str = "portal.advice.portal";
    pub const ADVICE_WORLD_MAP: &str = "portal.advice.world_map";
    pub const ADVICE_STATS: &str = "portal.advice.stats";
    pub const ADVICE_JUMP_DOWN: &str = "portal.advice.jump_down";
    /// A mini-dungeon's way in (`MD_*`) refusing a party member who isn't
    /// its leader, and saying every copy is taken.
    pub const DUNGEON_NOT_LEADER: &str = "portal.mini_dungeon.not_leader";
    pub const DUNGEON_FULL: &str = "portal.mini_dungeon.full";
    /// The Time Gate (`TD_neoCity_enter`) to Neo City's eras.
    pub const TIME_GATE_INACTIVE: &str = "time_gate.inactive";
    pub const TIME_GATE_NO_ERA: &str = "time_gate.no_era";
    /// The Mysterious Statue (`flower_in`) and Crumbling Statue
    /// (`flower_out`) of the Deep Forest's jump quest.
    pub const STATUE_INTRO: &str = "statue.intro";
    pub const STATUE_MENU: &str = "statue.menu";
    pub const CRUMBLING_ASK: &str = "crumbling.ask";
    /// Shane (`herb_in`) and Louis (`herb_out`) of the Forest of Patience.
    pub const SHANE_TOO_LOW: &str = "shane.too_low";
    pub const SHANE_ASK: &str = "shane.ask";
    pub const SHANE_DECLINE: &str = "shane.decline";
    pub const SHANE_SHORT: &str = "shane.short";
    pub const LOUIS_ASK: &str = "louis.ask";

    /// The Dolphin Taxi (`aqua_taxi`): a menu of a ride down (for a fare,
    /// or with a coupon) and a ride across, each with its yes or no.
    pub const DOLPHIN_MENU: &str = "dolphin.menu";
    pub const DOLPHIN_CHOICE: &str = "dolphin.choice";
    pub const DOLPHIN_COUPON: &str = "dolphin.coupon";
    pub const DOLPHIN_ASK: &str = "dolphin.ask";
    pub const DOLPHIN_COUPON_ASK: &str = "dolphin.coupon_ask";
    pub const DOLPHIN_SHORT: &str = "dolphin.short";

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

    /// Meren's Maker lessons (quests 6029-6036, Magatia): the end scripts of
    /// the three classes (`q6030e` Carson's alchemy, `q6031e` Hughes's
    /// science, `q6032e` Meren's practice), then Meren's crystal test
    /// (`q6033e`) and the Gold Anvil (`q6036e`), which teach the Maker's
    /// levels 2 and 3.
    pub const MAKER_ALCHEMY: [&str; 2] = ["maker.alchemy.0", "maker.alchemy.1"];
    pub const MAKER_SCIENCE: [&str; 2] = ["maker.science.0", "maker.science.1"];
    pub const MAKER_PRACTICE: [&str; 2] = ["maker.practice.0", "maker.practice.1"];
    pub const MAKER_CRYSTAL_TEST: [&str; 2] = ["maker.crystal_test.0", "maker.crystal_test.1"];
    pub const MAKER_GOLD_ANVIL: [&str; 2] = ["maker.gold_anvil.0", "maker.gold_anvil.1"];

    /// The family entitlements' names and descriptions, by index
    /// (`game::family::ENTITLEMENTS`), which the Family window lists.
    pub fn family_entitlement_name(index: usize) -> String {
        format!("family.entitlement.{index}.name")
    }

    pub fn family_entitlement_description(index: usize) -> String {
        format!("family.entitlement.{index}.description")
    }

    /// To a summoner: the family member they asked for said no.
    pub const FAMILY_SUMMON_DECLINED: &str = "family.summon_declined";
    /// To a player who tried the bonding entitlement with too few juniors
    /// online.
    pub const FAMILY_BONDING_SHORT: &str = "family.bonding_short";

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
    #[serde(default)]
    replace: Option<Vec<RewardEntry>>,
    /// Drops added to the pack's (or `replace`'s) list.
    #[serde(default)]
    add: Vec<RewardEntry>,
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
            let id = parse_id(&mob, "override mob id")?;
            let mut drops = entry
                .replace
                .unwrap_or_else(|| table.pack.get(&id).cloned().unwrap_or_default());
            drops.extend(entry.add);
            table.mobs.insert(id, drops);
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
            keys::HERACLE_MENU,
            keys::HERACLE_CREATE,
            keys::HERACLE_DISBAND,
            keys::HERACLE_GROW,
            keys::HERACLE_DISBAND_NOT_MASTER,
            keys::HERACLE_DISBAND_ASK,
            keys::HERACLE_GROW_NOT_MASTER,
            keys::HERACLE_GROW_FULL,
            keys::HERACLE_GROW_ASK,
            keys::HERACLE_GROW_NO_MESOS,
            keys::LEA_MENU,
            keys::LEA_EMBLEM,
            keys::LEA_NOT_MASTER,
            keys::LEA_ASK,
            keys::LEA_DECLINE,
            keys::LEA_NO_MESOS,
            keys::LENARIO_NOT_MASTER,
            keys::LENARIO_MENU,
            keys::LENARIO_INFO,
            keys::LENARIO_HOW,
            keys::LENARIO_CREATE,
            keys::LENARIO_GROW,
            keys::LENARIO_DISBAND,
            keys::LENARIO_INFO_SAY,
            keys::LENARIO_HOW_SAY,
            keys::LENARIO_IN_ALLIANCE,
            keys::LENARIO_NOT_LEADER,
            keys::LENARIO_CREATE_ASK,
            keys::LENARIO_CREATE_NO_MESOS,
            keys::LENARIO_NAME,
            keys::LENARIO_NAME_AGAIN,
            keys::LENARIO_NAME_CONFIRM,
            keys::LENARIO_CREATE_PARTY,
            keys::LENARIO_CREATED,
            keys::LENARIO_GROW_NOT_MASTER,
            keys::LENARIO_GROW_FULL,
            keys::LENARIO_GROW_ASK,
            keys::LENARIO_GROW_NO_MESOS,
            keys::LENARIO_GROWN,
            keys::LENARIO_DISBAND_NOT_MASTER,
            keys::LENARIO_DISBAND_ASK,
            keys::LENARIO_DISBANDED,
            keys::LENARIO_DECLINE,
            keys::LENARIO_FAILED,
            keys::ALLIANCE_NO_GUILD,
            keys::ALLIANCE_IN_ALLIANCE,
            keys::ALLIANCE_OFFLINE,
            keys::ALLIANCE_BUSY,
            keys::ALLIANCE_FULL,
            keys::ALLIANCE_DENIED,
            keys::ALLIANCE_LEFT,
            keys::ALLIANCE_EXPELLED,
            keys::ALLIANCE_MASTER,
            keys::ALLIANCE_RANK,
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
            keys::TAXI_INTRO,
            keys::TAXI_DISCOUNT,
            keys::TAXI_MENU,
            keys::TAXI_CHOICE,
            keys::TAXI_ASK,
            keys::TAXI_DECLINE,
            keys::TAXI_SHORT,
            keys::VIP_CAB_INTRO,
            keys::VIP_CAB_ASK,
            keys::VIP_CAB_DECLINE,
            keys::VIP_CAB_SHORT,
            keys::DANGER_ZONE_TAXI_INTRO,
            keys::DANGER_ZONE_TAXI_ASK,
            keys::DANGER_ZONE_TAXI_DECLINE,
            keys::DANGER_ZONE_TAXI_SHORT,
            keys::CAMEL_CAB_ASK,
            keys::CAMEL_CAB_DECLINE,
            keys::CAMEL_CAB_SHORT,
            keys::SUBWAY_BOARD_ASK,
            keys::SUBWAY_BOARD_DECLINE,
            keys::SUBWAY_BOARD_NO_TICKET,
            keys::SUBWAY_BOARD_CLOSING,
            keys::SUBWAY_BOARD_FULL,
            keys::SUBWAY_TO_NLC_MENU,
            keys::SUBWAY_TO_NLC_CHOICE,
            keys::SUBWAY_TO_NLC_ASK,
            keys::SUBWAY_TO_NLC_LEAVE,
            keys::SUBWAY_TO_KERNING_CITY_MENU,
            keys::SUBWAY_TO_KERNING_CITY_CHOICE,
            keys::SUBWAY_TO_KERNING_CITY_ASK,
            keys::SUBWAY_TO_KERNING_CITY_LEAVE,
            keys::SUBWAY_SELL_DECLINE,
            keys::SUBWAY_SELL_SHORT,
            keys::SUBWAY_LEAVE_DECLINE,
            keys::PLANE_BOOTH_MENU,
            keys::PLANE_BOOTH_BUY,
            keys::PLANE_BOOTH_BOARD,
            keys::PLANE_SELL_ASK,
            keys::PLANE_SELL_DECLINE,
            keys::PLANE_SELL_SHORT,
            keys::PLANE_SELL_FULL,
            keys::PLANE_BOARD_ASK,
            keys::PLANE_BOARD_DECLINE,
            keys::PLANE_BOARD_NO_TICKET,
            keys::PLANE_BOARD_CLOSED,
            keys::PLANE_LEAVE_ASK,
            keys::PLANE_LEAVE_DECLINE,
            keys::PLANE_RIDE,
            keys::WORLD_TRIP_INTRO,
            keys::WORLD_TRIP_OFFER,
            keys::WORLD_TRIP_CHOICE,
            keys::WORLD_TRIP_DESCRIBE,
            keys::WORLD_TRIP_MORE,
            keys::WORLD_TRIP_SHORT,
            keys::WORLD_TRIP_BACK_MENU,
            keys::WORLD_TRIP_BACK_YES,
            keys::WORLD_TRIP_BACK_NO,
            keys::WORLD_TRIP_BACK_GO,
            keys::WORLD_TRIP_BACK_STAY,
            keys::FLORINA_MENU,
            keys::FLORINA_PAY,
            keys::FLORINA_TICKET,
            keys::FLORINA_WHAT,
            keys::FLORINA_PAY_ASK,
            keys::FLORINA_TICKET_ASK,
            keys::FLORINA_DECLINE,
            keys::FLORINA_SHORT,
            keys::FLORINA_NO_TICKET,
            keys::FLORINA_WHAT_1,
            keys::FLORINA_WHAT_2,
            keys::FLORINA_LEAVE,
            keys::FLORINA_LEAVE_ASK,
            keys::FLORINA_LEAVE_DECLINE,
            keys::AMORIA_TO_AMORIA,
            keys::AMORIA_TO_HENESYS,
            keys::AMORIA_DECLINE,
            keys::AMORIA_BYE,
            keys::MALAYSIA_INTRO_CBD,
            keys::MALAYSIA_INTRO,
            keys::MALAYSIA_CHOICE,
            keys::MALAYSIA_ASK,
            keys::MALAYSIA_DECLINE,
            keys::MALAYSIA_SHORT,
            keys::MALAYSIA_HOME,
            keys::MALAYSIA_HOME_GO,
            keys::FERRY_INTRO,
            keys::FERRY_CHOICE,
            keys::FERRY_ASK,
            keys::FERRY_DECLINE,
            keys::FERRY_SHORT,
            keys::NLC_TAXI_ASK,
            keys::NLC_TAXI_DECLINE,
            keys::NLC_TAXI_SHORT,
            keys::KPQ_LAKELIS_NO_PARTY,
            keys::KPQ_LAKELIS_NOT_LEADER,
            keys::KPQ_LAKELIS_NOT_ELIGIBLE,
            keys::KPQ_LAKELIS_BUSY,
            keys::KPQ_LAKELIS_ASK,
            keys::KPQ_LAKELIS_DECLINE,
            keys::KPQ_NELLA_LEAVE,
            keys::KPQ_NELLA_HOME,
            keys::HPQ_TORY_MENU,
            keys::HPQ_TORY_ENTER,
            keys::HPQ_TORY_ABOUT,
            keys::HPQ_TORY_NO_PARTY,
            keys::HPQ_TORY_NOT_LEADER,
            keys::HPQ_TORY_NOT_ELIGIBLE,
            keys::HPQ_TORY_BUSY,
            keys::HPQ_TORY_ASK,
            keys::HPQ_TORY_DECLINE,
            keys::HPQ_GROWLIE_MENU,
            keys::HPQ_GROWLIE_ABOUT,
            keys::HPQ_GROWLIE_LEAVE,
            keys::HPQ_GROWLIE_LEAVE_ASK,
            keys::HPQ_GROWLIE_STAY,
            keys::HPQ_TOMMY_HOME,
            keys::HPQ_MOON_BUNNY_OUT,
            keys::HPQ_MOON_WEATHER,
            keys::HPQ_BUNNY_CAKE,
            keys::HPQ_BUNNY_SICK,
            keys::HPQ_BUNNY_GONE,
            keys::HPQ_BUNNY_LOST,
            keys::KPQ_CLOTO_Q1,
            keys::KPQ_CLOTO_Q2,
            keys::KPQ_CLOTO_Q3,
            keys::KPQ_CLOTO_Q4,
            keys::KPQ_CLOTO_Q5,
            keys::KPQ_CLOTO_Q6,
            keys::KPQ_CLOTO_Q7,
            keys::KPQ_CLOTO_RIGHT,
            keys::KPQ_CLOTO_WRONG,
            keys::KPQ_CLOTO_THANKS,
            keys::KPQ_CLOTO_CLEARED,
            keys::KPQ_CLOTO_LEADER_CLEAR,
            keys::KPQ_CLOTO_LEADER_SHORT,
            keys::KPQ_CLOTO_FULL,
            keys::KPQ_CLOTO_REFUSED,
            keys::KPQ_CLOTO_FOLLOW,
            keys::KPQ_CLOTO_S2_INTRO,
            keys::KPQ_CLOTO_S3_INTRO,
            keys::KPQ_CLOTO_S4_INTRO,
            keys::KPQ_CLOTO_AREAS_WRONG,
            keys::KPQ_CLOTO_AREAS_RIGHT,
            keys::KPQ_CLOTO_S5_SHORT,
            keys::KPQ_CLOTO_S5_CLEAR,
            keys::KPQ_CLOTO_S5_GATHER,
            keys::KPQ_CLOTO_S5_WON,
            keys::JEFF_INTRO,
            keys::JEFF_ASK,
            keys::JEFF_DECLINE,
            keys::JEFF_TOO_WEAK,
            keys::MC_FIRST_BLOCKED,
            keys::MC_OBSTACLE_BLOCKED,
            keys::MC_OBSTACLE_USED,
            keys::MC_CASTLE_BLOCKED,
            keys::TIME_LANE_LOST,
            keys::DISGUISE_NO_PERMIT,
            keys::SECRET_DOOR_LOCKED,
            keys::ARIANT_PALACE_NO_PASS,
            keys::WITCH_CAVE_CREEPY,
            keys::CURSED_FOREST_CLOSED,
            keys::CURSED_FOREST_DAYTIME,
            keys::RIEN_TUTOR_FITNESS_1,
            keys::RIEN_TUTOR_FITNESS_2,
            keys::RIEN_TUTOR_FITNESS_3,
            keys::ELEVATOR_AWAY,
            keys::ELLIN_GATE_PASSING,
            keys::TEMPLE_SHORTCUT,
            keys::RIEN_TUTOR_TO_TOWN,
            keys::TRAINING_GROUND_NO_LESSON,
            keys::MIRROR_CAVE_BLOCKED,
            keys::NINE_SPIRIT_NEST_BLOCKED,
            keys::SECURITY_ROOM_LOCKED,
            keys::PIG_FARM_PRIVATE,
            keys::DOLL_WAY_BLOCKED,
            keys::SAND_BANDITS_ONLY,
            keys::TREASURE_ROOM_OCCUPIED,
            keys::SOLO_MAP_OCCUPIED,
            keys::MONSTERS_LEFT,
            keys::MEDAL_PROGRESS,
            keys::MEDAL_PROGRESS_INFO,
            keys::MEDAL_EARNED,
            keys::MEDAL_EARNED_INFO,
            keys::DISGUISE_FOREST_OCCUPIED,
            keys::DISGUISE_ALREADY_FOUND,
            keys::MYSTERIOUS_FORCE,
            keys::MAGIC_SEED_SPENT,
            keys::MAGIC_SEED_NEEDED,
            keys::WARP_CARD_NEEDED,
            keys::PALACE_INTRUDER,
            keys::ADVICE_MOVE,
            keys::ADVICE_HEENA,
            keys::ADVICE_JUMP,
            keys::ADVICE_CLIMB,
            keys::ADVICE_SERA,
            keys::ADVICE_QUEST_WINDOW,
            keys::ADVICE_PORTAL,
            keys::ADVICE_WORLD_MAP,
            keys::ADVICE_STATS,
            keys::ADVICE_JUMP_DOWN,
            keys::DUNGEON_NOT_LEADER,
            keys::DUNGEON_FULL,
            keys::TIME_GATE_INACTIVE,
            keys::TIME_GATE_NO_ERA,
            keys::STATUE_INTRO,
            keys::STATUE_MENU,
            keys::CRUMBLING_ASK,
            keys::SHANE_TOO_LOW,
            keys::SHANE_ASK,
            keys::SHANE_DECLINE,
            keys::SHANE_SHORT,
            keys::LOUIS_ASK,
            keys::DOLPHIN_MENU,
            keys::DOLPHIN_CHOICE,
            keys::DOLPHIN_COUPON,
            keys::DOLPHIN_ASK,
            keys::DOLPHIN_COUPON_ASK,
            keys::DOLPHIN_SHORT,
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
            .chain(&keys::MAKER_ALCHEMY)
            .chain(&keys::MAKER_SCIENCE)
            .chain(&keys::MAKER_PRACTICE)
            .chain(&keys::MAKER_CRYSTAL_TEST)
            .chain(&keys::MAKER_GOLD_ANVIL)
            .chain(&keys::HPQ_TORY_ABOUT_PAGES)
            .chain(&keys::HPQ_GROWLIE_ABOUT_PAGES)
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

    #[test]
    fn manon_and_griffey_add_the_pirates_pentagon_and_star() {
        let pirate = |item| RewardEntry {
            prob: 0.4,
            ..quest_item(item, 6944)
        };
        let table = RewardTable::load(None).unwrap();
        assert_eq!(table.drops(8180000), [pirate(4031860)]);
        assert_eq!(table.drops(8180001), [pirate(4031861)]);

        let dir = std::env::temp_dir().join(format!("rustms-manon-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("rewards.json"),
            r#"{"8180000": [{"item": 4031343, "prob": 0.4}]}"#,
        )
        .unwrap();
        let table = RewardTable::load(Some(&dir)).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        let drops = table.drops(8180000);
        assert_eq!(drops[0].item, Some(4031343));
        assert_eq!(drops[1..], [pirate(4031860)]);
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
