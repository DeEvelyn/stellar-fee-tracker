use axum::{routing::get, Router};
use std::time::Duration;
use stellar_devkit::protocol::horizon::HorizonClient;
use tokio::net::TcpListener;

async fn start_mock_server(json_body: &str, delay_ms: u64) -> String {
    let body = json_body.to_string();
    let app = Router::new().route(
        "/fee_stats",
        get(move || {
            let b = body.clone();
            async move {
                if delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                axum::response::Json(serde_json::from_str::<serde_json::Value>(&b).unwrap())
            }
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

fn valid_fee_stats_json() -> &'static str {
    r#"{
        "last_ledger_base_fee": 100,
        "ledger_capacity_usage": 0.5,
        "min": 100,
        "mode": 200,
        "max": 300,
        "p10": 100,
        "p20": 150,
        "p30": 200
    }"#
}

#[tokio::test]
async fn fetch_fee_stats_from_mock_server() {
    let base_url = start_mock_server(valid_fee_stats_json(), 0).await;
    let client = HorizonClient::new(base_url);
    let stats = client.fetch_fee_stats().await.unwrap();
    assert_eq!(stats.last_ledger_base_fee, 100);
    assert!((stats.ledger_capacity_usage - 0.5).abs() < f64::EPSILON);
}

#[tokio::test]
async fn timeout_returns_error() {
    let base_url = start_mock_server(valid_fee_stats_json(), 5000).await;
    let client = HorizonClient::new(base_url).with_timeout_ms(100);
    let result = client.fetch_fee_stats().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn cache_hit_on_second_request() {
    let base_url = start_mock_server(valid_fee_stats_json(), 0).await;
    let client = HorizonClient::new(base_url);

    let _ = client.fetch_fee_stats().await.unwrap();
    let _ = client.fetch_fee_stats().await.unwrap();

    assert!(client.cache().cache_hits() >= 1);
}

#[tokio::test]
async fn pool_reuses_connections() {
    let base_url = start_mock_server(valid_fee_stats_json(), 0).await;
    let client = HorizonClient::new(base_url);

    for _ in 0..5 {
        let _ = client.fetch_fee_stats().await;
    }

    assert!(client.pool().hits() + client.pool().misses() > 0);
}
