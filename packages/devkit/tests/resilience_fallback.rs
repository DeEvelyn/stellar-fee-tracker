use stellar_devkit::resilience::fallback::with_fallback;

#[tokio::test]
async fn primary_success_skips_fallback() {
    let mut fallback_called = false;
    let result = with_fallback(
        || async { Ok::<_, &str>("ok") },
        || async {
            fallback_called = true;
            Err("should not be called")
        },
    )
    .await;
    assert_eq!(result, Ok("ok"));
    assert!(!fallback_called);
}

#[tokio::test]
async fn primary_failure_triggers_fallback() {
    let result = with_fallback(
        || async { Err::<&str, _>("primary failed") },
        || async { Ok::<_, &str>("fallback ok") },
    )
    .await;
    assert_eq!(result, Ok("fallback ok"));
}

#[tokio::test]
async fn fallback_failure_propagates_error() {
    let result: Result<&str, &str> = with_fallback(
        || async { Err("primary err") },
        || async { Err("fallback err") },
    )
    .await;
    assert_eq!(result, Err("fallback err"));
}
