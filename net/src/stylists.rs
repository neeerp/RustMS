//! The stylists: hair salons, plastic surgeons, lens shops and skin care
//! (v83 `hair_*`, `face_*`, `lens_*`, `skin_*` scripts). Each NPC offers
//! services for coupons; a VIP coupon lets the player choose the new look,
//! a regular or experimental one rolls it. Which NPC takes which coupon and
//! the styles each offers come from Cosmic's scripts, as facts.

use game::Player;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    Cut,
    Dye,
    Face,
    /// The face's colour, among the shop's colours.
    Lens,
    /// The face's colour, for one of the one-time lenses the player holds.
    OneTimeLens,
    Skin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    /// The player picks the look on a preview.
    Choose,
    /// The look is rolled among those on offer.
    Random,
}

use Look::*;
use Pick::*;

/// One menu entry of a stylist.
#[derive(Debug)]
pub struct Service {
    pub npc: i32,
    pub look: Look,
    pub coupon: i32,
    pub pick: Pick,
    /// A hair membership coupon that makes a cut free, and is kept.
    pub member: Option<i32>,
    /// Cuts and faces: the styles for men and women at colour 0. Lenses:
    /// the colours on offer.
    male: &'static [i32],
    female: &'static [i32],
}

/// The one-time lens of colour 0; colour `c` is this plus `c`.
pub const ONE_TIME_LENS: i32 = 5_152_100;

const fn s(
    npc: i32,
    look: Look,
    coupon: i32,
    pick: Pick,
    member: i32,
    male: &'static [i32],
    female: &'static [i32],
) -> Service {
    Service {
        npc,
        look,
        coupon,
        pick,
        member: if member == 0 { None } else { Some(member) },
        male,
        female,
    }
}

