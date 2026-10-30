use crate::actor::field_coconut::{self, CoconutGame, Shot, Turn};
use crate::actor::field_drops::{Drop, DropSet, Ownership};
use crate::actor::field_games::{GameRoomRequest, Games};
use crate::actor::field_kites::KiteSet;
use crate::actor::field_merchants::{MerchantRequest, Merchants};
use crate::actor::field_mists::MistSet;
use crate::actor::field_mobs::{split_exp, BossBar, Cast, CastArea, Catch, Kill, MobSet, Sharer};
use crate::actor::field_ox::{self, OxGame};
use crate::actor::field_race::{self, Race};
use crate::actor::field_reactors::{Hit, ReactorSet, Touch};
use crate::actor::field_shops::{ShopRoomRequest, Shops};
use crate::actor::field_snowball::{self, SnowballGame};
use crate::actor::field_summons::SummonSet;
use crate::actor::field_trades::{Outbox, RoomRequest, Trades};
use crate::actor::registry::MAP_NPC_OBJECT_ID_BASE;
use crate::db::spawn_db;
use crate::message::{ClientHandle, FieldKey, FieldMessage, ServerMessage};
use game::hired_merchant::{HiredMerchant, Stand};
use game::mob_skill::{self, MobSkillEffect};
use game::{wedding, BuffStat, FieldAvatar, MobStat};
use game_data::items::CatchInfo;
use game_data::server_content::keys;
use game_data::skills::CarnivalSkill;
use net::carnival::SpendRefusal;
use net::conti::{self, Happening, State, Target};
use net::loot::Loot;
use net::meso_explosion::{self, Blast};
use net::packet::build::world::buff::{build_own_skill_effect, build_skill_effect};
use net::packet::build::world::carnival::{build_carnival_died, build_carnival_summon};
use net::packet::build::world::coconut::{
    build_coconut_hit, build_coconut_score, build_field_team,
};
use net::packet::build::world::conti::{build_conti_move, build_countdown, build_event_clock};
use net::packet::build::world::door::{build_remove_door, build_spawn_door};
use net::packet::build::world::drop::{
    build_drop, build_explode_drop, build_pet_loot_drop, build_remove_drop, DropAppearance,
};
use net::packet::build::world::effect::{
    build_blow_weather, build_boss_hp_bar, build_field_effect, build_field_music,
    build_field_sound, build_item_emotion, build_mist_item_effect, build_obstacle_state,
    build_obstacle_states,
};
use net::packet::build::world::field::{
    build_player_enter_field, build_player_leave_field, build_show_chair,
    build_show_consume_effect, parse_movement, parse_movement_state,
};
use net::packet::build::world::kite::{build_remove_kite, build_spawn_kite, KiteView};
use net::packet::build::world::messaging::{build_broadcast, build_item_line, Broadcast};
use net::packet::build::world::mini_room::build_char_box;
use net::packet::build::world::mist::{build_remove_mist, build_spawn_mist, MistKind, MistView};
use net::packet::build::world::mob::{
    build_catch_fail, build_catch_with_item, build_control_mob, build_damage_mob, build_heal_mob,
    build_kill_mob, build_mob_hp, build_mob_stat_reset, build_mob_stat_set, build_move_mob,
    build_move_mob_response, build_show_magnet, build_spawn_mob, build_stop_controlling_mob,
    CatchFail, MobMoveView, MobView,
};
use net::packet::build::world::npc::{
    build_control_npc, build_npc_action, build_npc_special_action, build_remove_npc,
    build_spawn_npc, build_uncontrol_npc, ForeignNpc,
};
use net::packet::build::world::ox_quiz::{build_gm_event_instructions, build_ox_quiz};
use net::packet::build::world::pet::build_move_pet;
use net::packet::build::world::reactor::{
    build_hit_reactor, build_remove_reactor, build_spawn_reactor,
};
use net::packet::build::world::snowball::{
    build_hit_snowball, build_left_knock_back, build_snowball_message, build_snowball_state,
};
use net::packet::build::world::summon::{
    build_damage_summon, build_move_summon, build_remove_summon, build_spawn_summon,
    build_summon_attack, build_summon_skill, Gone, SummonView,
};
use net::reactors;
use net::skills::{self, Inflict, Takes};
use net::summons::{self, Limit, SummonAttack, SummonDamage, Support};
use net::{ClientId, FieldRequest, GmEvent};
use packet::Packet;
use rand::Rng;
use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use tokio::time::{interval, Instant, MissedTickBehavior};
use tracing::{debug, info, warn};

/// Hired merchants read from the database for the field.
enum Loaded {
    /// The merchants open here, as the field starts, with their
    /// blacklists.
    All(Vec<(HiredMerchant, Vec<String>)>),
    /// One merchant read again.
    One(i32, Option<HiredMerchant>),
}

struct Occupant {
    handle: ClientHandle,
    avatar: FieldAvatar,
    party: Option<i32>,
    /// The foothold they last moved onto, below 0 for a ladder or rope's.
    foothold: i16,
}

#[derive(Clone, Debug)]
pub struct FieldMapEntityNpc {
    pub object_id: i32,
    pub npc_id: i32,
    pub x: i16,
    pub y: i16,
    pub flip: bool,
    pub foothold: i16,
    pub rx0: i16,
    pub rx1: i16,
}

/// How far a player may be from an NPC and still start or complete its quest:
/// roughly the screen around the player, so the NPC must be on screen.
const QUEST_NPC_REACH_X: i32 = 1200;
const QUEST_NPC_REACH_Y: i32 = 800;

/// How long after a cast its mist starts, in 100 ms (OdinMS-family
/// servers; v83's client starts the mist's time then).
const MIST_DELAY: i16 = 8;

/// How far past a catch item's area a mob may stand and still be caught:
/// the client finds it by its body, and the server knows only its feet.
const CATCH_SLACK: i32 = 100;

/// How far from its owner a client may put a summon: a Puppet goes 200 px
/// ahead, onto a foothold within 80 px up or down (FUN_0096b140).
const SUMMON_REACH_X: i32 = 250;
const SUMMON_REACH_Y: i32 = 150;

/// How soon a trap may hurt a mob again: a second on the client, less some
/// lag between the clients that tell of it.
const TRAPPED_AGAIN: Duration = Duration::from_millis(900);

/// How long a weather item's weather blows (Cosmic's 30 s; the v83 client
/// waits for the server to stop it).
const WEATHER_TIME: Duration = Duration::from_secs(30);

/// The quest's own weather item ("rice cake quest") that tells Primrose
/// Hill to guard the Moon Bunny, and how long it blows (Cosmic's 7 s).
const MOON_WEATHER: i32 = 5120016;
const MOON_WEATHER_TIME: Duration = Duration::from_secs(7);

/// How long after a friendly mob in an event's copy goes the party is
/// sent off (Cosmic's `bunnyDefeated`, 5 s), shortened like events' time.
const FRIENDLY_LOST_AFTER: Duration = Duration::from_secs(5);

/// How long a Monster Carnival guardian's statuses last: for as long as it
/// stands, which outlasts any battle.
const GUARDED_FOR: Duration = Duration::from_secs(60 * 60);

/// A Monster Carnival guardian standing on its battle field: its reactor,
/// the side it guards, the `guardianGenPos` spot it took, and the statuses
/// its mob skill gives the side's mobs while it stands (Cosmic
/// `MapleMap.spawnGuardian`).
struct Guardian {
    object_id: i32,
    side: u8,
    spot: usize,
    mob_skill: i32,
    skill_id: i32,
    statuses: Vec<(MobStat, i16)>,
}

/// The immunity a mob can't have with `stat`: physical and magic immunity
/// don't go together.
fn other_immunity(stat: MobStat) -> Option<MobStat> {
    match stat {
        MobStat::PImmune => Some(MobStat::MImmune),
        MobStat::MImmune => Some(MobStat::PImmune),
        _ => None,
    }
}

/// The immunity that turns an attack whose mobs counter with `counter`
/// (PCounter, or MCounter for a spell) aside.
fn immunity_against(counter: MobStat) -> MobStat {
    if counter == MobStat::MCounter {
        MobStat::MImmune
    } else {
        MobStat::PImmune
    }
}

/// How many of those in its reach a mob's disease takes: Seduce up to its
/// `count`, the others all of them (Cosmic's `applyDisease`).
fn diseased_most(stat: BuffStat, count: i32) -> usize {
    usize::try_from(count)
        .ok()
        .filter(|&most| most > 0 && stat == BuffStat::Seduce)
        .unwrap_or(usize::MAX)
}

/// How long to wait before reading or writing hired merchants again after
/// the database failed.
const MERCHANT_RETRY: Duration = Duration::from_secs(5);
/// How often an expired merchant's closing is tried before giving up; the
/// next start of the field closes it then. A blacklist's save is tried as
/// often.
const EXPIRE_TRIES: u32 = 12;

/// The Moon Bunny's guards are told it is being hurt on every sixth hit
/// (Cosmic's script means to, but a bug there keeps the line from showing;
/// a recommended default in `v83ref/lanes/QUESTIONS.md`, row 180).
const BUNNY_SICK_HITS: u32 = 6;

/// The Watchhog's guard is told to protect it when it lost more than
/// `HOG_HURT` HP in `HOG_WATCH` (Cosmic `3rdJob_mount`'s `checkHogHealth`,
/// every ten seconds; here at the first hit after each ten seconds).
const HOG_WATCH: Duration = Duration::from_secs(10);
const HOG_HURT: i32 = 1000;

/// How long an item lies on a reactor before the reactor takes it
/// (Cosmic's `ActivateItemReactor`).
const ITEM_TAKEN_AFTER: Duration = Duration::from_secs(5);

/// The item a drop is, if a reactor may take it: one a player dropped, or
/// one a mob or reactor dropped for anyone, as Cosmic's `spawnDrop` also
/// sets off the reactors it lands on; never a quest drop.
/// How far from a player's feet the `n`th item (from 1) knocked out of
/// their bag lands: 0, then 37, -25, 62, -50, ... as Cosmic works it out.
fn shed_offset(n: i16) -> i16 {
    if n % 2 == 0 {
        25 * (n + 1) / 2
    } else {
        -(25 * (n / 2))
    }
}

fn reactor_item(loot: &Loot) -> Option<(i32, i16)> {
    match loot {
        Loot::Dropped(item) => Some((item.item_id, item.quantity)),
        &Loot::Item {
            item_id,
            quantity,
            quest: None,
        } => Some((item_id, quantity)),
        _ => None,
    }
}

/// A run's copy of a Holiday PQ arena (Cosmic `HolidayPQ_*`'s `level`,
/// `snowmanLevel`, `snowmanStep` and `missingDrops`).
struct HolidayArena {
    level: u8,
    snowman: net::events::Snowman,
    misses: u8,
    /// Snow Vigor where the snowman eats it, by when: (when, drop).
    eating: Vec<(Instant, i32)>,
    /// Whether Scrooge has come out.
    scrooge: bool,
    /// When the whole snowman next heals, while Scrooge is out and alive.
    heals_at: Option<Instant>,
}

/// An item a player put on a reactor that takes it, once it has lain there
/// long enough.
struct ItemOnReactor {
    at: Instant,
    drop: i32,
    reactor: i32,
}

/// One live copy of a map. It owns everything local to that map: who is on
/// it and where, its NPCs, mobs, drops, and reactors, which advance on
/// `tick`, and its players' mists and summons.
pub struct FieldActor {
    key: FieldKey,
    rx: mpsc::Receiver<FieldMessage>,
    occupants: HashMap<ClientId, Occupant>,
    map_npcs: Vec<FieldMapEntityNpc>,
    /// Who picks the NPCs' idle actions; without one they only stand.
    npc_controller: Option<ClientId>,
    mobs: MobSet,
    /// Mob skills cast and waiting for their animations to end.
    casts: Vec<Cast>,
    drops: DropSet,
    reactors: ReactorSet,
    items_on_reactors: Vec<ItemOnReactor>,
    mists: MistSet,
    summons: SummonSet,
    /// Players' Mystic Doors here, by owner.
    doors: HashMap<ClientId, (i16, i16)>,
    trades: Trades,
    shops: Shops,
    games: Games,
    merchants: Merchants,
    /// Owners in their merchant's room here from elsewhere on the channel,
    /// with a Store Remote Controller, and how to reach them.
    remote_owners: HashMap<ClientId, ClientHandle>,
    /// Remote visits that came before the merchants were read, if they are
    /// being read.
    remote_waiting: Option<Vec<(ClientId, MerchantRequest)>>,
    /// Merchants read from the database come back here; the sender is the
    /// field's own, so that a copy of a map still stops when its last
    /// outside sender goes.
    loaded_tx: mpsc::UnboundedSender<Loaded>,
    loaded_rx: mpsc::UnboundedReceiver<Loaded>,
    /// On a ship route's maps, the route time trips were last checked at.
    trips: Option<i64>,
    /// The weather item blowing here, its message, and when it stops.
    weather: Option<(i32, String, Instant)>,
    /// The named map objects moved to other states (FIELD_OBSTACLE_ONOFF),
    /// such as an Orbis jail's dropped spears, in the order they moved.
    obstacles: Vec<(String, i32)>,
    /// When a trap last hurt each mob, as a client told: every client in the
    /// map tells of the same hit.
    trapped: HashMap<i32, Instant>,
    /// Hits on the Moon Bunny since its guards last heard of them, and the
    /// rice cakes it made.
    bunny_hits: u32,
    bunny_cakes: u32,
    /// The Watchhog's HP when its guard last heard how it fares, and when.
    hog_watch: Option<(Instant, i32)>,
    /// When, after a friendly mob in an event's copy went, its party is
    /// sent off.
    lost_at: Option<Instant>,
    /// On the Tower of Goddess's Salion stage, how many Cellions go before
    /// a spot drops its loot, and how many have gone, once the first has.
    salion: Option<(u32, u32)>,
    /// In the Tower of Goddess's garden, how many pots have broken, and
    /// whether Papa Pixie came out, after which they grow nothing.
    pots: u32,
    papa_pixie: bool,
    /// In the Tower of Goddess's sealed room, how many of the team Eak
    /// wants on each platform, once he has been asked.
    split: Option<[u8; 3]>,
    /// On the Tower of Goddess's lounge and tall room, the right pad of
    /// each row of the climb, once anyone has stepped on one.
    climb: Option<Vec<u8>>,
    /// In the tall room, the two levers Eak wants up, once he has been
    /// asked.
    levers: Option<[bool; 5]>,
    /// In a Magatia library, what each of its books still holds
    /// (`net::events::draw_books`), once anyone has opened one.
    books: Option<[Option<net::events::Find>; net::events::MAGATIA_BOOKS]>,
    /// In a Magatia beaker room, the beakers filled so far.
    beakers: BTreeSet<i32>,
    /// In a run's copy of Yulete's office, once anyone has come in, what
    /// he makes of the party and when he leaves; no time once he has.
    yulete: Option<(net::events::Yulete, Option<Instant>)>,
    /// In a run's copy of the Magatia boss's room, whether Yulete has
    /// brought out his Frankenroid.
    frankenroid: bool,
    /// In a run's copy of Magatia's boss room, whether the escort fainted.
    escort_fainted: bool,
    /// In a jail under the sealed room, the levers that open its way on
    /// (`net::events::draw_jail`), once anyone has tried it; none once
    /// they have.
    jail: Option<u8>,
    /// On a copy of a Mu Lung Dojo floor, its boss until its challenger
    /// first arrives and it comes out (Cosmic `dojang_1st`).
    dojo_boss: Option<i32>,
    /// A copy of a Massacre stage, Nett's Pyramid's or the Dusty
    /// Platform's, whose kills, cool kills and misses its run counts.
    massacre: bool,
    /// A copy of a Monster Carnival battle field, whose kills earn CP.
    carnival: bool,
    /// Its guardians standing.
    guardians: Vec<Guardian>,
    kites: KiteSet,
    /// On a Coconut Season field (field type 4), its game.
    coconut: Option<CoconutGame>,
    /// On a Snowball field (field type 1), its game.
    snowball: Option<SnowballGame>,
    /// On the OX Quiz's field (field type 5), its game.
    ox: Option<OxGame>,
    /// On one of a timed GM event's maps, the event while it is on.
    race: Option<Race>,
    /// Crimsonwood Keep's fourth stage's stirges are gone, for good.
    stirges_gone: bool,
    /// In the Twilight of Gods' copy, once the first has come in, the wave
    /// of the goddess's guardians due or out, from the first; one past the
    /// last once that is down.
    guardians_wave: Option<usize>,
    /// In a run's copy of a Holiday PQ arena, its difficulty, its snowman,
    /// and its kills that left no Snow Vigor.
    holiday: Option<HolidayArena>,
    /// What this field does to itself later, once due: the Balrog's tomb
    /// lets its claw go a minute in, and brings out the sealed Balrog after
    /// the win.
    later: Vec<(Instant, FieldMessage)>,
    tick: Duration,
}

impl FieldActor {
    pub fn new(
        key: FieldKey,
        rx: mpsc::Receiver<FieldMessage>,
        map_npcs: Vec<FieldMapEntityNpc>,
        mut mobs: MobSet,
        drops: DropSet,
        mut reactors: ReactorSet,
        tick: Duration,
    ) -> Self {
        let holiday = net::events::holiday_arena(key.map_id)
            .filter(|_| key.instance_id != 0)
            .map(|level| HolidayArena {
                level,
                snowman: net::events::Snowman::default(),
                misses: 0,
                eating: Vec::new(),
                scrooge: false,
                heals_at: None,
            });
        // A Holiday PQ arena's mobs wait for its snowman (Cosmic
        // `allowSummonState(false)`).
        if holiday.is_some() {
            mobs.shut();
        }
        mobs.start(Instant::now());
        reactors.respawn(Instant::now());
        if reactors::shuffled(key.map_id) {
            reactors.shuffle(&mut rand::thread_rng());
        }
        // Crimsonwood Keep's fourth stage wakes a sigil of each job and
        // fakes, which then change places (Cosmic CWKPQ's
        // `generateMapReactors`).
        if key.map_id == net::events::CWK_ENTRY + 300 {
            let woken = net::events::cwk_wake(&mut rand::thread_rng());
            for name in woken {
                reactors.set_state(name, 1);
            }
            reactors.shuffle_named(&woken, &mut rand::thread_rng());
        }
        // The trips are read from the same moment the elevator's car is
        // placed, so that no change is missed or shown twice.
        let route_time = conti::now();
        if let Some((name, state)) = conti::car_state(key.map_id, route_time) {
            reactors.set_state(name, state);
        }
        let (loaded_tx, loaded_rx) = mpsc::unbounded_channel();
        let mut field = Self {
            key,
            rx,
            occupants: HashMap::new(),
            map_npcs,
            npc_controller: None,
            mobs,
            casts: Vec::new(),
            drops,
            reactors,
            items_on_reactors: Vec::new(),
            mists: MistSet::default(),
            summons: SummonSet::default(),
            doors: HashMap::new(),
            trades: Trades::default(),
            shops: Shops::default(),
            games: Games::default(),
            merchants: Merchants::default(),
            remote_owners: HashMap::new(),
            remote_waiting: None,
            loaded_tx,
            loaded_rx,
            trips: conti::has_trips(key.map_id).then_some(route_time),
            weather: None,
            obstacles: Vec::new(),
            trapped: HashMap::new(),
            bunny_hits: 0,
            bunny_cakes: 0,
            hog_watch: None,
            lost_at: None,
            salion: None,
            pots: 0,
            papa_pixie: false,
            split: None,
            climb: None,
            levers: None,
            books: None,
            beakers: BTreeSet::new(),
            yulete: None,
            frankenroid: false,
            escort_fainted: false,
            jail: None,
            dojo_boss: net::events::dojo_boss(key.map_id).filter(|_| key.instance_id != 0),
            massacre: net::events::massacre_stage(key.map_id) && key.instance_id != 0,
            carnival: net::carnival::battle_room(key.map_id).is_some() && key.instance_id != 0,
            guardians: Vec::new(),
            kites: KiteSet::default(),
            coconut: net::get_game_data()
                .ok()
                .and_then(|data| data.field(key.map_id))
                .and_then(|field| field.coconut.clone())
                .map(CoconutGame::new),
            snowball: net::get_game_data()
                .ok()
                .and_then(|data| data.field(key.map_id))
                .and_then(|field| field.snowball.clone())
                .map(|rules| {
                    let wait = Duration::from_millis(rules.snowman_wait.into());
                    SnowballGame::new(rules, net::events::scaled(wait))
                }),
            ox: net::get_game_data()
                .ok()
                .and_then(|data| data.field(key.map_id))
                .filter(|field| field.field_type == 5 && field.areas.len() >= 2)
                .map(|_| {
                    let times = [
                        field_ox::QUESTION_TIME,
                        field_ox::ANSWER_TIME,
                        field_ox::PAUSE_TIME,
                    ];
                    OxGame::new(times.map(net::events::scaled))
                }),
            race: None,
            stirges_gone: false,
            guardians_wave: None,
            holiday,
            later: Vec::new(),
            tick,
        };
        // A wedding's copy of the hunting ground opens with its one Giant
        // Cake.
        if key.map_id == wedding::HUNTING_GROUND && key.instance_id != 0 {
            field.board(wedding::GIANT_CAKE, 1, wedding::GIANT_CAKE_AT);
        }
        // A copy of one of Horntail's trials opens with its head.
        if let Some((_, head)) = net::events::trial_head(key.map_id) {
            if key.instance_id != 0 {
                field.bring_out_head(head);
            }
        }
        // A copy of King Pepe's room with one of the Yeti brothers.
        if key.map_id == net::events::KING_PEPE_ROOM && key.instance_id != 0 {
            let brothers = net::events::YETI_BROTHERS;
            let brother = brothers[rand::thread_rng().gen_range(0, brothers.len())];
            field.board(brother, 1, net::events::YETI_BROTHER_AT);
        }
        // And a copy of one of the Pirate Ship's decks with its crew.
        if key.instance_id != 0 {
            for &(mob_id, at, count) in net::events::opening_mobs(key.map_id) {
                field.board(mob_id, count, at);
            }
        }
        // The Balrog's claw gives way to its left arm a minute in (Cosmic
        // `BalrogBattle.releaseLeftClaw`).
        if key.map_id == net::events::BALROG_TOMB && key.instance_id != 0 {
            let claw = FieldMessage::Slay {
                mob_id: net::mobs::BALROG_CLAW,
            };
            field.after(net::events::BALROG_CLAW_AFTER, claw);
        }
        // The Twilight of Gods' statue takes no harm while the goddess's
        // guardians stand.
        if key.map_id == net::events::PINK_BEAN_ARENA && key.instance_id != 0 {
            field.mobs.ward(net::mobs::PINK_BEAN_STATUE, true);
        }
        // Crimsonwood Keep's fourth stage opens with its fires lit.
        if key.map_id == net::events::CWK_ENTRY + 300 {
            field.obstacles = net::events::cwk_fires();
        }
        field
    }

    pub async fn run(mut self) {
        info!(field = ?self.key, "FieldActor started");
        let mut ticker = interval(self.tick);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        self.load_merchants();

        loop {
            tokio::select! {
                message = self.rx.recv() => match message {
                    Some(FieldMessage::Close) | None => break,
                    Some(message) => self.handle_message(message),
                },
                Some(loaded) = self.loaded_rx.recv() => self.merchants_loaded(loaded),
                now = ticker.tick() => self.on_tick(now),
            }
        }
    }

    /// Advance time-driven field state: drop expiry, mobs leaving or coming
    /// out of dead ones, and mob and reactor respawns.
    fn on_tick(&mut self, now: Instant) {
        let (due, later) = std::mem::take(&mut self.later)
            .into_iter()
            .partition(|(at, _)| *at <= now);
        self.later = later;
        for (_, message) in due {
            self.handle_message(message);
        }
        self.run_trips();
        self.tick_merchants(now);
        self.tick_yulete(now);
        self.friendly_drops(now);
        if self.lost_at.is_some_and(|at| at <= now) {
            self.lost_at = None;
            self.lose_event();
        }
        if self
            .weather
            .as_ref()
            .is_some_and(|(_, _, until)| *until <= now)
        {
            self.weather = None;
            if let Ok(packet) = build_blow_weather(None) {
                self.broadcast(None, packet);
            }
        }
        for object_id in self.drops.expire(now) {
            if let Ok(packet) = build_remove_drop(object_id, None) {
                self.broadcast(None, packet);
            }
        }
        let (due, waiting) = std::mem::take(&mut self.items_on_reactors)
            .into_iter()
            .partition(|item: &ItemOnReactor| item.at <= now);
        self.items_on_reactors = waiting;
        // One item per reactor a tick: what else lay due on it is looked
        // at again after the take.
        let mut took = Vec::new();
        for item in due {
            let reactor = item.reactor;
            if !took.contains(&reactor) && self.take_item_on_reactor(item, now) {
                took.push(reactor);
            }
        }
        self.eat_snow_vigor(now);
        self.heal_whole_snowman(now);
        let spawned = self.reactors.respawn(now);
        self.show_reactors(&spawned);
        // Those whose time in their state is up go back (the forest's pond
        // filling again), and look at what lies on them.
        let reverted = self.reactors.revert(now);
        for view in &reverted {
            if let Ok(packet) = build_hit_reactor(view, 0, 0) {
                self.broadcast(None, packet);
            }
        }
        if !reverted.is_empty() {
            let lying: Vec<Drop> = self.drops.all().cloned().collect();
            for drop in lying {
                self.put_on_reactor(&drop, now);
            }
        }
        for object_id in self.mists.expire(now) {
            if let Ok(packet) = build_remove_mist(object_id) {
                self.broadcast(None, packet);
            }
        }
        for object_id in self.kites.expire(now) {
            if let Ok(packet) = build_remove_kite(object_id, true) {
                self.broadcast(None, packet);
            }
        }
        self.tick_coconut(now);
        self.tick_snowball(now);
        self.tick_ox(now);
        self.tick_race(now);
        self.reach_from_mists(now);
        for summon in self.summons.expire(now) {
            let view = summon.view;
            if let Ok(packet) = build_remove_summon(view.owner_id, view.object_id, Gone::Expired) {
                self.broadcast_about(view.owner_id, None, packet);
            }
        }
        let (due, waiting) = std::mem::take(&mut self.casts)
            .into_iter()
            .partition(|cast| cast.at <= now);
        self.casts = waiting;
        for cast in due {
            self.apply_cast(&cast, now);
        }
        for hurt in self.mobs.hurt_by_statuses(now) {
            self.show_mob_hp(hurt.from, hurt.object_id, hurt.percent);
            self.show_boss_bar(self.mobs.bar_for(hurt.object_id));
        }
        self.mobs.recover(now);
        for (object_id, lost) in self.mobs.expire_statuses(now) {
            if let Ok(packet) = build_mob_stat_reset(object_id, &lost) {
                self.broadcast(None, packet);
            }
        }
        let left = self.mobs.leave(now);
        self.show_left(left);
        let mut dead_part = false;
        for object_id in self.mobs.revive(now, net::mobs::info) {
            let Some(mob) = self.mobs.get(object_id) else {
                continue;
            };
            dead_part |= net::mobs::DEAD_HORNTAIL_PARTS.contains(&mob.info.mob_id);
            if let Ok(packet) = build_spawn_mob(&mob.view(true)) {
                self.broadcast(None, packet);
            }
            if let Some(controller) = mob.controller {
                self.send_control(controller, &[object_id]);
            }
        }
        let parts = net::mobs::DEAD_HORNTAIL_PARTS;
        if dead_part && parts.clone().all(|mob_id| !self.out(mob_id).is_empty()) {
            self.horntail_defeated(now);
        }
        let spawned = self.mobs.respawn(now, self.occupants.len());
        if spawned.is_empty() {
            return;
        }
        let controller = self.mob_candidate();
        for object_id in spawned {
            self.guard_new(object_id, now);
            let Some(mob) = self.mobs.get(object_id) else {
                continue;
            };
            if let Ok(packet) = build_spawn_mob(&mob.view(true)) {
                self.broadcast(None, packet);
            }
            if let Some(boss) = net::mobs::area_boss_on(self.key.map_id, mob.info.mob_id) {
                let notice =
                    net::content::text().get(keys::AREA_BOSS_APPEARED, &[("mob", boss.name)]);
                if let Ok(packet) = build_item_line(&notice, 0) {
                    self.broadcast(None, packet);
                }
            }
        }
        if let Some(controller) = controller {
            let controlled = self.mobs.assign_uncontrolled(controller);
            self.send_control(controller, &controlled);
        }
    }

    /// Show everyone the mobs that left, as `MobSet::leave` gives them.
    fn show_left(&self, left: Vec<(i32, u8, Option<BossBar>)>) {
        for (object_id, leave, bar) in left {
            if let Ok(packet) = build_kill_mob(object_id, leave) {
                self.broadcast(None, packet);
            }
            self.show_boss_bar(bar);
        }
    }

    /// Tell `client` it now moves these mobs.
    fn send_control(&self, client: ClientId, object_ids: &[i32]) {
        let Some(occupant) = self.occupants.get(&client) else {
            return;
        };
        for &object_id in object_ids {
            if let Some(mob) = self.mobs.get(object_id) {
                if let Ok(packet) = build_control_mob(&mob.view(false), false) {
                    occupant.handle.send_packet(packet);
                }
            }
        }
    }

    /// Who should move the map's mobs: anyone here but a hidden GM, whom
    /// they don't see (Cosmic's controller search skips one).
    fn mob_candidate(&self) -> Option<ClientId> {
        self.occupants
            .iter()
            .find(|(_, occupant)| !occupant.avatar.hidden)
            .map(|(&client_id, _)| client_id)
    }

    /// Who moves the mobs `from` brings out: `from`, unless hidden.
    fn mover_for(&self, from: ClientId) -> Option<ClientId> {
        match self.occupants.get(&from) {
            Some(occupant) if !occupant.avatar.hidden => Some(from),
            _ => self.mob_candidate(),
        }
    }

    /// Tell `client` it now picks every NPC's idle actions.
    fn send_npc_control(&mut self, client: ClientId) {
        let Some(occupant) = self.occupants.get(&client) else {
            return;
        };
        self.npc_controller = Some(client);
        for map_npc in &self.map_npcs {
            if let Ok(packet) = build_control_npc(&to_foreign_npc(map_npc)) {
                occupant.handle.send_packet(packet);
            }
        }
    }

    /// Ask the NPCs' controller to have NPC `npc_id` play its special
    /// action `action` (0x106). Its client answers with an NPC_ACTION, which
    /// `handle_npc_action` shows everyone.
    fn npc_special_action(&self, npc_id: i32, action: &str) {
        let Some(map_npc) = self.map_npcs.iter().find(|npc| npc.npc_id == npc_id) else {
            return;
        };
        let Some(controller) = self.npc_controller.and_then(|id| self.occupants.get(&id)) else {
            return;
        };
        if let Ok(packet) = build_npc_special_action(map_npc.object_id, action) {
            controller.handle.send_packet(packet);
        }
    }

    /// Show everyone the action the NPCs' controller chose.
    fn handle_npc_action(&self, from: ClientId, object_id: i32, action: &[u8]) {
        if self.npc_controller != Some(from)
            || !self.map_npcs.iter().any(|npc| npc.object_id == object_id)
        {
            return;
        }
        if let Ok(packet) = build_npc_action(object_id, action) {
            self.broadcast(None, packet);
        }
    }

    /// Seat `from` on a portable chair (0 to stand) and show the others.
    fn handle_chair(&mut self, from: ClientId, item_id: i32) {
        let Some(occupant) = self.occupants.get_mut(&from) else {
            return;
        };
        if occupant.avatar.chair == item_id {
            return;
        }
        occupant.avatar.chair = item_id;
        if let Ok(packet) = build_show_chair(occupant.avatar.id, item_id) {
            self.broadcast_by(from, packet);
        }
    }

