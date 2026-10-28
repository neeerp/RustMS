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
const GACHAPON_PLACEHOLDER: &str = include_str!("../gachapon_placeholder.toml");
const SHOP_ADDITIONS: &str = include_str!("../shop_additions.toml");

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
    pub const KPQ_NELLA_BONUS: &str = "kpq.nella.bonus";
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
    pub const KPQ_CLOTO_S5_FULL: &str = "kpq.cloto.s5.full";

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
    pub const HPQ_GROWLIE_CAKES: &str = "hpq.growlie.cakes";
    pub const HPQ_GROWLIE_CAKES_OK: &str = "hpq.growlie.cakes.ok";
    pub const HPQ_GROWLIE_CAKES_SHORT: &str = "hpq.growlie.cakes.short";
    pub const HPQ_TOMMY_PIGS: &str = "hpq.tommy.pigs";
    pub const HPQ_TOMMY_PIGS_ASK: &str = "hpq.tommy.pigs.ask";
    pub const HPQ_TOMMY_PIGS_LEADER: &str = "hpq.tommy.pigs.leader";
    pub const HPQ_TOMMY_PIGS_LEAVE: &str = "hpq.tommy.pigs.leave";
    pub const HPQ_TORY_REWARD_ASK: &str = "hpq.tory.reward.ask";
    pub const HPQ_TORY_REWARD_FULL: &str = "hpq.tory.reward.full";
    pub const HPQ_TORY_HOME_ASK: &str = "hpq.tory.home.ask";
    /// Ludibrium's party quest: the Red Sign, which takes a party in from
    /// Eos Tower's 101st floor (the terms with `{least}`, `{most}`, `{low}`
    /// and `{high}`) and says what the quest is; and Sgt. Anderson, who
    /// lets players out, then sends them down to Eos Tower.
    pub const LPQ_SIGN_MENU: &str = "lpq.sign.menu";
    pub const LPQ_SIGN_ENTER: &str = "lpq.sign.enter";
    pub const LPQ_SIGN_ABOUT: &str = "lpq.sign.about";
    pub const LPQ_SIGN_INFO: &str = "lpq.sign.info";
    pub const LPQ_SIGN_NO_PARTY: &str = "lpq.sign.no_party";
    pub const LPQ_SIGN_NOT_LEADER: &str = "lpq.sign.not_leader";
    pub const LPQ_SIGN_NOT_ELIGIBLE: &str = "lpq.sign.not_eligible";
    pub const LPQ_SIGN_BUSY: &str = "lpq.sign.busy";
    pub const LPQ_ANDERSON_LEAVE_ASK: &str = "lpq.anderson.leave.ask";
    pub const LPQ_ANDERSON_HOME: &str = "lpq.anderson.home";
    pub const LPQ_BALLOON_INTRO_1: &str = "lpq.balloon.intro.1";
    pub const LPQ_BALLOON_INTRO_2: &str = "lpq.balloon.intro.2";
    pub const LPQ_BALLOON_INTRO_3: &str = "lpq.balloon.intro.3";
    pub const LPQ_BALLOON_INTRO_4: &str = "lpq.balloon.intro.4";
    pub const LPQ_BALLOON_INTRO_5: &str = "lpq.balloon.intro.5";
    pub const LPQ_BALLOON_INTRO_7: &str = "lpq.balloon.intro.7";
    pub const LPQ_BALLOON_INTRO_9: &str = "lpq.balloon.intro.9";
    pub const LPQ_BALLOON_NOT_LEADER: &str = "lpq.balloon.not_leader";
    pub const LPQ_BALLOON_SHORT: &str = "lpq.balloon.short";
    pub const LPQ_BALLOON_DONE: &str = "lpq.balloon.done";
    pub const LPQ_BALLOON_HURRY: &str = "lpq.balloon.hurry";
    pub const LPQ_BALLOON_S6: &str = "lpq.balloon.s6";
    /// Stage 8's balloon: what it asks, not five on the crates, and the
    /// wrong five.
    pub const LPQ_BALLOON_INTRO_8: &str = "lpq.balloon.intro.8";
    pub const LPQ_BALLOON_FEW: &str = "lpq.balloon.few";
    pub const LPQ_BALLOON_WRONG: &str = "lpq.balloon.wrong";
    /// The Pink Balloon on the bonus stage: leave it early?, and staying.
    pub const LPQ_PINK_LEAVE_ASK: &str = "lpq.pink.leave.ask";
    pub const LPQ_PINK_STAY: &str = "lpq.pink.stay";
    /// Arturo with the reward, and to a member without room for it.
    pub const LPQ_ARTURO_THANKS: &str = "lpq.arturo.thanks";
    pub const LPQ_ARTURO_FULL: &str = "lpq.arturo.full";
    pub const LPQ_NOTICE_TRAP: &str = "lpq.notice.trap";
    /// Pink lines as Rombad (stage 7) and Alishar (the boss) come out.
    pub const LPQ_NOTICE_ROMBAD: &str = "lpq.notice.rombad";
    pub const LPQ_NOTICE_ALISHAR: &str = "lpq.notice.alishar";
    /// Orbis's party quest: Wonky, who takes a party into the Tower of
    /// Goddess from Orbis (the terms with `{least}`, `{most}`, `{low}` and
    /// `{high}`), says what the quest is, and lets players out from the
    /// cloud stage; and Eak, who sends them back to Orbis from the way out.
    pub const OPQ_WONKY_MENU: &str = "opq.wonky.menu";
    pub const OPQ_WONKY_ENTER: &str = "opq.wonky.enter";
    pub const OPQ_WONKY_ABOUT: &str = "opq.wonky.about";
    pub const OPQ_WONKY_INFO: &str = "opq.wonky.info";
    pub const OPQ_WONKY_NO_PARTY: &str = "opq.wonky.no_party";
    pub const OPQ_WONKY_NOT_LEADER: &str = "opq.wonky.not_leader";
    pub const OPQ_WONKY_NOT_ELIGIBLE: &str = "opq.wonky.not_eligible";
    pub const OPQ_WONKY_BUSY: &str = "opq.wonky.busy";
    pub const OPQ_WONKY_LEAVE_ASK: &str = "opq.wonky.leave.ask";
    pub const OPQ_EAK_HOME: &str = "opq.eak.home";
    /// Told on the cloud stage as Eak steps out of the light (a pink line).
    pub const OPQ_NOTICE_LIGHT: &str = "opq.notice.light";
    /// Said to each member as the team arrives on the cloud stage.
    pub const OPQ_EAK_ARRIVAL: &str = "opq.eak.arrival";
    pub const OPQ_EAK_CLOUD: &str = "opq.eak.cloud";
    pub const OPQ_EAK_LEADER_ONLY: &str = "opq.eak.leader_only";
    pub const OPQ_EAK_GATHER: &str = "opq.eak.gather";
    /// Told to a player at the hub's storage door while it is shut (a pink
    /// line).
    pub const OPQ_NOTICE_STORAGE_CLOSED: &str = "opq.notice.storage_closed";
    /// Told to a player at a jail's way on while `{n}` of its levers stand
    /// wrong (a pink line).
    pub const OPQ_NOTICE_JAIL_LEVERS: &str = "opq.notice.jail_levers";
    /// Eak in a room, asking for `{n}` of `{item}`.
    pub const OPQ_EAK_HAND_IN: &str = "opq.eak.hand_in";
    /// Eak giving the leader `{piece}`, a piece of the statue.
    pub const OPQ_EAK_PIECE: &str = "opq.eak.piece";
    /// Eak in a room already cleared, sending the team on.
    pub const OPQ_EAK_ROOM_CLEARED: &str = "opq.eak.room_cleared";
    /// Eak to a leader with no room in ETC for the piece.
    pub const OPQ_EAK_FULL: &str = "opq.eak.full";
    /// Eak in the music room before the day's record plays.
    pub const OPQ_EAK_MUSIC_ASK: &str = "opq.eak.music.ask";
    /// Eak once it plays, as the stone box comes out.
    pub const OPQ_EAK_MUSIC_PLAYED: &str = "opq.eak.music.played";
    /// Eak in the music room once it is cleared.
    pub const OPQ_EAK_MUSIC_DONE: &str = "opq.eak.music.done";
    /// Eak in the sealed room when not three stand on its platforms.
    pub const OPQ_EAK_HEADS_FEW: &str = "opq.eak.heads.few";
    /// Eak when one platform has the number he wants.
    pub const OPQ_EAK_HEADS_ONE_RIGHT: &str = "opq.eak.heads.one_right";
    /// Eak when no platform has the number he wants.
    pub const OPQ_EAK_HEADS_ALL_WRONG: &str = "opq.eak.heads.all_wrong";
    /// Eak when all three do, as the stone box comes out above.
    pub const OPQ_EAK_HEADS_RIGHT: &str = "opq.eak.heads.right";
    /// Eak in the sealed room once it is cleared.
    pub const OPQ_EAK_HEADS_DONE: &str = "opq.eak.heads.done";
    /// Eak in the tall room, before two levers are up.
    pub const OPQ_EAK_LEVERS_ASK: &str = "opq.eak.levers.ask";
    /// Eak in the tall room: one of the two levers up is right.
    pub const OPQ_EAK_LEVERS_ONE_RIGHT: &str = "opq.eak.levers.one_right";
    /// Eak in the tall room: neither lever up is right.
    pub const OPQ_EAK_LEVERS_NONE_RIGHT: &str = "opq.eak.levers.none_right";
    /// Eak in the tall room: the right two are up, and the stone box is out.
    pub const OPQ_EAK_LEVERS_RIGHT: &str = "opq.eak.levers.right";
    /// Eak in the tall room once it is cleared.
    pub const OPQ_EAK_LEVERS_DONE: &str = "opq.eak.levers.done";
    /// Eak in the garden, on how to draw out Papa Pixie.
    pub const OPQ_EAK_GARDEN: &str = "opq.eak.garden";
    /// Eak on the hub once the garden is cleared: the Grass of Life goes
    /// on the statue.
    pub const OPQ_EAK_GRASS: &str = "opq.eak.grass";
    /// Eak on the hub once Minerva is free.
    pub const OPQ_EAK_MINERVA: &str = "opq.eak.minerva";
    /// Told in the garden as Papa Pixie comes out (a pink line).
    pub const OPQ_NOTICE_PAPA_PIXIE: &str = "opq.notice.papa_pixie";
    /// Told to a player at the garden's way out who may not take it (a
    /// pink line).
    pub const OPQ_NOTICE_GARDEN_OUT: &str = "opq.notice.garden_out";
    /// Minerva on the hub, freed, asking whether to go to her prize room.
    pub const OPQ_MINERVA_ASK: &str = "opq.minerva.ask";
    /// Minerva's thanks in her prize room, before the prize.
    pub const OPQ_MINERVA_THANKS: &str = "opq.minerva.thanks";
    /// Minerva to a player without room for any of her prizes.
    pub const OPQ_MINERVA_FULL: &str = "opq.minerva.full";
    /// Eak in the hidden storage while its guards are left.
    pub const OPQ_EAK_STORAGE: &str = "opq.eak.storage";
    /// Minerva in the treasure room, asking whether to leave it.
    pub const OPQ_MINERVA_BONUS_ASK: &str = "opq.minerva.bonus_ask";
    /// Eak in the storage room while a monster or drop is left there.
    pub const OPQ_EAK_SALION_FIND: &str = "opq.eak.salion.find";
    /// Eak giving the statue's second piece in the bare storage room.
    pub const OPQ_EAK_SALION_FOUND: &str = "opq.eak.salion.found";
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
    /// Told (in pink) as the Killer Mushroom Spore (2430014) opens the
    /// `obstacle` portal's way, and as the Thorn Remover (2430015) clears
    /// the way to the castle.
    pub const MC_SPORE_USED: &str = "item.mc_spore.used";
    pub const MC_THORNS_REMOVED: &str = "item.mc_thorns.removed";
    /// James (1300008), found in the castle (2325), and his escape in a
    /// Mushroom Disguise (2327); the player answers him. Unsourced.
    pub const JAMES_SCARED: &str = "mc_james.scared";
    pub const JAMES_SENT_BY_BROTHER: &str = "mc_james.sent_by_brother";
    pub const JAMES_SAFE: &str = "mc_james.safe";
    pub const JAMES_THANKS: &str = "mc_james.thanks";
    pub const JAMES_DISGUISE: &str = "mc_james.disguise";
    /// Scarrs (1300007) gives a spare Killer Mushroom Spore (2338).
    /// Unsourced.
    pub const SCARRS_SPARE_SPORE: &str = "mc_scarrs.spare_spore";
    pub const SCARRS_SPORE_FULL: &str = "mc_scarrs.spore_full";
    pub const REMEMBERER_CHARMS: &str = "mc_rememberer.charms";
    pub const REMEMBERER_KARCASA: &str = "mc_rememberer.karcasa";
    pub const CAMEL_CAB_KARCASA: &str = "mc_camel_cab.karcasa";
    pub const CAMEL_CAB_MEERKATS_ASK: &str = "mc_camel_cab.meerkats_ask";
    pub const CAMEL_CAB_MEERKATS_GO: &str = "mc_camel_cab.meerkats_go";
    pub const CAMEL_CAB_MEERKATS_DONE: &str = "mc_camel_cab.meerkats_done";
    pub const CAMEL_CAB_SAHEL: &str = "mc_camel_cab.sahel";
    pub const CAMEL_CAB_NOT_HERE: &str = "mc_camel_cab.not_here";
    pub const CAMEL_CAB_PRIVATE: &str = "mc_camel_cab.private";
    pub const CAMEL_CAB_LEVEL_30: &str = "mc_camel_cab.level_30";
    pub const CAMEL_CAB_CASTLE_WAY: &str = "mc_camel_cab.castle_way";
    /// Joining Zenumist and Alcadno (`q3301e`, `q3303e`) and re-acquiring
    /// their capes (`q3305s`, `q3306s`).
    pub const HAN_ZENUMIST_ORES: &str = "magatia_han.zenumist_ores";
    pub const HAN_ALCADNO_ORES: &str = "magatia_han.alcadno_ores";
    pub const HAN_NO_ORES: &str = "magatia_han.no_ores";
    pub const HAN_ZENUMIST_WAIT: &str = "magatia_han.zenumist_wait";
    pub const HAN_ALCADNO_WAIT: &str = "magatia_han.alcadno_wait";
    pub const CARSON_CAPE_LOST: &str = "magatia_cape.carson_lost";
    pub const CARSON_CAPE_ASK: &str = "magatia_cape.carson_ask";
    pub const CARSON_CAPE_GO: &str = "magatia_cape.carson_go";
    pub const MAED_CAPE_LOST: &str = "magatia_cape.maed_lost";
    pub const MAED_CAPE_ASK: &str = "magatia_cape.maed_ask";
    pub const MAED_CAPE_GO: &str = "magatia_cape.maed_go";
    /// Dr. De Lang in Dran's Lab (`q3320s`, `q3321s`, `q3353s`, `q3354s`),
    /// Parwen's password (`q3360s`, `{password}`) and the Secret Passage
    /// asking for it.
    pub const PARWEN_KNOWS_ASK: &str = "magatia_de_lang.parwen_knows_ask";
    pub const DE_LANG_MISSING_STORY: &str = "magatia_de_lang.missing_story";
    pub const DE_LANG_MISSING_HELP: &str = "magatia_de_lang.missing_help";
    pub const DE_LANG_MISSING_ASK: &str = "magatia_de_lang.missing_ask";
    pub const DE_LANG_MISSING_PENDANT: &str = "magatia_de_lang.missing_pendant";
    pub const DE_LANG_MISSING_PIPES: &str = "magatia_de_lang.missing_pipes";
    pub const DE_LANG_MISSING_GO: &str = "magatia_de_lang.missing_go";
    pub const PARWEN_DE_LANG_WANTS: &str = "magatia_de_lang.parwen_wants";
    pub const PARWEN_DE_LANG_WANTS_ASK: &str = "magatia_de_lang.parwen_wants_ask";
    pub const DE_LANG_POTION_ASK: &str = "magatia_de_lang.potion_ask";
    pub const PARWEN_PASSWORD_KEY: &str = "magatia_de_lang.password_key";
    pub const PARWEN_PASSWORD_ASK: &str = "magatia_de_lang.password_ask";
    pub const PARWEN_PASSWORD_HURRY: &str = "magatia_de_lang.password_hurry";
    pub const PARWEN_PASSWORD_CODE: &str = "magatia_de_lang.password_code";
    pub const SECRET_PASSAGE_PASSWORD: &str = "magatia_de_lang.passage_password";
    pub const SECRET_PASSAGE_WRONG: &str = "magatia_de_lang.passage_wrong";
    /// End scripts that take what the quest asks for and give rewards:
    /// Dr. Kim (`q3452e`), Mr. Ku (`q3833e`) and Muhamad's answers
    /// (`q3953e`).
    pub const DR_KIM_THANKS: &str = "omega_dr_kim.thanks";
    pub const DR_KIM_FULL: &str = "omega_dr_kim.full";
    pub const MR_KU_THANKS: &str = "herb_town_mr_ku.thanks";
    pub const MR_KU_FULL: &str = "herb_town_mr_ku.full";
    pub const MUHAMAD_DEO_PEACEFUL: &str = "ariant_muhamad.deo_peaceful";
    pub const MUHAMAD_MERCHANTS: &str = "ariant_muhamad.merchants";
    pub const MUHAMAD_TRESPASSED: &str = "ariant_muhamad.trespassed";
    pub const MUHAMAD_QUEEN: &str = "ariant_muhamad.queen";
    pub const MUHAMAD_GUARDIANS: &str = "ariant_muhamad.guardians";
    pub const MUHAMAD_GUARDIANS_WHERE: &str = "ariant_muhamad.guardians_where";
    pub const MUHAMAD_EXPEDITION: &str = "ariant_muhamad.expedition";
    pub const MUHAMAD_DEO_MONSTER: &str = "ariant_muhamad.deo_monster";
    pub const MUHAMAD_CONVINCED: &str = "ariant_muhamad.convinced";
    pub const ARDIN_TEST_AGAIN: &str = "ariant_ardin.test_again";
    pub const ARDIN_TEST_ASK: &str = "ariant_ardin.test_ask";
    pub const ARDIN_TEST_GO: &str = "ariant_ardin.test_go";
    pub const ARDIN_TEST_BUSY: &str = "ariant_ardin.test_busy";
    pub const SORCERER_POTION_WORKED: &str = "temple_sorcerer.potion_worked";
    pub const METEORITE_FULL: &str = "omega_meteorite.full";
    pub const GRAY_GARBLED: &str = "omega_gray.garbled";
    pub const ELESKA_BOX_THANKS: &str = "ariant_eleska.box_thanks";
    pub const MANJI_DESERT_MISSION: &str = "perion_manji.desert_mission";
    pub const NAUTILUS_BUSH_SEARCH: &str = "nautilus_bush.search";
    pub const NAUTILUS_BUSH_SEARCHED: &str = "nautilus_bush.searched";
    pub const NAUTILUS_BUSH_FULL: &str = "nautilus_bush.full";
    pub const NAUTILUS_ABEL_FOUND: &str = "nautilus_abel.found";
    pub const NAUTILUS_ABEL_OTHER: &str = "nautilus_abel.other";
    pub const NAUTILUS_ABEL_FULL: &str = "nautilus_abel.full";
    pub const RUMOR_BLACKBULL: &str = "perion_rumor.blackbull";
    pub const RUMOR_MANJI: &str = "perion_rumor.manji";
    pub const RUMOR_AYAN: &str = "perion_rumor.ayan";
    pub const RUMOR_DANCES_WITH_BALROG: &str = "perion_rumor.dances_with_balrog";
    pub const RUMOR_BETTY: &str = "perion_rumor.betty";
    pub const TIENK_GREETING: &str = "lith_tienk.greeting";
    pub const RHO_LAST_SONG: &str = "kerning_rho.last_song";
    pub const NEW_LEAF_CITY_DECLINE: &str = "nlc_quest.decline";
    pub const BARRICADE_MARK_ASK: &str = "nlc_barricade.mark_ask";
    pub const BARRICADE_MARK_GO: &str = "nlc_barricade.mark_go";
    pub const LUKAN_STORM_ASK: &str = "nlc_lukan.storm_ask";
    pub const LUKAN_STORM_GO: &str = "nlc_lukan.storm_go";
    pub const TAGGRIN_WOODS_ASK: &str = "nlc_taggrin.woods_ask";
    pub const TAGGRIN_WOODS_GO: &str = "nlc_taggrin.woods_go";
    pub const TAGGRIN_PATH_ASK: &str = "nlc_taggrin.path_ask";
    pub const TAGGRIN_PATH_GO: &str = "nlc_taggrin.path_go";
    pub const TAGGRIN_WARRIORS_ASK: &str = "nlc_taggrin.warriors_ask";
    pub const TAGGRIN_WARRIORS_GO: &str = "nlc_taggrin.warriors_go";
    pub const LITA_LEPRECHAUNS_ASK: &str = "nlc_lita.leprechauns_ask";
    pub const LITA_ELDERWRAITHS_ASK: &str = "nlc_lita.elderwraiths_ask";
    pub const LITA_HORSEMAN_ASK: &str = "nlc_lita.horseman_ask";
    pub const LITA_BIGFOOT_ASK: &str = "nlc_lita.bigfoot_ask";
    pub const LITA_BOUNTY_GO: &str = "nlc_lita.bounty_go";
    pub const NEW_LEAF_CITY_NEEDS_YOU: &str = "nlc_quest.needs_you";
    pub const NEW_LEAF_CITY_ETC_FULL: &str = "nlc_quest.etc_full";
    pub const BARRICADE_JACK_ASK: &str = "nlc_barricade.jack_ask";
    pub const BARRICADE_JACK_GO: &str = "nlc_barricade.jack_go";
    pub const JACK_BROTHER: &str = "nlc_jack.brother";
    pub const JACK_MAP: &str = "nlc_jack.map";
    pub const JACK_SETUP_FULL: &str = "nlc_jack.setup_full";
    pub const LUKAN_BADGES_ASK: &str = "nlc_lukan.badges_ask";
    pub const LUKAN_BADGES_GO: &str = "nlc_lukan.badges_go";
    pub const LUKAN_BADGES_THANKS: &str = "nlc_lukan.badges_thanks";
    pub const LUKAN_BADGES_MISSING: &str = "nlc_lukan.badges_missing";
    pub const JACK_LETTER_ASK: &str = "nlc_jack.letter_ask";
    pub const JACK_LETTER_GO: &str = "nlc_jack.letter_go";
    pub const BARRICADE_GLYPHS_ASK: &str = "nlc_barricade.glyphs_ask";
    pub const BARRICADE_GLYPHS_GO: &str = "nlc_barricade.glyphs_go";
    pub const ELPAM_TRANSLATE: &str = "nlc_elpam.translate";
    pub const ELPAM_NO_LETTER: &str = "nlc_elpam.no_letter";
    pub const BARRICADE_TRANSLATED_ASK: &str = "nlc_barricade.translated_ask";
    pub const JACK_TRANSLATION: &str = "nlc_jack.translation";
    pub const JACK_NO_TRANSLATION: &str = "nlc_jack.no_translation";
    pub const JACK_TIDE_ASK: &str = "nlc_jack.tide_ask";
    pub const JACK_TIDE_GO: &str = "nlc_jack.tide_go";
    pub const JACK_KEYSTONE: &str = "nlc_jack.keystone";
    pub const JACK_KEYSTONE_LOST: &str = "nlc_jack.keystone_lost";
    pub const JACK_HURRY: &str = "nlc_jack.hurry";
    pub const ADVENTURER_TITLE: &str = "adventurer_medal.title";
    pub const ADVENTURER_MEDAL: &str = "adventurer_medal.medal";
    pub const ADVENTURER_MEDAL_FULL: &str = "adventurer_medal.full";
    pub const SHANKS_FOR_LUCAS: &str = "shanks.for_lucas";
    pub const OUTSTANDING_CITIZEN_NOT_YET: &str = "outstanding_citizen.not_yet";
    pub const MEDAL_TITLE: &str = "medal_title";
    pub const KNIGHT_TITLE: &str = "knight_title";
    /// Dalair sets Title Challenge - Celebrity (`q29002s`), and at its end
    /// (`q29002e`) finds the fame short or the time run out.
    pub const CELEBRITY_START: &str = "celebrity.start";
    pub const CELEBRITY_NOT_YET: &str = "celebrity.not_yet";
    pub const CELEBRITY_TOO_LATE: &str = "celebrity.too_late";
    pub const VETERAN_HUNTER_START: &str = "veteran_hunter.start";
    pub const VETERAN_HUNTER_NOT_YET: &str = "veteran_hunter.not_yet";
    pub const VETERAN_HUNTER_TOO_LATE: &str = "veteran_hunter.too_late";
    /// A job instructor's recommendation letter to the Mushking Empire
    /// (`q2300s`-`q2304s`), and the Head Security Officer reading it
    /// (`q2300e`-`q2304e`), the player answering him.
    pub const MC_LETTER_ASK: &str = "mc_letter.ask";
    pub const MC_LETTER_LATER: &str = "mc_letter.later";
    pub const MC_LETTER_FULL: &str = "mc_letter.full";
    pub const MC_LETTER_KINGDOM: &str = "mc_letter.kingdom";
    pub const MC_LETTER_WRITTEN: &str = "mc_letter.written";
    pub const MC_LETTER_TAKE_YOU: &str = "mc_letter.take_you";
    pub const MC_LETTER_ROUTE: &str = "mc_letter.route";
    pub const MC_OFFICER_LETTER: &str = "mc_officer.letter";
    pub const MC_OFFICER_LETTER_REPLY: &str = "mc_officer.letter.reply";
    pub const MC_OFFICER_WELCOME: &str = "mc_officer.welcome";
    /// The Secretary of Domestic Affairs (1300003) sends the player to
    /// look at the forest's barrier (`q2314s`) and at the castle's walls
    /// (`q2322s`), which the `investigate1` and `investigate2` portals let
    /// the player think over.
    pub const MC_FOREST_ASK: &str = "mc_secretary.forest.ask";
    pub const MC_FOREST_DECLINE: &str = "mc_secretary.forest.decline";
    pub const MC_FOREST_GO: &str = "mc_secretary.forest.go";
    pub const MC_WALLS_ASK: &str = "mc_secretary.walls.ask";
    pub const MC_WALLS_GO: &str = "mc_secretary.walls.go";
    pub const MC_BARRIER_INVESTIGATED: &str = "portal.investigate1";
    pub const MC_WALLS_INVESTIGATED: &str = "portal.investigate2";
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
    /// The Neglected Strolling Path's `in00` (`enterNepenthes`) when an
    /// Aran on To the Sealed Garden (21739) finds the garden taken.
    pub const SEALED_GARDEN_OCCUPIED: &str = "portal.enterNepenthes.occupied";
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
    /// Corba (2082003), who makes the player a dragon for the flight.
    pub const CORBA_MENU: &str = "corba.menu";
    pub const CORBA_CHOICE: &str = "corba.choice";
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

    /// The stylists (`net::stylists`): the menu, a label per look taking
    /// the coupon as `{item}`, the choice of looks or the yes to a rolled
    /// one, the line after, the line to a player without the coupon, and
    /// the line to one without a one-time lens.
    pub const STYLIST_MENU: &str = "stylist.menu";
    pub const STYLIST_CUT: &str = "stylist.cut";
    pub const STYLIST_DYE: &str = "stylist.dye";
    pub const STYLIST_FACE: &str = "stylist.face";
    pub const STYLIST_LENS: &str = "stylist.lens";
    pub const STYLIST_ONE_TIME_LENS: &str = "stylist.one_time_lens";
    pub const STYLIST_SKIN: &str = "stylist.skin";
    pub const STYLIST_CHOOSE: &str = "stylist.choose";
    pub const STYLIST_CHOOSE_LENS: &str = "stylist.choose_lens";
    pub const STYLIST_RANDOM: &str = "stylist.random";
    pub const STYLIST_DONE: &str = "stylist.done";
    pub const STYLIST_NO_COUPON: &str = "stylist.no_coupon";
    pub const STYLIST_NO_LENS: &str = "stylist.no_lens";

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

    /// JM's list of informants (Kerning City, quests 2214-2219): the Knocked
    /// Trash Can in the Hut in the Swamp (NPC 1052108) holds it from 5 to 8
    /// p.m. (`q2214e`, and `q2215e` for a lost list with its fee), then each informant tells
    /// what they know about Dyle (`q2216s` Mr. Pickall, `q2217s` Shumi,
    /// `q2218s` Nella, `q2219s` Jake). No licensed script has their text,
    /// so these keep their placeholders.
    pub const DYLE_TRASH_CAN_NOT_YET: &str = "dyle.trash_can.not_yet";
    pub const DYLE_TRASH_CAN_FULL: &str = "dyle.trash_can.full";
    pub const DYLE_TRASH_CAN_FOUND: &str = "dyle.trash_can.found";
    pub const DYLE_TRASH_CAN_SHORT: &str = "dyle.trash_can.short";
    pub const DYLE_TRASH_CAN_PAID: &str = "dyle.trash_can.paid";
    pub const DYLE_INFORMANTS: [&str; 4] = [
        "dyle.informant.0",
        "dyle.informant.1",
        "dyle.informant.2",
        "dyle.informant.3",
    ];

    /// The Remnants of Horned Tail (quest 3714, `q3714s`): Nine Spirit's
    /// Baby Dragon (NPC 2081011) in its nest hatches from an egg and gives a
    /// Dragon Stone, unless one is still held. No licensed script has the
    /// text, so these keep their placeholders.
    pub const NINE_SPIRIT_HATCHED: &str = "nine_spirit.baby.hatched";
    pub const NINE_SPIRIT_STONE_HELD: &str = "nine_spirit.baby.stone_held";
    pub const NINE_SPIRIT_FULL: &str = "nine_spirit.baby.full";

    /// Reef's Gratitude (quest 2228, `q2228s`): Reef (NPC 1032108) thanks
    /// the player for beating Faust. No licensed script has the text.
    pub const REEF_THANKS: &str = "reef.thanks";

    /// To Tristan's Tomb (quest 2245, `q2245s`): Mu Young (NPC 1061014)
    /// turns a player away while another is in the Jr. Balrog's room. No
    /// licensed script has the text.
    pub const TRISTAN_ROOM_TAKEN: &str = "tristan.room_taken";

    /// Dollhouse (quest 3230) in Eos Tower: Olson the Toy Soldier (NPC
    /// 2040002) with the hint and the way into the dollhouse room, or taken
    /// by another player; a wrong dollhouse (reactor 2200000) throwing the
    /// player out; and Mark the Toy Soldier (NPC 2040028) taking the
    /// pendulum, or still waiting for it, before the way back. No licensed
    /// script has the text.
    pub const DOLLHOUSE_HINT: &str = "dollhouse.hint";
    pub const DOLLHOUSE_ENTER: &str = "dollhouse.enter";
    pub const DOLLHOUSE_BUSY: &str = "dollhouse.busy";
    pub const DOLLHOUSE_WRONG: &str = "dollhouse.wrong";
    pub const DOLLHOUSE_FOUND: &str = "dollhouse.found";
    pub const DOLLHOUSE_NOT_FOUND: &str = "dollhouse.not_found";
    pub const DOLLHOUSE_BACK: &str = "dollhouse.back";

    /// The Missing Mechanical Parts (quest 3239) in the Toy Factory: Cheng
    /// the Assistant (NPC 2040003) with the way into Sector 4, or taken by
    /// another player; and in it, taking ten Machine Parts for a scroll, or
    /// asking for a free USE slot first, and the way out. No licensed script
    /// has the text.
    pub const MECHANICAL_PARTS_ENTER: &str = "mechanical_parts.enter";
    pub const MECHANICAL_PARTS_BUSY: &str = "mechanical_parts.busy";
    pub const MECHANICAL_PARTS_DONE: &str = "mechanical_parts.done";
    pub const MECHANICAL_PARTS_NO_ROOM: &str = "mechanical_parts.no_room";
    pub const MECHANICAL_PARTS_LEAVE: &str = "mechanical_parts.leave";

    /// King Pepe's room in the Mushroom Castle: the tower's way in
    /// (`TD_MC_enterboss1`) while another player is in it, and its way out
    /// (`out_pepeking`) giving the Wedding Hall key once the three Yeti
    /// brothers are beaten, or shut with no room for it. No licensed script
    /// has the text.
    pub const KING_PEPE_BUSY: &str = "king_pepe.busy";
    pub const KING_PEPE_KEY: &str = "king_pepe.key";
    pub const KING_PEPE_KEY_NO_ROOM: &str = "king_pepe.key_no_room";

    /// The Mushroom Castle's Wedding Hall: its door (`TD_MC_enterboss2`)
    /// locked before the key's quest or taken by another player, and
    /// Violetta's quest scripts (`q2332e`, `q2333s`, `q2333e`, `q2334s`,
    /// `q2342s`). No licensed script has the text.
    pub const WEDDING_HALL_LOCKED: &str = "wedding_hall.locked";
    pub const WEDDING_HALL_BUSY: &str = "wedding_hall.busy";
    pub const VIOLETTA_FOUND: &str = "violetta.found";
    pub const VIOLETTA_HELP: &str = "violetta.help";
    pub const VIOLETTA_PRIME_MINISTER: &str = "violetta.prime_minister";
    pub const VIOLETTA_PRIME_MINISTER_BEATEN: &str = "violetta.prime_minister_beaten";
    pub const VIOLETTA_FACE: &str = "violetta.face";
    pub const VIOLETTA_SEAL: &str = "violetta.seal";
    pub const VIOLETTA_SEAL_FULL: &str = "violetta.seal_full";
    pub const VIOLETTA_SEAL_KEPT: &str = "violetta.seal_kept";
    pub const VIOLETTA_MINIONS: &str = "violetta.minions";
    pub const VIOLETTA_KEY: &str = "violetta.key";
    pub const VIOLETTA_KEY_LUCK: &str = "violetta.key_luck";
    pub const VIOLETTA_KEY_FULL: &str = "violetta.key_full";

    /// Snowman's Rage: the Spirit of Snow Statue (NPC 2020012, `q3108s`),
    /// the broken statue where the clue is found.
    pub const SNOW_STATUE_CLUE: &str = "snow_statue.clue";

    /// In Search for the Lost Memory (3523-3527, 3529): a job instructor
    /// gives back the memory, and the Memory Keeper has the pass.
    pub const LOST_MEMORY_REGAINED: &str = "lost_memory.regained";

    /// The Cygnus Knights' black shadows (20706, 20718): a place with
    /// nothing, the Pig Farm's shadow, and the hidden Magic Library's,
    /// whose way out waits for its Blue Mushrooms to fall.
    pub const BLACK_SHADOW_NOTHING: &str = "black_shadow.nothing";
    pub const BLACK_SHADOW_SPOTTED: &str = "black_shadow.spotted";
    pub const BLACK_SHADOW_AMBUSH: &str = "black_shadow.ambush";
    pub const MAGIC_LIBRARY_MUSHROOMS_LEFT: &str = "portal.outMagiclib.mushrooms_left";

    /// The Puppeteer's door (1063011): its password prompt, a wrong one, a
    /// right one from someone not hunting him (pink), and an Aran's search
    /// on The Puppeteer's Cave (21728); Francis (1104000) catching the
    /// intruder, and his hideout taken.
    pub const PUPPETEER_DOOR_ASK: &str = "puppeteer_door.ask";
    pub const PUPPETEER_DOOR_WRONG: &str = "puppeteer_door.wrong";
    pub const PUPPETEER_DOOR_SHUT: &str = "puppeteer_door.shut";
    pub const PUPPETEER_DOOR_SEARCHED: &str = "puppeteer_door.searched";
    pub const FRANCIS_INTRUDER: &str = "francis.intruder";
    pub const FRANCIS_HIDEOUT_BUSY: &str = "francis.hideout_busy";

    /// "Gathering Some Strange Information" (21733): the Puppeteer's call
    /// in Tru's name (1204007, `q21733s`), and Tru (1002104, `q21733e`) on
    /// the trap once the Puppeteer is beaten, then teaching Polearm Mastery.
    pub const FAKE_TRU_CALL: &str = "fake_tru.call";
    pub const TRU_PUPPETEER_BEATEN: &str = "tru.puppeteer_beaten";
    pub const TRU_POLEARM_MASTERY: &str = "tru.polearm_mastery";

    /// The Aran's trail from the Puppeteer to Orbis (21734-21736): Tru
    /// (1002104) sends them down the Secret Passage (`q21734s`) and, the
    /// Puppeteer beaten, teaches Combo Drain (`q21734e`), hands over the
    /// Seal Stone of Victoria Island for Lilin (`q21735s`), and sends them
    /// to Orbis (`q21736s`). The passage's statues (1063017, 1063016) lead
    /// into the Puppeteer's arena and back out.
    pub const TRU_PUPPETEER_TRACES: &str = "tru.puppeteer_traces";
    pub const TRU_COMBO_DRAIN: [&str; 2] = ["tru.puppeteer_trail_beaten", "tru.combo_drain"];
    pub const TRU_SEAL_STONE: [&str; 2] = ["tru.seal_stone.news", "tru.seal_stone.given"];
    pub const TRU_SEAL_STONE_FULL: &str = "tru.seal_stone.full";
    /// Lilin (1201000) taking the stone (`q21735e`).
    pub const LILIN_SEAL_STONE: &str = "lilin.seal_stone";
    pub const TRU_ORBIS: [&str; 3] = ["tru.orbis.news", "tru.orbis.ask", "tru.orbis.go"];
    pub const TRU_ORBIS_DECLINED: &str = "tru.orbis.declined";
    pub const PASSAGE_STATUE_ASK: &str = "passage_statue.ask";
    pub const PASSAGE_STATUE_BUSY: &str = "passage_statue.busy";
    pub const PASSAGE_EXIT_ASK: &str = "passage_exit.ask";

    /// The Aran's trail from Orbis to Mu Lung (21738-21742): Spiruna
    /// (2032001) on the Sealed Garden (`q21738s`) and its guardian beaten
    /// (`q21739e`), Tru on the Orbis seal stone (`q21740s`), Lilin
    /// (1201000) teaching Combo Smash (`q21740e`), Tru sending the Aran to
    /// Mu Lung (`q21741s`), and Mr. Do (2090004) on the soaked Hanging
    /// Scroll (`q21742s`) and its writing restored (`q21742e`).
    pub const SPIRUNA_SEALED_GARDEN: [&str; 2] =
        ["spiruna.sealed_garden.news", "spiruna.sealed_garden.ask"];
    pub const SPIRUNA_SEALED_GARDEN_DECLINED: &str = "spiruna.sealed_garden.declined";
    pub const SPIRUNA_GARDEN_CLEARED: &str = "spiruna.garden_cleared";
    pub const TRU_ORBIS_STONE_STOLEN: &str = "tru.orbis_stone_stolen";
    pub const LILIN_COMBO_SMASH: [&str; 2] = ["lilin.orbis_stone", "lilin.combo_smash"];
    pub const TRU_MU_LUNG: [&str; 3] =
        ["tru.mu_lung.news", "tru.mu_lung.ask", "tru.mu_lung.keyword"];
    pub const TRU_MU_LUNG_DECLINED: &str = "tru.mu_lung.declined";
    pub const MR_DO_BLACK_SCROLL: [&str; 2] = ["mr_do.black_scroll.met", "mr_do.black_scroll.ask"];
    pub const MR_DO_SCROLL_SOAKED: [&str; 2] =
        ["mr_do.black_scroll.soaked", "mr_do.black_scroll.jin_jin"];
    pub const MR_DO_SCROLL_FULL: &str = "mr_do.black_scroll.full";
    pub const MR_DO_SCROLL_READ: &str = "mr_do.black_scroll.read";

    /// The Aran's trail from Mu Lung to Ereve (21744-21757): the Dojo's
    /// back alley (`enterBackStreet`), Mu Gong (2091007) and the Test
    /// (`q21746s`) and the Shadow Knight (`q21747s`, `q21747e`), the Sealed
    /// Shrine's pillar (2091009), Lilin teaching Final Charge (`q21748e`),
    /// Tru on the way into the past (`q21749s`), Athena Pierce then
    /// (2131000, `q21750e`, `q21753s`) and now (1012100, `q21754s`), and
    /// Neinheart (1101002) taking Lilin's letter (`q21757e`).
    pub const BACK_STREET_NO_ENTRY: &str = "portal.enterBackStreet.no_entry";
    pub const MU_GONG_TEST: &str = "mu_gong.test";
    pub const MU_GONG_ARENA_BUSY: &str = "mu_gong.arena_busy";
    pub const MU_GONG_SHADOW_KNIGHT: [&str; 2] = [
        "mu_gong.shadow_knight.ask",
        "mu_gong.shadow_knight.password",
    ];
    pub const MU_GONG_STONE_TAKEN: [&str; 2] =
        ["mu_gong.stone_taken.news", "mu_gong.stone_taken.tru"];
    pub const SHRINE_PILLAR_ASK: &str = "shrine_pillar.ask";
    pub const SHRINE_PILLAR_WRONG: &str = "shrine_pillar.wrong";
    pub const SHRINE_PILLAR_SHUT: &str = "shrine_pillar.shut";
    pub const SHRINE_PILLAR_BUSY: &str = "shrine_pillar.busy";
    pub const LILIN_FINAL_CHARGE: [&str; 2] = ["lilin.mu_lung_stone", "lilin.final_charge"];
    pub const TRU_BACK_TO_THE_PAST: [&str; 2] =
        ["tru.back_to_the_past.news", "tru.back_to_the_past.gate"];
    pub const ATHENA_REMEMBERS: [&str; 2] = ["athena.remembers.aran", "athena.remembers.sorry"];
    pub const ATHENA_LETTER_PROMISE: [&str; 2] = [
        "athena.letter_promise.letter",
        "athena.letter_promise.your_time",
    ];
    pub const ATHENA_LETTER: &str = "athena.letter";
    pub const ATHENA_LETTER_FULL: &str = "athena.letter.full";
    pub const NEINHEART_LETTER: &str = "neinheart.letter";

    /// The Aran's first steps and errands that run on scripts: Lilin
    /// (`q21700s`, `q21704s`, `q21720e`), Puo (1202006, `q21703s`/`e`),
    /// Bruce and Rowen reporting to Tru (`q21712s`, `q21716s`), the
    /// Puppeteer's invitation (1204000, `q21719s`) and Francis in the
    /// Puppeteer's Cave (1204001), Ilji (`q21729s`), and Teo, John and
    /// John's box (`q21766s`/`e`, `q21767s`).
    pub const LILIN_NEW_BEGINNINGS: [&str; 4] = [
        "lilin.new_beginnings.remember",
        "lilin.new_beginnings.how",
        "lilin.new_beginnings.ask",
        "lilin.new_beginnings.go",
    ];
    pub const LILIN_NEW_BEGINNINGS_DECLINED: &str = "lilin.new_beginnings.declined";
    pub const LILIN_NEW_BEGINNINGS_FULL: &str = "lilin.new_beginnings.full";
    pub const PUO_TARGET_PIGS: [&str; 3] = [
        "puo.target_pigs.lesson",
        "puo.target_pigs.ask",
        "puo.target_pigs.go",
    ];
    pub const PUO_TARGET_PIGS_DECLINED: &str = "puo.target_pigs.declined";
    pub const PUO_COMBO_ABILITY: [&str; 4] = [
        "puo.combo_ability.proud",
        "puo.combo_ability.ask",
        "puo.combo_ability.remembered",
        "puo.combo_ability.lilin",
    ];
    pub const PUO_COMBO_ABILITY_FULL: &str = "puo.combo_ability.full";
    pub const LILIN_BABY_STEPS: [&str; 3] = [
        "lilin.baby_steps.training",
        "lilin.baby_steps.combo",
        "lilin.baby_steps.body",
    ];
    pub const BRUCE_FIRST_ASSIGNMENT: [&str; 4] = [
        "bruce.first_assignment.puppet",
        "bruce.first_assignment.ask",
        "bruce.first_assignment.planned",
        "bruce.first_assignment.report",
    ];
    pub const ROWEN_SECOND_ASSIGNMENT: [&str; 4] = [
        "rowen.second_assignment.tree",
        "rowen.second_assignment.ask",
        "rowen.second_assignment.suspicious",
        "rowen.second_assignment.report",
    ];
    pub const ASSIGNMENT_DONE_DECLINED: &str = "aran.assignment_done.declined";
    pub const SHADOW_INVITATION: [&str; 3] = [
        "shadow.invitation.found",
        "shadow.invitation.who",
        "shadow.invitation.ask",
    ];
    pub const SHADOW_INVITATION_DECLINED: &str = "shadow.invitation.declined";
    pub const CAVE_FRANCIS_WARNING: [&str; 2] = ["francis.cave.warning", "francis.cave.thought"];
    pub const CAVE_FRANCIS_OUT: &str = "francis.cave.out";
    pub const LILIN_POLEARM_BOOSTER: [&str; 6] = [
        "lilin.polearm_booster.message",
        "lilin.polearm_booster.black_wings",
        "lilin.polearm_booster.prophecy",
        "lilin.polearm_booster.ask",
        "lilin.polearm_booster.remembered",
        "lilin.polearm_booster.train",
    ];
    pub const ILJI_STATUE: &str = "ilji.statue";
    pub const TEO_SUSPICIOUS_JOHN: [&str; 2] =
        ["teo.suspicious_john.favour", "teo.suspicious_john.where"];
    pub const TEO_JOHN_CLEARED: [&str; 2] = ["teo.john_cleared.ask", "teo.john_cleared.made_up"];
    pub const JOHNS_BOX_MEDICINE: &str = "johns_box.medicine";
    pub const JOHNS_BOX_FULL: &str = "johns_box.full";

    /// An Aran's wolf: Pucci's request (`q21600s`), Scadur's saddle
    /// (`q21604s`), the end of the Field of Wolves (`clearRider`), the
    /// pack's test (`q21613s`, and its retake) and Maha's evolution
    /// (`q21618s`, `q21618e`).
    pub const PUCCI_WOLF_REQUEST: [&str; 3] = [
        "pucci.wolf_request.strong",
        "pucci.wolf_request.ask",
        "pucci.wolf_request.nanuke",
    ];
    pub const PUCCI_WOLF_REQUEST_DECLINED: &str = "pucci.wolf_request.declined";
    pub const SCADUR_WOLF_MOUNT: [&str; 2] =
        ["scadur.wolf_mount.saddle", "scadur.wolf_mount.hides"];
    pub const FIELD_OF_WOLVES_CLEARED: &str = "field_of_wolves.cleared";
    pub const WOLF_TEST: [&str; 3] = ["wolf.test.pup", "wolf.test.friend", "wolf.test.ask"];
    pub const WOLF_TEST_DECLINED: &str = "wolf.test.declined";
    pub const WOLF_TEST_BUSY: &str = "wolf.test.busy";
    pub const WOLF_TEST_AGAIN: &str = "wolf.test.again";
    pub const MAHA_WOLF_EVOLUTION: [&str; 3] = [
        "maha.wolf_evolution.power",
        "maha.wolf_evolution.can_you",
        "maha.wolf_evolution.ask",
    ];
    pub const MAHA_WOLF_EVOLUTION_UNEQUIP: &str = "maha.wolf_evolution.unequip";
    pub const MAHA_WOLF_EVOLUTION_DONE: &str = "maha.wolf_evolution.done";
    pub const MAHA_WOLF_EVOLUTION_FULL: &str = "maha.wolf_evolution.full";
    /// "Life Alchemy, and the Missing Alchemist" (3314): the desk with
    /// Russellon's pills (2111015) and Russellon (2111009, `q3314e`).
    pub const RUSSELLON_PILL_TAKEN: &str = "russellon.pill.taken";
    pub const RUSSELLON_PILL_FULL: &str = "russellon.pill.full";
    pub const RUSSELLON_PILL_NOT_TAKEN: &str = "russellon.pill.not_taken";
    pub const RUSSELLON_PILL_DONE: &str = "russellon.pill.done";
    pub const RUSSELLON_REWARD_FULL: &str = "russellon.reward.full";
    /// "Yulete's Reward" (3382): Yulete (2112014, `q3382e`) and the marbles.
    pub const YULETE_MARBLES_EYE: &str = "yulete.marbles.eye";
    pub const YULETE_MARBLES_ROCK: &str = "yulete.marbles.rock";
    pub const YULETE_MARBLES_MORE: &str = "yulete.marbles.more";
    pub const YULETE_MARBLES_FULL: &str = "yulete.marbles.full";
    /// "Stealing Queen's Order of Silk" (3941): Karcasa (2101013, `q3941s`,
    /// `q3941e`), who gives the queen's silk only to Tigun.
    pub const KARCASA_SILK_STRANGER: &str = "karcasa.silk.stranger";
    pub const KARCASA_SILK_TIGUN: &str = "karcasa.silk.tigun";
    pub const KARCASA_SILK_HANDED: &str = "karcasa.silk.handed";
    pub const KARCASA_SILK_GIVEN: &str = "karcasa.silk.given";
    pub const KARCASA_SILK_FULL: &str = "karcasa.silk.full";
    /// "Lost Spirits" (8255): Adonis (9201106, `q8255s`) on the Zakum
    /// Diamonds.
    pub const ADONIS_SPIRITS_TALE: &str = "adonis.spirits.tale";
    pub const ADONIS_SPIRITS_RITUAL: &str = "adonis.spirits.ritual";
    /// "The Seed That Swallow Lost" (3609): Swallow (2071006) gives a new
    /// Magic Seed.
    pub const SWALLOW_NEW_SEED: &str = "swallow.new_seed";
    pub const SWALLOW_NEW_SEED_FULL: &str = "swallow.new_seed_full";
    pub const PIA_SHELL_WISH: &str = "pia.shell.wish";
    pub const PIA_SHELL_MISSING: &str = "pia.shell.missing";
    /// Kerning Square's 7th floor: June (1052125, `Depart_topFloorIn`) keeps
    /// the way up to the Spirit of Rock's floors and the VIP Zone.
    pub const JUNE_MENU: &str = "june.menu";
    pub const JUNE_HELPING_BLAKE: &str = "june.helping_blake";
    pub const JUNE_VIP: &str = "june.vip";
    pub const JUNE_NOT_HELPING: &str = "june.not_helping";
    pub const JUNE_BUSY: &str = "june.busy";
    pub const JUNE_VIP_OVER_LEVEL: &str = "june.vip.over_level";
    pub const JUNE_VIP_INVITATIONS: &str = "june.vip.invitations";
    pub const JUNE_NOT_VIP: &str = "june.not_vip";
    /// Admission to the VIP Zone (2291, `q2291e`).
    pub const JUNE_VIP_MISSING: &str = "june.vip.missing";
    pub const JUNE_VIP_WELCOME: &str = "june.vip.welcome";
    /// The Spirit of Rock's floors: a way on that stays shut.
    pub const DEPART_NO_ACCESS: &str = "depart.no_access";

    /// "A Cute Baby Bird" (3250): Ghosthunter Bob (2041026, `q3250s`)
    /// offers a Timer's Egg to raise, then, once it has grown
    /// (`q3250e`), takes it back.
    pub const BOB_BABY_BIRD_OFFER: [&str; 3] = [
        "bob.baby_bird.feeding",
        "bob.baby_bird.tamed",
        "bob.baby_bird.ask",
    ];
    pub const BOB_BABY_BIRD_DECLINED: &str = "bob.baby_bird.declined";
    pub const BOB_BABY_BIRD_GIVEN: [&str; 2] = ["bob.baby_bird.given", "bob.baby_bird.return"];
    pub const BOB_BABY_BIRD_FULL: &str = "bob.baby_bird.full";
    pub const BOB_BABY_BIRD_GROWN: &str = "bob.baby_bird.grown";

    /// Captain Al's lessons in leading a family (quests 2232-2234), each
    /// ended by his script (NPC 1002103, `q2232e`-`q2234e`): done, then not
    /// yet, for a junior taken, 1,000 Rep earned, and 2,000 earned with the
    /// Rep to spend used down below 500. No licensed script has the text,
    /// so these keep their placeholders.
    pub const CAPTAIN_AL_JUNIOR: [&str; 2] = [
        "family.captain_al.junior.done",
        "family.captain_al.junior.not_yet",
    ];
    pub const CAPTAIN_AL_REP: [&str; 2] = [
        "family.captain_al.rep.done",
        "family.captain_al.rep.not_yet",
    ];
    pub const CAPTAIN_AL_ENTITLEMENT: [&str; 2] = [
        "family.captain_al.entitlement.done",
        "family.captain_al.entitlement.not_yet",
    ];

    /// Trainer Bartos's Follow the Lead (quests 4646 and 4647): the hidden
    /// note of his test (NPC 9102100, `multipet_success`), then his end
    /// script `q4647e`, which takes a Pet Snack.
    pub const PET_NOTE_ASK: &str = "pet_trainer.note.ask";
    pub const PET_NOTE_FULL: &str = "pet_trainer.note.full";
    pub const PET_NOTE_HELD: &str = "pet_trainer.note.held";
    pub const PET_NOTE_NOTHING: &str = "pet_trainer.note.nothing";
    pub const PET_SECRET: [&str; 2] = ["pet_trainer.secret.0", "pet_trainer.secret.1"];
    pub const PET_SECRET_NO_SNACK: &str = "pet_trainer.secret.no_snack";

    /// Garnox's Rock of Evolution scripts (quests 8185, 4659 and 8189).
    pub const PET_EVOLUTION_ASK: &str = "pet_evolution.ask";
    pub const PET_EVOLUTION_DECLINE: &str = "pet_evolution.decline";
    pub const PET_EVOLUTION_DONE: &str = "pet_evolution.done";
    pub const PET_EVOLUTION_NO_MESOS: &str = "pet_evolution.no_mesos";
    pub const PET_EVOLUTION_NO_PET: &str = "pet_evolution.no_pet";
    pub const PET_EVOLUTION_REFUSED: &str = "pet_evolution.refused";

    /// A Gachapon machine (report c058): the question to a player with a
    /// ticket, the prize, the menu of explanations without one, and the
    /// refusals.
    pub const GACHAPON_ASK: &str = "gachapon.ask";
    pub const GACHAPON_WON: &str = "gachapon.won";
    pub const GACHAPON_FULL: &str = "gachapon.full";
    pub const GACHAPON_NO_TICKET: &str = "gachapon.no_ticket";
    pub const GACHAPON_MENU: &str = "gachapon.menu";
    pub const GACHAPON_WHAT_CHOICE: &str = "gachapon.what.choice";
    pub const GACHAPON_WHAT: &str = "gachapon.what";
    pub const GACHAPON_WHERE_CHOICE: &str = "gachapon.where.choice";
    pub const GACHAPON_WHERE: &str = "gachapon.where";

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

    /// To a proposer: one of the two is below the level to be engaged.
    pub const MARRIAGE_LEVEL: &str = "marriage.level";

    /// Moony's menu.
    pub const MOONY_MENU: &str = "moony.menu";

    /// Moony's choice to make a ring.
    pub const MOONY_MAKE: &str = "moony.make";

    /// Moony's choice to throw away ring boxes.
    pub const MOONY_DISCARD: &str = "moony.discard";

    /// Moony sends the player for their parents' blessing.
    pub const MOONY_NEED_BLESSING: &str = "moony.need_blessing";

    /// Moony, while the blessing is still sought.
    pub const MOONY_SEEKING: &str = "moony.seeking";

    /// Moony to a player who already holds a ring box.
    pub const MOONY_HAVE_BOX: &str = "moony.have_box";

    /// Moony to a woman who asks for a ring.
    pub const MOONY_GROOM_ONLY: &str = "moony.groom_only";

    /// Moony's choice of rings.
    pub const MOONY_CHOOSE: &str = "moony.choose";

    /// Moony's price for a ring: {box}, {stone} and {diamond} are item ids, {price} mesos.
    pub const MOONY_ASK: &str = "moony.ask";

    /// Moony, when the player turns the price down.
    pub const MOONY_DECLINE: &str = "moony.decline";

    /// Moony to a player short of the fee.
    pub const MOONY_SHORT: &str = "moony.short";

    /// Moony to a player without the stones.
    pub const MOONY_NO_STONES: &str = "moony.no_stones";

    /// Moony, when the box doesn't fit.
    pub const MOONY_FULL: &str = "moony.full";

    /// Moony hands over the ring box.
    pub const MOONY_DONE: &str = "moony.done";

    /// Moony throws the player's ring boxes away.
    pub const MOONY_DISCARDED: &str = "moony.discarded";

    /// Moony to a player without a ring box to throw away.
    pub const MOONY_NO_BOX: &str = "moony.no_box";

    /// Mom and Dad hear the player out.
    pub const PARENTS_ASK_1: &str = "parents.ask_1";

    /// Mom and Dad agree.
    pub const PARENTS_ASK_2: &str = "parents.ask_2";

    /// Mom and Dad set the task.
    pub const PARENTS_ASK_3: &str = "parents.ask_3";

    /// Mom and Dad, until the player has four proofs.
    pub const PARENTS_REMIND: &str = "parents.remind";

    /// Mom and Dad see the proofs.
    pub const PARENTS_PROUD: &str = "parents.proud";

    /// Mom and Dad take the proofs and give their blessing.
    pub const PARENTS_BLESS: &str = "parents.bless";

    /// A Nana asks a favour: {item} is the drop's item id.
    pub const NANA_FAVOUR: &str = "nana.favour";

    /// A Nana, when the favour is turned down.
    pub const NANA_DECLINE: &str = "nana.decline";

    /// A Nana names her drop.
    pub const NANA_TASK: &str = "nana.task";

    /// A Nana to a player with fewer than 50 of her drop.
    pub const NANA_NEED: &str = "nana.need";

    /// A Nana gives her Proof of Love.
    pub const NANA_THANKS: &str = "nana.thanks";

    /// A Nana to a player who holds her proof.
    pub const NANA_HAVE_PROOF: &str = "nana.have_proof";

    /// A Nana offers her proof again for the drops.
    pub const NANA_AGAIN: &str = "nana.again";

    /// A Nana, when her proof doesn't fit.
    pub const NANA_FULL: &str = "nana.full";

    /// Moony, Mom and Dad, and the Nanas (Amoria's engagement rings).
    pub const ENGAGEMENT: [&str; 30] = [
        MOONY_MENU,
        MOONY_MAKE,
        MOONY_DISCARD,
        MOONY_NEED_BLESSING,
        MOONY_SEEKING,
        MOONY_HAVE_BOX,
        MOONY_GROOM_ONLY,
        MOONY_CHOOSE,
        MOONY_ASK,
        MOONY_DECLINE,
        MOONY_SHORT,
        MOONY_NO_STONES,
        MOONY_FULL,
        MOONY_DONE,
        MOONY_DISCARDED,
        MOONY_NO_BOX,
        PARENTS_ASK_1,
        PARENTS_ASK_2,
        PARENTS_ASK_3,
        PARENTS_REMIND,
        PARENTS_PROUD,
        PARENTS_BLESS,
        NANA_FAVOUR,
        NANA_DECLINE,
        NANA_TASK,
        NANA_NEED,
        NANA_THANKS,
        NANA_HAVE_PROOF,
        NANA_AGAIN,
        NANA_FULL,
    ];

    /// A wedding assistant's greeting, over her menu.
    pub const WEDDING_MENU: &str = "wedding.assist.menu";

    /// The assistant's choice to hear how a wedding is prepared.
    pub const WEDDING_CHOICE_PREPARE: &str = "wedding.assist.choice_prepare";

    /// The assistant's choice to book the wedding.
    pub const WEDDING_CHOICE_RESERVE: &str = "wedding.assist.choice_reserve";

    /// How a wedding is prepared: `{ticket}` and `{premium}` are the venue's tickets.
    pub const WEDDING_PREPARE: &str = "wedding.assist.prepare";

    /// The assistant, to a player who isn't engaged.
    pub const WEDDING_NO_RING: &str = "wedding.reserve.no_ring";

    /// The assistant, to a couple booked already: `{ticket}`, `{minutes}` and `{channel}` tell which wedding and when.
    pub const WEDDING_BOOKED: &str = "wedding.reserve.booked";

    /// The assistant, when the partner isn't online.
    pub const WEDDING_PARTNER_OFFLINE: &str = "wedding.reserve.partner_offline";

    /// The assistant, when the player has a wedding ring already.
    pub const WEDDING_HAS_RING: &str = "wedding.reserve.has_ring";

    /// The assistant, when the partner has a wedding ring already.
    pub const WEDDING_PARTNER_HAS_RING: &str = "wedding.reserve.partner_has_ring";

    /// The assistant, when the partner is online but not beside the player.
    pub const WEDDING_PARTNER_ELSEWHERE: &str = "wedding.reserve.partner_elsewhere";

    /// The assistant, when the invitations won't fit the player's Etc tab.
    pub const WEDDING_NO_ROOM: &str = "wedding.reserve.no_room";

    /// The assistant, when the invitations won't fit the partner's Etc tab.
    pub const WEDDING_PARTNER_NO_ROOM: &str = "wedding.reserve.partner_no_room";

    /// The assistant, without a ticket in the Cash tab: `{ticket}` and `{premium}` are the venue's tickets.
    pub const WEDDING_NO_TICKET: &str = "wedding.reserve.no_ticket";

    /// The assistant, when the booking failed.
    pub const WEDDING_ERROR: &str = "wedding.reserve.error";

    /// The assistant, having booked the wedding: `{ticket}`, `{minutes}` and `{channel}` as in `wedding.reserve.booked`.
    pub const WEDDING_DONE: &str = "wedding.reserve.done";

    /// The line both partners get when the wedding is booked, as in `wedding.reserve.done`.
    pub const WEDDING_DONE_CHAT: &str = "wedding.reserve.done_chat";

    /// Victoria, the Cathedral's coordinator: how a wedding is booked.
    pub const WEDDING_VICTORIA: &str = "wedding.victoria.info";

    /// The assistant's menu choice for a guest asking to go in.
    pub const WEDDING_CHOICE_GUEST: &str = "wedding.assist.choice_guest";

    /// The assistant's menu choice, with a Wedding Invitation Ticket in the Cash tab, for more invitations.
    pub const WEDDING_CHOICE_EXTRA: &str = "wedding.assist.choice_extra";

    /// The assistant, to a guest without an invitation to a wedding here.
    pub const WEDDING_GUEST_NOT_INVITED: &str = "wedding.guest.not_invited";

    /// The assistant, to a guest early: `{ticket}`, `{minutes}` and `{channel}` as in `wedding.reserve.booked`.
    pub const WEDDING_GUEST_EARLY: &str = "wedding.guest.early";

    /// The assistant, to a guest once the wedding is due.
    pub const WEDDING_GUEST_WAIT: &str = "wedding.guest.wait";

    /// The assistant, asked for more invitations by a couple without a wedding booked here.
    pub const WEDDING_EXTRA_NOT_BOOKED: &str = "wedding.extra.not_booked";

    /// The assistant, asked for more invitations once the wedding is due.
    pub const WEDDING_EXTRA_STARTED: &str = "wedding.extra.started";

    /// The assistant, having made more invitations: `{quantity}` of them.
    pub const WEDDING_EXTRA_DONE: &str = "wedding.extra.done";

    /// The line to a partner inviting someone who doesn't exist: `{name}`.
    pub const WEDDING_INVITE_NOT_FOUND: &str = "wedding.invite.not_found";

    /// The line to a partner inviting their own couple.
    pub const WEDDING_INVITE_COUPLE: &str = "wedding.invite.couple";

    /// The line to a partner inviting a guest twice: `{name}`.
    pub const WEDDING_INVITE_ALREADY: &str = "wedding.invite.already";

    /// The line to a partner whose wedding has its most guests.
    pub const WEDDING_INVITE_FULL: &str = "wedding.invite.full";

    /// The line to a partner sending an invitation for a wedding not booked at its venue.
    pub const WEDDING_INVITE_NOT_BOOKED: &str = "wedding.invite.not_booked";

    /// The line to a partner sending an invitation once the wedding is due.
    pub const WEDDING_INVITE_STARTED: &str = "wedding.invite.started";

    /// The line to a partner whose invitation failed.
    pub const WEDDING_INVITE_ERROR: &str = "wedding.invite.error";

    /// The line to a partner whose invitation went out: `{name}`.
    pub const WEDDING_INVITE_SENT: &str = "wedding.invite.sent";
    pub const WEDDING_INVITE_RETURNED: &str = "wedding.invite.returned";

    /// The line to a guest online who got an invitation: `{groom}`, `{bride}`, `{minutes}` and `{channel}`.
    pub const WEDDING_INVITED: &str = "wedding.guest.invited";

    /// The note and parcel message to a guest who got an invitation by Duey: `{groom}`, `{bride}` and `{channel}`.
    pub const WEDDING_INVITED_NOTE: &str = "wedding.guest.note";

    /// Wayne's line to anyone without a Chapel wedding booked.
    pub const WEDDING_STARTER_INTRO: &str = "wedding.start.intro";

    /// Wayne's line to an engaged player without a Chapel wedding booked.
    pub const WEDDING_STARTER_NOT_BOOKED: &str = "wedding.start.not_booked";

    /// Wayne's line to one of a couple whose wedding is on another channel: `{channel}`.
    pub const WEDDING_STARTER_CHANNEL: &str = "wedding.start.channel";

    /// Wayne's line to one of a couple whose wedding is not due yet: `{minutes}` and `{channel}`.
    pub const WEDDING_STARTER_EARLY: &str = "wedding.start.early";

    /// Wayne's line to one of a couple without wedding clothes.
    pub const WEDDING_STARTER_CLOTHES: &str = "wedding.start.clothes";

    /// Wayne asks one of a couple whether to begin their wedding.
    pub const WEDDING_STARTER_ASK: &str = "wedding.start.ask";

    /// Wayne's line when the partner is not beside the one who asked.
    pub const WEDDING_STARTER_PARTNER: &str = "wedding.start.partner";

    /// Wayne's line when another wedding is still under way at the Chapel.
    pub const WEDDING_STARTER_BUSY: &str = "wedding.start.busy";

    /// Wayne's line when the wedding could not begin.
    pub const WEDDING_STARTER_ERROR: &str = "wedding.start.error";

    /// The High Priest's line in Amoria to anyone without a Cathedral wedding booked.
    pub const WEDDING_CATHEDRAL_STARTER_INTRO: &str = "wedding.start.cathedral_intro";

    /// The High Priest's line in Amoria to an engaged player without a Cathedral wedding booked.
    pub const WEDDING_CATHEDRAL_STARTER_NOT_BOOKED: &str = "wedding.start.cathedral_not_booked";

    /// The High Priest in Amoria asks one of a couple whether to begin their wedding.
    pub const WEDDING_CATHEDRAL_STARTER_ASK: &str = "wedding.start.cathedral_ask";

    /// The priest's line to anyone but the couple.
    pub const WEDDING_PRIEST_INTRO: &str = "wedding.priest.intro";

    /// The priest asks one of the couple for their vows: `{partner}`.
    pub const WEDDING_PRIEST_ASK: &str = "wedding.priest.ask";

    /// The priest's line to one of the couple not wearing wedding clothes.
    pub const WEDDING_PRIEST_CLOTHES: &str = "wedding.priest.clothes";

    /// The priest's line to one of the couple without their engagement ring or box.
    pub const WEDDING_PRIEST_TOKEN: &str = "wedding.priest.token";

    /// The priest's line to one of the couple without room in the Equip tab.
    pub const WEDDING_PRIEST_ROOM: &str = "wedding.priest.room";

    /// The priest's line once one of the couple took their vows.
    pub const WEDDING_PRIEST_WAITING: &str = "wedding.priest.waiting";

    /// The priest's line to the married couple.
    pub const WEDDING_PRIEST_MARRIED: &str = "wedding.priest.married";

    /// The Cathedral's priest's line to the married couple.
    pub const WEDDING_CATHEDRAL_PRIEST_MARRIED: &str = "wedding.priest.cathedral_married";

    /// The line to a guest whose blessing of the couple counted.
    pub const WEDDING_BLESSED: &str = "wedding.blessed";

    /// The priest's line when the vows could not be taken.
    pub const WEDDING_PRIEST_FAILED: &str = "wedding.priest.failed";

    /// The altar staff's line to the couple before they are married.
    pub const WEDDING_STAFF_WAIT: &str = "wedding.staff.wait";

    /// The altar staff asks the married couple whether to end the wedding.
    pub const WEDDING_STAFF_ASK: &str = "wedding.staff.ask";

    /// The altar staff asks the couple of a premium wedding, once married, whether to go on to the party room.
    pub const WEDDING_STAFF_PARTY_ASK: &str = "wedding.staff.party_ask";

    /// Travis, in the party room, asks whether to leave the wedding for Amoria.
    pub const WEDDING_PARTY_QUIT_ASK: &str = "wedding.party.quit_ask";

    /// Robin, in the party room, asks whether to leave the wedding by its exit.
    pub const WEDDING_PARTY_EXIT_ASK: &str = "wedding.party.exit_ask";

    /// Robin's menu after a premium wedding, in the party room and the hunting ground.
    pub const WEDDING_ROBIN_MENU: &str = "wedding.robin.menu";

    /// Robin's choice, in the party room, of the hunting ground.
    pub const WEDDING_ROBIN_HUNT: &str = "wedding.robin.hunt";

    /// Robin's choice, in the hunting ground, of the box room for seven Heart Keys.
    pub const WEDDING_ROBIN_BOXES: &str = "wedding.robin.boxes";

    /// Robin's choice of leaving the wedding.
    pub const WEDDING_ROBIN_OUT: &str = "wedding.robin.out";

    /// Robin tells a player without seven Heart Keys how to get them.
    pub const WEDDING_ROBIN_NO_KEYS: &str = "wedding.robin.no_keys";

    /// Robin, in the box room, asks whether to go back to the hunting ground.
    pub const WEDDING_ROBIN_BACK_ASK: &str = "wedding.robin.back_ask";

    /// The couple's greeter in the hall asks whether to write their wishlist, which goes once.
    pub const WEDDING_WISHLIST_ASK: &str = "wedding.wishlist.ask";

    /// The greeter, to one of the couple whose wishlist is with Pila.
    pub const WEDDING_WISHLIST_SENT: &str = "wedding.wishlist.sent";

    /// The greeter, to a guest: Pila takes the gifts.
    pub const WEDDING_WISHLIST_GUEST: &str = "wedding.wishlist.guest";

    /// Pila, in a wedding, asks whose wishlist to give from.
    pub const WEDDING_PILA_MENU: &str = "wedding.pila.menu";

    /// Pila's choice of the groom's wishlist.
    pub const WEDDING_PILA_GROOM: &str = "wedding.pila.groom";

    /// Pila's choice of the bride's wishlist.
    pub const WEDDING_PILA_BRIDE: &str = "wedding.pila.bride";

    /// Pila, to one of the couple who chose their own wishlist.
    pub const WEDDING_PILA_OWN: &str = "wedding.pila.own";

    /// Pila, to a guest who has given that partner a gift at this wedding.
    pub const WEDDING_PILA_GIVEN: &str = "wedding.pila.given";

    /// Pila, outside a wedding, asks someone with gifts waiting whether to take them.
    pub const WEDDING_PILA_GIFTS: &str = "wedding.pila.gifts";

    /// Pila, to anyone else.
    pub const WEDDING_PILA_HELLO: &str = "wedding.pila.hello";

    /// The wedding's staff ask a guest whether to leave it.
    pub const WEDDING_STAFF_GUEST_LEAVE: &str = "wedding.staff.guest_leave";

    /// The assistant asks a guest whether to go into the wedding.
    pub const WEDDING_GUEST_ASK: &str = "wedding.guest.enter_ask";

    /// The assistant asks a guest without wedding clothes whether to go in, with a word on them.
    pub const WEDDING_GUEST_ASK_CLOTHES: &str = "wedding.guest.enter_clothes";

    /// The assistant, to a guest on another channel than the wedding's: `{channel}` as in `wedding.reserve.booked`.
    pub const WEDDING_GUEST_CHANNEL: &str = "wedding.guest.enter_channel";

    /// The assistant, to a guest once the ceremony has begun.
    pub const WEDDING_GUEST_LATE: &str = "wedding.guest.enter_late";

    /// The assistant, to a guest when the hall holds its most guests.
    pub const WEDDING_GUEST_FULL: &str = "wedding.guest.enter_full";

    /// The assistant, to a guest who could not go in.
    pub const WEDDING_GUEST_ERROR: &str = "wedding.guest.enter_error";

    /// The wedding itself: the starters, the priests and the altar staff (report b071).
    pub const CEREMONY: [&str; 40] = [
        WEDDING_STARTER_INTRO,
        WEDDING_STARTER_NOT_BOOKED,
        WEDDING_STARTER_CHANNEL,
        WEDDING_STARTER_EARLY,
        WEDDING_STARTER_CLOTHES,
        WEDDING_STARTER_ASK,
        WEDDING_STARTER_PARTNER,
        WEDDING_STARTER_BUSY,
        WEDDING_STARTER_ERROR,
        WEDDING_CATHEDRAL_STARTER_INTRO,
        WEDDING_CATHEDRAL_STARTER_NOT_BOOKED,
        WEDDING_CATHEDRAL_STARTER_ASK,
        WEDDING_PRIEST_INTRO,
        WEDDING_PRIEST_ASK,
        WEDDING_PRIEST_CLOTHES,
        WEDDING_PRIEST_TOKEN,
        WEDDING_PRIEST_ROOM,
        WEDDING_PRIEST_WAITING,
        WEDDING_PRIEST_MARRIED,
        WEDDING_CATHEDRAL_PRIEST_MARRIED,
        WEDDING_BLESSED,
        WEDDING_PRIEST_FAILED,
        WEDDING_STAFF_WAIT,
        WEDDING_STAFF_ASK,
        WEDDING_STAFF_PARTY_ASK,
        WEDDING_PARTY_QUIT_ASK,
        WEDDING_PARTY_EXIT_ASK,
        WEDDING_ROBIN_MENU,
        WEDDING_ROBIN_HUNT,
        WEDDING_ROBIN_BOXES,
        WEDDING_ROBIN_OUT,
        WEDDING_ROBIN_NO_KEYS,
        WEDDING_ROBIN_BACK_ASK,
        WEDDING_STAFF_GUEST_LEAVE,
        WEDDING_GUEST_ASK,
        WEDDING_GUEST_ASK_CLOTHES,
        WEDDING_GUEST_CHANNEL,
        WEDDING_GUEST_LATE,
        WEDDING_GUEST_FULL,
        WEDDING_GUEST_ERROR,
    ];

    /// Wishlists and gifts: the greeters and Pila (report b085).
    pub const GIFTS: [&str; 10] = [
        WEDDING_WISHLIST_ASK,
        WEDDING_WISHLIST_SENT,
        WEDDING_WISHLIST_GUEST,
        WEDDING_PILA_MENU,
        WEDDING_PILA_GROOM,
        WEDDING_PILA_BRIDE,
        WEDDING_PILA_OWN,
        WEDDING_PILA_GIVEN,
        WEDDING_PILA_GIFTS,
        WEDDING_PILA_HELLO,
    ];

    /// Wedding invitations: sending, opening, and more of them.
    pub const INVITATIONS: [&str; 19] = [
        WEDDING_CHOICE_GUEST,
        WEDDING_CHOICE_EXTRA,
        WEDDING_GUEST_NOT_INVITED,
        WEDDING_GUEST_EARLY,
        WEDDING_GUEST_WAIT,
        WEDDING_EXTRA_NOT_BOOKED,
        WEDDING_EXTRA_STARTED,
        WEDDING_EXTRA_DONE,
        WEDDING_INVITE_NOT_FOUND,
        WEDDING_INVITE_COUPLE,
        WEDDING_INVITE_ALREADY,
        WEDDING_INVITE_FULL,
        WEDDING_INVITE_NOT_BOOKED,
        WEDDING_INVITE_STARTED,
        WEDDING_INVITE_ERROR,
        WEDDING_INVITE_SENT,
        WEDDING_INVITE_RETURNED,
        WEDDING_INVITED,
        WEDDING_INVITED_NOTE,
    ];

    /// Nicole and Bonnie (wedding reservations).
    pub const RESERVATIONS: [&str; 17] = [
        WEDDING_MENU,
        WEDDING_CHOICE_PREPARE,
        WEDDING_CHOICE_RESERVE,
        WEDDING_PREPARE,
        WEDDING_NO_RING,
        WEDDING_BOOKED,
        WEDDING_PARTNER_OFFLINE,
        WEDDING_HAS_RING,
        WEDDING_PARTNER_HAS_RING,
        WEDDING_PARTNER_ELSEWHERE,
        WEDDING_NO_ROOM,
        WEDDING_PARTNER_NO_ROOM,
        WEDDING_NO_TICKET,
        WEDDING_ERROR,
        WEDDING_DONE,
        WEDDING_DONE_CHAT,
        WEDDING_VICTORIA,
    ];

    pub const ZAKUM_CHIEF_MENU: &str = "zakum.chief.menu";
    pub const ZAKUM_CHIEF_THIRD_JOB: &str = "zakum.chief.third_job";
    pub const ZAKUM_CHIEF_ZAKUM: &str = "zakum.chief.zakum";
    pub const ZAKUM_CHIEF_APPROVED: &str = "zakum.chief.approved";
    pub const ZAKUM_ADOBIS_UNAPPROVED: &str = "zakum.adobis.unapproved";
    pub const ZAKUM_ADOBIS_WARNING: &str = "zakum.adobis.warning";
    pub const ZAKUM_ADOBIS_MENU: &str = "zakum.adobis.menu";
    pub const ZAKUM_ADOBIS_LAVA_CHOICE: &str = "zakum.adobis.lava.choice";
    pub const ZAKUM_ADOBIS_LAVA_ASK: &str = "zakum.adobis.lava.ask";
    pub const ZAKUM_ADOBIS_LAVA_HAVE: &str = "zakum.adobis.lava.have";
    pub const ZAKUM_ADOBIS_LAVA_EARLIER: &str = "zakum.adobis.lava.earlier";
    pub const ZAKUM_ADOBIS_FORGE_CHOICE: &str = "zakum.adobis.forge.choice";
    pub const ZAKUM_ADOBIS_FORGE_LACK: &str = "zakum.adobis.forge.lack";
    pub const ZAKUM_ADOBIS_FORGE_SHORT: &str = "zakum.adobis.forge.short";
    pub const ZAKUM_ADOBIS_FORGE_DONE: &str = "zakum.adobis.forge.done";
    pub const ZAKUM_ADOBIS_FORGE_FULL: &str = "zakum.adobis.forge.full";
    pub const ZAKUM_LIRA_REWARD: &str = "zakum.lira.reward";
    pub const ZAKUM_LIRA_GO: &str = "zakum.lira.go";
    pub const ZAKUM_LIRA_FULL: &str = "zakum.lira.full";
    pub const ZAKUM_AMON_LEAVE: &str = "zakum.amon.leave";
    pub const ZAKUM_GATE_UNAPPROVED: &str = "zakum.gate.unapproved";
    pub const ZAKUM_GATE_TRIALS: &str = "zakum.gate.trials";
    pub const ZAKUM_GATE_NO_EYE: &str = "zakum.gate.no_eye";
    pub const ZAKUM_ADOBIS_MINE_CHOICE: &str = "zakum.adobis.mine.choice";
    pub const ZAKUM_ADOBIS_MINE_NO_PARTY: &str = "zakum.adobis.mine.no_party";
    pub const ZAKUM_ADOBIS_MINE_NOT_LEADER: &str = "zakum.adobis.mine.not_leader";
    pub const ZAKUM_ADOBIS_MINE_NOT_ELIGIBLE: &str = "zakum.adobis.mine.not_eligible";
    pub const ZAKUM_ADOBIS_MINE_BUSY: &str = "zakum.adobis.mine.busy";
    pub const ZAKUM_ADOBIS_REFUSED: &str = "zakum.adobis.refused";
    pub const ZAKUM_AURA_MENU: &str = "zakum.aura.menu";
    pub const ZAKUM_AURA_ABOUT_CHOICE: &str = "zakum.aura.about.choice";
    pub const ZAKUM_AURA_ABOUT: &str = "zakum.aura.about";
    pub const ZAKUM_AURA_HAND_IN_CHOICE: &str = "zakum.aura.hand_in.choice";
    pub const ZAKUM_AURA_LEADER_ONLY: &str = "zakum.aura.leader_only";
    pub const ZAKUM_AURA_NEED_ORE: &str = "zakum.aura.need_ore";
    pub const ZAKUM_AURA_ASK: &str = "zakum.aura.ask";
    pub const ZAKUM_AURA_ASK_DOCUMENTS: &str = "zakum.aura.ask_documents";
    pub const ZAKUM_AURA_OUT_CHOICE: &str = "zakum.aura.out.choice";
    pub const ZAKUM_AURA_OUT_ASK: &str = "zakum.aura.out.ask";
    pub const ZAKUM_AURA_WON: &str = "zakum.aura.won";
    pub const ZAKUM_AURA_PRIZE_FULL: &str = "zakum.aura.prize_full";
    pub const ZAKUM_AURA_PRIZE_TAKEN: &str = "zakum.aura.prize_taken";
    pub const ZAKUM_MINE_NOT_WON: &str = "zakum.mine.not_won";
    pub const ZAKUM_MINE_NO_PRIZE: &str = "zakum.mine.no_prize";
    pub const ZAKUM_MINE_UNKNOWN_FORCE: &str = "zakum.mine.unknown_force";
    pub const ZAKUM_MINE_MIMICS: &str = "zakum.mine.mimics";
    pub const ZAKUM_ALI_BYE: &str = "zakum.ali.bye";
    /// The pink line to Zakum's altar as the Eye of Fire brings Zakum out.
    pub const ZAKUM_ALTAR_SUMMONED: &str = "zakum.altar.summoned";
    /// The line to the whole world as Zakum's last body dies.
    pub const ZAKUM_VICTORY: &str = "zakum.victory";
    /// Adobis's expeditions at the Entrance to Zakum (`{name}` is a
    /// member), the notices only Zakum's have, and the gate shut behind
    /// one; the rest is every squad's (`SQUAD_*`).
    pub const ZAKUM_SQUAD_LEVEL: &str = "zakum.squad.level";
    pub const ZAKUM_SQUAD_NO_EYE: &str = "zakum.squad.no_eye";
    pub const ZAKUM_SQUAD_ASK: &str = "zakum.squad.ask";
    pub const ZAKUM_SQUAD_START_CHOICE: &str = "zakum.squad.start_choice";
    pub const ZAKUM_SQUAD_JOIN_ASK: &str = "zakum.squad.join_ask";
    pub const ZAKUM_SQUAD_REJOIN_ASK: &str = "zakum.squad.rejoin_ask";
    pub const ZAKUM_SQUAD_TAKEN: &str = "zakum.squad.taken";
    pub const ZAKUM_SQUAD_NOTICE_OPENED: &str = "zakum.squad.notice.opened";
    pub const ZAKUM_SQUAD_NOTICE_STARTED: &str = "zakum.squad.notice.started";
    pub const ZAKUM_GATE_BLOCKED: &str = "zakum.gate.blocked";
    /// What every squad's NPC says whatever its kind, and its notices
    /// (`{name}` is a member).
    pub const SQUAD_OPENED: &str = "squad.opened";
    pub const SQUAD_MENU: &str = "squad.menu";
    pub const SQUAD_EXPEL_CHOICE: &str = "squad.expel_choice";
    pub const SQUAD_ALONE: &str = "squad.alone";
    pub const SQUAD_LIST: &str = "squad.list";
    pub const SQUAD_EXPELLED: &str = "squad.expelled";
    pub const SQUAD_TOO_FEW: &str = "squad.too_few";
    pub const SQUAD_STOP_CHOICE: &str = "squad.stop_choice";
    pub const SQUAD_STOPPED: &str = "squad.stopped";
    pub const SQUAD_REGISTERED: &str = "squad.registered";
    pub const SQUAD_JOINED: &str = "squad.joined";
    pub const SQUAD_BANNED: &str = "squad.banned";
    pub const SQUAD_FULL: &str = "squad.full";
    pub const SQUAD_REFUSED: &str = "squad.refused";
    pub const SQUAD_NOTICE_JOINED: &str = "squad.notice.joined";
    pub const SQUAD_NOTICE_EXPELLED: &str = "squad.notice.expelled";
    pub const SQUAD_NOTICE_STOPPED: &str = "squad.notice.stopped";

    /// The way to Papulatus: the portal at Deep Inside the Clocktower
    /// that takes a party in (`{least}`/`{most}` are the team's size), the
    /// Tombstone beside it, the Machine Apparatus that lets them out, and
    /// the pink line as the Piece of Cracked Dimension brings out the Time
    /// Sphere.
    pub const PAPULATUS_NO_PARTY: &str = "papulatus.portal.no_party";
    pub const PAPULATUS_NOT_LEADER: &str = "papulatus.portal.not_leader";
    pub const PAPULATUS_NOT_ELIGIBLE: &str = "papulatus.portal.not_eligible";
    pub const PAPULATUS_BUSY: &str = "papulatus.portal.busy";
    pub const PAPULATUS_TOMBSTONE: &str = "papulatus.tombstone";
    pub const PAPULATUS_MACHINE_LEAVE: &str = "papulatus.machine.leave";
    pub const PAPULATUS_SUMMONED: &str = "papulatus.summoned";
    pub const PAPULATUS: [&str; 7] = [
        PAPULATUS_NO_PARTY,
        PAPULATUS_NOT_LEADER,
        PAPULATUS_NOT_ELIGIBLE,
        PAPULATUS_BUSY,
        PAPULATUS_TOMBSTONE,
        PAPULATUS_MACHINE_LEAVE,
        PAPULATUS_SUMMONED,
    ];
    pub const HORNTAIL_KEROBEN_AWAY: &str = "horntail.keroben.away";
    pub const HORNTAIL_KEROBEN_MENU: &str = "horntail.keroben.menu";
    pub const HORNTAIL_KEROBEN_BUY_CHOICE: &str = "horntail.keroben.buy_choice";
    pub const HORNTAIL_KEROBEN_ENTER_CHOICE: &str = "horntail.keroben.enter_choice";
    pub const HORNTAIL_KEROBEN_BOUGHT: &str = "horntail.keroben.bought";
    pub const HORNTAIL_KEROBEN_SHORT: &str = "horntail.keroben.short";
    pub const HORNTAIL_KEROBEN_FULL: &str = "horntail.keroben.full";
    pub const HORNTAIL_KEROBEN_LOW_LEVEL: &str = "horntail.keroben.low_level";
    pub const HORNTAIL_SLATE_ASK: &str = "horntail.slate.ask";
    pub const HORNTAIL_SLATE_SEALED: &str = "horntail.slate.sealed";
    pub const HORNTAIL_CRYSTAL_LEAVE: &str = "horntail.crystal.leave";
    /// The Mark of the Squad's expeditions at the Entrance to Horntail's
    /// Cave, and the notices only Horntail's have; the rest is every
    /// squad's (`SQUAD_*`).
    pub const HORNTAIL_SQUAD_LEVEL: &str = "horntail.squad.level";
    pub const HORNTAIL_SQUAD_ASK: &str = "horntail.squad.ask";
    pub const HORNTAIL_SQUAD_START_CHOICE: &str = "horntail.squad.start_choice";
    pub const HORNTAIL_SQUAD_JOIN_ASK: &str = "horntail.squad.join_ask";
    pub const HORNTAIL_SQUAD_REJOIN_ASK: &str = "horntail.squad.rejoin_ask";
    pub const HORNTAIL_SQUAD_TAKEN: &str = "horntail.squad.taken";
    pub const HORNTAIL_SQUAD_NOTICE_OPENED: &str = "horntail.squad.notice.opened";
    pub const HORNTAIL_SQUAD_NOTICE_STARTED: &str = "horntail.squad.notice.started";
    /// A trial's door, while its head lives.
    pub const HORNTAIL_TRIAL_SEALED: &str = "horntail.trial.sealed";
    pub const HORNTAIL_SUMMONED: &str = "horntail.summoned";
    pub const HORNTAIL_VICTORY: &str = "horntail.victory";
    /// Keroben, the Encrypted Slate of the Squad, the Crystal of Roots,
    /// the Mark of the Squad and the trials: the way to Horntail.
    pub const HORNTAIL: [&str; 22] = [
        HORNTAIL_KEROBEN_AWAY,
        HORNTAIL_KEROBEN_MENU,
        HORNTAIL_KEROBEN_BUY_CHOICE,
        HORNTAIL_KEROBEN_ENTER_CHOICE,
        HORNTAIL_KEROBEN_BOUGHT,
        HORNTAIL_KEROBEN_SHORT,
        HORNTAIL_KEROBEN_FULL,
        HORNTAIL_KEROBEN_LOW_LEVEL,
        HORNTAIL_SLATE_ASK,
        HORNTAIL_SLATE_SEALED,
        HORNTAIL_CRYSTAL_LEAVE,
        HORNTAIL_SQUAD_LEVEL,
        HORNTAIL_SQUAD_ASK,
        HORNTAIL_SQUAD_START_CHOICE,
        HORNTAIL_SQUAD_JOIN_ASK,
        HORNTAIL_SQUAD_REJOIN_ASK,
        HORNTAIL_SQUAD_TAKEN,
        HORNTAIL_SQUAD_NOTICE_OPENED,
        HORNTAIL_SQUAD_NOTICE_STARTED,
        HORNTAIL_TRIAL_SEALED,
        HORNTAIL_SUMMONED,
        HORNTAIL_VICTORY,
    ];

    pub const HTPQ_KEEPER_MENU: &str = "htpq.keeper.menu";
    pub const HTPQ_KEEPER_ENTER: &str = "htpq.keeper.enter";
    pub const HTPQ_KEEPER_ABOUT: &str = "htpq.keeper.about";
    pub const HTPQ_KEEPER_INFO: &str = "htpq.keeper.info";
    pub const HTPQ_KEEPER_NO_PARTY: &str = "htpq.keeper.no_party";
    pub const HTPQ_KEEPER_NOT_LEADER: &str = "htpq.keeper.not_leader";
    pub const HTPQ_KEEPER_NOT_ELIGIBLE: &str = "htpq.keeper.not_eligible";
    pub const HTPQ_KEEPER_BUSY: &str = "htpq.keeper.busy";
    pub const HTPQ_KEEPER_LEADER_ONLY: &str = "htpq.keeper.leader_only";
    pub const HTPQ_KEEPER_KEYS_SHORT: &str = "htpq.keeper.keys_short";
    pub const HTPQ_KEEPER_LAST_KEYS_SHORT: &str = "htpq.keeper.last_keys_short";
    pub const HTPQ_ROOMS_ALONE: &str = "htpq.rooms.alone";
    pub const HTPQ_ROOMS_HOLDING_KEY: &str = "htpq.rooms.holding_key";
    pub const HTPQ_SEALED: &str = "htpq.seal.sealed";
    pub const HTPQ_SEAL_LEADER_ONLY: &str = "htpq.seal.leader_only";
    pub const HTPQ_SEAL_LIFTED: &str = "htpq.seal.lifted";
    pub const HTPQ_KEY_SENT: &str = "htpq.key.sent";
    pub const HTPQ_KEY_APPEARED: &str = "htpq.key.appeared";
    pub const HTPQ_LAMP_HIT: &str = "htpq.lamp.hit";
    pub const HTPQ_LAMP_LEADER_ONLY: &str = "htpq.lamp.leader_only";
    /// The gatekeeper and the ways through Horntail's Trial Grounds.
    pub const HTPQ: [&str; 20] = [
        HTPQ_KEEPER_MENU,
        HTPQ_KEEPER_ENTER,
        HTPQ_KEEPER_ABOUT,
        HTPQ_KEEPER_INFO,
        HTPQ_KEEPER_NO_PARTY,
        HTPQ_KEEPER_NOT_LEADER,
        HTPQ_KEEPER_NOT_ELIGIBLE,
        HTPQ_KEEPER_BUSY,
        HTPQ_KEEPER_LEADER_ONLY,
        HTPQ_KEEPER_KEYS_SHORT,
        HTPQ_KEEPER_LAST_KEYS_SHORT,
        HTPQ_ROOMS_ALONE,
        HTPQ_ROOMS_HOLDING_KEY,
        HTPQ_SEALED,
        HTPQ_SEAL_LEADER_ONLY,
        HTPQ_SEAL_LIFTED,
        HTPQ_KEY_SENT,
        HTPQ_KEY_APPEARED,
        HTPQ_LAMP_HIT,
        HTPQ_LAMP_LEADER_ONLY,
    ];

    /// The Chief's Residence approval, Adobis's trials, Lira, Amon, and
    /// the gate to Zakum.
    pub const ZAKUM: [&str; 59] = [
        ZAKUM_CHIEF_MENU,
        ZAKUM_CHIEF_THIRD_JOB,
        ZAKUM_CHIEF_ZAKUM,
        ZAKUM_CHIEF_APPROVED,
        ZAKUM_ADOBIS_UNAPPROVED,
        ZAKUM_ADOBIS_WARNING,
        ZAKUM_ADOBIS_MENU,
        ZAKUM_ADOBIS_LAVA_CHOICE,
        ZAKUM_ADOBIS_LAVA_ASK,
        ZAKUM_ADOBIS_LAVA_HAVE,
        ZAKUM_ADOBIS_LAVA_EARLIER,
        ZAKUM_ADOBIS_FORGE_CHOICE,
        ZAKUM_ADOBIS_FORGE_LACK,
        ZAKUM_ADOBIS_FORGE_SHORT,
        ZAKUM_ADOBIS_FORGE_DONE,
        ZAKUM_ADOBIS_FORGE_FULL,
        ZAKUM_LIRA_REWARD,
        ZAKUM_LIRA_GO,
        ZAKUM_LIRA_FULL,
        ZAKUM_AMON_LEAVE,
        ZAKUM_GATE_UNAPPROVED,
        ZAKUM_GATE_TRIALS,
        ZAKUM_GATE_NO_EYE,
        ZAKUM_ADOBIS_MINE_CHOICE,
        ZAKUM_ADOBIS_MINE_NO_PARTY,
        ZAKUM_ADOBIS_MINE_NOT_LEADER,
        ZAKUM_ADOBIS_MINE_NOT_ELIGIBLE,
        ZAKUM_ADOBIS_MINE_BUSY,
        ZAKUM_ADOBIS_REFUSED,
        ZAKUM_AURA_MENU,
        ZAKUM_AURA_ABOUT_CHOICE,
        ZAKUM_AURA_ABOUT,
        ZAKUM_AURA_HAND_IN_CHOICE,
        ZAKUM_AURA_LEADER_ONLY,
        ZAKUM_AURA_NEED_ORE,
        ZAKUM_AURA_ASK,
        ZAKUM_AURA_ASK_DOCUMENTS,
        ZAKUM_AURA_OUT_CHOICE,
        ZAKUM_AURA_OUT_ASK,
        ZAKUM_AURA_WON,
        ZAKUM_AURA_PRIZE_FULL,
        ZAKUM_AURA_PRIZE_TAKEN,
        ZAKUM_MINE_NOT_WON,
        ZAKUM_MINE_NO_PRIZE,
        ZAKUM_MINE_UNKNOWN_FORCE,
        ZAKUM_MINE_MIMICS,
        ZAKUM_ALI_BYE,
        ZAKUM_ALTAR_SUMMONED,
        ZAKUM_VICTORY,
        ZAKUM_SQUAD_LEVEL,
        ZAKUM_SQUAD_NO_EYE,
        ZAKUM_SQUAD_ASK,
        ZAKUM_SQUAD_START_CHOICE,
        ZAKUM_SQUAD_JOIN_ASK,
        ZAKUM_SQUAD_REJOIN_ASK,
        ZAKUM_SQUAD_TAKEN,
        ZAKUM_SQUAD_NOTICE_OPENED,
        ZAKUM_SQUAD_NOTICE_STARTED,
        ZAKUM_GATE_BLOCKED,
    ];

    /// What every squad's NPC and notices say, whatever its kind.
    pub const SQUAD: [&str; 17] = [
        SQUAD_OPENED,
        SQUAD_MENU,
        SQUAD_EXPEL_CHOICE,
        SQUAD_ALONE,
        SQUAD_LIST,
        SQUAD_EXPELLED,
        SQUAD_TOO_FEW,
        SQUAD_STOP_CHOICE,
        SQUAD_STOPPED,
        SQUAD_REGISTERED,
        SQUAD_JOINED,
        SQUAD_BANNED,
        SQUAD_FULL,
        SQUAD_REFUSED,
        SQUAD_NOTICE_JOINED,
        SQUAD_NOTICE_EXPELLED,
        SQUAD_NOTICE_STOPPED,
    ];

    /// The magic rocks of Orbis Tower and Eos Tower; `{item}` is the
    /// scroll and `{map}` the floor.
    pub const TOWER_ROCK_NONE: &str = "tower_rock.none";
    pub const TOWER_ROCK_MENU: &str = "tower_rock.menu";
    pub const TOWER_ROCK_ASK: &str = "tower_rock.ask";
    /// The piles at the end of the Forests of Patience, with no room for
    /// what they give.
    pub const PATIENCE_FULL: &str = "patience.full";
    /// Line 3's construction sites: Jake's tickets (`{price}` in mesos),
    /// the Kerning City gate's choice of site, the box at each site's end
    /// with no room for what it gives, and the way out.
    pub const LINE3_JAKE_HELLO: &str = "line3.jake.hello";
    pub const LINE3_JAKE_MENU: &str = "line3.jake.menu";
    pub const LINE3_JAKE_SHORT: &str = "line3.jake.short";
    pub const LINE3_JAKE_FULL: &str = "line3.jake.full";
    pub const LINE3_GATE_MENU: &str = "line3.gate.menu";
    pub const LINE3_GATE_SQUARE: &str = "line3.gate.square";
    pub const LINE3_GATE_SITE: &str = "line3.gate.site";
    pub const LINE3_GATE_SUBWAY: &str = "line3.gate.subway";
    pub const LINE3_GATE_TICKETS: &str = "line3.gate.tickets";
    pub const LINE3_GATE_NO_TICKET: &str = "line3.gate.no_ticket";
    pub const LINE3_CHEST_FULL: &str = "line3.chest.full";
    pub const LINE3_EXIT_SAY: &str = "line3.exit.say";
    pub const LINE3_EXIT_ASK: &str = "line3.exit.ask";
    pub const GUILD_QUEST_MENU: &str = "guild_quest.menu";
    pub const GUILD_QUEST_REGISTER_CHOICE: &str = "guild_quest.register_choice";
    pub const GUILD_QUEST_JOIN_CHOICE: &str = "guild_quest.join_choice";
    pub const GUILD_QUEST_DETAILS_CHOICE: &str = "guild_quest.details_choice";
    pub const GUILD_QUEST_DETAILS: &str = "guild_quest.details";
    pub const GUILD_QUEST_NO_GUILD: &str = "guild_quest.no_guild";
    pub const GUILD_QUEST_NOT_MASTER: &str = "guild_quest.not_master";
    pub const GUILD_QUEST_FULL: &str = "guild_quest.full";
    pub const GUILD_QUEST_LISTED: &str = "guild_quest.listed";
    pub const GUILD_QUEST_ASK: &str = "guild_quest.ask";
    pub const GUILD_QUEST_REGISTERED: &str = "guild_quest.registered";
    pub const GUILD_QUEST_NOT_CALLED: &str = "guild_quest.not_called";
    pub const GUILD_QUEST_REFUSED: &str = "guild_quest.refused";
    pub const GUILD_QUEST_NURIS_ASK: &str = "guild_quest.nuris.ask";
    pub const GUILD_QUEST_BRIEF: &str = "guild_quest.brief";
    pub const GUILD_QUEST_EXIT_ASK: &str = "guild_quest.exit.ask";
    pub const GUILD_QUEST_STATUES_NOT_LEADER: &str = "guild_quest.statues.not_leader";
    pub const GUILD_QUEST_STATUES_CLEARED: &str = "guild_quest.statues.cleared";
    pub const GUILD_QUEST_STATUES_INTRO: &str = "guild_quest.statues.intro";
    pub const GUILD_QUEST_STATUES_HARDER: &str = "guild_quest.statues.harder";
    pub const GUILD_QUEST_STATUES_WORKING: &str = "guild_quest.statues.working";
    pub const GUILD_QUEST_STATUES_RIGHT: &str = "guild_quest.statues.right";
    pub const GUILD_QUEST_STATUES_WRONG: &str = "guild_quest.statues.wrong";
    pub const GUILD_QUEST_STATUES_WON: &str = "guild_quest.statues.won";
    pub const GUILD_QUEST_STATUES_SHOWING: &str = "guild_quest.statues.showing";
    pub const GUILD_QUEST_STATUES_SHOWN: &str = "guild_quest.statues.shown";
    pub const GUILD_QUEST_STATUES_PART_PASSED: &str = "guild_quest.statues.part_passed";
    pub const GUILD_QUEST_STATUES_FAILED: &str = "guild_quest.statues.failed";
    pub const GUILD_QUEST_VASSALS_NOT_LEADER: &str = "guild_quest.vassals.not_leader";
    pub const GUILD_QUEST_VASSALS_CLEARED: &str = "guild_quest.vassals.cleared";
    pub const GUILD_QUEST_VASSALS_INTRO: &str = "guild_quest.vassals.intro";
    pub const GUILD_QUEST_VASSALS_NO_OFFERING: &str = "guild_quest.vassals.no_offering";
    pub const GUILD_QUEST_VASSALS_RIGHT: &str = "guild_quest.vassals.right";
    pub const GUILD_QUEST_VASSALS_HINT: &str = "guild_quest.vassals.hint";
    pub const GUILD_QUEST_VASSALS_FAILED: &str = "guild_quest.vassals.failed";
    pub const GUILD_QUEST_SOUL_APPEARS: &str = "guild_quest.soul.appears";
    pub const GUILD_QUEST_SOUL_NOT_LEADER: &str = "guild_quest.soul.not_leader";
    pub const GUILD_QUEST_SOUL_OPENED: &str = "guild_quest.soul.opened";
    pub const GUILD_QUEST_SOUL_CLEARED: &str = "guild_quest.soul.cleared";
    pub const GUILD_QUEST_GATE_CLOSED: &str = "guild_quest.gate.closed";
    pub const GUILD_QUEST_GATE_BLOCKED: &str = "guild_quest.gate.blocked";
    pub const GUILD_QUEST_GATE_TOO_HIGH: &str = "guild_quest.gate.too_high";
    pub const GUILD_QUEST_GATE_CRACK: &str = "guild_quest.gate.crack";
    pub const GUILD_QUEST_RUBIAN_NOT_LEADER: &str = "guild_quest.rubian.not_leader";
    pub const GUILD_QUEST_RUBIAN_MISSING: &str = "guild_quest.rubian.missing";
    pub const AMORIA_PQ_HALL_MENU: &str = "amoria_pq.hall.menu";
    pub const AMORIA_PQ_HALL_ENTER: &str = "amoria_pq.hall.enter";
    pub const AMORIA_PQ_HALL_TRADE: &str = "amoria_pq.hall.trade";
    pub const AMORIA_PQ_HALL_ENTER_ASK: &str = "amoria_pq.hall.enter_ask";
    pub const AMORIA_PQ_HALL_NO_TICKET: &str = "amoria_pq.hall.no_ticket";
    pub const AMORIA_PQ_HALL_TRADE_ASK: &str = "amoria_pq.hall.trade_ask";
    pub const AMORIA_PQ_HALL_HAS_TICKET: &str = "amoria_pq.hall.has_ticket";
    pub const AMORIA_PQ_HALL_NO_KEYS: &str = "amoria_pq.hall.no_keys";
    pub const AMORIA_PQ_HALL_LATER: &str = "amoria_pq.hall.later";
    pub const AMORIA_PQ_HALL_NO_ROOM: &str = "amoria_pq.hall.no_room";
    pub const AMORIA_PQ_LOBBY_MENU: &str = "amoria_pq.lobby.menu";
    pub const AMORIA_PQ_LOBBY_ENTER: &str = "amoria_pq.lobby.enter";
    pub const AMORIA_PQ_LOBBY_LEAVE: &str = "amoria_pq.lobby.leave";
    pub const AMORIA_PQ_LOBBY_ABOUT: &str = "amoria_pq.lobby.about";
    pub const AMORIA_PQ_LOBBY_INFO: &str = "amoria_pq.lobby.info";
    pub const AMORIA_PQ_LOBBY_NO_PARTY: &str = "amoria_pq.lobby.no_party";
    pub const AMORIA_PQ_LOBBY_NOT_LEADER: &str = "amoria_pq.lobby.not_leader";
    pub const AMORIA_PQ_LOBBY_NOT_ELIGIBLE: &str = "amoria_pq.lobby.not_eligible";
    pub const AMORIA_PQ_LOBBY_BUSY: &str = "amoria_pq.lobby.busy";
    pub const AMORIA_PQ_EXIT_ASK: &str = "amoria_pq.exit.ask";
    pub const AMORIA_PQ_AMOS_CLEARED: &str = "amoria_pq.amos.cleared";
    pub const AMORIA_PQ_AMOS_NOT_LEADER: &str = "amoria_pq.amos.not_leader";
    pub const AMORIA_PQ_STAGE1_INTRO: &str = "amoria_pq.stage1.intro";
    pub const AMORIA_PQ_STAGE1_SEE_GLIMMER: &str = "amoria_pq.stage1.see_glimmer";
    pub const AMORIA_PQ_STAGE2_INTRO: &str = "amoria_pq.stage2.intro";
    pub const AMORIA_PQ_STAGE2_MOBS: &str = "amoria_pq.stage2.mobs";
    pub const AMORIA_PQ_STAGE2_FEW: &str = "amoria_pq.stage2.few";
    pub const AMORIA_PQ_STAGE2_ALL_DIFFER: &str = "amoria_pq.stage2.all_differ";
    pub const AMORIA_PQ_STAGE2_ALL_DIFFER_NOTICE: &str = "amoria_pq.stage2.all_differ_notice";
    pub const AMORIA_PQ_STAGE2_ONE_SAME: &str = "amoria_pq.stage2.one_same";
    pub const AMORIA_PQ_STAGE2_ONE_SAME_NOTICE: &str = "amoria_pq.stage2.one_same_notice";
    pub const AMORIA_PQ_STAGE2_RESET: &str = "amoria_pq.stage2.reset";
    pub const AMORIA_PQ_STAGE2_RESET_NOTICE: &str = "amoria_pq.stage2.reset_notice";
    pub const AMORIA_PQ_STAGE3_INTRO: &str = "amoria_pq.stage3.intro";
    pub const AMORIA_PQ_STAGE3_FEW: &str = "amoria_pq.stage3.few";
    pub const AMORIA_PQ_STAGE4_INTRO: &str = "amoria_pq.stage4.intro";
    pub const AMORIA_PQ_STAGE4_SHORT: &str = "amoria_pq.stage4.short";
    pub const AMORIA_PQ_STAGE4_DONE: &str = "amoria_pq.stage4.done";
    pub const AMORIA_PQ_STAGE4_NOTICE: &str = "amoria_pq.stage4.notice";
    pub const AMORIA_PQ_STAGE5_INTRO: &str = "amoria_pq.stage5.intro";
    pub const AMORIA_PQ_STAGE5_APART: &str = "amoria_pq.stage5.apart";
    pub const AMORIA_PQ_STAGE5_TELEPORTED: &str = "amoria_pq.stage5.teleported";
    pub const AMORIA_PQ_STAGE5_DONE: &str = "amoria_pq.stage5.done";
    pub const AMORIA_PQ_STAGE5_NOTICE: &str = "amoria_pq.stage5.notice";
    pub const AMORIA_PQ_STAGE5_GATE_SHUT: &str = "amoria_pq.stage5.gate_shut";
    pub const AMORIA_PQ_STAGE6_FANG: &str = "amoria_pq.stage6.fang";
    pub const AMORIA_PQ_STAGE6_DONE: &str = "amoria_pq.stage6.done";
    pub const AMORIA_PQ_BONUS_HURRY: &str = "amoria_pq.bonus.hurry";
    pub const CWK_SQUAD_LEVEL: &str = "cwk.squad.level";
    pub const CWK_SQUAD_ASK: &str = "cwk.squad.ask";
    pub const CWK_SQUAD_START_CHOICE: &str = "cwk.squad.start_choice";
    pub const CWK_SQUAD_JOIN_ASK: &str = "cwk.squad.join_ask";
    pub const CWK_SQUAD_REJOIN_ASK: &str = "cwk.squad.rejoin_ask";
    pub const CWK_SQUAD_TAKEN: &str = "cwk.squad.taken";
    pub const CWK_SQUAD_NOTICE_OPENED: &str = "cwk.squad.notice.opened";
    pub const CWK_SQUAD_NOTICE_STARTED: &str = "cwk.squad.notice.started";
    pub const CWK_GATE_NO_KEY: &str = "cwk.gate.no_key";
    pub const CWK_WAY_BLOCKED: &str = "cwk.way.blocked";
    pub const CWK_WAY_NOT_ACTIVATED: &str = "cwk.way.not_activated";
    pub const CWK_WAY_SHUT: &str = "cwk.way.shut";
    pub const CWK_STAGE1_FOUND: &str = "cwk.stage1.found";
    pub const CWK_SIGIL_WARRIOR: &str = "cwk.sigil.warrior";
    pub const CWK_SIGIL_ARCHER: &str = "cwk.sigil.archer";
    pub const CWK_SIGIL_MAGE: &str = "cwk.sigil.mage";
    pub const CWK_SIGIL_THIEF: &str = "cwk.sigil.thief";
    pub const CWK_SIGIL_PIRATE: &str = "cwk.sigil.pirate";
    pub const CWK_SIGILS_DONE: &str = "cwk.sigils.done";
    pub const CWK_MENHIRS_WARRIOR: &str = "cwk.menhirs.warrior";
    pub const CWK_MENHIRS_ARCHER: &str = "cwk.menhirs.archer";
    pub const CWK_MENHIRS_MAGE: &str = "cwk.menhirs.mage";
    pub const CWK_MENHIRS_THIEF: &str = "cwk.menhirs.thief";
    pub const CWK_MENHIRS_PIRATE: &str = "cwk.menhirs.pirate";
    pub const CWK_STEPS_TO_GO: &str = "cwk.steps.to_go";
    pub const CWK_STEPS_NOT_ALL: &str = "cwk.steps.not_all";
    pub const CWK_STEPS_OPEN: &str = "cwk.steps.open";
    pub const CWK_ROOM_ONLY_WARRIOR: &str = "cwk.room.only.warrior";
    pub const CWK_ROOM_ONLY_ARCHER: &str = "cwk.room.only.archer";
    pub const CWK_ROOM_ONLY_MAGE: &str = "cwk.room.only.mage";
    pub const CWK_ROOM_ONLY_THIEF: &str = "cwk.room.only.thief";
    pub const CWK_ROOM_ONLY_PIRATE: &str = "cwk.room.only.pirate";
    pub const CWK_ROOM_GIVEN: &str = "cwk.room.given";
    pub const CWK_ROOM_FULL: &str = "cwk.room.full";
    pub const CWK_ROOM_TAKEN: &str = "cwk.room.taken";
    pub const CWK_ROOM_GUARDIANS: &str = "cwk.room.guardians";
    pub const CWK_ROOM_MASTERS: &str = "cwk.room.masters";
    pub const CWK_ROOM_MOBS: &str = "cwk.room.mobs";
    pub const CWK_ROOM_EYES: &str = "cwk.room.eyes";
    pub const CWK_WEAPON_RESTORED: &str = "cwk.weapon.restored";
    pub const CWK_WEAPONS_DONE: &str = "cwk.weapons.done";
    pub const CWK_LORE_WARRIOR_HUB: &str = "cwk.lore.warrior.hub";
    pub const CWK_LORE_ARCHER_HUB: &str = "cwk.lore.archer.hub";
    pub const CWK_LORE_MAGE_HUB: &str = "cwk.lore.mage.hub";
    pub const CWK_LORE_THIEF_HUB: &str = "cwk.lore.thief.hub";
    pub const CWK_LORE_PIRATE_HUB: &str = "cwk.lore.pirate.hub";
    pub const CWK_LORE_WARRIOR_COURTYARD: &str = "cwk.lore.warrior.courtyard";
    pub const CWK_LORE_ARCHER_COURTYARD: &str = "cwk.lore.archer.courtyard";
    pub const CWK_LORE_MAGE_COURTYARD: &str = "cwk.lore.mage.courtyard";
    pub const CWK_LORE_THIEF_COURTYARD: &str = "cwk.lore.thief.courtyard";
    pub const CWK_LORE_PIRATE_COURTYARD: &str = "cwk.lore.pirate.courtyard";
    pub const CWK_HOST_WELCOME_1: &str = "cwk.host.welcome.1";
    pub const CWK_HOST_WELCOME_2: &str = "cwk.host.welcome.2";
    pub const CWK_HOST_WELCOME_3: &str = "cwk.host.welcome.3";
    pub const CWK_HOST_ESCORTING: &str = "cwk.host.escorting";
    pub const CWK_HOST_ESCORT_BEATEN: &str = "cwk.host.escort_beaten";
    pub const CWK_HOST_MASTERS: &str = "cwk.host.masters";
    pub const CWK_HOST_MASTERS_ESCORTING: &str = "cwk.host.masters_escorting";
    pub const CWK_HOST_BEATEN: &str = "cwk.host.beaten";
    pub const CWK_HOST_WELL_PLAYED: &str = "cwk.host.well_played";
    pub const CWK_HOST_LEADER: &str = "cwk.host.leader";
    pub const CWK_ESCORT_OUT: &str = "cwk.escort.out";
    pub const CWK_MASTERS_OUT: &str = "cwk.masters.out";
    pub const CWK_BOSS_OPEN: &str = "cwk.boss.open";
    pub const CWK_GUARDIANS_DETECTED: &str = "cwk.guardians.detected";
    pub const CWK_JACK_IDLE: &str = "cwk.jack.idle";
    pub const CWK_JACK_STAGE1_1: &str = "cwk.jack.stage1.1";
    pub const CWK_JACK_STAGE1_2: &str = "cwk.jack.stage1.2";
    pub const CWK_JACK_STAGE1_3: &str = "cwk.jack.stage1.3";
    pub const CWK_JACK_STAGE2_1: &str = "cwk.jack.stage2.1";
    pub const CWK_JACK_STAGE2_2: &str = "cwk.jack.stage2.2";
    pub const CWK_JACK_STAGE3_1: &str = "cwk.jack.stage3.1";
    pub const CWK_JACK_STAGE3_2: &str = "cwk.jack.stage3.2";
    pub const CWK_JACK_STAGE4_1: &str = "cwk.jack.stage4.1";
    pub const CWK_JACK_STAGE4_2: &str = "cwk.jack.stage4.2";
    pub const CWK_JACK_STAGE5_1: &str = "cwk.jack.stage5.1";
    pub const CWK_JACK_STAGE5_2: &str = "cwk.jack.stage5.2";
    pub const CWK_JACK_STAGE5_3: &str = "cwk.jack.stage5.3";
    pub const CWK_JACK_STAGE5_4: &str = "cwk.jack.stage5.4";
    pub const CWK_JACK_REST: &str = "cwk.jack.rest";
    pub const CWK_STIRGES_GONE: &str = "cwk.stirges.gone";
    pub const SHOWA_SQUAD_LEVEL: &str = "showa.squad.level";
    pub const SHOWA_SQUAD_ASK: &str = "showa.squad.ask";
    pub const SHOWA_SQUAD_START_CHOICE: &str = "showa.squad.start_choice";
    pub const SHOWA_SQUAD_JOIN_ASK: &str = "showa.squad.join_ask";
    pub const SHOWA_SQUAD_REJOIN_ASK: &str = "showa.squad.rejoin_ask";
    pub const SHOWA_SQUAD_TAKEN: &str = "showa.squad.taken";
    pub const SHOWA_SQUAD_NOTICE_OPENED: &str = "showa.squad.notice.opened";
    pub const SHOWA_SQUAD_NOTICE_STARTED: &str = "showa.squad.notice.started";
    pub const SHOWA_SQUAD_NO_COMB: &str = "showa.squad.no_comb";
    pub const SHOWA_KONPEI_TOWN_MENU: &str = "showa.konpei.town.menu";
    pub const SHOWA_KONPEI_TOWN_ABOUT_CHOICE: &str = "showa.konpei.town.about_choice";
    pub const SHOWA_KONPEI_TOWN_ABOUT: &str = "showa.konpei.town.about";
    pub const SHOWA_KONPEI_TOWN_GO_CHOICE: &str = "showa.konpei.town.go_choice";
    pub const SHOWA_KONPEI_HIDEOUT_ASK: &str = "showa.konpei.hideout.ask";
    pub const SHOWA_KONPEI_ARENA_LEAVE_ASK: &str = "showa.konpei.arena.leave_ask";
    pub const SHOWA_KONPEI_ARENA_WON: &str = "showa.konpei.arena.won";
    pub const SHOWA_KONPEI_ARENA_FULL: &str = "showa.konpei.arena.full";
    pub const SHOWA_KONPEI_CLEAR: &str = "showa.konpei.clear";
    pub const SHOWA_NO_CASUALTIES: &str = "showa.no_casualties";
    pub const BALROG_SQUAD_LEVEL: &str = "balrog.squad.level";
    pub const BALROG_SQUAD_ASK: &str = "balrog.squad.ask";
    pub const BALROG_SQUAD_START_CHOICE: &str = "balrog.squad.start_choice";
    pub const BALROG_SQUAD_JOIN_ASK: &str = "balrog.squad.join_ask";
    pub const BALROG_SQUAD_REJOIN_ASK: &str = "balrog.squad.rejoin_ask";
    pub const BALROG_SQUAD_TAKEN: &str = "balrog.squad.taken";
    pub const BALROG_SQUAD_NOTICE_OPENED: &str = "balrog.squad.notice.opened";
    pub const BALROG_SQUAD_NOTICE_STARTED: &str = "balrog.squad.notice.started";
    pub const BALROG_KEEPER_LEAVE_ASK: &str = "balrog.keeper.leave_ask";
    pub const BALROG_KEEPER_WON: &str = "balrog.keeper.won";
    pub const BALROG_VICTORY: &str = "balrog.victory";
    pub const BALROG_END_FULL: &str = "balrog.end.full";
    pub const BALROG_SCROLLS_HELLO: &str = "balrog.scrolls.hello";
    pub const BALROG_SCROLLS_REDEEM_CHOICE: &str = "balrog.scrolls.redeem_choice";
    pub const BALROG_SCROLLS_PICK: &str = "balrog.scrolls.pick";
    pub const BALROG_SCROLLS_NO_LEATHER: &str = "balrog.scrolls.no_leather";
    pub const BALROG_SCROLLS_FULL: &str = "balrog.scrolls.full";
    pub const BALROG_SCROLLS_GIVEN: &str = "balrog.scrolls.given";
    pub const LATANICA_PORTAL_NO_ESSENCE: &str = "latanica.portal.no_essence";
    pub const LATANICA_PORTAL_NO_PARTY: &str = "latanica.portal.no_party";
    pub const LATANICA_PORTAL_NOT_LEADER: &str = "latanica.portal.not_leader";
    pub const LATANICA_PORTAL_NOT_ELIGIBLE: &str = "latanica.portal.not_eligible";
    pub const LATANICA_PORTAL_BUSY: &str = "latanica.portal.busy";
    pub const LATANICA_SUMMONED: &str = "latanica.summoned";
    pub const LATANICA_BOB_LEAVE_ASK: &str = "latanica.bob.leave_ask";
    pub const LATANICA_BOB_WON: &str = "latanica.bob.won";
    pub const PINK_BEAN_SQUAD_LEVEL: &str = "pink_bean.squad.level";
    pub const PINK_BEAN_SQUAD_ASK: &str = "pink_bean.squad.ask";
    pub const PINK_BEAN_SQUAD_START_CHOICE: &str = "pink_bean.squad.start_choice";
    pub const PINK_BEAN_SQUAD_JOIN_ASK: &str = "pink_bean.squad.join_ask";
    pub const PINK_BEAN_SQUAD_REJOIN_ASK: &str = "pink_bean.squad.rejoin_ask";
    pub const PINK_BEAN_SQUAD_TAKEN: &str = "pink_bean.squad.taken";
    pub const PINK_BEAN_SQUAD_NOTICE_OPENED: &str = "pink_bean.squad.notice.opened";
    pub const PINK_BEAN_SQUAD_NOTICE_STARTED: &str = "pink_bean.squad.notice.started";
    pub const PINK_BEAN_KEEPER_LEAVE_ASK: &str = "pink_bean.keeper.leave_ask";
    pub const PINK_BEAN_KEEPER_WON: &str = "pink_bean.keeper.won";
    pub const AMORIA_PQ_STAGE6_WAIT: &str = "amoria_pq.stage6.wait";
    pub const AMORIA_PQ_STAGE6_DEAD: &str = "amoria_pq.stage6.dead";
    pub const AMORIA_PQ_GLIMMER_TASK: &str = "amoria_pq.glimmer.task";
    pub const AMORIA_PQ_GLIMMER_MEMBER_TASK: &str = "amoria_pq.glimmer.member_task";
    pub const AMORIA_PQ_GLIMMER_FIERRY_OUT: &str = "amoria_pq.glimmer.fierry_out";
    pub const AMORIA_PQ_GLIMMER_BEAT_FIERRY: &str = "amoria_pq.glimmer.beat_fierry";
    pub const AMORIA_PQ_GLIMMER_SHARD_TAKEN: &str = "amoria_pq.glimmer.shard_taken";
    pub const AMORIA_PQ_GENDER_WOMEN_ONLY: &str = "amoria_pq.gender.women_only";
    pub const AMORIA_PQ_GENDER_MEN_ONLY: &str = "amoria_pq.gender.men_only";
    pub const LINE3_EXIT_STAY: &str = "line3.exit.stay";
    pub const MAKER_PICK: &str = "maker.pick";
    pub const MAKER_ASK: &str = "maker.ask";
    pub const MAKER_MESO: &str = "maker.meso";
    pub const MAKER_SHORT: &str = "maker.short";
    pub const MAKER_FULL: &str = "maker.full";
    pub const MAKER_DONE: &str = "maker.done";
    pub const MAKER_SERRYL_HELLO: &str = "maker.serryl.hello";
    pub const MAKER_SERRYL_KNUCKLERS: &str = "maker.serryl.knucklers";
    pub const MAKER_SERRYL_GUNS: &str = "maker.serryl.guns";
    pub const MAKER_SERRYL_GLOVES: &str = "maker.serryl.gloves";
    pub const MAKER_NEVE_HELLO: &str = "maker.neve.hello";
    pub const MAKER_NEVE_WARRIOR: &str = "maker.neve.warrior";
    pub const MAKER_NEVE_BOWMAN: &str = "maker.neve.bowman";
    pub const MAKER_NEVE_MAGICIAN: &str = "maker.neve.magician";
    pub const MAKER_NEVE_THIEF: &str = "maker.neve.thief";
    pub const MAKER_MOS_HELLO: &str = "maker.mos.hello";
    pub const MAKER_MOS_WHAT: &str = "maker.mos.what";
    pub const MAKER_MOS_STIMULATOR: &str = "maker.mos.stimulator";
    pub const MAKER_MOS_WARRIOR: &str = "maker.mos.warrior";
    pub const MAKER_MOS_BOWMAN: &str = "maker.mos.bowman";
    pub const MAKER_MOS_MAGICIAN: &str = "maker.mos.magician";
    pub const MAKER_MOS_THIEF: &str = "maker.mos.thief";
    pub const MAKER_MOS_PIRATE: &str = "maker.mos.pirate";
    pub const MAKER_MOS_WARRIOR_STIMULATED: &str = "maker.mos.warrior_stimulated";
    pub const MAKER_MOS_BOWMAN_STIMULATED: &str = "maker.mos.bowman_stimulated";
    pub const MAKER_MOS_MAGICIAN_STIMULATED: &str = "maker.mos.magician_stimulated";
    pub const MAKER_MOS_THIEF_STIMULATED: &str = "maker.mos.thief_stimulated";
    pub const MAKER_MOS_PIRATE_STIMULATED: &str = "maker.mos.pirate_stimulated";
    pub const MAKER_MOS_DAGGER: &str = "maker.mos.dagger";
    pub const MAKER_MOS_LOST: &str = "maker.mos.lost";
    pub const QUEST_INFO_OASIS: &str = "quest_info.oasis";
    pub const QUEST_INFO_SHAMAN_ROCK: &str = "quest_info.shaman_rock";
    pub const QUEST_INFO_SHINY_STONE: &str = "quest_info.shiny_stone";
    pub const QUEST_INFO_HARP_MISSED: &str = "quest_info.harp_missed";
    pub const QUEST_INFO_HARP_PLAYED: &str = "quest_info.harp_played";
    pub const QUEST_INFO_PENTAGRAM: &str = "quest_info.pentagram";
    pub const QUEST_INFO_SECRET_WALL: &str = "quest_info.secret_wall";
    pub const QUEST_INFO_WALL_ASK: &str = "quest_info.wall_ask";
    pub const QUEST_INFO_WALL_READ: &str = "quest_info.wall_read";
    pub const REFINER_WHAT: &str = "refiner.what";
    pub const REFINER_HOW_MANY: &str = "refiner.how_many";
    pub const REFINER_ASK: &str = "refiner.ask";
    pub const REFINER_MINERAL: &str = "refiner.mineral";
    pub const REFINER_JEWEL: &str = "refiner.jewel";
    pub const REFINER_ROCK: &str = "refiner.rock";
    pub const REFINER_CRYSTAL: &str = "refiner.crystal";
    pub const REFINER_MATERIALS: &str = "refiner.materials";
    pub const REFINER_ARROWS: &str = "refiner.arrows";
    pub const REFINER_THUNDER_HELLO: &str = "refiner.thunder.hello";
    pub const REFINER_THUNDER_HELMETS: &str = "refiner.thunder.helmets";
    pub const REFINER_THUNDER_SHIELDS: &str = "refiner.thunder.shields";
    pub const REFINER_CHRIS_HELLO: &str = "refiner.chris.hello";
    pub const REFINER_CHRIS_HOOF: &str = "refiner.chris.hoof";
    pub const REFINER_CHRIS_HOOF_ASK: &str = "refiner.chris.hoof_ask";
    pub const REFINER_VOGEN_HELLO: &str = "refiner.vogen.hello";
    pub const REFINER_PI_HELLO: &str = "refiner.pi.hello";
    pub const REFINER_MUHAMAD_HELLO: &str = "refiner.muhamad.hello";
    pub const REFINER_MUHAMAD_BYE: &str = "refiner.muhamad.bye";
    pub const ALTAIRE_HELLO: &str = "altaire.hello";
    pub const ALTAIRE_TRADE: &str = "altaire.trade";
    pub const ALTAIRE_SHORT: &str = "altaire.short";
    pub const ALTAIRE_HOW_MANY: &str = "altaire.how_many";
    pub const ALTAIRE_DONE: &str = "altaire.done";
    pub const ALTAIRE_FULL: &str = "altaire.full";
    pub const REFINER_UPGRADE_CLAW: &str = "refiner.upgrade_claw";
    pub const REFINER_MAKE_CLAW: &str = "refiner.make_claw";
    pub const REFINER_MAKE_GLOVE: &str = "refiner.make_glove";
    pub const REFINER_UPGRADE_GLOVE: &str = "refiner.upgrade_glove";
    pub const REFINER_BOW: &str = "refiner.bow";
    pub const REFINER_CROSSBOW: &str = "refiner.crossbow";
    pub const REFINER_SMITH_HELLO: &str = "refiner.smith.hello";
    pub const REFINER_VICIOUS_HELLO: &str = "refiner.vicious.hello";
    pub const REFINER_JM_HELLO: &str = "refiner.jm.hello";
    pub const MAKER_STIMULATOR: &str = "maker.stimulator";
    pub const MAKER_STIMULATOR_LOST: &str = "maker.stimulator_lost";
    pub const MAKER_FRANCOIS_HELLO: &str = "maker.francois.hello";
    pub const MAKER_CHRISHRAMA_HELLO: &str = "maker.chrishrama.hello";
    pub const MAKER_GORDON_HELLO: &str = "maker.gordon.hello";
    pub const MAKER_SARAH_HELLO: &str = "maker.sarah.hello";
    pub const MAKER_TARA_HELLO: &str = "maker.tara.hello";
    pub const MAKER_RYDOLE_HELLO: &str = "maker.rydole.hello";
    pub const MAKER_MAKE_WAND: &str = "maker.make_wand";
    pub const MAKER_MAKE_STAFF: &str = "maker.make_staff";
    pub const MAKER_UPGRADE_HAT: &str = "maker.upgrade_hat";
    pub const MAKER_SHOES_WARRIOR: &str = "maker.shoes.warrior";
    pub const MAKER_SHOES_BOWMAN: &str = "maker.shoes.bowman";
    pub const MAKER_SHOES_MAGICIAN: &str = "maker.shoes.magician";
    pub const MAKER_SHOES_THIEF: &str = "maker.shoes.thief";
    pub const MAKER_SHOES_WARRIOR_STIMULATED: &str = "maker.shoes.warrior_stimulated";
    pub const MAKER_SHOES_BOWMAN_STIMULATED: &str = "maker.shoes.bowman_stimulated";
    pub const MAKER_SHOES_MAGICIAN_STIMULATED: &str = "maker.shoes.magician_stimulated";
    pub const MAKER_SHOES_THIEF_STIMULATED: &str = "maker.shoes.thief_stimulated";
    pub const MAKER_GLOVES_WARRIOR: &str = "maker.gloves.warrior";
    pub const MAKER_GLOVES_BOWMAN: &str = "maker.gloves.bowman";
    pub const MAKER_GLOVES_MAGICIAN: &str = "maker.gloves.magician";
    pub const MAKER_GLOVES_THIEF: &str = "maker.gloves.thief";
    pub const MAKER_GLOVES_WARRIOR_UPGRADED: &str = "maker.gloves.warrior_upgraded";
    pub const MAKER_GLOVES_BOWMAN_UPGRADED: &str = "maker.gloves.bowman_upgraded";
    pub const MAKER_GLOVES_MAGICIAN_UPGRADED: &str = "maker.gloves.magician_upgraded";
    pub const MAKER_GLOVES_THIEF_UPGRADED: &str = "maker.gloves.thief_upgraded";
    pub const MILK_TANGYOON_FULL: &str = "milk.tangyoon.full";
    pub const MILK_COW_LAST: &str = "milk.cow.last";
    pub const MILK_COW_NO_BOTTLE: &str = "milk.cow.no_bottle";
    pub const MILK_COW_FULL: &str = "milk.cow.full";
    pub const MILK_CALF_EMPTY: &str = "milk.calf.empty";
    pub const MILK_CALF_DRINKS: &str = "milk.calf.drinks";
    pub const MILK_CALF_NO_BOTTLE: &str = "milk.calf.no_bottle";
    pub const MILK_JUG_NOT_FULL: &str = "milk.jug_not_full";
    /// An area boss came out, to its map; `{mob}` is its name.
    pub const AREA_BOSS_APPEARED: &str = "area_boss.appeared";
    /// Lord Jonathan (1092019) and his test for "Jonathan's Court".
    pub const JONATHAN_HELLO: &str = "jonathan.hello";
    pub const JONATHAN_TEST_ASK: &str = "jonathan.test.ask";
    pub const JONATHAN_TEST_BUSY: &str = "jonathan.test.busy";
    /// Pink, to whoever hit the real Bart.
    pub const JONATHAN_BART_FOUND: &str = "jonathan.bart_found";
    /// Kyrin's 4th-job tests (6330, 6370): her offer, a taken room, the
    /// room's way out too soon (pink), and her word to one who held out.
    pub const KYRIN_TEST_ASK: &str = "kyrin.test.ask";
    pub const KYRIN_TEST_BUSY: &str = "kyrin.test.busy";
    pub const KYRIN_TEST_WAIT: &str = "kyrin.test.wait";
    pub const KYRIN_TEST_PASSED: &str = "kyrin.test.passed";
    /// The Insignificant Being (1061012) on the way into the Junior
    /// Balrog's other world (6107, 6108): not on the quests, the wrong job,
    /// too low a level, and the world taken.
    pub const BALROG_WORLD_NOT_ALLOWED: &str = "balrog_world.not_allowed";
    pub const BALROG_WORLD_JOB: &str = "balrog_world.job";
    pub const BALROG_WORLD_LEVEL: &str = "balrog_world.level";
    pub const BALROG_WORLD_BUSY: &str = "balrog_world.busy";
    /// Papulatus in the past (2043000), sending a visitor back to their time.
    pub const PAST_PAPULATUS_SEND_BACK: &str = "papulatus.past.send_back";
    /// Delli's rescue (6410): Shulynch (1092008) in the Nautilus's Training
    /// Room sending a Corsair on the quest to look for Delli, and Shulynch
    /// (1095000) on the way offering the way back.
    pub const DELLI_SHULYNCH_ASK: &str = "delli.shulynch.ask";
    pub const DELLI_SHULYNCH_LEAVE: &str = "delli.shulynch.leave";
    /// Delli (2095000) where the party gathers: the menu, its lines, what
    /// Delli asks, and why a party can't go; the pink line at the way out
    /// before Delli is safe; and the offer of the way back after.
    pub const DELLI_MENU: &str = "delli.menu";
    pub const DELLI_ENTER: &str = "delli.enter";
    pub const DELLI_ABOUT: &str = "delli.about";
    pub const DELLI_INFO: &str = "delli.info";
    pub const DELLI_NO_PARTY: &str = "delli.no_party";
    pub const DELLI_NOT_LEADER: &str = "delli.not_leader";
    pub const DELLI_NOT_ELIGIBLE: &str = "delli.not_eligible";
    pub const DELLI_BUSY: &str = "delli.busy";
    pub const DELLI_NOT_WON: &str = "delli.not_won";
    pub const DELLI_HOME_ASK: &str = "delli.home.ask";
    /// The Ludibrium Maze: Rolly (9103001) in Ludibrium, the menu, its
    /// lines, the rules, and why a party can't go; and Rolly (9103003) on
    /// the way out, sending everyone back to Ludibrium.
    pub const LUDIMAZE_MENU: &str = "ludimaze.menu";
    pub const LUDIMAZE_ENTER: &str = "ludimaze.enter";
    pub const LUDIMAZE_ABOUT: &str = "ludimaze.about";
    pub const LUDIMAZE_INFO: &str = "ludimaze.info";
    pub const LUDIMAZE_NO_PARTY: &str = "ludimaze.no_party";
    pub const LUDIMAZE_NOT_LEADER: &str = "ludimaze.not_leader";
    pub const LUDIMAZE_NOT_ELIGIBLE: &str = "ludimaze.not_eligible";
    pub const LUDIMAZE_BUSY: &str = "ludimaze.busy";
    pub const LUDIMAZE_HOME: &str = "ludimaze.home";
    pub const LUDIMAZE_SUMMONED: &str = "ludimaze.summoned";
    pub const LUDIMAZE_PIETRI_NOT_LEADER: &str = "ludimaze.pietri.not_leader";
    pub const LUDIMAZE_PIETRI_WAIT: &str = "ludimaze.pietri.wait";
    pub const LUDIMAZE_PIETRI_SHORT: &str = "ludimaze.pietri.short";
    pub const LUDIMAZE_PIETRI_ASK: &str = "ludimaze.pietri.ask";
    pub const LUDIMAZE_PIETRI_DONE: &str = "ludimaze.pietri.done";
    pub const LUDIMAZE_PRESENT_ASK: &str = "ludimaze.present.ask";
    pub const LUDIMAZE_PRESENT_FULL: &str = "ludimaze.present.full";
    pub const TYLUS_MENU: &str = "tylus.menu";
    pub const TYLUS_ENTER: &str = "tylus.enter";
    pub const TYLUS_ABOUT: &str = "tylus.about";
    pub const TYLUS_INFO: &str = "tylus.info";
    pub const TYLUS_NO_PARTY: &str = "tylus.no_party";
    pub const TYLUS_NOT_LEADER: &str = "tylus.not_leader";
    pub const TYLUS_NOT_ELIGIBLE: &str = "tylus.not_eligible";
    pub const TYLUS_BUSY: &str = "tylus.busy";
    pub const TYLUS_FALLEN: &str = "tylus.fallen";
    pub const TYLUS_THANKS: &str = "tylus.thanks";
    pub const ELLIN_MENU: &str = "ellin.menu";
    pub const ELLIN_ENTER: &str = "ellin.enter";
    pub const ELLIN_ABOUT: &str = "ellin.about";
    pub const ELLIN_INFO: &str = "ellin.info";
    pub const ELLIN_NO_PARTY: &str = "ellin.no_party";
    pub const ELLIN_NOT_LEADER: &str = "ellin.not_leader";
    pub const ELLIN_NOT_ELIGIBLE: &str = "ellin.not_eligible";
    pub const ELLIN_BUSY: &str = "ellin.busy";
    pub const ELLIN_STAGE1_SHUT: &str = "ellin.stage1.shut";
    pub const ELLIN_STAGE2_SHUT: &str = "ellin.stage2.shut";
    pub const ELLIN_GUIDE_LEAVE: &str = "ellin.guide.leave";
    pub const ELLIN_GUIDE_WELCOME: &str = "ellin.guide.welcome";
    pub const ELLIN_GUIDE_MAZE_END: &str = "ellin.guide.maze_end";
    pub const ELLIN_GUIDE_FLOWERS: &str = "ellin.guide.flowers";
    pub const ELLIN_GUIDE_MARBLES: &str = "ellin.guide.marbles";
    pub const ELLIN_GUIDE_NO_ROOM: &str = "ellin.guide.no_room";
    pub const ELLIN_GUIDE_FLOWERS_LEAVE: &str = "ellin.guide.flowers.leave";
    pub const ELLIN_GUIDE_ALL_CAUGHT: &str = "ellin.guide.all_caught";
    pub const ELLIN_GUIDE_HAND_IN: &str = "ellin.guide.hand_in";
    pub const ELLIN_GUIDE_TO_LEADER: &str = "ellin.guide.to_leader";
    pub const ELLIN_MILEPOST_LEAVE: &str = "ellin.milepost.leave";
    pub const ELLIN_SPRITE_TO_ALTAR: &str = "ellin.sprite.to_altar";
    pub const ELLIN_SPRITE_LEAVE: &str = "ellin.sprite.leave";
    pub const ELLIN_ALTAR_GOLEM: &str = "ellin.altar.golem";
    pub const ELLIN_STAGE6_SHUT: &str = "ellin.stage6.shut";
    pub const ELLIN_STAGE6_WAIT: &str = "ellin.stage6.wait";
    pub const ELLIN_HOME_NO_ROOM: &str = "ellin.home.no_room";
    pub const ELLIN_PRIZE: &str = "ellin.prize";
    pub const ELLIN_PRIZE_MENU: &str = "ellin.prize.menu";
    pub const ELLIN_PRIZE_ALTAIRE: &str = "ellin.prize.altaire";
    pub const ELLIN_PRIZE_GLITTERING: &str = "ellin.prize.glittering";
    pub const ELLIN_PRIZE_BRILLIANT: &str = "ellin.prize.brilliant";
    pub const ELLIN_PRIZE_NO_FRAGMENTS: &str = "ellin.prize.no_fragments";
    pub const ELLIN_PRIZE_NO_ALTAIRE: &str = "ellin.prize.no_altaire";
    pub const ELLIN_PRIZE_NO_GLITTERING: &str = "ellin.prize.no_glittering";
    pub const ELLIN_PRIZE_NO_ROOM: &str = "ellin.prize.no_room";
    pub const MAGATIA_ENTER: &str = "magatia.enter";
    pub const MAGATIA_ABOUT: &str = "magatia.about";
    pub const MAGATIA_NO_PARTY: &str = "magatia.no_party";
    pub const MAGATIA_NOT_LEADER: &str = "magatia.not_leader";
    pub const MAGATIA_NOT_ELIGIBLE: &str = "magatia.not_eligible";
    pub const MAGATIA_BUSY: &str = "magatia.busy";
    pub const MAGATIA_ROMEO_MENU: &str = "magatia.romeo.menu";
    pub const MAGATIA_ROMEO_INFO: &str = "magatia.romeo.info";
    pub const MAGATIA_ROMEO_RETREAT: &str = "magatia.romeo.retreat";
    pub const MAGATIA_JULIET_MENU: &str = "magatia.juliet.menu";
    pub const MAGATIA_JULIET_INFO: &str = "magatia.juliet.info";
    pub const MAGATIA_JULIET_RETREAT: &str = "magatia.juliet.retreat";
    pub const MAGATIA_BOOK_NOTHING: &str = "magatia.book.nothing";
    pub const MAGATIA_BOOK_MESOS: &str = "magatia.book.mesos";
    pub const MAGATIA_BOOK_EXP: &str = "magatia.book.exp";
    pub const MAGATIA_BOOK_LETTER: &str = "magatia.book.letter";
    pub const MAGATIA_BOOK_LETTER_FULL: &str = "magatia.book.letter_full";
    pub const MAGATIA_BOOK_PASS: &str = "magatia.book.pass";
    pub const MAGATIA_PORTAL_SHUT: &str = "magatia.portal_shut";
    pub const MAGATIA_LETTER: &str = "magatia.letter";
    pub const MAGATIA_ROMEO_SHOCKED: &str = "magatia.romeo.shocked";
    pub const MAGATIA_JULIET_SHOCKED: &str = "magatia.juliet.shocked";
    pub const MAGATIA_OPEN_DOOR: &str = "magatia.open_door";
    pub const MAGATIA_ROMEO_HURRY: &str = "magatia.romeo.hurry";
    pub const MAGATIA_JULIET_HURRY: &str = "magatia.juliet.hurry";
    pub const MAGATIA_FILES: &str = "magatia.files";
    pub const MAGATIA_FILES_LEADER: &str = "magatia.files_leader";
    pub const MAGATIA_YULETE_LEAVING: &str = "magatia.yulete.leaving";
    pub const MAGATIA_YULETE_WATCHED: &str = "magatia.yulete.watched";
    pub const MAGATIA_YULETE_SNEAKY: &str = "magatia.yulete.sneaky";
    pub const MAGATIA_YULETE_CAUGHT: &str = "magatia.yulete.caught";
    pub const MAGATIA_YULETE_DISGUSTED: &str = "magatia.yulete.disgusted";
    pub const MAGATIA_YULETE_FOOLED: &str = "magatia.yulete.fooled";
    pub const MAGATIA_ROOM_TAKEN: &str = "magatia.room_taken";
    pub const MAGATIA_YULETE_PRESENTS: &str = "magatia.yulete.presents";
    pub const MAGATIA_YULETE_MIGHTY: &str = "magatia.yulete.mighty";
    pub const MAGATIA_YULETE_BEHOLD: &str = "magatia.yulete.behold";
    pub const MAGATIA_YULETE_BEATEN: &str = "magatia.yulete.beaten";
    pub const MAGATIA_FRANKENROID: &str = "magatia.frankenroid";
    pub const MAGATIA_FRANKENROID_MIGHTY: &str = "magatia.frankenroid_mighty";
    pub const MAGATIA_JULIET_FAINTED: &str = "magatia.juliet.fainted";
    pub const MAGATIA_ROMEO_FAINTED: &str = "magatia.romeo.fainted";
    pub const MAGATIA_ROMEO_SAVED: &str = "magatia.romeo.saved";
    pub const MAGATIA_JULIET_SAVED: &str = "magatia.juliet.saved";
    pub const MAGATIA_ROMEO_SAVED_HURT: &str = "magatia.romeo.saved_hurt";
    pub const MAGATIA_JULIET_SAVED_HURT: &str = "magatia.juliet.saved_hurt";
    pub const MAGATIA_GIFT: &str = "magatia.gift";
    pub const MAGATIA_GIFT_FULL: &str = "magatia.gift_full";
    pub const MAGATIA_TRACES_DEFEATED: &str = "magatia.traces.defeated";
    pub const MAGATIA_TRACES_CHEER: &str = "magatia.traces.cheer";
    pub const MAGATIA_TRACES_SORRY: &str = "magatia.traces.sorry";
    pub const MAGATIA_THANKS_REHAB: &str = "magatia.thanks.rehab";
    pub const MAGATIA_THANKS_JAIL: &str = "magatia.thanks.jail";
    pub const MAGATIA_THANKS_FULL: &str = "magatia.thanks.full";
    pub const MAGATIA_PIPE_DARK: &str = "magatia.pipe_dark";
    pub const PIRATE_MENU: &str = "pirate.menu";
    pub const PIRATE_ENTER: &str = "pirate.enter";
    pub const PIRATE_ABOUT: &str = "pirate.about";
    pub const PIRATE_INFO: &str = "pirate.info";
    pub const PIRATE_NO_PARTY: &str = "pirate.no_party";
    pub const PIRATE_NOT_LEADER: &str = "pirate.not_leader";
    pub const PIRATE_NOT_ELIGIBLE: &str = "pirate.not_eligible";
    pub const PIRATE_BUSY: &str = "pirate.busy";
    pub const PIRATE_GUON_LEAVE: &str = "pirate.guon.leave";
    pub const PIRATE_GUON_BOXES: &str = "pirate.guon.boxes";
    pub const PIRATE_GUON_MARKS: &str = "pirate.guon.marks";
    pub const PIRATE_GUON_DECK: &str = "pirate.guon.deck";
    pub const PIRATE_GUON_BONUS: &str = "pirate.guon.bonus";
    pub const PIRATE_GUON_CREW: &str = "pirate.guon.crew";
    pub const PIRATE_GUON_DOORS: &str = "pirate.guon.doors";
    pub const PIRATE_GUON_BOSS: &str = "pirate.guon.boss";
    pub const PIRATE_PORTAL_SHUT: &str = "pirate.portal_shut";
    pub const PIRATE_GUON_RISING: &str = "pirate.guon.rising";
    pub const PIRATE_GUON_VETERAN: &str = "pirate.guon.veteran";
    pub const PIRATE_GUON_MARKS_IN: &str = "pirate.guon.marks_in";
    pub const PIRATE_MARKS_SHUT: &str = "pirate.marks_shut";
    pub const PIRATE_GUON_CHEST: &str = "pirate.guon.chest";
    pub const PIRATE_WU_FREED: &str = "pirate.wu.freed";
    pub const PIRATE_WU_WAIT: &str = "pirate.wu.wait";
    pub const PIRATE_WU_THANKS: &str = "pirate.wu.thanks";
    pub const PIRATE_WU_MENU: &str = "pirate.wu.menu";
    pub const PIRATE_WU_OUT: &str = "pirate.wu.out";
    pub const PIRATE_WU_HAT: &str = "pirate.wu.hat";
    pub const PIRATE_WU_ROOM_ETC: &str = "pirate.wu.room_etc";
    pub const PIRATE_WU_ROOM_EQUIP: &str = "pirate.wu.room_equip";
    pub const PIRATE_WU_HAT_GIVEN: &str = "pirate.wu.hat_given";
    pub const PIRATE_WU_NEED_FEATHERS: &str = "pirate.wu.need_feathers";
    pub const PIRATE_WU_BEST: &str = "pirate.wu.best";
    pub const PIRATE_GUON_SAVED: &str = "pirate.guon.saved";
    pub const DOJO_SO_GONG_MENU: &str = "dojo.so_gong.menu";
    pub const DOJO_SO_GONG_ALONE: &str = "dojo.so_gong.alone";
    pub const DOJO_SO_GONG_PARTY: &str = "dojo.so_gong.party";
    pub const DOJO_SO_GONG_PARTY_LEADER: &str = "dojo.so_gong.party_leader";
    pub const DOJO_SO_GONG_PARTY_SPREAD: &str = "dojo.so_gong.party_spread";
    pub const DOJO_SO_GONG_ABOUT: &str = "dojo.so_gong.about";
    pub const DOJO_SO_GONG_INFO: &str = "dojo.so_gong.info";
    pub const DOJO_SO_GONG_LOW_LEVEL: &str = "dojo.so_gong.low_level";
    pub const DOJO_SO_GONG_BUSY: &str = "dojo.so_gong.busy";
    pub const DOJO_SO_GONG_QUIT: &str = "dojo.so_gong.quit";
    pub const DOJO_SO_GONG_QUIT_NO: &str = "dojo.so_gong.quit_no";
    pub const DOJO_SO_GONG_QUIT_DONE: &str = "dojo.so_gong.quit_done";
    pub const DOJO_SO_GONG_REST: &str = "dojo.so_gong.rest";
    pub const DOJO_SO_GONG_CONTINUE: &str = "dojo.so_gong.continue";
    pub const DOJO_SO_GONG_LEAVE: &str = "dojo.so_gong.leave";
    pub const DOJO_SO_GONG_LEAVE_ASK: &str = "dojo.so_gong.leave_ask";
    pub const DOJO_BOARD_MENU: &str = "dojo.board.menu";
    pub const DOJO_BOARD_CHALLENGE: &str = "dojo.board.challenge";
    pub const DOJO_BOARD_READ: &str = "dojo.board.read";
    pub const DOJO_BOARD_NOTICE_0: &str = "dojo.board.notice.0";
    pub const DOJO_BOARD_NOTICE_1: &str = "dojo.board.notice.1";
    pub const DOJO_BOARD_ASK: &str = "dojo.board.ask";
    pub const DOJO_BOARD_DECLINE: &str = "dojo.board.decline";
    /// The Dimensional Mirror's labels, under its icons, and its one line
    /// when nothing is open to the player.
    pub const MIRROR_DOJO: &str = "mirror.dojo";
    pub const MIRROR_CARNIVAL: &str = "mirror.carnival";
    pub const MIRROR_PYRAMID: &str = "mirror.pyramid";
    pub const MIRROR_NOWHERE: &str = "mirror.nowhere";
    /// One line each for NPCs that have nothing else to say: no speech in
    /// String.nx and no script of their own.
    pub const LINE_RYKO: &str = "npc_line.ryko";
    pub const LINE_ADONIS: &str = "npc_line.adonis";
    pub const LINE_DOCUMENT_ROLL: &str = "npc_line.document_roll";
    pub const LINE_PRACTICE_CHART: &str = "npc_line.practice_chart";
    pub const LINE_KNIGHT_ARMOR: &str = "npc_line.knight_armor";
    pub const LINE_SHAREN_WILL: &str = "npc_line.sharen_will";
    pub const LINE_GRAVE_LIGHT: &str = "npc_line.grave_light";
    pub const LINE_ENIGMA_TOMBSTONE: &str = "npc_line.enigma_tombstone";
    pub const LINE_WORN_TOMBSTONE: &str = "npc_line.worn_tombstone";
    pub const DOJO_UP_MONSTERS: &str = "dojo.up.monsters";
    pub const DOJO_NEXT_SHUT: &str = "dojo.next.shut";
    pub const DOJO_POINTS: &str = "dojo.points";
    pub const DOJO_ENERGY_RESET: &str = "dojo.energy_reset";
    pub const DOJO_SO_GONG_RECORD: &str = "dojo.so_gong.record";
    pub const DOJO_SO_GONG_RECORD_ASK: &str = "dojo.so_gong.record_ask";
    pub const DOJO_SO_GONG_RECORD_NO: &str = "dojo.so_gong.record_no";
    pub const DOJO_SO_GONG_RECORDED: &str = "dojo.so_gong.recorded";
    pub const DOJO_SO_GONG_RECORD_KEPT: &str = "dojo.so_gong.record_kept";
    pub const DOJO_SO_GONG_RESUME: &str = "dojo.so_gong.resume";
    pub const DOJO_SO_GONG_BELT: &str = "dojo.so_gong.belt";
    pub const DOJO_SO_GONG_RESET: &str = "dojo.so_gong.reset";
    pub const DOJO_SO_GONG_MEDAL: &str = "dojo.so_gong.medal";
    pub const DOJO_SO_GONG_ROOM: &str = "dojo.so_gong.room";
    pub const DOJO_SO_GONG_RESET_ASK: &str = "dojo.so_gong.reset_ask";
    pub const DOJO_SO_GONG_RESET_DONE: &str = "dojo.so_gong.reset_done";
    pub const DOJO_SO_GONG_RESET_NO: &str = "dojo.so_gong.reset_no";
    pub const DOJO_SO_GONG_BELTS: &str = "dojo.so_gong.belts";
    pub const DOJO_SO_GONG_BELT_CHOICE: &str = "dojo.so_gong.belt_choice";
    pub const DOJO_SO_GONG_BELT_HELD: &str = "dojo.so_gong.belt_held";
    pub const DOJO_SO_GONG_BELT_GIVEN: &str = "dojo.so_gong.belt_given";
    pub const DOJO_SO_GONG_BELT_NEEDS: &str = "dojo.so_gong.belt_needs";
    pub const DOJO_SO_GONG_BELT_BEFORE: &str = "dojo.so_gong.belt_before";
    pub const DOJO_SO_GONG_BELT_LACKING: &str = "dojo.so_gong.belt_lacking";
    pub const DOJO_SO_GONG_BELT_LACKS_BELT: &str = "dojo.so_gong.belt_lacks_belt";
    pub const DOJO_SO_GONG_BELT_LACKS_POINTS: &str = "dojo.so_gong.belt_lacks_points";
    pub const DOJO_SO_GONG_BELT_AND: &str = "dojo.so_gong.belt_and";
    pub const DOJO_SO_GONG_MEDAL_TRY: &str = "dojo.so_gong.medal_try";
    pub const DOJO_SO_GONG_MEDAL_NO: &str = "dojo.so_gong.medal_no";
    pub const DOJO_SO_GONG_MEDAL_ALL: &str = "dojo.so_gong.medal_all";
    pub const DOJO_SO_GONG_MEDAL_MORE: &str = "dojo.so_gong.medal_more";
    pub const DOJO_SO_GONG_MEDAL_GIVEN: &str = "dojo.so_gong.medal_given";
    pub const DOJO_SO_GONG_MEDAL_ROOM: &str = "dojo.so_gong.medal_room";
    pub const DOJO_SO_GONG_TRIAL_ASK: &str = "dojo.so_gong.trial_ask";
    pub const DOJO_SO_GONG_TRIAL_NO: &str = "dojo.so_gong.trial_no";
    pub const DOJO_TRIAL_TAUNT: &str = "dojo.trial.taunt";
    pub const DOJO_TRIAL_STAY: &str = "dojo.trial.stay";
    pub const PYRAMID_DUARTE_MENU: &str = "pyramid.duarte.menu";
    pub const PYRAMID_DUARTE_ABOUT: &str = "pyramid.duarte.about";
    pub const PYRAMID_DUARTE_ENTER: &str = "pyramid.duarte.enter";
    pub const PYRAMID_DUARTE_TREASURES: &str = "pyramid.duarte.treasures";
    pub const PYRAMID_DUARTE_INFO: &str = "pyramid.duarte.info";
    pub const PYRAMID_DUARTE_RULES: &str = "pyramid.duarte.rules";
    pub const PYRAMID_DUARTE_LUCK: &str = "pyramid.duarte.luck";
    pub const PYRAMID_DUARTE_TREASURES_INFO: &str = "pyramid.duarte.treasures_info";
    pub const PYRAMID_DUARTE_HOW: &str = "pyramid.duarte.how";
    pub const PYRAMID_DUARTE_ALONE: &str = "pyramid.duarte.alone";
    pub const PYRAMID_DUARTE_PARTY: &str = "pyramid.duarte.party";
    pub const PYRAMID_DUARTE_MODE: &str = "pyramid.duarte.mode";
    pub const PYRAMID_DUARTE_LOW_LEVEL: &str = "pyramid.duarte.low_level";
    pub const PYRAMID_DUARTE_ONLY_HELL: &str = "pyramid.duarte.only_hell";
    pub const PYRAMID_DUARTE_NO_PARTY: &str = "pyramid.duarte.no_party";
    pub const PYRAMID_DUARTE_NOT_LEADER: &str = "pyramid.duarte.not_leader";
    pub const PYRAMID_DUARTE_PARTY_TERMS: &str = "pyramid.duarte.party_terms";
    pub const PYRAMID_DUARTE_BUSY: &str = "pyramid.duarte.busy";
    pub const PYRAMID_DUARTE_REFUSED: &str = "pyramid.duarte.refused";
    pub const PYRAMID_DUARTE_FORFEIT_ASK: &str = "pyramid.duarte.forfeit_ask";
    pub const PYRAMID_DUARTE_RESULT_ASK: &str = "pyramid.duarte.result_ask";
    pub const PYRAMID_DUARTE_SHADES_ASK: &str = "pyramid.duarte.shades_ask";
    pub const PYRAMID_DUARTE_MEDAL: &str = "pyramid.duarte.medal";
    pub const PYRAMID_DUARTE_MEDAL_HAVE: &str = "pyramid.duarte.medal_have";
    pub const PYRAMID_DUARTE_MEDAL_MORE: &str = "pyramid.duarte.medal_more";
    pub const PYRAMID_DUARTE_MEDAL_ROOM: &str = "pyramid.duarte.medal_room";
    pub const PYRAMID_DUARTE_MEDAL_GIVEN: &str = "pyramid.duarte.medal_given";
    pub const PYRAMID_DUARTE_TOMB: &str = "pyramid.duarte.tomb";
    pub const PYRAMID_DUARTE_TOMB_GEM: &str = "pyramid.duarte.tomb_gem";
    pub const PYRAMID_DUARTE_TOMB_NO_GEM: &str = "pyramid.duarte.tomb_no_gem";
    pub const PYRAMID_DUARTE_TOMB_FULL: &str = "pyramid.duarte.tomb_full";
    pub const PYRAMID_DUARTE_SHADES_PASSED: &str = "pyramid.duarte.shades_passed";
    pub const PYRAMID_DUARTE_SHADES_NOW: &str = "pyramid.duarte.shades_now";
    pub const PYRAMID_DUARTE_SHADES_LATER: &str = "pyramid.duarte.shades_later";
    pub const PYRAMID_DUARTE_SHADES_GEM: &str = "pyramid.duarte.shades_gem";
    pub const PYRAMID_DUARTE_SHADES_NO_ROOM: &str = "pyramid.duarte.shades_no_room";
    pub const CARNIVAL_SPIEGELMANN_MENU: &str = "carnival.spiegelmann.menu";
    pub const CARNIVAL_SPIEGELMANN_ENTER: &str = "carnival.spiegelmann.enter";
    pub const CARNIVAL_SPIEGELMANN_ABOUT: &str = "carnival.spiegelmann.about";
    pub const CARNIVAL_SPIEGELMANN_INFO: &str = "carnival.spiegelmann.info";
    pub const CARNIVAL_SPIEGELMANN_RULES: &str = "carnival.spiegelmann.rules";
    pub const CARNIVAL_SPIEGELMANN_ROOMS: &str = "carnival.spiegelmann.rooms";
    pub const CARNIVAL_SPIEGELMANN_ROOM: &str = "carnival.spiegelmann.room";
    pub const CARNIVAL_SPIEGELMANN_LEVELS: &str = "carnival.spiegelmann.levels";
    pub const CARNIVAL_SPIEGELMANN_NO_PARTY: &str = "carnival.spiegelmann.no_party";
    pub const CARNIVAL_SPIEGELMANN_NOT_LEADER: &str = "carnival.spiegelmann.not_leader";
    pub const CARNIVAL_SPIEGELMANN_TERMS: &str = "carnival.spiegelmann.terms";
    pub const CARNIVAL_SPIEGELMANN_BUSY: &str = "carnival.spiegelmann.busy";
    pub const CARNIVAL_SPIEGELMANN_GO: &str = "carnival.spiegelmann.go";
    pub const CARNIVAL_GUIDE_BACK: &str = "carnival.guide.back";
    pub const CARNIVAL_SPIEGELMANN_WAITING: &str = "carnival.spiegelmann.waiting";
    pub const CARNIVAL_SPIEGELMANN_TAKEN: &str = "carnival.spiegelmann.taken";
    pub const CARNIVAL_SPIEGELMANN_SENT: &str = "carnival.spiegelmann.sent";
    pub const CARNIVAL_SPIEGELMANN_UNEQUAL: &str = "carnival.spiegelmann.unequal";
    pub const CARNIVAL_SPIEGELMANN_FACING: &str = "carnival.spiegelmann.facing";
    pub const CARNIVAL_SPIEGELMANN_FULL: &str = "carnival.spiegelmann.full";
    pub const CARNIVAL_CHALLENGE_ASK: &str = "carnival.challenge.ask";
    pub const CARNIVAL_CHALLENGE_MEMBER: &str = "carnival.challenge.member";
    pub const CARNIVAL_CHALLENGE_DECLINED: &str = "carnival.challenge.declined";
    pub const CARNIVAL_CHALLENGE_UNEQUAL: &str = "carnival.challenge.unequal";
    pub const CARNIVAL_WAITING: &str = "carnival.waiting";
    pub const CARNIVAL_TEAM_RED: &str = "carnival.team.red";
    pub const CARNIVAL_TEAM_BLUE: &str = "carnival.team.blue";
    pub const CARNIVAL_ROOM_WAIT: &str = "carnival.room.wait";
    pub const CARNIVAL_ROOM_LEAVE: &str = "carnival.room.leave";
    pub const CARNIVAL_OVERTIME: &str = "carnival.overtime";
    pub const CARNIVAL_GUIDE_WON: &str = "carnival.guide.won";
    pub const CARNIVAL_GUIDE_LOST: &str = "carnival.guide.lost";
    pub const COCONUT_FELL_MAPLE: &str = "coconut.fell.maple";
    pub const COCONUT_FELL_STORY: &str = "coconut.fell.story";
    pub const COCONUT_OVERTIME: &str = "coconut.overtime";
    pub const SNOWBALL_ENDED: &str = "snowball.ended";
    pub const LEVEL_200: &str = "level.200";
    pub const GM_EVENT_OPENED: &str = "gm_event.opened";
    pub const GM_EVENT_MENU: &str = "gm_event.menu";
    pub const GM_EVENT_ABOUT: &str = "gm_event.about";
    pub const GM_EVENT_INFO: &str = "gm_event.info";
    pub const GM_EVENT_GO: &str = "gm_event.go";
    pub const GM_EVENT_NOT_OPEN: &str = "gm_event.not_open";
    pub const GM_EVENT_PIETRO: &str = "gm_event.pietro";
    pub const GM_EVENT_PIETRO_FULL: &str = "gm_event.pietro_full";
    pub const GM_EVENT_PIETRA: &str = "gm_event.pietra";
    pub const GM_EVENT_HARRY_ASK: &str = "gm_event.harry_ask";
    pub const OX_WRONG: &str = "ox.wrong";
    pub const OX_WON: &str = "ox.won";
    /// The Physical Fitness Test's notices, in the order they go out.
    pub const FITNESS_NOTICE: [&str; 13] = [
        "fitness.notice.0",
        "fitness.notice.1",
        "fitness.notice.2",
        "fitness.notice.3",
        "fitness.notice.4",
        "fitness.notice.5",
        "fitness.notice.6",
        "fitness.notice.7",
        "fitness.notice.8",
        "fitness.notice.9",
        "fitness.notice.10",
        "fitness.notice.11",
        "fitness.notice.12",
    ];
    pub const OLA_NOTICE: [&str; 9] = [
        "ola.notice.0",
        "ola.notice.1",
        "ola.notice.2",
        "ola.notice.3",
        "ola.notice.4",
        "ola.notice.5",
        "ola.notice.6",
        "ola.notice.7",
        "ola.notice.8",
    ];
    pub const JEWEL_NOTICE: [&str; 2] = ["jewel.notice.0", "jewel.notice.1"];
    pub const DOJO_TAUNT: [&str; 3] = ["dojo.taunt.0", "dojo.taunt.1", "dojo.taunt.2"];
    pub const MILK_TANGYOON: [&str; 2] = ["milk.tangyoon.0", "milk.tangyoon.1"];
    pub const MILK_COW_FILLS: [&str; 3] =
        ["milk.cow.fills.0", "milk.cow.fills.1", "milk.cow.fills.2"];

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
    /// Drops that replace whatever the pack (or `like`) says for it.
    #[serde(default)]
    replace: Option<Vec<RewardEntry>>,
    /// Quest items, by item id: they drop only while their quest is started,
    /// and always drop then, even without the pack.
    #[serde(default)]
    quest_items: BTreeMap<String, i16>,
    /// Drops added to the pack's (or `like`'s) list.
    #[serde(default)]
    add: Vec<RewardEntry>,
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
            let mut drops = entry.replace.unwrap_or_else(|| {
                table
                    .pack
                    .get(&entry.like.unwrap_or(id))
                    .cloned()
                    .unwrap_or_default()
            });
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
            drops.extend(entry.add);
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

