//! Chat commands (LEDGER row 205): a line starting with `!` is a GM's
//! command, one starting with `@` a command anyone may use. v83 has neither
//! (its client keeps only `/`); these are ours, as servers of the time had
//! them, so their text is ours too and lives here rather than in the
//! content pack.
//!
//! Each command needs a tier of GM (`gm_level`, set by SQL): 1 helps
//! (warps, Hide), 2 runs the game (items, mesos, NX, levels, notices), 3
//! administers. A GM's `!` line is never chat; a player's is. Every `!`
//! line a GM sends is logged at info under the `gm_audit` target.
//! `RUSTMS_GM_COMMANDS=0` turns all of them off, so every line is chat.

use crate::error::NetworkError;
use crate::handler::{BroadcastScope, Discipline, WorldAction, WorldContext};
use crate::items;
use crate::packet::build::world::{
    drop::build_meso_gain_in_chat,
    effect::{build_foreign_effect, CharEffect},
    inventory,
    messaging::{build_broadcast, Broadcast},
    skill::build_update_skill,
    stat::{self, StatUpdate},
};
use crate::packet::handle::world::{set_hidden, warp_to_spawn_point, BLOCK_DAYS};
use db::cash_shop::CashType;
use db::coupon::Reward;
use tracing::info;

/// Anyone.
pub const PLAYER: i16 = 0;
/// Warps and Hide.
pub const HELPER: i16 = 1;
/// Items, mesos, NX, levels and notices.
pub const GM: i16 = 2;
/// Bans and the server.
pub const ADMIN: i16 = 3;

/// The tier a `gm_level` gives: levels above 3 are admins too.
pub fn tier(gm_level: i16) -> i16 {
    gm_level.clamp(PLAYER, ADMIN)
}

/// What a command did: the actions to take, or a line telling the user why
/// nothing was done.
type Outcome = Result<Result<Vec<WorldAction>, String>, NetworkError>;

struct Command {
    prefix: char,
    name: &'static str,
    tier: i16,
    /// The arguments, as `!help` shows them.
    usage: &'static str,
    run: fn(&mut WorldContext, &[&str]) -> Outcome,
}

const COMMANDS: &[Command] = &[
    Command {
        prefix: '@',
        name: "help",
        tier: PLAYER,
        usage: "",
        run: help,
    },
    Command {
        prefix: '@',
        name: "dispose",
        tier: PLAYER,
        usage: "",
        run: dispose,
    },
    Command {
        prefix: '!',
        name: "help",
        tier: HELPER,
        usage: "",
        run: help,
    },
    Command {
        prefix: '!',
        name: "whereami",
        tier: HELPER,
        usage: "",
        run: whereami,
    },
    Command {
        prefix: '!',
        name: "warp",
        tier: HELPER,
        usage: "<map id> [portal]",
        run: warp,
    },
    Command {
        prefix: '!',
        name: "hide",
        tier: HELPER,
        usage: "",
        run: hide,
    },
    Command {
        prefix: '!',
        name: "unhide",
        tier: HELPER,
        usage: "",
        run: unhide,
    },
    Command {
        prefix: '!',
        name: "heal",
        tier: HELPER,
        usage: "",
        run: heal,
    },
    Command {
        prefix: '!',
        name: "item",
        tier: GM,
        usage: "<item id> [quantity]",
        run: item,
    },
    Command {
        prefix: '!',
        name: "meso",
        tier: GM,
        usage: "<amount>",
        run: meso,
    },
    Command {
        prefix: '!',
        name: "exp",
        tier: GM,
        usage: "<amount>",
        run: exp,
    },
    Command {
        prefix: '!',
        name: "gp",
        tier: GM,
        usage: "<amount>",
        run: guild_points,
    },
    Command {
        prefix: '!',
        name: "level",
        tier: GM,
        usage: "<level>",
        run: level,
    },
    Command {
        prefix: '!',
        name: "job",
        tier: GM,
        usage: "<job id>",
        run: job,
    },
    Command {
        prefix: '!',
        name: "ap",
        tier: GM,
        usage: "<amount>",
        run: ap,
    },
    Command {
        prefix: '!',
        name: "sp",
        tier: GM,
        usage: "<amount>",
        run: sp,
    },
    Command {
        prefix: '!',
        name: "skill",
        tier: GM,
        usage: "<skill id> [level]",
        run: skill,
    },
    Command {
        prefix: '!',
        name: "givenx",
        tier: GM,
        usage: "[nx|mp|np] [name] <amount>",
        run: give_nx,
    },
    Command {
        prefix: '!',
        name: "coupon",
        tier: GM,
        usage: "<nx=N|mp=N|np=N|meso=N|item id[xN]>... [days=N] [code=CODE]",
        run: coupon,
    },
    Command {
        prefix: '!',
        name: "notice",
        tier: GM,
        usage: "<text>",
        run: notice,
    },
    Command {
        prefix: '!',
        name: "dc",
        tier: GM,
        usage: "<name>",
        run: disconnect,
    },
    Command {
        prefix: '!',
        name: "jail",
        tier: GM,
        usage: "<name> [minutes]",
        run: jail,
    },
    Command {
        prefix: '!',
        name: "unjail",
        tier: GM,
        usage: "<name>",
        run: unjail,
    },
    Command {
        prefix: '!',
        name: "mute",
        tier: GM,
        usage: "<name> [minutes] [reason]",
        run: mute,
    },
    Command {
        prefix: '!',
        name: "unmute",
        tier: GM,
        usage: "<name>",
        run: unmute,
    },
    Command {
        prefix: '!',
        name: "ban",
        tier: ADMIN,
        usage: "<name> [days] [reason] [note]",
        run: ban,
    },
    Command {
        prefix: '!',
        name: "unban",
        tier: ADMIN,
        usage: "<name>",
        run: unban,
    },
];

