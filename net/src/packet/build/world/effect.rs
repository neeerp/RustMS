use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// FIELD_EFFECT mode 3: play `Map.nx/Effect.img/<path>` on the screen, e.g.
/// a map name banner (`maplemap/enter/10000`).
pub fn build_field_effect(path: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::FieldEffect as i16)?;
    p.write_byte(3)?;
    p.write_str_with_length(path)?;
    Ok(p)
}

/// FIELD_EFFECT mode 2: show the map object called `name` that the map
/// keeps hidden until told, e.g. a party quest stage's `gate`.
pub fn build_field_object(name: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::FieldEffect as i16)?;
    p.write_byte(2)?;
    p.write_str_with_length(name)?;
    Ok(p)
}

/// FIELD_OBSTACLE_ONOFF: show the map object called `name`, such as an
/// Orbis jail's spear `trap1`, in `state` (FUN_00642aca).
pub fn build_obstacle_state(name: &str, state: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::FieldObstacleOnOff as i16)?;
    p.write_str_with_length(name)?;
    p.write_int(state)?;
    Ok(p)
}

/// FIELD_OBSTACLE_ONOFF_LIST: the named map objects that are in other
/// states than their first, for a player who enters the field.
pub fn build_obstacle_states(states: &[(String, i32)]) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::FieldObstacleOnOffList as i16)?;
    p.write_int(states.len() as i32)?;
    for (name, state) in states {
        p.write_str_with_length(name)?;
        p.write_int(*state)?;
    }
    Ok(p)
}

/// FIELD_EFFECT mode 4: play `Sound.nx/Field.img/<path>`, e.g. a party
/// quest stage's clear jingle (`Party1/Clear`).
pub fn build_field_sound(path: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::FieldEffect as i16)?;
    p.write_byte(4)?;
    p.write_str_with_length(path)?;
    Ok(p)
}

/// FIELD_EFFECT mode 6: play `Sound.nx/<path>` as the field's music
/// instead of its own, e.g. Zakum's (`Bgm06/FinalFight`).
pub fn build_field_music(path: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::FieldEffect as i16)?;
    p.write_byte(6)?;
    p.write_str_with_length(path)?;
    Ok(p)
}

/// FIELD_EFFECT mode 5 (v83 0x5336ca): the boss HP bar across the top of
/// the screen for the mob template `mob_id`, in the colours of its
/// `hpTagColor` and `hpTagBgcolor`. An `hp` below 1 takes the bar down.
pub fn build_boss_hp_bar(
    mob_id: i32,
    hp: i32,
    max_hp: i32,
    colors: (u8, u8),
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::FieldEffect as i16)?;
    p.write_byte(5)?;
    p.write_int(mob_id)?;
    p.write_int(hp)?;
    p.write_int(max_hp)?;
    p.write_byte(colors.0)?;
    p.write_byte(colors.1)?;
    Ok(p)
}

/// BLOW_WEATHER (v83 FUN_00535179): the weather of the 512xxxx item
/// `item_id` with its message bar, or `None` to stop it. The client never
/// stops one by itself.
pub fn build_blow_weather(weather: Option<(i32, &str)>) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::BlowWeather as i16)?;
    p.write_byte(0)?;
    match weather {
        Some((item_id, message)) => {
            p.write_int(item_id)?;
            p.write_str_with_length(message)?;
        }
        None => p.write_int(0)?,
    }
    Ok(p)
}

/// PLAY_JUKEBOX (v83 FUN_00535224): `name` plays the song of the 510xxxx
/// item `item_id`, which loops until each player leaves the map.
pub fn build_play_jukebox(item_id: i32, name: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::PlayJukebox as i16)?;
    p.write_int(item_id)?;
    p.write_str_with_length(name)?;
    Ok(p)
}

/// SHOW_ITEM_GAIN_INCHAT mode 0x17: show a UI image (`UI/tutorial.img/25`)
/// anchored to the player, as the tutorial hints do.
pub fn build_avatar_effect(path: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowItemGainInChat as i16)?;
    p.write_byte(0x17)?;
    p.write_str_with_length(path)?;
    p.write_int(1)?;
    Ok(p)
}

/// SHOW_ITEM_GAIN_INCHAT mode 0x12: a scene's part from `Effect.nx`
/// (`Effect/Direction2.img/open/back0`), which the client plays on its own
/// clock.
pub fn build_scene_part(path: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowItemGainInChat as i16)?;
    p.write_byte(0x12)?;
    p.write_str_with_length(path)?;
    Ok(p)
}

