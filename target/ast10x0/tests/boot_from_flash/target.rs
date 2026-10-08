// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Boot-from-flash de-risk test: the image copies itself into the boot flash on
//! FMC CS0, overlaying an Aspeed secure-boot header, then the board is restarted
//! with FWSPICK low to see whether the boot ROM runs it.
//!
//! The AST1060 is non-XIP, so the image the Pi loads over UART is already a
//! complete contiguous copy of itself in SRAM. There is no separate bootstrap
//! payload: the running image is its own source.
//!
//! Kernel-only, so a failure can only be about flash and routing.

#![no_std]
#![no_main]

use ast10x0_peripherals::aperture::{take_aperture, Aperture};
use ast10x0_peripherals::scu::pinctrl::PINCTRL_FMC_QUAD;
use ast10x0_peripherals::scu::ScuRegisters;
use ast10x0_peripherals::smc::{
    FlashConfig, FmcUninit, SmcConfig, SmcController, SmcError, SmcInstance, SmcTopology,
    SpiNorFlash, SpiNorFlashDevice,
};
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

/// The boot flash lives on FMC CS0. CS1 is the external part and is left alone.
struct FmcInstance;

impl SmcInstance for FmcInstance {
    type Regs = Aperture;
    const CONTROLLER: SmcController = SmcController::Fmc;
    const CONFIG: SmcConfig = SmcConfig {
        cs0: Some(FlashConfig { spi_clock_mhz: 50 }),
        cs1: None,
        dma_enabled: false,
        enable_interrupts: false,
        topology: SmcTopology::BootSpi { master_idx: 0 },
    };
}

unsafe extern "C" {
    static pw_boot_vector_table_addr: u8;
    static _pw_static_init_flash_start: u8;
    static __sdata: u8;
    static __edata: u8;
}

/// Largest unit this test moves in one step: one flash page.
const CHUNK: usize = 256;

/// Where the boot ROM looks for the Aspeed secure-boot header, immediately past
/// a full 256-entry Cortex-M vector table. Our table is much shorter, so this
/// lands in the padding the linker already reserves.
const SB_HEADER_OFFSET: usize = 0x400;

/// The part is non-XIP, so the ROM has to copy the image into SRAM before it can
/// jump. `img_size` is the only field that tells it how far to copy; the rest of
/// the header is zero on an unfused part.
fn sb_header(img_size: usize) -> [u8; 32] {
    let mut header = [0u8; 32];
    header[8..12].copy_from_slice(&(img_size as u32).to_le_bytes());
    header
}

/// Fills `out` with the bytes that belong at `offset` in the staged copy: the
/// image, with the secure-boot header overlaid where the two intersect.
fn staged_chunk(image: &[u8], offset: usize, header: &[u8; 32], out: &mut [u8]) {
    out.copy_from_slice(&image[offset..offset + out.len()]);
    let start = SB_HEADER_OFFSET.max(offset);
    let end = (SB_HEADER_OFFSET + header.len()).min(offset + out.len());
    if start < end {
        out[start - offset..end - offset]
            .copy_from_slice(&header[start - SB_HEADER_OFFSET..end - SB_HEADER_OFFSET]);
    }
}

/// A JEDEC ID of all-00 means the bus is held low, all-ff means it is floating.
/// A byte count from `read()` proves nothing: it is controller-local and passes
/// on a dead bus.
fn jedec_is_present(id: [u8; 3]) -> bool {
    id != [0x00, 0x00, 0x00] && id != [0xff, 0xff, 0xff]
}

/// How many bytes of SRAM the loaded `.bin` occupies, starting at address 0:
/// everything up to the load address of `.static_init_ram` plus its size.
fn image_len() -> usize {
    let load_start = &raw const _pw_static_init_flash_start as usize;
    let data_len = (&raw const __edata as usize) - (&raw const __sdata as usize);
    load_start + data_len
}

fn run_test() -> Result<(), SmcError> {
    // SAFETY: kernel main() runs once with exclusive hardware ownership.
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    scu.apply_pinctrl_group(PINCTRL_FMC_QUAD);

    let mut fmc = unsafe {
        FmcUninit::<FmcInstance>::new(take_aperture(), take_aperture(), take_aperture())?
    }
    .init()?;

    let jedec = SpiNorFlash::new(fmc.cs0()?)?.jedec_id()?;
    pw_log::info!(
        "boot flash: FMC CS0 JEDEC ID {:02x} {:02x} {:02x}",
        jedec[0] as u32,
        jedec[1] as u32,
        jedec[2] as u32
    );
    if !jedec_is_present(jedec) {
        return Err(SmcError::DeviceNotSupported);
    }

    let geom = fmc.cs0()?.geometry();
    let len = image_len();
    pw_log::info!(
        "boot flash: staging {} bytes, sector={} page={}",
        len as u32,
        geom.sector_size as u32,
        geom.page_size as u32
    );
    if len as u64 > geom.capacity_bytes {
        return Err(SmcError::InvalidCapacity);
    }

    // SAFETY: on this non-XIP part the loaded image is contiguous SRAM. It begins
    // one word before the vector table symbol, where the linker puts the initial
    // stack pointer, and `len` is its extent.
    let image = unsafe {
        let start = (&raw const pw_boot_vector_table_addr).sub(4);
        core::slice::from_raw_parts(start, len)
    };

    let header = sb_header(len);
    let mut staged = [0u8; CHUNK];

    {
        let mut flash = SpiNorFlash::new(fmc.cs0()?)?;
        let mut offset = 0;
        while offset < len {
            flash.erase_sector(offset as u32)?;
            offset += geom.sector_size as usize;
        }
        let page_len = (geom.page_size as usize).min(CHUNK);
        for (i, page) in image.chunks(page_len).enumerate() {
            let buf = &mut staged[..page.len()];
            staged_chunk(image, i * page_len, &header, buf);
            flash.program_page((i * page_len) as u32, buf)?;
        }
    }

    let flash = SpiNorFlash::new(fmc.cs0()?)?;
    let mut readback = [0u8; CHUNK];
    for (i, chunk) in image.chunks(CHUNK).enumerate() {
        let want = &mut staged[..chunk.len()];
        staged_chunk(image, i * CHUNK, &header, want);
        let got = &mut readback[..chunk.len()];
        flash.read((i * CHUNK) as u32, got)?;
        if got != want {
            pw_log::error!("boot flash: readback differs at {}", (i * CHUNK) as u32);
            return Err(SmcError::HardwareError);
        }
    }

    pw_log::info!("boot flash: image staged and verified; restart with FWSPICK low");
    Ok(())
}

fn smc_error_str(e: SmcError) -> &'static str {
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

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 boot from flash";

    fn main() -> ! {
        let sentinel = match run_test() {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(e) => {
                pw_log::error!("boot flash test failed: {}", smc_error_str(e) as &str);
                b"TEST_RESULT:FAIL\n"
            }
        };
        let _ = console_backend_write_all(sentinel);

        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
