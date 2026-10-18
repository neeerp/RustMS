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
}

/// Guild names follow the character name rules: 4 to 12 ASCII letters or
/// digits.
pub fn name_is_valid(name: &str) -> bool {
    (4..=12).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_four_to_twelve_letters_or_digits() {
        assert!(name_is_valid("Testers"));
        assert!(name_is_valid("Guild123"));
        assert!(!name_is_valid("abc"));
        assert!(!name_is_valid("thirteenchars"));
        assert!(!name_is_valid("two words"));
    }
}