    fn handle_message(&mut self, message: FieldMessage) {
        match message {
            FieldMessage::Join {
                handle,
                avatar,
                party,
            } => self.handle_join(handle, avatar, party),
            FieldMessage::Leave { client_id, staying } => self.handle_leave(client_id, staying),
            FieldMessage::Close => {}
            FieldMessage::Show(packet) => self.broadcast(None, packet),
            FieldMessage::ClearMobs => self.clear_mobs(),
            FieldMessage::SwapMobs { only } => self.swap_mobs(only),
            FieldMessage::BringOut { mob_id, at } => {
                self.board(mob_id, 1, at);
            }
            FieldMessage::Slay { mob_id } => self.slay(mob_id),
            FieldMessage::GuardiansWake => self.guardians_wake(),
            FieldMessage::CarnivalSummon {
                summoner,
                index,
                mob_id,
                side,
                cap,
                reply,
            } => {
                let _ = reply.send(self.carnival_summon(summoner, index, mob_id, side, cap));
            }
            FieldMessage::CarnivalGuard {
                summoner,
                index,
                guardian,
                side,
                cap,
                reply,
            } => {
                let _ = reply.send(self.carnival_guard(summoner, index, guardian, side, cap));
            }
            FieldMessage::CarnivalAnnounce {
                summoner,
                tab,
                index,
            } => {
                let name = self.occupants.get(&summoner).map(|o| o.avatar.name.clone());
                if let Some(Ok(packet)) = name.map(|name| build_carnival_summon(tab, index, &name))
                {
                    self.broadcast(None, packet);
                }
            }
            FieldMessage::CarnivalDied { died, side, lost } => {
                let name = self.occupants.get(&died).map(|o| o.avatar.name.clone());
                if let Some(Ok(packet)) = name.map(|name| build_carnival_died(side, &name, lost)) {
                    self.broadcast(None, packet);
                }
            }
            FieldMessage::SetParty { client_id, party } => {
                if let Some(occupant) = self.occupants.get_mut(&client_id) {
                    occupant.party = party;
                }
            }
            FieldMessage::UpdateAvatar {
                client_id,
                avatar,
                announce,
            } => {
                for packet in announce {
                    self.broadcast_by(client_id, packet);
                }
                if let Some(occupant) = self.occupants.get_mut(&client_id) {
                    // Keep the live position, chair and balloon, and where
                    // the pets still out are (new ones start at the
                    // owner); take everything else.
                    let live = &mut occupant.avatar;
                    let mut avatar = avatar;
                    for pet in &mut avatar.pets {
                        match live.pets.iter().find(|old| old.serial == pet.serial) {
                            Some(old) => *pet = old.clone(),
                            None => (pet.x, pet.y) = (live.x, live.y),
                        }
                    }
                    let hid = avatar.hidden != live.hidden;
                    occupant.avatar = FieldAvatar {
                        x: live.x,
                        y: live.y,
                        stance: live.stance,
                        chair: live.chair,
                        mini_room: live.mini_room.take(),
                        team: live.team,
                        ..avatar
                    };
                    if hid {
                        self.show_hidden(client_id);
                        self.hide_from_mobs(client_id);
                    }
                }
            }
            // Chat is off on the OX Quiz's field while a game is on, but
            // for GMs.
            FieldMessage::Chat { from, .. }
                if self.ox.as_ref().is_some_and(OxGame::on)
                    && self.occupants.get(&from).is_some_and(|o| !o.avatar.gm) => {}
            FieldMessage::Chat { from, packet } => self.broadcast_seen_by(from, None, packet),
            FieldMessage::Positions { reply } => {
                let positions = self
                    .occupants
                    .values()
                    .map(|occupant| (occupant.avatar.x, occupant.avatar.y))
                    .collect();
                let _ = reply.send(positions);
            }
            FieldMessage::ShopSearch { item_id, reply } => {
                let (map_id, channel_id) = (self.key.map_id, self.key.channel_id);
                let mut hits = self.shops.search(item_id, map_id, channel_id);
                hits.extend(self.merchants.search(item_id, map_id, channel_id));
                let _ = reply.send(hits);
            }
            FieldMessage::ShopGo { room, name, reply } if self.merchants.contains(room) => {
                let _ = reply.send(self.merchants.go_result(room, &name));
            }
            FieldMessage::ShopGo { room, name, reply } => {
                let _ = reply.send(self.shops.go_result(room, &name));
            }
            FieldMessage::ReserveTeam { client_id, reply } => {
                let now = Instant::now();
                let team = match (&mut self.coconut, &mut self.snowball) {
                    (Some(game), _) => Some(game.reserve(client_id, now)),
                    (_, Some(game)) => Some(game.reserve(client_id, now)),
                    _ => None,
                };
                let _ = reply.send(team);
            }
            FieldMessage::RunTimedEvent { started } => self.run_timed_event(started),
            FieldMessage::TimedEventOn { reply } => {
                let _ = reply.send(self.race.is_some());
            }
            FieldMessage::SendToGmEvent { from, map_id } => {
                for (&client_id, occupant) in &self.occupants {
                    if client_id != from {
                        occupant.handle.send(ServerMessage::GoToGmEvent {
                            from_map: self.key.map_id,
                            map_id,
                        });
                    }
                }
            }
            FieldMessage::MobCount { reply } => {
                let _ = reply.send(self.mobs.live().count());
            }
            FieldMessage::HolidaySnowman { reply } => {
                let arena = self.holiday.as_ref();
                let _ = reply
                    .send(arena.map_or((0, false), |arena| (arena.snowman.form, arena.scrooge)));
            }
            FieldMessage::DropCount { reply } => {
                let _ = reply.send(self.drops.all().count());
            }
            FieldMessage::FloorItems { reply } => {
                let items = (self.drops.all())
                    .filter_map(|drop| Some((reactor_item(&drop.loot)?.0, self.landing(drop))));
                let _ = reply.send(items.collect());
            }
            FieldMessage::TakeItems { ids } => {
                let taken: Vec<i32> = (self.drops.all())
                    .filter(|drop| {
                        reactor_item(&drop.loot).is_some_and(|(id, _)| ids.contains(&id))
                    })
                    .map(|drop| drop.object_id)
                    .collect();
                for object_id in taken {
                    self.drops.remove(object_id);
                    if let Ok(packet) = build_remove_drop(object_id, None) {
                        self.send_drop_packet(packet);
                    }
                }
            }
            FieldMessage::ReactorStates { names, reply } => {
                let states = names
                    .iter()
                    .map(|name| self.reactors.named(name).map(|r| r.state));
                let _ = reply.send(states.collect());
            }
            FieldMessage::SetReactor { name, state } => {
                for view in self.reactors.set_state(name, state) {
                    if let Ok(packet) = build_hit_reactor(&view, 0, 0) {
                        self.broadcast(None, packet);
                    }
                }
            }
            FieldMessage::ForwardReactor { name, state } => {
                for view in self.reactors.force_forward(name, state) {
                    if let Ok(packet) = build_hit_reactor(&view, 0, 0) {
                        self.broadcast(None, packet);
                    }
                }
            }
            FieldMessage::OpenReactor { name } => self.open_reactor(name),
            FieldMessage::PlayReactor { name } => {
                let views: Vec<_> = (self.reactors.live())
                    .filter(|reactor| self.reactors.name(reactor.object_id) == Some(name))
                    .map(|reactor| reactor.view())
                    .collect();
                for view in views {
                    if let Ok(packet) = build_hit_reactor(&view, 0, 0) {
                        self.broadcast(None, packet);
                    }
                }
            }
            FieldMessage::Split { reply } => {
                let split = self
                    .split
                    .get_or_insert_with(|| net::events::draw_split(&mut rand::thread_rng()));
                let _ = reply.send(*split);
            }
            FieldMessage::Climb { reply } => {
                let map_id = self.key.map_id;
                let climb = self.climb.get_or_insert_with(|| {
                    net::events::draw_climb(map_id, &mut rand::thread_rng())
                });
                let _ = reply.send(climb.clone());
            }
            FieldMessage::Yulete {
                shocked,
                caught,
                won,
                reply,
            } => {
                let said = match self.call_frankenroid(caught, won) {
                    Some(key) => Some(key),
                    None => self.talk_to_yulete(shocked),
                };
                let _ = reply.send(said);
            }
            FieldMessage::Jail { reply } => {
                let _ = reply.send(self.check_jail());
            }
            FieldMessage::Levers { reply } => {
                let levers = self
                    .levers
                    .get_or_insert_with(|| net::events::draw_levers(&mut rand::thread_rng()));
                let _ = reply.send(*levers);
            }
            FieldMessage::Move {
                from,
                packet,
                start,
                movement_bytes,
            } => {
                if let Some(occupant) = self.occupants.get_mut(&from) {
                    let avatar = &mut occupant.avatar;
                    if let Some((x, y, stance, foothold)) =
                        parse_movement(&movement_bytes, (start.0, start.1, avatar.stance))
                    {
                        avatar.x = x;
                        avatar.y = y;
                        avatar.stance = stance;
                        if let Some(foothold) = foothold {
                            occupant.foothold = foothold;
                        }
                    }
                }
                self.broadcast_by(from, packet);
            }
            FieldMessage::NpcInteraction {
                from,
                object_id,
                request_id,
            } => self.handle_npc_interaction(from, object_id, request_id),
            FieldMessage::QuestNpc {
                from,
                npc_id,
                request_id,
            } => self.handle_quest_npc(from, npc_id, request_id),
            FieldMessage::FromClient { from, request } => match request {
                FieldRequest::MobMove {
                    object_id,
                    move_id,
                    want_attack,
                    action,
                    skill_data,
                    start,
                    movement,
                } => self.handle_mob_move(
                    from,
                    object_id,
                    move_id,
                    (want_attack, action, skill_data),
                    start,
                    &movement,
                ),
                FieldRequest::MobDistance {
                    object_id,
                    distance,
                } => self.handle_mob_distance(from, object_id, distance),
                FieldRequest::MobBomb { object_id } => self.handle_mob_bomb(object_id),
                FieldRequest::FieldDamageMob { object_id, damage } => {
                    self.handle_field_damage_mob(from, object_id, damage)
                }
                FieldRequest::MobDamageMobFriendly { attacker, victim } => {
                    self.handle_friendly_hit(attacker, victim)
                }
                FieldRequest::MobDamageMob {
                    attacker,
                    victim,
                    damage,
                } => self.handle_mob_damage_mob(from, attacker, victim, damage),
                FieldRequest::NpcAction { object_id, action } => {
                    self.handle_npc_action(from, object_id, &action)
                }
                FieldRequest::Attack {
                    broadcast,
                    skill,
                    counter,
                    lines,
                    hits,
                    inflicts,
                    takes,
                    blast,
                    pockets,
                } => {
                    let hits = match blast {
                        Some(blast) => self.set_off(from, &blast, hits),
                        None => hits,
                    };
                    self.pick_pockets(from, pockets);
                    self.handle_attack(
                        from,
                        Some(broadcast),
                        Some(skill),
                        Some((counter, lines)),
                        hits,
                        &inflicts,
                        takes,
                    )
                }
                FieldRequest::Reflect {
                    object_id,
                    mob_id,
                    damage,
                } => self.handle_reflect(from, object_id, mob_id, damage),
                FieldRequest::Mist {
                    skill_id,
                    level,
                    left,
                    inflict,
                } => self.handle_mist(from, skill_id, level, left, inflict),
                FieldRequest::ItemMist {
                    item_id,
                    area,
                    lasts,
                } => self.handle_item_mist(from, item_id, area, lasts),
                FieldRequest::Broadcast(packet) => self.broadcast_by(from, packet),
                FieldRequest::BroadcastAll(packet) => self.broadcast(Some(from), packet),
                FieldRequest::Magnet { effect, caught } => self.handle_magnet(from, effect, caught),
                FieldRequest::MobDebuff {
                    skill_id,
                    level,
                    origin,
                    mobs,
                    inflict,
                } => self.handle_mob_debuff(from, skill_id, level, origin, &mobs, &inflict),
                FieldRequest::Chair(item_id) => self.handle_chair(from, item_id),
                FieldRequest::DiseaseOthers(disease) => {
                    let map_id = self.key.map_id;
                    for (&id, occupant) in &self.occupants {
                        if id != from {
                            occupant
                                .handle
                                .send(ServerMessage::Diseased { disease, map_id });
                        }
                    }
                }
                FieldRequest::Weather { item_id, message } => self.blow_weather(item_id, message),
                FieldRequest::Kite { item_id, message } => self.fly_kite(from, item_id, message),
                FieldRequest::CoconutHit { index } => self.hit_coconut(from, index),
                FieldRequest::SnowballHit { target, delay } => {
                    self.hit_snowball(from, target, delay)
                }
                FieldRequest::IntoSnowball => self.knock_back(from),
                FieldRequest::GmEvent(event) => self.run_gm_event(event),
                FieldRequest::Pickup {
                    object_id,
                    x,
                    y,
                    needs,
                } => self.handle_pickup(from, object_id, x, y, &needs),
                FieldRequest::Throw { loot, vanish } => self.handle_throw(from, loot, vanish),
                FieldRequest::Shed { item_id, count } => self.handle_shed(from, item_id, count),
                FieldRequest::HitReactor {
                    object_id,
                    flags,
                    delay,
                    skill,
                } => self.handle_hit_reactor(from, object_id, flags, delay, skill),
                FieldRequest::TouchReactor {
                    object_id,
                    touching,
                } => self.handle_touch_reactor(from, object_id, touching),
                FieldRequest::CharInfo { character_id } => {
                    self.handle_char_info(from, character_id)
                }
                FieldRequest::Fame {
                    target,
                    giver_name,
                    up,
                } => self.handle_fame(from, target, giver_name, up),
                FieldRequest::Summon {
                    skill_id,
                    level,
                    at,
                    left,
                    lasts,
                    appear,
                } => self.handle_summon(from, skill_id, level, at, left, lasts, appear),
                FieldRequest::SummonMobs(mobs) => self.handle_summon_mobs(from, mobs),
                FieldRequest::BringOutMob { mob_id, at } => {
                    self.handle_bring_out_mob(from, mob_id, at)
                }
                FieldRequest::BringOutMobs { mob_id, at } => self.bring_out_mobs(from, mob_id, &at),
                FieldRequest::BringOutNpc { npc_id, at } => {
                    if !self.map_npcs.iter().any(|npc| npc.npc_id == npc_id) {
                        self.appear(npc_id, at);
                    }
                }
                FieldRequest::OpenReactor { name } => self.open_reactor(name),
                FieldRequest::NpcSpecialAction { npc_id, action } => {
                    self.npc_special_action(npc_id, action)
                }
                FieldRequest::ClearMobs => self.clear_mobs(),
                FieldRequest::BringOutSnowman => self.bring_out_snowman(),
                FieldRequest::BringOutScrooge => self.bring_out_scrooge(),
                FieldRequest::SetReactorState { name, state } => {
                    for view in self.reactors.force_forward(name, state) {
                        if let Ok(packet) = build_hit_reactor(&view, 0, 0) {
                            self.broadcast(None, packet);
                        }
                    }
                }
                FieldRequest::Catch {
                    object_id,
                    slot,
                    item_id,
                    catch,
                } => self.handle_catch(from, object_id, slot, item_id, &catch),
                FieldRequest::Unsummon { skill_id, why } => {
                    self.handle_unsummon(from, skill_id, why)
                }
                FieldRequest::SummonMove {
                    object_id,
                    start,
                    movement,
                } => self.handle_summon_move(from, object_id, start, movement),
                FieldRequest::PetMove {
                    index,
                    start,
                    movement,
                } => self.handle_pet_move(from, index, start, movement),
                FieldRequest::PetPickup {
                    index,
                    object_id,
                    x,
                    y,
                    mesos,
                    items,
                    ignored,
                    needs,
                } => {
                    let now = Instant::now();
                    let party = self.occupants.get(&from).and_then(|o| o.party);
                    let taken = self.drops.take_for_pet(
                        from,
                        party,
                        object_id,
                        (x, y),
                        now,
                        (mesos, items),
                        &ignored,
                        &needs,
                    );
                    if let (Ok(drop), Some(occupant)) = (taken, self.occupants.get(&from)) {
                        occupant
                            .handle
                            .send(ServerMessage::PetPickedUp { drop, pet: index });
                    }
                }
                FieldRequest::SummonAttack { attack, limits } => {
                    self.handle_summon_attack(from, attack, &limits)
                }
                FieldRequest::SummonDamage(damage) => self.handle_summon_damage(from, damage),
                FieldRequest::Beholder {
                    object_id,
                    action,
                    support,
                } => self.handle_beholder(from, object_id, action, support),
            },
            FieldMessage::SpawnLoot {
                owner,
                ownership,
                loot,
                x,
                y,
                dropper,
            } => self.spawn_loot(owner, ownership, loot, x, y, dropper),
            FieldMessage::PetLooted {
                looter,
                pet,
                drop,
                taken,
            } => {
                // The pet's drop was never taken off the field, so it only
                // goes back in the set when it didn't fit.
                if !taken {
                    self.drops.restore(drop);
                } else if let Ok(packet) = build_pet_loot_drop(drop.object_id, looter, pet) {
                    self.send_drop_packet(packet);
                }
            }
            FieldMessage::ReturnDrop { drop } => {
                let now = Instant::now();
                if let Ok(packet) = build_drop(&drop.view(now), DropAppearance::Lying) {
                    self.show_drop(&drop, packet);
                }
                self.drops.restore(drop);
            }
            FieldMessage::OpenDoor { owner, at } => {
                self.doors.insert(owner, at);
                if let Ok(packet) = build_spawn_door(owner, at, true) {
                    self.broadcast(None, packet);
                }
            }
            FieldMessage::CloseDoor { owner } => {
                if self.doors.remove(&owner).is_some() {
                    if let Ok(packet) = build_remove_door(owner) {
                        self.broadcast(None, packet);
                    }
                }
            }
            FieldMessage::Trade { from, request } => self.handle_trade(from, request),
            FieldMessage::PersonalShop { from, request } => self.handle_shop(from, request),
            FieldMessage::MiniGame { from, request } => self.handle_game(from, request),
            FieldMessage::Merchant { from, request } => self.handle_merchant(from, request),
        }
    }

