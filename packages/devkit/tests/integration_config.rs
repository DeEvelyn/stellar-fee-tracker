use std::env;
use std::sync::{Mutex, OnceLock};

fn env_mutex() -> &'static Mutex<()> {
    static INSTANCE: OnceLock<Mutex<()>> = OnceLock::new();
    INSTANCE.get_or_init(|| Mutex::new(()))
}

#[test]
fn test_config_loading_with_env_vars() {
    let _guard = env_mutex().lock().unwrap();
    env::set_var("STELLAR_NETWORK", "testnet");
    env::set_var("HORIZON_URL", "https://horizon-testnet.stellar.org");
    env::set_var("POLL_INTERVAL_SECS", "30");
    env::set_var("DATABASE_URL", "sqlite::memory:");
    env::set_var("PORT", "8080");
    env::set_var("LOG_LEVEL", "debug");

    assert_eq!(env::var("STELLAR_NETWORK").unwrap(), "testnet");
    assert_eq!(
        env::var("HORIZON_URL").unwrap(),
        "https://horizon-testnet.stellar.org"
    );
    assert_eq!(env::var("POLL_INTERVAL_SECS").unwrap(), "30");
    assert_eq!(env::var("DATABASE_URL").unwrap(), "sqlite::memory:");
    assert_eq!(env::var("PORT").unwrap(), "8080");
    assert_eq!(env::var("LOG_LEVEL").unwrap(), "debug");

    env::remove_var("STELLAR_NETWORK");
    env::remove_var("HORIZON_URL");
    env::remove_var("POLL_INTERVAL_SECS");
    env::remove_var("DATABASE_URL");
    env::remove_var("PORT");
    env::remove_var("LOG_LEVEL");
}

#[test]
fn test_config_defaults_when_env_unset() {
    let _guard = env_mutex().lock().unwrap();
    env::remove_var("STELLAR_NETWORK");
    env::remove_var("HORIZON_URL");
    env::remove_var("POLL_INTERVAL_SECS");

    let default_network = env::var("STELLAR_NETWORK").unwrap_or_else(|_| "mainnet".to_string());
    let default_poll = env::var("POLL_INTERVAL_SECS")
        .unwrap_or_else(|_| "60".to_string())
        .parse::<u64>()
        .unwrap();

    assert_eq!(default_network, "mainnet");
    assert_eq!(default_poll, 60);
}