/// Run `line` if it is a command for its sender. `None` if it is chat: a
/// line without a prefix, a player's `!` line, an `@` line that names no
/// command, or any line while commands are off.
pub fn run(ctx: &mut WorldContext, line: &str) -> Option<Result<Vec<WorldAction>, NetworkError>> {
    let prefix = line.chars().next().filter(|c| matches!(c, '!' | '@'))?;
    let tier = tier(ctx.player.gm_level);
    if !crate::policy::gm_commands_enabled() || (prefix == '!' && tier == PLAYER) {
        return None;
    }
    let mut words = line[1..].split_whitespace();
    let name = words.next().unwrap_or("");
    let args: Vec<&str> = words.collect();
    let command = COMMANDS
        .iter()
        .find(|c| c.prefix == prefix && c.tier <= tier && c.name.eq_ignore_ascii_case(name));
    // "@_@" and the like are chat.
    if prefix == '@' && command.is_none() {
        return None;
    }
    let outcome = match command {
        Some(command) => (command.run)(ctx, &args),
        None => Ok(Err(format!(
            "Unknown command. Type {prefix}help for a list."
        ))),
    };
    if prefix == '!' {
        let done = matches!(outcome, Ok(Ok(_)));
        info!(
            target: "gm_audit",
            character_id = ctx.player.stats.id,
            character = %ctx.player.stats.name,
            gm_level = ctx.player.gm_level,
            line,
            done,
            "GM command"
        );
    }
    Some(outcome.and_then(|outcome| match outcome {
        Ok(actions) => Ok(actions),
        Err(why) => Ok(vec![say(&why)?]),
    }))
}

/// Why `!warp` and `!heal` do nothing for the dead, who leave by the
/// client's own revive.
const DEAD: &str = "Not while dead.";

/// A line in the user's chat.
fn say(text: &str) -> Result<WorldAction, NetworkError> {
    Ok(WorldAction::Reply(build_broadcast(Broadcast::Pink, text)?))
}

fn said(text: &str) -> Outcome {
    Ok(Ok(vec![say(text)?]))
}

/// How to call the `!` command `name`.
fn usage(name: &str) -> String {
    let usage = COMMANDS
        .iter()
        .find(|c| c.prefix == '!' && c.name == name)
        .map_or("", |c| c.usage);
    format!("Usage: !{name} {usage}")
}

/// The one argument a command takes, as a number.
fn number<T: std::str::FromStr>(args: &[&str]) -> Option<T> {
    match args {
        [value] => value.parse().ok(),
        _ => None,
    }
}

/// The commands the user may run, `@` ones first.
fn help(ctx: &mut WorldContext, _: &[&str]) -> Outcome {
    let tier = tier(ctx.player.gm_level);
    let mut actions = Vec::new();
    for command in COMMANDS.iter().filter(|c| c.tier <= tier) {
        let line = format!("{}{} {}", command.prefix, command.name, command.usage);
        actions.push(say(line.trim_end())?);
    }
    Ok(Ok(actions))
}

/// Let the client act again, if a dialog left it stuck, and close any
/// dialog, shop or storage the server still has open for it.
fn dispose(_: &mut WorldContext, _: &[&str]) -> Outcome {
    Ok(Ok(vec![
        WorldAction::CloseDialogs,
        WorldAction::Reply(stat::build_stat_update(true, &[])?),
    ]))
}

fn whereami(ctx: &mut WorldContext, _: &[&str]) -> Outcome {
    said(&format!(
        "Map {}, channel {}.",
        ctx.player.stats.map_id,
        ctx.channel_id + 1
    ))
}

/// To a map's portal by name, or its first.
fn warp(ctx: &mut WorldContext, args: &[&str]) -> Outcome {
    let (map_id, portal) = match args {
        [map_id] => (map_id, None),
        [map_id, portal] => (map_id, Some(*portal)),
        _ => return Ok(Err(usage("warp"))),
    };
    let Ok(map_id) = map_id.parse::<i32>() else {
        return Ok(Err(usage("warp")));
    };
    if ctx.player.stats.hp <= 0 {
        return Ok(Err(DEAD.into()));
    }
    let Some(field) = crate::game_data::get()?.field(map_id) else {
        return Ok(Err(format!("There is no map {map_id}.")));
    };
    let spawn_point = match portal {
        None => 0,
        Some(name) => match field.portal_by_name(name) {
            Some(portal) => u8::try_from(portal.id).unwrap_or(0),
            None => return Ok(Err(format!("Map {map_id} has no portal {name}."))),
        },
    };
    Ok(Ok(warp_to_spawn_point(
        ctx.player,
        map_id,
        spawn_point,
        ctx.channel_id,
    )?))
}

fn hide(ctx: &mut WorldContext, _: &[&str]) -> Outcome {
    Ok(Ok(set_hidden(ctx.player, true)?.actions))
}

fn unhide(ctx: &mut WorldContext, _: &[&str]) -> Outcome {
    Ok(Ok(set_hidden(ctx.player, false)?.actions))
}

