/// Maximum number of concurrent HTTP requests when fetching token lists
pub const TOKEN_FETCH_CONCURRENCY: usize = 10;

/// Capacity of the broadcast channel for balance events per subscription
pub const BROADCAST_CHANNEL_CAPACITY: usize = 256;

/// Default interval (seconds) between full balance snapshot updates
pub const DEFAULT_SNAPSHOT_INTERVAL_SECS: u64 = 60;
