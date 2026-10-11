use super::{Account, NewAccount};
use crate::schema;
use diesel::expression_methods::*;
use diesel::{QueryDsl, RunQueryDsl, SaveChangesDsl};
use schema::accounts;
use schema::accounts::dsl::*;

pub fn get_account(user: &str) -> crate::Result<Account> {
    let mut connection = crate::connection()?;

    Ok(accounts
        .filter(user_name.eq(user))
        .first::<Account>(&mut connection)?)
}

pub fn get_account_by_id(a_id: i32) -> crate::Result<Account> {
    let mut connection = crate::connection()?;

    Ok(accounts
        .filter(id.eq(a_id))
        .first::<Account>(&mut connection)?)
}

pub fn create_account<'a>(user: &'a str, pw: &'a str) -> crate::Result<Account> {
    let mut connection = crate::connection()?;

    let new_account = NewAccount {
        user_name: user,
        password: pw,
    };

    Ok(diesel::insert_into(accounts::table)
        .values(&new_account)
        .get_result::<Account>(&mut connection)?)
}

pub fn update_account(acc: &Account) -> crate::Result<Account> {
    let mut connection = crate::connection()?;
    Ok(acc.save_changes(&mut connection)?)
}