/// Full HP and MP.
fn heal(ctx: &mut WorldContext, _: &[&str]) -> Outcome {
    let player = &mut ctx.player;
    if player.stats.hp <= 0 {
        return Ok(Err(DEAD.into()));
    }
    let hp = player.add_hp(player.effective_max_hp());
    let mp = player.add_mp(player.effective_max_mp());
    let updates = [StatUpdate::Hp(hp), StatUpdate::Mp(mp)];
    Ok(Ok(vec![WorldAction::Reply(stat::build_stat_update(
        true, &updates,
    )?)]))
}

/// Into the bags. Pets and other cash items need a cash serial, which only
/// the Cash Shop gives.
fn item(ctx: &mut WorldContext, args: &[&str]) -> Outcome {
    let (item_id, quantity) = match args {
        [item_id] => (item_id.parse::<i32>().ok(), Some(1)),
        [item_id, quantity] => (item_id.parse().ok(), quantity.parse::<i16>().ok()),
        _ => (None, None),
    };
    let (Some(item_id), Some(quantity @ 1..)) = (item_id, quantity) else {
        return Ok(Err(usage("item")));
    };
    if items::info(item_id).is_none() {
        return Ok(Err(format!("There is no item {item_id}.")));
    }
    if crate::coupon::for_cash_inventory(item_id) {
        return Ok(Err("Cash items come from the Cash Shop.".into()));
    }
    if items::refuses_only(&ctx.player.inventory, item_id, quantity.into()) {
        return Ok(Err("That item is one of a kind.".into()));
    }
    let Ok(changes) = items::give(ctx.player.inventory_mut(), item_id, quantity) else {
        return Ok(Err("There is no room for it.".into()));
    };
    ctx.player.mark_dirty();
    Ok(Ok(vec![
        WorldAction::Reply(inventory::build_inventory_operation(true, &changes)?),
        WorldAction::Reply(inventory::build_item_gain_in_chat(
            item_id,
            quantity.into(),
        )?),
    ]))
}

/// Mesos gained, or lost for a negative amount, down to none.
fn meso(ctx: &mut WorldContext, args: &[&str]) -> Outcome {
    let Some(amount) = number::<i32>(args) else {
        return Ok(Err(usage("meso")));
    };
    let meso = i64::from(ctx.player.meso) + i64::from(amount);
    let meso = meso.clamp(0, i64::from(i32::MAX)) as i32;
    let gained = meso - ctx.player.meso;
    ctx.player.meso = meso;
    ctx.player.mark_dirty();
    Ok(Ok(vec![
        WorldAction::Reply(stat::build_stat_update(true, &[StatUpdate::Meso(meso)])?),
        WorldAction::Reply(build_meso_gain_in_chat(gained)?),
    ]))
}

/// EXP gained as from a monster, levels and all.
fn exp(_: &mut WorldContext, args: &[&str]) -> Outcome {
    match number::<i32>(args) {
        Some(amount @ 1..) => Ok(Ok(vec![WorldAction::GainExp(amount)])),
        _ => Ok(Err(usage("exp"))),
    }
}

/// Points for the GM's guild, or taken from it if negative.
fn guild_points(ctx: &mut WorldContext, args: &[&str]) -> Outcome {
    if ctx.player.guild.is_none() {
        return Ok(Err("You are not in a guild.".to_string()));
    }
    match number::<i32>(args) {
        Some(amount) if amount != 0 => Ok(Ok(vec![WorldAction::Guild(
            crate::GuildRequest::GainPoints { amount },
        )])),
        _ => Ok(Err(usage("gp"))),
    }
}

/// The level, with no EXP into it. Nothing else a level brings (AP, SP,
/// HP) comes with it; `!ap` and `!sp` set those.
fn level(ctx: &mut WorldContext, args: &[&str]) -> Outcome {
    let Some(level @ 1..=game::exp::MAX_LEVEL) = number::<u8>(args) else {
        return Ok(Err(usage("level")));
    };
    let stats = &mut ctx.player.stats;
    let raised = level > stats.level;
    stats.level = level;
    stats.exp = 0;
    let update = stat::build_stat_update(true, &[StatUpdate::Level(level), StatUpdate::Exp(0)])?;
    ctx.player.mark_dirty();
    let mut announce = Vec::new();
    if raised {
        announce.push(build_foreign_effect(ctx.client_id, CharEffect::LevelUp)?);
    }
    Ok(Ok(vec![
        WorldAction::Reply(update),
        WorldAction::AvatarChanged(announce),
    ]))
}

/// The job, with nothing an advancement brings (SP, HP, skills).
fn job(ctx: &mut WorldContext, args: &[&str]) -> Outcome {
    let Some(job) = number::<i16>(args).filter(|&job| known_job(job)) else {
        return Ok(Err(usage("job")));
    };
    let changed = ctx.player.stats.job != job;
    ctx.player.stats.job = job;
    ctx.player.mark_dirty();
    let mut announce = Vec::new();
    if changed {
        announce.push(build_foreign_effect(ctx.client_id, CharEffect::JobChange)?);
    }
    Ok(Ok(vec![
        WorldAction::Reply(stat::build_stat_update(true, &[StatUpdate::Job(job)])?),
        WorldAction::AvatarChanged(announce),
    ]))
}