/// Every stylist's services, each NPC's in its menu's order.
#[rustfmt::skip]
const SERVICES: &[Service] = &[
    s(1012103, Cut, 5150001, Choose, 5420002, &[30060, 30140, 30200, 30210, 30310, 33040, 33100], &[31150, 31300, 31350, 31700, 31740, 34050, 34110]),
    s(1012103, Dye, 5151001, Choose, 0, &[], &[]),
    s(1012104, Cut, 5150000, Random, 0, &[30060, 30140, 30200, 30210, 30310, 30610, 33040, 33100], &[31070, 31080, 31150, 31300, 31350, 31700, 34050, 34110]),
    s(1012104, Cut, 5150010, Random, 0, &[30030, 30140, 30200, 30210, 30310, 30610, 33040, 33100], &[31070, 31150, 31300, 31350, 31430, 31700, 34050, 34110]),
    s(1012104, Dye, 5151000, Random, 0, &[], &[]),
    s(1012117, Cut, 5150040, Random, 0, &[30010, 30070, 30080, 30090, 30100, 30690, 30760, 33000], &[31130, 31530, 31820, 31920, 31940, 34000, 34030]),
    s(1012117, Cut, 5150044, Choose, 0, &[30010, 30070, 30080, 30090, 30100, 30480, 30560, 30690, 30760, 30850, 30890, 30930, 30950], &[31020, 31130, 31510, 31530, 31820, 31860, 31890, 31920, 31940, 31950, 34000]),
    s(1012105, Skin, 5153000, Choose, 0, &[], &[]),
    s(1052004, Face, 5152001, Choose, 0, &[20000, 20001, 20003, 20004, 20005, 20006, 20007, 20008, 20012, 20014, 20015, 20022, 20028, 20031], &[21000, 21001, 21002, 21003, 21004, 21005, 21006, 21007, 21008, 21012, 21013, 21014, 21023, 21026]),
    s(1052005, Face, 5152000, Random, 0, &[20000, 20005, 20008, 20012, 20016, 20022, 20032], &[21000, 21002, 21008, 21014, 21020, 21024, 21029]),
    s(9200100, Lens, 5152010, Random, 0, &[0, 1, 2, 4, 6, 7], &[0, 1, 2, 4, 6, 7]),
    s(9200100, Lens, 5152013, Choose, 0, &[0, 1, 2, 4, 6, 7], &[0, 1, 2, 4, 6, 7]),
    s(9200100, OneTimeLens, 5152100, Choose, 0, &[], &[]),
    s(1052100, Cut, 5150003, Choose, 5420003, &[30040, 30130, 30780, 30850, 30860, 30920, 33040], &[31090, 31140, 31330, 31440, 31760, 31880, 34050]),
    s(1052100, Dye, 5151003, Choose, 0, &[], &[]),
    s(1052101, Cut, 5150002, Random, 0, &[30040, 30130, 30520, 30770, 30780, 30850, 30920, 33040], &[31060, 31140, 31330, 31440, 31520, 31750, 31760, 31880, 34050]),
    s(1052101, Cut, 5150011, Random, 0, &[30130, 30430, 30520, 30770, 30780, 30850, 30920, 33040], &[31060, 31140, 31330, 31520, 31760, 31880, 34010, 34050]),
    s(1052101, Dye, 5151002, Random, 0, &[], &[]),
    s(2010001, Cut, 5150005, Choose, 5420004, &[30230, 30260, 30280, 30340, 30490], &[31110, 31220, 31230, 31630, 31790]),
    s(2010001, Dye, 5151005, Choose, 0, &[], &[]),
    s(2012007, Cut, 5154000, Random, 0, &[30030, 30020, 30000, 30270, 30230], &[31040, 31000, 31250, 31220, 31260]),
    s(2012007, Cut, 5150004, Random, 0, &[30230, 30260, 30280, 30340, 30490, 30530, 30630, 30740], &[31110, 31220, 31230, 31630, 31650, 31710, 31790, 31890, 31930]),
    s(2012007, Cut, 5150013, Random, 0, &[30230, 30280, 30340, 30490, 30530, 30740], &[31110, 31220, 31230, 31710, 31790, 31890, 31930]),
    s(2012007, Dye, 5151004, Random, 0, &[], &[]),
    s(2010002, Face, 5152005, Choose, 0, &[20000, 20001, 20003, 20004, 20005, 20006, 20007, 20008, 20012, 20014, 20022, 20028, 20031], &[21000, 21001, 21002, 21003, 21004, 21005, 21006, 21007, 21008, 21012, 21014, 21023, 21026]),
    s(2012009, Face, 5152004, Random, 0, &[20003, 20011, 20021, 20022, 20023, 20027, 20031], &[21004, 21007, 21010, 21012, 21020, 21021, 21030]),
    s(9200101, Lens, 5152011, Random, 0, &[1, 3, 4, 7], &[1, 3, 4, 7]),
    s(9200101, Lens, 5152014, Choose, 0, &[1, 3, 4, 7], &[1, 3, 4, 7]),
    s(9200101, OneTimeLens, 5152100, Choose, 0, &[], &[]),
    s(2012008, Skin, 5153001, Choose, 0, &[], &[]),
    s(2041007, Cut, 5150007, Choose, 5420005, &[30160, 30190, 30250, 30640, 30660, 30840, 30870, 30990], &[31270, 31290, 31550, 31680, 31810, 31830, 31840, 31870]),
    s(2041007, Dye, 5151007, Choose, 0, &[], &[]),
    s(2041009, Cut, 5150006, Random, 0, &[30190, 30220, 30250, 30540, 30610, 30620, 30640, 30650, 30660, 30840, 30870, 30940, 30990], &[31170, 31270, 31290, 31510, 31540, 31550, 31600, 31640, 31680, 31810, 31830, 31840, 31870]),
    s(2041009, Cut, 5150012, Random, 0, &[30030, 30190, 30220, 30250, 30540, 30610, 30620, 30640, 30650, 30660, 30840, 30990], &[31170, 31270, 31430, 31510, 31540, 31550, 31600, 31680, 31810, 31830, 31840, 31870]),
    s(2041009, Dye, 5151006, Random, 0, &[], &[]),
    s(2040019, Face, 5152006, Random, 0, &[20001, 20003, 20007, 20013, 20021, 20023, 20025], &[21002, 21004, 21006, 21008, 21022, 21027, 21029]),
    s(2041010, Face, 5152007, Choose, 0, &[20000, 20001, 20003, 20004, 20005, 20006, 20007, 20008, 20011, 20012, 20014, 20031], &[21000, 21001, 21002, 21003, 21004, 21005, 21006, 21007, 21008, 21010, 21012, 21014]),
    s(9200102, Lens, 5152012, Random, 0, &[2, 3, 4, 5, 7], &[2, 3, 4, 5, 7]),
    s(9200102, Lens, 5152015, Choose, 0, &[2, 3, 4, 5, 7], &[2, 3, 4, 5, 7]),
    s(9200102, OneTimeLens, 5152100, Choose, 0, &[], &[]),
    s(2041013, Skin, 5153002, Choose, 0, &[], &[]),
    s(2090100, Cut, 5150025, Choose, 5420006, &[30150, 30240, 30370, 30420, 30640, 30710, 30750, 30810], &[31140, 31160, 31180, 31300, 31460, 31470, 31660, 31910]),
    s(2090100, Dye, 5151020, Choose, 0, &[], &[]),
    s(2090101, Cut, 5150024, Random, 0, &[30030, 30150, 30240, 30370, 30420, 30550, 30600, 30640, 30700, 30710, 30720, 30750, 30810, 30830], &[31140, 31160, 31180, 31210, 31300, 31430, 31460, 31470, 31660, 31690, 31800, 31890, 31910, 31940]),
    s(2090101, Dye, 5151019, Random, 0, &[], &[]),
    s(2090102, Skin, 5153006, Choose, 0, &[], &[]),
    s(2090103, Face, 5152028, Choose, 0, &[20000, 20001, 20004, 20005, 20006, 20007, 20009, 20012, 20022, 20028, 20031], &[21000, 21003, 21005, 21006, 21008, 21009, 21011, 21012, 21023, 21024, 21026]),
    s(2090103, Lens, 5152041, Choose, 0, &[0, 1, 3, 5, 6, 7], &[0, 1, 3, 5, 6, 7]),
    s(2090103, OneTimeLens, 5152100, Choose, 0, &[], &[]),
    s(2090104, Face, 5152027, Random, 0, &[20002, 20005, 20007, 20011, 20014, 20017, 20029], &[21001, 21010, 21013, 21018, 21020, 21021, 21030]),
    s(2090104, Lens, 5152042, Random, 0, &[0, 1, 3, 5, 6, 7], &[0, 1, 3, 5, 6, 7]),
    s(2100005, Cut, 5150026, Random, 0, &[30150, 30170, 30180, 30320, 30330, 30410, 30460, 30680, 30800, 30820, 30900], &[31090, 31190, 31330, 31340, 31400, 31420, 31520, 31620, 31650, 31660, 34000]),
    s(2100005, Dye, 5151021, Random, 0, &[], &[]),
    s(2100006, Cut, 5150027, Choose, 0, &[30150, 30170, 30180, 30320, 30330, 30410, 30460, 30820, 30900], &[31040, 31090, 31190, 31330, 31340, 31400, 31420, 31620, 31660]),
    s(2100006, Dye, 5151022, Choose, 0, &[], &[]),
    s(2100007, Skin, 5153007, Choose, 0, &[], &[]),
    s(2100008, Face, 5152030, Choose, 0, &[20000, 20004, 20005, 20012, 20013, 20031], &[21000, 21003, 21006, 21009, 21012, 21024]),
    s(2100008, Lens, 5152047, Choose, 0, &[0, 1, 3, 6, 7], &[0, 1, 3, 6, 7]),
    s(2100008, OneTimeLens, 5152100, Choose, 0, &[], &[]),
    s(2100009, Face, 5152029, Random, 0, &[20001, 20003, 20009, 20010, 20025, 20031], &[21002, 21009, 21011, 21013, 21016, 21029, 21030]),
    s(2100009, Lens, 5152048, Random, 0, &[0, 1, 3, 6, 7], &[0, 1, 3, 6, 7]),
    s(9120100, Cut, 5150009, Choose, 0, &[30260, 30280, 30340, 30710, 30780, 30800, 30810, 30820, 30920], &[31000, 31030, 31100, 31350, 31460, 31550, 31770, 31790, 31850]),
    s(9120100, Dye, 5151009, Choose, 0, &[], &[]),
    s(9120101, Cut, 5150008, Random, 0, &[30260, 30280, 30340, 30360, 30710, 30780, 30790, 30800, 30810, 30820, 30920], &[31350, 31410, 31460, 31540, 31550, 31710, 31720, 31770, 31790, 31800, 31850, 34000]),
    s(9120101, Dye, 5151008, Random, 0, &[], &[]),
    s(9120102, Face, 5152009, Choose, 0, &[20000, 20004, 20005, 20012, 20020, 20031], &[21000, 21003, 21006, 21012, 21021, 21024]),
    s(9120102, Lens, 5152045, Choose, 0, &[0, 1, 2, 3, 4, 5, 7], &[0, 1, 2, 3, 4, 5, 7]),
    s(9120102, OneTimeLens, 5152100, Choose, 0, &[], &[]),
    s(9120103, Face, 5152008, Random, 0, &[20000, 20016, 20019, 20020, 20021, 20024, 20026], &[21000, 21002, 21009, 21016, 21022, 21025, 21027]),
    s(9120103, Lens, 5152046, Random, 0, &[0, 1, 2, 3, 4, 5, 7], &[0, 1, 2, 3, 4, 5, 7]),
    s(9201015, Cut, 5150020, Choose, 5420000, &[30050, 30300, 30410, 30450, 30510, 30570, 30580, 30590, 30660, 30910], &[31150, 31220, 31260, 31310, 31420, 31480, 31490, 31580, 31590, 31610, 31630]),
    s(9201015, Dye, 5151017, Choose, 0, &[], &[]),
    s(9201016, Cut, 5150019, Random, 0, &[30000, 30020, 30110, 30130, 30160, 30190, 30240, 30270, 30430], &[31000, 31030, 31050, 31070, 31090, 31150, 31310, 31910, 34010]),
    s(9201016, Dye, 5151016, Random, 0, &[], &[]),
    s(9201017, Lens, 5152025, Random, 0, &[0, 1, 3, 4, 5, 7], &[0, 1, 3, 4, 5, 7]),
    s(9201017, Lens, 5152026, Choose, 0, &[0, 1, 3, 4, 5, 7], &[0, 1, 3, 4, 5, 7]),
    s(9201017, OneTimeLens, 5152100, Choose, 0, &[], &[]),
    s(9201018, Face, 5152022, Choose, 0, &[20000, 20001, 20003, 20004, 20005, 20006, 20007, 20008, 20018, 20019], &[21001, 21002, 21003, 21004, 21005, 21006, 21007, 21012, 21018, 21019]),
    s(9201019, Face, 5152021, Random, 0, &[20002, 20005, 20007, 20011, 20014, 20027, 20029], &[21001, 21005, 21007, 21017, 21018, 21020, 21022]),
    s(9201061, Lens, 5152035, Random, 0, &[1, 2, 3, 4, 5, 6, 7], &[1, 2, 3, 4, 5, 6, 7]),
    s(9201062, Lens, 5152036, Choose, 0, &[1, 2, 3, 4, 5, 6, 7], &[1, 2, 3, 4, 5, 6, 7]),
    s(9201062, OneTimeLens, 5152100, Choose, 0, &[], &[]),
    s(9201063, Cut, 5150030, Random, 0, &[30250, 30400, 30430, 30440, 30490, 30730, 30830, 30870, 30880, 33100], &[31320, 31450, 31560, 31570, 31690, 31720, 31730, 31830, 34010]),
    s(9201063, Dye, 5151025, Random, 0, &[], &[]),
    s(9201064, Cut, 5150031, Choose, 5420001, &[30250, 30490, 30730, 30870, 30880, 33100], &[31320, 31450, 31560, 31730, 31830]),
    s(9201064, Dye, 5151026, Choose, 0, &[], &[]),
    s(9201065, Skin, 5153009, Choose, 0, &[], &[]),
    s(9201069, Face, 5152034, Choose, 0, &[20000, 20001, 20003, 20004, 20005, 20006, 20008, 20012, 20031], &[21001, 21002, 21003, 21004, 21005, 21006, 21008, 21012, 21016]),
    s(9201070, Face, 5152033, Random, 0, &[20001, 20008, 20011, 20013, 20024, 20029, 20032], &[21000, 21007, 21011, 21012, 21017, 21020, 21022]),
    s(9270023, Face, 5152037, Random, 0, &[20002, 20005, 20006, 20013, 20017, 20021, 20024], &[21002, 21003, 21014, 21016, 21017, 21021, 21027]),
    s(9270024, Face, 5152038, Choose, 0, &[20005, 20012, 20013, 20020, 20021, 20026], &[21006, 21009, 21011, 21012, 21021, 21025]),
    s(9270025, Skin, 5153010, Choose, 0, &[], &[]),
    s(9270026, Lens, 5152039, Random, 0, &[0, 1, 2, 3, 4, 5, 6, 7], &[0, 1, 2, 3, 4, 5, 6, 7]),
    s(9270026, Lens, 5152040, Choose, 0, &[2, 3, 4, 7], &[2, 3, 4, 7]),
    s(9270026, OneTimeLens, 5152100, Choose, 0, &[], &[]),
    s(9270036, Cut, 5150033, Choose, 0, &[30000, 30020, 30110, 30120, 30270, 30290, 30310, 30670, 30840], &[31010, 31050, 31110, 31120, 31240, 31250, 31280, 31670, 31810]),
    s(9270036, Dye, 5151028, Choose, 0, &[], &[]),
    s(9270037, Cut, 5150032, Random, 0, &[30110, 30180, 30260, 30290, 30300, 30350, 30470, 30720, 30840], &[31110, 31200, 31250, 31280, 31600, 31640, 31670, 31810, 34020]),
    s(9270037, Dye, 5151027, Random, 0, &[], &[]),
];

