//! Guild alliances (the client's "Guild Union"): a few guilds under one
//! name, whose members see each other in the client's Alliance tab and
//! talk in alliance chat.
//!
//! Each member of an alliance guild has an alliance rank: 1 for the
//! alliance's master, 2 for the masters of its other guilds, and 3 to 5
//! for everyone else (report b023, section 4.5).

use crate::guild::{self, Guild, GuildMember, NOTICE_MAX_LEN};

/// The alliance master's rank.
pub const MASTER_RANK: u8 = 1;
/// The rank of the other guilds' masters.
pub const GUILD_MASTER_RANK: u8 = 2;
/// The highest rank an ordinary member may reach.
pub const MEMBER_RANK: u8 = 3;
/// The rank of members of guilds in no alliance, and of members who join.
pub const LOWEST_RANK: u8 = 5;
/// The rank titles a new alliance starts with (OdinMS and HeavenMS).
pub const DEFAULT_TITLES: [&str; 5] = ["Master", "Jr. Master", "Member", "Member", "Member"];
/// Mesos Lenario charges to found an alliance (HeavenMS).
pub const CREATE_COST: i32 = 2_000_000;
/// Mesos for room for one more guild (HeavenMS).
pub const GROW_COST: i32 = 1_000_000;
/// The guilds a new alliance has room for: its two founders.
pub const START_CAPACITY: u8 = 2;
/// The most guilds Lenario lets an alliance grow to (HeavenMS).
pub const MAX_CAPACITY: u8 = 5;
/// Alliance names are guild names (`name_is_valid`): 4 to 12 bytes.
pub const NAME_LEN: std::ops::RangeInclusive<i16> = 4..=12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alliance {
    pub id: i32,
    pub name: String,
    /// Titles of ranks 1 to 5.
    pub titles: [String; 5],
    /// The most guilds it may hold.
    pub capacity: u8,
    pub notice: String,
    /// Its guilds, by id.
    pub guilds: Vec<Guild>,
}

impl Alliance {
    pub fn guild_ids(&self) -> Vec<i32> {
        self.guilds.iter().map(|guild| guild.id).collect()
    }

    pub fn guild(&self, id: i32) -> Option<&Guild> {
        self.guilds.iter().find(|guild| guild.id == id)
    }

    /// The guild `cid` is in, with their member record.
    pub fn member(&self, cid: i32) -> Option<(&Guild, &GuildMember)> {
        self.guilds
            .iter()
            .find_map(|guild| guild.member(cid).map(|member| (guild, member)))
    }

    fn rank_of(&self, cid: i32) -> Option<u8> {
        self.member(cid).map(|(_, member)| member.alliance_rank)
    }

