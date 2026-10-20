//! Skill macros (report 206): up to five named lists of three skills, which
//! the v83 client runs one after another from a type-8 key. The client
//! keeps them by place, so a type-8 key's action is a place in the list,
//! and sends the whole list after every edit.

/// The most macros a character keeps; the v83 client reads no more.
pub const MAX_MACROS: usize = 5;
/// The longest name, in bytes (v83 FUN_00631d45).
pub const MAX_NAME_BYTES: usize = 12;

/// The skills held down to charge (v83 FUN_004fb08f), which no macro holds.
const KEYDOWN: [i32; 16] = [
    5101004, 2221001, 1121001, 1221001, 1321001, 2121001, 2321001, 3121004, 3221001, 5201002,
    5221004, 13111002, 14111006, 15101003, 22121000, 22151001,
];
/// Teleport, Flash Jump, Meso Explosion and 11101005, which no macro holds
/// either (v83 FUN_004fb292).
const MOVES: [i32; 9] = [
    2101002, 2201002, 2301001, 12101003, 22101001, 4111006, 14101004, 4211006, 11101005,
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkillMacro {
    pub name: String,
    /// Used without its name said aloud; the wire byte is 1 for silent.
    pub silent: bool,
    /// The skills in order; 0 is an empty slot.
    pub skills: [i32; 3],
}

/// A character's macros, by place in the list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkillMacros(Vec<SkillMacro>);

/// Why a list from the client was turned away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacroRefusal {
    TooMany,
    NameTooLong,
}

impl SkillMacros {
    /// The list as saved, cut to `MAX_MACROS`.
    pub fn new(mut macros: Vec<SkillMacro>) -> Self {
        macros.truncate(MAX_MACROS);
        Self(macros)
    }

    pub fn macros(&self) -> &[SkillMacro] {
        &self.0
    }

    /// Take the client's whole list in place of this one.
    pub fn replace(&mut self, macros: Vec<SkillMacro>) -> Result<(), MacroRefusal> {
        if macros.len() > MAX_MACROS {
            return Err(MacroRefusal::TooMany);
        }
        if macros.iter().any(|m| m.name.len() > MAX_NAME_BYTES) {
            return Err(MacroRefusal::NameTooLong);
        }
        self.0 = macros;
        Ok(())
    }
}

/// Whether a macro may hold `skill_id` (v83 FUN_004fb292): not a passive
/// skill (digit 0) or a digit-9 one, not one held down to charge or a
/// movement skill, and not one with a cooldown at any level. 0 is an empty
/// slot.
pub fn may_hold(skill_id: i32, has_cooltime: bool) -> bool {
    if skill_id == 0 {
        return true;
    }
    let digit = skill_id / 1000 % 10;
    digit != 0
        && digit != 9
        && !KEYDOWN.contains(&skill_id)
        && !MOVES.contains(&skill_id)
        && !has_cooltime
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> SkillMacro {
        SkillMacro {
            name: name.into(),
            ..SkillMacro::default()
        }
    }

    #[test]
    fn the_list_is_replaced_whole() {
        let mut macros = SkillMacros::new(vec![named("a"), named("b")]);
        assert_eq!(macros.replace(vec![named("c")]), Ok(()));
        assert_eq!(macros.macros(), [named("c")]);
    }

    #[test]
    fn six_macros_or_a_long_name_are_turned_away() {
        let mut macros = SkillMacros::new(vec![named("kept")]);
        assert_eq!(
            macros.replace(vec![named("a"); 6]),
            Err(MacroRefusal::TooMany)
        );
        assert_eq!(
            macros.replace(vec![named("thirteen byte")]),
            Err(MacroRefusal::NameTooLong)
        );
        assert_eq!(macros.replace(vec![named("twelve bytes")]), Ok(()));
        assert_eq!(SkillMacros::new(vec![named("a"); 7]).macros().len(), 5);
    }

    #[test]
    fn what_a_macro_may_hold() {
        assert!(may_hold(0, false));
        assert!(may_hold(1001004, false), "Power Strike");
        assert!(!may_hold(1000000, false), "a passive skill");
        assert!(!may_hold(1009000, false), "digit 9");
        assert!(!may_hold(2121001, false), "Big Bang is charged");
        assert!(!may_hold(2101002, false), "Teleport");
        assert!(!may_hold(4111006, false), "Flash Jump");
        assert!(!may_hold(4211006, false), "Meso Explosion");
        assert!(!may_hold(1121006, true), "a cooldown");
    }
}
