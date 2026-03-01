use super::{Account, NewAccount};
use crate::establish_connection;
use crate::schema;
use diesel::expression_methods::*;
use diesel::{QueryDsl, QueryResult, RunQueryDsl, SaveChangesDsl};
use schema::accounts;
use schema::accounts::dsl::*;

pub fn get_account(user: &str) -> QueryResult<Account> {
    let mut connection = establish_connection();

    accounts
        .filter(user_name.eq(user))
        .first::<Account>(&mut connection)
}

pub fn get_account_by_id(a_id: i32) -> QueryResult<Account> {
    let mut connection = establish_connection();

    accounts.filter(id.eq(a_id)).first::<Account>(&mut connection)
}

pub fn create_account<'a>(user: &'a str, pw: &'a str) -> QueryResult<Account> {
    let mut connection = establish_connection();

    let new_account = NewAccount {
        user_name: user,
        password: pw,
    };

    diesel::insert_into(accounts::table)
        .values(&new_account)
        .get_result::<Account>(&mut connection)
}

pub fn update_account(acc: &Account) -> QueryResult<Account> {
    let mut connection = establish_connection();
    acc.save_changes(&mut connection)
}
