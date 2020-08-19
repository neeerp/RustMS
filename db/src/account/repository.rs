use super::{Account, NewAccount};
use crate::establish_connection;
use crate::schema;
use diesel::expression_methods::*;
use diesel::{QueryDsl, QueryResult, RunQueryDsl, SaveChangesDsl};
use schema::accounts;
use schema::accounts::dsl::*;

pub fn get_account(user: &str) -> Option<Account> {
    let connection = establish_connection();

    let results = accounts
        .filter(user_name.eq(user))
        .first::<Account>(&connection);

    // TODO: Could add error handling...
    results.ok()
}

pub fn create_account<'a>(user: &'a str, pw: &'a str) -> Option<Account> {
    let connection = establish_connection();

    let new_account = NewAccount {
        user_name: user,
        password: pw,
    };

    diesel::insert_into(accounts::table)
        .values(&new_account)
        .get_result::<Account>(&connection)
        .ok()
}

pub fn update_account(acc: &Account) -> QueryResult<Account> {
    let connection = establish_connection();
    acc.save_changes(&connection)
}
