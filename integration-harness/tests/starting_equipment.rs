use integration_harness::login_two_players_to_world;
use integration_harness::packets::decode_spawn_player;
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use std::collections::BTreeMap;
use tokio::time::{timeout, Duration};

/// The harness creates male characters with these (see CharacterTemplate).
const TOP: i32 = 1040002;
const BOTTOM: i32 = 1060002;
const SHOES: i32 = 1072001;
const WEAPON: i32 = 1302000;

#[tokio::test]
async fn starting_equipment_is_equipped_and_visible_to_others() {
    let config = load_multi_harness_config_or_fail().await;
    let (mut first, second) = login_two_players_to_world(&config)
        .await
        .expect("two-player login-to-world flow failed");

    // The creator's own SET_FIELD carries the chosen items in their slots.
    let expected_equipped = BTreeMap::from([(5, TOP), (6, BOTTOM), (7, SHOES), (11, WEAPON)]);
    assert_eq!(second.equipped, expected_equipped);

    // Other players see those items on the avatar instead of a fixed outfit.
    let presence = timeout(
        Duration::from_secs(5),
        first.connection.read_packet("same-map presence"),
    )
    .await
    .expect("timed out waiting for presence")
    .expect("failed to read presence");
    let spawn = decode_spawn_player(&presence.packet).expect("failed to decode spawn");
    assert_eq!(spawn.character_id, second.character_id);
    assert_eq!(
        spawn.visible_equips,
        BTreeMap::from([(5, TOP), (6, BOTTOM), (7, SHOES), (11, WEAPON)])
    );
}
