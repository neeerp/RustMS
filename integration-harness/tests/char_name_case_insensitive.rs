use integration_harness::check_char_name_available;
use integration_harness::preconditions::load_harness_config_or_fail;

fn swap_case(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_uppercase() {
                c.to_ascii_lowercase()
            } else {
                c.to_ascii_uppercase()
            }
        })
        .collect()
}

#[tokio::test]
async fn existing_name_is_unavailable_regardless_of_case() {
    let config = load_harness_config_or_fail().await;
    let variant = swap_case(&config.character_name);
    assert_ne!(
        variant, config.character_name,
        "fixture character name needs at least one letter to vary its case"
    );

    let available = check_char_name_available(&config, &variant)
        .await
        .expect("check-char-name flow failed");

    assert!(
        !available,
        "`{}` should be taken because `{}` already exists",
        variant, config.character_name
    );
}
