use super::*;

#[tokio::test]
async fn test_rate_limiting() {
    let cache = setup_test_cache();
    let rate_limiter = RateLimiter::new(cache, 2); // 2 requests per minute for testing
    let api_key = generate_test_api_key();

    // First request should succeed
    rate_limiter.check_rate_limit(&api_key)
        .await
        .expect("First request should not be rate limited");

    // Second request should succeed
    rate_limiter.check_rate_limit(&api_key)
        .await
        .expect("Second request should not be rate limited");

    // Third request should fail
    let result = rate_limiter.check_rate_limit(&api_key).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), crate::error::ApiError::RateLimitExceeded));

    // Wait for rate limit to reset
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

    // Should succeed again after reset
    rate_limiter.check_rate_limit(&api_key)
        .await
        .expect("Request should succeed after rate limit reset");
} 