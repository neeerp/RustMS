use std::time::{SystemTime, UNIX_EPOCH};

/// Offset between the Windows FILETIME epoch (1601) and the Unix epoch, in
/// 100ns ticks.
const FILETIME_UNIX_OFFSET: i64 = 116_444_736_000_000_000;

/// 1900-01-01: the "unset" timestamp (`getTime(-2)` in HeavenMS), used for
/// the inventory header and the trailer of every equipment entry.
pub const ZERO_TIME: i64 = 94_354_848_000_000_000;
/// 2078-12-31 (`getTime(-3)` in HeavenMS): the date of a pet that never
/// dies, as v83 takes one dated 2079 for dead.
pub const PERMANENT_TIME: i64 = 150_841_440_000_000_000;
/// 2079-01-01 (`getTime(-1)` in HeavenMS): what an item or skill with no
/// expiration (stored as -1 in the reference) is sent as.
pub const DEFAULT_TIME: i64 = 150_842_304_000_000_000;

/// Convert a system time into a Windows FILETIME tick count.
pub fn filetime(time: SystemTime) -> i64 {
    let millis = time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    millis * 10_000 + FILETIME_UNIX_OFFSET
}

/// FILETIME for an optional expiration: `None` (never expires) is the
/// far-future default. Sending `ZERO_TIME` here would mean "expired in 1900".
pub fn expiration(time: Option<SystemTime>) -> i64 {
    time.map_or(DEFAULT_TIME, filetime)
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
        assert_eq!(expiration(None), DEFAULT_TIME);
    }
}
