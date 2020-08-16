use crate::*;
use models::{Account, NewAccount};
// use std::time::SystemTime;

pub fn get_account(user: &str) -> Option<Account> {
    use schema::accounts::dsl::*;
    let connection = establish_connection();

    let results = accounts
        .filter(user_name.eq(user))
        .first::<Account>(&connection);

    // TODO: Could add error handling...
    results.ok()
}

pub fn create_account<'a>(user: &'a str, pw: &'a str) -> Option<Account> {
    use schema::accounts;
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

pub fn login_account(acc: &Account) -> QueryResult<usize> {
    use schema::accounts;
    let connection = establish_connection();

    diesel::update(accounts::table)
        .set(acc)
        .execute(&connection)
}
