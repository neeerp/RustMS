mod accept;
pub mod error;
mod handler;
mod helpers;
pub mod session;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
