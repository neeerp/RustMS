//! Expedition squads (Cosmic `Expedition`): a leader gathers players at the
//! event's NPC, party or not, then takes those on the event's lobby into
//! it (`events::Event::squad`). One squad of each kind per channel; it
//! lapses if its leader hasn't started it within `REGISTRATION`. Pure
//! rules; the directory keeps the squads.

use crate::events::EventKind;
use crate::ClientId;
use game_data::server_content::keys;
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// How long a squad gathers before it lapses (Cosmic `ExpeditionType`'s
/// registration minutes). Not shortened for testing, so a test has time
/// to gather one.
pub const REGISTRATION: Duration = Duration::from_secs(5 * 60);

/// What a kind of squad's NPC and notices say that names its foe or its
/// place (`keys`); the rest is the same for every kind (`keys::SQUAD_*`).
#[derive(Debug)]
pub struct SquadText {
    /// The player isn't of the event's levels (`{low}`, `{high}`).
    pub level: &'static str,
    /// Lead one? (`events`' terms: `{least}`, `{most}`, `{low}`, `{high}`)
    pub ask: &'static str,
    pub start_choice: &'static str,
    pub join_ask: &'static str,
    /// Go back into it, while it runs?
    pub rejoin_ask: &'static str,
    /// Another is under way.
    pub taken: &'static str,
    /// To those on the lobby as `{name}` gathers one.
    pub notice_opened: &'static str,
    /// To its roster as its leader, `{name}`, takes it in.
    pub notice_started: &'static str,
}

const ZAKUM: SquadText = SquadText {
    level: keys::ZAKUM_SQUAD_LEVEL,
    ask: keys::ZAKUM_SQUAD_ASK,
    start_choice: keys::ZAKUM_SQUAD_START_CHOICE,
    join_ask: keys::ZAKUM_SQUAD_JOIN_ASK,
    rejoin_ask: keys::ZAKUM_SQUAD_REJOIN_ASK,
    taken: keys::ZAKUM_SQUAD_TAKEN,
    notice_opened: keys::ZAKUM_SQUAD_NOTICE_OPENED,
    notice_started: keys::ZAKUM_SQUAD_NOTICE_STARTED,
};

const HORNTAIL: SquadText = SquadText {
    level: keys::HORNTAIL_SQUAD_LEVEL,
    ask: keys::HORNTAIL_SQUAD_ASK,
    start_choice: keys::HORNTAIL_SQUAD_START_CHOICE,
    join_ask: keys::HORNTAIL_SQUAD_JOIN_ASK,
    rejoin_ask: keys::HORNTAIL_SQUAD_REJOIN_ASK,
    taken: keys::HORNTAIL_SQUAD_TAKEN,
    notice_opened: keys::HORNTAIL_SQUAD_NOTICE_OPENED,
    notice_started: keys::HORNTAIL_SQUAD_NOTICE_STARTED,
};

/// What `kind`'s squad says, if its event is gathered by one.
pub fn text(kind: EventKind) -> Option<&'static SquadText> {
    match kind {
        EventKind::Zakum => Some(&ZAKUM),
        EventKind::Horntail => Some(&HORNTAIL),
        _ => None,
    }
}

/// What a squad's NPC finds for a player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SquadView {
    /// There is none on the channel: the player may gather one.
    None,
    /// The player leads it, still gathering: the other members, in the
    /// order they joined.
    Leading(Vec<(ClientId, String)>),
    /// The player is in it, waiting for its leader to start it.
    Registered {
        leader: String,
    },
    /// The player may join it.
    Joinable,
    /// Its leader expelled the player, who may not come back.
    Banned {
        leader: String,
    },
    Full,
    /// It is under way with the player on its roster: they may go back in.
    Rejoin,
    /// It is under way without the player.
    Taken,
}

/// What a player tells a squad, at its NPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SquadCommand {
    /// Gather one, and lead it.
    Open,
    Join,
    /// The leader turns a member out, for good.
    Expel(ClientId),
    /// The leader takes it in.
    Start,
    /// The leader calls it off.
    Stop,
    /// A member on its roster goes back into it while it runs.
    Rejoin,
}

#[derive(Clone, Debug)]
pub struct Squad {
    kind: EventKind,
    /// The leader first, then the others in the order they joined.
    members: Vec<(ClientId, String)>,
    banned: HashSet<ClientId>,
    /// When it lapses, unless started.
    closes: Instant,
    /// The event it started, once it has.
    started: Option<u32>,
}

impl Squad {
    pub fn open(kind: EventKind, leader: ClientId, name: String, now: Instant) -> Self {
        Self {
            kind,
            members: vec![(leader, name)],
            banned: HashSet::new(),
            closes: now + REGISTRATION,
            started: None,
        }
    }

    pub fn leader(&self) -> ClientId {
        self.members[0].0
    }

    pub fn leader_name(&self) -> &str {
        &self.members[0].1
    }

    pub fn members(&self) -> &[(ClientId, String)] {
        &self.members
    }

    pub fn has(&self, who: ClientId) -> bool {
        self.members.iter().any(|&(id, _)| id == who)
    }

    /// The event it started, if it has.
    pub fn started(&self) -> Option<u32> {
        self.started
    }

