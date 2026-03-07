//! The purpose of this module is to put all relevant types in one place
//! for the schema to make use of.

use crate::schema::sql_types::{KeybindType, SessionState};
use diesel::query_builder::QueryId;
pub use diesel::sql_types::*;

impl QueryId for KeybindType {
    type QueryId = Self;

    const HAS_STATIC_QUERY_ID: bool = true;
}

impl QueryId for SessionState {
    type QueryId = Self;

    const HAS_STATIC_QUERY_ID: bool = true;
}
