use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::{check_char_name, create_another_character};

/// The name check's code for a name the client says "You cannot use".
const NOT_ALLOWED: u8 = 2;
/// The creation result the client answers with the same notice.
const NAME_REFUSED: u8 = 0x1E;

#[tokio::test]
async fn forbidden_and_confusable_names_are_refused() {
    let config = load_harness_config_or_fail().await;

    for name in ["TheAdmin", "xxMAPLExx", "IlIl1234"] {
        let result = check_char_name(&config, name)
            .await
            .expect("check-char-name flow failed");
        assert_eq!(result, NOT_ALLOWED, "`{name}` should not be allowed");
    }
}

#[tokio::test]
async fn creating_a_character_with_a_refused_name_answers_instead_of_disconnecting() {
    let config = load_harness_config_or_fail().await;

    let result = create_another_character(&config, "TheAdmin")
        .await
        .expect("create flow failed");

    assert_eq!(result, NAME_REFUSED);
}
