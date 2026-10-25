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
use crate::handler::{BroadcastScope, WorldAction, WorldContext};
use crate::items;
use crate::packet::build::world::{
    drop::build_meso_gain_in_chat,
    effect::{build_foreign_effect, CharEffect},
    inventory,
    messaging::{build_broadcast, Broadcast},
    stat::{self, StatUpdate},
};
use crate::packet::handle::world::{set_hidden, warp_to_spawn_point};
use db::cash_shop::CashType;
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
        name: "givenx",
        tier: GM,
        usage: "[nx|mp|np] [name] <amount>",
        run: give_nx,
    },
    Command {
        prefix: '!',
        name: "notice",
        tier: GM,
        usage: "<text>",
        run: notice,
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
