// Performance testing module for may-redis.
//
// Validates Redis load patterns matching Sesame-IDAM operations at scale
// (up to 2000 users with realistic JWT-like key/value distributions).
//
// Modules:
// - jwt: JWT claim generation and refresh token data structures
// - redis_ops: High-throughput Redis operation helpers
// - metrics: Throughput and latency measurement utilities
// - scenarios: Reusable test scenarios (population, login burst, etc.)

pub mod jwt;
pub mod metrics;
pub mod redis_ops;
pub mod scenarios;

// Re-export commonly used types
pub use jwt::{RefreshTokenData, generate_user_jwt};
pub use redis_ops::{
    populate_users,
    cleanup_users,
    batch_denylist_check,
};
pub use metrics::{ThroughputResult, LatencyProfile};
