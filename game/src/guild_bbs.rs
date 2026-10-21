//! The guild bulletin board's limits.

/// The number of the guild's notice thread; other threads count from 1.
pub const NOTICE: i32 = 0;
/// Threads in one page of the list, as the client counts pages.
pub const PAGE_SIZE: i64 = 10;
/// The longest title, body and reply, in characters.
pub const TITLE_MAX_LEN: usize = 25;
pub const BODY_MAX_LEN: usize = 600;
pub const REPLY_MAX_LEN: usize = 25;
/// The first cash emoticon item; icon `100 + n` needs item `CASH_ICON_ITEM + n`.
pub const CASH_ICON_ITEM: i32 = 5_290_000;

/// The item a player must own to post with `icon`: `Ok(None)` for the three
/// free icons, `Err(())` for an icon the client doesn't have.
pub fn icon_item(icon: i32) -> Result<Option<i32>, ()> {
    match icon {
        0..=2 => Ok(None),
        100..=106 => Ok(Some(CASH_ICON_ITEM + icon - 100)),
        _ => Err(()),
    }
}

/// `text` cut to `max` characters, as the reference server stores it.
pub fn cut(text: &str, max: usize) -> &str {
    text.char_indices()
        .nth(max)
        .map_or(text, |(at, _)| &text[..at])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cash_icons_need_their_emoticon() {
        assert_eq!(icon_item(0), Ok(None));
        assert_eq!(icon_item(2), Ok(None));
        assert_eq!(icon_item(3), Err(()));
        assert_eq!(icon_item(100), Ok(Some(5_290_000)));
        assert_eq!(icon_item(106), Ok(Some(5_290_006)));
        assert_eq!(icon_item(107), Err(()));
        assert_eq!(icon_item(-1), Err(()));
    }

    #[test]
    fn cut_keeps_whole_characters() {
        assert_eq!(cut("hello", 25), "hello");
        assert_eq!(cut("hello", 3), "hel");
        assert_eq!(cut("héllo", 2), "hé");
    }
}
