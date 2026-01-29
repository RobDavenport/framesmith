#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod bytes;
pub mod error;
pub mod view;

pub use error::Error;
pub use view::PackView;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_fails_too_short() {
        let result = PackView::parse(&[]);
        assert!(matches!(result, Err(Error::TooShort)));
    }
}
