//! Guilds: a named group of characters with a master, shown under each
//! member's name and in the client's Guild window.

use std::ops::RangeInclusive;

/// Lowest level that may found a guild. The client doesn't check; this is
/// the HeavenMS default.
pub const MIN_LEVEL: u8 = 10;
/// Mesos it costs to found a guild; the HeavenMS default, not from the
/// client.
pub const CREATE_COST: i32 = 1_500_000;
/// The rank titles a new guild starts with (OdinMS and HeavenMS).
pub const DEFAULT_TITLES: [&str; 5] = ["Master", "Jr. Master", "Member", "Member", "Member"];
/// The master's rank; members range from 1 to 5.
pub const MASTER_RANK: u8 = 1;
/// A jr. master, who may invite and expel too.
pub const JR_MASTER_RANK: u8 = 2;
/// The longest notice the client's prompt takes (FUN_009114f1).
pub const NOTICE_MAX_LEN: usize = 100;
/// The most members Heracle lets a guild grow to; the HeavenMS default.
pub const MAX_CAPACITY: u8 = 100;
/// Members each of Heracle's steps adds; the HeavenMS default.
pub const CAPACITY_STEP: u8 = 5;

/// Mesos Heracle asks to grow a guild of `capacity` by a step, at the
/// HeavenMS default prices: 500,000 up to 15 members, a million more for
/// each step past that, and 5 million past 30.
pub fn capacity_cost(capacity: u8) -> i32 {
    let capacity = i32::from(capacity);
    if capacity > 30 {
        5_000_000
    } else {
        500_000 + (capacity - 15).max(0) / i32::from(CAPACITY_STEP) * 1_000_000
    }
}

/// The guild headquarters in Orbis, where Heracle and Lea are.
pub const HEADQUARTERS: i32 = 200_000_301;
/// Mesos Lea asks to make or change the guild's emblem; the HeavenMS
/// default, not from the client.
pub const EMBLEM_COST: i32 = 5_000_000;
/// The emblem frames in the client's `UI/GuildMark.img/BackGround`.
const EMBLEM_BACKGROUNDS: RangeInclusive<u16> = 1000..=1030;
/// The marks in each of `UI/GuildMark.img/Mark`'s folders: Animal, Plant,
/// Pattern, Letter and Etc. The client's emblem maker steps through each
/// folder's ids up to the last.
const EMBLEM_MARKS: [RangeInclusive<u16>; 5] = [
    2000..=2020,
    3000..=3006,
    4000..=4021,
    5000..=5035,
    9000..=9026,
];
/// The colours of each frame and mark.
const EMBLEM_COLORS: RangeInclusive<u8> = 1..=16;

/// The emblem shown beside the guild name. All zeros means none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Emblem {
    pub background: u16,
    pub background_color: u8,
    pub mark: u16,
    pub mark_color: u8,
}

impl Emblem {
    /// Whether the client's emblem maker could have made this emblem: a
    /// frame and mark it has, each in one of its colours.
    pub fn is_valid(&self) -> bool {
        EMBLEM_BACKGROUNDS.contains(&self.background)
            && EMBLEM_MARKS.iter().any(|marks| marks.contains(&self.mark))
            && EMBLEM_COLORS.contains(&self.background_color)
            && EMBLEM_COLORS.contains(&self.mark_color)
    }
}

