use integration_harness::login_to_redirect;
use integration_harness::preconditions::load_harness_config_or_fail;

#[tokio::test]
async fn login_redirect_selected_channel() {
    let config = load_harness_config_or_fail()
        .await
        .with_channel(0, 2)
        .with_expected_redirect_addr(
            "127.0.0.1:19485"
                .parse()
                .expect("valid redirect test address"),
        );

    let redirect = login_to_redirect(&config)
        .await
        .expect("channel-specific redirect flow failed");

    assert_eq!(redirect.redirect_port, 19485);
}