/// NPC shops: the pack's `shops.json`, by NPC id, and the shopkeepers it
/// lacks from `shop_additions.toml`. Without the pack only those keep one.
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
        #[derive(Deserialize)]
        struct Additions {
            shops: BTreeMap<String, Vec<(i32, i32)>>,
        }
        let additions: Additions = toml::from_str(SHOP_ADDITIONS)
            .map_err(|e| GameDataError::InvalidData(format!("shop_additions.toml: {e}")))?;
        for (npc, rows) in additions.shops {
            let npc = npc.parse().map_err(|_| {
                GameDataError::InvalidData(format!("shop_additions.toml: NPC id {npc}"))
            })?;
            npcs.entry(npc).or_insert_with(|| {
                rows.into_iter()
                    .map(|(item, price)| ShopEntry { item, price })
                    .collect()
            });
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

/// How rare a Gachapon prize is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GachaponTier {
    Common,
    Uncommon,
    Rare,
}

/// A Gachapon prize: `count` of `item`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub struct GachaponPrize {
    pub item: i32,
    #[serde(default = "one")]
    pub count: i16,
}

fn one() -> i16 {
    1
}

/// The prizes of each tier.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct GachaponPool {
    #[serde(default)]
    pub common: Vec<GachaponPrize>,
    #[serde(default)]
    pub uncommon: Vec<GachaponPrize>,
    #[serde(default)]
    pub rare: Vec<GachaponPrize>,
}