/// What `npc` offers, if it is a stylist.
pub fn services(npc: i32) -> impl Iterator<Item = &'static Service> {
    SERVICES.iter().filter(move |service| service.npc == npc)
}

impl Service {
    /// The looks on offer to `player`, each with the one-time lens it
    /// takes if any. The look worn is left out, except a skin.
    pub fn looks(&self, player: &Player) -> Vec<(i32, Option<i32>)> {
        let stats = &player.stats;
        let (hair, face) = (stats.hair, stats.face);
        let gender = i32::from(stats.gender.min(1));
        let list = if gender == 0 { self.male } else { self.female };
        // A face's style with colour `c`.
        let lens = |c: i32| face % 100 + 20_000 + 1_000 * gender + 100 * c;
        let styles: Vec<(i32, Option<i32>)> = match self.look {
            Skin => return (0..=4).map(|skin| (skin, None)).collect(),
            Cut => list.iter().map(|base| (base + hair % 10, None)).collect(),
            Dye => (0..8).map(|c| (hair / 10 * 10 + c, None)).collect(),
            Face => list
                .iter()
                .map(|base| (base + face / 100 % 10 * 100, None))
                .collect(),
            Lens => list.iter().map(|&c| (lens(c), None)).collect(),
            OneTimeLens => (0..8)
                .filter(|&c| player.inventory.count(ONE_TIME_LENS + c) > 0)
                .map(|c| (lens(c), Some(ONE_TIME_LENS + c)))
                .collect(),
        };
        styles
            .into_iter()
            // A one-time lens pays for its own colour only.
            .filter_map(|(style, lens)| match lens {
                Some(_) => crate::items::has_style(style).then_some((style, lens)),
                None => Some((style_or_base(style)?, lens)),
            })
            .filter(|&(style, _)| style != hair && style != face)
            .collect()
    }

