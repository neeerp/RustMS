//! Guilds: a named group of characters with a master, shown under each
//! member's name and in the client's Guild window.

/// Lowest level that may found a guild. The client doesn't check; this is
/// the HeavenMS default.
pub const MIN_LEVEL: u8 = 10;
/// Mesos it costs to found a guild; the HeavenMS default, not from the
/// client.
pub const CREATE_COST: i32 = 1_500_000;
/// The rank titles a new guild starts with (OdinMS and HeavenMS).
pub const DEFAULT_TITLES: [&str; 5] = ["Master", "Jr. Master", "Member", "", ""];
/// The master's rank; members range from 1 to 5.
pub const MASTER_RANK: u8 = 1;
/// A jr. master, who may invite and expel too.
pub const JR_MASTER_RANK: u8 = 2;
/// The rank new members join at.
pub const MEMBER_RANK: u8 = 5;

/// The emblem shown beside the guild name. All zeros means none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Emblem {
    pub background: u16,
    pub background_color: u8,
    pub mark: u16,
    pub mark_color: u8,
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

/// Guild names follow the character name rules: 4 to 12 ASCII letters or
/// digits.
pub fn name_is_valid(name: &str) -> bool {
    (4..=12).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric())
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

    #[test]
    fn names_are_four_to_twelve_letters_or_digits() {
        assert!(name_is_valid("Testers"));
        assert!(name_is_valid("Guild123"));
        assert!(!name_is_valid("abc"));
        assert!(!name_is_valid("thirteenchars"));
        assert!(!name_is_valid("two words"));
    }
}