impl GachaponPool {
    fn tier(&self, tier: GachaponTier) -> &[GachaponPrize] {
        match tier {
            GachaponTier::Common => &self.common,
            GachaponTier::Uncommon => &self.uncommon,
            GachaponTier::Rare => &self.rare,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct GachaponFile {
    #[serde(default)]
    global: GachaponPool,
    #[serde(default)]
    machines: BTreeMap<String, GachaponPool>,
}

/// Gachapon prizes: the pack's `gachapon.json`, else our committed
/// stand-ins (`gachapon_placeholder.toml`). Every machine draws from the
/// global prizes and its own.
#[derive(Debug, Default)]
pub struct GachaponTable {
    global: GachaponPool,
    machines: BTreeMap<i32, GachaponPool>,
    from_pack: bool,
}

impl GachaponTable {
    pub fn load(pack: Option<&Path>) -> Result<Self, GameDataError> {
        let path = pack.map(|p| p.join("gachapon.json")).filter(|p| p.exists());
        let (file, from_pack): (GachaponFile, _) = match &path {
            Some(path) => {
                let file = std::fs::File::open(path)?;
                let parsed = serde_json::from_reader(std::io::BufReader::new(file))
                    .map_err(|e| GameDataError::InvalidData(format!("{}: {e}", path.display())))?;
                (parsed, true)
            }
            None => (
                toml::from_str(GACHAPON_PLACEHOLDER).map_err(|e| {
                    GameDataError::InvalidData(format!("gachapon_placeholder.toml: {e}"))
                })?,
                false,
            ),
        };
        let mut machines = BTreeMap::new();
        for (npc, pool) in file.machines {
            machines.insert(parse_id(&npc, "gachapon machine")?, pool);
        }
        Ok(Self {
            global: file.global,
            machines,
            from_pack,
        })
    }

    /// The prizes of `tier` machine `npc_id` draws from: its own, then the
    /// global ones.
    pub fn prizes(&self, npc_id: i32, tier: GachaponTier) -> Vec<GachaponPrize> {
        let own = self
            .machines
            .get(&npc_id)
            .map_or(&[][..], |pool| pool.tier(tier));
        own.iter().chain(self.global.tier(tier)).copied().collect()
    }

    /// Whether the prizes came from the pack rather than the stand-ins.
    pub fn from_pack(&self) -> bool {
        self.from_pack
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
            keys::KPQ_NELLA_BONUS,
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
            keys::KPQ_CLOTO_S5_FULL,
            keys::JEFF_INTRO,
            keys::JEFF_ASK,
            keys::JEFF_DECLINE,
            keys::JEFF_TOO_WEAK,
            keys::MC_FIRST_BLOCKED,
            keys::MC_OBSTACLE_BLOCKED,
            keys::MC_OBSTACLE_USED,
            keys::MC_CASTLE_BLOCKED,
            keys::MC_SPORE_USED,
            keys::MC_THORNS_REMOVED,
            keys::JAMES_SCARED,
            keys::JAMES_SENT_BY_BROTHER,
            keys::JAMES_SAFE,
            keys::JAMES_THANKS,
            keys::JAMES_DISGUISE,
            keys::SCARRS_SPARE_SPORE,
            keys::SCARRS_SPORE_FULL,
            keys::REMEMBERER_CHARMS,
            keys::REMEMBERER_KARCASA,
            keys::CAMEL_CAB_KARCASA,
            keys::CAMEL_CAB_MEERKATS_ASK,
            keys::CAMEL_CAB_MEERKATS_GO,
            keys::CAMEL_CAB_MEERKATS_DONE,
            keys::CAMEL_CAB_SAHEL,
            keys::CAMEL_CAB_NOT_HERE,
            keys::CAMEL_CAB_PRIVATE,
            keys::CAMEL_CAB_LEVEL_30,
            keys::CAMEL_CAB_CASTLE_WAY,
            keys::HAN_ZENUMIST_ORES,
            keys::HAN_ALCADNO_ORES,
            keys::HAN_NO_ORES,
            keys::HAN_ZENUMIST_WAIT,
            keys::HAN_ALCADNO_WAIT,
            keys::CARSON_CAPE_LOST,
            keys::CARSON_CAPE_ASK,
            keys::CARSON_CAPE_GO,
            keys::MAED_CAPE_LOST,
            keys::MAED_CAPE_ASK,
            keys::MAED_CAPE_GO,
            keys::PARWEN_KNOWS_ASK,
            keys::DE_LANG_MISSING_STORY,
            keys::DE_LANG_MISSING_HELP,
            keys::DE_LANG_MISSING_ASK,
            keys::DE_LANG_MISSING_PENDANT,
            keys::DE_LANG_MISSING_PIPES,
            keys::DE_LANG_MISSING_GO,
            keys::PARWEN_DE_LANG_WANTS,
            keys::PARWEN_DE_LANG_WANTS_ASK,
            keys::DE_LANG_POTION_ASK,
            keys::PARWEN_PASSWORD_KEY,
            keys::PARWEN_PASSWORD_ASK,
            keys::PARWEN_PASSWORD_HURRY,
            keys::PARWEN_PASSWORD_CODE,
            keys::SECRET_PASSAGE_PASSWORD,
            keys::SECRET_PASSAGE_WRONG,
            keys::DR_KIM_THANKS,
            keys::DR_KIM_FULL,
            keys::MR_KU_THANKS,
            keys::MR_KU_FULL,
            keys::MUHAMAD_DEO_PEACEFUL,
            keys::MUHAMAD_MERCHANTS,
            keys::MUHAMAD_TRESPASSED,
            keys::MUHAMAD_QUEEN,
            keys::MUHAMAD_GUARDIANS,
            keys::MUHAMAD_GUARDIANS_WHERE,
            keys::MUHAMAD_EXPEDITION,
            keys::MUHAMAD_DEO_MONSTER,
            keys::MUHAMAD_CONVINCED,
            keys::ARDIN_TEST_AGAIN,
            keys::ARDIN_TEST_ASK,
            keys::ARDIN_TEST_GO,
            keys::ARDIN_TEST_BUSY,
            keys::SORCERER_POTION_WORKED,
            keys::METEORITE_FULL,
            keys::GRAY_GARBLED,
            keys::ELESKA_BOX_THANKS,
            keys::MANJI_DESERT_MISSION,
            keys::NAUTILUS_BUSH_SEARCH,
            keys::NAUTILUS_BUSH_SEARCHED,
            keys::NAUTILUS_BUSH_FULL,
            keys::NAUTILUS_ABEL_FOUND,
            keys::NAUTILUS_ABEL_OTHER,
            keys::NAUTILUS_ABEL_FULL,
            keys::RUMOR_BLACKBULL,
            keys::RUMOR_MANJI,
            keys::RUMOR_AYAN,
            keys::RUMOR_DANCES_WITH_BALROG,
            keys::RUMOR_BETTY,
            keys::TIENK_GREETING,
            keys::RHO_LAST_SONG,
            keys::NEW_LEAF_CITY_DECLINE,
            keys::BARRICADE_MARK_ASK,
            keys::BARRICADE_MARK_GO,
            keys::LUKAN_STORM_ASK,
            keys::LUKAN_STORM_GO,
            keys::TAGGRIN_WOODS_ASK,
            keys::TAGGRIN_WOODS_GO,
            keys::TAGGRIN_PATH_ASK,
            keys::TAGGRIN_PATH_GO,
            keys::TAGGRIN_WARRIORS_ASK,
            keys::TAGGRIN_WARRIORS_GO,
            keys::LITA_LEPRECHAUNS_ASK,
            keys::LITA_ELDERWRAITHS_ASK,
            keys::LITA_HORSEMAN_ASK,
            keys::LITA_BIGFOOT_ASK,
            keys::LITA_BOUNTY_GO,
            keys::NEW_LEAF_CITY_NEEDS_YOU,
            keys::NEW_LEAF_CITY_ETC_FULL,
            keys::BARRICADE_JACK_ASK,
            keys::BARRICADE_JACK_GO,
            keys::JACK_BROTHER,
            keys::JACK_MAP,
            keys::JACK_SETUP_FULL,
            keys::LUKAN_BADGES_ASK,
            keys::LUKAN_BADGES_GO,
            keys::LUKAN_BADGES_THANKS,
            keys::LUKAN_BADGES_MISSING,
            keys::JACK_LETTER_ASK,
            keys::JACK_LETTER_GO,
            keys::BARRICADE_GLYPHS_ASK,
            keys::BARRICADE_GLYPHS_GO,
            keys::ELPAM_TRANSLATE,
            keys::ELPAM_NO_LETTER,
            keys::BARRICADE_TRANSLATED_ASK,
            keys::JACK_TRANSLATION,
            keys::JACK_NO_TRANSLATION,
            keys::JACK_TIDE_ASK,
            keys::JACK_TIDE_GO,
            keys::JACK_KEYSTONE,
            keys::JACK_KEYSTONE_LOST,
            keys::JACK_HURRY,
            keys::ADVENTURER_TITLE,
            keys::ADVENTURER_MEDAL,
            keys::ADVENTURER_MEDAL_FULL,
            keys::SHANKS_FOR_LUCAS,
            keys::OUTSTANDING_CITIZEN_NOT_YET,
            keys::MEDAL_TITLE,
            keys::KNIGHT_TITLE,
            keys::CELEBRITY_START,
            keys::CELEBRITY_NOT_YET,
            keys::CELEBRITY_TOO_LATE,
            keys::VETERAN_HUNTER_START,
            keys::VETERAN_HUNTER_NOT_YET,
            keys::VETERAN_HUNTER_TOO_LATE,
            keys::MC_LETTER_ASK,
            keys::MC_LETTER_LATER,
            keys::MC_LETTER_FULL,
            keys::MC_LETTER_KINGDOM,
            keys::MC_LETTER_WRITTEN,
            keys::MC_LETTER_TAKE_YOU,
            keys::MC_LETTER_ROUTE,
            keys::MC_OFFICER_LETTER,
            keys::MC_OFFICER_LETTER_REPLY,
            keys::MC_OFFICER_WELCOME,
            keys::MC_FOREST_ASK,
            keys::MC_FOREST_DECLINE,
            keys::MC_FOREST_GO,
            keys::MC_WALLS_ASK,
            keys::MC_WALLS_GO,
            keys::MC_BARRIER_INVESTIGATED,
            keys::MC_WALLS_INVESTIGATED,
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
            keys::CORBA_MENU,
            keys::CORBA_CHOICE,
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
            keys::STYLIST_MENU,
            keys::STYLIST_CUT,
            keys::STYLIST_DYE,
            keys::STYLIST_FACE,
            keys::STYLIST_LENS,
            keys::STYLIST_ONE_TIME_LENS,
            keys::STYLIST_SKIN,
            keys::STYLIST_CHOOSE,
            keys::STYLIST_CHOOSE_LENS,
            keys::STYLIST_RANDOM,
            keys::STYLIST_DONE,
            keys::STYLIST_NO_COUPON,
            keys::STYLIST_NO_LENS,
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
            keys::LPQ_SIGN_MENU,
            keys::LPQ_SIGN_ENTER,
            keys::LPQ_SIGN_ABOUT,
            keys::LPQ_SIGN_INFO,
            keys::LPQ_SIGN_NO_PARTY,
            keys::LPQ_SIGN_NOT_LEADER,
            keys::LPQ_SIGN_NOT_ELIGIBLE,
            keys::LPQ_SIGN_BUSY,
            keys::LPQ_ANDERSON_LEAVE_ASK,
            keys::LPQ_ANDERSON_HOME,
            keys::LPQ_BALLOON_INTRO_1,
            keys::LPQ_BALLOON_INTRO_2,
            keys::LPQ_BALLOON_INTRO_3,
            keys::LPQ_BALLOON_INTRO_4,
            keys::LPQ_BALLOON_INTRO_5,
            keys::LPQ_BALLOON_INTRO_7,
            keys::LPQ_BALLOON_INTRO_9,
            keys::LPQ_BALLOON_NOT_LEADER,
            keys::LPQ_BALLOON_SHORT,
            keys::LPQ_BALLOON_DONE,
            keys::LPQ_BALLOON_HURRY,
            keys::LPQ_BALLOON_S6,
            keys::LPQ_BALLOON_INTRO_8,
            keys::LPQ_BALLOON_FEW,
            keys::LPQ_BALLOON_WRONG,
            keys::LPQ_PINK_LEAVE_ASK,
            keys::LPQ_PINK_STAY,
            keys::LPQ_ARTURO_THANKS,
            keys::LPQ_ARTURO_FULL,
            keys::LPQ_NOTICE_TRAP,
            keys::LPQ_NOTICE_ROMBAD,
            keys::LPQ_NOTICE_ALISHAR,
            keys::OPQ_WONKY_MENU,
            keys::OPQ_WONKY_ENTER,
            keys::OPQ_WONKY_ABOUT,
            keys::OPQ_WONKY_INFO,
            keys::OPQ_WONKY_NO_PARTY,
            keys::OPQ_WONKY_NOT_LEADER,
            keys::OPQ_WONKY_NOT_ELIGIBLE,
            keys::OPQ_WONKY_BUSY,
            keys::OPQ_WONKY_LEAVE_ASK,
            keys::OPQ_EAK_HOME,
            keys::OPQ_NOTICE_LIGHT,
            keys::OPQ_EAK_ARRIVAL,
            keys::OPQ_EAK_CLOUD,
            keys::OPQ_EAK_LEADER_ONLY,
            keys::OPQ_EAK_GATHER,
            keys::OPQ_NOTICE_STORAGE_CLOSED,
            keys::OPQ_NOTICE_JAIL_LEVERS,
            keys::PET_NOTE_ASK,
            keys::PET_NOTE_FULL,
            keys::PET_NOTE_HELD,
            keys::PET_NOTE_NOTHING,
            keys::PET_SECRET_NO_SNACK,
            keys::PET_EVOLUTION_ASK,
            keys::PET_EVOLUTION_DECLINE,
            keys::PET_EVOLUTION_DONE,
            keys::PET_EVOLUTION_NO_MESOS,
            keys::PET_EVOLUTION_NO_PET,
            keys::PET_EVOLUTION_REFUSED,
            keys::GACHAPON_ASK,
            keys::GACHAPON_WON,
            keys::GACHAPON_FULL,
            keys::GACHAPON_NO_TICKET,
            keys::GACHAPON_MENU,
            keys::GACHAPON_WHAT_CHOICE,
            keys::GACHAPON_WHAT,
            keys::GACHAPON_WHERE_CHOICE,
            keys::GACHAPON_WHERE,
            keys::OPQ_EAK_HAND_IN,
            keys::OPQ_EAK_PIECE,
            keys::OPQ_EAK_ROOM_CLEARED,
            keys::OPQ_EAK_FULL,
            keys::OPQ_EAK_MUSIC_ASK,
            keys::OPQ_EAK_MUSIC_PLAYED,
            keys::OPQ_EAK_MUSIC_DONE,
            keys::OPQ_EAK_HEADS_FEW,
            keys::OPQ_EAK_HEADS_ONE_RIGHT,
            keys::OPQ_EAK_HEADS_ALL_WRONG,
            keys::OPQ_EAK_HEADS_RIGHT,
            keys::OPQ_EAK_HEADS_DONE,
            keys::OPQ_EAK_LEVERS_ASK,
            keys::OPQ_EAK_LEVERS_ONE_RIGHT,
            keys::OPQ_EAK_LEVERS_NONE_RIGHT,
            keys::OPQ_EAK_LEVERS_RIGHT,
            keys::OPQ_EAK_LEVERS_DONE,
            keys::OPQ_EAK_GARDEN,
            keys::OPQ_EAK_GRASS,
            keys::OPQ_EAK_MINERVA,
            keys::OPQ_NOTICE_PAPA_PIXIE,
            keys::OPQ_NOTICE_GARDEN_OUT,
            keys::OPQ_MINERVA_ASK,
            keys::OPQ_MINERVA_THANKS,
            keys::OPQ_MINERVA_FULL,
            keys::OPQ_EAK_STORAGE,
            keys::OPQ_MINERVA_BONUS_ASK,
            keys::OPQ_EAK_SALION_FIND,
            keys::OPQ_EAK_SALION_FOUND,
        ];
        for key in named
            .iter()
            .chain(&keys::ROGER_START)
            .chain(&keys::ROGER_END)
            .chain(&keys::SHANKS_LETTER)
            .chain(&keys::MAKER_ALCHEMY)
            .chain(&keys::MAKER_SCIENCE)
            .chain(&keys::MAKER_PRACTICE)
            .chain(&keys::MAKER_CRYSTAL_TEST)
            .chain(&keys::MAKER_GOLD_ANVIL)
            .chain(&keys::DYLE_INFORMANTS)
            .chain(&keys::CAPTAIN_AL_JUNIOR)
            .chain(&keys::CAPTAIN_AL_REP)
            .chain(&keys::CAPTAIN_AL_ENTITLEMENT)
            .chain(&[
                keys::REEF_THANKS,
                keys::TRISTAN_ROOM_TAKEN,
                keys::DOLLHOUSE_HINT,
                keys::DOLLHOUSE_ENTER,
                keys::DOLLHOUSE_BUSY,
                keys::DOLLHOUSE_WRONG,
                keys::DOLLHOUSE_FOUND,
                keys::DOLLHOUSE_NOT_FOUND,
                keys::DOLLHOUSE_BACK,
                keys::MECHANICAL_PARTS_ENTER,
                keys::MECHANICAL_PARTS_BUSY,
                keys::MECHANICAL_PARTS_DONE,
                keys::MECHANICAL_PARTS_NO_ROOM,
                keys::MECHANICAL_PARTS_LEAVE,
                keys::KING_PEPE_BUSY,
                keys::KING_PEPE_KEY,
                keys::KING_PEPE_KEY_NO_ROOM,
                keys::WEDDING_HALL_LOCKED,
                keys::WEDDING_HALL_BUSY,
                keys::VIOLETTA_FOUND,
                keys::VIOLETTA_HELP,
                keys::VIOLETTA_PRIME_MINISTER,
                keys::VIOLETTA_PRIME_MINISTER_BEATEN,
                keys::VIOLETTA_FACE,
                keys::VIOLETTA_SEAL,
                keys::VIOLETTA_SEAL_FULL,
                keys::VIOLETTA_SEAL_KEPT,
                keys::VIOLETTA_MINIONS,
                keys::VIOLETTA_KEY,
                keys::VIOLETTA_KEY_LUCK,
                keys::VIOLETTA_KEY_FULL,
                keys::SNOW_STATUE_CLUE,
                keys::LOST_MEMORY_REGAINED,
                keys::BLACK_SHADOW_NOTHING,
                keys::BLACK_SHADOW_SPOTTED,
                keys::BLACK_SHADOW_AMBUSH,
                keys::MAGIC_LIBRARY_MUSHROOMS_LEFT,
                keys::PUPPETEER_DOOR_ASK,
                keys::PUPPETEER_DOOR_WRONG,
                keys::PUPPETEER_DOOR_SHUT,
                keys::PUPPETEER_DOOR_SEARCHED,
                keys::FRANCIS_INTRUDER,
                keys::FRANCIS_HIDEOUT_BUSY,
                keys::FAKE_TRU_CALL,
                keys::TRU_PUPPETEER_BEATEN,
                keys::TRU_POLEARM_MASTERY,
                keys::TRU_PUPPETEER_TRACES,
                keys::TRU_COMBO_DRAIN[0],
                keys::TRU_COMBO_DRAIN[1],
                keys::TRU_SEAL_STONE[0],
                keys::TRU_SEAL_STONE[1],
                keys::TRU_SEAL_STONE_FULL,
                keys::LILIN_SEAL_STONE,
                keys::TRU_ORBIS[0],
                keys::TRU_ORBIS[1],
                keys::TRU_ORBIS[2],
                keys::TRU_ORBIS_DECLINED,
                keys::PASSAGE_STATUE_ASK,
                keys::PASSAGE_STATUE_BUSY,
                keys::PASSAGE_EXIT_ASK,
                keys::SPIRUNA_SEALED_GARDEN[0],
                keys::SPIRUNA_SEALED_GARDEN[1],
                keys::SPIRUNA_SEALED_GARDEN_DECLINED,
                keys::SPIRUNA_GARDEN_CLEARED,
                keys::TRU_ORBIS_STONE_STOLEN,
                keys::LILIN_COMBO_SMASH[0],
                keys::LILIN_COMBO_SMASH[1],
                keys::TRU_MU_LUNG[0],
                keys::TRU_MU_LUNG[1],
                keys::TRU_MU_LUNG[2],
                keys::TRU_MU_LUNG_DECLINED,
                keys::MR_DO_BLACK_SCROLL[0],
                keys::MR_DO_BLACK_SCROLL[1],
                keys::MR_DO_SCROLL_SOAKED[0],
                keys::MR_DO_SCROLL_SOAKED[1],
                keys::MR_DO_SCROLL_FULL,
                keys::MR_DO_SCROLL_READ,
                keys::SEALED_GARDEN_OCCUPIED,
                keys::BACK_STREET_NO_ENTRY,
                keys::MU_GONG_TEST,
                keys::MU_GONG_ARENA_BUSY,
                keys::MU_GONG_SHADOW_KNIGHT[0],
                keys::MU_GONG_SHADOW_KNIGHT[1],
                keys::MU_GONG_STONE_TAKEN[0],
                keys::MU_GONG_STONE_TAKEN[1],
                keys::SHRINE_PILLAR_ASK,
                keys::SHRINE_PILLAR_WRONG,
                keys::SHRINE_PILLAR_SHUT,
                keys::SHRINE_PILLAR_BUSY,
                keys::LILIN_FINAL_CHARGE[0],
                keys::LILIN_FINAL_CHARGE[1],
                keys::TRU_BACK_TO_THE_PAST[0],
                keys::TRU_BACK_TO_THE_PAST[1],
                keys::ATHENA_REMEMBERS[0],
                keys::ATHENA_REMEMBERS[1],
                keys::ATHENA_LETTER_PROMISE[0],
                keys::ATHENA_LETTER_PROMISE[1],
                keys::ATHENA_LETTER,
                keys::ATHENA_LETTER_FULL,
                keys::NEINHEART_LETTER,
                keys::LILIN_NEW_BEGINNINGS[0],
                keys::LILIN_NEW_BEGINNINGS[1],
                keys::LILIN_NEW_BEGINNINGS[2],
                keys::LILIN_NEW_BEGINNINGS[3],
                keys::LILIN_NEW_BEGINNINGS_DECLINED,
                keys::LILIN_NEW_BEGINNINGS_FULL,
                keys::PUO_TARGET_PIGS[0],
                keys::PUO_TARGET_PIGS[1],
                keys::PUO_TARGET_PIGS[2],
                keys::PUO_TARGET_PIGS_DECLINED,
                keys::PUO_COMBO_ABILITY[0],
                keys::PUO_COMBO_ABILITY[1],
                keys::PUO_COMBO_ABILITY[2],
                keys::PUO_COMBO_ABILITY[3],
                keys::PUO_COMBO_ABILITY_FULL,
                keys::LILIN_BABY_STEPS[0],
                keys::LILIN_BABY_STEPS[1],
                keys::LILIN_BABY_STEPS[2],
                keys::BRUCE_FIRST_ASSIGNMENT[0],
                keys::BRUCE_FIRST_ASSIGNMENT[1],
                keys::BRUCE_FIRST_ASSIGNMENT[2],
                keys::BRUCE_FIRST_ASSIGNMENT[3],
                keys::ROWEN_SECOND_ASSIGNMENT[0],
                keys::ROWEN_SECOND_ASSIGNMENT[1],
                keys::ROWEN_SECOND_ASSIGNMENT[2],
                keys::ROWEN_SECOND_ASSIGNMENT[3],
                keys::ASSIGNMENT_DONE_DECLINED,
                keys::SHADOW_INVITATION[0],
                keys::SHADOW_INVITATION[1],
                keys::SHADOW_INVITATION[2],
                keys::SHADOW_INVITATION_DECLINED,
                keys::CAVE_FRANCIS_WARNING[0],
                keys::CAVE_FRANCIS_WARNING[1],
                keys::CAVE_FRANCIS_OUT,
                keys::LILIN_POLEARM_BOOSTER[0],
                keys::LILIN_POLEARM_BOOSTER[1],
                keys::LILIN_POLEARM_BOOSTER[2],
                keys::LILIN_POLEARM_BOOSTER[3],
                keys::LILIN_POLEARM_BOOSTER[4],
                keys::LILIN_POLEARM_BOOSTER[5],
                keys::ILJI_STATUE,
                keys::TEO_SUSPICIOUS_JOHN[0],
                keys::TEO_SUSPICIOUS_JOHN[1],
                keys::TEO_JOHN_CLEARED[0],
                keys::TEO_JOHN_CLEARED[1],
                keys::JOHNS_BOX_MEDICINE,
                keys::JOHNS_BOX_FULL,
                keys::PUCCI_WOLF_REQUEST[0],
                keys::PUCCI_WOLF_REQUEST[1],
                keys::PUCCI_WOLF_REQUEST[2],
                keys::PUCCI_WOLF_REQUEST_DECLINED,
                keys::SCADUR_WOLF_MOUNT[0],
                keys::SCADUR_WOLF_MOUNT[1],
                keys::FIELD_OF_WOLVES_CLEARED,
                keys::WOLF_TEST[0],
                keys::WOLF_TEST[1],
                keys::WOLF_TEST[2],
                keys::WOLF_TEST_DECLINED,
                keys::WOLF_TEST_BUSY,
                keys::WOLF_TEST_AGAIN,
                keys::MAHA_WOLF_EVOLUTION[0],
                keys::MAHA_WOLF_EVOLUTION[1],
                keys::MAHA_WOLF_EVOLUTION[2],
                keys::MAHA_WOLF_EVOLUTION_UNEQUIP,
                keys::MAHA_WOLF_EVOLUTION_DONE,
                keys::MAHA_WOLF_EVOLUTION_FULL,
                keys::RUSSELLON_PILL_TAKEN,
                keys::RUSSELLON_PILL_FULL,
                keys::RUSSELLON_PILL_NOT_TAKEN,
                keys::RUSSELLON_PILL_DONE,
                keys::RUSSELLON_REWARD_FULL,
                keys::YULETE_MARBLES_EYE,
                keys::YULETE_MARBLES_ROCK,
                keys::YULETE_MARBLES_MORE,
                keys::YULETE_MARBLES_FULL,
                keys::KARCASA_SILK_STRANGER,
                keys::KARCASA_SILK_TIGUN,
                keys::KARCASA_SILK_HANDED,
                keys::KARCASA_SILK_GIVEN,
                keys::KARCASA_SILK_FULL,
                keys::ADONIS_SPIRITS_TALE,
                keys::ADONIS_SPIRITS_RITUAL,
                keys::SWALLOW_NEW_SEED,
                keys::SWALLOW_NEW_SEED_FULL,
                keys::PIA_SHELL_WISH,
                keys::PIA_SHELL_MISSING,
                keys::JUNE_MENU,
                keys::JUNE_HELPING_BLAKE,
                keys::JUNE_VIP,
                keys::JUNE_NOT_HELPING,
                keys::JUNE_BUSY,
                keys::JUNE_VIP_OVER_LEVEL,
                keys::JUNE_VIP_INVITATIONS,
                keys::JUNE_NOT_VIP,
                keys::JUNE_VIP_MISSING,
                keys::JUNE_VIP_WELCOME,
                keys::DEPART_NO_ACCESS,
                keys::BOB_BABY_BIRD_OFFER[0],
                keys::BOB_BABY_BIRD_OFFER[1],
                keys::BOB_BABY_BIRD_OFFER[2],
                keys::BOB_BABY_BIRD_DECLINED,
                keys::BOB_BABY_BIRD_GIVEN[0],
                keys::BOB_BABY_BIRD_GIVEN[1],
                keys::BOB_BABY_BIRD_FULL,
                keys::BOB_BABY_BIRD_GROWN,
                keys::NINE_SPIRIT_HATCHED,
                keys::NINE_SPIRIT_STONE_HELD,
                keys::NINE_SPIRIT_FULL,
                keys::DYLE_TRASH_CAN_NOT_YET,
                keys::DYLE_TRASH_CAN_FULL,
                keys::DYLE_TRASH_CAN_FOUND,
                keys::DYLE_TRASH_CAN_SHORT,
                keys::DYLE_TRASH_CAN_PAID,
            ])
            .chain(&keys::MILK_TANGYOON)
            .chain(&keys::DOJO_TAUNT)
            .chain(&keys::FITNESS_NOTICE)
            .chain(&keys::OLA_NOTICE)
            .chain(&keys::JEWEL_NOTICE)
            .chain(&keys::MILK_COW_FILLS)
            .chain(&keys::PET_SECRET)
            .chain(&keys::HPQ_TORY_ABOUT_PAGES)
            .chain(&keys::HPQ_GROWLIE_ABOUT_PAGES)
            .chain(&keys::ENGAGEMENT)
            .chain(&keys::RESERVATIONS)
            .chain(&keys::INVITATIONS)
            .chain(&keys::CEREMONY)
            .chain(&keys::GIFTS)
            .chain(&keys::ZAKUM)
            .chain(&keys::SQUAD)
            .chain(&keys::PAPULATUS)
            .chain(&keys::HORNTAIL)
            .chain(&keys::HTPQ)
            .chain(&[
                keys::AREA_BOSS_APPEARED,
                keys::JONATHAN_HELLO,
                keys::JONATHAN_TEST_ASK,
                keys::JONATHAN_TEST_BUSY,
                keys::JONATHAN_BART_FOUND,
                keys::KYRIN_TEST_ASK,
                keys::KYRIN_TEST_BUSY,
                keys::KYRIN_TEST_WAIT,
                keys::KYRIN_TEST_PASSED,
                keys::BALROG_WORLD_NOT_ALLOWED,
                keys::BALROG_WORLD_JOB,
                keys::BALROG_WORLD_LEVEL,
                keys::BALROG_WORLD_BUSY,
                keys::PAST_PAPULATUS_SEND_BACK,
                keys::DELLI_SHULYNCH_ASK,
                keys::DELLI_SHULYNCH_LEAVE,
                keys::DELLI_MENU,
                keys::DELLI_ENTER,
                keys::DELLI_ABOUT,
                keys::DELLI_INFO,
                keys::DELLI_NO_PARTY,
                keys::DELLI_NOT_LEADER,
                keys::DELLI_NOT_ELIGIBLE,
                keys::DELLI_BUSY,
                keys::DELLI_NOT_WON,
                keys::DELLI_HOME_ASK,
                keys::LUDIMAZE_MENU,
                keys::LUDIMAZE_ENTER,
                keys::LUDIMAZE_ABOUT,
                keys::LUDIMAZE_INFO,
                keys::LUDIMAZE_NO_PARTY,
                keys::LUDIMAZE_NOT_LEADER,
                keys::LUDIMAZE_NOT_ELIGIBLE,
                keys::LUDIMAZE_BUSY,
                keys::LUDIMAZE_HOME,
                keys::LUDIMAZE_SUMMONED,
                keys::LUDIMAZE_PIETRI_NOT_LEADER,
                keys::LUDIMAZE_PIETRI_WAIT,
                keys::LUDIMAZE_PIETRI_SHORT,
                keys::LUDIMAZE_PIETRI_ASK,
                keys::LUDIMAZE_PIETRI_DONE,
                keys::LUDIMAZE_PRESENT_ASK,
                keys::LUDIMAZE_PRESENT_FULL,
                keys::TYLUS_MENU,
                keys::TYLUS_ENTER,
                keys::TYLUS_ABOUT,
                keys::TYLUS_INFO,
                keys::TYLUS_NO_PARTY,
                keys::TYLUS_NOT_LEADER,
                keys::TYLUS_NOT_ELIGIBLE,
                keys::TYLUS_BUSY,
                keys::TYLUS_FALLEN,
                keys::TYLUS_THANKS,
                keys::ELLIN_MENU,
                keys::ELLIN_ENTER,
                keys::ELLIN_ABOUT,
                keys::ELLIN_INFO,
                keys::ELLIN_NO_PARTY,
                keys::ELLIN_NOT_LEADER,
                keys::ELLIN_NOT_ELIGIBLE,
                keys::ELLIN_BUSY,
                keys::ELLIN_STAGE1_SHUT,
                keys::ELLIN_STAGE2_SHUT,
                keys::ELLIN_GUIDE_LEAVE,
                keys::ELLIN_GUIDE_WELCOME,
                keys::ELLIN_GUIDE_MAZE_END,
                keys::ELLIN_GUIDE_FLOWERS,
                keys::ELLIN_GUIDE_MARBLES,
                keys::ELLIN_GUIDE_NO_ROOM,
                keys::ELLIN_GUIDE_FLOWERS_LEAVE,
                keys::ELLIN_GUIDE_ALL_CAUGHT,
                keys::ELLIN_GUIDE_HAND_IN,
                keys::ELLIN_GUIDE_TO_LEADER,
                keys::ELLIN_MILEPOST_LEAVE,
                keys::ELLIN_SPRITE_TO_ALTAR,
                keys::ELLIN_SPRITE_LEAVE,
                keys::ELLIN_ALTAR_GOLEM,
                keys::ELLIN_STAGE6_SHUT,
                keys::ELLIN_STAGE6_WAIT,
                keys::ELLIN_HOME_NO_ROOM,
                keys::ELLIN_PRIZE,
                keys::ELLIN_PRIZE_MENU,
                keys::ELLIN_PRIZE_ALTAIRE,
                keys::ELLIN_PRIZE_GLITTERING,
                keys::ELLIN_PRIZE_BRILLIANT,
                keys::ELLIN_PRIZE_NO_FRAGMENTS,
                keys::ELLIN_PRIZE_NO_ALTAIRE,
                keys::ELLIN_PRIZE_NO_GLITTERING,
                keys::ELLIN_PRIZE_NO_ROOM,
                keys::MAGATIA_ENTER,
                keys::MAGATIA_ABOUT,
                keys::MAGATIA_NO_PARTY,
                keys::MAGATIA_NOT_LEADER,
                keys::MAGATIA_NOT_ELIGIBLE,
                keys::MAGATIA_BUSY,
                keys::MAGATIA_ROMEO_MENU,
                keys::MAGATIA_ROMEO_INFO,
                keys::MAGATIA_ROMEO_RETREAT,
                keys::MAGATIA_JULIET_MENU,
                keys::MAGATIA_JULIET_INFO,
                keys::MAGATIA_JULIET_RETREAT,
                keys::MAGATIA_BOOK_NOTHING,
                keys::MAGATIA_BOOK_MESOS,
                keys::MAGATIA_BOOK_EXP,
                keys::MAGATIA_BOOK_LETTER,
                keys::MAGATIA_BOOK_LETTER_FULL,
                keys::MAGATIA_BOOK_PASS,
                keys::MAGATIA_PORTAL_SHUT,
                keys::MAGATIA_LETTER,
                keys::MAGATIA_ROMEO_SHOCKED,
                keys::MAGATIA_JULIET_SHOCKED,
                keys::MAGATIA_OPEN_DOOR,
                keys::MAGATIA_ROMEO_HURRY,
                keys::MAGATIA_JULIET_HURRY,
                keys::MAGATIA_FILES,
                keys::MAGATIA_FILES_LEADER,
                keys::MAGATIA_YULETE_LEAVING,
                keys::MAGATIA_YULETE_WATCHED,
                keys::MAGATIA_YULETE_SNEAKY,
                keys::MAGATIA_YULETE_CAUGHT,
                keys::MAGATIA_YULETE_DISGUSTED,
                keys::MAGATIA_YULETE_FOOLED,
                keys::MAGATIA_ROOM_TAKEN,
                keys::MAGATIA_YULETE_PRESENTS,
                keys::MAGATIA_YULETE_MIGHTY,
                keys::MAGATIA_YULETE_BEHOLD,
                keys::MAGATIA_YULETE_BEATEN,
                keys::MAGATIA_FRANKENROID,
                keys::MAGATIA_FRANKENROID_MIGHTY,
                keys::MAGATIA_JULIET_FAINTED,
                keys::MAGATIA_ROMEO_FAINTED,
                keys::MAGATIA_ROMEO_SAVED,
                keys::MAGATIA_JULIET_SAVED,
                keys::MAGATIA_ROMEO_SAVED_HURT,
                keys::MAGATIA_JULIET_SAVED_HURT,
                keys::MAGATIA_GIFT,
                keys::MAGATIA_GIFT_FULL,
                keys::MAGATIA_TRACES_DEFEATED,
                keys::MAGATIA_TRACES_CHEER,
                keys::MAGATIA_TRACES_SORRY,
                keys::MAGATIA_THANKS_REHAB,
                keys::MAGATIA_THANKS_JAIL,
                keys::MAGATIA_THANKS_FULL,
                keys::MAGATIA_PIPE_DARK,
                keys::PIRATE_MENU,
                keys::PIRATE_ENTER,
                keys::PIRATE_ABOUT,
                keys::PIRATE_INFO,
                keys::PIRATE_NO_PARTY,
                keys::PIRATE_NOT_LEADER,
                keys::PIRATE_NOT_ELIGIBLE,
                keys::PIRATE_BUSY,
                keys::PIRATE_GUON_LEAVE,
                keys::PIRATE_GUON_BOXES,
                keys::PIRATE_GUON_MARKS,
                keys::PIRATE_GUON_DECK,
                keys::PIRATE_GUON_BONUS,
                keys::PIRATE_GUON_CREW,
                keys::PIRATE_GUON_DOORS,
                keys::PIRATE_GUON_BOSS,
                keys::PIRATE_PORTAL_SHUT,
                keys::PIRATE_GUON_RISING,
                keys::PIRATE_GUON_VETERAN,
                keys::PIRATE_GUON_MARKS_IN,
                keys::PIRATE_MARKS_SHUT,
                keys::PIRATE_GUON_CHEST,
                keys::PIRATE_WU_FREED,
                keys::PIRATE_WU_WAIT,
                keys::PIRATE_WU_THANKS,
                keys::PIRATE_WU_MENU,
                keys::PIRATE_WU_OUT,
                keys::PIRATE_WU_HAT,
                keys::PIRATE_WU_ROOM_ETC,
                keys::PIRATE_WU_ROOM_EQUIP,
                keys::PIRATE_WU_HAT_GIVEN,
                keys::PIRATE_WU_NEED_FEATHERS,
                keys::PIRATE_WU_BEST,
                keys::PIRATE_GUON_SAVED,
                keys::DOJO_SO_GONG_MENU,
                keys::DOJO_SO_GONG_ALONE,
                keys::DOJO_SO_GONG_PARTY,
                keys::DOJO_SO_GONG_PARTY_LEADER,
                keys::DOJO_SO_GONG_PARTY_SPREAD,
                keys::DOJO_SO_GONG_ABOUT,
                keys::DOJO_SO_GONG_INFO,
                keys::DOJO_SO_GONG_LOW_LEVEL,
                keys::DOJO_SO_GONG_BUSY,
                keys::DOJO_SO_GONG_QUIT,
                keys::DOJO_SO_GONG_QUIT_NO,
                keys::DOJO_SO_GONG_QUIT_DONE,
                keys::DOJO_SO_GONG_REST,
                keys::DOJO_SO_GONG_CONTINUE,
                keys::DOJO_SO_GONG_LEAVE,
                keys::DOJO_SO_GONG_LEAVE_ASK,
                keys::DOJO_BOARD_MENU,
                keys::DOJO_BOARD_CHALLENGE,
                keys::DOJO_BOARD_READ,
                keys::DOJO_BOARD_NOTICE_0,
                keys::DOJO_BOARD_NOTICE_1,
                keys::DOJO_BOARD_ASK,
                keys::DOJO_BOARD_DECLINE,
                keys::MIRROR_DOJO,
                keys::MIRROR_CARNIVAL,
                keys::MIRROR_PYRAMID,
                keys::MIRROR_NOWHERE,
                keys::LINE_RYKO,
                keys::LINE_ADONIS,
                keys::LINE_DOCUMENT_ROLL,
                keys::LINE_PRACTICE_CHART,
                keys::LINE_KNIGHT_ARMOR,
                keys::LINE_SHAREN_WILL,
                keys::LINE_GRAVE_LIGHT,
                keys::LINE_ENIGMA_TOMBSTONE,
                keys::LINE_WORN_TOMBSTONE,
                keys::DOJO_UP_MONSTERS,
                keys::DOJO_NEXT_SHUT,
                keys::DOJO_POINTS,
                keys::DOJO_ENERGY_RESET,
                keys::DOJO_SO_GONG_RECORD,
                keys::DOJO_SO_GONG_RECORD_ASK,
                keys::DOJO_SO_GONG_RECORD_NO,
                keys::DOJO_SO_GONG_RECORDED,
                keys::DOJO_SO_GONG_RECORD_KEPT,
                keys::DOJO_SO_GONG_RESUME,
                keys::DOJO_SO_GONG_BELT,
                keys::DOJO_SO_GONG_RESET,
                keys::DOJO_SO_GONG_MEDAL,
                keys::DOJO_SO_GONG_ROOM,
                keys::DOJO_SO_GONG_RESET_ASK,
                keys::DOJO_SO_GONG_RESET_DONE,
                keys::DOJO_SO_GONG_RESET_NO,
                keys::DOJO_SO_GONG_BELTS,
                keys::DOJO_SO_GONG_BELT_CHOICE,
                keys::DOJO_SO_GONG_BELT_HELD,
                keys::DOJO_SO_GONG_BELT_GIVEN,
                keys::DOJO_SO_GONG_BELT_NEEDS,
                keys::DOJO_SO_GONG_BELT_BEFORE,
                keys::DOJO_SO_GONG_BELT_LACKING,
                keys::DOJO_SO_GONG_BELT_LACKS_BELT,
                keys::DOJO_SO_GONG_BELT_LACKS_POINTS,
                keys::DOJO_SO_GONG_BELT_AND,
                keys::DOJO_SO_GONG_MEDAL_TRY,
                keys::DOJO_SO_GONG_MEDAL_NO,
                keys::DOJO_SO_GONG_MEDAL_ALL,
                keys::DOJO_SO_GONG_MEDAL_MORE,
                keys::DOJO_SO_GONG_MEDAL_GIVEN,
                keys::DOJO_SO_GONG_MEDAL_ROOM,
                keys::DOJO_SO_GONG_TRIAL_ASK,
                keys::DOJO_SO_GONG_TRIAL_NO,
                keys::DOJO_TRIAL_TAUNT,
                keys::DOJO_TRIAL_STAY,
                keys::PYRAMID_DUARTE_MENU,
                keys::PYRAMID_DUARTE_ABOUT,
                keys::PYRAMID_DUARTE_ENTER,
                keys::PYRAMID_DUARTE_TREASURES,
                keys::PYRAMID_DUARTE_INFO,
                keys::PYRAMID_DUARTE_RULES,
                keys::PYRAMID_DUARTE_LUCK,
                keys::PYRAMID_DUARTE_TREASURES_INFO,
                keys::PYRAMID_DUARTE_HOW,
                keys::PYRAMID_DUARTE_ALONE,
                keys::PYRAMID_DUARTE_PARTY,
                keys::PYRAMID_DUARTE_MODE,
                keys::PYRAMID_DUARTE_LOW_LEVEL,
                keys::PYRAMID_DUARTE_ONLY_HELL,
                keys::PYRAMID_DUARTE_NO_PARTY,
                keys::PYRAMID_DUARTE_NOT_LEADER,
                keys::PYRAMID_DUARTE_PARTY_TERMS,
                keys::PYRAMID_DUARTE_BUSY,
                keys::PYRAMID_DUARTE_REFUSED,
                keys::PYRAMID_DUARTE_FORFEIT_ASK,
                keys::PYRAMID_DUARTE_RESULT_ASK,
                keys::PYRAMID_DUARTE_SHADES_ASK,
                keys::PYRAMID_DUARTE_MEDAL,
                keys::PYRAMID_DUARTE_MEDAL_HAVE,
                keys::PYRAMID_DUARTE_MEDAL_MORE,
                keys::PYRAMID_DUARTE_MEDAL_ROOM,
                keys::PYRAMID_DUARTE_MEDAL_GIVEN,
                keys::PYRAMID_DUARTE_TOMB,
                keys::PYRAMID_DUARTE_TOMB_GEM,
                keys::PYRAMID_DUARTE_TOMB_NO_GEM,
                keys::PYRAMID_DUARTE_TOMB_FULL,
                keys::PYRAMID_DUARTE_SHADES_PASSED,
                keys::PYRAMID_DUARTE_SHADES_NOW,
                keys::PYRAMID_DUARTE_SHADES_LATER,
                keys::PYRAMID_DUARTE_SHADES_GEM,
                keys::PYRAMID_DUARTE_SHADES_NO_ROOM,
                keys::CARNIVAL_SPIEGELMANN_MENU,
                keys::CARNIVAL_SPIEGELMANN_ENTER,
                keys::CARNIVAL_SPIEGELMANN_ABOUT,
                keys::CARNIVAL_SPIEGELMANN_INFO,
                keys::CARNIVAL_SPIEGELMANN_RULES,
                keys::CARNIVAL_SPIEGELMANN_ROOMS,
                keys::CARNIVAL_SPIEGELMANN_ROOM,
                keys::CARNIVAL_SPIEGELMANN_LEVELS,
                keys::CARNIVAL_SPIEGELMANN_NO_PARTY,
                keys::CARNIVAL_SPIEGELMANN_NOT_LEADER,
                keys::CARNIVAL_SPIEGELMANN_TERMS,
                keys::CARNIVAL_SPIEGELMANN_BUSY,
                keys::CARNIVAL_SPIEGELMANN_GO,
                keys::CARNIVAL_GUIDE_BACK,
                keys::CARNIVAL_SPIEGELMANN_WAITING,
                keys::CARNIVAL_SPIEGELMANN_TAKEN,
                keys::CARNIVAL_SPIEGELMANN_SENT,
                keys::CARNIVAL_SPIEGELMANN_UNEQUAL,
                keys::CARNIVAL_SPIEGELMANN_FACING,
                keys::CARNIVAL_SPIEGELMANN_FULL,
                keys::CARNIVAL_CHALLENGE_ASK,
                keys::CARNIVAL_CHALLENGE_MEMBER,
                keys::CARNIVAL_CHALLENGE_DECLINED,
                keys::CARNIVAL_CHALLENGE_UNEQUAL,
                keys::CARNIVAL_WAITING,
                keys::CARNIVAL_TEAM_RED,
                keys::CARNIVAL_TEAM_BLUE,
                keys::CARNIVAL_ROOM_WAIT,
                keys::CARNIVAL_ROOM_LEAVE,
                keys::CARNIVAL_OVERTIME,
                keys::CARNIVAL_GUIDE_WON,
                keys::CARNIVAL_GUIDE_LOST,
                keys::COCONUT_FELL_MAPLE,
                keys::COCONUT_FELL_STORY,
                keys::COCONUT_OVERTIME,
                keys::SNOWBALL_ENDED,
                keys::LEVEL_200,
                keys::GM_EVENT_OPENED,
                keys::GM_EVENT_MENU,
                keys::GM_EVENT_ABOUT,
                keys::GM_EVENT_INFO,
                keys::GM_EVENT_GO,
                keys::GM_EVENT_NOT_OPEN,
                keys::GM_EVENT_PIETRO,
                keys::GM_EVENT_PIETRO_FULL,
                keys::GM_EVENT_PIETRA,
                keys::GM_EVENT_HARRY_ASK,
                keys::OX_WRONG,
                keys::OX_WON,
                keys::TOWER_ROCK_NONE,
                keys::TOWER_ROCK_MENU,
                keys::TOWER_ROCK_ASK,
                keys::PATIENCE_FULL,
                keys::LINE3_JAKE_HELLO,
                keys::LINE3_JAKE_MENU,
                keys::LINE3_JAKE_SHORT,
                keys::LINE3_JAKE_FULL,
                keys::LINE3_GATE_MENU,
                keys::LINE3_GATE_SITE,
                keys::LINE3_GATE_SUBWAY,
                keys::LINE3_GATE_TICKETS,
                keys::LINE3_GATE_NO_TICKET,
                keys::LINE3_CHEST_FULL,
                keys::LINE3_EXIT_SAY,
                keys::LINE3_EXIT_ASK,
                keys::LINE3_EXIT_STAY,
                keys::MAKER_PICK,
                keys::MAKER_ASK,
                keys::MAKER_MESO,
                keys::MAKER_SHORT,
                keys::MAKER_FULL,
                keys::MAKER_DONE,
                keys::MAKER_SERRYL_HELLO,
                keys::MAKER_SERRYL_KNUCKLERS,
                keys::MAKER_SERRYL_GUNS,
                keys::MAKER_SERRYL_GLOVES,
                keys::MAKER_NEVE_HELLO,
                keys::MAKER_NEVE_WARRIOR,
                keys::MAKER_NEVE_BOWMAN,
                keys::MAKER_NEVE_MAGICIAN,
                keys::MAKER_NEVE_THIEF,
                keys::MAKER_MOS_HELLO,
                keys::MAKER_MOS_WHAT,
                keys::MAKER_MOS_STIMULATOR,
                keys::MAKER_MOS_WARRIOR,
                keys::MAKER_MOS_BOWMAN,
                keys::MAKER_MOS_MAGICIAN,
                keys::MAKER_MOS_THIEF,
                keys::MAKER_MOS_PIRATE,
                keys::MAKER_MOS_WARRIOR_STIMULATED,
                keys::MAKER_MOS_BOWMAN_STIMULATED,
                keys::MAKER_MOS_MAGICIAN_STIMULATED,
                keys::MAKER_MOS_THIEF_STIMULATED,
                keys::MAKER_MOS_PIRATE_STIMULATED,
                keys::MAKER_MOS_DAGGER,
                keys::MAKER_MOS_LOST,
                keys::QUEST_INFO_OASIS,
                keys::QUEST_INFO_SECRET_WALL,
                keys::QUEST_INFO_WALL_ASK,
                keys::QUEST_INFO_WALL_READ,
                keys::QUEST_INFO_SHAMAN_ROCK,
                keys::QUEST_INFO_SHINY_STONE,
                keys::QUEST_INFO_HARP_MISSED,
                keys::QUEST_INFO_HARP_PLAYED,
                keys::QUEST_INFO_PENTAGRAM,
                keys::REFINER_WHAT,
                keys::REFINER_HOW_MANY,
                keys::REFINER_ASK,
                keys::REFINER_MINERAL,
                keys::REFINER_JEWEL,
                keys::REFINER_ROCK,
                keys::REFINER_CRYSTAL,
                keys::REFINER_MATERIALS,
                keys::REFINER_ARROWS,
                keys::REFINER_THUNDER_HELLO,
                keys::REFINER_THUNDER_HELMETS,
                keys::REFINER_THUNDER_SHIELDS,
                keys::REFINER_CHRIS_HELLO,
                keys::REFINER_CHRIS_HOOF,
                keys::REFINER_CHRIS_HOOF_ASK,
                keys::REFINER_VOGEN_HELLO,
                keys::REFINER_PI_HELLO,
                keys::REFINER_MUHAMAD_HELLO,
                keys::REFINER_MUHAMAD_BYE,
                keys::ALTAIRE_HELLO,
                keys::ALTAIRE_TRADE,
                keys::ALTAIRE_SHORT,
                keys::ALTAIRE_HOW_MANY,
                keys::ALTAIRE_DONE,
                keys::ALTAIRE_FULL,
                keys::REFINER_UPGRADE_CLAW,
                keys::REFINER_MAKE_CLAW,
                keys::REFINER_MAKE_GLOVE,
                keys::REFINER_UPGRADE_GLOVE,
                keys::REFINER_BOW,
                keys::REFINER_CROSSBOW,
                keys::REFINER_SMITH_HELLO,
                keys::REFINER_VICIOUS_HELLO,
                keys::REFINER_JM_HELLO,
                keys::MAKER_STIMULATOR,
                keys::MAKER_STIMULATOR_LOST,
                keys::MAKER_FRANCOIS_HELLO,
                keys::MAKER_CHRISHRAMA_HELLO,
                keys::MAKER_GORDON_HELLO,
                keys::MAKER_SARAH_HELLO,
                keys::MAKER_TARA_HELLO,
                keys::MAKER_RYDOLE_HELLO,
                keys::MAKER_MAKE_WAND,
                keys::MAKER_MAKE_STAFF,
                keys::MAKER_UPGRADE_HAT,
                keys::MAKER_SHOES_WARRIOR,
                keys::MAKER_SHOES_BOWMAN,
                keys::MAKER_SHOES_MAGICIAN,
                keys::MAKER_SHOES_THIEF,
                keys::MAKER_SHOES_WARRIOR_STIMULATED,
                keys::MAKER_SHOES_BOWMAN_STIMULATED,
                keys::MAKER_SHOES_MAGICIAN_STIMULATED,
                keys::MAKER_SHOES_THIEF_STIMULATED,
                keys::MAKER_GLOVES_WARRIOR,
                keys::MAKER_GLOVES_BOWMAN,
                keys::MAKER_GLOVES_MAGICIAN,
                keys::MAKER_GLOVES_THIEF,
                keys::MAKER_GLOVES_WARRIOR_UPGRADED,
                keys::MAKER_GLOVES_BOWMAN_UPGRADED,
                keys::MAKER_GLOVES_MAGICIAN_UPGRADED,
                keys::MAKER_GLOVES_THIEF_UPGRADED,
                keys::MILK_TANGYOON_FULL,
                keys::MILK_COW_LAST,
                keys::MILK_COW_NO_BOTTLE,
                keys::MILK_COW_FULL,
                keys::MILK_CALF_EMPTY,
                keys::MILK_CALF_DRINKS,
                keys::MILK_CALF_NO_BOTTLE,
                keys::MILK_JUG_NOT_FULL,
            ])
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
    fn ludibriums_stage_7_monsters_drop_the_other_pass() {
        let table = RewardTable::load(None).unwrap();
        for mob in 9300169..=9300171 {
            let [pass] = &table.drops(mob)[..] else {
                panic!("{mob}: {:?}", table.drops(mob));
            };
            assert_eq!(
                (pass.item, pass.prob, pass.quest),
                (Some(4001156), 1.0, None)
            );
        }
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

    #[test]
    fn every_salion_spot_drops_a_piece_of_minervas_statue() {
        let table = RewardTable::load(None).unwrap();
        for reactor_id in 2001002..=2001015 {
            let drops = table.reactor_drops(reactor_id);
            assert_eq!(drops.len(), 1, "{reactor_id}");
            assert_eq!((drops[0].item, drops[0].prob), (Some(4001045), 1.0));
            assert_eq!(drops[0].quest, None);
        }
    }

    #[test]
    fn the_orbis_records_and_stone_boxes_drop_without_the_pack() {
        let table = RewardTable::load(None).unwrap();
        let only = |reactor_id| {
            let drops = table.reactor_drops(reactor_id);
            assert_eq!(drops.len(), 1, "{reactor_id}");
            assert_eq!((drops[0].prob, drops[0].quest), (1.0, None));
            drops[0].item
        };
        // The records, Sunday's to Saturday's.
        for day in 0..7 {
            assert_eq!(only(2002004 + day), Some(4001056 + day));
        }
        assert_eq!(only(2002011), Some(4001046));
        assert_eq!(only(2002012), Some(4001047));
        assert_eq!(only(2002013), Some(4001049));
        // The garden's Grass of Life, and the seeds its mobs drop.
        assert_eq!(only(2002003), Some(4001055));
        for (mob, seed) in [(9300048, 4001053), (9300049, 4001074), (9300039, 4001054)] {
            let drops = table.drops(mob);
            assert_eq!(drops.len(), 1, "{mob}");
            assert_eq!((drops[0].item, drops[0].prob), (Some(seed), 1.0));
        }
    }

    #[test]
    fn the_dead_mines_keys_documents_and_fire_ore_drop_without_the_pack() {
        let table = RewardTable::load(None).unwrap();
        for (reactor_id, item) in [
            (2112004, 4001016),
            (2112011, 4001016),
            (2112005, 4001015),
            (2112012, 4001015),
            (2112014, 4001018),
        ] {
            let drops = table.reactor_drops(reactor_id);
            assert_eq!(drops.len(), 1, "{reactor_id}");
            assert_eq!((drops[0].item, drops[0].prob), (Some(item), 1.0));
        }
    }

    #[test]
    fn the_balrog_and_its_clear_maps_box_drop_without_the_pack() {
        let table = RewardTable::load(None).unwrap();
        let body = table.drops(8830000);
        assert_eq!(body.len(), 75);
        assert_eq!((body[0].money, body[0].prob), (Some(7000), 0.4));
        let leather = body.iter().find(|d| d.item == Some(4001261)).unwrap();
        assert_eq!(leather.prob, 0.6);
        let scrolls = table.reactor_drops(1052001);
        assert_eq!(scrolls.len(), 44);
        assert!((2040728..=2040739).all(|s| scrolls.iter().any(|d| d.item == Some(s))));
    }

    #[test]
    fn the_ghost_ship_and_its_captain_drop_their_essences_without_the_pack() {
        let table = RewardTable::load(None).unwrap();
        for (mob, essence) in [
            (9420509, 4000380),
            (9420510, 4000379),
            (9420511, 4000382),
            (9420512, 4000383),
            (9420513, 4000384),
        ] {
            let drops = table.drops(mob);
            let drop = drops.iter().find(|d| d.item == Some(essence)).unwrap();
            assert_eq!(drop.prob, 0.3, "{mob}");
        }
        assert_eq!(table.drops(9420510).len(), 33);
        let captain = table.drops(9420513);
        assert_eq!((captain[0].money, captain.len()), (Some(2000), 31));
        let potions = captain.iter().find(|d| d.item == Some(2000006)).unwrap();
        assert_eq!((potions.min, potions.max), (Some(1), Some(4)));
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
        // Without it, only the shopkeepers it lacks keep a shop.
        let additions = ShopTable::load(None).unwrap();
        assert_eq!(additions.len(), 14);
        assert_eq!(
            additions.shop(1091000).unwrap()[0],
            ShopEntry {
                item: 1492000,
                price: 3000
            }
        );
        assert!(additions.shop(1011100).is_none());

        let dir = std::env::temp_dir().join(format!("rustms-shops-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("shops.json"),
            r#"{"1011100": [{"item": 2000000, "price": 50, "stock": 100000},
                            {"item": 2070001, "stock": 100000, "unitPrice": 0.4}],
                "1091000": [{"item": 2000000, "price": 50}]}"#,
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
        // The pack's rows win over an addition for the same shopkeeper.
        assert_eq!(shops.shop(1091000).unwrap().len(), 1);
        assert_eq!(shops.shop(9270021), additions.shop(9270021));
        assert!(shops.shop(11000).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn gachapon_prizes_fall_back_to_the_stand_ins() {
        let stand_ins = GachaponTable::load(None).unwrap();
        assert!(!stand_ins.from_pack());
        for tier in [
            GachaponTier::Common,
            GachaponTier::Uncommon,
            GachaponTier::Rare,
        ] {
            assert!(!stand_ins.prizes(9100100, tier).is_empty(), "{tier:?}");
        }
        assert!(stand_ins
            .prizes(9100100, GachaponTier::Common)
            .contains(&GachaponPrize {
                item: 2000000,
                count: 30
            }));

        // The pack replaces them all; a machine's own come first.
        let dir = std::env::temp_dir().join(format!("rustms-gachapon-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("gachapon.json"),
            r#"{"global": {"common": [{"item": 2000001, "count": 10}]},
                "machines": {"9100101": {"common": [{"item": 1302000}], "rare": [{"item": 2049100}]}}}"#,
        )
        .unwrap();
        let pack = GachaponTable::load(Some(&dir)).unwrap();
        assert!(pack.from_pack());
        let potion = GachaponPrize {
            item: 2000001,
            count: 10,
        };
        let sword = GachaponPrize {
            item: 1302000,
            count: 1,
        };
        assert_eq!(pack.prizes(9100101, GachaponTier::Common), [sword, potion]);
        assert_eq!(pack.prizes(9100100, GachaponTier::Common), [potion]);
        assert!(pack.prizes(9100100, GachaponTier::Rare).is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