    /// What the service takes from `player`: nothing for a member's cut,
    /// the coupon if they hold one. `None` if they can't pay.
    pub fn payment(&self, player: &Player) -> Option<Option<i32>> {
        let holds = |item: i32| player.inventory.count(item) > 0;
        if self.look == OneTimeLens || self.member.is_some_and(holds) {
            Some(None)
        } else {
            holds(self.coupon).then_some(Some(self.coupon))
        }
    }
}

/// `style` if it exists, or else the same style in colour 0 (as v83
/// servers offer a style that comes in one colour only).
fn style_or_base(style: i32) -> Option<i32> {
    if crate::items::has_style(style) {
        return Some(style);
    }
    let base = match style / 10_000 {
        2 => style / 1_000 * 1_000 + style % 100,
        _ => style / 10 * 10,
    };
    (base != style && crate::items::has_style(base)).then_some(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::{InventoryType, Item};

    fn player(gender: u8, face: i32, held: &[i32]) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.gender = gender;
        player.stats.face = face;
        for (slot, &item) in (1..).zip(held) {
            player
                .inventory
                .insert(InventoryType::Cash, slot, Item::stack(item, 1));
        }
        player
    }

    fn service(npc: i32, look: Look) -> &'static Service {
        services(npc).find(|service| service.look == look).unwrap()
    }

    #[test]
    fn a_member_cuts_for_free_and_keeps_the_card() {
        let natalie = service(1012103, Cut);
        assert_eq!(natalie.payment(&player(0, 20000, &[5420002])), Some(None));
        assert_eq!(
            natalie.payment(&player(0, 20000, &[5150001])),
            Some(Some(5150001))
        );
        assert_eq!(natalie.payment(&player(0, 20000, &[5420003])), None);
        let dye = service(1012103, Dye);
        assert_eq!(dye.payment(&player(0, 20000, &[5420002])), None);
    }

    #[test]
    fn surgery_keeps_the_colour_and_lenses_the_style() {
        let man = player(0, 20000, &[]);
        let faces = service(1052005, Face).looks(&man);
        assert_eq!(faces[0], (20005, None), "the face worn is left out");
        let lenses = service(9200100, Lens).looks(&man);
        let lenses: Vec<_> = lenses.into_iter().map(|(face, _)| face).collect();
        assert_eq!(lenses, [20100, 20200, 20400, 20600, 20700]);
    }

    #[test]
    fn a_one_time_lens_is_the_payment() {
        let one_time = service(9200100, OneTimeLens);
        assert!(one_time.looks(&player(1, 21002, &[])).is_empty());
        let woman = player(1, 21002, &[ONE_TIME_LENS + 3]);
        assert_eq!(one_time.looks(&woman), [(21302, Some(ONE_TIME_LENS + 3))]);
        assert_eq!(one_time.payment(&woman), Some(None));
    }

    #[test]
    fn every_skin_is_offered() {
        let skins = service(1012105, Skin).looks(&player(0, 20000, &[]));
        assert_eq!(skins, (0..=4).map(|skin| (skin, None)).collect::<Vec<_>>());
    }
}
