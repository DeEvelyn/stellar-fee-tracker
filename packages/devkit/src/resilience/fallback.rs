use std::future::Future;

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
