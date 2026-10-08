// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What the writer and the reader of the shared-flash test both need: the
//! staging flash's identity, the pattern they agree on, and the sentinel.

use ast10x0_peripherals::aperture::Aperture;
use ast10x0_peripherals::smc::{
    FlashConfig, FlashGeometry, Pinned, SmcConfig, SmcController, SmcError, SmcInstance,
    SmcTopology,
};
use console_backend::console_backend_write_all;

/// The shared staging flash on SPI1 CS0: a 64 MiB part with 4 KiB sectors.
pub const STAGING_FLASH_GEOMETRY: FlashGeometry = FlashGeometry {
    capacity_bytes: 0x0400_0000,
    page_size: 256,
    sector_size: 4096,
    block_size: 65536,
};

/// The sector the PLDM firmware device stages into, so this test exercises the
/// address the real flow uses.
pub const MAGIC_OFFSET: u32 = 0x10_0000;

/// One page, prefixed with a magic so it is distinguishable from all-00/all-ff.
pub const PATTERN_LEN: usize = 256;

const fn pattern() -> [u8; PATTERN_LEN] {
    let magic = *b"OPRTSHM1";
    let mut out = [0u8; PATTERN_LEN];
    let mut i = 0;
    while i < magic.len() {
        out[i] = magic[i];
        i += 1;
    }
    while i < PATTERN_LEN {
        out[i] = i as u8;
        i += 1;
    }
    out
}

pub const PATTERN: [u8; PATTERN_LEN] = pattern();

pub struct Spi1Instance;

impl SmcInstance for Spi1Instance {
    type Regs = Aperture;
    const CONTROLLER: SmcController = SmcController::Spi1;
    const CONFIG: SmcConfig = SmcConfig {
        cs0: Some(FlashConfig { spi_clock_mhz: 25 }),
        cs1: None,
        dma_enabled: false,
        enable_interrupts: false,
        topology: SmcTopology::HostSpi { master_idx: 0 },
    };

    /// Pinned, so `init` reaches the device without an SFDP read: the part's
    /// BFP table runs past the 256 bytes `Discover` reads, which it rejects.
    type Cs0Geometry = Pinned<STAGING_FLASH_GEOMETRY>;
}

pub fn smc_error_str(e: SmcError) -> &'static str {
    match e {
        SmcError::HardwareError => "HardwareError",
        SmcError::Timeout => "Timeout",
        SmcError::DmaAborted => "DmaAborted",
        SmcError::DmaLengthMismatch => "DmaLengthMismatch",
        SmcError::InvalidChipSelect => "InvalidChipSelect",
        SmcError::InvalidCapacity => "InvalidCapacity",
        SmcError::DeviceNotSupported => "DeviceNotSupported",
        SmcError::WriteProtected => "WriteProtected",
        SmcError::WriteInProgress => "WriteInProgress",
        SmcError::ControllerNotReady => "ControllerNotReady",
        SmcError::DmaNotEnabled => "DmaNotEnabled",
    }
}

/// Report the verdict and park: kernel-only targets have no `shutdown`.
pub fn finish(result: Result<(), SmcError>) -> ! {
    let sentinel = match result {
        Ok(()) => b"TEST_RESULT:PASS\n",
        Err(e) => {
            pw_log::error!("shared flash test failed: {}", smc_error_str(e) as &str);
            b"TEST_RESULT:FAIL\n"
        }
    };
    let _ = console_backend_write_all(sentinel);

    #[expect(clippy::empty_loop)]
    loop {}
}
