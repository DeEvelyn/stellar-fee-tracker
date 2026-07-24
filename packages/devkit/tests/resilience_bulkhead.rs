use std::time::Duration;

use stellar_devkit::resilience::bulkhead::Bulkhead;

#[tokio::test]
async fn max_concurrent_is_respected() {
    let bulkhead = Bulkhead::new(2);
    let p1 = bulkhead.acquire().await;
    let p2 = bulkhead.acquire().await;
    assert_eq!(bulkhead.max_concurrent(), 2);
    drop(p1);
    drop(p2);
}

#[tokio::test]
async fn fail_fast_returns_none_when_full() {
    let bulkhead = Bulkhead::new(1);
    let _p1 = bulkhead.acquire().await;
    assert!(bulkhead.try_acquire().is_none());
}

#[tokio::test]
async fn fail_fast_succeeds_after_release() {
    let bulkhead = Bulkhead::new(1);
    let p1 = bulkhead.acquire().await;
    assert!(bulkhead.try_acquire().is_none());
    drop(p1);
    assert!(bulkhead.try_acquire().is_some());
}

#[tokio::test]
async fn queue_mode_waits_for_permit() {
    let bulkhead = Bulkhead::new(1);
    let p1 = bulkhead.acquire().await;

    let bh = bulkhead.clone();
    let handle = tokio::spawn(async move {
        let _p = bh.acquire().await;
        "acquired"
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!handle.is_finished());

    drop(p1);
    let result = handle.await.unwrap();
    assert_eq!(result, "acquired");
}

#[tokio::test]
async fn multiple_permits_up_to_max() {
    let bulkhead = Bulkhead::new(3);
    let p1 = bulkhead.acquire().await;
    let p2 = bulkhead.acquire().await;
    let p3 = bulkhead.acquire().await;
    assert!(bulkhead.try_acquire().is_none());
    drop(p1);
    assert!(bulkhead.try_acquire().is_some());
    drop(p2);
    drop(p3);
}