    /// Everyone in the alliance's guilds, online or not.
    pub fn member_ids(&self) -> impl Iterator<Item = i32> + '_ {
        self.guilds
            .iter()
            .flat_map(|guild| guild.members.iter().map(|member| member.id))
    }

    pub fn is_master(&self, cid: i32) -> bool {
        self.rank_of(cid) == Some(MASTER_RANK)
    }

    /// The master of the alliance's master guild.
    pub fn master(&self) -> Option<&GuildMember> {
        self.guilds
            .iter()
            .flat_map(|guild| &guild.members)
            .find(|member| member.alliance_rank == MASTER_RANK)
    }

    /// The lowest rank with a title, which the client lets nobody pass
    /// (`0xa3ae1f`): 3 while title 4 is empty, 4 while title 5 is.
    pub fn lowest_rank(&self) -> u8 {
        guild::lowest_rank(&self.titles)
    }

    /// Whether member `by` may invite another guild: only the master,
    /// while there is room (the client's BtInvite enable).
    pub fn may_invite(&self, by: i32) -> bool {
        self.is_master(by) && self.guilds.len() < usize::from(self.capacity)
    }

    /// The guild member `by` would take out of the alliance by leaving:
    /// only the master of a guild other than the master's (BtWithdraw needs
    /// rank 2).
    pub fn leaving_guild(&self, by: i32) -> Option<&Guild> {
        self.member(by)
            .filter(|(_, member)| {
                member.alliance_rank == GUILD_MASTER_RANK && member.rank == guild::MASTER_RANK
            })
            .map(|(guild, _)| guild)
    }

    /// Whether member `by` may expel guild `guild_id`: only the master, and
    /// not their own guild.
    pub fn may_expel(&self, by: i32, guild_id: i32) -> bool {
        self.member(by)
            .is_some_and(|(own, member)| member.alliance_rank == MASTER_RANK && own.id != guild_id)
            && self.guild(guild_id).is_some()
    }

    /// Whether member `by` may hand the alliance to `to`: only the master,
    /// to the master of another of its guilds (BtKick and BtChange need the
    /// row's guild rank 1).
    pub fn may_change_master(&self, by: i32, to: i32) -> bool {
        self.is_master(by)
            && by != to
            && self
                .member(to)
                .is_some_and(|(_, member)| member.rank == guild::MASTER_RANK)
    }

    /// The rank member `by` may move `member_id` to, a step up or down, as
    /// the client's Up and Down buttons allow: the master or a guild master
    /// moves anyone between rank 3 and the lowest rank with a title.
    pub fn rank_change(&self, by: i32, member_id: i32, up: bool) -> Option<u8> {
        let by = self.rank_of(by)?;
        let rank = self.rank_of(member_id)?;
        if by > GUILD_MASTER_RANK || rank < MEMBER_RANK {
            return None;
        }
        if up {
            (rank > MEMBER_RANK).then(|| rank - 1)
        } else {
            (rank < self.lowest_rank()).then(|| rank + 1)
        }
    }

    /// Whether member `by` may give the ranks these titles. Only the master
    /// edits them, under the rules guild titles follow: no rank loses its
    /// title while members hold it, and title 5 needs title 4.
    pub fn may_set_titles(
        &self,
        by: i32,
        titles: &[String; 5],
        forbids: impl Fn(&str) -> bool,
    ) -> bool {
        let lowest = guild::lowest_rank(titles);
        self.is_master(by)
            && titles
                .iter()
                .zip(&self.titles)
                .enumerate()
                .all(|(i, (title, old))| {
                    title == old || (guild::title_is_valid(i, title) && !forbids(title))
                })
            && !(titles[3].is_empty() && !titles[4].is_empty())
            && self
                .guilds
                .iter()
                .flat_map(|guild| &guild.members)
                .all(|member| member.alliance_rank <= lowest)
    }

    /// Whether member `by` may post this notice: the master or a guild
    /// master, with text the client's prompt can send.
    pub fn may_set_notice(&self, by: i32, notice: &str) -> bool {
        self.rank_of(by)
            .is_some_and(|rank| rank <= GUILD_MASTER_RANK)
            && !notice.is_empty()
            && notice.len() <= NOTICE_MAX_LEN
            && !notice.chars().any(|c| c < ' ' && c != '\r' && c != '\n')
    }
}

/// The alliance rank a member of a joining guild gets: 2 for its master,
/// 5 for the rest.
pub fn joining_rank(guild_rank: u8) -> u8 {
    if guild_rank == guild::MASTER_RANK {
        GUILD_MASTER_RANK
    } else {
        LOWEST_RANK
    }
}

