use crate::boot_log::BootLog;
use crate::boot_svc::BootSvc;
use crate::rom_error::RomError;
use crate::tags::RetRamVersion;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

#[derive(FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct RetRam {
    pub version: RetRamVersion,
    pub reset_reasons: u32,
    pub boot_svc: BootSvc,
    pub reserved: [u8; 1652],
    pub boot_log: BootLog,
    pub last_shutdown_reason: RomError,
    pub owner: [u8; 2048],
}

impl RetRam {
    pub unsafe fn mut_ref() -> &'static mut RetRam {
        unsafe {
            let rr = core::slice::from_raw_parts_mut(0x4060_0000 as *mut u8, 4096);
            RetRam::mut_from_bytes(rr).unwrap()
        }
    }
}