    /// A trading room request from an occupant; anyone else's is dropped,
    /// with any decision it carries.
    fn handle_trade(&mut self, from: ClientId, request: RoomRequest) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let (name, look) = (&occupant.avatar.name, &occupant.avatar.look);
        let in_shop = self.shops.is_member(from) || self.games.is_member(from);
        let in_trade = self.trades.is_member(from);
        let busy = in_shop || self.merchants.is_member(from);
        let out = match request {
            RoomRequest::Create { room } => self.trades.create(room, from, name, look, busy),
            RoomRequest::Invite { target } => {
                let target_name = self.occupants.get(&target).map(|o| o.avatar.name.as_str());
                self.trades.invite(from, target, target_name)
            }
            RoomRequest::Decline { room, reason } => self.trades.decline(from, room, reason, name),
            // Merchants, shops, games and trading rooms share the visit.
            RoomRequest::Visit { room, .. } if self.merchants.contains(room) => {
                let now = (Instant::now(), SystemTime::now());
                let (id, busy) = (occupant.avatar.id, in_shop || in_trade);
                self.merchants.visit(from, id, room, name, look, busy, now)
            }
            RoomRequest::Visit { room, .. } if self.shops.contains(room) => {
                let busy = in_trade || self.games.is_member(from) || self.merchants.is_member(from);
                self.shops.visit(from, room, name, look, busy)
            }
            RoomRequest::Visit {
                room,
                password,
                records,
            } if self.games.contains(room) => {
                let busy = in_trade || self.shops.is_member(from) || self.merchants.is_member(from);
                let password = password.as_deref();
                self.games
                    .visit(from, room, name, look, password, records, busy)
            }
            RoomRequest::Visit { room, .. } => self.trades.visit(from, room, name, look, busy),
            RoomRequest::Chat { text } => self.trades.chat(from, &text),
            RoomRequest::PutItem { slot, item } => self.trades.put_item(from, slot, item),
            RoomRequest::Meso { total } => self.trades.meso(from, total),
            RoomRequest::Confirm => self.trades.confirm(from),
            RoomRequest::Answer { result, decision } => self.trades.answer(from, result, decision),
            RoomRequest::Leave => self.trades.leave(from),
        };
        self.deliver_trade(out);
        self.show_balloons();
        self.show_merchants();
    }

    /// A personal shop request from an occupant; anyone else's is dropped,
    /// with any oneshot it carries.
    fn handle_shop(&mut self, from: ClientId, request: ShopRoomRequest) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let (name, look) = (&occupant.avatar.name, &occupant.avatar.look);
        let shops = &mut self.shops;
        let out = match request {
            ShopRoomRequest::Create { room, title, skin } => {
                let busy = self.trades.is_member(from)
                    || self.games.is_member(from)
                    || self.merchants.is_member(from);
                shops.create(room, from, name, look, title, skin, busy)
            }
            ShopRoomRequest::Listings(listings) => shops.listings(from, listings),
            ShopRoomRequest::Open => {
                shops.open(from);
                Outbox::new()
            }
            ShopRoomRequest::Chat { text } => shops.chat(from, &text),
            ShopRoomRequest::Quote { index, reply } => {
                shops.quote(from, index, reply);
                Outbox::new()
            }
            ShopRoomRequest::Purchase(sale) => shops.purchase(from, sale),
            ShopRoomRequest::Ban {
                slot,
                name,
                overstayed,
            } => shops.ban(from, slot, &name, overstayed),
            ShopRoomRequest::Blacklist(names) => {
                shops.blacklist(from, names);
                Outbox::new()
            }
            ShopRoomRequest::BlacklistAdd(name) => {
                shops.blacklist_add(from, name);
                Outbox::new()
            }
            ShopRoomRequest::BlacklistRemove(name) => {
                shops.blacklist_remove(from, &name);
                Outbox::new()
            }
            ShopRoomRequest::SoldOut => shops.sold_out(from),
            ShopRoomRequest::Leave => shops.leave(from),
        };
        self.deliver_trade(out);
        self.show_balloons();
    }

    /// An Omok or match cards request from an occupant; anyone else's is
    /// dropped.
    fn handle_game(&mut self, from: ClientId, request: GameRoomRequest) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let (name, look) = (&occupant.avatar.name, &occupant.avatar.look);
        let out = match request {
            GameRoomRequest::Create {
                room,
                kind,
                title,
                password,
                piece,
                record,
            } => {
                let busy = self.trades.is_member(from)
                    || self.shops.is_member(from)
                    || self.merchants.is_member(from);
                self.games.create(
                    room, from, name, look, kind, title, password, piece, record, busy,
                )
            }
            GameRoomRequest::Play(request) => self.games.play(from, request),
            GameRoomRequest::Chat { text } => self.games.chat(from, &text),
            GameRoomRequest::Leave => self.games.leave(from),
        };
        self.deliver_trade(out);
        self.show_balloons();
    }

    /// Put up, change, or take down the balloons over shop and game owners.
    fn show_balloons(&mut self) {
        let mut balloons = self.shops.take_balloons();
        balloons.extend(self.games.take_balloons());
        for (owner, balloon) in balloons {
            let Some(occupant) = self.occupants.get_mut(&owner) else {
                continue;
            };
            occupant.avatar.mini_room = balloon;
            match build_char_box(owner, occupant.avatar.mini_room.as_ref()) {
                Ok(packet) => self.broadcast_seen_by(owner, None, packet),
                Err(error) => warn!(owner, %error, "Failed to build a room balloon"),
            }
        }
    }

    /// Send what the rooms say, to occupants and to owners here from
    /// elsewhere; those no longer in their merchant are forgotten after.
    fn deliver_trade(&mut self, out: Outbox) {
        for (to, message) in out {
            if let Some(occupant) = self.occupants.get(&to) {
                occupant.handle.send(message);
            } else if let Some(handle) = self.remote_owners.get(&to) {
                handle.send(message);
            }
        }
        let merchants = &self.merchants;
        self.remote_owners.retain(|&id, _| merchants.is_member(id));
    }

    /// A hired merchant request from an occupant, or from an owner in
    /// their merchant from elsewhere; anyone else's is dropped, with any
    /// oneshot it carries, but for a buyer's settling, which may come
    /// after they left, and an owner's merchant standing again after a
    /// failed close, as they are no longer in it.
    fn handle_merchant(&mut self, from: ClientId, request: MerchantRequest) {
        let now = Instant::now();
        let wall = SystemTime::now();
        if let Some(waiting) = &mut self.remote_waiting {
            // A remote visit waits for the merchants, and so does whatever
            // its owner sends next; leaving drops the visit.
            let visiting = waiting.iter().any(|(id, _)| *id == from);
            match request {
                MerchantRequest::RemoteVisit { .. } => {
                    waiting.push((from, request));
                    return;
                }
                MerchantRequest::Leave if visiting => {
                    waiting.retain(|(id, _)| *id != from);
                    return;
                }
                _ if visiting => {
                    waiting.push((from, request));
                    return;
                }
                _ => {}
            }
        }
        if let MerchantRequest::RemoteVisit { .. } = request {
            let MerchantRequest::RemoteVisit {
                sn,
                character_id,
                name,
                look,
                handle,
            } = request
            else {
                unreachable!("matched above")
            };
            self.remote_owners.insert(from, handle);
            let out =
                self.merchants
                    .remote_visit(from, character_id, sn, &name, &look, (now, wall));
            self.deliver_trade(out);
            self.show_merchants();
            return;
        }
        if let MerchantRequest::Settle {
            owner,
            id,
            sale,
            refused,
        } = request
        {
            let out = self.merchants.settle(owner, id, sale, refused, wall);
            self.deliver_trade(out);
            self.show_merchants();
            return;
        }
        if let MerchantRequest::Place {
            merchant,
            blacklist,
        } = request
        {
            self.merchants.place(from, *merchant, blacklist);
            self.show_merchants();
            return;
        }
        let at = self.occupants.get(&from).map(|o| (o.avatar.x, o.avatar.y));
        if at.is_none() && !self.remote_owners.contains_key(&from) {
            return;
        }
        if let MerchantRequest::Stand { reply, hold } = request {
            let stand = at.and_then(|(x, y)| self.merchant_stand(from, x, y));
            if let Some(stand) = stand.as_ref().filter(|_| hold) {
                self.merchants.hold_stand(from, stand.x, stand.y);
            }
            let _ = reply.send(stand);
            return;
        }
        let merchants = &mut self.merchants;
        let out = match request {
            MerchantRequest::Chat { text } => merchants.chat(from, &text),
            MerchantRequest::Buy {
                index,
                bundles,
                reply,
            } => merchants.buy(from, index, bundles, reply, now),
            MerchantRequest::Settle { .. }
            | MerchantRequest::Place { .. }
            | MerchantRequest::Stand { .. }
            | MerchantRequest::RemoteVisit { .. } => unreachable!("answered above"),
            MerchantRequest::Restock { stock, meso } => {
                merchants.restock(from, stock, meso);
                Outbox::new()
            }
            MerchantRequest::Visitors => merchants.visitors(from),
            MerchantRequest::Blacklist => merchants.blacklist(from),
            MerchantRequest::BlacklistAdd(name) => {
                merchants.blacklist_add(from, name);
                Outbox::new()
            }
            MerchantRequest::BlacklistRemove(name) => {
                merchants.blacklist_remove(from, &name);
                Outbox::new()
            }
            MerchantRequest::Reopen | MerchantRequest::Leave => merchants.leave(from, now),
            MerchantRequest::Unseat { owner } => merchants.unseat(from, owner, now),
            MerchantRequest::Close { reply } => merchants.close(from, reply),
        };
        self.deliver_trade(out);
        self.show_merchants();
    }

    /// Where `from`, at (x, y), would set up a merchant: the ground under
    /// them on a map with merchants, clear of the others and of the
    /// personal shops.
    fn merchant_stand(&self, from: ClientId, x: i16, y: i16) -> Option<Stand> {
        if self.key.instance_id != 0 || !net::personal_shop::allowed_on(self.key.map_id) {
            return None;
        }
        let (ground, foothold) = self.drops.foothold_below(x, y)?;
        let shops = self
            .occupants
            .iter()
            .filter(|(&id, o)| id != from && o.avatar.mini_room.is_some())
            .map(|(_, o)| (o.avatar.x, o.avatar.y));
        self.merchants
            .clear_of(from, x, ground, shops)
            .then_some(Stand {
                channel_id: self.key.channel_id,
                map_id: self.key.map_id,
                x,
                y: ground,
                foothold,
            })
    }

    /// Show everyone the merchants that came, changed or went, and read
    /// again those that may be out of date.
    fn show_merchants(&mut self) {
        for owner in self.merchants.take_rereads() {
            self.reload_merchant(owner);
        }
        for (owner, blacklist) in self.merchants.take_blacklists() {
            tokio::spawn(async move {
                for _ in 0..EXPIRE_TRIES {
                    let names = blacklist.clone();
                    match spawn_db(move || db::merchant::set_blacklist(owner, &names)).await {
                        Ok(()) => return,
                        Err(error) => warn!(owner, %error, "Failed to save a merchant's blacklist"),
                    }
                    tokio::time::sleep(MERCHANT_RETRY).await;
                }
            });
        }
        for shown in self.merchants.take_shown() {
            match self.merchants.show(shown) {
                Some(Ok(packet)) => self.broadcast(None, packet),
                Some(Err(error)) => warn!(?shown, %error, "Failed to build a merchant packet"),
                None => {}
            }
        }
    }

    /// Read the merchants open here, on a map that has them.
    fn load_merchants(&mut self) {
        if self.key.instance_id != 0 || !net::personal_shop::allowed_on(self.key.map_id) {
            return;
        }
        self.remote_waiting = Some(Vec::new());
        let (channel_id, map_id) = (self.key.channel_id, self.key.map_id);
        let loaded = self.loaded_tx.clone();
        tokio::spawn(async move {
            // Until the field closes.
            while !loaded.is_closed() {
                match spawn_db(move || db::merchant::load_map(channel_id, map_id)).await {
                    Ok(merchants) => {
                        let _ = loaded.send(Loaded::All(merchants));
                        return;
                    }
                    Err(error) => warn!(channel_id, map_id, %error, "Failed to load merchants"),
                }
                tokio::time::sleep(MERCHANT_RETRY).await;
            }
        });
    }

    /// Read `owner`'s merchant again; the merchant sells nothing until
    /// this is back.
    fn reload_merchant(&self, owner: i32) {
        let loaded = self.loaded_tx.clone();
        tokio::spawn(async move {
            while !loaded.is_closed() {
                match spawn_db(move || db::merchant::load(owner)).await {
                    Ok(merchant) => {
                        let _ = loaded.send(Loaded::One(owner, merchant));
                        return;
                    }
                    Err(error) => warn!(owner, %error, "Failed to reload a merchant"),
                }
                tokio::time::sleep(MERCHANT_RETRY).await;
            }
        });
    }

    fn merchants_loaded(&mut self, loaded: Loaded) {
        match loaded {
            Loaded::All(merchants) => {
                self.merchants.load(merchants);
                for (from, visit) in self.remote_waiting.take().unwrap_or_default() {
                    self.handle_merchant(from, visit);
                }
            }
            Loaded::One(owner, merchant) => {
                let out = self.merchants.reload(owner, merchant, SystemTime::now());
                self.deliver_trade(out);
            }
        }
        self.show_merchants();
    }

    /// Close the merchants whose time is up, and stop waiting on purchases
    /// held too long.
    fn tick_merchants(&mut self, now: Instant) {
        let wall = SystemTime::now();
        for owner in self.merchants.stale(now) {
            warn!(
                owner,
                "Purchases from a merchant held too long; reading it again"
            );
        }
        let (out, due) = self.merchants.expire(wall);
        self.deliver_trade(out);
        for owner in due {
            tokio::spawn(async move {
                for _ in 0..EXPIRE_TRIES {
                    match spawn_db(move || db::merchant::expire(owner, wall)).await {
                        Ok(_) => {
                            info!(owner, "Hired merchant closed: its time is up");
                            return;
                        }
                        Err(error) => warn!(owner, %error, "Failed to close an expired merchant"),
                    }
                    tokio::time::sleep(MERCHANT_RETRY).await;
                }
            });
        }
        self.show_merchants();
    }

    fn spawn_loot(
        &mut self,
        owner: ClientId,
        ownership: Ownership,
        loot: Vec<net::loot::Loot>,
        x: i16,
        y: i16,
        dropper: i32,
    ) {
        let now = Instant::now();
        let packets: Vec<(Drop, Packet)> = self
            .drops
            .spawn(loot, owner, ownership, x, y, dropper, now)
            .into_iter()
            .filter_map(|drop| {
                build_drop(
                    &drop.view(now),
                    DropAppearance::Falling {
                        from: (x, y),
                        delay: 0,
                    },
                )
                .ok()
                .map(|packet| (drop.clone(), packet))
            })
            .collect();
        for (drop, packet) in packets {
            self.show_drop(&drop, packet);
            self.put_on_reactor(&drop, now);
        }
    }

    /// Pickpocket's mesos fall from each mob still here, anyone's at once,
    /// within 50 of it and 100 ms apart (Cosmic's rule).
    fn pick_pockets(&mut self, from: ClientId, pockets: Vec<(i32, Vec<i32>)>) {
        let now = Instant::now();
        for (object_id, mesos) in pockets {
            let Some(mob) = self.mobs.get(object_id) else {
                continue;
            };
            let (x, y) = (mob.x, mob.y);
            for (index, amount) in mesos.into_iter().enumerate() {
                let place = x.saturating_add((rand::random::<u32>() % 100) as i16 - 50);
                let drop = self
                    .drops
                    .pocket(amount, from, (x, y), place, object_id, now);
                let falling = DropAppearance::Falling {
                    from: (x, y),
                    delay: (index as i16).saturating_mul(100),
                };
                if let Ok(packet) = build_drop(&drop.view(now), falling) {
                    let drop = drop.clone();
                    self.show_drop(&drop, packet);
                }
            }
        }
    }

    /// Have `character_id`, if they are here, answer `from`'s request for
    /// their info: their own actor holds the current level and fame.
    fn handle_char_info(&self, from: ClientId, character_id: i32) {
        let Some(requester) = self.occupants.get(&from) else {
            return;
        };
        if let Some(target) = self
            .occupants
            .values()
            .find(|occupant| occupant.avatar.id == character_id)
        {
            target.handle.send(ServerMessage::CharInfoRequested {
                requester: requester.handle.clone(),
            });
        } else if let Ok(packet) = net::packet::build::world::stat::build_stat_update(true, &[]) {
            // Unlock the client, which waits on CHAR_INFO otherwise.
            requester.handle.send_packet(packet);
        }
    }

    /// Hand `from`'s fame to `target`, if they are here.
    fn handle_fame(&self, from: ClientId, target: i32, giver_name: String, up: bool) {
        let Some(giver) = self.occupants.get(&from) else {
            return;
        };
        if let Some(occupant) = self
            .occupants
            .values()
            .find(|occupant| occupant.avatar.id == target)
        {
            occupant.handle.send(ServerMessage::Famed {
                giver: giver.handle.clone(),
                giver_name,
                up,
            });
        } else {
            giver.handle.send(ServerMessage::FameMissed { target });
        }
    }

    /// Put down what a player threw away where they stand, or show it
    /// falling and fading if it can't lie on the ground.
    fn handle_throw(&mut self, from: ClientId, loot: net::loot::Loot, vanish: bool) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let (x, y) = (occupant.avatar.x, occupant.avatar.y);
        let now = Instant::now();
        let (drop, appearance) = if vanish {
            let drop = Drop {
                object_id: self.drops.next_object_id(),
                loot,
                owner: from,
                ownership: Ownership::Owner,
                x,
                y,
                dropper: 0,
                dropped_at: now,
                player_drop: true,
            };
            (drop, DropAppearance::Vanishing { from: (x, y) })
        } else {
            let drop = self.drops.throw(loot, from, x, y, now).clone();
            self.put_on_reactor(&drop, now);
            let falling = DropAppearance::Falling {
                from: (x, y),
                delay: 0,
            };
            (drop, falling)
        };
        if let Ok(packet) = build_drop(&drop.view(now), appearance) {
            self.show_drop(&drop, packet);
        }
    }

    /// Items a mob's hit knocked out of `from`'s bag fall one by one from
    /// them, for anyone at once: the `n`th (from 1) lands `shed_offset(n)`
    /// from their feet, on the ground below (Cosmic's TakeDamageHandler).
    fn handle_shed(&mut self, from: ClientId, item_id: i32, count: i16) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let (x, y) = (occupant.avatar.x, occupant.avatar.y);
        let now = Instant::now();
        for n in 1..=count {
            let at = x.saturating_add(shed_offset(n));
            let ground = self.drops.ground_below(at, y.saturating_sub(1));
            let loot = Loot::Dropped(game::Item::stack(item_id, 1));
            let drop = self
                .drops
                .throw(loot, from, at, ground.unwrap_or(y), now)
                .clone();
            self.put_on_reactor(&drop, now);
            let falling = DropAppearance::Falling {
                from: (x, y),
                delay: 0,
            };
            if let Ok(packet) = build_drop(&drop.view(now), falling) {
                self.show_drop(&drop, packet);
            }
        }
    }

    /// Where a drop lying at its spot comes to rest: on the ground below.
    fn landing(&self, drop: &Drop) -> (i16, i16) {
        let ground = self.drops.ground_below(drop.x, drop.y.saturating_sub(1));
        (drop.x, ground.unwrap_or(drop.y))
    }

    /// If a drop lands on a reactor that takes it, the reactor takes it a
    /// little later.
    fn put_on_reactor(&mut self, drop: &Drop, now: Instant) {
        let Some((item_id, quantity)) = reactor_item(&drop.loot) else {
            return;
        };
        self.put_on_snowman(drop, item_id, quantity, now);
        if !self.reactors.any_takes_items() {
            return;
        }
        if self
            .items_on_reactors
            .iter()
            .any(|on| on.drop == drop.object_id)
        {
            return;
        }
        let at = self.landing(drop);
        if let Some(reactor) = self.reactors.taking(item_id, quantity, at) {
            self.items_on_reactors.push(ItemOnReactor {
                at: now + ITEM_TAKEN_AFTER,
                drop: drop.object_id,
                reactor,
            });
        }
    }

    /// The reactor takes the item put on it, if both are still there and
    /// it still wants it, and moves on. A reactor that waits for another
    /// item then looks at what already lies on it. Says whether it took it.
    fn take_item_on_reactor(&mut self, on: ItemOnReactor, now: Instant) -> bool {
        let Some(drop) = self.drops.get(on.drop) else {
            return false;
        };
        let Some((item_id, quantity)) = reactor_item(&drop.loot) else {
            return false;
        };
        let (owner, at) = (drop.owner, self.landing(drop));
        let Some(hit) = self
            .reactors
            .take_item(on.reactor, item_id, quantity, at, now)
        else {
            return false;
        };
        if let Some(drop) = self.drops.remove(on.drop) {
            if let Ok(packet) = build_remove_drop(drop.object_id, None) {
                self.send_drop_packet(packet);
            }
        }
        self.show_reactor_hit(owner, hit, 0);
        if self.reactors.takes_items(on.reactor) {
            let lying: Vec<Drop> = self.drops.all().cloned().collect();
            for drop in lying {
                self.put_on_reactor(&drop, now);
            }
        }
        true
    }

    /// Send a drop packet to everyone here. Those who don't see a quest
    /// drop ignore it going away.
    fn send_drop_packet(&self, packet: Packet) {
        for occupant in self.occupants.values() {
            occupant.handle.send_packet(packet.clone());
        }
    }

    /// Show a drop to everyone here who may see it: a quest drop only to
    /// those who still need it, which their client actors decide.
    fn show_drop(&self, drop: &Drop, packet: Packet) {
        for occupant in self.occupants.values() {
            show_drop_to(&occupant.handle, drop, packet.clone());
        }
    }

    fn handle_pickup(
        &mut self,
        from: ClientId,
        object_id: i32,
        x: i16,
        y: i16,
        needs: &net::loot::QuestNeeds,
    ) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let handle = occupant.handle.clone();
        match self
            .drops
            .take(from, occupant.party, object_id, x, y, Instant::now(), needs)
        {
            Ok(drop) => {
                if let Ok(packet) = build_remove_drop(object_id, Some(from)) {
                    self.send_drop_packet(packet);
                }
                handle.send(ServerMessage::PickedUp { drop });
            }
            Err(_) => {
                // Unlock the client's pickup.
                if let Ok(packet) = net::packet::build::world::stat::build_stat_update(true, &[]) {
                    handle.send_packet(packet);
                }
            }
        }
    }

    fn handle_mob_move(
        &mut self,
        from: ClientId,
        object_id: i32,
        move_id: i16,
        (want_attack, action, skill_data): (bool, u8, i32),
        start: (i16, i16),
        movement: &[u8],
    ) {
        if !self.mobs.move_mob(from, object_id, start, movement) {
            return;
        }
        let Some(act) = self.mobs.act(
            object_id,
            (want_attack, action, skill_data),
            Instant::now(),
            &net::mobs::skill_level,
            rand::random(),
        ) else {
            return;
        };
        if let Some(occupant) = self.occupants.get(&from) {
            let grant = act.grant.unwrap_or_default();
            if let Ok(packet) =
                build_move_mob_response(object_id, move_id, act.next_attack, act.mp, grant)
            {
                occupant.handle.send_packet(packet);
            }
        }
        self.casts.extend(act.cast);
        let skill_data = if act.action == action { skill_data } else { 0 };
        let moved = MobMoveView {
            object_id,
            next_attack: act.next_attack,
            action: act.action,
            skill_data,
            start,
        };
        if let Ok(packet) = build_move_mob(&moved, movement) {
            self.broadcast(Some(from), packet);
        }
    }

    /// `from` attacked the mobs in `hits` with `skill`, showing the others
    /// `broadcast`; each one with the `counter` status hurts them back, once
    /// (v83 FUN_007937e3, rolled again here: its client only shows its own
    /// roll). A mob that only some skills hurt takes nothing from the others,
    /// as their v83 clients count it a miss (FUN_0068146c); a reflection
    /// (no skill) isn't judged. What it `takes` from them goes back to
    /// `from` as one message, and on a Nett's Pyramid stage what it did to
    /// the mobs goes to their run. A mob immune to the attack's kind
    /// (PImmune, or MImmune for a spell) takes at most 1 a damage line
    /// (Cosmic's `AbstractDealDamageHandler`): `counter` comes with the
    /// lines it dealt each mob.
    #[allow(clippy::too_many_arguments)]
    fn handle_attack(
        &mut self,
        from: ClientId,
        broadcast: Option<Packet>,
        skill: Option<i32>,
        counter: Option<(MobStat, u8)>,
        hits: Vec<(i32, i64)>,
        inflicts: &[Inflict],
        takes: Takes,
    ) {
        let Some(attacker) = self.occupants.get(&from) else {
            return;
        };
        let (id, level, side) = (
            attacker.avatar.id,
            attacker.avatar.level,
            attacker.avatar.team,
        );
        let attacker = attacker.handle.clone();
        if let Some(broadcast) = broadcast {
            self.broadcast_by(from, broadcast);
        }
        // Players can't hurt a friendly mob (`damagedByMob`), only mobs can,
        // and no one a shielded one: their hits do nothing to it at all.
        let friendly = |object_id| {
            self.mobs
                .get(object_id)
                .is_some_and(|mob| mob.info.damaged_by_mob)
        };
        let hits: Vec<_> = hits
            .into_iter()
            .filter(|&(object_id, _)| !friendly(object_id) && !self.mobs.shielded(object_id))
            .collect();
        let now = Instant::now();
        let countered: i32 = counter.map_or(0, |(stat, _)| {
            hits.iter()
                .map(|&(object_id, _)| self.mobs.counter(object_id, stat, rand::random()))
                .filter(|&damage| damage > 0)
                .sum()
        });
        if countered > 0 {
            attacker.send(ServerMessage::Countered {
                damage: countered,
                map_id: self.key.map_id,
            });
        }
        let (mut hp_taken, mut mp_taken) = (0, 0);
        let mut massacre = net::pyramid::Hits::default();
        let mut carnival_cp = 0;
        for (object_id, damage) in hits {
            // Steal rolls on every mob hit, until one gives (Cosmic's rule).
            if takes.steal > 0 && rand::random::<f64>() * 100.0 < f64::from(takes.steal) {
                if let Some((mob_id, x, y)) = self.mobs.steal(object_id) {
                    attacker.send(ServerMessage::Stole {
                        mob_id,
                        object_id,
                        x,
                        y,
                    });
                }
            }
            let Some(mob) = self.mobs.get(object_id) else {
                continue;
            };
            let hurts = skill.is_none_or(|skill| mob.info.hurt_by(skill));
            // A Monster Carnival side earns nothing for its own mobs.
            let cp = if mob.team == Some(side) {
                0
            } else {
                mob.info.cp
            };
            let immune =
                counter.filter(|&(stat, _)| mob.statuses.contains_key(&immunity_against(stat)));
            let mut damage = match immune {
                _ if !hurts => 0,
                Some((_, lines)) => damage.min(i64::from(lines)),
                None => damage,
            };
            let max_hp = i64::from(mob.info.max_hp);
            if let Some(cap) =
                skill.and_then(|skill| net::skills::pq_strike_cap(skill, max_hp, mob.info.boss))
            {
                damage = damage.min(cap);
            }
            // Mortal Blow's kill takes what the mob has left.
            if takes.life > 0
                && damage > 0
                && !mob.info.boss
                && mob.hp * 100 <= max_hp * i64::from(takes.life)
            {
                damage = damage.max(mob.hp);
            }
            if damage > 0 {
                hp_taken += (damage * i64::from(takes.hp) / 100).min(max_hp);
            }
            let cool = net::pyramid::cool(&mob.info, damage, rand::random::<u32>() % 100);
            match self.mobs.damage(from, object_id, damage, now) {
                Some(Ok(_))
                    if damage == 0
                        && self
                            .mobs
                            .get(object_id)
                            .is_some_and(|mob| mob.info.remove_on_miss) =>
                {
                    if let Some(leave) = self.mobs.missed(object_id, now) {
                        if let Ok(packet) = build_kill_mob(object_id, leave) {
                            self.broadcast(None, packet);
                        }
                    }
                    massacre.misses += 1;
                }
                Some(Ok(percent)) => {
                    self.show_mob_hp(from, object_id, percent);
                    if damage > 0 {
                        self.show_boss_bar(self.mobs.bar_for(object_id));
                    }
                    let eater = takes
                        .mp
                        .filter(|eater| damage > 0 && eater.eats(rand::random()));
                    if let Some(eater) = eater {
                        mp_taken += self.mobs.eat_mp(object_id, eater.x);
                    }
                    if net::skills::provokes(skill) {
                        self.provoke(from, &attacker, object_id, now);
                    }
                    // A miss puts nothing on it.
                    for inflict in inflicts.iter().filter(|_| damage > 0) {
                        if inflict.takes(rand::random()) {
                            self.put_on(from, object_id, inflict, now);
                        }
                    }
                }
                Some(Err(kill)) => {
                    if cool {
                        massacre.cools += 1;
                    } else {
                        massacre.kills += 1;
                    }
                    carnival_cp += cp;
                    self.killed(&kill);
                }
                None => {}
            }
        }
        if self.massacre && massacre.any() {
            attacker.send(ServerMessage::Massacre(massacre));
        }
        if self.carnival && carnival_cp > 0 {
            attacker.send(ServerMessage::CarnivalCp(carnival_cp));
        }
        // MP Eater shows when it ate, once for the attack.
        if let Some(eater) = takes.mp.filter(|_| mp_taken > 0) {
            if let Ok(packet) = build_own_skill_effect(eater.skill_id, level, eater.level) {
                attacker.send_packet(packet);
            }
            if let Ok(packet) = build_skill_effect(id, eater.skill_id, level, eater.level) {
                self.broadcast_by(from, packet);
            }
        }
        if hp_taken > 0 || mp_taken > 0 {
            attacker.send(ServerMessage::Absorbed {
                hp: i32::try_from(hp_taken).unwrap_or(i32::MAX),
                mp: mp_taken,
                map_id: self.key.map_id,
            });
        }
    }

    /// A mob died: everyone sees it go, and those who hurt it share its
    /// EXP.
    fn killed(&mut self, kill: &Kill) {
        if let Ok(packet) = build_kill_mob(kill.object_id, kill.leave) {
            self.broadcast(None, packet);
        }
        self.show_boss_bar(kill.boss_bar);
        let sharers: Vec<Sharer> = self
            .occupants
            .iter()
            .map(|(&client, occupant)| Sharer {
                client,
                level: occupant.avatar.level,
                party: occupant.party,
            })
            .collect();
        // One roll per kill, for whoever hit it hardest.
        let looter = kill.damage_by.first().map(|&(client, _)| client);
        for share in split_exp(kill.exp, kill.level, &kill.damage_by, &sharers) {
            if let Some(occupant) = self.occupants.get(&share.client) {
                occupant.handle.send(ServerMessage::MobKilled {
                    mob_id: kill.mob_id,
                    object_id: kill.object_id,
                    exp: share.exp,
                    party_bonus: share.party_bonus,
                    looter: looter == Some(share.client),
                    x: kill.x,
                    y: kill.y,
                    showdown: kill.showdown,
                });
            }
        }
        // Each living player here takes what it leaves (Cosmic
        // `getBuffToGive`); the client knows who is alive.
        let buff = net::mobs::info(kill.mob_id).map_or(0, |info| info.buff);
        if buff > 0 {
            for occupant in self.occupants.values() {
                occupant.handle.send(ServerMessage::FieldBuff(buff));
            }
        }
        // A kill that lets out the map's time mob tells everyone here, as So
        // Gong runs away from his trial (Cosmic `Monster.killBy`).
        let time_mob = net::get_game_data()
            .ok()
            .and_then(|data| data.field(self.key.map_id)?.time_mob.clone());
        if let Some((mob_id, message)) = time_mob {
            let lets_out =
                net::mobs::info(kill.mob_id).is_some_and(|info| info.revive.contains(&mob_id));
            if lets_out {
                if let Ok(packet) = build_item_line(&message, 0) {
                    self.broadcast(None, packet);
                }
            }
        }
        self.make_real(kill.mob_id);
        self.leave_snow_vigor(kill);
        match (kill.mob_id, self.key.map_id) {
            (net::mobs::ZAKUM_LAST, net::mobs::ZAKUM_ALTAR_MAP) => self.zakum_falls(),
            (net::mobs::HORNTAIL, net::events::HORNTAILS_CAVE) => self.horntail_falls(),
            (net::mobs::PAPULATUS_LAST, net::events::ORIGIN_OF_CLOCKTOWER) => self.boss_falls(),
            (net::mobs::LATANICA, net::events::ENGINE_ROOM) if self.key.instance_id != 0 => {
                self.boss_falls()
            }
            (net::events::PRIME_MINISTER, net::events::WEDDING_HALL_COPY)
                if self.key.instance_id != 0 =>
            {
                self.boss_falls()
            }
            // An era's Nex in its own room (Cosmic `GuardianNex`'s
            // `monsterKilled`).
            (mob, map)
                if self.key.instance_id != 0
                    && (0..net::events::NEX_BOSSES.len() as u8).any(|era| {
                        net::events::nex_room(era) == map
                            && net::events::NEX_BOSSES[usize::from(era)] == mob
                    }) =>
            {
                self.boss_falls()
            }
            // An era's boss's last form in its room (Cosmic `TD_Battle`'s
            // `monsterKilled`).
            (mob, map)
                if self.key.instance_id != 0
                    && (1..=net::events::NEO_BOSSES.len() as u8).any(|era| {
                        net::events::neo_boss_room(era) == map
                            && net::events::NEO_BOSSES[usize::from(era) - 1] == mob
                    }) =>
            {
                self.boss_falls()
            }
            // The sealed Balrog comes out a little later (Cosmic
            // `BalrogBattle.spawnSealedBalrog`), and everyone here counts
            // the kill Balrog and the Seal (2239) asks for.
            (net::mobs::BALROG_BODY, net::events::BALROG_TOMB) if self.key.instance_id != 0 => {
                self.boss_falls();
                self.announce(keys::BALROG_VICTORY, &[]);
                let sealed = FieldMessage::BringOut {
                    mob_id: net::mobs::SEALED_BALROG,
                    at: net::events::BALROG_AT,
                };
                self.after(net::events::SEALED_BALROG_AFTER, sealed);
                for occupant in self.occupants.values() {
                    occupant.handle.send(ServerMessage::MobKilled {
                        mob_id: net::mobs::BALROG_SEALED_CREDIT,
                        object_id: kill.object_id,
                        exp: 0,
                        party_bonus: 0,
                        looter: false,
                        x: kill.x,
                        y: kill.y,
                        showdown: kill.showdown,
                    });
                }
            }
            // Every mob in the arena goes with The Boss (Cosmic
            // `ShowaBattle.clearPQ`).
            (net::events::THE_BOSS, net::events::SHOWA_ARENA) if self.key.instance_id != 0 => {
                self.boss_falls();
                self.clear_mobs();
            }
            // Every mob in the arena goes with Pink Bean, and the world reads
            // which channel's expedition won (Cosmic
            // `PinkBeanBattle.monsterKilled`, `broadcastPinkBeanVictory`).
            (net::mobs::PINK_BEAN, net::events::PINK_BEAN_ARENA) if self.key.instance_id != 0 => {
                self.boss_falls();
                self.clear_mobs();
                let channel = (u16::from(self.key.channel_id) + 1).to_string();
                self.announce(keys::PINK_BEAN_VICTORY, &[("channel", &channel)]);
            }
            // Pink Bean comes out where its statue fell (Royals's statue
            // has no `revive`, unlike Cosmic's).
            (net::mobs::PINK_BEAN_STATUE, net::events::PINK_BEAN_ARENA)
                if self.key.instance_id != 0 && self.out(net::mobs::PINK_BEAN).is_empty() =>
            {
                self.board(net::mobs::PINK_BEAN, 1, (kill.x, kill.y));
            }
            (mob_id, net::events::PINK_BEAN_ARENA)
                if net::events::pink_bean_guardian(mob_id) && self.key.instance_id != 0 =>
            {
                self.guardian_falls()
            }
            (mob_id, _) if net::events::is_frankenroid(mob_id) => self.frankenroid_falls(),
            (mob_id, map_id)
                if net::events::is_lord_pirate(mob_id)
                    && map_id == net::events::PIRATE_STAGES[5]
                    && self.key.instance_id != 0 =>
            {
                self.lord_pirate_falls()
            }
            (net::events::TRISTAN_BALROG, net::events::TRISTAN_ROOM)
                if self.key.instance_id != 0 =>
            {
                self.tristan_balrog_falls()
            }
            _ => {}
        }
        // A trial's head clears its stage, which unseals its door; each
        // member hears of it and those here see it (the event's
        // `stage_cleared`).
        let head = net::events::trial_head(self.key.map_id)
            .filter(|(_, head)| head.mob == kill.mob_id && self.key.instance_id != 0);
        if let Some((stage, _)) = head {
            for occupant in self.occupants.values() {
                occupant.handle.send(ServerMessage::ClearEventStage(stage));
            }
        }
        // A stage cleared by killing all its mobs clears with the last, and
        // one cleared by its boss with the boss.
        let cleared = net::events::cleared_by_killing(self.key.map_id)
            .filter(|_| self.mobs.live().next().is_none() && self.sealed())
            .or(net::events::cleared_by_boss(self.key.map_id, kill.mob_id))
            .filter(|_| self.key.instance_id != 0);
        if let Some(stage) = cleared {
            for occupant in self.occupants.values() {
                occupant.handle.send(ServerMessage::ClearEventStage(stage));
            }
            // Yulete's office's way out opens with its clear (Cosmic
            // `MagatiaPQ_Z.monsterKilled`'s `forceHitReactor(1)`).
            let map_id = self.key.map_id;
            let office =
                net::events::MagatiaSide::of(map_id).filter(|side| side.stages()[4] == map_id);
            if let Some(side) = office {
                for view in self.reactors.set_state(side.office_door(), 1) {
                    if let Ok(packet) = build_hit_reactor(&view, 0, 0) {
                        self.broadcast(None, packet);
                    }
                }
            }
        }
    }

    /// Yulete, there since the first came into a run's copy of his office,
    /// leaves once his time is up and anyone is there to see it: the run
    /// hears what he said, by whether he was caught off guard, and five of
    /// each of the mobs he left come out at each of their spots (Cosmic
    /// `MagatiaPQ_Z.yuleteAction`).
    fn tick_yulete(&mut self, now: Instant) {
        let Some((mood, Some(leaves))) = self.yulete else {
            return;
        };
        if leaves > now || self.occupants.is_empty() {
            return;
        }
        self.yulete = Some((net::events::Yulete::Leaving, None));
        let npc_id = net::events::MagatiaSide::of(self.key.map_id).map(net::npc_scripts::yulete);
        if let Some(index) = self
            .map_npcs
            .iter()
            .position(|npc| Some(npc.npc_id) == npc_id)
        {
            let object_id = self.map_npcs.remove(index).object_id;
            for packet in [build_uncontrol_npc(object_id), build_remove_npc(object_id)] {
                if let Ok(packet) = packet {
                    self.broadcast(None, packet);
                }
            }
        }
        for at in net::events::YULETE_GUARD_SPOTS {
            for mob_id in net::events::YULETE_GUARDS {
                self.board(mob_id, 5, at);
            }
        }
        let caught = mood == net::events::Yulete::Caught;
        if let Some(occupant) = self.occupants.values().next() {
            occupant.handle.send(ServerMessage::YuleteLeft(caught));
        }
    }

    /// What Yulete says to someone who talks to him in his office, as text
    /// key, by whether anyone here stands within `YULETE_SEES` of where he
    /// came out and whether Romeo or Juliet read the letter
    /// (`net::events::yulete_talk`); `None` once he has left, or anywhere
    /// else.
    fn talk_to_yulete(&mut self, shocked: bool) -> Option<&'static str> {
        let (mood, leaves) = self.yulete?;
        leaves?;
        // Measured from where he was put, not the ground he stands on, as
        // Cosmic's `2112000` does.
        let (x, y) = net::events::YULETE_AT;
        let (x, y) = (i32::from(x), i32::from(y));
        let sees = net::events::YULETE_SEES;
        let too_close = self.occupants.values().any(|occupant| {
            let dx = i32::from(occupant.avatar.x) - x;
            let dy = i32::from(occupant.avatar.y) - y;
            dx * dx + dy * dy < sees * sees
        });
        let (key, mood) = net::events::yulete_talk(mood, too_close, shocked);
        self.yulete = Some((mood, leaves));
        Some(key)
    }

    /// What Yulete says to someone who talks to him in a run's copy of the
    /// boss's room, as text key (`net::events::yulete_in_lab`): the first
    /// time before the run is won, he brings out his Frankenroid on the
    /// ground below where it stands, the mighty one if he was `caught` off
    /// guard in his office, and the run hears of it. `None` anywhere else.
    fn call_frankenroid(&mut self, caught: bool, won: bool) -> Option<&'static str> {
        let map_id = self.key.map_id;
        let side = net::events::MagatiaSide::of(map_id)
            .filter(|side| side.stages()[6] == map_id && self.key.instance_id != 0)?;
        let key = net::events::yulete_in_lab(won, self.frankenroid, caught);
        if won || self.frankenroid {
            return Some(key);
        }
        self.frankenroid = true;
        self.board(side.frankenroid(caught), 1, net::events::FRANKENROID_AT);
        if let Some(occupant) = self.occupants.values().next() {
            occupant.handle.send(ServerMessage::FrankenroidOut(caught));
        }
        Some(key)
    }

    /// A Frankenroid died in a run's copy of the boss's room: every mob
    /// but the escort goes and no more come, the side's Romeo or Juliet
    /// comes out, and the run is won, its clear shown and paid to every
    /// member (Cosmic `MagatiaPQ_Z.monsterKilled`).
    fn frankenroid_falls(&mut self) {
        let map_id = self.key.map_id;
        let side = net::events::MagatiaSide::of(map_id)
            .filter(|side| side.stages()[6] == map_id && self.key.instance_id != 0);
        let Some(side) = side else {
            return;
        };
        let saved = net::npc_scripts::magatia_hub_host(side);
        if !self.map_npcs.iter().any(|npc| npc.npc_id == saved) {
            self.appear(saved, net::events::MAGATIA_SAVED_AT);
        }
        self.clear_mobs();
        // Each says again whether the escort fainted before the clear, so
        // the run knows it whichever member's clear comes first.
        for occupant in self.occupants.values() {
            if self.escort_fainted {
                occupant.handle.send(ServerMessage::EscortFailed);
            }
            occupant.handle.send(ServerMessage::ClearEventStage(
                net::events::MAGATIA_BOSS_STAGE,
            ));
        }
    }

    /// The Lord Pirate is beaten: everyone here is told, and Wu Yang comes
    /// out where he stood, once (Cosmic `PiratePQ`'s `monsterKilled`).
    fn lord_pirate_falls(&mut self) {
        let wu_yang = net::npc_scripts::WU_YANG;
        if self.map_npcs.iter().any(|npc| npc.npc_id == wu_yang) {
            return;
        }
        let notice = net::content::text().get(keys::PIRATE_WU_FREED, &[]);
        if let Ok(packet) = build_broadcast(Broadcast::Pink, &notice) {
            self.broadcast(None, packet);
        }
        self.appear(wu_yang, net::events::LORD_PIRATE_AT);
    }

    /// The unknown Jr. Balrog is beaten: Tristan's Spirit comes out, once,
    /// for 2238 to end at (Cosmic `BalrogQuest.monsterKilled`).
    fn tristan_balrog_falls(&mut self) {
        let spirit = net::events::TRISTANS_SPIRIT;
        if !self.map_npcs.iter().any(|npc| npc.npc_id == spirit) {
            self.appear(spirit, net::events::TRISTANS_SPIRIT_AT);
        }
    }

    /// Whether every reactor here that must be set before the stage clears
    /// is (`net::events::seals`).
    fn sealed(&self) -> bool {
        net::events::seals(self.key.map_id).iter().all(|name| {
            self.reactors
                .named(name)
                .is_some_and(|reactor| reactor.state > reactors::first_state(reactor.reactor_id))
        })
    }

    /// Put `head` on the ground below its place, for whoever comes in to
    /// control (Cosmic `spawnMonsterOnGroundBelow`).
    fn bring_out_head(&mut self, head: &net::events::TrialHead) {
        let Some(info) = net::mobs::info(head.mob) else {
            return;
        };
        let (x, y) = head.at;
        let ground = self.drops.ground_below(x, y.saturating_sub(1)).unwrap_or(y);
        self.mobs
            .place(info, (x, ground.saturating_sub(1)), 5, None, Instant::now());
    }

    /// A boss whose death wins its fight died: the map sees the party
    /// quests' clear effect and hears their jingle, and in an event's copy
    /// of the map the event is won (Cosmic `showClearEffect`, `clearPQ`).
    fn boss_falls(&mut self) {
        for packet in [
            build_field_effect("quest/party/clear"),
            build_field_sound("Party1/Clear"),
        ]
        .into_iter()
        .flatten()
        {
            self.broadcast(None, packet);
        }
        if self.key.instance_id != 0 {
            for occupant in self.occupants.values() {
                occupant.handle.send(ServerMessage::ClearEventStage(1));
            }
        }
    }

    /// Zakum's last body died on its altar: it falls as any boss, and the
    /// world reads of it (Cosmic `ZakumBattle.monsterKilled`,
    /// `broadcastZakumVictory`).
    fn zakum_falls(&mut self) {
        self.boss_falls();
        self.announce(keys::ZAKUM_VICTORY, &[]);
    }

    /// Handle `message` `after` (scaled, `net::events::scaled`) from now.
    fn after(&mut self, after: Duration, message: FieldMessage) {
        let at = Instant::now() + net::events::scaled(after);
        self.later.push((at, message));
    }

    /// The world reads the line `key` with `vars` (an item line), through
    /// the first client here that takes it.
    fn announce(&self, key: &str, vars: &[(&str, &str)]) {
        let line = net::content::text().get(key, vars);
        let Ok(packet) = build_item_line(&line, 0) else {
            return;
        };
        self.occupants.values().any(|occupant| {
            occupant
                .handle
                .send(ServerMessage::Announce(packet.clone()))
        });
    }

    /// Horntail died in his cave: in the expedition's copy, its last stage
    /// clears, which wins it and shows those here the clear; and the world
    /// reads of it (Cosmic `HorntailBattle.monsterKilled`,
    /// `broadcastHorntailVictory`).
    fn horntail_falls(&mut self) {
        if self.key.instance_id != 0 {
            let stage = net::events::HORNTAIL.last_stage();
            for occupant in self.occupants.values() {
                occupant.handle.send(ServerMessage::ClearEventStage(stage));
            }
        }
        self.announce(keys::HORNTAIL_VICTORY, &[]);
    }

    /// The object ids of the live `mob_id`s here.
    fn out(&self, mob_id: i32) -> Vec<i32> {
        let mobs = self.mobs.live().filter(|mob| mob.info.mob_id == mob_id);
        mobs.map(|mob| mob.object_id).collect()
    }

    /// Kill every live `mob_id` here outright, as one that dies, giving
    /// nothing, as no one hurt it.
    fn slay(&mut self, mob_id: i32) {
        let now = Instant::now();
        for object_id in self.out(mob_id) {
            if let Some(kill) = self.mobs.slay(object_id, now) {
                self.killed(&kill);
            }
        }
    }

    /// Everyone in the Twilight of Gods hears the guardians' next wave is
    /// coming, which wakes a while later (Cosmic `dropMessage(5, ..)`, then
    /// `schedule("startWave", 15 * 1000)`).
    fn guardians_stir(&mut self, notice: &str) {
        let notice = net::content::text().get(notice, &[]);
        if let Ok(packet) = build_broadcast(Broadcast::Pink, &notice) {
            self.broadcast(None, packet);
        }
        let wake = FieldMessage::GuardiansWake;
        self.after(net::events::PINK_BEAN_WAVE_AFTER, wake);
    }

    /// The guardians of the wave due wake, as many as its number, in their
    /// order: each where one of its statues stands, which goes, or at the
    /// statues' spot if none does (Cosmic `startWave`'s `spawnJrBoss`).
    fn guardians_wake(&mut self) {
        let Some(wave) = self.guardians_wave else {
            return;
        };
        let guardians = net::events::PINK_BEAN_GUARDIANS;
        let now = Instant::now();
        for &(statues, guardian) in guardians.iter().take(wave) {
            let statue = self
                .mobs
                .live()
                .find(|mob| statues.contains(&mob.info.mob_id))
                .map(|mob| (mob.object_id, (mob.x, mob.y)));
            let at = match statue {
                Some((object_id, at)) => {
                    let gone = self.mobs.take_away_one(object_id, now);
                    self.show_left(gone);
                    at
                }
                None => net::events::PINK_BEAN_GUARDIANS_AT,
            };
            self.board(guardian, 1, at);
        }
    }

    /// One of the goddess's guardians fell. With none left awake, the next
    /// wave stirs; after the last, the item that calls the lightning falls
    /// on the statue's reactor, and the statue takes harm (Cosmic
    /// `PinkBeanBattle.monsterKilled`).
    fn guardian_falls(&mut self) {
        let awake = self
            .mobs
            .live()
            .any(|mob| net::events::pink_bean_guardian(mob.info.mob_id));
        let Some(wave) = self.guardians_wave.filter(|_| !awake) else {
            return;
        };
        if wave > net::events::PINK_BEAN_WAVES {
            return;
        }
        self.guardians_wave = Some(wave + 1);
        if wave < net::events::PINK_BEAN_WAVES {
            self.guardians_stir(keys::PINK_BEAN_WAVE_NEXT);
            return;
        }
        self.mobs.ward(net::mobs::PINK_BEAN_STATUE, false);
        let spot = self.reactors.spot_of(reactors::PINK_BEAN_LIGHTNING);
        let owner = self.occupants.keys().next().copied();
        let (Some((x, y)), Some(owner)) = (spot, owner) else {
            // No one here to drop it for: the lightning strikes at once.
            self.slay(net::mobs::PINK_BEAN_STATUE);
            return;
        };
        let lightning = net::loot::Loot::Item {
            item_id: net::events::PINK_BEAN_LIGHTNING_ITEM,
            quantity: 1,
            quest: None,
        };
        self.spawn_loot(owner, Ownership::FreeForAll, vec![lightning], x, y, 0);
        let notice = net::content::text().get(keys::PINK_BEAN_WAVE_LAST, &[]);
        if let Ok(packet) = build_item_line(&notice, 0) {
            self.broadcast(None, packet);
        }
    }

    /// All eight of Horntail's dead parts are out: he dies, then they do,
    /// each as if those who hurt it had killed it (Cosmic's `revive` with
    /// `isHorntailDefeated`).
    fn horntail_defeated(&mut self, now: Instant) {
        let dead_parts = net::mobs::DEAD_HORNTAIL_PARTS.rev();
        let doomed: Vec<i32> = std::iter::once(net::mobs::HORNTAIL)
            .chain(dead_parts)
            .flat_map(|mob_id| self.out(mob_id))
            .collect();
        for object_id in doomed {
            if let Some(kill) = self.mobs.slay(object_id, now) {
                self.killed(&kill);
            }
        }
    }

    /// The mobs the death of a `mob_id` left unshielded become real: their
    /// clients see them just there (Cosmic `makeMonsterReal`), and the
    /// first player here moves them.
    fn make_real(&mut self, mob_id: i32) {
        let real = self.mobs.unshielded_by(mob_id);
        if real.is_empty() {
            return;
        }
        for &object_id in &real {
            if let Some(Ok(packet)) = self
                .mobs
                .get(object_id)
                .map(|mob| build_spawn_mob(&mob.view(false)))
            {
                self.broadcast(None, packet);
            }
        }
        if let Some(controller) = self.mob_candidate() {
            let controlled = self.mobs.assign_uncontrolled(controller);
            self.send_control(controller, &controlled);
        }
    }

    /// In a jail, what its way on makes of its levers, the draw made now if
    /// no one has tried it yet; `None` on any other map.
    fn check_jail(&mut self) -> Option<net::events::JailCheck> {
        let map_id = self.key.map_id;
        let levers = net::events::jail_levers(map_id)?;
        let mask = *self
            .jail
            .get_or_insert_with(|| net::events::draw_jail(map_id, &mut rand::thread_rng()));
        let up: Vec<bool> = (1..=levers)
            .map(|i| {
                let lever = self.reactors.named(&format!("lever{i}"));
                lever.is_some_and(|lever| lever.state > 0)
            })
            .collect();
        let check = net::events::check_jail(mask, &up);
        // Once open, the way stays open.
        if check == net::events::JailCheck::Solved {
            self.jail = Some(0);
        }
        Some(check)
    }

    /// Show the named map object `name` in `state` to everyone here and to
    /// those who come later (Cosmic `moveEnvironment`).
    fn move_obstacle(&mut self, name: &str, state: i32) {
        match self.obstacles.iter_mut().find(|(moved, _)| moved == name) {
            Some((_, moved)) => *moved = state,
            None => self.obstacles.push((name.to_string(), state)),
        }
        if let Ok(packet) = build_obstacle_state(name, state) {
            self.broadcast(None, packet);
        }
    }

    /// A client saw a trap hurt mob `object_id`. Only a map with traps that
    /// hurt mobs, some of them moved, takes it, no more than they can roll,
    /// and once a second for each mob, as each client tells of the same hit
    /// (0x667ce6).
    fn handle_field_damage_mob(&mut self, from: ClientId, object_id: i32, damage: i32) {
        let now = Instant::now();
        let most = net::get_game_data()
            .ok()
            .and_then(|data| data.field(self.key.map_id))
            .map_or(0, |field| field.traps.max_mob_hit());
        if self.obstacles.is_empty() || !(1..=most).contains(&damage) {
            return;
        }
        if self
            .trapped
            .get(&object_id)
            .is_some_and(|&at| now.duration_since(at) < TRAPPED_AGAIN)
        {
            return;
        }
        let Some(max_hp) = self.mobs.get(object_id).map(|mob| mob.info.max_hp) else {
            return;
        };
        if self.mobs.shielded(object_id) {
            return;
        }
        self.trapped.insert(object_id, now);
        // No client draws a trap's number itself, so all see it here, the
        // last one too.
        let show = |field: &Self, hp: i32| {
            if let Ok(packet) = build_damage_mob(object_id, damage, hp, max_hp) {
                field.broadcast(None, packet);
            }
        };
        match self.mobs.damage(from, object_id, i64::from(damage), now) {
            Some(Ok(_)) => {
                let Some(mob) = self.mobs.get(object_id) else {
                    return;
                };
                show(self, i32::try_from(mob.hp).unwrap_or(i32::MAX));
                let bar = self.mobs.bar_for(object_id);
                self.show_boss_bar(bar);
            }
            Some(Err(kill)) => {
                show(self, 0);
                self.trapped.remove(&object_id);
                self.killed(&kill);
            }
            None => {}
        }
    }

    /// `from`'s client says a mob's attack hurt a Dazzled mob: it takes the
    /// damage, if it may, as if `from` dealt it (as in Cosmic), and the
    /// others see the number `from` already showed.
    fn handle_mob_damage_mob(&mut self, from: ClientId, attacker: i32, victim: i32, damage: i32) {
        let now = Instant::now();
        if self.mobs.shielded(victim) {
            return;
        }
        let Some(damage) = self.mobs.mob_hit(attacker, victim, damage, now) else {
            return;
        };
        match self.mobs.damage(from, victim, damage, now) {
            Some(Ok(_)) => {
                let Some(mob) = self.mobs.get(victim) else {
                    return;
                };
                let hp = i32::try_from(mob.hp).unwrap_or(i32::MAX);
                let packet = build_damage_mob(victim, damage as i32, hp, mob.info.max_hp);
                let bar = self.mobs.bar_for(victim);
                if let Ok(packet) = packet {
                    self.broadcast(Some(from), packet);
                }
                if damage > 0 {
                    self.show_boss_bar(bar);
                }
            }
            Some(Err(kill)) => self.killed(&kill),
            None => {}
        }
    }

    /// `from`'s Meso Explosion sets off the mesos in its box that they may
    /// take, each going off for everyone in turn, and its targets take only
    /// what those could deal: each line no more than its drop's worth, a
    /// mob's `fixedDamage` for each that hits (the client sends them raw),
    /// and nothing from a drop it couldn't set off.
    fn set_off(&mut self, from: ClientId, blast: &Blast, hits: Vec<(i32, i64)>) -> Vec<(i32, i64)> {
        let Some(occupant) = self.occupants.get(&from) else {
            return Vec::new();
        };
        let (user, party) = ((occupant.avatar.x, occupant.avatar.y), occupant.party);
        let reaches = |x, y| meso_explosion::reaches(blast.lt, blast.rb, blast.left, user, (x, y));
        let now = Instant::now();
        let mut mesos = Vec::new();
        for (index, &object_id) in blast.drops.iter().enumerate() {
            let drop = self.drops.explode(from, party, object_id, reaches, now);
            if drop.is_some() {
                let delay = meso_explosion::delay(blast.delay, index);
                if let Ok(packet) = build_explode_drop(object_id, delay) {
                    self.send_drop_packet(packet);
                }
            }
            mesos.push(drop.and_then(|drop| match drop.loot {
                net::loot::Loot::Mesos(amount) => Some(amount),
                _ => None,
            }));
        }
        hits.into_iter()
            .zip(&blast.lines)
            .map(|((object_id, _), lines)| {
                let mob = self.mobs.get(object_id);
                let fixed = mob.map_or(0, |mob| mob.info.fixed_damage);
                // Each target counts its own lines, so its immunity caps
                // here, not at the attack's most.
                let immune = mob.is_some_and(|mob| mob.statuses.contains_key(&MobStat::PImmune));
                let damage: i64 = lines
                    .iter()
                    .map(|&(source, line)| match mesos[source] {
                        Some(amount) if line <= meso_explosion::line_cap(blast.x, amount) => {
                            if fixed > 0 && line > 0 {
                                i64::from(fixed)
                            } else {
                                i64::from(line)
                            }
                        }
                        _ => 0,
                    })
                    .sum();
                let damage = if immune {
                    damage.min(lines.len() as i64)
                } else {
                    damage
                };
                (object_id, damage)
            })
            .collect()
    }

    /// `from` used Monster Magnet. The others see it catch each live mob,
    /// or fail to; a mob caught (never a boss) chases `from`, whose client
    /// pulls it in.
    fn handle_magnet(&mut self, from: ClientId, effect: Packet, caught: Vec<(i32, bool)>) {
        let Some(user) = self.occupants.get(&from) else {
            return;
        };
        let user = user.handle.clone();
        let now = Instant::now();
        for (object_id, success) in caught {
            let Some(mob) = self.mobs.get(object_id) else {
                continue;
            };
            let success = success && !mob.info.boss;
            // The mob's, so everyone sees it pulled.
            if let Ok(packet) = build_show_magnet(object_id, success) {
                self.broadcast(Some(from), packet);
            }
            if success {
                self.provoke(from, &user, object_id, now);
            }
        }
        self.broadcast_by(from, effect);
    }

    /// `from` used a skill that reaches mobs on these, as their client saw
    /// them. Each in its area takes `inflict`, or loses the buffs the skill
    /// takes away, with the skill's chance (`prop`), and everyone sees.
    fn handle_mob_debuff(
        &mut self,
        from: ClientId,
        skill_id: i32,
        level: i32,
        origin: Option<(i16, i16)>,
        mobs: &[i32],
        inflict: &Inflict,
    ) {
        let Some(user) = self.occupants.get(&from) else {
            return;
        };
        let origin = origin.unwrap_or((user.avatar.x, user.avatar.y));
        let Some(skill) = skills::at_level(skill_id, level) else {
            return;
        };
        let cancels = skills::mob_cancels(skill_id);
        let effect = &skill.effect;
        let now = Instant::now();
        let mut reached: Vec<i32> = Vec::new();
        for &object_id in mobs {
            if reached.len() >= effect.mob_count.max(1) as usize || reached.contains(&object_id) {
                continue;
            }
            let Some(mob) = self.mobs.get(object_id) else {
                continue;
            };
            if mob.hp > 0 && skills::in_area(effect, origin, (mob.x, mob.y)) {
                reached.push(object_id);
            }
        }
        for object_id in reached {
            if !inflict.takes(rand::random()) {
                continue;
            }
            let lost = self.mobs.remove_statuses(object_id, cancels);
            if !lost.is_empty() {
                if let Ok(packet) = build_mob_stat_reset(object_id, &lost) {
                    self.broadcast(None, packet);
                }
            }
            self.put_on(from, object_id, inflict, now);
        }
    }

    /// `from`'s spell left a mist around them, which everyone sees.
    fn handle_mist(
        &mut self,
        from: ClientId,
        skill_id: i32,
        level: i32,
        left: bool,
        inflict: Inflict,
    ) {
        let Some(user) = self.occupants.get(&from) else {
            return;
        };
        let Some(skill) = skills::at_level(skill_id, level) else {
            return;
        };
        let view = MistView {
            object_id: 0,
            kind: skills::mist_kind(skill_id),
            owner_id: user.avatar.id,
            skill_id,
            level: level as u8,
            delay: MIST_DELAY,
            area: skills::area_box(&skill.effect, (user.avatar.x, user.avatar.y), left),
            element: 0,
        };
        let lasts = inflict.duration;
        let view = self.mists.add(view, from, inflict, lasts, Instant::now());
        if let Ok(packet) = build_spawn_mist(&view) {
            self.broadcast(None, packet);
        }
    }

    /// `from` used a cash mist item: its cloud covers `area` around them.
    /// Like the v83 client's own check (FUN_00431866), none is left while
    /// one of theirs from that item is there or they stand in any item's
    /// cloud; the item is spent either way.
    fn handle_item_mist(
        &mut self,
        from: ClientId,
        item_id: i32,
        area: (i32, i32, i32, i32),
        lasts: Duration,
    ) {
        let Some(user) = self.occupants.get(&from) else {
            return;
        };
        let (owner_id, x, y) = (user.avatar.id, user.avatar.x, user.avatar.y);
        // The client asks PtInRect, so the right and bottom edges are out.
        let (px, py) = (i32::from(x), i32::from(y));
        let busy = self.mists.all().any(|mist| {
            let (left, top, right, bottom) = mist.view.area;
            mist.view.kind == MistKind::Item
                && ((mist.view.owner_id == owner_id && mist.view.skill_id == item_id)
                    || (left <= px && px < right && top <= py && py < bottom))
        });
        if busy {
            return;
        }
        let (left, top, right, bottom) = area;
        let (x, y) = (i32::from(x), i32::from(y));
        let view = MistView {
            object_id: 0,
            kind: MistKind::Item,
            owner_id,
            skill_id: item_id,
            level: 1,
            delay: 0,
            area: (x + left, y + top, x + right, y + bottom),
            element: 0,
        };
        let inflict = Inflict {
            skill_id: item_id,
            prop: 0,
            duration: lasts,
            statuses: Vec::new(),
        };
        let view = self.mists.add(view, from, inflict, lasts, Instant::now());
        if let Ok(packet) = build_spawn_mist(&view) {
            self.broadcast(None, packet);
        }
        // The puff on its user, for everyone, and the user's face.
        if let Some(user) = self.occupants.get(&from) {
            for packet in [
                build_mist_item_effect(None, item_id),
                build_item_emotion(item_id),
            ]
            .into_iter()
            .flatten()
            {
                user.handle.send_packet(packet);
            }
        }
        if let Ok(packet) = build_mist_item_effect(Some(owner_id), item_id) {
            self.broadcast_by(from, packet);
        }
    }

    /// `from` called up a summon. It replaces their summon of that kind,
    /// which the others see dismissed.
    #[allow(clippy::too_many_arguments)]
    fn handle_summon(
        &mut self,
        from: ClientId,
        skill_id: i32,
        level: i32,
        at: (i16, i16),
        left: bool,
        lasts: Duration,
        appear: bool,
    ) {
        let Some(owner) = self.occupants.get(&from) else {
            return;
        };
        let (Some((moves, assist)), Some(skill)) =
            (summons::kind(skill_id), skills::at_level(skill_id, level))
        else {
            return;
        };
        let avatar = &owner.avatar;
        let near = (i32::from(at.0) - i32::from(avatar.x)).abs() <= SUMMON_REACH_X
            && (i32::from(at.1) - i32::from(avatar.y)).abs() <= SUMMON_REACH_Y;
        let (x, y) = if near { at } else { (avatar.x, avatar.y) };
        let view = SummonView {
            owner_id: avatar.id,
            object_id: 0,
            skill_id,
            owner_level: avatar.level,
            level: level as u8,
            x,
            y,
            stance: u8::from(left),
            moves,
            assist,
        };
        let hp = summons::hp(&skill);
        let (view, replaced) = self.summons.add(view, from, hp, lasts, Instant::now());
        if let Some(old) = replaced {
            self.remove_summon(&old.view, Gone::Dismissed);
        }
        if let Ok(packet) = build_spawn_summon(&view, appear) {
            self.broadcast_about(view.owner_id, None, packet);
        }
    }

    /// Bring out the mobs from `from`'s summoning sack on the ground below
    /// them, as Cosmic does (y - 1, the ground below that, less 1), facing
    /// their way and under their control.
    fn handle_summon_mobs(&mut self, from: ClientId, mobs: Vec<i32>) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let (x, y) = (occupant.avatar.x, occupant.avatar.y);
        let stance = 4 | (occupant.avatar.stance & 1);
        let ground = self.drops.ground_below(x, y.saturating_sub(1)).unwrap_or(y);
        let at = (x, ground.saturating_sub(1));
        let now = Instant::now();
        let mover = self.mover_for(from);
        let mut placed = Vec::new();
        for info in mobs.into_iter().filter_map(net::mobs::info) {
            let object_id = self.mobs.place(info, at, stance, mover, now);
            let Some(mob) = self.mobs.get(object_id) else {
                continue;
            };
            if let Ok(packet) = build_spawn_mob(&mob.view(true)) {
                self.broadcast(None, packet);
            }
            placed.push(object_id);
        }
        if let Some(mover) = mover {
            self.send_control(mover, &placed);
        }
    }

    /// Bring out the mob a script calls for on the ground below `(x, y)`
    /// (as a summoning sack places one), under `from`'s control, unless one
    /// is out already.
    fn handle_bring_out_mob(&mut self, from: ClientId, mob_id: i32, at: (i16, i16)) {
        if self.mobs.live().any(|mob| mob.info.mob_id == mob_id) {
            return;
        }
        self.bring_out_mobs(from, mob_id, &[at]);
    }

    /// Bring out a `mob_id` on the ground below each of `points`, for
    /// `from`'s client to control if nobody else does.
    fn bring_out_mobs(&mut self, from: ClientId, mob_id: i32, points: &[(i16, i16)]) {
        let Some(info) = net::mobs::info(mob_id) else {
            return;
        };
        let mover = self.mover_for(from);
        let now = Instant::now();
        let mut placed = Vec::new();
        for &(x, y) in points {
            let ground = self.drops.ground_below(x, y.saturating_sub(1)).unwrap_or(y);
            let at = (x, ground.saturating_sub(1));
            let object_id = self.mobs.place(info.clone(), at, 4, mover, now);
            let Some(mob) = self.mobs.get(object_id) else {
                continue;
            };
            if let Ok(packet) = build_spawn_mob(&mob.view(true)) {
                self.broadcast(None, packet);
            }
            placed.push(object_id);
        }
        if let Some(mover) = mover {
            self.send_control(mover, &placed);
        }
    }

    fn handle_unsummon(&mut self, from: ClientId, skill_id: Option<i32>, why: Gone) {
        for summon in self.summons.remove_owned(from, skill_id) {
            self.remove_summon(&summon.view, why);
        }
    }

    fn remove_summon(&self, view: &SummonView, why: Gone) {
        if let Ok(packet) = build_remove_summon(view.owner_id, view.object_id, why) {
            self.broadcast_about(view.owner_id, None, packet);
        }
    }

    fn handle_summon_move(
        &mut self,
        from: ClientId,
        object_id: i32,
        start: (i16, i16),
        movement: Vec<u8>,
    ) {
        let Some(summon) = self.summons.owned(from, object_id) else {
            return;
        };
        let view = &mut summon.view;
        if let Some((x, y, stance)) =
            parse_movement_state(&movement, (start.0, start.1, view.stance))
        {
            (view.x, view.y, view.stance) = (x, y, stance);
        }
        let owner_id = view.owner_id;
        if let Ok(packet) = build_move_summon(owner_id, object_id, start, &movement) {
            self.broadcast_by(from, packet);
        }
    }

    fn handle_pet_move(&mut self, from: ClientId, index: u8, start: (i16, i16), movement: Vec<u8>) {
        let Some(occupant) = self.occupants.get_mut(&from) else {
            return;
        };
        let owner_id = occupant.avatar.id;
        let Some(pet) = occupant.avatar.pets.get_mut(usize::from(index)) else {
            return;
        };
        if let Some((x, y, stance, foothold)) =
            parse_movement(&movement, (start.0, start.1, pet.stance))
        {
            (pet.x, pet.y, pet.stance) = (x, y, stance);
            pet.foothold = foothold.unwrap_or(pet.foothold);
        }
        if let Ok(packet) = build_move_pet(owner_id, index, start, &movement) {
            self.broadcast_by(from, packet);
        }
    }

    /// `from`'s summon attacked. An attack beyond its skill's reach or
    /// damage cap, or one that hits a mob twice, counts for nothing.
    fn handle_summon_attack(&mut self, from: ClientId, attack: SummonAttack, limits: &[Limit]) {
        let Some(summon) = self.summons.owned(from, attack.object_id) else {
            return;
        };
        let view = summon.view;
        let Some(limit) = limits.iter().find(|limit| limit.skill_id == view.skill_id) else {
            return;
        };
        let hits = &attack.hits;
        let mut mobs: Vec<i32> = hits.iter().map(|hit| hit.object_id).collect();
        mobs.sort_unstable();
        mobs.dedup();
        if hits.len() > limit.mob_count as usize
            || mobs.len() != hits.len()
            || hits
                .iter()
                .any(|hit| !(0..=limit.max_damage).contains(&hit.damage))
        {
            warn!(
                from,
                skill_id = view.skill_id,
                ?attack,
                ?limit,
                "Summon attack refused"
            );
            return;
        }
        let Ok(broadcast) = build_summon_attack(&view, attack.action, hits) else {
            return;
        };
        let hits = hits
            .iter()
            .map(|hit| (hit.object_id, i64::from(hit.damage)))
            .collect();
        // A v83 client counters only its own attacks.
        self.handle_attack(
            from,
            Some(broadcast),
            Some(view.skill_id),
            None,
            hits,
            limit.inflict.as_slice(),
            Takes::default(),
        );
    }

    /// `from`'s Power Guard hurt the mob that touched them as one of their
    /// hits would; everyone saw it in their DAMAGE_PLAYER.
    fn handle_reflect(&mut self, from: ClientId, object_id: i32, mob_id: i32, damage: i64) {
        if self
            .mobs
            .get(object_id)
            .is_some_and(|mob| mob.info.mob_id == mob_id)
        {
            let hit = vec![(object_id, damage)];
            self.handle_attack(from, None, None, None, hit, &[], Takes::default());
        }
    }

    /// A mob hurt `from`'s Puppet: the others see it, and it breaks at
    /// 0 HP, ending its owner's buff.
    fn handle_summon_damage(&mut self, from: ClientId, damage: SummonDamage) {
        let Some(summon) = self.summons.owned(from, damage.object_id) else {
            return;
        };
        let Some(hp) = summon.hp.as_mut() else {
            return;
        };
        *hp -= damage.damage;
        let (broken, view) = (*hp <= 0, summon.view);
        if let Ok(packet) = build_damage_summon(view.owner_id, &damage) {
            self.broadcast_by(from, packet);
        }
        if !broken {
            return;
        }
        self.summons.remove(view.object_id);
        self.remove_summon(&view, Gone::Broken);
        if let Some(owner) = self.occupants.get(&from) {
            owner.handle.send(ServerMessage::SummonBroken {
                skill_id: view.skill_id,
            });
        }
    }

    /// `from`'s Beholder heals or buffs them, if it may yet: the others see
    /// it act.
    fn handle_beholder(&mut self, from: ClientId, object_id: i32, action: u8, support: Support) {
        let Some(summon) = self.summons.owned(from, object_id) else {
            return;
        };
        if !summon.may_support(&support, Instant::now()) {
            debug!(object_id, ?support, "Beholder too soon");
            return;
        }
        let owner_id = summon.view.owner_id;
        if let Ok(packet) = build_summon_skill(owner_id, object_id, action) {
            self.broadcast_by(from, packet);
        }
        if let Some(owner) = self.occupants.get(&from) {
            owner.handle.send(ServerMessage::Beholder(support));
        }
    }

    /// Each mist due puts its statuses, with its chance, on the live mobs
    /// in it that lack them (OdinMS-family servers).
    fn reach_from_mists(&mut self, now: Instant) {
        let mut reached = Vec::new();
        for mist in self.mists.reaching(now) {
            for mob in self.mobs.live() {
                let lacks = mist
                    .inflict
                    .statuses
                    .iter()
                    .any(|(stat, _)| !mob.statuses.contains_key(stat));
                // A friendly mob takes nothing from players' mists.
                let friendly = mob.info.damaged_by_mob;
                if mob.hp > 0 && lacks && !friendly && mist.covers((mob.x, mob.y)) {
                    reached.push((mist.owner, mob.object_id, mist.inflict.clone()));
                }
            }
        }
        for (owner, object_id, inflict) in reached {
            if inflict.takes(rand::random()) {
                self.put_on(owner, object_id, &inflict, now);
            }
        }
    }

    /// A mob's buff takes hold with its chance on each mob it reaches,
    /// counters with `spread`.
    fn buff_mobs(&mut self, cast: &Cast, statuses: &[(MobStat, i16)], spread: i32, now: Instant) {
        let effect = &cast.effect;
        if rand::random::<f64>() * 100.0 >= f64::from(effect.prop) {
            return;
        }
        let until = now + Duration::from_secs(effect.duration.max(0) as u64);
        let skill_id = cast.skill | cast.level << 16;
        for object_id in self.mobs.reached_by(cast) {
            let Some(mob) = self.mobs.get(object_id) else {
                continue;
            };
            let statuses: Vec<_> = statuses
                .iter()
                .filter(|(stat, _)| {
                    other_immunity(*stat).is_none_or(|other| !mob.statuses.contains_key(&other))
                })
                .copied()
                .collect();
            let controller = mob.controller.unwrap_or_default();
            let given = self
                .mobs
                .add_statuses(object_id, &statuses, skill_id, controller, now, until);
            self.mobs.spread_counters(object_id, &given, spread);
            self.show_statuses(object_id, &given, now);
        }
    }

    /// A mob skill's animation ended: buff or heal the mobs it reaches, or
    /// call up its summons.
    fn apply_cast(&mut self, cast: &Cast, now: Instant) {
        let effect = &cast.effect;
        match mob_skill::effect(cast.skill, effect.x) {
            Some(MobSkillEffect::Statuses(statuses)) => {
                self.buff_mobs(cast, &statuses, 0, now);
            }
            Some(MobSkillEffect::Counters(stats)) => {
                let value = i16::try_from(effect.x).unwrap_or(i16::MAX);
                let statuses: Vec<_> = stats.iter().map(|&stat| (stat, value)).collect();
                self.buff_mobs(cast, &statuses, effect.y, now);
            }
            Some(MobSkillEffect::Heal) => {
                for object_id in self.mobs.reached_by(cast) {
                    let Some((healed, hp, max_hp)) = self.mobs.heal(object_id, effect.x, effect.y)
                    else {
                        continue;
                    };
                    if healed <= 0 {
                        continue;
                    }
                    if let Ok(packet) = build_heal_mob(object_id, healed, hp, max_hp) {
                        self.broadcast(None, packet);
                    }
                    self.show_boss_bar(self.mobs.bar_for(object_id));
                }
            }
            Some(MobSkillEffect::Summon) => {
                let summoned: Vec<_> = effect
                    .summons
                    .iter()
                    .filter_map(|&mob_id| net::mobs::info(mob_id))
                    .collect();
                let (area, drops) = (self.mobs.area(cast), &self.drops);
                let mut rng = rand::thread_rng();
                let spawned = self.mobs.summon(
                    cast.object_id,
                    &summoned,
                    effect.limit,
                    || summon_spot(drops, area.as_ref()?, &mut rng),
                    now,
                );
                let summon_effect = u8::try_from(effect.summon_effect).ok();
                for object_id in spawned {
                    let Some(mob) = self.mobs.get(object_id) else {
                        continue;
                    };
                    let view = MobView {
                        summon_effect,
                        ..mob.view(false)
                    };
                    let controller = mob.controller.and_then(|c| self.occupants.get(&c));
                    if let Ok(packet) = build_spawn_mob(&view) {
                        self.broadcast(None, packet);
                    }
                    if let (Some(occupant), Ok(packet)) =
                        (controller, build_control_mob(&view, false))
                    {
                        occupant.handle.send_packet(packet);
                    }
                }
            }
            Some(MobSkillEffect::Mist) => {
                if rand::random::<f64>() * 100.0 >= f64::from(effect.prop) {
                    return;
                }
                // It lays its mist over its area; each client in it hurts
                // itself (v83 FUN_00431783), so it puts nothing on mobs.
                let (Some(caster), Some(area)) =
                    (self.mobs.get(cast.object_id), self.mobs.area(cast))
                else {
                    return;
                };
                let view = MistView {
                    object_id: 0,
                    kind: MistKind::Mob,
                    owner_id: cast.object_id,
                    skill_id: cast.skill,
                    level: cast.level as u8,
                    delay: 0,
                    area: (area.left, area.top, area.right, area.bottom),
                    element: effect.element,
                };
                let inflict = Inflict {
                    skill_id: cast.skill,
                    prop: 100,
                    duration: Duration::from_secs(effect.duration.max(0) as u64),
                    statuses: Vec::new(),
                };
                let owner = caster.controller.unwrap_or_default();
                let lasts = inflict.duration;
                let view = self.mists.add(view, owner, inflict, lasts, now);
                if let Ok(packet) = build_spawn_mist(&view) {
                    self.broadcast(None, packet);
                }
            }
            Some(MobSkillEffect::Disease(stat)) => {
                if rand::random::<f64>() * 100.0 >= f64::from(effect.prop) {
                    return;
                }
                let Some(disease) =
                    mob_skill::inflicted(cast.skill, cast.level, effect.x, effect.duration)
                else {
                    return;
                };
                let most = diseased_most(stat, effect.count);
                let map_id = self.key.map_id;
                self.tell_reached(cast, |nth| {
                    (nth < most).then_some(ServerMessage::Diseased { disease, map_id })
                });
            }
            Some(MobSkillEffect::Dispel) => {
                if rand::random::<f64>() * 100.0 >= f64::from(effect.prop) {
                    return;
                }
                let map_id = self.key.map_id;
                self.tell_reached(cast, |_| Some(ServerMessage::Dispelled { map_id }));
            }
            Some(MobSkillEffect::Banish) => {
                if rand::random::<f64>() * 100.0 >= f64::from(effect.prop) {
                    return;
                }
                let Some(to) = self
                    .mobs
                    .get(cast.object_id)
                    .and_then(|mob| mob.info.banish.clone())
                else {
                    return;
                };
                // Up to `count` of them (inferred from the data: Cosmic
                // sends everyone in reach).
                let most = usize::try_from(effect.count)
                    .ok()
                    .filter(|&most| most > 0)
                    .unwrap_or(usize::MAX);
                let map_id = self.key.map_id;
                self.tell_reached(cast, |nth| {
                    (nth < most).then(|| ServerMessage::Banished {
                        to: to.clone(),
                        map_id,
                    })
                });
            }
            None => {}
        }
    }

    /// Show the ships leaving and docking, and move their passengers, as
    /// the trips through this map change (`net::conti`).
    fn run_trips(&mut self) {
        let Some(last) = self.trips else {
            return;
        };
        let now = conti::now();
        self.trips = Some(now);
        for happening in conti::happenings(self.key.map_id, last, now) {
            match happening {
                Happening::Show(target, state) => {
                    if let Ok(packet) = build_conti_move(target, state) {
                        self.broadcast(None, packet);
                    }
                }
                Happening::MoveAll { map_id, portal } => {
                    for occupant in self.occupants.values() {
                        occupant.handle.send(ServerMessage::ShipWarp {
                            from_map: self.key.map_id,
                            map_id,
                            portal,
                        });
                    }
                }
                Happening::Board {
                    mob_id,
                    count,
                    at: (x, y),
                } => {
                    self.board(mob_id, count, (x, y));
                }
                Happening::Retreat => {
                    if let Ok(packet) = build_conti_move(Target::Deck, State::MobDestroy) {
                        self.broadcast(None, packet);
                    }
                    let left = self.mobs.leave_all(Instant::now());
                    self.show_left(left);
                }
            }
        }
        // The elevator's door, from the clock: shut as the car leaves,
        // open as it docks.
        if let Some((name, state)) = conti::car_state(self.key.map_id, now) {
            for view in self.reactors.set_state(name, state) {
                if let Ok(packet) = build_hit_reactor(&view, 0, 0) {
                    self.broadcast(None, packet);
                }
            }
        }
    }

    /// Bring out NPC `npc_id` on the ground below (x, y), there until the
    /// field closes, and hand it to the NPCs' controller.
    fn appear(&mut self, npc_id: i32, (x, y): (i16, i16)) {
        let (ground, foothold) = self
            .drops
            .foothold_below(x, y.saturating_sub(10))
            .unwrap_or((y, 0));
        let object_id = self
            .map_npcs
            .iter()
            .map(|npc| npc.object_id + 1)
            .max()
            .unwrap_or(MAP_NPC_OBJECT_ID_BASE);
        let npc = FieldMapEntityNpc {
            object_id,
            npc_id,
            x,
            y: ground,
            flip: false,
            foothold,
            rx0: x.saturating_sub(50),
            rx1: x.saturating_add(50),
        };
        if let Ok(packet) = build_spawn_npc(&to_foreign_npc(&npc)) {
            self.broadcast(None, packet);
        }
        let controller = self.npc_controller.and_then(|id| self.occupants.get(&id));
        if let Some(occupant) = controller {
            if let Ok(packet) = build_control_npc(&to_foreign_npc(&npc)) {
                occupant.handle.send_packet(packet);
            }
        }
        self.map_npcs.push(npc);
    }

    /// Bring `count` of `mob_id` aboard on the ground below `(x, y)`, under
    /// the first player here, or the next to come; returns how many came.
    fn board(&mut self, mob_id: i32, count: usize, (x, y): (i16, i16)) -> usize {
        let ground = self.drops.ground_below(x, y.saturating_sub(1)).unwrap_or(y);
        let at = (x, ground.saturating_sub(1));
        let now = Instant::now();
        let mut came = 0;
        for info in std::iter::repeat_with(|| net::mobs::info(mob_id))
            .take(count)
            .flatten()
        {
            let object_id = self.mobs.place(info, at, 5, None, now);
            if let Some(mob) = self.mobs.get(object_id) {
                if let Ok(packet) = build_spawn_mob(&mob.view(true)) {
                    self.broadcast(None, packet);
                }
            }
            came += 1;
        }
        if let Some(controller) = self.mob_candidate() {
            let controlled = self.mobs.assign_uncontrolled(controller);
            self.send_control(controller, &controlled);
        }
        came
    }

    /// `summoner` spent CP on a `mob_id`, `index` of the field's list, for
    /// their side `side`: unless the side has `cap` of its summons out, it
    /// comes out at one of the side's `mobGenPos` spots, playing for it,
    /// and everyone hears who summoned it (Cosmic
    /// `MonsterCarnivalHandler`). Otherwise, why not.
    fn carnival_summon(
        &mut self,
        summoner: ClientId,
        index: u8,
        mob_id: i32,
        side: u8,
        cap: usize,
    ) -> Result<(), SpendRefusal> {
        let (Some(occupant), true, Some(info)) = (
            self.occupants.get(&summoner),
            self.carnival,
            net::mobs::info(mob_id),
        ) else {
            return Err(SpendRefusal::Failed);
        };
        if self.mobs.bought(side) >= cap {
            return Err(SpendRefusal::Limit);
        }
        let spots: Vec<_> = net::get_game_data()
            .ok()
            .and_then(|data| {
                Some(
                    data.field(self.key.map_id)?
                        .carnival
                        .as_ref()?
                        .mob_spots
                        .clone(),
                )
            })
            .unwrap_or_default()
            .into_iter()
            .filter(|spot| spot.team.is_none_or(|team| team == side))
            .collect();
        let spot = spots.get(rand::random::<usize>() % spots.len().max(1));
        let ((x, y), foothold) = spot.map_or(((occupant.avatar.x, occupant.avatar.y), 0), |spot| {
            ((spot.x, spot.y), spot.foothold)
        });
        let name = occupant.avatar.name.clone();
        let ground = self.drops.ground_below(x, y.saturating_sub(1)).unwrap_or(y);
        let at = (x, ground.saturating_sub(1));
        let now = Instant::now();
        let object_id = self.mobs.buy(info, at, foothold, side, now);
        self.guard_new(object_id, now);
        if let Some(mob) = self.mobs.get(object_id) {
            if let Ok(packet) = build_spawn_mob(&mob.view(true)) {
                self.broadcast(None, packet);
            }
        }
        if let Some(controller) = self.mob_candidate() {
            let controlled = self.mobs.assign_uncontrolled(controller);
            self.send_control(controller, &controlled);
        }
        if let Ok(packet) = build_carnival_summon(0, index, &name) {
            self.broadcast(None, packet);
        }
        Ok(())
    }

    /// `summoner` spent CP on `guardian`, `index` of the field's guardian
    /// list, for their side `side`: unless the side has `cap` standing or
    /// that one, it stands at a free `guardianGenPos` spot as the side's
    /// guardian reactor, the side's mobs get its mob skill's statuses, and
    /// everyone hears who placed it (Cosmic `MonsterCarnivalHandler`,
    /// `spawnGuardian`). Otherwise, why not.
    fn carnival_guard(
        &mut self,
        summoner: ClientId,
        index: u8,
        guardian: CarnivalSkill,
        side: u8,
        cap: usize,
    ) -> Result<(), SpendRefusal> {
        let level = net::mobs::skill_level(guardian.mob_skill, guardian.level);
        let statuses = match level.and_then(|level| mob_skill::effect(guardian.mob_skill, level.x))
        {
            Some(MobSkillEffect::Statuses(statuses)) => statuses,
            _ => Vec::new(),
        };
        let (Some(occupant), true, Some(reactor), false) = (
            self.occupants.get(&summoner),
            self.carnival,
            net::reactors::template(net::carnival::guardian_reactor(side)),
            statuses.is_empty(),
        ) else {
            return Err(SpendRefusal::Failed);
        };
        let spots = net::get_game_data()
            .ok()
            .and_then(|data| {
                Some(
                    data.field(self.key.map_id)?
                        .carnival
                        .as_ref()?
                        .guardian_spots
                        .clone(),
                )
            })
            .unwrap_or_default();
        let standing: Vec<_> = self
            .guardians
            .iter()
            .map(|guardian| (guardian.side, guardian.mob_skill, guardian.spot))
            .collect();
        let spot = net::carnival::guardian_spot(
            &spots,
            &standing,
            side,
            guardian.mob_skill,
            cap,
            rand::random(),
        )?;
        let name = occupant.avatar.name.clone();
        let at = (spots[spot].x, spots[spot].y);
        let object_id = self
            .reactors
            .place(reactor, at, net::carnival::GUARDIAN_STATE);
        if let Some(Ok(packet)) = self
            .reactors
            .get(object_id)
            .map(|r| build_spawn_reactor(&r.view()))
        {
            self.broadcast(None, packet);
        }
        self.guardians.push(Guardian {
            object_id,
            side,
            spot,
            mob_skill: guardian.mob_skill,
            skill_id: guardian.mob_skill | guardian.level << 16,
            statuses,
        });
        let now = Instant::now();
        let guarded: Vec<i32> = self
            .mobs
            .live()
            .filter(|mob| mob.team == Some(side) && mob.hp > 0)
            .map(|mob| mob.object_id)
            .collect();
        for object_id in guarded {
            let given = self.guard(object_id, self.guardians.len() - 1, now);
            self.show_statuses(object_id, &given, now);
        }
        if let Ok(packet) = build_carnival_summon(2, index, &name) {
            self.broadcast(None, packet);
        }
        Ok(())
    }

    /// Give a mob of its side the statuses of the `index`th guardian
    /// standing; returns those it got.
    fn guard(&mut self, object_id: i32, index: usize, now: Instant) -> Vec<MobStat> {
        let (Some(guardian), Some(mob)) = (self.guardians.get(index), self.mobs.get(object_id))
        else {
            return Vec::new();
        };
        if mob.team != Some(guardian.side) {
            return Vec::new();
        }
        let statuses: Vec<_> = guardian
            .statuses
            .iter()
            .filter(|(stat, _)| {
                other_immunity(*stat).is_none_or(|other| !mob.statuses.contains_key(&other))
            })
            .copied()
            .collect();
        let skill_id = guardian.skill_id;
        self.mobs.add_statuses(
            object_id,
            &statuses,
            skill_id,
            ClientId::default(),
            now,
            now + GUARDED_FOR,
        )
    }

    /// A mob that just came out has its side's guardians' statuses (Cosmic
    /// `spawnMonster`'s team buffs), which its spawn shows.
    fn guard_new(&mut self, object_id: i32, now: Instant) {
        for index in 0..self.guardians.len() {
            self.guard(object_id, index, now);
        }
    }

    /// A guardian broke: its spot is free again, and its side's mobs lose
    /// what it gave them (Cosmic `mcGuardian0`'s `dispelAllMonsters`). An
    /// immunity it held back from them is theirs again.
    fn guardian_broke(&mut self, object_id: i32) {
        let Some(at) = self
            .guardians
            .iter()
            .position(|guardian| guardian.object_id == object_id)
        else {
            return;
        };
        let guardian = self.guardians.remove(at);
        let stats: Vec<MobStat> = guardian.statuses.iter().map(|&(stat, _)| stat).collect();
        let held: Vec<i32> = self
            .mobs
            .live()
            .filter(|mob| {
                mob.team == Some(guardian.side)
                    && stats.iter().any(|stat| {
                        mob.statuses
                            .get(stat)
                            .is_some_and(|status| status.skill_id == guardian.skill_id)
                    })
            })
            .map(|mob| mob.object_id)
            .collect();
        let now = Instant::now();
        for object_id in held {
            let lost = self.mobs.remove_statuses(object_id, &stats);
            if let Ok(packet) = build_mob_stat_reset(object_id, &lost) {
                self.broadcast(None, packet);
            }
            let mut given = Vec::new();
            for index in 0..self.guardians.len() {
                given.extend(self.guard(object_id, index, now));
            }
            if !given.is_empty() {
                self.show_statuses(object_id, &given, now);
            }
        }
    }

    /// Zakum's altar took the Eye of Fire: its music plays, and its body
    /// comes out on the ground below `(x, y)`, trembling in, not yet real
    /// (no controller, and its arms shield it), then its arms with it.
    fn bring_out_zakum(&mut self, (x, y): (i16, i16)) {
        if let Ok(packet) = build_field_music(reactors::ZAKUM_MUSIC) {
            self.broadcast(None, packet);
        }
        let ground = self.drops.ground_below(x, y.saturating_sub(1)).unwrap_or(y);
        let at = (x, ground.saturating_sub(1));
        if let Some(info) = net::mobs::info(net::mobs::ZAKUM_BODY) {
            let object_id = self.mobs.place(info, at, 5, None, Instant::now());
            let view = self.mobs.get(object_id).map(|mob| MobView {
                fake: true,
                ..mob.view(false)
            });
            if let Some(Ok(packet)) = view.as_ref().map(build_spawn_mob) {
                self.broadcast(None, packet);
            }
        }
        for arm in net::mobs::ZAKUM_ARMS {
            self.board(arm, 1, (x, y));
        }
        let notice = net::content::text().get(keys::ZAKUM_ALTAR_SUMMONED, &[]);
        if let Ok(packet) = build_broadcast(Broadcast::Pink, &notice) {
            self.broadcast(None, packet);
        }
        for occupant in self.occupants.values() {
            occupant.handle.send(ServerMessage::BossSummoned);
        }
    }

    /// Horntail's stone broke: its music plays, and unless Horntail is
    /// already coming, it comes out on the ground below `HORNTAIL_AT`: the
    /// intro, then its body and eight parts out of it, sharing the body's
    /// HP, and the expedition's clock starts over (through whoever broke
    /// the stone, or someone still here). Then everyone here reads that it
    /// comes.
    fn bring_out_horntail(&mut self, by: ClientId) {
        use net::mobs::{HORNTAIL, HORNTAIL_AT, HORNTAIL_INTRO, HORNTAIL_PARTS};
        if let Ok(packet) = build_field_music(reactors::HORNTAIL_MUSIC) {
            self.broadcast(None, packet);
        }
        let coming = self
            .mobs
            .live()
            .any(|mob| mob.info.mob_id == HORNTAIL_INTRO);
        // The nine come out here, so the summoning, which one hit kills,
        // lets out nothing more (its `revive` lists them all).
        let intro = net::mobs::info(HORNTAIL_INTRO)
            .filter(|_| !coming)
            .map(|info| game_data::mobs::MobInfo {
                revive: Vec::new(),
                ..info
            });
        let body = net::mobs::info(HORNTAIL);
        if let (Some(intro), Some(body)) = (intro, body) {
            let (x, y) = HORNTAIL_AT;
            let ground = self.drops.ground_below(x, y.saturating_sub(1)).unwrap_or(y);
            let at = (x, ground.saturating_sub(1));
            let now = Instant::now();
            let intro = self.mobs.place(intro, at, 5, None, now);
            if let Some(Ok(packet)) = self
                .mobs
                .get(intro)
                .map(|mob| build_spawn_mob(&mob.view(true)))
            {
                self.broadcast(None, packet);
            }
            let body = self.mobs.place(body, at, 5, None, now);
            let parts: Vec<i32> = HORNTAIL_PARTS
                .filter_map(net::mobs::info)
                .map(|part| self.mobs.place(part, at, 5, None, now))
                .collect();
            self.mobs.link(body, &parts);
            // As Cosmic's updateBossSpawn, his bar shows as he comes out.
            self.show_boss_bar(self.mobs.bar_for(body));
            for object_id in std::iter::once(body).chain(parts.iter().copied()) {
                let view = self.mobs.get(object_id).map(|mob| MobView {
                    parent: Some(intro),
                    ..mob.view(false)
                });
                if let Some(Ok(packet)) = view.as_ref().map(build_spawn_mob) {
                    self.broadcast(None, packet);
                }
            }
            if let Some(controller) = self.mob_candidate() {
                let controlled = self.mobs.assign_uncontrolled(controller);
                self.send_control(controller, &controlled);
            }
            let teller = self
                .occupants
                .get(&by)
                .or_else(|| self.occupants.values().next());
            if let Some(occupant) = teller.filter(|_| self.key.instance_id != 0) {
                occupant.handle.send(ServerMessage::RestartEventClock);
            }
        }
        let line = net::content::text().get(keys::HORNTAIL_SUMMONED, &[]);
        if let Ok(packet) = build_item_line(&line, 0) {
            self.broadcast(None, packet);
        }
    }

    /// Send what `message` gives for the `nth` player a mob's cast
    /// reaches: everyone in its area, or without one its controller
    /// (Cosmic's MobSkill.applyEffect).
    fn tell_reached(&self, cast: &Cast, message: impl Fn(usize) -> Option<ServerMessage>) {
        let Some(caster) = self.mobs.get(cast.object_id) else {
            return;
        };
        let reached: Vec<ClientId> = match self.mobs.area(cast) {
            Some(area) => self
                .occupants
                .iter()
                .filter(|(_, occupant)| area.contains((occupant.avatar.x, occupant.avatar.y)))
                .map(|(&client_id, _)| client_id)
                .collect(),
            None => caster.controller.into_iter().collect(),
        };
        for (nth, client_id) in reached.into_iter().enumerate() {
            let (Some(occupant), Some(message)) = (self.occupants.get(&client_id), message(nth))
            else {
                continue;
            };
            occupant.handle.send(message);
        }
    }

    /// Show everyone the statuses a mob was just given.
    fn show_statuses(&self, object_id: i32, given: &[MobStat], now: Instant) {
        let Some(mob) = self.mobs.get(object_id) else {
            return;
        };
        let views: Vec<_> = mob
            .status_views(now)
            .into_iter()
            .filter(|view| given.contains(&view.stat))
            .collect();
        if views.is_empty() {
            return;
        }
        if let Ok(packet) = build_mob_stat_set(object_id, &views) {
            self.broadcast(None, packet);
        }
    }

    /// Put `inflict`'s statuses on a mob for `from`, unless it is shielded,
    /// and show everyone the ones it got.
    fn put_on(&mut self, from: ClientId, object_id: i32, inflict: &Inflict, now: Instant) {
        let Some(mob) = self.mobs.get(object_id) else {
            return;
        };
        if self.mobs.shielded(object_id) {
            return;
        }
        let statuses: Vec<_> = inflict
            .statuses
            .iter()
            .map(|&(stat, amount)| (stat, amount.against(mob.info.max_hp)))
            .collect();
        if statuses.is_empty() {
            return;
        }
        let until = now + inflict.duration;
        let given =
            self.mobs
                .add_statuses(object_id, &statuses, inflict.skill_id, from, now, until);
        self.show_statuses(object_id, &given, now);
    }

    /// Give the mob `from` hit to its client with aggro, taking it from
    /// whoever moved it before.
    fn provoke(&mut self, from: ClientId, attacker: &ClientHandle, object_id: i32, now: Instant) {
        if self.occupants.get(&from).is_some_and(|o| o.avatar.hidden) {
            return;
        }
        let Some(previous) = self.mobs.provoke(from, object_id, now) else {
            return;
        };
        self.hand_over(object_id, previous, from, attacker, true);
    }

    /// `from` reported how far it is from a firstAttack mob; the closest
    /// player moves it.
    fn handle_mob_distance(&mut self, from: ClientId, object_id: i32, distance: i32) {
        let Some(occupant) = self.occupants.get(&from).filter(|o| !o.avatar.hidden) else {
            return;
        };
        let handle = occupant.handle.clone();
        let Some(previous) = self
            .mobs
            .report_distance(from, object_id, distance, Instant::now())
        else {
            return;
        };
        self.hand_over(object_id, previous, from, &handle, false);
    }

    /// Show the whole field a mob's boss HP bar as it now stands, as Kinoko
    /// does on every change (FIELD_EFFECT 5). Mobs whose bar the client
    /// draws itself (`HPgaugeHide`) have none here.
    fn show_boss_bar(&self, bar: Option<BossBar>) {
        let Some(bar) = bar else {
            return;
        };
        if let Ok(packet) = build_boss_hp_bar(bar.mob_id, bar.hp, bar.max_hp, bar.colors) {
            self.broadcast(None, packet);
        }
    }

    /// A client says mob `attacker` touched the friendly mob `victim`:
    /// everyone sees what it lost and its HP. The Moon Bunny's guards hear
    /// of every sixth hit and of its going home, and its going in an
    /// event's copy loses its party the event (Cosmic's script); so does
    /// Willi's on Save Delli!, at once and without a word (Cosmic
    /// DelliBattle's `friendlyKilled`).
    fn handle_friendly_hit(&mut self, attacker: i32, victim: i32) {
        let now = Instant::now();
        let mut rng = rand::thread_rng();
        let Some(hit) = self.mobs.friendly_hit(attacker, victim, now, &mut rng) else {
            return;
        };
        if let Ok(packet) = build_damage_mob(victim, hit.damage, hit.hp, hit.max_hp) {
            self.broadcast(None, packet);
        }
        let bunny = hit.mob_id == reactors::MOON_BUNNY;
        let text = net::content::text();
        if !hit.gone {
            if hit.mob_id == net::events::WATCH_HOG {
                self.watch_hog(hit.hp + hit.damage, hit.hp, now);
            }
            self.bunny_hits += u32::from(bunny);
            if self.bunny_hits >= BUNNY_SICK_HITS {
                self.bunny_hits = 0;
                self.item_line(&text.get(keys::HPQ_BUNNY_SICK, &[]));
            }
            return;
        }
        // It goes home rather than dying (Cosmic's `killFriendlies`).
        if let Ok(packet) = build_kill_mob(victim, 0) {
            self.broadcast(None, packet);
        }
        if bunny {
            self.item_line(&text.get(keys::HPQ_BUNNY_GONE, &[]));
        }
        // Tylus's going only takes his Trust with him (Cosmic).
        if hit.mob_id == net::events::TYLUS_MOB {
            self.item_line(&text.get(keys::TYLUS_FALLEN, &[]));
        }
        if bunny && self.key.instance_id != 0 {
            self.lost_at = Some(now + net::events::scaled(FRIENDLY_LOST_AFTER));
        }
        // The Magatia escort's fainting only costs the run some of its
        // EXP and gifts (Cosmic `friendlyKilled`).
        let fainted =
            net::events::magatia_fainted(hit.mob_id).filter(|_| self.key.instance_id != 0);
        if let Some(key) = fainted {
            self.item_line(&text.get(key, &[]));
            self.escort_fainted = true;
            if let Some(occupant) = self.occupants.values().next() {
                occupant.handle.send(ServerMessage::EscortFailed);
            }
        }
        if net::events::protected(hit.mob_id) && self.key.instance_id != 0 {
            self.fail_event();
        }
        self.snowman_melted(hit.mob_id);
    }

    /// The Watchhog had `before` HP and now has `hp`: once ten seconds have
    /// passed since its guard last heard, they hear to protect it if it
    /// lost too much since.
    fn watch_hog(&mut self, before: i32, hp: i32, now: Instant) {
        let (since, was) = *self.hog_watch.get_or_insert((now, before));
        if now.duration_since(since) < HOG_WATCH {
            return;
        }
        if was - hp > HOG_HURT {
            self.item_line(&net::content::text().get(keys::KENTA_HOG_PROTECT, &[]));
        }
        self.hog_watch = Some((now, hp));
    }

    /// The Snow Spirit brings out a Holiday arena's snowman in its first
    /// form, and Scrooge's men start to come, as everyone hears (Cosmic
    /// `9105004`'s second page).
    fn bring_out_snowman(&mut self) {
        let Some(arena) = self
            .holiday
            .as_mut()
            .filter(|arena| arena.snowman.form == 0)
        else {
            return;
        };
        arena.snowman.form = 1;
        let snowman = net::events::holiday_snowman(arena.level, 1);
        self.board(snowman, 1, net::events::HOLIDAY_SNOWMAN_AT);
        self.mobs.open();
        self.pink_line(keys::HOLIDAY_SNOWMAN_OUT);
    }

    /// The Snow Spirit brings out Scrooge where the whole snowman stands,
    /// as everyone hears, and the snowman starts to heal every while
    /// (Cosmic `9105004`'s leader page, `snowmanHeal`).
    fn bring_out_scrooge(&mut self) {
        let Some(arena) = self
            .holiday
            .as_mut()
            .filter(|arena| arena.snowman.form == net::events::HOLIDAY_FORMS && !arena.scrooge)
        else {
            return;
        };
        arena.scrooge = true;
        arena.heals_at = Some(Instant::now() + net::events::HOLIDAY_SNOWMAN_HEAL_EVERY);
        let scrooge = net::events::holiday_scrooge(arena.level);
        self.pink_line(keys::HOLIDAY_SCROOGE_OUT);
        self.board(scrooge, 1, net::events::HOLIDAY_SNOWMAN_AT);
    }

    /// While Scrooge is out, the whole snowman heals 200 HP more than 200
    /// for each difficulty every 10 seconds (Cosmic `snowmanHeal`); it
    /// stops once Scrooge falls.
    fn heal_whole_snowman(&mut self, now: Instant) {
        let Some(level) = (self.holiday.as_ref())
            .filter(|arena| arena.heals_at.is_some_and(|at| at <= now))
            .map(|arena| arena.level)
        else {
            return;
        };
        let scrooge = !self.out(net::events::holiday_scrooge(level)).is_empty();
        if let Some(arena) = self.holiday.as_mut() {
            arena.heals_at = scrooge.then(|| now + net::events::HOLIDAY_SNOWMAN_HEAL_EVERY);
        }
        if !scrooge {
            return;
        }
        let snowman = net::events::holiday_snowman(level, net::events::HOLIDAY_FORMS);
        let hp = net::events::holiday_snowman_heal(level);
        for object_id in self.out(snowman) {
            if let Some((healed, hp, max_hp)) = self.mobs.heal(object_id, hp, 0) {
                if let Ok(packet) = build_heal_mob(object_id, healed, hp, max_hp) {
                    self.broadcast(None, packet);
                }
            }
        }
    }

    /// A mob killed in a Holiday arena while its snowman grows may leave
    /// Snow Vigor, real or fake, for anyone, where it fell (Cosmic
    /// `monsterKilled`).
    fn leave_snow_vigor(&mut self, kill: &Kill) {
        let Some(arena) = self.holiday.as_mut() else {
            return;
        };
        if !(1..net::events::HOLIDAY_FORMS).contains(&arena.snowman.form) {
            return;
        }
        let mut rng = rand::thread_rng();
        let (roll, real) = (rng.gen_range(0.0, 1.0), rng.gen_range(0.0, 1.0));
        let Some(item_id) = net::events::holiday_token(&mut arena.misses, roll, real) else {
            return;
        };
        let killer = kill.damage_by.first().map(|&(client, _)| client);
        let Some(owner) = killer.or_else(|| self.occupants.keys().next().copied()) else {
            return;
        };
        let loot = vec![Loot::Item {
            item_id,
            quantity: 1,
            quest: None,
        }];
        let (x, y) = (kill.x, kill.y);
        self.spawn_loot(owner, Ownership::FreeForAll, loot, x, y, kill.object_id);
    }

    /// Snow Vigor, real or fake, landing where a Holiday arena's snowman
    /// eats it, is eaten once it has lain there as long as an item on a
    /// reactor (Cosmic's collectors).
    fn put_on_snowman(&mut self, drop: &Drop, item_id: i32, quantity: i16, now: Instant) {
        let tokens = [net::events::SNOW_VIGOR, net::events::FAKE_SNOW_VIGOR];
        if quantity != 1 || !tokens.contains(&item_id) {
            return;
        }
        let at = self.landing(drop);
        let Some(arena) = self.holiday.as_mut() else {
            return;
        };
        if net::events::holiday_feeds_at(at)
            && !arena.eating.iter().any(|&(_, d)| d == drop.object_id)
        {
            arena.eating.push((now + ITEM_TAKEN_AFTER, drop.object_id));
        }
    }

    /// The snowman eats the Snow Vigor that has lain long enough where it
    /// eats it, while it grows; it leaves what lies there before it is out
    /// or once it is whole (Cosmic's collectors, which close then).
    fn eat_snow_vigor(&mut self, now: Instant) {
        let Some(arena) = self.holiday.as_mut() else {
            return;
        };
        let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut arena.eating)
            .into_iter()
            .partition(|&(at, _)| at <= now);
        arena.eating = waiting;
        for (_, object_id) in due {
            let growing = self.holiday.as_ref().map(|arena| arena.snowman.form);
            if !growing.is_some_and(|form| (1..net::events::HOLIDAY_FORMS).contains(&form)) {
                continue;
            }
            let Some(drop) = self.drops.remove(object_id) else {
                continue;
            };
            if let Ok(packet) = build_remove_drop(object_id, None) {
                self.send_drop_packet(packet);
            }
            let fake = reactor_item(&drop.loot)
                .is_some_and(|(item_id, _)| item_id == net::events::FAKE_SNOW_VIGOR);
            self.feed_snowman(fake);
        }
    }

    /// A Holiday arena's snowman eats Snow Vigor. A fake takes it a step
    /// back, as everyone hears; a real one heals it, or grows it into its
    /// next form, and the last form clears the stage: every other mob goes
    /// and no more come (Cosmic `snowmanSnack`, `snowmanSnackFake`,
    /// `snowmanEvolve`).
    fn feed_snowman(&mut self, fake: bool) {
        let Some(arena) = self.holiday.as_mut() else {
            return;
        };
        let level = arena.level;
        let was = net::events::holiday_snowman(level, arena.snowman.form);
        if fake {
            if arena.snowman.fake() {
                self.pink_line(keys::HOLIDAY_FAKE_VIGOR);
            }
            return;
        }
        match arena.snowman.feed(level) {
            None => {}
            Some(net::events::Feeding::Healed(hp)) => {
                for object_id in self.out(was) {
                    if let Some((healed, hp, max_hp)) = self.mobs.heal(object_id, hp, 0) {
                        if let Ok(packet) = build_heal_mob(object_id, healed, hp, max_hp) {
                            self.broadcast(None, packet);
                        }
                    }
                }
            }
            Some(net::events::Feeding::Grew(form)) => {
                // The old form vanishes as the new comes out in its place
                // (Cosmic `killMonster(.., 2)`, a leave the client doesn't
                // play).
                let now = Instant::now();
                for object_id in self.out(was) {
                    self.mobs.take_away_one(object_id, now);
                    if let Ok(packet) = build_kill_mob(object_id, 0) {
                        self.broadcast(None, packet);
                    }
                }
                let grown = net::events::holiday_snowman(level, form);
                self.board(grown, 1, net::events::HOLIDAY_SNOWMAN_AT);
                if form < net::events::HOLIDAY_FORMS {
                    return;
                }
                self.clear_mobs();
                let stage = net::events::HOLIDAY_SNOWMAN_STAGE;
                for occupant in self.occupants.values() {
                    occupant.handle.send(ServerMessage::ClearEventStage(stage));
                }
            }
        }
    }

    /// A Holiday arena's snowman, `mob_id`, melted under Scrooge's men:
    /// its form before comes back, as everyone hears, or, from its first
    /// form or its whole one, the run is lost (QUESTIONS b297).
    fn snowman_melted(&mut self, mob_id: i32) {
        let Some(arena) = self.holiday.as_mut() else {
            return;
        };
        if net::events::holiday_snowman(arena.level, arena.snowman.form) != mob_id {
            return;
        }
        let level = arena.level;
        match arena.snowman.melted() {
            Some(form) => {
                let snowman = net::events::holiday_snowman(level, form);
                self.board(snowman, 1, net::events::HOLIDAY_SNOWMAN_AT);
                self.pink_line(keys::HOLIDAY_SNOWMAN_MELTED);
            }
            None => {
                // The run is lost: no other snowman comes out here.
                self.holiday = None;
                self.fail_event();
            }
        }
    }

    /// A pink line, text `key`, for everyone here.
    fn pink_line(&self, key: &str) {
        let line = net::content::text().get(key, &[]);
        if let Ok(packet) = build_broadcast(Broadcast::Pink, &line) {
            self.broadcast(None, packet);
        }
    }

    /// A Magatia beaker filled for the first time (Cosmic `2618000`'s
    /// `hit`): with the third, the stage clears, every mob goes, and the
    /// room's door opens. A beaker that drains and fills again counts once.
    fn fill_beaker(&mut self, object_id: i32) {
        let side = net::events::MagatiaSide::of(self.key.map_id)
            .filter(|side| self.key.instance_id != 0 && self.key.map_id == side.stages()[2]);
        let Some(side) = side else {
            return;
        };
        if !self.beakers.insert(object_id) || self.beakers.len() != net::events::MAGATIA_BEAKERS {
            return;
        }
        for occupant in self.occupants.values() {
            occupant.handle.send(ServerMessage::ClearEventStage(
                net::events::MAGATIA_BEAKER_STAGE,
            ));
        }
        self.clear_mobs();
        for view in self.reactors.force_forward(side.beaker_door(), 1) {
            if let Ok(packet) = build_hit_reactor(&view, 0, 0) {
                self.broadcast(None, packet);
            }
        }
    }

    /// The reactors named `name` here open, shown to everyone, and shut
    /// again when their open state times out (Cosmic `hitReactor` on a
    /// reactor whose first state has no events).
    fn open_reactor(&mut self, name: &str) {
        for view in self.reactors.open(name, Instant::now()) {
            if let Ok(packet) = build_hit_reactor(&view, 0, 0) {
                self.broadcast(None, packet);
            }
        }
    }

    /// A Magatia hub's side door, `object_id`, opened with a Card Key: the
    /// party's copy of the side room behind it opens its way back too.
    fn open_side_room(&mut self, by: ClientId, object_id: i32) {
        let side = net::events::MagatiaSide::of(self.key.map_id)
            .filter(|side| self.key.instance_id != 0 && self.key.map_id == side.stages()[3]);
        let (Some(side), Some(name)) = (side, self.reactors.name(object_id)) else {
            return;
        };
        let Some(&(_, room, exit)) = side.side_doors().iter().find(|(door, ..)| *door == name)
        else {
            return;
        };
        let occupant = self
            .occupants
            .get(&by)
            .or_else(|| self.occupants.values().next());
        if let Some(occupant) = occupant {
            occupant.handle.send(ServerMessage::OpenEventReactor {
                map_id: room,
                name: exit,
            });
        }
    }

    /// Every mob here that isn't friendly goes, as if it left, and no more
    /// come: the event this copy is of was won.
    fn clear_mobs(&mut self) {
        for object_id in self.mobs.clear_hostile() {
            if let Ok(packet) = build_kill_mob(object_id, 1) {
                self.broadcast(None, packet);
            }
        }
    }

    /// The mobs here that aren't friendly go, as if they left, and `only`
    /// these come in their place, from the next spawn pass, or none.
    fn swap_mobs(&mut self, only: Option<i32>) {
        for object_id in self.mobs.swap_hostile(only) {
            if let Ok(packet) = build_kill_mob(object_id, 1) {
                self.broadcast(None, packet);
            }
        }
    }

    /// The friendly mobs due to drop what they make do, on the ground
    /// around them, for the party of a player here (Cosmic's rule), and the
    /// Moon Bunny's guards hear how many rice cakes it has made.
    fn friendly_drops(&mut self, now: Instant) {
        for (object_id, mob_id, x, y) in self.mobs.friendly_drops(now) {
            let Some((&owner, occupant)) = self.occupants.iter().next() else {
                continue;
            };
            let ownership = occupant.party.map_or(Ownership::Owner, Ownership::Party);
            let loot = net::loot::roll_friendly(mob_id, &mut rand::thread_rng());
            self.spawn_loot(owner, ownership, loot, x, y, object_id);
            if mob_id == reactors::MOON_BUNNY {
                self.bunny_cakes += 1;
                let count = self.bunny_cakes.to_string();
                let line = net::content::text().get(keys::HPQ_BUNNY_CAKE, &[("count", &count)]);
                self.item_line(&line);
            }
        }
    }

    /// The party lost the friendly mob it protected: everyone here hears
    /// so, and their event ends, which sends them off.
    fn lose_event(&mut self) {
        let line = net::content::text().get(keys::HPQ_BUNNY_LOST, &[]);
        if let Ok(packet) = build_broadcast(Broadcast::Pink, &line) {
            self.broadcast(None, packet);
        }
        self.fail_event();
    }

    /// Everyone here is told their event failed, which ends it.
    fn fail_event(&self) {
        let id = self.key.instance_id;
        for occupant in self.occupants.values() {
            occupant.handle.send(ServerMessage::EventFailed { id });
        }
    }

    /// A light-blue line for everyone here (BROADCAST_MSG 6, no item).
    fn item_line(&self, line: &str) {
        if let Ok(packet) = build_item_line(line, 0) {
            self.broadcast(None, packet);
        }
    }

    /// A client says a Darkstar has a player in reach: it blows up, giving
    /// nothing, as in Cosmic, with leave type 4 so every client plays its
    /// explosion and the players it reaches get hit.
    fn handle_mob_bomb(&mut self, object_id: i32) {
        if !self.mobs.bomb(object_id, Instant::now()) {
            return;
        }
        if let Ok(packet) = build_kill_mob(object_id, 4) {
            self.broadcast(None, packet);
        }
    }

    /// A player threw the catch item `item_id` from USE `slot` at a mob:
    /// everyone sees the mob caught, and the player's client actor trades
    /// the item for what it makes, or the player hears the mob is too
    /// strong.
    fn handle_catch(
        &mut self,
        from: ClientId,
        object_id: i32,
        slot: i16,
        item_id: i32,
        catch: &CatchInfo,
    ) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let handle = occupant.handle.clone();
        let (x, y) = (i32::from(occupant.avatar.x), i32::from(occupant.avatar.y));
        let (left, top, right, bottom) = catch.area;
        let reach = CastArea {
            left: x + i32::from(left) - CATCH_SLACK,
            top: y + i32::from(top) - CATCH_SLACK,
            right: x + i32::from(right) + CATCH_SLACK,
            bottom: y + i32::from(bottom) + CATCH_SLACK,
        };
        match self.mobs.catch(
            object_id,
            catch.mob,
            reach,
            catch.mob_hp,
            catch.prop,
            Instant::now(),
        ) {
            Catch::Caught => {
                if let Ok(packet) = build_catch_with_item(object_id, item_id, true) {
                    self.broadcast(None, packet);
                }
                if let Ok(packet) = build_kill_mob(object_id, 1) {
                    self.broadcast(None, packet);
                }
                handle.send(ServerMessage::Caught {
                    slot,
                    item_id,
                    create: catch.create,
                });
                // A stage cleared by catching its mobs clears with the last.
                let cleared = net::events::cleared_by_catching(self.key.map_id)
                    .filter(|_| self.key.instance_id != 0 && self.mobs.all_gone());
                if let Some(stage) = cleared {
                    for occupant in self.occupants.values() {
                        occupant.handle.send(ServerMessage::ClearEventStage(stage));
                    }
                }
                return;
            }
            Catch::TooStrong => {
                if let Ok(packet) = build_catch_fail(CatchFail::TooStrong, item_id) {
                    handle.send_packet(packet);
                }
            }
            // The map sees it fail, and the item is kept.
            Catch::Escaped => {
                if let Ok(packet) = build_catch_with_item(object_id, item_id, false) {
                    self.broadcast(None, packet);
                }
            }
            Catch::Missed => {}
        }
        if let Ok(packet) = net::packet::build::world::stat::build_stat_update(true, &[]) {
            handle.send_packet(packet);
        }
    }

    /// Tell `previous` it no longer moves the mob, and `to` that it does.
    fn hand_over(
        &self,
        object_id: i32,
        previous: Option<ClientId>,
        to: ClientId,
        handle: &ClientHandle,
        aggro: bool,
    ) {
        if let Some(previous) = previous.filter(|&previous| previous != to) {
            if let (Some(occupant), Ok(packet)) = (
                self.occupants.get(&previous),
                build_stop_controlling_mob(object_id),
            ) {
                occupant.handle.send_packet(packet);
            }
        }
        if let Some(mob) = self.mobs.get(object_id) {
            if let Ok(packet) = build_control_mob(&mob.view(false), aggro) {
                handle.send_packet(packet);
            }
        }
    }

    /// A player hit a reactor, or used `skill` by it: show its new state,
    /// or break it and have the player's client actor roll its drops.
    fn handle_hit_reactor(
        &mut self,
        from: ClientId,
        object_id: i32,
        flags: i32,
        delay: u16,
        skill: Option<i32>,
    ) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        // A side's own guardians stand up to it.
        let side = occupant.avatar.team;
        if self
            .guardians
            .iter()
            .any(|guardian| guardian.object_id == object_id && guardian.side == side)
        {
            return;
        }
        let position = (occupant.avatar.x, occupant.avatar.y);
        let now = Instant::now();
        let hit = match skill {
            Some(skill_id) => self.reactors.skill_used(object_id, skill_id, position, now),
            None => self.reactors.hit(object_id, flags, position, now),
        };
        if let Some(hit) = hit {
            self.show_reactor_hit(from, hit, delay);
        }
    }

    /// A player stepped onto a reactor set off by touch, or off it.
    fn handle_touch_reactor(&mut self, from: ClientId, object_id: i32, touching: bool) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let position = (occupant.avatar.x, occupant.avatar.y);
        if let Some(touch) = self.reactors.touch(object_id, from, touching, position) {
            self.reactor_touched(touch);
        }
    }

    /// Show a reactor moved as the first stepped onto it or the last off,
    /// and do what it does: a flame of the Keep's fourth stage flips its
    /// fires, and with enough on its stirges, they are gone.
    fn reactor_touched(&mut self, touch: Touch) {
        if let Some((view, event)) = &touch.moved {
            if let Ok(packet) = build_hit_reactor(view, 0, *event) {
                self.broadcast(None, packet);
            }
            for &fire in net::events::cwk_flame(touch.reactor_id).unwrap_or_default() {
                let state = self.obstacles.iter().find(|(name, _)| name == fire);
                let state = net::events::cwk_flip(state.map(|&(_, state)| state));
                self.move_obstacle(fire, state);
            }
        }
        self.stirges_touched(touch);
    }

    /// With enough on the Keep's fourth stage's stirges at once, every
    /// stirge there is gone.
    fn stirges_touched(&mut self, touch: Touch) {
        if touch.reactor_id == net::events::CWK_STIRGE
            && touch.touching
            && !self.stirges_gone
            && self.reactors.standing_on(touch.reactor_id) >= net::events::CWK_STIRGE_STANDING
        {
            // Cosmic `mapMessage(6, ..)`, then `killAllMonsters`.
            self.stirges_gone = true;
            let notice = net::content::text().get(keys::CWK_STIRGES_GONE, &[]);
            if let Ok(packet) = build_item_line(&notice, 0) {
                self.broadcast(None, packet);
            }
            self.clear_mobs();
        }
    }

    /// Show a reactor's new state, or break it and do what it does: bring
    /// out its mob, or have the client actor of `by`, who broke it, roll
    /// its drops.
    fn show_reactor_hit(&mut self, by: ClientId, hit: Hit, delay: u16) {
        let view = match hit {
            Hit::Moved { view, event } => {
                // The striker's event hears of a statue before anyone sees
                // it struck, so a guess is in when the keeper is asked.
                if matches!(reactors::action(view.reactor_id), reactors::Action::Statue) {
                    let name = self.reactors.name(view.object_id);
                    let statue = name.and_then(net::guild_quest::statue_number);
                    if let (Some(statue), Some(occupant)) = (statue, self.occupants.get(&by)) {
                        occupant.handle.send(ServerMessage::StatueStruck(statue));
                    }
                }
                if let Ok(packet) = build_hit_reactor(&view, delay, event) {
                    self.broadcast(None, packet);
                }
                match reactors::action(view.reactor_id) {
                    reactors::Action::Bloom => self.bloom(),
                    reactors::Action::Sigil => {
                        let job = net::events::cwk_sigil(view.reactor_id);
                        // An altar takes its weapon a while after it lands,
                        // when the one who dropped it may have gone, so
                        // anyone still here tells the event.
                        let teller =
                            (self.occupants.get(&by)).or_else(|| self.occupants.values().next());
                        if let (Some(job), Some(occupant)) = (job, teller) {
                            occupant.handle.send(ServerMessage::SigilLit(job));
                        }
                        // The third stage's set its menhirs going.
                        if self.key.map_id == net::events::CWK_ENTRY + 200 {
                            for name in job.map_or(&[][..], net::events::cwk_menhirs) {
                                self.move_obstacle(name, 1);
                            }
                        }
                    }
                    reactors::Action::Spear => self.spears_set(by),
                    reactors::Action::KingGate => self.king_gate_opened(by),
                    reactors::Action::Lever => {
                        let name = self.reactors.name(view.object_id);
                        if let Some(n) = name.and_then(|name| name.strip_prefix("lever")) {
                            let trap = format!("trap{n}");
                            self.move_obstacle(&trap, 1);
                        }
                    }
                    // Cosmic `3008000`'s `hit`: everyone here, as it takes
                    // the item.
                    reactors::Action::ExpToAll(exp) => {
                        for occupant in self.occupants.values() {
                            occupant.handle.send(ServerMessage::GainExp(exp));
                        }
                    }
                    reactors::Action::Beaker if view.state == reactors::BEAKER_FULL => {
                        self.fill_beaker(view.object_id)
                    }
                    // Cosmic `2708000`'s `hit`: `killMonster(8820000)`.
                    reactors::Action::Lightning => self.slay(net::mobs::PINK_BEAN_STATUE),
                    _ => {}
                }
                return;
            }
            // It stays, in its new state, and does what it does.
            Hit::Acted { view, event } => {
                if let Ok(packet) = build_hit_reactor(&view, delay, event) {
                    self.broadcast(None, packet);
                }
                view
            }
            // The client plays the hit animation of the state it broke from
            // as it removes the reactor.
            Hit::Broke(view) => {
                if let Ok(packet) = build_remove_reactor(&view) {
                    self.broadcast(None, packet);
                }
                view
            }
        };
        if let Some(notice) = reactors::breaker_notice(view.reactor_id) {
            let notice = net::content::text().get(notice, &[]);
            let packet = build_broadcast(Broadcast::Pink, &notice);
            if let (Some(occupant), Ok(packet)) = (self.occupants.get(&by), packet) {
                occupant.handle.send(ServerMessage::SendPacket(packet));
            }
        }
        match reactors::action(view.reactor_id) {
            reactors::Action::BringOut(mob_id, count) => {
                self.board(mob_id, count, (view.x, view.y.saturating_sub(10)));
            }
            reactors::Action::BringOutAt { mob_id, count, at } => {
                self.board(mob_id, count, at);
            }
            reactors::Action::BringOutAnyOf(mobs, count) => {
                let at = (view.x, view.y.saturating_sub(10));
                for _ in 0..count {
                    let mob_id = mobs[rand::thread_rng().gen_range(0, mobs.len())];
                    self.board(mob_id, 1, at);
                }
            }
            reactors::Action::BringOutLots(lots) => {
                for (mob_id, count) in lots {
                    self.board(mob_id, count, (view.x, view.y.saturating_sub(10)));
                }
            }
            // Cosmic `spawnMonster(..)` twice, then `mapMessage(5, ..)`.
            reactors::Action::MazeBox => {
                for (mob_id, count) in reactors::maze_box(view.reactor_id, self.key.map_id) {
                    self.board(mob_id, count, (view.x, view.y.saturating_sub(10)));
                }
                let notice = net::content::text().get(keys::LUDIMAZE_SUMMONED, &[]);
                if let Ok(packet) = build_broadcast(Broadcast::Pink, &notice) {
                    self.broadcast(None, packet);
                }
            }
            // Cosmic `mapMessage(5, ..)`, `changeMusic(..)`, then
            // `spawnMonster(.., x, y)`.
            reactors::Action::SpawnAt {
                mob_id,
                at,
                notice,
                music,
            } => {
                let notice = notice.map(|notice| net::content::text().get(notice, &[]));
                if let Some(Ok(packet)) = notice.map(|n| build_broadcast(Broadcast::Pink, &n)) {
                    self.broadcast(None, packet);
                }
                if let Some(Ok(packet)) = music.map(build_field_music) {
                    self.broadcast(None, packet);
                }
                self.board(mob_id, 1, at);
            }
            // Cosmic `changeMusic(..)`, `spawnMonster(..)`, then
            // `spawnMonster(.., x, y)` for each guard.
            reactors::Action::Summon {
                mob_id,
                guards,
                music,
            } => {
                if let Ok(packet) = build_field_music(music) {
                    self.broadcast(None, packet);
                }
                self.board(mob_id, 1, (view.x, view.y.saturating_sub(10)));
                for &(guard, at) in guards {
                    self.board(guard, 1, at);
                }
            }
            // Cosmic `mapMessage(5, ..)` then `spawnNpc(..)`.
            reactors::Action::Appear { npc_id, notice } => {
                let notice = net::content::text().get(notice, &[]);
                if let Ok(packet) = build_broadcast(Broadcast::Pink, &notice) {
                    self.broadcast(None, packet);
                }
                self.appear(npc_id, (view.x, view.y));
            }
            // Everyone here is told, then goes (Cosmic `warpMap`).
            reactors::Action::WarpMap { notice, .. } => {
                let notice = net::content::text().get(notice, &[]);
                if let Ok(packet) = build_broadcast(Broadcast::Pink, &notice) {
                    self.broadcast(None, packet);
                }
                for occupant in self.occupants.values() {
                    occupant.handle.send(ServerMessage::ReactorBroken {
                        field: self.key,
                        reactor_id: view.reactor_id,
                        object_id: view.object_id,
                        x: view.x,
                        y: view.y,
                    });
                }
            }
            // Another Cellion comes out at the next spot, until enough have
            // gone; then this spot drops its loot, and clears the stage, by
            // whoever broke it or someone still here (Cosmic `2001002`).
            reactors::Action::Salion => {
                let (limit, gone) = *self
                    .salion
                    .get_or_insert_with(|| (reactors::salion_limit(&mut rand::thread_rng()), 0));
                if gone < limit {
                    self.salion = Some((limit, gone + 1));
                    if let Some(at) = self.reactors.spot_of(reactors::salion_next(gone + 1)) {
                        self.board(reactors::CELLION, 1, at);
                    }
                } else if let Some(occupant) = self
                    .occupants
                    .get(&by)
                    .or_else(|| self.occupants.values().next())
                {
                    occupant.handle.send(ServerMessage::ReactorBroken {
                        field: self.key,
                        reactor_id: view.reactor_id,
                        object_id: view.object_id,
                        x: view.x,
                        y: view.y,
                    });
                }
            }
            reactors::Action::Guardian => self.guardian_broke(view.object_id),
            // A Nependeath grows out of it, or a Royal (Cosmic `2001000`).
            reactors::Action::Pot => {
                if !self.papa_pixie {
                    self.pots += 1;
                    let mob_id = reactors::pot_mob(self.pots, &mut rand::thread_rng());
                    self.board(mob_id, 1, (view.x, view.y.saturating_sub(10)));
                }
            }
            // Cosmic `killAllMonsters`, `allowSummonState(false)`, then
            // `spawnMonster` and `mapMessage(5, ..)`.
            reactors::Action::PapaPixie => {
                let left = self.mobs.leave_all(Instant::now());
                self.show_left(left);
                self.papa_pixie = true;
                self.board(reactors::PAPA_PIXIE, 1, reactors::PAPA_PIXIE_AT);
                let notice = net::content::text().get(keys::OPQ_NOTICE_PAPA_PIXIE, &[]);
                if let Ok(packet) = build_broadcast(Broadcast::Pink, &notice) {
                    self.broadcast(None, packet);
                }
            }
            // Cosmic `changeMusic`, `spawnFakeMonster` (the body first),
            // `spawnMonster` for each arm, then `mapMessage(5, ..)`.
            reactors::Action::Zakum => self.bring_out_zakum((view.x, view.y)),
            // Cosmic `changeMusic`, `spawnHorntailOnGroundBelow` and
            // `restartEventTimer` unless the intro is out, then
            // `mapMessage(6, ..)`.
            reactors::Action::Horntail => self.bring_out_horntail(by),
            // Cosmic `2618001`/`2618002`'s `act`: the way back out of the
            // side room it leads to opens too.
            reactors::Action::SideDoor => self.open_side_room(by, view.object_id),
            // Cosmic `2519000`'s `act`: `setAllowSpawnPointInBox(false,
            // ..)` around it, then the clear if the other seals are set and
            // no monster is left (as the last kill's, `killed`).
            reactors::Action::Seal {
                half: (across, down),
            } => {
                self.mobs.deny_box(
                    (view.x.saturating_sub(across), view.y.saturating_sub(down)),
                    (view.x.saturating_add(across), view.y.saturating_add(down)),
                );
                let cleared = net::events::cleared_by_killing(self.key.map_id).filter(|_| {
                    self.key.instance_id != 0 && self.mobs.live().next().is_none() && self.sealed()
                });
                if let Some(stage) = cleared {
                    for occupant in self.occupants.values() {
                        occupant.handle.send(ServerMessage::ClearEventStage(stage));
                    }
                }
            }
            // Whoever broke it drops its loot or goes where it sends them.
            // Whoever put the last item on it may have gone; the loot then
            // falls to someone still here. Minerva's statue brings her out
            // first (Cosmic `spawnNpc`), as does the treasure room's first
            // box; a Trial Grounds room's seal tells the room its key has
            // gone (Cosmic `mapMessage(6, ..)`).
            action => {
                match action {
                    reactors::Action::PassKey => {
                        let notice = net::content::text().get(keys::HTPQ_KEY_SENT, &[]);
                        if let Ok(packet) = build_broadcast(Broadcast::Pink, &notice) {
                            self.broadcast(None, packet);
                        }
                    }
                    reactors::Action::AppearAndClears(npc_id) => {
                        self.appear(npc_id, (view.x, view.y));
                    }
                    reactors::Action::SprayAndAppear { npc_id, at, .. }
                        if !self.map_npcs.iter().any(|npc| npc.npc_id == npc_id) =>
                    {
                        self.appear(npc_id, at);
                    }
                    _ => {}
                }
                let occupant = match action {
                    reactors::Action::Drops
                    | reactors::Action::Spray { .. }
                    | reactors::Action::SprayAndAppear { .. }
                    | reactors::Action::DropsAndClears
                    | reactors::Action::AppearAndClears(_)
                    | reactors::Action::PassKey => self
                        .occupants
                        .get(&by)
                        .or_else(|| self.occupants.values().next()),
                    _ => self.occupants.get(&by),
                };
                if let Some(occupant) = occupant {
                    occupant.handle.send(ServerMessage::ReactorBroken {
                        field: self.key,
                        reactor_id: view.reactor_id,
                        object_id: view.object_id,
                        x: view.x,
                        y: view.y,
                    });
                }
            }
        }
    }

    fn handle_join(&mut self, handle: ClientHandle, mut avatar: FieldAvatar, party: Option<i32>) {
        let client_id = handle.client_id;
        // A rejoin replaces the previous entry without announcing a leave.
        self.occupants.remove(&client_id);
        for touch in self.reactors.step_off(client_id) {
            self.reactor_touched(touch);
        }
        // On a Coconut Season field everyone plays, on the smaller team.
        if let Some(game) = &mut self.coconut {
            avatar.team = game.join(client_id);
        }
        // So on a Snowball field.
        if let Some(game) = &mut self.snowball {
            avatar.team = game.join(client_id);
        }
        // It also ends any trade or shop the player had here.
        let out = self.trades.leave(client_id);
        self.deliver_trade(out);
        let out = self.shops.leave(client_id);
        self.deliver_trade(out);
        let out = self.games.leave(client_id);
        self.deliver_trade(out);
        self.show_balloons();
        let out = self.merchants.leave(client_id, Instant::now());
        self.deliver_trade(out);
        self.show_merchants();
        // Its summons come back with it.
        for summon in self.summons.remove_owned(client_id, None) {
            self.remove_summon(&summon.view, Gone::Silently);
        }

        // Yulete waits in a run's copy of his office for the first to come
        // in, and leaves a little after (Cosmic `MagatiaPQ_Z.changedMap`).
        let map_id = self.key.map_id;
        let office = net::events::MagatiaSide::of(map_id)
            .filter(|side| side.stages()[4] == map_id && self.key.instance_id != 0);
        if let (Some(side), None) = (office, self.yulete) {
            self.appear(net::npc_scripts::yulete(side), net::events::YULETE_AT);
            let leaves = Instant::now() + net::events::YULETE_STAYS;
            self.yulete = Some((net::events::Yulete::Unmet, Some(leaves)));
        }
        // And in the boss's room, as do his traces and the one who thanks
        // the party in the rooms after it.
        let waiting = net::npc_scripts::magatia_waiting(map_id).filter(|&(npc_id, _)| {
            self.key.instance_id != 0 && !self.map_npcs.iter().any(|npc| npc.npc_id == npc_id)
        });
        if let Some((npc_id, at)) = waiting {
            self.appear(npc_id, at);
        }

        for map_npc in &self.map_npcs {
            match build_spawn_npc(&to_foreign_npc(map_npc)) {
                Ok(packet) => {
                    handle.send_packet(packet);
                }
                Err(error) => {
                    warn!(client_id, npc_id = map_npc.npc_id, %error, "Failed to build NPC spawn")
                }
            }
        }

        for occupant in self.occupants.values() {
            if !sees(&avatar, &occupant.avatar) {
                continue;
            }
            match player_spawn(&occupant.avatar) {
                Ok(packets) => {
                    for packet in packets {
                        handle.send_packet(packet);
                    }
                }
                Err(error) => warn!(client_id, %error, "Failed to build occupant spawn"),
            }
        }

        match player_spawn(&avatar) {
            Ok(packets) => {
                for packet in packets {
                    self.broadcast_seen(&avatar, packet);
                }
            }
            Err(error) => warn!(client_id, %error, "Failed to build join spawn"),
        }

        for mob in self.mobs.live() {
            if let Ok(packet) = build_spawn_mob(&mob.view(false)) {
                handle.send_packet(packet);
            }
        }

        for reactor in self.reactors.live() {
            if let Ok(packet) = build_spawn_reactor(&reactor.view()) {
                handle.send_packet(packet);
            }
        }

        for mist in self.mists.views() {
            if let Ok(packet) = build_spawn_mist(mist) {
                handle.send_packet(packet);
            }
        }

        for (&owner, &at) in &self.doors {
            if let Ok(packet) = build_spawn_door(owner, at, false) {
                handle.send_packet(packet);
            }
        }

        // After their owners, whose summons they are; a hidden GM's only
        // to GMs.
        for summon in self.summons.views().filter(|summon| {
            self.occupants
                .values()
                .find(|occupant| occupant.avatar.id == summon.owner_id)
                .is_none_or(|owner| sees(&avatar, &owner.avatar))
        }) {
            if let Ok(packet) = build_spawn_summon(summon, false) {
                handle.send_packet(packet);
            }
        }

        let now = Instant::now();
        for drop in self.drops.all() {
            if let Ok(packet) = build_drop(&drop.view(now), DropAppearance::Lying) {
                show_drop_to(&handle, drop, packet);
            }
        }

        if let Some((item_id, message, _)) = &self.weather {
            if let Ok(packet) = build_blow_weather(Some((*item_id, message))) {
                handle.send_packet(packet);
            }
        }

        if !self.obstacles.is_empty() {
            if let Ok(packet) = build_obstacle_states(&self.obstacles) {
                handle.send_packet(packet);
            }
        }

        for kite in self.kites.views() {
            if let Ok(packet) = build_spawn_kite(kite) {
                handle.send_packet(packet);
            }
        }

        if let Some(game) = &self.coconut {
            for packet in coconut_entry(game, client_id, now) {
                handle.send_packet(packet);
            }
        }
        if let Some(game) = &self.snowball {
            for packet in snowball_entry(game, now) {
                handle.send_packet(packet);
            }
        }
        if let Some(game) = &self.ox {
            for packet in ox_entry(game, now) {
                handle.send_packet(packet);
            }
        }
        if let Some(race) = &self.race {
            if let Ok(packet) = build_countdown(field_coconut::seconds(race.left(now))) {
                handle.send_packet(packet);
            }
        } else if net::gm_event::timed_event(self.key.map_id)
            .is_some_and(|event| event.maps[0] != self.key.map_id)
            && !avatar.gm
        {
            // The way past the lobby is only open while the event is on.
            // One who comes in after the time is up here, from a map whose
            // time came a tick later, goes out too.
            handle.send(ServerMessage::ShipWarp {
                from_map: self.key.map_id,
                map_id: net::gm_event::EXIT_MAP,
                portal: conti::Portal::Id(0),
            });
        }

        for packet in self.merchants.spawns() {
            match packet {
                Ok(packet) => {
                    handle.send_packet(packet);
                }
                Err(error) => warn!(client_id, %error, "Failed to build a merchant spawn"),
            }
        }

        let hidden = avatar.hidden;
        self.occupants.insert(
            client_id,
            Occupant {
                handle,
                avatar,
                party,
                foothold: 0,
            },
        );
        if !hidden {
            let controlled = self.mobs.assign_uncontrolled(client_id);
            self.send_control(client_id, &controlled);
        }
        // A rejoining controller has forgotten it controls them.
        if self.npc_controller.is_none() || self.npc_controller == Some(client_id) {
            self.send_npc_control(client_id);
        }
        if let Some(boss) = self.dojo_boss.take() {
            self.bring_out_dojo_boss(boss);
        }
        // The first into the Twilight of Gods' copy starts the guardians'
        // first wave's clock (Cosmic `PinkBeanBattle.afterSetup`).
        if self.key.map_id == net::events::PINK_BEAN_ARENA
            && self.key.instance_id != 0
            && self.guardians_wave.is_none()
        {
            self.guardians_wave = Some(1);
            self.guardians_stir(keys::PINK_BEAN_WAVE_FIRST);
        }
    }

    /// A Dojo floor's boss comes out where it may, drawn at random, in a
    /// puff of smoke (Cosmic `spawnDojoMonster`'s summon effect 15). Its
    /// data marks it a boss, but it fights as an ordinary mob.
    fn bring_out_dojo_boss(&mut self, mob_id: i32) {
        let spots = &net::events::DOJO_BOSS_AT;
        let (x, y) = spots[rand::thread_rng().gen_range(0, spots.len())];
        let Some(mut info) = net::mobs::info(mob_id) else {
            warn!(
                mob_id,
                map_id = self.key.map_id,
                "No Dojo boss to bring out"
            );
            return;
        };
        // Not a boss to skills and statuses (Cosmic `setBoss(false)`).
        info.boss = false;
        let ground = self.drops.ground_below(x, y.saturating_sub(1)).unwrap_or(y);
        let object_id =
            self.mobs
                .place(info, (x, ground.saturating_sub(1)), 5, None, Instant::now());
        let view = self.mobs.get(object_id).map(|mob| MobView {
            summon_effect: Some(net::events::DOJO_BOSS_EFFECT),
            ..mob.view(true)
        });
        if let Some(Ok(packet)) = view.as_ref().map(build_spawn_mob) {
            self.broadcast(None, packet);
        }
        if let Some(controller) = self.mob_candidate() {
            let controlled = self.mobs.assign_uncontrolled(controller);
            self.send_control(controller, &controlled);
        }
    }

    /// A player used a weather item: everyone here sees it, in place of any
    /// that was blowing, until it stops. A buff weather (5121xxx) also
    /// gives everyone here its item's effect (Cosmic `stateChangeItem`).
    fn blow_weather(&mut self, item_id: i32, message: String) {
        self.blow_weather_for(item_id, message, WEATHER_TIME);
        let buff = net::items::info(item_id).map_or(0, |info| info.state_change_item);
        if buff > 0 {
            for occupant in self.occupants.values() {
                occupant.handle.send(ServerMessage::FieldBuff(buff));
            }
        }
    }

    fn blow_weather_for(&mut self, item_id: i32, message: String, time: Duration) {
        match build_blow_weather(Some((item_id, &message))) {
            Ok(packet) => self.broadcast(None, packet),
            Err(error) => warn!(item_id, %error, "Failed to build weather"),
        }
        self.weather = Some((item_id, message, Instant::now() + time));
    }

    /// A spear was set here: the event of whoever set it, or of someone
    /// still here, hears how many are, and moves the knights' hall's gate.
    fn spears_set(&mut self, by: ClientId) {
        let set = (self.reactors.live())
            .filter(|spear| spear.reactor_id == net::guild_quest::SPEAR && spear.state > 0)
            .count();
        let set = u8::try_from(set).unwrap_or(u8::MAX);
        let occupant = (self.occupants.get(&by)).or_else(|| self.occupants.values().next());
        if let Some(occupant) = occupant {
            occupant.handle.send(ServerMessage::SpearsSet(set));
        }
    }

    /// The King's Corridor's door took the earring: the event of whoever
    /// set it, or of someone still here, hears of it.
    fn king_gate_opened(&self, by: ClientId) {
        let occupant = (self.occupants.get(&by)).or_else(|| self.occupants.values().next());
        if let Some(occupant) = occupant {
            occupant.handle.send(ServerMessage::KingGateOpened);
        }
    }

    /// A moonflower bloomed: the full moon waxes to as many phases as there
    /// are flowers in bloom, and as it turns full, the Moon Bunny comes out
    /// below it and everyone here is told to guard it.
    fn bloom(&mut self) {
        let flowers = self
            .reactors
            .live()
            .filter(|reactor| reactors::action(reactor.reactor_id) == reactors::Action::Bloom);
        let (mut bloomed, mut all) = (0u8, 0u8);
        for flower in flowers {
            all += 1;
            bloomed += u8::from(flower.state > 0);
        }
        let waxed = self.reactors.set_state(reactors::FULL_MOON, bloomed);
        for view in &waxed {
            if let Ok(packet) = build_hit_reactor(view, 0, 0) {
                self.broadcast(None, packet);
            }
        }
        let [moon] = waxed[..] else {
            return;
        };
        if bloomed < all {
            return;
        }
        if self.board(reactors::MOON_BUNNY, 1, (moon.x, moon.y)) == 0 {
            warn!(map_id = self.key.map_id, "No Moon Bunny to bring out");
            return;
        }
        let text = net::content::text();
        self.item_line(&text.get(keys::HPQ_MOON_BUNNY_OUT, &[]));
        let message = text.get(keys::HPQ_MOON_WEATHER, &[]);
        self.blow_weather_for(MOON_WEATHER, message, MOON_WEATHER_TIME);
    }

    /// A player flew a kite: it floats above where they stand, for everyone
    /// here and whoever comes, until its hour is up.
    fn fly_kite(&mut self, from: ClientId, item_id: i32, message: String) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let view = KiteView {
            object_id: 0,
            item_id,
            message,
            name: occupant.avatar.name.clone(),
            x: occupant.avatar.x,
            y: occupant.avatar.y,
        };
        let kite = self.kites.add(view, Instant::now());
        match build_spawn_kite(kite) {
            Ok(packet) => self.broadcast(None, packet),
            Err(error) => warn!(item_id, %error, "Failed to build kite"),
        }
    }

    /// Nut `index` takes `from`'s hit; everyone sees what it does, and a
    /// fall scores for the hitter's team.
    fn hit_coconut(&mut self, from: ClientId, index: u16) {
        let Some(game) = &mut self.coconut else {
            return;
        };
        let mut rng = rand::thread_rng();
        let Some(shot) = game.hit(from, index, Instant::now(), &mut || rng.gen()) else {
            return;
        };
        let [maple, story] = game.scores();
        if let Ok(packet) = build_coconut_hit(index as i16, field_coconut::HIT_DELAY, shot.state())
        {
            self.broadcast(None, packet);
        }
        let Shot::Fell { team } = shot else {
            return;
        };
        if let Ok(packet) = build_coconut_score(maple, story) {
            self.broadcast(None, packet);
        }
        let Some(name) = self.occupants.get(&from).map(|o| o.avatar.name.clone()) else {
            return;
        };
        let key = if team == field_coconut::MAPLE {
            keys::COCONUT_FELL_MAPLE
        } else {
            keys::COCONUT_FELL_STORY
        };
        let line = net::content::text().get(key, &[("name", &name)]);
        if let Ok(packet) = build_broadcast(Broadcast::Pink, &line) {
            self.broadcast(None, packet);
        }
    }

    /// A GM's `/start` or `/reset` for the event on this field.
    fn run_gm_event(&mut self, event: GmEvent) {
        if self.snowball.is_some() {
            return self.run_snowball_event(event);
        }
        if self.ox.is_some() {
            return self.run_ox_event(event);
        }
        let Some(game) = &mut self.coconut else {
            return;
        };
        let now = Instant::now();
        let packets = match event {
            GmEvent::Start => {
                let length =
                    net::events::scaled(Duration::from_secs(game.rules().time_default.into()));
                if !game.start(now, length) {
                    return;
                }
                info!(field = ?self.key, "Coconut game started");
                [
                    build_coconut_hit(-1, field_coconut::SHOW_DELAY, 0),
                    build_coconut_score(0, 0),
                    build_countdown(field_coconut::seconds(length)),
                ]
            }
            GmEvent::Reset => {
                if !game.reset() {
                    return;
                }
                info!(field = ?self.key, "Coconut game reset");
                [
                    build_coconut_hit(-1, 0, 0),
                    build_coconut_score(0, 0),
                    build_countdown(0),
                ]
            }
        };
        for packet in packets.into_iter().flatten() {
            self.broadcast(None, packet);
        }
    }

    /// The Coconut game's clock: overtime on a tie, the result at the end,
    /// and everyone sent off when it has been shown.
    fn tick_coconut(&mut self, now: Instant) {
        let Some(game) = &mut self.coconut else {
            return;
        };
        let rules = game.rules();
        let overtime = net::events::scaled(Duration::from_secs(rules.time_expand.into()));
        let finish = net::events::scaled(Duration::from_secs(rules.time_finish.into()));
        let Some(turn) = game.tick(now, overtime, finish) else {
            return;
        };
        let game = &*game;
        match turn {
            Turn::Overtime(length) => {
                let line = net::content::text().get(keys::COCONUT_OVERTIME, &[]);
                let packets = [
                    build_broadcast(Broadcast::Pink, &line),
                    build_countdown(field_coconut::seconds(length)),
                ];
                for packet in packets.into_iter().flatten() {
                    self.broadcast(None, packet);
                }
            }
            Turn::Ended { winner } => {
                let rules = game.rules();
                for (&client_id, occupant) in &self.occupants {
                    let won = winner.is_some() && game.team(client_id) == winner;
                    let (effect, sound) = if won {
                        (&rules.effect_win, &rules.sound_win)
                    } else {
                        (&rules.effect_lose, &rules.sound_lose)
                    };
                    for packet in [build_field_effect(effect), build_field_sound(sound)]
                        .into_iter()
                        .flatten()
                    {
                        occupant.handle.send_packet(packet);
                    }
                }
            }
            Turn::WarpOut { winner } => {
                for (&client_id, occupant) in &self.occupants {
                    let won = winner.is_some() && game.team(client_id) == winner;
                    occupant.handle.send(ServerMessage::ShipWarp {
                        from_map: self.key.map_id,
                        map_id: if won {
                            field_coconut::WINNERS_MAP
                        } else {
                            field_coconut::EXIT_MAP
                        },
                        portal: conti::Portal::Id(0),
                    });
                }
            }
        }
    }

    /// `from` hits `what` on the Snowball field; everyone sees the hit,
    /// and what it moved.
    fn hit_snowball(&mut self, from: ClientId, what: u8, delay: u16) {
        let Some(game) = &mut self.snowball else {
            return;
        };
        let mut rng = rand::thread_rng();
        let Some(hit) = game.hit(from, what, Instant::now(), &mut || rng.gen()) else {
            return;
        };
        // The state first: the client draws a snowman's HP bar from it as
        // the hit shows. A hit's delay is the hitter's swing, a moment.
        let mut packets = Vec::new();
        if hit.changed {
            packets.push(snowball_state(game));
        }
        packets.push(build_hit_snowball(
            hit.what,
            hit.damage,
            delay.min(field_snowball::MAX_HIT_DELAY),
        ));
        for (team, message) in hit.messages {
            packets.push(build_snowball_message(team, message));
        }
        for packet in packets.into_iter().flatten() {
            self.broadcast(None, packet);
        }
    }

    /// `from` walked into a ball, and is thrown back.
    fn knock_back(&mut self, from: ClientId) {
        let Some(game) = &mut self.snowball else {
            return;
        };
        if !game.knock(from, Instant::now()) {
            return;
        }
        if let (Some(occupant), Ok(packet)) = (self.occupants.get(&from), build_left_knock_back()) {
            occupant.handle.send_packet(packet);
        }
    }

    /// A GM's `/start` or `/reset` on the OX Quiz field: ten questions
    /// drawn from one set, or the game stopped (report b171).
    fn run_ox_event(&mut self, event: GmEvent) {
        let Some(game) = &mut self.ox else {
            return;
        };
        let packet = match event {
            GmEvent::Start => {
                let Some(quiz) = net::gm_event::ox_quiz() else {
                    return warn!(field = ?self.key, "No OX Quiz questions to ask");
                };
                let questions = field_ox::draw(quiz, &mut rand::thread_rng());
                if questions.is_empty() {
                    return warn!(field = ?self.key, "No OX Quiz set has enough questions");
                }
                let Some(first) = game.start(questions, Instant::now()) else {
                    return;
                };
                info!(field = ?self.key, set = first.set, "OX Quiz started");
                build_ox_quiz(true, first.set, first.number)
            }
            GmEvent::Reset => {
                let Some(set) = game.reset() else {
                    return;
                };
                info!(field = ?self.key, "OX Quiz reset");
                // The field has no way out but its exit's.
                for occupant in self.occupants.values().filter(|o| !o.avatar.gm) {
                    occupant.handle.send(ServerMessage::ShipWarp {
                        from_map: self.key.map_id,
                        map_id: field_ox::EXIT_MAP,
                        portal: conti::Portal::Id(0),
                    });
                }
                build_ox_quiz(false, set, 0)
            }
        };
        if let Ok(packet) = packet {
            self.broadcast(None, packet);
        }
    }

    /// The OX Quiz's time: the next question, its answer and who got it
    /// wrong, and those sent off; after the last, everyone left won.
    fn tick_ox(&mut self, now: Instant) {
        let Some(turn) = self.ox.as_mut().and_then(|game| game.tick(now)) else {
            return;
        };
        match turn {
            field_ox::Turn::Ask(question) => {
                if let Ok(packet) = build_ox_quiz(true, question.set, question.number) {
                    self.broadcast(None, packet);
                }
            }
            field_ox::Turn::Answer(question) => {
                let data = net::get_game_data().ok();
                let areas = data
                    .and_then(|data| data.field(self.key.map_id))
                    .map_or(&[][..], |field| field.areas.as_slice());
                let (right, wrong) = self
                    .occupants
                    .iter()
                    .filter(|(_, occupant)| !occupant.avatar.gm)
                    .map(|(&client_id, occupant)| {
                        let avatar = &occupant.avatar;
                        let given = field_ox::answer(areas, avatar.x, avatar.y, occupant.foothold);
                        (client_id, given == Some(question.o))
                    })
                    .partition::<Vec<_>, _>(|&(_, right)| right);
                let ids =
                    |judged: Vec<(ClientId, bool)>| judged.into_iter().map(|(id, _)| id).collect();
                if let Some(game) = &mut self.ox {
                    game.judged(ids(wrong), ids(right));
                }
                if let Ok(packet) = build_ox_quiz(false, question.set, question.number) {
                    self.broadcast(None, packet);
                }
            }
            field_ox::Turn::SendOff {
                set,
                wrong,
                right,
                last,
            } => self.send_off_ox(set, &wrong, &right, last),
        }
    }

    /// Those who answered wrong leave for the exit; on the last question
    /// those who answered right leave for the winners' map, and anyone who
    /// came in too late to answer it for the exit. A game nobody is left to
    /// play stops.
    fn send_off_ox(&mut self, set: u8, wrong: &[ClientId], right: &[ClientId], last: bool) {
        let wrong_line = net::content::text().get(keys::OX_WRONG, &[]);
        let won_line = net::content::text().get(keys::OX_WON, &[]);
        let clear = build_ox_quiz(false, set, 0).ok();
        let mut playing = false;
        for (client_id, occupant) in &self.occupants {
            let out = wrong.contains(client_id) || (last && !right.contains(client_id));
            if !out && !last {
                playing |= !occupant.avatar.gm;
                continue;
            }
            if let Some(packet) = &clear {
                occupant.handle.send_packet(packet.clone());
            }
            if occupant.avatar.gm {
                continue;
            }
            let (line, map_id) = if out {
                (&wrong_line, field_ox::EXIT_MAP)
            } else {
                (&won_line, field_ox::WINNERS_MAP)
            };
            if let Ok(packet) = build_broadcast(Broadcast::Pink, line) {
                occupant.handle.send_packet(packet);
            }
            occupant.handle.send(ServerMessage::ShipWarp {
                from_map: self.key.map_id,
                map_id,
                portal: conti::Portal::Id(0),
            });
        }
        if last {
            info!(field = ?self.key, "OX Quiz over");
        } else if !playing {
            info!(field = ?self.key, "OX Quiz has nobody left");
            if self.ox.as_mut().and_then(OxGame::reset).is_some() {
                if let Some(packet) = clear {
                    self.broadcast(None, packet);
                }
            }
        }
    }

    /// A GM's `/start` (at `started`) or `/reset` (None) of the timed event
    /// on this map: its clock and notices start, or everyone still inside
    /// goes out at once.
    fn run_timed_event(&mut self, started: Option<Instant>) {
        let Some(event) = net::gm_event::timed_event(self.key.map_id) else {
            return;
        };
        match started {
            Some(started) if self.race.is_none() => {
                info!(field = ?self.key, "Timed GM event started");
                let race = Race::new(event, started, net::events::scaled);
                let now = Instant::now();
                let clock = field_coconut::seconds(race.left(now));
                self.race = Some(race);
                // Find the Jewel's halls start full, whatever the last run
                // broke there.
                let refilled = self.reactors.refill(now);
                self.show_reactors(&refilled);
                if let Ok(packet) = build_countdown(clock) {
                    self.broadcast(None, packet);
                }
            }
            None if self.race.is_some() => {
                info!(field = ?self.key, "Timed GM event reset");
                self.end_race();
            }
            _ => {}
        }
    }

    /// Show everyone on the map the reactors just spawned.
    fn show_reactors(&self, object_ids: &[i32]) {
        for &object_id in object_ids {
            if let Some(reactor) = self.reactors.get(object_id) {
                if let Ok(packet) = build_spawn_reactor(&reactor.view()) {
                    self.broadcast(None, packet);
                }
            }
        }
    }

    /// The timed event's notices as they come due, and its end.
    fn tick_race(&mut self, now: Instant) {
        while let Some(turn) = self.race.as_mut().and_then(|race| race.tick(now)) {
            match turn {
                field_race::Turn::Notice(key) => {
                    let line = net::content::text().get(key, &[]);
                    if let Ok(packet) = build_broadcast(Broadcast::Notice, &line) {
                        self.broadcast(None, packet);
                    }
                }
                field_race::Turn::Over => {
                    info!(field = ?self.key, "Timed GM event over");
                    self.end_race();
                }
            }
        }
    }

    /// The timed event ends: its clock goes, and everyone still on the map
    /// but GMs goes out.
    fn end_race(&mut self) {
        self.race = None;
        if let Ok(packet) = build_countdown(0) {
            self.broadcast(None, packet);
        }
        for occupant in self.occupants.values().filter(|o| !o.avatar.gm) {
            occupant.handle.send(ServerMessage::ShipWarp {
                from_map: self.key.map_id,
                map_id: net::gm_event::EXIT_MAP,
                portal: conti::Portal::Id(0),
            });
        }
    }

    /// A GM's `/start` or `/reset` on a Snowball field.
    fn run_snowball_event(&mut self, event: GmEvent) {
        let Some(game) = &mut self.snowball else {
            return;
        };
        let now = Instant::now();
        let clock = match event {
            GmEvent::Start => {
                let length = net::events::scaled(field_snowball::LENGTH);
                if !game.start(now, length) {
                    return;
                }
                info!(field = ?self.key, "Snowball game started");
                field_snowball::seconds(length)
            }
            GmEvent::Reset => {
                if !game.reset() {
                    return;
                }
                info!(field = ?self.key, "Snowball game reset");
                0
            }
        };
        for packet in [snowball_state(game), build_countdown(clock)]
            .into_iter()
            .flatten()
        {
            self.broadcast(None, packet);
        }
    }

    /// The Snowball game's time: snowmen back up, the balls' pace, the end,
    /// and everyone sent off when it has been shown.
    fn tick_snowball(&mut self, now: Instant) {
        let Some(game) = &mut self.snowball else {
            return;
        };
        let finish = net::events::scaled(field_snowball::FINISH);
        let turns = game.tick(now, finish);
        let game = &*game;
        let mut packets = Vec::new();
        for turn in turns {
            match turn {
                field_snowball::Turn::Changed => packets.push(snowball_state(game)),
                field_snowball::Turn::Message { team, message } => {
                    packets.push(build_snowball_message(team, message));
                }
                field_snowball::Turn::Ended { winner } => {
                    info!(field = ?self.key, ?winner, "Snowball game over");
                    let line = net::content::text().get(keys::SNOWBALL_ENDED, &[]);
                    packets.push(snowball_state(game));
                    packets.push(build_countdown(0));
                    packets.push(build_broadcast(Broadcast::Pink, &line));
                }
                field_snowball::Turn::WarpOut { winner } => {
                    for (&client_id, occupant) in &self.occupants {
                        let won = winner.is_some() && game.team(client_id) == winner;
                        occupant.handle.send(ServerMessage::ShipWarp {
                            from_map: self.key.map_id,
                            map_id: if won {
                                field_snowball::WINNERS_MAP
                            } else {
                                field_snowball::EXIT_MAP
                            },
                            portal: conti::Portal::Id(0),
                        });
                    }
                }
            }
        }
        for packet in packets.into_iter().flatten() {
            self.broadcast(None, packet);
        }
    }

    fn handle_leave(&mut self, client_id: ClientId, staying: bool) {
        let Some(left) = self.occupants.remove(&client_id) else {
            return;
        };
        for touch in self.reactors.step_off(client_id) {
            self.reactor_touched(touch);
        }
        let out = self.trades.leave(client_id);
        self.deliver_trade(out);
        let out = self.shops.leave(client_id);
        self.deliver_trade(out);
        let out = self.games.leave(client_id);
        self.deliver_trade(out);
        self.show_balloons();
        let out = self.merchants.leave(client_id, Instant::now());
        self.deliver_trade(out);
        self.show_merchants();
        // The others drop a player's summons with the player.
        self.summons.remove_owned(client_id, None);
        if let Some(game) = &mut self.coconut {
            game.leave(client_id);
        }
        if let Some(game) = &mut self.snowball {
            game.leave(client_id);
        }
        // Everyone sees the player go before control passes on.
        match build_player_leave_field(client_id) {
            Ok(packet) => self.broadcast_seen(&left.avatar, packet),
            Err(error) => warn!(client_id, %error, "Failed to build leave packet"),
        }
        let next = self.occupants.keys().next().copied();
        let mover = self.mob_candidate();
        let handed = self.mobs.release(client_id, mover);
        if let Some(mover) = mover {
            self.send_control(mover, &handed);
        }
        if self.npc_controller == Some(client_id) {
            self.npc_controller = None;
            if let Some(next) = next {
                self.send_npc_control(next);
            }
        }
        // A boss or solo room is fresh for its next challenger, but not
        // for one warping within it.
        if next.is_none() && !staying && net::npc_scripts::resets_when_empty(self.key.map_id) {
            let now = Instant::now();
            self.mobs.renew(now);
            self.reactors.renew(now);
            self.guardians.clear();
            self.salion = None;
            self.pots = 0;
            self.papa_pixie = false;
            if reactors::shuffled(self.key.map_id) {
                self.reactors.shuffle(&mut rand::thread_rng());
            }
        }
    }

    fn handle_npc_interaction(&mut self, from: ClientId, object_id: i32, request_id: u64) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let Some(index) = self
            .map_npcs
            .iter()
            .position(|npc| npc.object_id == object_id)
        else {
            return;
        };
        let npc = &self.map_npcs[index];
        // A Magatia library's book, in a run's copy, gives up what it holds
        // once; the hall's books below it hold nothing.
        let book =
            net::npc_scripts::magatia_book_side(npc.npc_id).filter(|_| self.key.instance_id != 0);
        if let Some(side) = book {
            let npc_id = npc.npc_id;
            let find = if self.key.map_id == side.stages()[0] {
                let slot = self.map_npcs[..index]
                    .iter()
                    .filter(|npc| npc.npc_id == npc_id)
                    .count();
                let books = self
                    .books
                    .get_or_insert_with(|| net::events::draw_books(&mut rand::thread_rng()));
                books.get_mut(slot).and_then(Option::take)
            } else {
                None
            };
            occupant.handle.send(ServerMessage::BookOpened {
                npc_id,
                field: self.key,
                request_id,
                find,
            });
            return;
        }
        // The field validates map object identity; the client actor owns the
        // conversation and serializes quest persistence with character state.
        let nth = self.map_npcs[..index]
            .iter()
            .filter(|other| other.npc_id == npc.npc_id)
            .count();
        occupant.handle.send(ServerMessage::NpcConversation {
            npc_id: npc.npc_id,
            nth: u8::try_from(nth).unwrap_or(u8::MAX),
            field: self.key,
            request_id,
        });
    }

    fn handle_quest_npc(&self, from: ClientId, npc_id: i32, request_id: u64) {
        let Some(occupant) = self.occupants.get(&from) else {
            return;
        };
        let (x, y) = (i32::from(occupant.avatar.x), i32::from(occupant.avatar.y));
        let nearby = self.map_npcs.iter().any(|npc| {
            npc.npc_id == npc_id
                && (i32::from(npc.x) - x).abs() <= QUEST_NPC_REACH_X
                && (i32::from(npc.y) - y).abs() <= QUEST_NPC_REACH_Y
        });
        occupant
            .handle
            .send(ServerMessage::QuestNpcChecked { request_id, nearby });
    }

    /// Send to every occupant, optionally skipping one. Never waits.
    fn broadcast(&self, except: Option<ClientId>, packet: Packet) {
        for (&client_id, occupant) in &self.occupants {
            if Some(client_id) != except {
                occupant.handle.send_packet(packet.clone());
            }
        }
    }

    /// The HP bar of a mob `from` hurt, for them and their party here
    /// (Cosmic's rule), so a party's Dojo gauge follows every member's hits.
    fn show_mob_hp(&self, from: ClientId, object_id: i32, percent: u8) {
        let Some(party) = self.occupants.get(&from).map(|o| o.party) else {
            return;
        };
        let Ok(packet) = build_mob_hp(object_id, percent) else {
            return;
        };
        for (&client_id, occupant) in &self.occupants {
            if client_id == from || (party.is_some() && occupant.party == party) {
                occupant.handle.send_packet(packet.clone());
            }
        }
    }

    /// Send what `from` did to the others here who see them.
    fn broadcast_by(&self, from: ClientId, packet: Packet) {
        self.broadcast_seen_by(from, Some(from), packet);
    }

    /// Send `packet`, about `from`, to those here but `except` who see them.
    fn broadcast_seen_by(&self, from: ClientId, except: Option<ClientId>, packet: Packet) {
        match self.occupants.get(&from) {
            Some(occupant) => self.broadcast_about(occupant.avatar.id, except, packet),
            None => self.broadcast(except, packet),
        }
    }

    /// Send `packet`, about the character `owner_id` or what is theirs, to
    /// those here but `except` who see them: a hidden GM's shows only to
    /// GMs, as v83 drops what is about a character it doesn't know
    /// (report b177).
    fn broadcast_about(&self, owner_id: i32, except: Option<ClientId>, packet: Packet) {
        let owner = self
            .occupants
            .values()
            .find(|occupant| occupant.avatar.id == owner_id);
        for (&client_id, occupant) in &self.occupants {
            if Some(client_id) != except
                && owner.is_none_or(|owner| sees(&occupant.avatar, &owner.avatar))
            {
                occupant.handle.send_packet(packet.clone());
            }
        }
    }

    /// Send a player's coming or going to those here who see the player.
    fn broadcast_seen(&self, avatar: &FieldAvatar, packet: Packet) {
        for occupant in self.occupants.values() {
            if sees(&occupant.avatar, avatar) {
                occupant.handle.send_packet(packet.clone());
            }
        }
    }

    /// A GM hid, so their mobs go to someone the mobs see, or came out of
    /// hiding and moves those no one does (Cosmic's `hide`).
    fn hide_from_mobs(&mut self, client_id: ClientId) {
        let Some(gm) = self.occupants.get(&client_id) else {
            return;
        };
        if !gm.avatar.hidden {
            let controlled = self.mobs.assign_uncontrolled(client_id);
            return self.send_control(client_id, &controlled);
        }
        let mover = self.mob_candidate();
        let handed = self.mobs.hand_off(client_id, mover);
        for &object_id in &handed {
            if let Ok(packet) = build_stop_controlling_mob(object_id) {
                gm.handle.send_packet(packet);
            }
        }
        if let Some(mover) = mover {
            self.send_control(mover, &handed);
        }
    }

    /// A GM hid or came out of hiding: those not GMs lose or see the GM,
    /// and the GM's summons.
    fn show_hidden(&self, client_id: ClientId) {
        let Some(gm) = self.occupants.get(&client_id) else {
            return;
        };
        let summons = self
            .summons
            .views()
            .filter(|summon| summon.owner_id == gm.avatar.id);
        let packets: Result<Vec<Packet>, _> = if gm.avatar.hidden {
            summons
                .map(|summon| {
                    build_remove_summon(summon.owner_id, summon.object_id, Gone::Silently)
                })
                .chain([build_player_leave_field(client_id)])
                .collect()
        } else {
            player_spawn(&gm.avatar).and_then(|mut packets| {
                for summon in summons {
                    packets.push(build_spawn_summon(summon, false)?);
                }
                Ok(packets)
            })
        };
        let packets = match packets {
            Ok(packets) => packets,
            Err(error) => return warn!(client_id, %error, "Failed to build hide packet"),
        };
        for (&other, occupant) in &self.occupants {
            if other != client_id && !occupant.avatar.gm {
                for packet in &packets {
                    occupant.handle.send_packet(packet.clone());
                }
            }
        }
    }
}

