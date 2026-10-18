//! Pets: cash items (500xxxx) that follow their owner once summoned. The
//! client keeps the summoned pets in a list that closes up when one leaves
//! (v83 FUN_0092eb37); the server keeps the same list so the indexes it
//! sends match.

/// At most three pets follow a player.
pub const MAX_PETS: usize = 3;

/// Follow the Lead (beginner, Noblesse, Legend), without which a player
/// keeps one pet out at a time.
pub const FOLLOW_THE_LEAD: [i32; 3] = [8, 10000018, 20000024];

/// What a pet item carries besides the item itself (v83 GW_ItemSlotPet).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PetData {
    pub name: String,
    pub level: u8,
    pub closeness: i16,
    pub fullness: u8,
    /// `petSkill` bits; 0x100 lets the pet speak at events.
    pub skill: u16,
    pub attribute: i16,
}

impl PetData {
    /// A pet fresh out of the Cash Shop, called `name`.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            level: 1,
            closeness: 0,
            fullness: 100,
            skill: 0,
            attribute: 0,
        }
    }
}

pub fn is_pet(item_id: i32) -> bool {
    item_id / 10_000 == 500
}

/// A summoned pet as the field shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldPet {
    pub item_id: i32,
    pub name: String,
    pub serial: i64,
    pub x: i16,
    pub y: i16,
    pub stance: u8,
    pub foothold: i16,
}

/// One change to a player's pet list, at its position in the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PetChange {
    /// The pet now at `index`; those after it moved up one.
    Summoned { index: u8, serial: i64 },
    /// The pet that was at `index`; those after it moved down one.
    Dismissed { index: u8 },
}

/// Summon the pet `serial` into `pets`, the summoned serials in list order,
/// or dismiss it if it is out. Without Follow the Lead (`lead`) a new pet
/// replaces the one out; with it, `boss` puts the new pet first. Nothing
/// changes if three pets are already out.
pub fn toggle(pets: &mut Vec<i64>, serial: i64, lead: bool, boss: bool) -> Vec<PetChange> {
    if let Some(index) = pets.iter().position(|&pet| pet == serial) {
        pets.remove(index);
        return vec![PetChange::Dismissed { index: index as u8 }];
    }
    let mut changes = Vec::new();
    if !lead {
        while !pets.is_empty() {
            pets.remove(0);
            changes.push(PetChange::Dismissed { index: 0 });
        }
    }
    if pets.len() >= MAX_PETS {
        return changes;
    }
    let index = if boss { 0 } else { pets.len() };
    pets.insert(index, serial);
    changes.push(PetChange::Summoned {
        index: index as u8,
        serial,
    });
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pet_out_goes_back_and_the_rest_close_up() {
        let mut pets = vec![1, 2, 3];
        assert_eq!(
            toggle(&mut pets, 1, true, false),
            [PetChange::Dismissed { index: 0 }]
        );
        assert_eq!(pets, [2, 3]);
    }

    #[test]
    fn without_follow_the_lead_a_new_pet_replaces_the_old() {
        let mut pets = vec![1];
        assert_eq!(
            toggle(&mut pets, 2, false, false),
            [
                PetChange::Dismissed { index: 0 },
                PetChange::Summoned {
                    index: 0,
                    serial: 2
                }
            ]
        );
        assert_eq!(pets, [2]);
    }

    #[test]
    fn with_follow_the_lead_pets_join_the_end_or_lead() {
        let mut pets = vec![1];
        toggle(&mut pets, 2, true, false);
        assert_eq!(pets, [1, 2]);
        assert_eq!(
            toggle(&mut pets, 3, true, true),
            [PetChange::Summoned {
                index: 0,
                serial: 3
            }]
        );
        assert_eq!(pets, [3, 1, 2]);
        assert_eq!(toggle(&mut pets, 4, true, false), []);
        assert_eq!(pets, [3, 1, 2]);
    }
}
