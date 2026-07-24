use std::future::Future;

/// Execute `primary`; if it returns `Err`, execute `fallback` and return its result.
///
/// Both closures are invoked at most once.
pub async fn with_fallback<T, E, F1, F2, Fut1, Fut2>(primary: F1, fallback: F2) -> Result<T, E>
where
    F1: FnOnce() -> Fut1,
    Fut1: Future<Output = Result<T, E>>,
    F2: FnOnce() -> Fut2,
    Fut2: Future<Output = Result<T, E>>,
{
    match primary().await {
        Ok(val) => Ok(val),
        Err(_) => fallback().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn primary_succeeds() {
        let result = with_fallback(
            || async { Ok::<_, &str>("primary") },
            || async { Ok::<_, &str>("fallback") },
        )
        .await;
        assert_eq!(result, Ok("primary"));
    }

    #[tokio::test]
    async fn primary_fails_fallback_succeeds() {
        let result = with_fallback(
            || async { Err::<&str, _>("boom") },
            || async { Ok::<_, &str>("fallback") },
        )
        .await;
        assert_eq!(result, Ok("fallback"));
    }

    #[tokio::test]
    async fn both_fail() {
        let result = with_fallback(
            || async { Err::<&str, _>("err1") },
            || async { Err::<&str, _>("err2") },
        )
        .await;
        assert_eq!(result, Err("err2"));
    }
}