/// SPAWN_PLAYER for `avatar`, then its item buff's aura, which v83 keeps
/// apart from the spawn (report c029 A).
/// What a player arriving on a Coconut Season field is told: the score,
/// their team, and the clock if a game is on. The nuts already down stay
/// up for them (report b167).
fn coconut_entry(game: &CoconutGame, client_id: ClientId, now: Instant) -> Vec<Packet> {
    let [maple, story] = game.scores();
    let mut packets = vec![
        build_coconut_score(maple, story),
        build_field_team(game.team(client_id)),
    ];
    // The client keeps the nuts hidden until told to show them, and knows
    // none that came down before it arrived.
    if game.on() {
        packets.push(build_coconut_hit(-1, 0, 0));
        for (index, state) in game.downed() {
            packets.push(build_coconut_hit(index as i16, 0, state));
        }
    }
    if let Some(seconds) = game.remaining(now) {
        packets.push(build_countdown(seconds));
    }
    packets.into_iter().flatten().collect()
}

/// The Snowball field's state, as everyone on it is sent it.
fn snowball_state(game: &SnowballGame) -> Result<Packet, net::error::NetworkError> {
    build_snowball_state(game.state(), game.snowmen(), game.balls(), None)
}

/// What a player arriving on a Snowball field is told: the field's state,
/// with the damages its hits show, and the clock if a game is on
/// (report b168).
fn snowball_entry(game: &SnowballGame, now: Instant) -> Vec<Packet> {
    let state = build_snowball_state(
        game.state(),
        game.snowmen(),
        game.balls(),
        Some(game.damages()),
    );
    let clock = game.remaining(now).map(build_countdown);
    [Some(state), clock]
        .into_iter()
        .flatten()
        .flatten()
        .collect()
}