/// Whether v83 has `job`: a beginner of the three lines, a GM, or an
/// advancement of one of their branches (an Explorer's warrior and
/// magician take three second jobs, the rest two; Cygnus Knights and the
/// Aran one, and Cygnus Knights stop at the third advancement).
fn known_job(job: i16) -> bool {
    if matches!(job, 0 | 900 | 910 | 1000 | 2000) {
        return true;
    }
    if job < 0 {
        return false;
    }
    let seconds = match (job / 1000, job % 1000 / 100) {
        (0, 1 | 2) => 3,
        (0, 3..=5) => 2,
        (1, 1..=5) | (2, 1) => 1,
        _ => 0,
    };
    let (second, third) = (job % 100 / 10, job % 10);
    let last = if job / 1000 == 1 { 1 } else { 2 };
    seconds > 0
        && second <= seconds
        && (if second == 0 {
            third == 0
        } else {
            third <= last
        })
}

fn ap(ctx: &mut WorldContext, args: &[&str]) -> Outcome {
    let Some(ap @ 0..) = number::<i16>(args) else {
        return Ok(Err(usage("ap")));
    };
    ctx.player.stats.ap = ap;
    ctx.player.mark_dirty();
    Ok(Ok(vec![WorldAction::Reply(stat::build_stat_update(
        true,
        &[StatUpdate::Ap(ap)],
    )?)]))
}

fn sp(ctx: &mut WorldContext, args: &[&str]) -> Outcome {
    let Some(sp @ 0..) = number::<i16>(args) else {
        return Ok(Err(usage("sp")));
    };
    ctx.player.stats.sp = sp;
    ctx.player.mark_dirty();
    Ok(Ok(vec![WorldAction::Reply(stat::build_stat_update(
        true,
        &[StatUpdate::Sp(sp)],
    )?)]))
}

/// A skill at `level`, its highest if none is given, whatever the job;
/// level 0 takes it away. The only way to the event skills (1013, 1017 and
/// the like, report b183), which nothing in v83 hands out.
fn skill(ctx: &mut WorldContext, args: &[&str]) -> Outcome {
    let (id, level) = match args {
        [id] => (id.parse::<i32>().ok(), Some(None)),
        [id, level] => (id.parse().ok(), level.parse::<i32>().ok().map(Some)),
        _ => (None, None),
    };
    let (Some(info), Some(level)) = (id.and_then(crate::skills::info), level) else {
        return Ok(Err(usage("skill")));
    };
    let level = level.unwrap_or(info.max_level);
    if !(0..=info.max_level).contains(&level) {
        return Ok(Err(format!("Levels go from 0 to {}.", info.max_level)));
    }
    let record = game::SkillRecord {
        skill_id: info.skill_id,
        level,
        // A fourth-job skill shows only up to its master level.
        master_level: if game::skill::is_fourth_job(info.skill_id) {
            info.max_level
        } else {
            0
        },
        expiration: None,
    };
    if level == 0 {
        ctx.player.skills.remove(record.skill_id);
    } else {
        ctx.player.skills.set(record.clone());
    }
    ctx.player.mark_dirty();
    Ok(Ok(vec![WorldAction::Reply(build_update_skill(&record)?)]))
}

/// A notice to the whole world.
/// NX for a character's account, the GM's own if no name is given: NX
/// Credit unless `mp` (Maple Points) or `np` (NX Prepaid) comes first. A
/// negative amount takes it away.
fn give_nx(_: &mut WorldContext, args: &[&str]) -> Outcome {
    let (cash_type, args) = match args {
        [kind, rest @ ..] if !rest.is_empty() => match kind.to_ascii_lowercase().as_str() {
            "nx" => (CashType::Credit, rest),
            "mp" => (CashType::MaplePoints, rest),
            "np" => (CashType::Prepaid, rest),
            _ => (CashType::Credit, args),
        },
        _ => (CashType::Credit, args),
    };
    let (target, amount) = match args {
        [amount] => (None, amount),
        [name, amount] => (Some(name.to_string()), amount),
        _ => return Ok(Err(usage("givenx"))),
    };
    match amount.parse::<i32>() {
        Ok(amount) if amount != 0 => Ok(Ok(vec![WorldAction::GiveCash {
            target,
            cash_type,
            amount,
        }])),
        _ => Ok(Err(usage("givenx"))),
    }
}

/// The longest a coupon may last.
const COUPON_DAYS: u32 = 3650;

/// A Cash Shop coupon (`net::coupon`) for players to redeem once: NX,
/// mesos, and items, cash ones included, which only a coupon can give.
fn coupon(_: &mut WorldContext, args: &[&str]) -> Outcome {
    let (mut code, mut days, mut rewards) = (None, None, Vec::new());
    for arg in args {
        let arg = arg.to_ascii_lowercase();
        let amount = |value: &str| value.parse::<i32>().ok().filter(|&n| n > 0);
        let reward = match arg.split_once('=') {
            Some(("code", value)) => match crate::coupon::normalize(value) {
                Some(value) => {
                    code = Some(value);
                    continue;
                }
                None => None,
            },
            Some(("days", value)) => match value.parse() {
                Ok(value @ 1..=COUPON_DAYS) => {
                    days = Some(value);
                    continue;
                }
                _ => None,
            },
            Some(("nx", value)) => amount(value).map(Reward::Credit),
            Some(("mp", value)) => amount(value).map(Reward::MaplePoints),
            Some(("np", value)) => amount(value).map(Reward::Prepaid),
            Some(("meso", value)) => amount(value).map(Reward::Mesos),
            Some(_) => None,
            None => {
                let (item_id, quantity) = arg.split_once('x').unwrap_or((&arg, "1"));
                match (item_id.parse::<i32>(), quantity.parse::<i16>()) {
                    (Ok(item_id), Ok(quantity @ 1..)) => {
                        if items::info(item_id).is_none() {
                            return Ok(Err(format!("There is no item {item_id}.")));
                        }
                        Some(Reward::Item {
                            item_id,
                            quantity: quantity.into(),
                        })
                    }
                    _ => None,
                }
            }
        };
        match reward {
            Some(reward) => rewards.push(reward),
            None => return Ok(Err(usage("coupon"))),
        }
    }
    if rewards.is_empty() {
        return Ok(Err(usage("coupon")));
    }
    Ok(Ok(vec![WorldAction::MakeCoupon {
        code,
        days,
        rewards,
    }]))
}

