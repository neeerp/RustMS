use integration_harness::login_to_redirect;
use integration_harness::preconditions::load_harness_config_or_fail;

#[tokio::test]
async fn login_redirect_selected_channel() {
    let config = load_harness_config_or_fail().await.with_channel(0, 2);
    let expected = config.channel_addr(2);

    let redirect = login_to_redirect(&config)
        .await
        .expect("channel-specific redirect flow failed");

    // Every channel has its own port; the redirect must point at channel 3's.
    assert_eq!(redirect.redirect_port, expected.port());
    assert_ne!(redirect.redirect_port, config.channel_addr(0).port());
}