    /// Whether it gathered too long without starting.
    pub fn lapsed(&self, now: Instant) -> bool {
        self.started.is_none() && now >= self.closes
    }

    /// How long it has left to gather.
    pub fn closes_in(&self, now: Instant) -> Duration {
        self.closes.saturating_duration_since(now)
    }

    pub fn view(&self, who: ClientId) -> SquadView {
        let leader = || self.leader_name().to_owned();
        if self.started.is_some() {
            return if self.has(who) {
                SquadView::Rejoin
            } else {
                SquadView::Taken
            };
        }
        if who == self.leader() {
            SquadView::Leading(self.members[1..].to_vec())
        } else if self.has(who) {
            SquadView::Registered { leader: leader() }
        } else if self.banned.contains(&who) {
            SquadView::Banned { leader: leader() }
        } else if self.members.len() >= *self.kind.event().team.end() {
            SquadView::Full
        } else {
            SquadView::Joinable
        }
    }

    /// `who` joins, if they still may. Returns whether they did.
    pub fn join(&mut self, who: ClientId, name: String) -> bool {
        if self.view(who) != SquadView::Joinable {
            return false;
        }
        self.members.push((who, name));
        true
    }

    /// The leader, `by`, turns `who` out and bans them, while it gathers.
    /// Returns their name if they were in it.
    pub fn expel(&mut self, by: ClientId, who: ClientId) -> Option<String> {
        if by != self.leader() || who == by || self.started.is_some() {
            return None;
        }
        let index = self.members.iter().position(|&(id, _)| id == who)?;
        self.banned.insert(who);
        Some(self.members.remove(index).1)
    }

    /// Whether `by` may take it in: its leader, while it gathers, with
    /// enough in it (`Event::least`), wherever they are (Cosmic counts the
    /// whole roster; only those on the lobby go).
    pub fn may_start(&self, by: ClientId) -> bool {
        by == self.leader()
            && self.started.is_none()
            && self.members.len() >= self.kind.event().least()
    }

    /// It started as event `id`. Its whole roster may come in while it
    /// runs, those who weren't there to go too (Cosmic's `canJoin`).
    pub fn start(&mut self, id: u32) {
        self.started = Some(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(id: ClientId) -> String {
        format!("m{id}")
    }

    #[test]
    fn a_squad_gathers_bans_and_starts_with_those_who_go() {
        let now = Instant::now();
        let mut squad = Squad::open(EventKind::Zakum, 1, named(1), now);
        assert_eq!(squad.view(1), SquadView::Leading(Vec::new()));
        assert_eq!(squad.view(2), SquadView::Joinable);
        for id in 2..=6 {
            assert!(squad.join(id, named(id)));
        }
        assert!(!squad.join(2, named(2)), "already in");
        assert_eq!(squad.view(2), SquadView::Registered { leader: named(1) });
        // Only the leader expels, never themself, and the expelled may not
        // come back.
        assert_eq!(squad.expel(2, 3), None);
        assert_eq!(squad.expel(1, 1), None);
        assert_eq!(squad.expel(1, 6), Some(named(6)));
        assert_eq!(squad.expel(1, 6), None);
        assert_eq!(squad.view(6), SquadView::Banned { leader: named(1) });
        assert!(!squad.join(6, named(6)));
        // Five are too few for Zakum; six do.
        assert!(!squad.may_start(1));
        assert!(squad.join(7, named(7)));
        assert!(!squad.may_start(2), "only the leader starts it");
        assert!(squad.may_start(1));
        let SquadView::Leading(others) = squad.view(1) else {
            panic!("{:?}", squad.view(1));
        };
        assert_eq!(
            others.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [2, 3, 4, 5, 7]
        );
        // Member 7 was away, but may still come in.
        squad.start(9);
        assert_eq!(squad.started(), Some(9));
        assert!(!squad.may_start(1));
        assert_eq!(squad.view(2), SquadView::Rejoin);
        assert_eq!(squad.view(1), SquadView::Rejoin);
        assert_eq!(squad.view(7), SquadView::Rejoin);
        assert_eq!(squad.view(6), SquadView::Taken, "expelled");
        assert_eq!(squad.expel(1, 2), None, "not once under way");
        assert!(!squad.join(8, named(8)));
    }

    #[test]
    fn a_squad_is_full_at_thirty_and_lapses_unstarted() {
        let now = Instant::now();
        let mut squad = Squad::open(EventKind::Zakum, 1, named(1), now);
        for id in 2..=30 {
            assert!(squad.join(id, named(id)));
        }
        assert_eq!(squad.view(31), SquadView::Full);
        assert!(!squad.join(31, named(31)));
        assert!(!squad.lapsed(now + REGISTRATION - Duration::from_secs(1)));
        assert_eq!(
            squad.closes_in(now + Duration::from_secs(60)),
            REGISTRATION - Duration::from_secs(60)
        );
        assert!(squad.lapsed(now + REGISTRATION));
        squad.start(1);
        assert!(
            !squad.lapsed(now + REGISTRATION),
            "started, it no longer lapses"
        );
    }

    #[test]
    fn every_squads_event_has_its_texts() {
        for kind in EventKind::ALL {
            assert_eq!(kind.event().squad, text(kind).is_some(), "{kind:?}");
        }
    }
}