/// What others see of a member's guild: the name under theirs and the
/// emblem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildTag {
    pub id: i32,
    pub name: String,
    pub emblem: Emblem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildMember {
    pub id: i32,
    pub name: String,
    pub job: i16,
    pub level: u8,
    pub rank: u8,
    /// Logged in on any channel; filled in by the world directory.
    pub online: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Guild {
    pub id: i32,
    pub name: String,
    /// Titles of ranks 1 to 5.
    pub titles: [String; 5],
    pub capacity: u8,
    pub emblem: Emblem,
    pub notice: String,
    pub points: i32,
    pub alliance_id: i32,
    /// The master first, as the client expects.
    pub members: Vec<GuildMember>,
}

impl Guild {
    pub fn tag(&self) -> GuildTag {
        GuildTag {
            id: self.id,
            name: self.name.clone(),
            emblem: self.emblem,
        }
    }

    pub fn member(&self, id: i32) -> Option<&GuildMember> {
        self.members.iter().find(|member| member.id == id)
    }

    pub fn is_master(&self, id: i32) -> bool {
        self.member(id)
            .is_some_and(|member| member.rank == MASTER_RANK)
    }

    /// Whether member `by` may change the emblem: only the master.
    pub fn may_set_emblem(&self, by: i32) -> bool {
        self.is_master(by)
    }

    /// Whether member `by` may grow the guild a step from `from` members,
    /// the capacity they were quoted: only the master, below the most.
    pub fn may_grow(&self, by: i32, from: u8) -> bool {
        self.is_master(by) && self.capacity == from && from < MAX_CAPACITY
    }

    /// The lowest rank with a title, which the client lets nobody pass
    /// (FUN_00a3930b): 3 while title 4 is empty, 4 while title 5 is.
    pub fn lowest_rank(&self) -> u8 {
        lowest_rank(&self.titles)
    }

    /// Whether member `by` may invite someone: a master or jr. master, while
    /// there is room.
    pub fn may_invite(&self, by: i32) -> bool {
        self.member(by)
            .is_some_and(|member| member.rank <= JR_MASTER_RANK)
            && self.members.len() < usize::from(self.capacity)
    }

    /// Whether member `by` may put `member_id` out. Masters and jr. masters
    /// may, but nobody expels the master, and only the master expels a
    /// jr. master.
    pub fn may_expel(&self, by: i32, member_id: i32) -> bool {
        let (Some(by), Some(member)) = (self.member(by), self.member(member_id)) else {
            return false;
        };
        by.id != member.id
            && by.rank <= JR_MASTER_RANK
            && member.rank != MASTER_RANK
            && (member.rank != JR_MASTER_RANK || by.rank == MASTER_RANK)
    }
}

impl Guild {
    /// Whether member `by` may move `member_id` to `rank`, one step up or
    /// down, as the client's Up and Down buttons allow (FUN_0090f17c): the
    /// master moves anyone between rank 2 and the lowest, a jr. master
    /// anyone between rank 3 and the lowest. Down stops at the lowest rank
    /// with a title; anyone below it may still go up.
    pub fn may_change_rank(&self, by: i32, member_id: i32, rank: u8) -> bool {
        let (Some(by), Some(member)) = (self.member(by), self.member(member_id)) else {
            return false;
        };
        by.rank <= JR_MASTER_RANK
            && member.rank > by.rank
            && rank > by.rank
            && rank.abs_diff(member.rank) == 1
            && (rank < member.rank || rank <= self.lowest_rank())
    }

    /// Whether member `by` may give the ranks these titles. Only the master
    /// edits them, and only as the client's rank window does
    /// (FUN_00916e8d): no rank loses its title while members hold it, and
    /// title 5 needs title 4. `forbids` is the forbidden-word check guild
    /// names get. The client sends the titles it didn't edit as they were,
    /// so those aren't checked again; the default "Master" holds a
    /// forbidden word.
    pub fn may_set_titles(
        &self,
        by: i32,
        titles: &[String; 5],
        forbids: impl Fn(&str) -> bool,
    ) -> bool {
        let lowest = lowest_rank(titles);
        self.member(by).is_some_and(|by| by.rank == MASTER_RANK)
            && titles
                .iter()
                .zip(&self.titles)
                .enumerate()
                .all(|(i, (title, old))| {
                    title == old || (title_is_valid(i, title) && !forbids(title))
                })
            && !(titles[3].is_empty() && !titles[4].is_empty())
            && self.members.iter().all(|member| member.rank <= lowest)
    }

    /// Whether member `by` may post this notice: a master or jr. master,
    /// with text the client's prompt can send, which refuses control
    /// characters other than CR and LF (FUN_007a06f3).
    pub fn may_set_notice(&self, by: i32, notice: &str) -> bool {
        self.member(by)
            .is_some_and(|member| member.rank <= JR_MASTER_RANK)
            && !notice.is_empty()
            && notice.len() <= NOTICE_MAX_LEN
            && !notice.chars().any(|c| c < ' ' && c != '\r' && c != '\n')
    }
}

fn lowest_rank(titles: &[String; 5]) -> u8 {
    if titles[3].is_empty() {
        3
    } else if titles[4].is_empty() {
        4
    } else {
        5
    }
}

/// Ranks 1 to 3 need a title and ranks 4 and 5 may have none. A title
/// is checked as a guild name (FUN_00916e8d calls FUN_0079fc74) in a box
/// that takes 10.
fn title_is_valid(index: usize, title: &str) -> bool {
    (title.is_empty() && index >= 3) || (title.len() <= 10 && name_is_valid(title))
}

/// Guild names follow the character name rules (FUN_0079fc74): 4 to 12
/// ASCII letters or digits, with fewer than four of 'I' and 'l' between
/// them.
pub fn name_is_valid(name: &str) -> bool {
    (4..=12).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric())
        && name.bytes().filter(|&b| b == b'I' || b == b'l').count() < 4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guild(ranks: &[u8]) -> Guild {
        Guild {
            id: 4,
            name: "Testers".into(),
            titles: DEFAULT_TITLES.map(String::from),
            capacity: 4,
            emblem: Emblem::default(),
            notice: String::new(),
            points: 0,
            alliance_id: 0,
            members: ranks
                .iter()
                .enumerate()
                .map(|(i, &rank)| GuildMember {
                    id: i as i32 + 1,
                    name: format!("Member{i}"),
                    job: 0,
                    level: 10,
                    rank,
                    online: true,
                })
                .collect(),
        }
    }

    #[test]
    fn masters_and_jr_masters_invite_while_there_is_room() {
        let testers = guild(&[1, 2, 5]);
        assert!(testers.may_invite(1));
        assert!(testers.may_invite(2));
        assert!(!testers.may_invite(3));
        assert!(!testers.may_invite(9));
        assert!(!guild(&[1, 2, 5, 5]).may_invite(1));
    }

    #[test]
    fn the_master_grows_the_guild_by_steps_to_100() {
        let mut testers = guild(&[1, 2]);
        testers.capacity = 10;
        assert!(testers.is_master(1) && !testers.is_master(2) && !testers.is_master(9));
        assert!(testers.may_grow(1, 10));
        assert!(!testers.may_grow(2, 10));
        assert!(!testers.may_grow(1, 15), "quoted for another capacity");
        testers.capacity = 95;
        assert!(testers.may_grow(1, 95));
        testers.capacity = MAX_CAPACITY;
        assert!(!testers.may_grow(1, MAX_CAPACITY));
    }

    #[test]
    fn growing_costs_more_up_to_30_members() {
        let costs = [10, 15, 20, 25, 30, 35, 95].map(capacity_cost);
        assert_eq!(
            costs,
            [500_000, 500_000, 1_500_000, 2_500_000, 3_500_000, 5_000_000, 5_000_000]
        );
    }

    #[test]
    fn only_the_master_expels_jr_masters_and_nobody_the_master() {
        let testers = guild(&[1, 2, 2, 5, 3]);
        assert!(testers.may_expel(1, 2));
        assert!(testers.may_expel(1, 4));
        assert!(testers.may_expel(2, 4));
        assert!(!testers.may_expel(2, 3));
        assert!(!testers.may_expel(2, 1));
        assert!(!testers.may_expel(4, 5));
        assert!(!testers.may_expel(1, 1));
        assert!(!testers.may_expel(1, 9));
    }

    fn titles(titles: [&str; 5]) -> [String; 5] {
        titles.map(String::from)
    }

    fn forbids_nothing(_: &str) -> bool {
        false
    }

    #[test]
    fn the_lowest_rank_is_the_last_with_a_title() {
        let mut testers = guild(&[1]);
        assert_eq!(testers.lowest_rank(), 5);
        testers.titles = titles(["Master", "Jr. Master", "Member", "Rookie", ""]);
        assert_eq!(testers.lowest_rank(), 4);
        testers.titles = titles(["Master", "Jr. Master", "Member", "", ""]);
        assert_eq!(testers.lowest_rank(), 3);
    }

    #[test]
    fn ranks_move_one_step_below_the_mover() {
        let testers = guild(&[1, 2, 3, 5, 2]);
        assert!(testers.may_change_rank(1, 2, 3));
        assert!(testers.may_change_rank(1, 3, 2));
        assert!(testers.may_change_rank(1, 4, 4));
        assert!(testers.may_change_rank(2, 3, 4));
        assert!(testers.may_change_rank(2, 4, 4));
        // Nobody becomes master, skips a rank, passes the lowest or moves
        // themselves, and a jr. master moves nobody to or from rank 2.
        assert!(!testers.may_change_rank(1, 2, 1));
        assert!(!testers.may_change_rank(1, 3, 5));
        assert!(!testers.may_change_rank(1, 4, 6));
        assert!(!testers.may_change_rank(1, 1, 2));
        assert!(!testers.may_change_rank(2, 3, 2));
        assert!(!testers.may_change_rank(2, 5, 3));
        assert!(!testers.may_change_rank(3, 4, 4));
        assert!(!testers.may_change_rank(1, 9, 3));

        // Below the lowest rank with a title, up but not down.
        let mut short = guild(&[1, 3, 4]);
        short.titles = titles(["Master", "Jr. Master", "Member", "", ""]);
        assert!(!short.may_change_rank(1, 2, 4));
        assert!(short.may_change_rank(1, 3, 3));
        assert!(!short.may_change_rank(1, 3, 5));
    }

    #[test]
    fn the_master_sets_titles_the_client_could_show() {
        let testers = guild(&[1, 2, 3]);
        assert!(testers.may_set_titles(
            1,
            &titles(["Boss", "Deputy", "Grunt", "", ""]),
            forbids_nothing
        ));
        assert!(testers.may_set_titles(
            1,
            &titles(["Boss", "Deputy", "Grunt", "Newbie", ""]),
            forbids_nothing
        ));
        assert!(testers.may_set_titles(
            1,
            &titles(["Boss", "Deputy", "Grunt", "Newbie", "Rookie"]),
            forbids_nothing
        ));
        // Untouched titles go back as they were, which the client's own
        // check would refuse.
        assert!(testers.may_set_titles(
            1,
            &titles(["Boss", "Jr. Master", "Grunt", "", ""]),
            forbids_nothing
        ));
        assert!(!testers.may_set_titles(
            2,
            &titles(["Boss", "Deputy", "Grunt", "", ""]),
            forbids_nothing
        ));
        assert!(!testers.may_set_titles(
            1,
            &titles(["Boss", "", "Grunt", "", ""]),
            forbids_nothing
        ));
        assert!(!testers.may_set_titles(
            1,
            &titles(["Bos", "Deputy", "Grunt", "", ""]),
            forbids_nothing
        ));
        assert!(!testers.may_set_titles(
            1,
            &titles(["Boss", "Jr Master", "Grunt", "", ""]),
            forbids_nothing
        ));
        assert!(!testers.may_set_titles(
            1,
            &titles(["Boss", "Deputy", "Grunt", "New", ""]),
            forbids_nothing
        ));
        assert!(!testers.may_set_titles(
            1,
            &titles(["Boss", "Deputy", "Grunt", "", "Newbie"]),
            forbids_nothing
        ));
        assert!(!testers.may_set_titles(
            1,
            &titles(["Boss", "Deputy", "Elevenchars", "", ""]),
            forbids_nothing
        ));

        // Words a guild name can't hold, but untouched titles keep theirs.
        let test = |title: &str| title.to_ascii_lowercase().contains("test");
        assert!(!testers.may_set_titles(1, &titles(["Boss", "Deputy", "Tester", "", ""]), test));
        assert!(testers.may_set_titles(1, &titles(["Boss", "Deputy", "Grunt", "", ""]), test));
        let mut tested = guild(&[1, 2, 3]);
        tested.titles = titles(["Master", "Jr. Master", "Testers", "", ""]);
        assert!(tested.may_set_titles(1, &titles(["Boss", "Deputy", "Testers", "", ""]), test));

        let full = guild(&[1, 4, 5]);
        assert!(!full.may_set_titles(
            1,
            &titles(["Boss", "Deputy", "Grunt", "Newbie", ""]),
            forbids_nothing
        ));
        assert!(guild(&[1, 4]).may_set_titles(
            1,
            &titles(["Boss", "Deputy", "Grunt", "Newbie", ""]),
            forbids_nothing
        ));
    }

    #[test]
    fn masters_and_jr_masters_post_notices() {
        let testers = guild(&[1, 2, 3]);
        assert!(testers.may_set_notice(1, "Hello"));
        assert!(testers.may_set_notice(2, &"x".repeat(NOTICE_MAX_LEN)));
        assert!(!testers.may_set_notice(3, "Hello"));
        assert!(!testers.may_set_notice(1, ""));
        assert!(!testers.may_set_notice(1, &"x".repeat(NOTICE_MAX_LEN + 1)));
        assert!(!testers.may_set_notice(1, "hi\0x"));
        assert!(!testers.may_set_notice(1, "a\tb"));
        assert!(testers.may_set_notice(1, "a\r\nb"));
    }

    #[test]
    fn only_the_master_changes_the_emblem() {
        let testers = guild(&[1, 2, 3]);
        assert!(testers.may_set_emblem(1));
        assert!(!testers.may_set_emblem(2));
        assert!(!testers.may_set_emblem(3));
        assert!(!testers.may_set_emblem(9));
    }

    #[test]
    fn emblems_are_those_the_maker_makes() {
        let emblem = |background, background_color, mark, mark_color| Emblem {
            background,
            background_color,
            mark,
            mark_color,
        };
        assert!(emblem(1000, 1, 2000, 1).is_valid());
        assert!(emblem(1030, 16, 9026, 16).is_valid());
        for mark in [2020, 3000, 3006, 4021, 5000, 5035, 9000] {
            assert!(emblem(1005, 3, mark, 7).is_valid(), "{mark}");
        }
        assert!(!Emblem::default().is_valid());
        assert!(!emblem(999, 1, 2000, 1).is_valid());
        assert!(!emblem(1031, 1, 2000, 1).is_valid());
        for mark in [1999, 2021, 3007, 4022, 5036, 6000, 8999, 9027] {
            assert!(!emblem(1000, 1, mark, 1).is_valid(), "{mark}");
        }
        assert!(!emblem(1000, 0, 2000, 1).is_valid());
        assert!(!emblem(1000, 17, 2000, 1).is_valid());
        assert!(!emblem(1000, 1, 2000, 0).is_valid());
        assert!(!emblem(1000, 1, 2000, 17).is_valid());
    }

    #[test]
    fn names_are_four_to_twelve_letters_or_digits() {
        assert!(name_is_valid("Testers"));
        assert!(name_is_valid("Guild123"));
        assert!(!name_is_valid("abc"));
        assert!(!name_is_valid("thirteenchars"));
        assert!(!name_is_valid("two words"));
        assert!(!name_is_valid("IllIX"));
        assert!(name_is_valid("IllX"));
    }
}
