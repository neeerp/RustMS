use std::time::{SystemTime, UNIX_EPOCH};

/// Offset between the Windows FILETIME epoch (1601) and the Unix epoch, in
/// 100ns ticks.
const FILETIME_UNIX_OFFSET: i64 = 116_444_736_000_000_000;

/// "No expiration" sentinel the client uses for items and timestamps.
pub const ZERO_TIME: i64 = 94_354_848_000_000_000;
/// Far-future timestamp used for permanent items.
pub const PERMANENT_TIME: i64 = 150_841_440_000_000_000;
/// Default high timestamp used by several login packets.
pub const DEFAULT_TIME: i64 = 150_842_304_000_000_000;

/// Convert a system time into a Windows FILETIME tick count.
pub fn filetime(time: SystemTime) -> i64 {
    let millis = time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    millis * 10_000 + FILETIME_UNIX_OFFSET
}

/// FILETIME for an optional expiration: `None` means it never expires.
pub fn expiration(time: Option<SystemTime>) -> i64 {
    time.map_or(ZERO_TIME, filetime)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn unix_epoch_maps_to_the_filetime_offset() {
        assert_eq!(filetime(UNIX_EPOCH), FILETIME_UNIX_OFFSET);
        assert_eq!(
            filetime(UNIX_EPOCH + Duration::from_millis(1)),
            FILETIME_UNIX_OFFSET + 10_000
        );
        assert_eq!(expiration(None), ZERO_TIME);
    }
}
