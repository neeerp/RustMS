mod accept;
mod client;
pub mod error;
mod helpers;
mod packet;
pub mod session;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