/// What a player arriving on the OX Quiz field is told: the map's help,
/// and the question up with the event clock at its time left, or the one
/// whose answer shows (report b171).
fn ox_entry(game: &OxGame, now: Instant) -> Vec<Packet> {
    let mut packets = vec![build_gm_event_instructions(0)];
    if let Some((question, left)) = game.showing(now) {
        if let Some(left) = left {
            // Below 0 the client puts up a running clock, if none is.
            let seconds = field_coconut::seconds(left) as i32;
            packets.push(build_event_clock(-seconds));
        }
        packets.push(build_ox_quiz(left.is_some(), question.set, question.number));
    }
    packets.into_iter().flatten().collect()
}

fn player_spawn(avatar: &FieldAvatar) -> Result<Vec<Packet>, net::error::NetworkError> {
    let mut packets = vec![build_player_enter_field(avatar)?];
    if avatar.consume_effect != 0 {
        packets.push(build_show_consume_effect(avatar.id, avatar.consume_effect)?);
    }
    Ok(packets)
}

/// Where a summon whose skill covers `area` comes out: a random place in
/// it, on the ground below, with that ground's foothold; None if there is
/// no ground below. (The Papulatus Clock's Darkstars and Pianus's Bloody
/// Booms are the summons with an area.)
fn summon_spot(drops: &DropSet, area: &CastArea, rng: &mut impl Rng) -> Option<((i16, i16), i16)> {
    let x = rng.gen_range(area.left, area.right.max(area.left) + 1);
    let y = rng.gen_range(area.top, area.bottom.max(area.top) + 1);
    let (x, y) = (i16::try_from(x).ok()?, i16::try_from(y).ok()?);
    let (ground, foothold) = drops.foothold_below(x, y)?;
    Some(((x, ground), foothold))
}

