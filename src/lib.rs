extern crate serde_json;
pub mod handler;
pub mod service;
pub use json_action;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