/// LOCK_UI: hides and locks the UI while a scene plays, or gives it back.
pub fn build_lock_ui(locked: bool) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::LockUi as i16)?;
    p.write_byte(u8::from(locked))?;
    Ok(p)
}

/// SPAWN_GUIDE: brings the tutorial guide out at the player's feet, or
/// sends it away.
pub fn build_spawn_guide(shown: bool) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SpawnGuide as i16)?;
    p.write_byte(u8::from(shown))?;
    Ok(p)
}

/// TALK_GUIDE mode 0: a balloon over the guide, `width` px wide, for
/// `duration` ms.
pub fn build_guide_says(text: &str, width: i32, duration: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::TalkGuide as i16)?;
    p.write_byte(0)?;
    p.write_str_with_length(text)?;
    p.write_int(width)?;
    p.write_int(duration)?;
    Ok(p)
}

/// TALK_GUIDE mode 1: `UI/tutorial.img/<hint>` over the guide for
/// `duration` ms.
pub fn build_guide_hint(hint: i32, duration: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::TalkGuide as i16)?;
    p.write_byte(1)?;
    p.write_int(hint)?;
    p.write_int(duration)?;
    Ok(p)
}

/// The charm that kept a dead player's EXP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharmUsed {
    /// A Safety Charm, with the charms left and the days the next one lasts.
    SafetyCharm { left: u8, days: u8 },
    /// Another charm item, which the line names.
    Item(i32),
}

/// SHOW_ITEM_GAIN_INCHAT mode 6 (v83 FUN_009377d9): the grey chat line
/// saying a charm kept the player's EXP. The client reads both counts
/// before the item, which only the second form shows.
pub fn build_charm_used(charm: CharmUsed) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowItemGainInChat as i16)?;
    p.write_byte(6)?;
    match charm {
        CharmUsed::SafetyCharm { left, days } => {
            p.write_byte(1)?;
            p.write_byte(left)?;
            p.write_byte(days)?;
        }
        CharmUsed::Item(item_id) => {
            p.write_byte(0)?;
            p.write_byte(0)?;
            p.write_byte(0)?;
            p.write_int(item_id)?;
        }
    }
    Ok(p)
}

/// SHOW_ITEM_GAIN_INCHAT mode 0x15: the grey chat line saying a Wheel of
/// Destiny revived the player where they died, with the wheels `left`.
pub fn build_wheel_used(left: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowItemGainInChat as i16)?;
    p.write_byte(0x15)?;
    p.write_byte(left)?;
    Ok(p)
}

/// An effect played on a character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharEffect {
    LevelUp = 0,
    /// The portal sound, for a scripted warp (FUN_009377d9 mode 7).
    PortalSound = 7,
    JobChange = 8,
    QuestClear = 9,
    MonsterCard = 13,
}

/// SHOW_ITEM_GAIN_INCHAT: `effect` on this player.
pub fn build_own_effect(effect: CharEffect) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowItemGainInChat as i16)?;
    p.write_byte(effect as u8)?;
    Ok(p)
}

/// SHOW_FOREIGN_EFFECT: `effect` on another player.
pub fn build_foreign_effect(character_id: i32, effect: CharEffect) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowForeignEffect as i16)?;
    p.write_int(character_id)?;
    p.write_byte(effect as u8)?;
    Ok(p)
}

/// A cash mist's puff (FUN_009377d9 mode 20) on its user: SHOW_ITEM_GAIN_INCHAT
/// for them, SHOW_FOREIGN_EFFECT naming `character_id` for the others.
pub fn build_mist_item_effect(
    character_id: Option<i32>,
    item_id: i32,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    match character_id {
        Some(id) => {
            p.write_short(SendOpcode::ShowForeignEffect as i16)?;
            p.write_int(id)?;
        }
        None => p.write_short(SendOpcode::ShowItemGainInChat as i16)?,
    }
    p.write_byte(20)?;
    p.write_int(item_id)?;
    Ok(p)
}

/// A reward box's prize (FUN_009377d9 mode 14, report c038): the prize's use
/// sound, and the entry's `Effect` drawn where the opener stands, as the data
/// writes it. SHOW_ITEM_GAIN_INCHAT for the opener, SHOW_FOREIGN_EFFECT
/// naming `character_id` for the others.
pub fn build_reward_item_effect(
    character_id: Option<i32>,
    item_id: i32,
    effect: Option<&str>,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    match character_id {
        Some(id) => {
            p.write_short(SendOpcode::ShowForeignEffect as i16)?;
            p.write_int(id)?;
        }
        None => p.write_short(SendOpcode::ShowItemGainInChat as i16)?,
    }
    p.write_byte(14)?;
    p.write_int(item_id)?;
    match effect {
        Some(path) => {
            p.write_byte(1)?;
            p.write_str_with_length(path)?;
        }
        None => {
            p.write_byte(0)?;
        }
    }
    Ok(p)
}