fn notice(_: &mut WorldContext, args: &[&str]) -> Outcome {
    if args.is_empty() {
        return Ok(Err(usage("notice")));
    }
    let packet = build_broadcast(Broadcast::Notice, &args.join(" "))?;
    Ok(Ok(vec![WorldAction::Broadcast {
        scope: BroadcastScope::World,
        packet,
    }]))
}

/// `!dc`: disconnect a character.
fn disconnect(_: &mut WorldContext, args: &[&str]) -> Outcome {
    match args {
        [name] => Ok(Ok(vec![WorldAction::Discipline(Discipline::Disconnect {
            target: name.to_string(),
        })])),
        _ => Ok(Err(usage("dc"))),
    }
}

/// The jail: "Cellar: Camp Conference Room", which has no way out.
pub const JAIL_MAP: i32 = 300000012;
/// Where a jailed character comes out: the conference room's door, in the
/// cellar outside (as Cosmic's `jail_out.js` lets them out).
pub const JAIL_EXIT: (i32, &str) = (300000010, "in01");
/// How long `!jail` keeps a character in by default (as Cosmic's).
const JAIL_MINUTES: u32 = 5;
/// The most `!jail` adds at once: a year.
const JAIL_MINUTES_MAX: u32 = 365 * 24 * 60;

/// `!jail`: hold a character in the jail map for some minutes more, online
/// or not.
fn jail(_: &mut WorldContext, args: &[&str]) -> Outcome {
    let (name, minutes) = match args {
        [name] => (name, JAIL_MINUTES),
        [name, minutes] => match minutes.parse::<u32>() {
            Ok(minutes @ 1..) => (name, minutes.min(JAIL_MINUTES_MAX)),
            _ => return Ok(Err(usage("jail"))),
        },
        _ => return Ok(Err(usage("jail"))),
    };
    Ok(Ok(vec![WorldAction::Discipline(Discipline::Jail {
        target: name.to_string(),
        minutes,
    })]))
}

/// `!unjail`: let a character out of jail.
fn unjail(_: &mut WorldContext, args: &[&str]) -> Outcome {
    match args {
        [name] => Ok(Ok(vec![WorldAction::Discipline(Discipline::Release {
            target: name.to_string(),
        })])),
        _ => Ok(Err(usage("unjail"))),
    }
}

/// How long `!mute` blocks chat by default: an hour.
const MUTE_MINUTES: u32 = 60;

/// `!mute`: block a character's account from chatting, for an hour unless
/// a number of minutes comes first (at most a year), giving a reason (a
/// ChatBlockReason.img type or number; "foul" if none).
fn mute(_: &mut WorldContext, args: &[&str]) -> Outcome {
    let Some((name, mut rest)) = args.split_first() else {
        return Ok(Err(usage("mute")));
    };
    let mut minutes = MUTE_MINUTES;
    if let Some((first, after)) = rest.split_first() {
        if let Ok(n) = first.parse::<u32>() {
            if n == 0 {
                return Ok(Err(usage("mute")));
            }
            minutes = n.min(JAIL_MINUTES_MAX);
            rest = after;
        }
    }
    let reason = match rest {
        [] => 1,
        [reason] => match reason.parse::<u8>() {
            Ok(n @ 1..=6) => n,
            _ => match crate::chat_block::REASONS
                .iter()
                .position(|r| r.eq_ignore_ascii_case(reason))
            {
                Some(i) => i as u8 + 1,
                None => return Ok(Err(usage("mute"))),
            },
        },
        _ => return Ok(Err(usage("mute"))),
    };
    Ok(Ok(vec![WorldAction::Discipline(Discipline::Mute {
        target: name.to_string(),
        minutes,
        reason,
    })]))
}

/// `!unmute`: lift a character's account's chat block.
fn unmute(_: &mut WorldContext, args: &[&str]) -> Outcome {
    match args {
        [name] => Ok(Ok(vec![WorldAction::Discipline(Discipline::Unmute {
            target: name.to_string(),
        })])),
        _ => Ok(Err(usage("unmute"))),
    }
}

/// The reasons a block may give, by the `type` of their Etc/BlockReason.img
/// node, whose number is the place here plus one (as v83's `/block` finds
/// them, report b175 s4).
const BLOCK_REASONS: [&str; 13] = [
    "HACK",
    "BOT",
    "AD",
    "HARASS",
    "CURSE",
    "SCAM",
    "MISCONDUCT",
    "SELL",
    "ICASH",
    "TEMP",
    "GM",
    "IPROGRAM",
    "MEGAPHONE",
];