/// Alliance names follow the guild name rules.
pub fn name_is_valid(name: &str) -> bool {
    guild::name_is_valid(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guild::Emblem;

    /// Guild `id` with members `id * 10 + i`, each `(guild rank, alliance
    /// rank)`.
    fn guild(id: i32, ranks: &[(u8, u8)]) -> Guild {
        Guild {
            id,
            name: format!("Guild{id}"),
            titles: guild::DEFAULT_TITLES.map(String::from),
            capacity: 10,
            emblem: Emblem::default(),
            notice: String::new(),
            points: 0,
            alliance_id: 7,
            members: ranks
                .iter()
                .enumerate()
                .map(|(i, &(rank, alliance_rank))| GuildMember {
                    id: id * 10 + i as i32,
                    name: format!("Member{id}{i}"),
                    job: 0,
                    level: 10,
                    rank,
                    alliance_rank,
                    online: true,
                })
                .collect(),
        }
    }

    /// Guild 1 is the master's (members 10..), guild 2 another (20..).
    fn alliance() -> Alliance {
        Alliance {
            id: 7,
            name: "Union".into(),
            titles: DEFAULT_TITLES.map(String::from),
            capacity: 3,
            notice: String::new(),
            guilds: vec![
                guild(1, &[(1, 1), (2, 5), (5, 3)]),
                guild(2, &[(1, 2), (5, 4)]),
            ],
        }
    }

    #[test]
    fn the_master_invites_while_there_is_room() {
        let mut union = alliance();
        assert!(union.may_invite(10));
        assert!(!union.may_invite(20));
        assert!(!union.may_invite(11));
        union.capacity = 2;
        assert!(!union.may_invite(10));
    }

    #[test]
    fn only_other_guild_masters_leave() {
        let union = alliance();
        assert_eq!(union.leaving_guild(20).map(|guild| guild.id), Some(2));
        assert!(union.leaving_guild(10).is_none(), "the master can't leave");
        assert!(union.leaving_guild(21).is_none());
        assert!(union.leaving_guild(99).is_none());
    }

    #[test]
    fn the_master_expels_other_guilds_and_hands_over_to_their_masters() {
        let union = alliance();
        assert!(union.may_expel(10, 2));
        assert!(!union.may_expel(10, 1));
        assert!(!union.may_expel(10, 3));
        assert!(!union.may_expel(20, 1));
        assert!(union.may_change_master(10, 20));
        assert!(!union.may_change_master(10, 21));
        assert!(!union.may_change_master(10, 10));
        assert!(!union.may_change_master(20, 10));
    }

    #[test]
    fn ranks_move_between_3_and_the_lowest_titled() {
        let mut union = alliance();
        assert_eq!(union.rank_change(10, 12, false), Some(4));
        assert_eq!(union.rank_change(20, 21, true), Some(3));
        assert_eq!(union.rank_change(20, 21, false), Some(5));
        assert_eq!(union.rank_change(10, 11, true), Some(4));
        assert_eq!(union.rank_change(10, 11, false), None);
        assert_eq!(union.rank_change(10, 12, true), None);
        assert_eq!(
            union.rank_change(10, 20, false),
            None,
            "guild masters stay 2"
        );
        assert_eq!(
            union.rank_change(11, 12, false),
            None,
            "members move nobody"
        );
        union.titles[4].clear();
        assert_eq!(union.rank_change(10, 21, false), None);
        assert_eq!(union.rank_change(10, 11, true), Some(4));
    }

    #[test]
    fn titles_keep_every_held_rank() {
        let union = alliance();
        let titles = |t: [&str; 5]| t.map(String::from);
        let none = |_: &str| false;
        assert!(union.may_set_titles(
            10,
            &titles(["Boss", "Chief", "Grunt", "Newbie", "Rookie"]),
            none
        ));
        assert!(!union.may_set_titles(
            20,
            &titles(["Boss", "Chief", "Grunt", "Newbie", "Rookie"]),
            none
        ));
        assert!(!union.may_set_titles(10, &titles(["Boss", "Chief", "Grunt", "Newbie", ""]), none));
        assert!(!union.may_set_titles(10, &titles(["Boss", "Chief", "Grunt", "", "Rookie"]), none));
    }

    #[test]
    fn masters_and_guild_masters_post_the_notice() {
        let union = alliance();
        assert!(union.may_set_notice(10, "Hello"));
        assert!(union.may_set_notice(20, "Hello"));
        assert!(!union.may_set_notice(11, "Hello"));
        assert!(!union.may_set_notice(10, ""));
        assert!(!union.may_set_notice(10, &"x".repeat(101)));
    }

    #[test]
    fn joining_masters_rank_2_and_the_rest_5() {
        assert_eq!(joining_rank(1), 2);
        assert_eq!(joining_rank(2), 5);
        assert_eq!(joining_rank(5), 5);
    }
}
