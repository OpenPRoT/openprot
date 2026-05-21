#![no_std]

pub mod error;
pub mod flash;
mod mubi;

pub use flash::EarlgreyFlashAddress;
pub use mubi::AsMubi;