/// `!ban`: block a character's account, for good unless a number of days
/// comes first, giving a reason (a BlockReason.img type or number) and a
/// note for the record if they follow.
fn ban(_: &mut WorldContext, args: &[&str]) -> Outcome {
    let Some((name, mut rest)) = args.split_first() else {
        return Ok(Err(usage("ban")));
    };
    let mut days = None;
    if let Some((first, after)) = rest.split_first() {
        if let Ok(n) = first.parse::<u32>() {
            if n == 0 {
                return Ok(Err(usage("ban")));
            }
            days = Some(n.min(BLOCK_DAYS));
            rest = after;
        }
    }
    let mut reason = 0;
    if let Some((first, after)) = rest.split_first() {
        let found = match first.parse::<u8>() {
            Ok(n @ 1..=13) => Some(n),
            _ => BLOCK_REASONS
                .iter()
                .position(|r| r.eq_ignore_ascii_case(first))
                .map(|i| i as u8 + 1),
        };
        if let Some(found) = found {
            reason = found;
            rest = after;
        }
    }
    Ok(Ok(vec![WorldAction::Discipline(Discipline::Block {
        target: name.to_string(),
        reason,
        days,
        note: rest.join(" "),
        slash: false,
    })]))
}

/// `!unban`: lift a character's account's block.
fn unban(_: &mut WorldContext, args: &[&str]) -> Outcome {
    match args {
        [name] => Ok(Ok(vec![WorldAction::Discipline(Discipline::Unblock {
            target: name.to_string(),
        })])),
        _ => Ok(Err(usage("unban"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::SendOpcode;
    use game::Player;

    const BROADCAST: [u8; 2] = (SendOpcode::BroadcastMsg as i16).to_le_bytes();

    fn player(gm_level: i16) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.gm_level = gm_level;
        player
    }

    fn run_as(player: &mut Player, line: &str) -> Option<Vec<WorldAction>> {
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        run(&mut ctx, line).map(Result::unwrap)
    }

    /// The text of each pink line among `actions`.
    fn lines(actions: &[WorldAction]) -> Vec<String> {
        actions
            .iter()
            .filter_map(|action| match action {
                WorldAction::Reply(packet)
                    if packet.bytes[..3] == [BROADCAST[0], BROADCAST[1], Broadcast::Pink as u8] =>
                {
                    let len = u16::from_le_bytes([packet.bytes[3], packet.bytes[4]]) as usize;
                    Some(String::from_utf8_lossy(&packet.bytes[5..5 + len]).into_owned())
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_players_bang_line_is_chat_and_at_lines_are_commands() {
        let mut p = player(0);
        assert!(run_as(&mut p, "!item 2000000").is_none());
        assert!(run_as(&mut p, "hello").is_none());
        assert!(run_as(&mut p, "@dispose").is_some());
        assert!(run_as(&mut p, "@_@").is_none());
        assert_eq!(p.inventory.count(2000000), 0);
    }

    #[test]
    fn commands_need_their_tier() {
        let mut helper = player(HELPER);
        let refused = run_as(&mut helper, "!meso 100").unwrap();
        assert_eq!(lines(&refused), ["Unknown command. Type !help for a list."]);
        assert_eq!(helper.meso, player(0).meso);

        let mut gm = player(GM);
        run_as(&mut gm, "!meso 100").unwrap();
        assert_eq!(gm.meso, player(0).meso + 100);

        // Each tier's !help lists what it may run.
        let helps = |gm_level| lines(&run_as(&mut player(gm_level), "!help").unwrap());
        assert!(helps(HELPER).iter().all(|line| !line.starts_with("!item")));
        assert!(helps(GM)
            .iter()
            .any(|line| line == "!item <item id> [quantity]"));
        assert_eq!(tier(7), ADMIN);
    }

    #[test]
    fn skill_teaches_any_skill_at_its_highest_level_or_the_one_given() {
        if crate::skills::data().is_err() {
            return;
        }
        let mut gm = player(GM);
        let sent = run_as(&mut gm, "!skill 1013").unwrap();
        assert!(
            matches!(&sent[..], [WorldAction::Reply(p)] if p.opcode() == SendOpcode::UpdateSkills as i16)
        );
        assert_eq!(gm.skills.level(1013), 2);
        run_as(&mut gm, "!skill 1013 1").unwrap();
        assert_eq!(gm.skills.level(1013), 1);
        run_as(&mut gm, "!skill 1013 0").unwrap();
        assert!(gm.skills.get(1013).is_none());
        // A fourth-job skill is mastered.
        run_as(&mut gm, "!skill 1121000 5").unwrap();
        assert_eq!(gm.skills.get(1121000).unwrap().master_level, 30);

        let said = |line| lines(&run_as(&mut player(GM), line).unwrap());
        assert_eq!(said("!skill 1013 3"), ["Levels go from 0 to 2."]);
        assert_eq!(said("!skill 999"), ["Usage: !skill <skill id> [level]"]);
        assert_eq!(said("!skill 1013 x"), ["Usage: !skill <skill id> [level]"]);
    }

    #[test]
    fn ban_reads_its_days_reason_and_note() {
        let discipline = |gm_level, line| {
            run_as(&mut player(gm_level), line)
                .unwrap()
                .into_iter()
                .find_map(|action| match action {
                    WorldAction::Discipline(d) => Some(d),
                    _ => None,
                })
        };
        let block = |target: &str, reason, days, note: &str| {
            Some(Discipline::Block {
                target: target.into(),
                reason,
                days,
                note: note.into(),
                slash: false,
            })
        };
        assert_eq!(discipline(ADMIN, "!ban Bob"), block("Bob", 0, None, ""));
        assert_eq!(
            discipline(ADMIN, "!ban Bob 7 bot farming slimes"),
            block("Bob", 2, Some(7), "farming slimes")
        );
        assert_eq!(
            discipline(ADMIN, "!ban Bob 3 13 left"),
            block("Bob", 13, Some(3), "left")
        );
        assert_eq!(
            discipline(ADMIN, "!ban Bob 99999 hack"),
            block("Bob", 1, Some(BLOCK_DAYS), "")
        );
        assert_eq!(
            discipline(ADMIN, "!ban Bob spam"),
            block("Bob", 0, None, "spam")
        );
        assert_eq!(discipline(ADMIN, "!ban Bob 0"), None);
        assert_eq!(discipline(GM, "!ban Bob"), None);
        assert_eq!(
            discipline(ADMIN, "!unban Bob"),
            Some(Discipline::Unblock {
                target: "Bob".into()
            })
        );
        assert_eq!(discipline(GM, "!unban Bob"), None);
        assert_eq!(
            discipline(GM, "!dc Bob"),
            Some(Discipline::Disconnect {
                target: "Bob".into()
            })
        );
        assert_eq!(discipline(HELPER, "!dc Bob"), None);
    }

    #[test]
    fn jail_reads_its_minutes() {
        let discipline = |gm_level, line| {
            run_as(&mut player(gm_level), line)
                .unwrap()
                .into_iter()
                .find_map(|action| match action {
                    WorldAction::Discipline(d) => Some(d),
                    _ => None,
                })
        };
        let jail = |minutes| {
            Some(Discipline::Jail {
                target: "Bob".into(),
                minutes,
            })
        };
        assert_eq!(discipline(GM, "!jail Bob"), jail(5));
        assert_eq!(discipline(GM, "!jail Bob 30"), jail(30));
        assert_eq!(discipline(GM, "!jail Bob 99999999"), jail(JAIL_MINUTES_MAX));
        assert_eq!(discipline(GM, "!jail Bob 0"), None);
        assert_eq!(discipline(GM, "!jail Bob soon"), None);
        assert_eq!(discipline(HELPER, "!jail Bob"), None);
        assert_eq!(
            discipline(GM, "!unjail Bob"),
            Some(Discipline::Release {
                target: "Bob".into()
            })
        );
        assert_eq!(discipline(HELPER, "!unjail Bob"), None);
    }

    #[test]
    fn mute_reads_its_minutes_and_reason() {
        let discipline = |gm_level, line| {
            run_as(&mut player(gm_level), line)
                .unwrap()
                .into_iter()
                .find_map(|action| match action {
                    WorldAction::Discipline(d) => Some(d),
                    _ => None,
                })
        };
        let mute = |minutes, reason| {
            Some(Discipline::Mute {
                target: "Bob".into(),
                minutes,
                reason,
            })
        };
        assert_eq!(discipline(GM, "!mute Bob"), mute(60, 1));
        assert_eq!(discipline(GM, "!mute Bob 10"), mute(10, 1));
        assert_eq!(discipline(GM, "!mute Bob 10 advertising"), mute(10, 2));
        assert_eq!(discipline(GM, "!mute Bob Penalty"), mute(60, 6));
        assert_eq!(discipline(GM, "!mute Bob 5 3"), mute(5, 3));
        assert_eq!(discipline(GM, "!mute Bob 5 7"), None);
        assert_eq!(discipline(GM, "!mute Bob 0"), None);
        assert_eq!(discipline(GM, "!mute Bob spam"), None);
        assert_eq!(discipline(HELPER, "!mute Bob"), None);
        assert_eq!(
            discipline(GM, "!unmute Bob"),
            Some(Discipline::Unmute {
                target: "Bob".into()
            })
        );
    }

    #[test]
    fn mesos_stay_within_bounds() {
        let mut gm = player(GM);
        run_as(&mut gm, "!meso -999999999").unwrap();
        assert_eq!(gm.meso, 0);
        run_as(&mut gm, &format!("!meso {}", i32::MAX)).unwrap();
        run_as(&mut gm, "!meso 5").unwrap();
        assert_eq!(gm.meso, i32::MAX);
    }

    #[test]
    fn bad_arguments_show_the_usage() {
        let mut gm = player(GM);
        let actions = run_as(&mut gm, "!level 0").unwrap();
        assert_eq!(lines(&actions), ["Usage: !level <level>"]);
        let actions = run_as(&mut gm, "!item").unwrap();
        assert_eq!(lines(&actions), ["Usage: !item <item id> [quantity]"]);
        assert_eq!(
            lines(&run_as(&mut gm, "!job 140").unwrap()),
            ["Usage: !job <job id>"]
        );
    }

    #[test]
    fn guild_points_need_a_guild_and_an_amount() {
        let mut gm = player(GM);
        let actions = run_as(&mut gm, "!gp 50").unwrap();
        assert_eq!(lines(&actions), ["You are not in a guild."]);
        gm.guild = Some(game::guild::GuildTag {
            id: 4,
            name: "Testers".into(),
            emblem: Default::default(),
            alliance_id: 0,
        });
        let actions = run_as(&mut gm, "!gp -20").unwrap();
        assert!(matches!(
            actions[..],
            [WorldAction::Guild(crate::GuildRequest::GainPoints {
                amount: -20
            })]
        ));
        let actions = run_as(&mut gm, "!gp 0").unwrap();
        assert_eq!(lines(&actions), ["Usage: !gp <amount>"]);
    }

    #[test]
    fn levels_jobs_and_points_are_set() {
        let mut gm = player(GM);
        let actions = run_as(&mut gm, "!level 70").unwrap();
        assert_eq!((gm.stats.level, gm.stats.exp), (70, 0));
        assert!(matches!(&actions[1], WorldAction::AvatarChanged(announce) if announce.len() == 1));
        run_as(&mut gm, "!job 211").unwrap();
        run_as(&mut gm, "!ap 12").unwrap();
        run_as(&mut gm, "!sp 3").unwrap();
        assert_eq!((gm.stats.job, gm.stats.ap, gm.stats.sp), (211, 12, 3));
        // A lower level is no level-up for others to see.
        let actions = run_as(&mut gm, "!level 10").unwrap();
        assert!(matches!(&actions[1], WorldAction::AvatarChanged(announce) if announce.is_empty()));
    }

    #[test]
    fn givenx_defaults_to_nx_credit_and_the_gm() {
        let mut gm = player(GM);
        let give = |gm: &mut Player, line: &str| match run_as(gm, line).unwrap().as_slice() {
            [WorldAction::GiveCash {
                target,
                cash_type,
                amount,
            }] => Some((target.clone(), *cash_type, *amount)),
            _ => None,
        };
        assert_eq!(
            give(&mut gm, "!givenx 5000"),
            Some((None, CashType::Credit, 5000))
        );
        assert_eq!(
            give(&mut gm, "!givenx MP Alice 20"),
            Some((Some("Alice".into()), CashType::MaplePoints, 20))
        );
        assert_eq!(
            give(&mut gm, "!givenx np -3"),
            Some((None, CashType::Prepaid, -3))
        );
        assert_eq!(
            give(&mut gm, "!givenx Bob 1"),
            Some((Some("Bob".into()), CashType::Credit, 1))
        );
        for line in [
            "!givenx",
            "!givenx nx",
            "!givenx 0",
            "!givenx a b 1",
            "!givenx 1e3",
        ] {
            assert_eq!(
                lines(&run_as(&mut gm, line).unwrap()),
                ["Usage: !givenx [nx|mp|np] [name] <amount>"],
                "{line}"
            );
        }
        // Not for helpers.
        let mut helper = player(HELPER);
        assert_eq!(
            lines(&run_as(&mut helper, "!givenx 5000").unwrap()),
            ["Unknown command. Type !help for a list."]
        );
    }

    #[test]
    fn coupons_take_rewards_an_expiry_and_a_code() {
        let mut gm = player(GM);
        let actions = run_as(
            &mut gm,
            "!coupon NX=5000 meso=100 2000000x5 1002140 days=7 code=Spring1",
        )
        .unwrap();
        let [WorldAction::MakeCoupon {
            code,
            days,
            rewards,
        }] = actions.as_slice()
        else {
            panic!("{actions:?}");
        };
        assert_eq!((code.as_deref(), *days), (Some("SPRING1"), Some(7)));
        assert_eq!(
            rewards,
            &[
                Reward::Credit(5000),
                Reward::Mesos(100),
                Reward::Item {
                    item_id: 2_000_000,
                    quantity: 5
                },
                Reward::Item {
                    item_id: 1_002_140,
                    quantity: 1
                },
            ]
        );
        let usage = "Usage: !coupon <nx=N|mp=N|np=N|meso=N|item id[xN]>... [days=N] [code=CODE]";
        for line in [
            "!coupon",
            "!coupon days=7",
            "!coupon nx=0",
            "!coupon nx=-5",
            "!coupon 2000000x0",
            "!coupon nx=1 days=0",
            "!coupon nx=1 days=99999",
            "!coupon nx=1 code=",
            "!coupon nx=1 color=red",
        ] {
            assert_eq!(lines(&run_as(&mut gm, line).unwrap()), [usage], "{line}");
        }
        assert_eq!(
            lines(&run_as(&mut gm, "!coupon 1").unwrap()),
            ["There is no item 1."]
        );
    }

    #[test]
    fn known_jobs_are_v83s() {
        for job in [
            0, 100, 110, 132, 230, 312, 422, 522, 900, 910, 1000, 1511, 2000, 2112,
        ] {
            assert!(known_job(job), "{job}");
        }
        for job in [-1, 140, 240, 330, 600, 1120, 1112, 2120, 113, 3000] {
            assert!(!known_job(job), "{job}");
        }
    }

    #[test]
    fn hide_and_heal() {
        let mut helper = player(HELPER);
        helper.stats.hp = 1;
        run_as(&mut helper, "!heal").unwrap();
        assert_eq!(helper.stats.hp, helper.stats.max_hp);
        // The dead revive as the client lets them, not by command.
        helper.stats.hp = 0;
        for line in ["!heal", "!warp 100000000"] {
            assert_eq!(lines(&run_as(&mut helper, line).unwrap()), [DEAD]);
        }
        assert_eq!(
            (helper.stats.hp, helper.stats.map_id),
            (0, player(0).stats.map_id)
        );
        helper.stats.hp = 1;
        let actions = run_as(&mut helper, "!hide").unwrap();
        assert!(helper.hidden);
        assert!(actions
            .iter()
            .any(|action| matches!(action, WorldAction::HiddenChanged)));
        run_as(&mut helper, "!unhide").unwrap();
        assert!(!helper.hidden);
    }
}
