//! Pets: cash items (500xxxx) that follow their owner once summoned. The
//! client keeps the summoned pets in a list that closes up when one leaves
//! (v83 FUN_0092eb37); the server keeps the same list so the indexes it
//! sends match.

/// At most three pets follow a player.
pub const MAX_PETS: usize = 3;

/// Follow the Lead (beginner, Noblesse, Legend), without which a player
/// keeps one pet out at a time.
pub const FOLLOW_THE_LEAD: [i32; 3] = [8, 10000018, 20000024];

/// The closeness a pet of each level needs to reach the next (Cosmic's
/// ExpTable; the v83 client does not carry it). Level 30 is the last.
const CLOSENESS: [i16; 30] = [
    1, 1, 3, 6, 14, 31, 60, 108, 181, 287, 434, 632, 891, 1224, 1642, 2161, 2793, 3557, 4467, 5542,
    6801, 8263, 9950, 11882, 14084, 16578, 19391, 22547, 26074, 30000,
];

pub const MAX_CLOSENESS: i16 = 30000;

/// A pet this full won't eat, and loses closeness for being fed.
pub const MAX_FULLNESS: u8 = 100;

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

impl PetData {
    /// Add `amount` closeness, which may be negative, keeping it between
    /// none and the most a pet can have, and move its level to match. True
    /// if the level rose.
    pub fn add_closeness(&mut self, amount: i16) -> bool {
        let before = self.level;
        self.closeness = self
            .closeness
            .saturating_add(amount)
            .clamp(0, MAX_CLOSENESS);
        while let Some(&needed) = CLOSENESS.get(usize::from(self.level)) {
            if self.closeness < needed {
                break;
            }
            self.level += 1;
        }
        while self.level > 1 && self.closeness < CLOSENESS[usize::from(self.level) - 1] {
            self.level -= 1;
        }
        self.level > before
    }

    /// Feed the pet food worth `fullness` (Cosmic's PetFoodHandler). A pet
    /// that isn't full eats it, and grows closer if it was hungry (75 or
    /// less); a full one loses a point of closeness.
    pub fn feed(&mut self, fullness: i32) -> Fed {
        if self.fullness >= MAX_FULLNESS {
            self.add_closeness(-1);
            return Fed {
                enjoyed: false,
                levelled: false,
            };
        }
        let hungry = self.fullness <= 75;
        let fed = i32::from(self.fullness) + fullness.max(0);
        self.fullness = fed.min(i32::from(MAX_FULLNESS)) as u8;
        Fed {
            enjoyed: true,
            levelled: hungry && self.add_closeness(1),
        }
    }

    /// Feed the pet a cash food (524xxxx) worth `closeness` (Cosmic's
    /// UseCashItemHandler): full or not, it eats, fills up and grows closer.
    /// True if its level rose.
    pub fn feed_cash(&mut self, closeness: i16) -> bool {
        self.fullness = MAX_FULLNESS;
        self.add_closeness(closeness)
    }

    /// The pet grows `hunger` hungrier. True if that left it at 5 or less:
    /// it goes home, and comes back at 15.
    pub fn grow_hungry(&mut self, hunger: i32) -> bool {
        let left = i32::from(self.fullness) - hunger.max(0);
        if left <= 5 {
            self.fullness = 15;
            return true;
        }
        self.fullness = left as u8;
        false
    }
}

/// What feeding a pet did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fed {
    /// It ate; a full pet refuses.
    pub enjoyed: bool,
    pub levelled: bool,
}

/// One of the commands a pet knows (`Item.nx Pet/<id>.img/interact/<n>`).
/// The client matches the words; the server rolls whether it obeys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PetCommand {
    /// The pet levels it answers to (`l0`, `l1`).
    pub levels: (u8, u8),
    /// Percent chance it obeys (`prob`).
    pub prob: i32,
    /// The closeness obeying earns (`inc`).
    pub inc: i16,
}

impl PetCommand {
    /// Whether `pet` obeys, given `roll` in 0..100. A pet never obeys a
    /// command outside its levels.
    pub fn obeyed(&self, pet: &PetData, roll: i32) -> bool {
        (self.levels.0..=self.levels.1).contains(&pet.level) && roll < self.prob
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

    #[test]
    fn closeness_raises_the_level() {
        let mut pet = PetData::new("Baby Tiger");
        assert!(pet.add_closeness(1));
        assert_eq!((pet.level, pet.closeness), (2, 1));
        assert!(!pet.add_closeness(1));
        assert_eq!((pet.level, pet.closeness), (2, 2));
        assert!(pet.add_closeness(12));
        assert_eq!((pet.level, pet.closeness), (5, 14));
        pet.add_closeness(i16::MAX);
        assert_eq!((pet.level, pet.closeness), (30, MAX_CLOSENESS));
        assert!(!pet.add_closeness(3));
        assert_eq!((pet.level, pet.closeness), (30, MAX_CLOSENESS));
    }

    #[test]
    fn losing_closeness_lowers_the_level() {
        let mut pet = PetData::new("Baby Tiger");
        pet.add_closeness(14);
        assert_eq!(pet.level, 5);
        assert!(!pet.add_closeness(-1));
        assert_eq!((pet.level, pet.closeness), (4, 13));
        pet.add_closeness(-20);
        assert_eq!((pet.level, pet.closeness), (1, 0));
    }

    #[test]
    fn a_hungry_pet_eats_and_grows_closer() {
        let mut pet = PetData::new("Baby Tiger");
        pet.fullness = 75;
        let fed = pet.feed(30);
        assert_eq!(
            fed,
            Fed {
                enjoyed: true,
                levelled: true
            }
        );
        assert_eq!((pet.fullness, pet.closeness, pet.level), (100, 1, 2));
    }

    #[test]
    fn a_pet_that_isnt_hungry_just_eats() {
        let mut pet = PetData::new("Baby Tiger");
        pet.fullness = 76;
        assert!(pet.feed(30).enjoyed);
        assert_eq!((pet.fullness, pet.closeness), (100, 0));
    }

    #[test]
    fn a_full_pet_refuses_and_grows_distant() {
        let mut pet = PetData::new("Baby Tiger");
        pet.add_closeness(1);
        let fed = pet.feed(30);
        assert!(!fed.enjoyed && !fed.levelled);
        assert_eq!((pet.fullness, pet.closeness, pet.level), (100, 0, 1));
    }

    #[test]
    fn cash_food_fills_a_pet_full_or_not() {
        let mut pet = PetData::new("Baby Tiger");
        assert!(pet.feed_cash(100));
        assert_eq!((pet.fullness, pet.closeness, pet.level), (100, 100, 7));
        pet.fullness = 20;
        assert!(!pet.feed_cash(1));
        assert_eq!((pet.fullness, pet.closeness), (100, 101));
    }

    #[test]
    fn a_starving_pet_goes_home() {
        let mut pet = PetData::new("Baby Tiger");
        pet.fullness = 9;
        assert!(!pet.grow_hungry(3));
        assert_eq!(pet.fullness, 6);
        assert!(pet.grow_hungry(3));
        assert_eq!(pet.fullness, 15);
    }

    #[test]
    fn a_pet_obeys_only_at_its_levels() {
        let command = PetCommand {
            levels: (1, 9),
            prob: 30,
            inc: 1,
        };
        let mut pet = PetData::new("Baby Tiger");
        assert!(command.obeyed(&pet, 29));
        assert!(!command.obeyed(&pet, 30));
        pet.level = 10;
        assert!(!command.obeyed(&pet, 0));
    }
}