/// Show `drop` to one player: a quest drop goes through their client actor,
/// which shows it only if they still need it.
fn show_drop_to(handle: &ClientHandle, drop: &Drop, packet: Packet) {
    match drop.quest_item() {
        Some((quest, item_id)) => {
            handle.send(ServerMessage::QuestDrop {
                quest,
                item_id,
                packet,
            });
        }
        None => {
            handle.send_packet(packet);
        }
    }
}

/// Whether `viewer` sees `avatar`: a hidden GM shows only to GMs.
fn sees(viewer: &FieldAvatar, avatar: &FieldAvatar) -> bool {
    !avatar.hidden || viewer.gm
}

fn to_foreign_npc(npc: &FieldMapEntityNpc) -> ForeignNpc {
    ForeignNpc {
        object_id: npc.object_id,
        npc_id: npc.npc_id,
        x: npc.x,
        y: npc.y,
        flip: npc.flip,
        foothold: npc.foothold,
        rx0: npc.rx0,
        rx1: npc.rx1,
        enabled: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::AvatarLook;
    use net::packet::op::SendOpcode;
    use std::collections::BTreeMap;
    use tokio::time::timeout;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn items_knocked_out_of_a_bag_land_on_either_side_in_turn() {
        let offsets: Vec<i16> = (1..=6).map(shed_offset).collect();
        assert_eq!(offsets, [0, 37, -25, 62, -50, 87]);
    }

    #[test]
    fn summons_land_somewhere_in_their_skills_area() {
        use game_data::footholds::{Foothold, Footholds};
        let floor = |id, x1, x2, y| Foothold {
            id,
            x1,
            y1: y,
            x2,
            y2: y,
        };
        // Two of the Origin of Clocktower's ledges and its floor; the High
        // Darkstars' area over a clock at (0, -386), short of the floor's
        // end at 262.
        let drops = DropSet::new(Footholds::new(vec![
            floor(16, -129, -74, -538),
            floor(4, -634, -605, -584),
            floor(21, -928, 262, -386),
        ]));
        let area = CastArea {
            left: -500,
            top: -886,
            right: 250,
            bottom: -536,
        };
        let mut rng = rand::thread_rng();
        let mut ledges = 0;
        for _ in 0..500 {
            let ((x, y), fh) = summon_spot(&drops, &area, &mut rng).unwrap();
            assert!((-500..=250).contains(&x), "{x}");
            match fh {
                16 => assert!((-129..=-74).contains(&x) && y == -538),
                4 => assert!((-634..=-605).contains(&x) && y == -584),
                _ => assert_eq!((fh, y), (21, -386)),
            }
            ledges += usize::from(fh == 16);
        }
        assert!(ledges > 0);
        // No ground below: back to the caster.
        let past = CastArea {
            left: 300,
            top: -500,
            right: 400,
            bottom: -400,
        };
        assert_eq!(summon_spot(&drops, &past, &mut rng), None);
    }

    fn avatar(id: i32, x: i16, y: i16) -> FieldAvatar {
        FieldAvatar {
            id,
            name: format!("p{id}"),
            level: 1,
            job: 0,
            look: AvatarLook {
                gender: 0,
                skin: 0,
                face: 20000,
                hair: 30000,
                visible: BTreeMap::from([(5, 1040002)]),
                masked: BTreeMap::new(),
                cash_weapon: None,
                pets: [0; 3],
            },
            x,
            y,
            stance: 2,
            chair: 0,
            guild: None,
            buffs: Vec::new(),
            mini_room: None,
            pets: Vec::new(),
            chalkboard: None,
            item_effect: 0,
            consume_effect: 0,
            berserk: false,
            rings: game::ring::RingLooks::default(),
            energy: None,
            dash: [None; 2],
            ride: None,
            mount: game::Mount::default(),
            gm: false,
            hidden: false,
            team: 0,
        }
    }

    fn client(id: i32) -> (ClientHandle, mpsc::Receiver<ServerMessage>) {
        let (tx, rx) = mpsc::channel(16);
        (
            ClientHandle::new(id, id as u64, tx, CancellationToken::new()),
            rx,
        )
    }

    fn spawn_field() -> mpsc::Sender<FieldMessage> {
        let (tx, rx) = mpsc::channel(16);
        let actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        tokio::spawn(actor.run());
        tx
    }

    async fn next_packet(rx: &mut mpsc::Receiver<ServerMessage>) -> Packet {
        match timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            ServerMessage::SendPacket(packet) => packet,
            other => panic!("expected a packet, got {other:?}"),
        }
    }

    async fn next_opcode(rx: &mut mpsc::Receiver<ServerMessage>) -> i16 {
        next_packet(rx).await.opcode()
    }

    #[tokio::test]
    async fn the_map_sees_who_sits_on_a_chair() {
        let field = spawn_field();
        let (first, _first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        let relaxer = 3010000i32.to_le_bytes();

        field
            .send(FieldMessage::Join {
                handle: first,
                avatar: avatar(1, 10, 20),
                party: None,
            })
            .await
            .unwrap();
        field
            .send(FieldMessage::FromClient {
                from: 1,
                request: FieldRequest::Chair(3010000),
            })
            .await
            .unwrap();
        // Someone arriving later sees the player already seated.
        field
            .send(FieldMessage::Join {
                handle: second,
                avatar: avatar(2, 30, 40),
                party: None,
            })
            .await
            .unwrap();
        let spawn = next_packet(&mut second_rx).await;
        assert_eq!(spawn.opcode(), SendOpcode::SpawnPlayer as i16);
        assert!(spawn.bytes.windows(4).any(|bytes| bytes == relaxer));

        field
            .send(FieldMessage::FromClient {
                from: 1,
                request: FieldRequest::Chair(0),
            })
            .await
            .unwrap();
        let stood = next_packet(&mut second_rx).await;
        assert_eq!(stood.opcode(), SendOpcode::ShowChair as i16);
        assert_eq!(&stood.bytes[2..], &[1, 0, 0, 0, 0, 0, 0, 0]);
    }

    /// The packets `rx` holds now.
    fn sent(rx: &mut mpsc::Receiver<ServerMessage>) -> Vec<Packet> {
        let mut packets = Vec::new();
        while let Ok(message) = rx.try_recv() {
            if let ServerMessage::SendPacket(packet) = message {
                packets.push(packet);
            }
        }
        packets
    }

    #[tokio::test]
    async fn a_coconut_game_runs_from_the_gms_start_to_the_warp_out() {
        let (_tx, rx) = mpsc::channel(16);
        let mut actor = FieldActor::new(
            FieldKey::new(0, 109080000),
            rx,
            Vec::new(),
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        actor.coconut = Some(CoconutGame::new(game_data::Coconut {
            count_falling: 10,
            time_default: 300,
            time_expand: 120,
            time_finish: 12,
            effect_win: "event/coconut/victory".into(),
            effect_lose: "event/coconut/lose".into(),
            sound_win: "Coconut/Victory".into(),
            sound_lose: "Coconut/Failed".into(),
            ..Default::default()
        }));
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        let of = |packets: &[Packet], op: SendOpcode| -> Vec<Vec<u8>> {
            let op = op as i16;
            packets
                .iter()
                .filter(|packet| packet.opcode() == op)
                .map(|packet| packet.bytes[2..].to_vec())
                .collect()
        };
        for (handle, id) in [(first, 1), (second, 2)] {
            actor.handle_message(FieldMessage::Join {
                handle,
                avatar: avatar(id, 0, 0),
                party: None,
            });
        }
        // Each is told the score and their team, and none a clock yet.
        let joined = sent(&mut first_rx);
        assert_eq!(of(&joined, SendOpcode::CoconutScore), [vec![0, 0, 0, 0]]);
        assert_eq!(of(&joined, SendOpcode::FieldSpecificData), [vec![0]]);
        assert!(of(&joined, SendOpcode::Clock).is_empty());
        let joined = sent(&mut second_rx);
        assert_eq!(of(&joined, SendOpcode::FieldSpecificData), [vec![1]]);
        assert_eq!(actor.occupants[&2].avatar.team, 1);

        // A hit before the start does nothing.
        actor.handle_message(FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::CoconutHit { index: 4 },
        });
        assert!(sent(&mut second_rx).is_empty());
        let start = Instant::now();
        actor.handle_message(FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::GmEvent(GmEvent::Start),
        });
        let started = sent(&mut second_rx);
        // -1 every nut, shown after 5000 ms.
        assert_eq!(
            of(&started, SendOpcode::CoconutHit),
            [vec![0xFF, 0xFF, 0x88, 0x13, 0]]
        );
        assert_eq!(of(&started, SendOpcode::Clock), [vec![2, 44, 1, 0, 0]]);
        actor.handle_message(FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::CoconutHit { index: 4 },
        });
        // Nut 4 shakes for both.
        assert_eq!(
            of(&sent(&mut second_rx), SendOpcode::CoconutHit),
            [vec![4, 0, 0xE8, 3, 1]]
        );
        sent(&mut first_rx);

        // A tie plays on, then is a draw that both lose.
        actor.on_tick(start + Duration::from_secs(301));
        let over = sent(&mut first_rx);
        assert_eq!(of(&over, SendOpcode::Clock), [vec![2, 120, 0, 0, 0]]);
        actor.on_tick(start + Duration::from_secs(422));
        let ended = sent(&mut second_rx);
        let effects = of(&ended, SendOpcode::FieldEffect);
        assert_eq!(effects.len(), 2);
        assert!(effects[0].ends_with(b"event/coconut/lose"));
        assert!(effects[1].ends_with(b"Coconut/Failed"));
        actor.on_tick(start + Duration::from_secs(435));
        let warps = |rx: &mut mpsc::Receiver<ServerMessage>| {
            let mut maps = Vec::new();
            while let Ok(message) = rx.try_recv() {
                if let ServerMessage::ShipWarp { map_id, .. } = message {
                    maps.push(map_id);
                }
            }
            maps
        };
        assert_eq!(warps(&mut first_rx), [field_coconut::EXIT_MAP]);
        assert_eq!(warps(&mut second_rx), [field_coconut::EXIT_MAP]);
    }

    #[tokio::test]
    async fn a_snowball_game_runs_from_the_gms_start_to_the_warp_out() {
        let (_tx, rx) = mpsc::channel(16);
        let mut actor = FieldActor::new(
            FieldKey::new(0, 109060000),
            rx,
            Vec::new(),
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        actor.snowball = Some(SnowballGame::new(
            game_data::Snowball {
                x_max: 2,
                damage_ball: 10,
                damage_snowman: 15,
                damage_snowman_strong: 45,
                snowman_hp: 7500,
                sections: [1, 5, 6],
                ..Default::default()
            },
            Duration::from_secs(10),
        ));
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        let of = |packets: &[Packet], op: SendOpcode| -> Vec<Vec<u8>> {
            let op = op as i16;
            packets
                .iter()
                .filter(|packet| packet.opcode() == op)
                .map(|packet| packet.bytes[2..].to_vec())
                .collect()
        };
        for (handle, id) in [(first, 1), (second, 2)] {
            actor.handle_message(FieldMessage::Join {
                handle,
                avatar: avatar(id, 0, 0),
                party: None,
            });
        }
        // Each is told the field's state with its damages, and no clock.
        let idle = [0, 100, 0, 0, 0, 100, 0, 0, 0, 0, 0, 8, 0, 0, 8];
        let joined = sent(&mut first_rx);
        let mut entry = idle.to_vec();
        entry.extend([10, 0, 15, 0, 45, 0]);
        assert_eq!(of(&joined, SendOpcode::SnowballState), [entry]);
        assert!(of(&joined, SendOpcode::Clock).is_empty());
        sent(&mut second_rx);
        assert_eq!(actor.occupants[&2].avatar.team, field_snowball::MAPLE);

        let start = Instant::now();
        actor.handle_message(FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::GmEvent(GmEvent::Start),
        });
        let started = sent(&mut second_rx);
        let mut running = idle.to_vec();
        running[0] = 1;
        assert_eq!(of(&started, SendOpcode::SnowballState), [running]);
        assert_eq!(of(&started, SendOpcode::Clock), [vec![2, 0x58, 2, 0, 0]]);

        // Story's three hits on its ball roll it one step, past section 1.
        let hit = |actor: &mut FieldActor, from, target| {
            actor.handle_message(FieldMessage::FromClient {
                from,
                request: FieldRequest::SnowballHit { target, delay: 300 },
            });
        };
        hit(&mut actor, 1, 0);
        assert_eq!(
            of(&sent(&mut second_rx), SendOpcode::HitSnowball),
            [vec![0, 10, 0, 0x2C, 1]]
        );
        // Story can't hit Maple's ball.
        hit(&mut actor, 1, 1);
        assert!(sent(&mut second_rx).is_empty());
        for _ in 0..2 {
            actor.snowball.as_mut().unwrap().forget_hits();
            hit(&mut actor, 1, 0);
        }
        let rolled = sent(&mut second_rx);
        assert_eq!(of(&rolled, SendOpcode::HitSnowball).len(), 2);
        assert_eq!(of(&rolled, SendOpcode::SnowballState)[0][9..11], [1, 0]);
        assert_eq!(of(&rolled, SendOpcode::SnowballMessage), [vec![0, 1]]);
        sent(&mut first_rx);

        // Walking into a ball throws the walker back, and only them.
        actor.handle_message(FieldMessage::FromClient {
            from: 2,
            request: FieldRequest::IntoSnowball,
        });
        assert_eq!(
            of(&sent(&mut second_rx), SendOpcode::LeftKnockBack).len(),
            1
        );
        assert!(sent(&mut first_rx).is_empty());

        // Story's ball is further on when the time runs out: Maple's fades.
        actor.on_tick(start + Duration::from_secs(601));
        let ended = sent(&mut second_rx);
        assert_eq!(of(&ended, SendOpcode::SnowballState)[0][0], 3);
        assert_eq!(of(&ended, SendOpcode::Clock), [vec![2, 0, 0, 0, 0]]);
        assert_eq!(of(&ended, SendOpcode::BroadcastMsg).len(), 1);
        actor.on_tick(start + Duration::from_secs(611));
        let warps = |rx: &mut mpsc::Receiver<ServerMessage>| {
            let mut maps = Vec::new();
            while let Ok(message) = rx.try_recv() {
                if let ServerMessage::ShipWarp { map_id, .. } = message {
                    maps.push(map_id);
                }
            }
            maps
        };
        assert_eq!(warps(&mut first_rx), [field_snowball::WINNERS_MAP]);
        assert_eq!(warps(&mut second_rx), [field_snowball::EXIT_MAP]);
    }

    #[test]
    fn a_player_arriving_mid_game_sees_the_nuts_the_others_see() {
        let mut game = CoconutGame::new(game_data::Coconut {
            count_falling: 10,
            ..Default::default()
        });
        let hits = |game: &CoconutGame, id| -> Vec<Vec<u8>> {
            coconut_entry(game, id, Instant::now())
                .iter()
                .filter(|packet| packet.opcode() == SendOpcode::CoconutHit as i16)
                .map(|packet| packet.bytes[2..].to_vec())
                .collect()
        };
        game.join(1);
        // Before a game the nuts stay hidden.
        assert!(hits(&game, 1).is_empty());
        let now = Instant::now();
        game.start(now, Duration::from_secs(300));
        // Three hits shake nut 2, the fourth brings it down.
        for second in 0..4 {
            game.hit(1, 2, now + Duration::from_secs(second), &mut || 0.0);
        }
        game.join(2);
        assert_eq!(
            hits(&game, 2),
            [vec![0xFF, 0xFF, 0, 0, 0], vec![2, 0, 0, 0, 3]]
        );
    }

    #[tokio::test]
    async fn a_buff_weather_gives_everyone_here_its_items_effect() {
        if net::items::info(5121000).is_none() {
            return;
        }
        let (_tx, rx) = mpsc::channel(16);
        let mut actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        for (handle, id) in [(first, 1), (second, 2)] {
            actor.handle_message(FieldMessage::Join {
                handle,
                avatar: avatar(id, 10, 20),
                party: None,
            });
        }
        let buffs = |rx: &mut mpsc::Receiver<ServerMessage>| -> Vec<i32> {
            std::iter::from_fn(|| rx.try_recv().ok())
                .filter_map(|message| match message {
                    ServerMessage::FieldBuff(item_id) => Some(item_id),
                    _ => None,
                })
                .collect()
        };
        buffs(&mut first_rx);
        buffs(&mut second_rx);

        // Fighting Spirit gives what a 2022071 does; a snow, nothing.
        for item_id in [5121000, 5120000] {
            actor.handle_message(FieldMessage::FromClient {
                from: 1,
                request: FieldRequest::Weather {
                    item_id,
                    message: "hi".into(),
                },
            });
        }
        assert_eq!(buffs(&mut first_rx), [2022071]);
        assert_eq!(buffs(&mut second_rx), [2022071]);
    }

    #[tokio::test]
    async fn a_weather_blows_for_whoever_comes_until_it_stops() {
        let (_tx, rx) = mpsc::channel(16);
        let mut actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        let snow = [0x8E, 0, 0, 0, 0x20, 0x4E, 0, 2, 0, b'h', b'i'];
        let weather = |packets: Vec<Packet>| -> Vec<Vec<u8>> {
            packets
                .into_iter()
                .filter(|packet| packet.opcode() == SendOpcode::BlowWeather as i16)
                .map(|packet| packet.bytes)
                .collect()
        };

        actor.handle_message(FieldMessage::Join {
            handle: first,
            avatar: avatar(1, 10, 20),
            party: None,
        });
        sent(&mut first_rx);
        actor.handle_message(FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::Weather {
                item_id: 5120000,
                message: "hi".into(),
            },
        });
        assert_eq!(weather(sent(&mut first_rx)), [snow.to_vec()]);

        actor.handle_message(FieldMessage::Join {
            handle: second,
            avatar: avatar(2, 30, 40),
            party: None,
        });
        assert_eq!(weather(sent(&mut second_rx)), [snow.to_vec()]);

        actor.on_tick(Instant::now() + Duration::from_secs(29));
        assert!(weather(sent(&mut first_rx)).is_empty(), "still blowing");
        actor.on_tick(Instant::now() + WEATHER_TIME);
        let stop = vec![0x8E, 0, 0, 0, 0, 0, 0];
        assert_eq!(weather(sent(&mut first_rx)), [stop.clone()]);
        assert_eq!(weather(sent(&mut second_rx)), [stop]);
        actor.on_tick(Instant::now() + WEATHER_TIME * 2);
        assert!(weather(sent(&mut first_rx)).is_empty(), "stopped once");
    }

    #[test]
    fn an_area_boss_coming_out_is_told_to_its_map() {
        let mano = *net::mobs::area_boss_on(104000400, 2220000).unwrap();
        let spawn = game_data::MapMobTemplate {
            mob_id: mano.mob,
            x: mano.at.0,
            y: mano.at.1,
            foothold: 0,
            flip: false,
            mob_time: net::mobs::AREA_BOSS_TIME,
            hide: false,
            team: None,
        };
        let info = game_data::mobs::MobInfo {
            mob_id: mano.mob,
            max_hp: 8,
            boss: true,
            ..Default::default()
        };
        let (_tx, rx) = mpsc::channel(16);
        let mut actor = FieldActor::new(
            FieldKey::new(0, mano.map),
            rx,
            Vec::new(),
            MobSet::new([(spawn, info)]),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_secs(60),
        );
        let (first, mut first_rx) = client(1);
        actor.handle_message(FieldMessage::Join {
            handle: first,
            avatar: avatar(1, 10, 20),
            party: None,
        });
        sent(&mut first_rx);

        // Its point waits at most seven tenths of its mobTime at first.
        let time = Duration::from_secs(net::mobs::AREA_BOSS_TIME as u64);
        actor.on_tick(Instant::now() + time);
        let packets = sent(&mut first_rx);
        let opcodes: Vec<i16> = packets.iter().map(Packet::opcode).collect();
        assert!(
            opcodes.contains(&(SendOpcode::SpawnMonster as i16)),
            "{opcodes:?}"
        );
        let line = net::content::text().get(keys::AREA_BOSS_APPEARED, &[("mob", mano.name)]);
        let notice = build_item_line(&line, 0).unwrap();
        assert!(packets.iter().any(|packet| packet.bytes == notice.bytes));

        // Nothing more while it is out.
        actor.on_tick(Instant::now() + time * 3);
        assert!(sent(&mut first_rx)
            .iter()
            .all(|packet| packet.bytes != notice.bytes));
    }

    #[tokio::test]
    async fn a_kite_flies_over_its_flier_for_whoever_comes_until_its_hour_is_up() {
        let (_tx, rx) = mpsc::channel(16);
        let mut actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        let kites = |packets: Vec<Packet>, op: i16| -> Vec<Vec<u8>> {
            packets
                .into_iter()
                .filter(|packet| packet.opcode() == op)
                .map(|packet| packet.bytes)
                .collect()
        };

        actor.handle_message(FieldMessage::Join {
            handle: first,
            avatar: avatar(1, 10, 20),
            party: None,
        });
        sent(&mut first_rx);
        actor.handle_message(FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::Kite {
                item_id: 5080000,
                message: "hi\n\n".into(),
            },
        });
        let first_kite = actor.kites.views().next().unwrap().clone();
        assert_eq!(first_kite.object_id, crate::actor::object_ids::KITES.start);
        assert_eq!(
            (first_kite.x, first_kite.y),
            (10, 20),
            "where its flier stands"
        );
        let spawn = build_spawn_kite(&first_kite).unwrap().bytes;
        assert_eq!(
            kites(sent(&mut first_rx), SendOpcode::SpawnKite as i16),
            [spawn.clone()]
        );

        actor.handle_message(FieldMessage::Join {
            handle: second,
            avatar: avatar(2, 30, 40),
            party: None,
        });
        assert_eq!(
            kites(sent(&mut second_rx), SendOpcode::SpawnKite as i16),
            [spawn]
        );

        actor.on_tick(Instant::now() + Duration::from_secs(60 * 59));
        assert!(
            kites(sent(&mut first_rx), SendOpcode::RemoveKite as i16).is_empty(),
            "still flying"
        );
        actor.on_tick(Instant::now() + crate::actor::field_kites::KITE_TIME);
        let down = build_remove_kite(first_kite.object_id, true).unwrap().bytes;
        assert_eq!(
            kites(sent(&mut first_rx), SendOpcode::RemoveKite as i16),
            [down.clone()]
        );
        assert_eq!(
            kites(sent(&mut second_rx), SendOpcode::RemoveKite as i16),
            [down]
        );
        assert_eq!(actor.kites.views().count(), 0);
    }

    fn mob(x: i16, boss: bool) -> (game_data::MapMobTemplate, game_data::mobs::MobInfo) {
        (
            game_data::MapMobTemplate {
                mob_id: 100100,
                x,
                y: -25,
                foothold: 45,
                flip: false,
                mob_time: 0,
                hide: false,
                team: None,
            },
            game_data::mobs::MobInfo {
                mob_id: 100100,
                max_hp: 8,
                boss,
                ..Default::default()
            },
        )
    }

    /// The opcodes `rx` gets until it goes quiet.
    async fn opcodes(rx: &mut mpsc::Receiver<ServerMessage>) -> Vec<i16> {
        let mut opcodes = Vec::new();
        while let Ok(Some(message)) = timeout(Duration::from_millis(100), rx.recv()).await {
            if let ServerMessage::SendPacket(packet) = message {
                opcodes.push(packet.opcode());
            }
        }
        opcodes
    }

    #[tokio::test]
    async fn monster_magnet_shows_its_catches_and_the_caught_chase_the_user() {
        let mut mobs = MobSet::new([mob(100, false), mob(200, true)]);
        let spawned = mobs.spawn_all(Instant::now());
        let (tx, rx) = mpsc::channel(16);
        let actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            mobs,
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_secs(60),
        );
        tokio::spawn(actor.run());
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        for (handle, id) in [(first, 1), (second, 2)] {
            tx.send(FieldMessage::Join {
                handle,
                avatar: avatar(id, 0, 0),
                party: None,
            })
            .await
            .unwrap();
        }
        opcodes(&mut first_rx).await;
        opcodes(&mut second_rx).await;

        let effect = Packet::new_empty();
        tx.send(FieldMessage::FromClient {
            from: 2,
            request: FieldRequest::Magnet {
                effect,
                caught: vec![(spawned[0], true), (spawned[1], true), (999, true)],
            },
        })
        .await
        .unwrap();

        // The others see the snail caught and the boss not, then the skill;
        // the snail now chases the user, whose client moves it.
        let magnet = SendOpcode::ShowMagnet as i16;
        let control = SendOpcode::SpawnMonsterControl as i16;
        let seen = opcodes(&mut first_rx).await;
        assert_eq!(seen.iter().filter(|&&op| op == magnet).count(), 2);
        assert_eq!(seen.iter().filter(|&&op| op == control).count(), 1);
        assert_eq!(opcodes(&mut second_rx).await, [control]);
    }

    #[tokio::test]
    async fn drain_takes_a_share_and_mortal_blow_finishes_the_weak() {
        let mut mobs = MobSet::new([mob(100, false)]);
        let snail = mobs.spawn_all(Instant::now())[0];
        let (tx, rx) = mpsc::channel(16);
        let actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            mobs,
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_secs(60),
        );
        tokio::spawn(actor.run());
        let (handle, mut client_rx) = client(1);
        tx.send(FieldMessage::Join {
            handle,
            avatar: avatar(1, 0, 0),
            party: None,
        })
        .await
        .unwrap();
        opcodes(&mut client_rx).await;
        let attack = |damage, takes| FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::Attack {
                broadcast: Packet::new_empty(),
                skill: 0,
                counter: game::MobStat::PCounter,
                lines: 1,
                hits: vec![(snail, damage)],
                inflicts: Vec::new(),
                takes,
                blast: None,
                pockets: Vec::new(),
            },
        };
        // What it took back, and whether the snail died.
        let outcome = |rx: &mut mpsc::Receiver<ServerMessage>| {
            let (mut taken, mut killed) = (Vec::new(), false);
            while let Ok(message) = rx.try_recv() {
                match message {
                    ServerMessage::Absorbed { hp, mp, .. } => taken.push((hp, mp)),
                    ServerMessage::SendPacket(packet) => {
                        killed |= packet.opcode() == SendOpcode::KillMonster as i16
                    }
                    _ => {}
                }
            }
            (taken, killed)
        };

        // Half of 4 comes back, and the snail at half its HP is not yet
        // weak enough for a blow that takes 40%.
        let blow = Takes {
            hp: 50,
            mp: None,
            life: 40,
            steal: 0,
        };
        tx.send(attack(4, blow)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(outcome(&mut client_rx), (vec![(2, 0)], false));
        // A hit of 1 leaves it at 3 and drains nothing; then a blow that
        // takes 40% finishes it, and the drain counts the 3 it took.
        tx.send(attack(1, Takes { life: 0, ..blow })).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(outcome(&mut client_rx), (vec![], false));
        tx.send(attack(1, blow)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(outcome(&mut client_rx), (vec![(1, 0)], true));
    }

    #[test]
    fn seduce_takes_up_to_its_count_and_other_diseases_all() {
        assert_eq!(diseased_most(BuffStat::Seduce, 10), 10);
        assert_eq!(diseased_most(BuffStat::Seduce, 0), usize::MAX);
        assert_eq!(diseased_most(BuffStat::Weakness, 1), usize::MAX);
    }

    #[tokio::test]
    async fn an_immune_mob_takes_1_a_line_from_the_kind_it_is_immune_to() {
        let mut mobs = MobSet::new([mob(100, false)]);
        let snail = mobs.spawn_all(Instant::now())[0];
        let now = Instant::now();
        let immune = [(game::MobStat::PImmune, 1)];
        let later = now + Duration::from_secs(60);
        mobs.add_statuses(snail, &immune, 140, 0, now, later);
        let (tx, rx) = mpsc::channel(16);
        let actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            mobs,
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_secs(60),
        );
        tokio::spawn(actor.run());
        let (handle, mut client_rx) = client(1);
        tx.send(FieldMessage::Join {
            handle,
            avatar: avatar(1, 0, 0),
            party: None,
        })
        .await
        .unwrap();
        opcodes(&mut client_rx).await;
        // Drain gives back all it did, so it shows the damage dealt.
        let drain = Takes {
            hp: 100,
            ..Takes::default()
        };
        let attack = |counter, lines, damage| FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::Attack {
                broadcast: Packet::new_empty(),
                skill: 0,
                counter,
                lines,
                hits: vec![(snail, damage)],
                inflicts: Vec::new(),
                takes: drain,
                blast: None,
                pockets: Vec::new(),
            },
        };
        let dealt = |rx: &mut mpsc::Receiver<ServerMessage>| {
            let mut dealt = Vec::new();
            while let Ok(message) = rx.try_recv() {
                if let ServerMessage::Absorbed { hp, .. } = message {
                    dealt.push(hp);
                }
            }
            dealt
        };

        // Three lines of a swing do 1 each; a spell does all it did.
        tx.send(attack(game::MobStat::PCounter, 3, 6))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(dealt(&mut client_rx), [3]);
        tx.send(attack(game::MobStat::MCounter, 1, 4))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(dealt(&mut client_rx), [4]);
    }

    #[tokio::test]
    async fn meso_explosion_sets_off_only_the_mesos_it_may_and_caps_their_hits() {
        let (template, mut info) = mob(100, false);
        info.max_hp = 20_000;
        let mut mobs = MobSet::new([(template, info)]);
        let snail = mobs.spawn_all(Instant::now())[0];
        let mut drops = DropSet::default();
        let now = Instant::now();
        let mut put = |owner, x| {
            drops.spawn(
                vec![net::loot::Loot::Mesos(1000)],
                owner,
                Ownership::Owner,
                x,
                0,
                0,
                now,
            )[0]
            .object_id
        };
        // The player's, another's, the player's out of reach, and the
        // player's again.
        let ids = [put(1, -100), put(2, -150), put(1, 500), put(1, -200)];
        let (tx, rx) = mpsc::channel(16);
        let actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            mobs,
            drops,
            ReactorSet::default(),
            Duration::from_secs(60),
        );
        tokio::spawn(actor.run());
        let (handle, mut client_rx) = client(1);
        tx.send(FieldMessage::Join {
            handle,
            avatar: avatar(1, 0, 0),
            party: None,
        })
        .await
        .unwrap();
        opcodes(&mut client_rx).await;
        // A line from a 1,000 meso drop deals 8,000 at most at level 30.
        let explode = |lines: Vec<(usize, i32)>| FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::Attack {
                broadcast: Packet::new_empty(),
                skill: meso_explosion::MESO_EXPLOSION,
                counter: game::MobStat::PCounter,
                lines: 1,
                hits: vec![(snail, 0)],
                inflicts: Vec::new(),
                takes: Takes::default(),
                blast: Some(Blast {
                    drops: ids.to_vec(),
                    delay: 300,
                    lines: vec![lines],
                    x: 1000,
                    lt: (-300, -120),
                    rb: (0, 50),
                    left: true,
                }),
                pockets: Vec::new(),
            },
        };
        // What went off, when, and the snail's HP after.
        let outcome = |rx: &mut mpsc::Receiver<ServerMessage>| {
            use packet::io::read::PktRead;
            let (mut exploded, mut hp) = (Vec::new(), None);
            while let Ok(ServerMessage::SendPacket(packet)) = rx.try_recv() {
                let mut r = std::io::Cursor::new(&packet.bytes[2..]);
                if packet.opcode() == SendOpcode::RemoveItemFromMap as i16 {
                    assert_eq!(r.read_byte().unwrap(), 4);
                    exploded.push((r.read_int().unwrap(), r.read_short().unwrap()));
                } else if packet.opcode() == SendOpcode::ShowMonsterHp as i16 {
                    r.read_int().unwrap();
                    hp = Some(r.read_byte().unwrap());
                }
            }
            (exploded, hp)
        };

        // Only the player's two in reach go off, a tenth of a second apart
        // by their place in the list, and the line too big for its drop
        // counts for nothing.
        tx.send(explode(vec![(0, 8000), (1, 8000), (2, 8000), (3, 9000)]))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(
            outcome(&mut client_rx),
            (vec![(ids[0], 300), (ids[3], 600)], Some(60))
        );
        // Set off, they're gone.
        tx.send(explode(vec![(0, 8000)])).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(outcome(&mut client_rx), (vec![], Some(60)));
    }

    #[tokio::test]
    async fn occupants_see_each_other_join_and_leave() {
        let field = spawn_field();
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);

        field
            .send(FieldMessage::Join {
                handle: first,
                avatar: avatar(1, 10, 20),
                party: None,
            })
            .await
            .unwrap();
        field
            .send(FieldMessage::Join {
                handle: second,
                avatar: avatar(2, 30, 40),
                party: None,
            })
            .await
            .unwrap();

        // The newcomer sees the existing occupant; the occupant sees the newcomer.
        assert_eq!(
            next_opcode(&mut second_rx).await,
            SendOpcode::SpawnPlayer as i16
        );
        assert_eq!(
            next_opcode(&mut first_rx).await,
            SendOpcode::SpawnPlayer as i16
        );

        field
            .send(FieldMessage::Leave {
                client_id: 2,
                staying: false,
            })
            .await
            .unwrap();
        assert_eq!(
            next_opcode(&mut first_rx).await,
            SendOpcode::RemovePlayerFromMap as i16
        );
    }

    #[tokio::test]
    async fn a_hidden_gm_shows_only_to_gms() {
        let (_tx, rx) = mpsc::channel(16);
        let mut actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        let players = |packets: Vec<Packet>| -> Vec<i16> {
            packets
                .iter()
                .map(Packet::opcode)
                .filter(|&op| {
                    op == SendOpcode::SpawnPlayer as i16
                        || op == SendOpcode::RemovePlayerFromMap as i16
                })
                .collect()
        };
        let gm = |id, hidden| FieldAvatar {
            gm: true,
            hidden,
            ..avatar(id, 0, 0)
        };
        let (hider, mut hider_rx) = client(1);
        let (other_gm, mut other_gm_rx) = client(2);
        let (player, mut player_rx) = client(3);
        let joining = [
            (hider, gm(1, false)),
            (other_gm, gm(2, false)),
            (player, avatar(3, 0, 0)),
        ];
        for (handle, avatar) in joining {
            actor.handle_message(FieldMessage::Join {
                handle,
                avatar,
                party: None,
            });
        }
        let (spawn, remove) = (
            SendOpcode::SpawnPlayer as i16,
            SendOpcode::RemovePlayerFromMap as i16,
        );
        sent(&mut hider_rx);
        sent(&mut other_gm_rx);
        sent(&mut player_rx);

        let hide = |hidden| FieldMessage::UpdateAvatar {
            client_id: 1,
            avatar: gm(1, hidden),
            announce: Vec::new(),
        };
        actor.handle_message(hide(true));
        assert_eq!(players(sent(&mut player_rx)), [remove]);
        assert!(
            players(sent(&mut other_gm_rx)).is_empty(),
            "a GM still sees"
        );
        assert!(players(sent(&mut hider_rx)).is_empty());

        // Someone coming in does not see the hidden GM, and the GM's going
        // is no news to them.
        let (late, mut late_rx) = client(4);
        actor.handle_message(FieldMessage::Join {
            handle: late,
            avatar: avatar(4, 0, 0),
            party: None,
        });
        assert_eq!(players(sent(&mut late_rx)), [spawn, spawn]);
        sent(&mut player_rx);
        sent(&mut other_gm_rx);
        actor.handle_message(hide(false));
        assert_eq!(players(sent(&mut late_rx)), [spawn]);
        assert_eq!(players(sent(&mut player_rx)), [spawn]);
        assert!(players(sent(&mut other_gm_rx)).is_empty());

        actor.handle_message(hide(true));
        sent(&mut late_rx);
        actor.handle_message(FieldMessage::Leave {
            client_id: 1,
            staying: false,
        });
        assert!(players(sent(&mut late_rx)).is_empty());
        assert_eq!(players(sent(&mut other_gm_rx)), [remove]);
    }

    #[test]
    fn mobs_pass_by_a_hidden_gm() {
        let mut mobs = MobSet::new([mob(100, false)]);
        let snail = mobs.spawn_all(Instant::now())[0];
        let (_tx, rx) = mpsc::channel(16);
        let mut actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            mobs,
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_secs(60),
        );
        let gm = |hidden| FieldAvatar {
            gm: true,
            hidden,
            ..avatar(1, 0, 0)
        };
        let hide = |hidden| FieldMessage::UpdateAvatar {
            client_id: 1,
            avatar: gm(hidden),
            announce: Vec::new(),
        };
        // Whether each control packet gives the mob or takes it.
        let control = |rx: &mut _| -> Vec<bool> {
            sent(rx)
                .iter()
                .filter(|p| p.opcode() == SendOpcode::SpawnMonsterControl as i16)
                .map(|p| p.bytes[2] != 0)
                .collect()
        };
        let controller = |actor: &FieldActor| actor.mobs.get(snail).unwrap().controller;
        let (hider, mut hider_rx) = client(1);
        actor.handle_message(FieldMessage::Join {
            handle: hider.clone(),
            avatar: gm(true),
            party: None,
        });
        // Alone and hidden, the GM moves nothing, even what they hit.
        actor.provoke(1, &hider, snail, Instant::now());
        assert_eq!(controller(&actor), None);
        assert!(control(&mut hider_rx).is_empty());

        actor.handle_message(hide(false));
        assert_eq!(controller(&actor), Some(1));
        assert_eq!(control(&mut hider_rx), [true]);
        let (player, mut player_rx) = client(2);
        actor.handle_message(FieldMessage::Join {
            handle: player,
            avatar: avatar(2, 0, 0),
            party: None,
        });
        assert!(control(&mut player_rx).is_empty());

        // Hiding hands the GM's mobs on.
        actor.handle_message(hide(true));
        assert_eq!(controller(&actor), Some(2));
        assert_eq!(control(&mut hider_rx), [false]);
        assert_eq!(control(&mut player_rx), [true]);
        actor.handle_message(FieldMessage::Leave {
            client_id: 2,
            staying: false,
        });
        assert_eq!(controller(&actor), None);
        assert!(control(&mut hider_rx).is_empty());

        // Nor what the GM brings out.
        if net::mobs::info(100100).is_none() {
            return;
        }
        actor.handle_summon_mobs(1, vec![100100]);
        assert!(control(&mut hider_rx).is_empty());
        assert!(actor.mobs.live().all(|mob| mob.controller.is_none()));
    }

    #[tokio::test]
    async fn what_a_hidden_gm_does_shows_only_to_gms() {
        let (_tx, rx) = mpsc::channel(16);
        let mut actor = FieldActor::new(
            FieldKey::new(0, 100000000),
            rx,
            Vec::new(),
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        let gm = |id, hidden| FieldAvatar {
            gm: true,
            hidden,
            ..avatar(id, 0, 0)
        };
        let (hider, mut hider_rx) = client(1);
        let (other_gm, mut other_gm_rx) = client(2);
        let (player, mut player_rx) = client(3);
        let joining = [
            (hider, gm(1, true)),
            (other_gm, gm(2, false)),
            (player, avatar(3, 0, 0)),
        ];
        for (handle, avatar) in joining {
            actor.handle_message(FieldMessage::Join {
                handle,
                avatar,
                party: None,
            });
        }
        sent(&mut hider_rx);
        sent(&mut other_gm_rx);
        sent(&mut player_rx);
        let opcodes =
            |rx: &mut _| -> Vec<i16> { sent(rx).iter().map(Packet::opcode).collect::<Vec<_>>() };
        let request = |request| FieldMessage::FromClient { from: 1, request };

        // The line goes to the GM too; the rest to the others.
        actor.handle_message(FieldMessage::Chat {
            from: 1,
            packet: Packet::new(&[0xA2, 0x00]),
        });
        actor.handle_message(FieldMessage::Move {
            from: 1,
            packet: Packet::new(&[0xB9, 0x00]),
            start: (0, 0),
            movement_bytes: Vec::new(),
        });
        actor.handle_message(request(FieldRequest::Chair(3010000)));
        actor.handle_message(request(FieldRequest::Broadcast(Packet::new(&[0xC6, 0x00]))));
        actor.handle_message(request(FieldRequest::BroadcastAll(Packet::new(&[
            0x8A, 0x00,
        ]))));
        let chair = SendOpcode::ShowChair as i16;
        assert_eq!(opcodes(&mut other_gm_rx), [0xA2, 0xB9, chair, 0xC6, 0x8A]);
        assert_eq!(opcodes(&mut hider_rx), [0xA2]);
        assert_eq!(
            opcodes(&mut player_rx),
            [0x8A],
            "only what is about the map"
        );

        // The GM's summons come and go with the GM.
        if skills::data().is_err() {
            return;
        }
        let (spawn, remove) = (
            SendOpcode::SpawnSummon as i16,
            SendOpcode::RemoveSummon as i16,
        );
        actor.handle_message(summon(3111005, (0, 0), Duration::from_secs(60)));
        assert!(opcodes(&mut other_gm_rx).contains(&spawn));
        assert!(!opcodes(&mut player_rx).contains(&spawn));
        let (late, mut late_rx) = client(4);
        actor.handle_message(FieldMessage::Join {
            handle: late,
            avatar: avatar(4, 0, 0),
            party: None,
        });
        assert!(!opcodes(&mut late_rx).contains(&spawn));
        let hide = |hidden| FieldMessage::UpdateAvatar {
            client_id: 1,
            avatar: gm(1, hidden),
            announce: Vec::new(),
        };
        actor.handle_message(hide(false));
        assert!(opcodes(&mut player_rx).contains(&spawn));
        assert!(opcodes(&mut late_rx).contains(&spawn));
        actor.handle_message(hide(true));
        assert!(opcodes(&mut player_rx).contains(&remove));
        assert!(
            !opcodes(&mut other_gm_rx).contains(&remove),
            "a GM still sees"
        );
    }

    #[tokio::test]
    async fn a_stalled_client_cannot_block_the_field() {
        let field = spawn_field();
        let (tx, _never_read) = mpsc::channel(1);
        let kill = CancellationToken::new();
        let stalled = ClientHandle::new(1, 1, tx, kill.clone());
        let (healthy, mut healthy_rx) = client(2);

        field
            .send(FieldMessage::Join {
                handle: stalled,
                avatar: avatar(1, 0, 0),
                party: None,
            })
            .await
            .unwrap();
        field
            .send(FieldMessage::Join {
                handle: healthy,
                avatar: avatar(2, 0, 0),
                party: None,
            })
            .await
            .unwrap();
        for _ in 0..4 {
            field
                .send(FieldMessage::Chat {
                    from: 2,
                    packet: Packet::new(&[0x00, 0x00]),
                })
                .await
                .unwrap();
        }

        // The healthy client still receives everything; the stalled one is kicked.
        assert_eq!(
            next_opcode(&mut healthy_rx).await,
            SendOpcode::SpawnPlayer as i16
        );
        for _ in 0..4 {
            next_opcode(&mut healthy_rx).await;
        }
        assert!(kill.is_cancelled());
    }

    #[tokio::test]
    async fn quest_npcs_must_be_on_the_map_and_within_reach() {
        let (tx, rx) = mpsc::channel(16);
        let heena = FieldMapEntityNpc {
            object_id: 1_000_000_000,
            npc_id: 2101,
            x: 130,
            y: 185,
            flip: false,
            foothold: 0,
            rx0: 80,
            rx1: 180,
        };
        let actor = FieldActor::new(
            FieldKey::new(0, 10000),
            rx,
            vec![heena],
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        tokio::spawn(actor.run());
        let (near, mut near_rx) = client(1);
        let (far, mut far_rx) = client(2);
        tx.send(FieldMessage::Join {
            handle: near,
            avatar: avatar(1, 600, 185),
            party: None,
        })
        .await
        .unwrap();
        tx.send(FieldMessage::Join {
            handle: far,
            avatar: avatar(2, 1500, 185),
            party: None,
        })
        .await
        .unwrap();

        async fn checked(rx: &mut mpsc::Receiver<ServerMessage>) -> (u64, bool) {
            loop {
                match timeout(Duration::from_secs(1), rx.recv())
                    .await
                    .unwrap()
                    .unwrap()
                {
                    ServerMessage::QuestNpcChecked { request_id, nearby } => {
                        return (request_id, nearby)
                    }
                    ServerMessage::SendPacket(_) => continue,
                    other => panic!("unexpected {other:?}"),
                }
            }
        }

        tx.send(FieldMessage::QuestNpc {
            from: 1,
            npc_id: 2101,
            request_id: 7,
        })
        .await
        .unwrap();
        assert_eq!(checked(&mut near_rx).await, (7, true));
        // Sera is not on this copy of the map.
        tx.send(FieldMessage::QuestNpc {
            from: 1,
            npc_id: 2100,
            request_id: 8,
        })
        .await
        .unwrap();
        assert_eq!(checked(&mut near_rx).await, (8, false));
        tx.send(FieldMessage::QuestNpc {
            from: 2,
            npc_id: 2101,
            request_id: 9,
        })
        .await
        .unwrap();
        assert_eq!(checked(&mut far_rx).await, (9, false));
    }

    #[tokio::test]
    async fn one_player_picks_the_npcs_actions_for_everyone() {
        let (tx, rx) = mpsc::channel(16);
        let maria = FieldMapEntityNpc {
            object_id: 1_000_000_000,
            npc_id: 2100,
            x: 130,
            y: 185,
            flip: false,
            foothold: 0,
            rx0: 80,
            rx1: 180,
        };
        let actor = FieldActor::new(
            FieldKey::new(0, 50000),
            rx,
            vec![maria],
            MobSet::default(),
            DropSet::default(),
            ReactorSet::default(),
            Duration::from_millis(50),
        );
        tokio::spawn(actor.run());
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        for (handle, id) in [(first, 1), (second, 2)] {
            tx.send(FieldMessage::Join {
                handle,
                avatar: avatar(id, 100, 185),
                party: None,
            })
            .await
            .unwrap();
        }
        let control = SendOpcode::SpawnNpcRequestController as i16;
        assert!(opcodes(&mut first_rx).await.contains(&control));
        assert!(!opcodes(&mut second_rx).await.contains(&control));

        let action = |from| FieldMessage::FromClient {
            from,
            request: FieldRequest::NpcAction {
                object_id: 1_000_000_000,
                action: vec![2, 0xff],
            },
        };
        tx.send(action(2)).await.unwrap();
        assert!(opcodes(&mut first_rx).await.is_empty());
        tx.send(action(1)).await.unwrap();
        for rx in [&mut first_rx, &mut second_rx] {
            let packet = next_packet(rx).await;
            assert_eq!(packet.opcode(), SendOpcode::NpcAction as i16);
            assert_eq!(&packet.bytes[2..], &[0x00, 0xca, 0x9a, 0x3b, 2, 0xff]);
        }

        tx.send(FieldMessage::Leave {
            client_id: 1,
            staying: false,
        })
        .await
        .unwrap();
        assert!(opcodes(&mut second_rx).await.contains(&control));
    }

    #[tokio::test]
    async fn the_target_answers_a_char_info_request() {
        let field = spawn_field();
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        for (handle, id) in [(first, 1), (second, 2)] {
            field
                .send(FieldMessage::Join {
                    handle,
                    avatar: avatar(id, 0, 0),
                    party: None,
                })
                .await
                .unwrap();
        }
        next_opcode(&mut first_rx).await;
        next_opcode(&mut second_rx).await;

        // Character 3 is not here, so only the request for 2 reaches anyone;
        // the requester is unlocked instead.
        for character_id in [3, 2] {
            field
                .send(FieldMessage::FromClient {
                    from: 1,
                    request: FieldRequest::CharInfo { character_id },
                })
                .await
                .unwrap();
        }
        match timeout(Duration::from_secs(1), second_rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            ServerMessage::CharInfoRequested { requester } => assert_eq!(requester.client_id, 1),
            other => panic!("expected a char info request, got {other:?}"),
        }
        assert_eq!(
            next_opcode(&mut first_rx).await,
            SendOpcode::StatChange as i16
        );
        assert!(first_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn fame_reaches_its_target_only_on_the_givers_map() {
        let field = spawn_field();
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        for (handle, id) in [(first, 1), (second, 2)] {
            field
                .send(FieldMessage::Join {
                    handle,
                    avatar: avatar(id, 0, 0),
                    party: None,
                })
                .await
                .unwrap();
        }
        next_opcode(&mut first_rx).await;
        next_opcode(&mut second_rx).await;

        for target in [3, 2] {
            field
                .send(FieldMessage::FromClient {
                    from: 1,
                    request: FieldRequest::Fame {
                        target,
                        giver_name: "Al".into(),
                        up: true,
                    },
                })
                .await
                .unwrap();
        }
        match timeout(Duration::from_secs(1), second_rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            ServerMessage::Famed {
                giver,
                giver_name,
                up,
            } => {
                assert_eq!((giver.client_id, giver_name.as_str(), up), (1, "Al", true))
            }
            other => panic!("expected fame, got {other:?}"),
        }
        match timeout(Duration::from_secs(1), first_rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            ServerMessage::FameMissed { target } => assert_eq!(target, 3),
            other => panic!("expected a miss, got {other:?}"),
        }
        assert!(first_rx.try_recv().is_err());
    }

    /// Two players on a field with a long tick, everything they got so far
    /// read.
    async fn two_players() -> (
        mpsc::Sender<FieldMessage>,
        mpsc::Receiver<ServerMessage>,
        mpsc::Receiver<ServerMessage>,
    ) {
        let field = spawn_field();
        let (first, mut first_rx) = client(1);
        let (second, mut second_rx) = client(2);
        for (handle, id) in [(first, 1), (second, 2)] {
            field
                .send(FieldMessage::Join {
                    handle,
                    avatar: avatar(id, 0, 0),
                    party: None,
                })
                .await
                .unwrap();
        }
        opcodes(&mut first_rx).await;
        opcodes(&mut second_rx).await;
        (field, first_rx, second_rx)
    }

    #[tokio::test]
    async fn a_summoning_sacks_mobs_show_to_all_under_its_opener() {
        if net::mobs::info(100100).is_none() {
            return;
        }
        let (field, mut first_rx, mut second_rx) = two_players().await;
        field
            .send(FieldMessage::FromClient {
                from: 1,
                request: FieldRequest::SummonMobs(vec![100100, 100101]),
            })
            .await
            .unwrap();
        for _ in 0..2 {
            assert_eq!(
                next_opcode(&mut second_rx).await,
                SendOpcode::SpawnMonster as i16
            );
        }
        let seen = opcodes(&mut first_rx).await;
        let spawned = SendOpcode::SpawnMonster as i16;
        let controlled = SendOpcode::SpawnMonsterControl as i16;
        assert_eq!(seen, [spawned, spawned, controlled, controlled]);
    }

    #[tokio::test]
    async fn a_mound_takes_what_lies_on_it_in_turn_and_keeps_it_through_a_warp_within() {
        let Some(data) = net::reactors::template(2302003) else {
            return;
        };
        let spot = game_data::MapReactorTemplate {
            reactor_id: 2302003,
            x: 105,
            y: 164,
            flip: false,
            reactor_time: -1,
            name: String::new(),
        };
        let (_tx, rx) = mpsc::channel(16);
        let mut actor = FieldActor::new(
            FieldKey::new(0, 923000100),
            rx,
            Vec::new(),
            MobSet::default(),
            DropSet::default(),
            ReactorSet::new([(spot, data)]),
            Duration::from_millis(50),
        );
        let (handle, mut rx) = client(1);
        let join = |handle: &ClientHandle| FieldMessage::Join {
            handle: handle.clone(),
            avatar: avatar(1, 105, 78),
            party: None,
        };
        actor.handle_message(join(&handle));
        opcodes(&mut rx).await;
        let throw = |item_id| FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::Throw {
                loot: Loot::Dropped(game::Item::stack(item_id, 1)),
                vanish: false,
            },
        };
        let state = |actor: &FieldActor| actor.reactors.live().next().map(|r| r.state);

        // The book before the flame, which it wants first.
        actor.handle_message(throw(4001107));
        actor.handle_message(throw(4001108));
        let start = Instant::now();
        actor.on_tick(start + Duration::from_secs(4));
        assert_eq!(state(&actor), Some(0));
        actor.on_tick(start + Duration::from_secs(6));
        assert_eq!(state(&actor), Some(1));
        assert_eq!(actor.drops.all().count(), 1);

        // A warp within the room doesn't start it afresh.
        actor.handle_message(FieldMessage::Leave {
            client_id: 1,
            staying: true,
        });
        actor.handle_message(join(&handle));
        assert_eq!(state(&actor), Some(1));

        // Then it takes the book lying there already, and breaks.
        actor.on_tick(start + Duration::from_secs(12));
        assert_eq!(state(&actor), None);
        assert_eq!(actor.drops.all().count(), 0);
        let mut seen = Vec::new();
        let mut broken = false;
        while let Ok(Some(message)) = timeout(Duration::from_millis(100), rx.recv()).await {
            match message {
                ServerMessage::SendPacket(packet) => seen.push(packet.opcode()),
                ServerMessage::ReactorBroken { reactor_id, .. } => broken = reactor_id == 2302003,
                _ => {}
            }
        }
        assert!(broken);
        for opcode in [SendOpcode::HitReactor, SendOpcode::RemoveReactor] {
            assert!(seen.contains(&(opcode as i16)), "{seen:?}");
        }
        let removed = SendOpcode::RemoveItemFromMap as i16;
        assert_eq!(seen.iter().filter(|&&op| op == removed).count(), 2);

        // Left for good, the room is fresh for the next.
        actor.handle_message(FieldMessage::Leave {
            client_id: 1,
            staying: false,
        });
        assert_eq!(state(&actor), Some(0));
    }

    #[tokio::test]
    async fn the_cellions_drop_sets_off_its_spot_and_the_last_spot_drops_its_loot() {
        let spot = |reactor_id, x, y| {
            let data = net::reactors::template(reactor_id)?;
            let spot = game_data::MapReactorTemplate {
                reactor_id,
                x,
                y,
                flip: false,
                reactor_time: -1,
                name: String::new(),
            };
            Some((spot, data))
        };
        let (Some(first), Some(next), Some(_)) = (
            spot(2001002, -222, -520),
            spot(2001013, 140, -2731),
            net::mobs::info(reactors::CELLION),
        ) else {
            return;
        };
        let (_tx, rx) = mpsc::channel(16);
        let mut actor = FieldActor::new(
            FieldKey::new(0, 920010300),
            rx,
            Vec::new(),
            MobSet::default(),
            DropSet::default(),
            ReactorSet::new([first, next]),
            Duration::from_millis(50),
        );
        let (handle, mut rx) = client(1);
        actor.handle_message(FieldMessage::Join {
            handle,
            avatar: avatar(1, 0, -244),
            party: None,
        });
        opcodes(&mut rx).await;
        actor.salion = Some((1, 0));
        let cellion_drops = |x, y| FieldMessage::SpawnLoot {
            owner: 1,
            ownership: Ownership::Owner,
            loot: vec![Loot::Item {
                item_id: 4001074,
                quantity: 1,
                quest: None,
            }],
            x,
            y,
            dropper: 1,
        };
        let broken = |rx: &mut mpsc::Receiver<ServerMessage>| {
            std::iter::from_fn(|| rx.try_recv().ok()).any(|message| {
                matches!(message, ServerMessage::ReactorBroken { reactor_id, .. }
                    if (2001002..=2001015).contains(&reactor_id))
            })
        };

        // The first Cellion's drop breaks its spot, and the next comes out
        // at 2001013's.
        actor.handle_message(cellion_drops(-222, -520));
        let start = Instant::now();
        actor.on_tick(start + Duration::from_secs(6));
        assert_eq!(actor.reactors.live().count(), 1);
        assert_eq!(actor.drops.all().count(), 0);
        let cellions: Vec<_> = actor
            .mobs
            .live()
            .map(|mob| (mob.info.mob_id, mob.x))
            .collect();
        assert_eq!(cellions, [(reactors::CELLION, 140)]);
        assert!(!broken(&mut rx));

        // That one's is the last: its spot drops the loot.
        actor.handle_message(cellion_drops(140, -2731));
        actor.on_tick(start + Duration::from_secs(12));
        assert_eq!(actor.reactors.live().count(), 0);
        assert_eq!(actor.mobs.live().count(), 1);
        assert!(broken(&mut rx));
        assert_eq!(actor.salion, Some((1, 1)));

        // A quest drop doesn't lie on a reactor.
        let quest_drop = Loot::Item {
            item_id: 4001074,
            quantity: 1,
            quest: Some(1),
        };
        assert_eq!(reactor_item(&quest_drop), None);
    }

    fn summon(skill_id: i32, at: (i16, i16), lasts: Duration) -> FieldMessage {
        FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::Summon {
                skill_id,
                level: 20,
                at,
                left: true,
                lasts,
                appear: true,
            },
        }
    }

    #[tokio::test]
    async fn a_puppet_shows_to_all_takes_its_owners_hits_and_breaks() {
        if skills::data().is_err() {
            return;
        }
        let (field, mut first_rx, mut second_rx) = two_players().await;
        let puppet = summons::PUPPETS[0];
        // Too far from its owner: it goes where they stand.
        field
            .send(summon(puppet, (900, 0), Duration::from_secs(60)))
            .await
            .unwrap();
        let spawned = next_packet(&mut second_rx).await;
        assert_eq!(spawned.opcode(), SendOpcode::SpawnSummon as i16);
        let object_id = i32::from_le_bytes(spawned.bytes[6..10].try_into().unwrap());
        assert_eq!(&spawned.bytes[16..20], [0, 0, 0, 0]);
        assert_eq!(
            next_opcode(&mut first_rx).await,
            SendOpcode::SpawnSummon as i16
        );

        // Someone arriving sees it after its owner.
        let (third, mut third_rx) = client(3);
        field
            .send(FieldMessage::Join {
                handle: third,
                avatar: avatar(3, 0, 0),
                party: None,
            })
            .await
            .unwrap();
        let seen = opcodes(&mut third_rx).await;
        let at = |op: SendOpcode| {
            let op = op as i16;
            seen.iter().position(|&seen| seen == op).unwrap()
        };
        assert!(at(SendOpcode::SpawnPlayer) < at(SendOpcode::SpawnSummon));
        opcodes(&mut first_rx).await;
        opcodes(&mut second_rx).await;

        let hit = |from, damage| FieldMessage::FromClient {
            from,
            request: FieldRequest::SummonDamage(SummonDamage {
                object_id,
                attack: -1,
                damage,
                mob: Some((100100, false)),
            }),
        };
        // Only its owner's client says what hurt it.
        field.send(hit(2, 1_000_000)).await.unwrap();
        field.send(hit(1, 10)).await.unwrap();
        assert_eq!(
            next_opcode(&mut second_rx).await,
            SendOpcode::DamageSummon as i16
        );
        let hp = summons::hp(&skills::at_level(puppet, 20).unwrap()).unwrap();
        field.send(hit(1, hp)).await.unwrap();
        assert_eq!(
            next_opcode(&mut second_rx).await,
            SendOpcode::DamageSummon as i16
        );
        let removed = next_packet(&mut second_rx).await;
        assert_eq!(removed.opcode(), SendOpcode::RemoveSummon as i16);
        assert_eq!(removed.bytes[10], Gone::Broken as u8);
        assert_eq!(
            next_opcode(&mut first_rx).await,
            SendOpcode::RemoveSummon as i16
        );
        match timeout(Duration::from_secs(1), first_rx.recv()).await {
            Ok(Some(ServerMessage::SummonBroken { skill_id })) => assert_eq!(skill_id, puppet),
            other => panic!("expected the puppet broken, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_summon_attacks_within_its_limits_and_goes_when_its_time_is_up() {
        if skills::data().is_err() {
            return;
        }
        let (field, mut first_rx, mut second_rx) = two_players().await;
        let hawk = 3111005;
        field
            .send(summon(hawk, (0, 0), Duration::from_millis(300)))
            .await
            .unwrap();
        let spawned = next_packet(&mut second_rx).await;
        let object_id = i32::from_le_bytes(spawned.bytes[6..10].try_into().unwrap());
        opcodes(&mut first_rx).await;

        let attack = |damage: &[i32]| FieldMessage::FromClient {
            from: 1,
            request: FieldRequest::SummonAttack {
                attack: SummonAttack {
                    object_id,
                    action: 0x84,
                    hits: damage
                        .iter()
                        .enumerate()
                        .map(|(i, &damage)| summons::SummonHit {
                            object_id: 2_000_000_000 + i as i32,
                            action: 6,
                            damage,
                        })
                        .collect(),
                },
                limits: vec![Limit {
                    skill_id: hawk,
                    mob_count: 1,
                    max_damage: 100,
                    inflict: None,
                }],
            },
        };
        field.send(attack(&[101])).await.unwrap();
        field.send(attack(&[50, 50])).await.unwrap();
        field.send(attack(&[100])).await.unwrap();
        // Only the last keeps to the limits; its owner already saw it.
        assert_eq!(
            opcodes(&mut second_rx).await,
            [SendOpcode::SummonAttack as i16]
        );
        assert!(first_rx.try_recv().is_err());

        let removed = next_packet(&mut second_rx).await;
        assert_eq!(removed.opcode(), SendOpcode::RemoveSummon as i16);
        assert_eq!(removed.bytes[10], Gone::Expired as u8);
    }
}
