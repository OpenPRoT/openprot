#![no_std]

pub mod boot_log;
pub mod boot_svc;
pub mod error;
mod misc;
mod mubi;
mod perso_tlv;
pub mod ret_ram;
pub mod rom_error;
pub mod tags;

pub use mubi::AsMubi;
pub use perso_tlv::{PersoCertificate, PersoTlvType};

pub trait CheckDigest {
    /// Check the digest on a data structure by calling `f` to generate a digest.
    fn check_digest<F>(&self, f: F) -> bool
    where
        F: Fn(&[u8]) -> [u8; 32];

    /// Set the digest on a data structure by calling `f` to generate the digest.
    fn set_digest<F>(&mut self, f: F)
    where
        F: Fn(&[u8]) -> [u8; 32];
}

pub trait GetData<T> {
    fn get(&self) -> Option<&T>;
    fn get_mut(&mut self) -> &mut T;
}