/// ITEM_EMOTION (0xE2, FUN_00938b3e): the user of the cash mist `item_id`
/// pulls one of its `emotion` faces, which their client picks and shows.
pub fn build_item_emotion(item_id: i32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ItemEmotion as i16)?;
    p.write_int(item_id)?;
    Ok(p)
}

/// SHOW_SCROLL_EFFECT (0xA7, v83 FUN_0093354d): the glow, sound and (for
/// the scroller) chat line of a scroll on `character_id`. `success` is 1 or
/// 0, or -1 to refuse a Legendary Spirit request; `cursed` means the item was
/// destroyed; `legendary` answers a Legendary Spirit request, which only the
/// skill's open dialog can show; `white` means a White Scroll was used.
pub fn build_scroll_effect(
    character_id: i32,
    success: i8,
    cursed: bool,
    legendary: bool,
    white: bool,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ShowScrollEffect as i16)?;
    p.write_int(character_id)?;
    p.write_byte(success as u8)?;
    p.write_byte(u8::from(cursed))?;
    p.write_byte(u8::from(legendary))?;
    p.write_byte(u8::from(white))?;
    Ok(p)
}

/// PLAYER_HINT: an instruction balloon above the player.
pub fn build_hint(text: &str, width: i16, height: i16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::PlayerHint as i16)?;
    p.write_str_with_length(text)?;
    p.write_short(width)?;
    p.write_short(height)?;
    p.write_byte(1)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn the_guide_comes_out_says_and_hints() {
        let spawn = build_spawn_guide(true).unwrap();
        let mut expect = Expect::new(&spawn.bytes);
        expect.short(SendOpcode::SpawnGuide as i16).byte(1);
        expect.end();
        let says = build_guide_says("Hi", 200, 4000).unwrap();
        let mut expect = Expect::new(&says.bytes);
        expect.short(SendOpcode::TalkGuide as i16).byte(0);
        expect.string("Hi").int(200).int(4000);
        expect.end();
        let hint = build_guide_hint(11, 7000).unwrap();
        let mut expect = Expect::new(&hint.bytes);
        expect.short(SendOpcode::TalkGuide as i16).byte(1);
        expect.int(11).int(7000);
        expect.end();
    }

    #[test]
    fn field_music_names_its_track() {
        let packet = build_field_music("Bgm06/FinalFight").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::FieldEffect as i16).byte(6);
        expect.string("Bgm06/FinalFight");
        expect.end();
    }

    #[test]
    fn a_cash_mist_puffs_on_its_user() {
        let own = build_mist_item_effect(None, 5281000).unwrap();
        let mut expect = Expect::new(&own.bytes);
        expect
            .short(SendOpcode::ShowItemGainInChat as i16)
            .byte(20)
            .int(5281000);
        expect.end();
        let foreign = build_mist_item_effect(Some(7), 5281001).unwrap();
        let mut expect = Expect::new(&foreign.bytes);
        expect
            .short(SendOpcode::ShowForeignEffect as i16)
            .int(7)
            .byte(20)
            .int(5281001);
        expect.end();
        let face = build_item_emotion(5281000).unwrap();
        let mut expect = Expect::new(&face.bytes);
        expect.short(0xE2).int(5281000);
        expect.end();
    }

    #[test]
    fn obstacles_are_set_one_or_all_by_name() {
        let packet = build_obstacle_state("trap3", 1).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::FieldObstacleOnOff as i16);
        expect.string("trap3").int(1);
        expect.end();
        let states = [("trap1".to_string(), 1), ("trap7".to_string(), 1)];
        let packet = build_obstacle_states(&states).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::FieldObstacleOnOffList as i16)
            .int(2);
        expect.string("trap1").int(1).string("trap7").int(1);
        expect.end();
    }

    #[test]
    fn field_effect_is_mode_three_with_a_path() {
        let packet = build_field_effect("maplemap/enter/10000").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::FieldEffect as i16).byte(3);
        expect.string("maplemap/enter/10000");
        expect.end();
    }

    #[test]
    fn field_object_is_mode_two_with_a_name() {
        let packet = build_field_object("gate").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::FieldEffect as i16).byte(2);
        expect.string("gate");
        expect.end();
    }

    #[test]
    fn field_sound_is_mode_four_with_a_path() {
        let packet = build_field_sound("Party1/Clear").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::FieldEffect as i16).byte(4);
        expect.string("Party1/Clear");
        expect.end();
    }

    #[test]
    fn boss_hp_bar_is_mode_five() {
        let packet = build_boss_hp_bar(8800000, 33_000_000, 66_000_000, (1, 3)).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::FieldEffect as i16).byte(5);
        expect.int(8800000).int(33_000_000).int(66_000_000);
        expect.byte(1).byte(3);
        expect.end();
    }

    #[test]
    fn a_jukebox_names_its_player() {
        let packet = build_play_jukebox(5100000, "Tester").unwrap();
        Expect::new(&packet.bytes)
            .short(0x8F)
            .int(5100000)
            .string("Tester")
            .end();
    }

    #[test]
    fn a_weather_carries_its_message_and_stops_with_item_zero() {
        let packet = build_blow_weather(Some((5120000, "Tester's snow : hi"))).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x8E).byte(0).int(5120000);
        expect.string("Tester's snow : hi");
        expect.end();
        let packet = build_blow_weather(None).unwrap();
        Expect::new(&packet.bytes).short(0x8E).byte(0).int(0).end();
    }

    #[test]
    fn avatar_effect_carries_the_ui_path() {
        let packet = build_avatar_effect("UI/tutorial.img/25").unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::ShowItemGainInChat as i16)
            .byte(0x17);
        expect.string("UI/tutorial.img/25").int(1);
        expect.end();
    }

    #[test]
    fn a_safety_charm_line_has_the_charms_and_days_left() {
        let packet = build_charm_used(CharmUsed::SafetyCharm { left: 4, days: 89 }).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowItemGainInChat as i16).byte(6);
        expect.byte(1).byte(4).byte(89);
        expect.end();
    }

    #[test]
    fn another_charm_is_named_after_both_counts() {
        let packet = build_charm_used(CharmUsed::Item(4140903)).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowItemGainInChat as i16).byte(6);
        expect.byte(0).byte(0).byte(0).int(4140903);
        expect.end();
    }

    #[test]
    fn a_wheel_line_has_the_wheels_left() {
        let packet = build_wheel_used(2).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::ShowItemGainInChat as i16)
            .byte(0x15)
            .byte(2);
        expect.end();
    }

    #[test]
    fn own_effect_is_just_the_effect() {
        let packet = build_own_effect(CharEffect::QuestClear).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowItemGainInChat as i16).byte(9);
        expect.end();
    }

    #[test]
    fn foreign_effect_names_the_player() {
        let packet = build_foreign_effect(7, CharEffect::LevelUp).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::ShowForeignEffect as i16)
            .int(7)
            .byte(0);
        expect.end();
    }

    #[test]
    fn scroll_effect_has_the_outcome_bytes() {
        let packet = build_scroll_effect(7, 1, false, false, true).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(SendOpcode::ShowScrollEffect as i16).int(7);
        expect.byte(1).byte(0).byte(0).byte(1);
        expect.end();
        let refusal = build_scroll_effect(7, -1, false, true, false).unwrap();
        assert_eq!(&refusal.bytes[6..], &[0xFF, 0, 1, 0]);
    }

    #[test]
    fn hint_has_text_size_and_trailer() {
        let packet = build_hint("Once you leave", 150, 5).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect
            .short(SendOpcode::PlayerHint as i16)
            .string("Once you leave");
        expect.short(150).short(5).byte(1);
        expect.end();
    }

    #[test]
    fn a_prize_plays_its_box_effect() {
        let own =
            build_reward_item_effect(None, 2049100, Some("Effect/BasicEff/Event1/Best")).unwrap();
        let mut expect = Expect::new(&own.bytes);
        expect
            .short(SendOpcode::ShowItemGainInChat as i16)
            .byte(14)
            .int(2049100)
            .byte(1)
            .string("Effect/BasicEff/Event1/Best");
        expect.end();
        let foreign = build_reward_item_effect(Some(7), 4001201, None).unwrap();
        let mut expect = Expect::new(&foreign.bytes);
        expect
            .short(SendOpcode::ShowForeignEffect as i16)
            .int(7)
            .byte(14)
            .int(4001201)
            .byte(0);
        expect.end();
    }
}
