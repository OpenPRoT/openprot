// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Tests the FPGA flash driver against a copy of the host code that implements
//! the FPGA's flash controllers.
//!
//! On the VCK190 FPGA board, the FPGA wrapper provides a block of registers
//! for each flash controller. The host is the board's ARM cores, which run
//! caliptra-sw's `ModelFpgaSubsystem`. A program on the host calls
//! `ModelFpgaSubsystem::step` in a loop (the step loop). Each call polls
//! those registers and transfers pages between them and host memory. This
//! test runs the driver against a copy of that host code:
//!
//! * [`FpgaHost`] contains the flash handling of `ModelFpgaSubsystem`, copied
//!   from caliptra-sw `hw-model/src/model_fpga_subsystem.rs` at commit
//!   b99535d. The copy does not follow the pin in
//!   `third_party/caliptra/versions.bzl`, so compare it with upstream when
//!   the pin changes. The copy differs from upstream in three places, each
//!   marked with a comment that begins `Changed from upstream`.
//! * `caliptra_hw_model_fpga_regs` provides the register layout. It is built
//!   from `hw-model/src/fpga_regs.rs` at the caliptra-sw commit that
//!   `third_party/caliptra` pins, so it follows the pin.
//! * [`Wrapper`] is the storage for the registers. [`Board::new`] sets their
//!   reset values, which are from caliptra-mcu-sw
//!   `hw/fpga/src/caliptra_fpga_realtime_regs.rdl`.
//!
//! The driver accesses the registers through [`FpgaMmioBlock`]. [`Board::mcu`]
//! translates the address that the driver uses on the board to the same
//! offset in [`Wrapper`]. [`Board`] runs the host's step loop between the
//! driver's register accesses.

use core::cell::{Cell, RefCell, UnsafeCell};
use core::ops::Range;

use caliptra_hw_model_fpga_regs::{FlashControl, FlashCtrlRegs, FlashOpStatus};
use hal_flash::{BlockingFlash, Flash, FlashAddress};
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use util_error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS;
use util_types::PowerOf2Usize;
use veer_flash::{
    BufWord, FpgaFlashCtrl, FpgaMmio, FpgaMmioBlock, Mmio, NoWait, PageController, Reg, SoftFlash,
    ERASE_SIZE, FLASH_VEER_OP_FAILED, FLASH_VEER_TIMEOUT, FPGA_PRIMARY_FLASH_CTRL_ADDR,
    FPGA_SECONDARY_FLASH_CTRL_ADDR, PAGE_SIZE,
};

/// Address of the FPGA wrapper on the AXI bus that the MCU and the host share.
/// The host's mapping of the wrapper starts at this address (caliptra-sw
/// `hw/fpga/io_module/mcu_io_module.c`).
const WRAPPER_ADDR: usize = 0xA401_0000;

/// Length of the host's mapping of the wrapper.
const WRAPPER_SIZE: usize = 0x1_0000;

/// Storage for the registers of the FPGA wrapper.
struct Wrapper {
    /// The host's mapping of the registers, as in `ModelFpgaSubsystem`.
    ptr: *mut u32,
    _registers: Box<[UnsafeCell<u32>]>,
}

impl Wrapper {
    fn new() -> Self {
        let registers: Box<[UnsafeCell<u32>]> =
            (0..WRAPPER_SIZE / 4).map(|_| UnsafeCell::new(0)).collect();
        Self {
            ptr: UnsafeCell::raw_get(registers.as_ptr()),
            _registers: registers,
        }
    }
}

// Copied from caliptra-sw: the next two constants, and `flash_ctrl_regs`,
// `handle_flash` and `handle_flash_ctrl` in `FpgaHost`.

// Default flash size: 16MB each, initialized to 0xFF
const DEFAULT_FLASH_SIZE: usize = 16 * 1024 * 1024;

// page size of simulated flash
const FLASH_PAGE_SIZE: usize = 256;

/// The flash handling of caliptra-sw's `ModelFpgaSubsystem`.
struct FpgaHost {
    wrapper: Wrapper,
    // Flash storage buffers for the flash controllers
    primary_flash: Vec<u8>,
    secondary_flash: Vec<u8>,
}

impl FpgaHost {
    /// Creates the flash storage in the same way as
    /// `ModelFpgaSubsystem::new_unbooted` does when it is given no initial
    /// contents.
    fn new() -> Self {
        Self {
            wrapper: Wrapper::new(),
            primary_flash: vec![0xFF; DEFAULT_FLASH_SIZE],
            secondary_flash: vec![0xFF; DEFAULT_FLASH_SIZE],
        }
    }

    /// Get a reference to the flash controller registers at the specified offset
    fn flash_ctrl_regs(&self, is_primary: bool) -> &FlashCtrlRegs {
        let offset = if is_primary { 0x2000 } else { 0x3000 };
        // SAFETY: These are at fixed, known addresses relative to the wrapper.
        unsafe {
            let wrapper_base = self.wrapper.ptr as *const u8;
            let flash_ctrl_ptr = wrapper_base.add(offset) as *const FlashCtrlRegs;
            &*flash_ctrl_ptr
        }
    }

    /// Implement a pair of simulated flash controllers, backed by data
    /// stored in the host memory.
    fn handle_flash(&mut self) {
        self.handle_flash_ctrl(true);
        self.handle_flash_ctrl(false);
    }

    /// Handle operations for a single flash controller
    ///
    /// Changed from upstream: upstream returns early while the Caliptra
    /// subsystem is in reset. This test has no reset state, so the copy omits
    /// that check.
    fn handle_flash_ctrl(&mut self, is_primary: bool) {
        let (start, page_num, op) = {
            let regs = self.flash_ctrl_regs(is_primary);
            (
                regs.fl_control.read(FlashControl::START),
                regs.page_num.get() as usize,
                regs.fl_control.read(FlashControl::OP),
            )
        };

        // Check if START bit is set
        if start == 0 {
            return;
        }

        let flash = if is_primary {
            &mut self.primary_flash
        } else {
            &mut self.secondary_flash
        };
        let flash_len = flash.len();
        let flash_offset = page_num * FLASH_PAGE_SIZE;

        // Ensure flash buffer is large enough
        let required_size = flash_offset + FLASH_PAGE_SIZE;
        if flash_len < required_size {
            flash.resize(required_size, 0xFF);
        }

        let mut error = 0u32;
        if flash_offset + FLASH_PAGE_SIZE > flash.len() {
            error = 1;
            println!(
                "[flash-ctrl] Flash operation FAILED: Flash offset 0x{:x} + size {} exceeds flash length {}",
                flash_offset,
                FLASH_PAGE_SIZE,
                flash.len()
            );
        } else {
            match op {
                1 => {
                    // Read page: copy from flash buffer to memory at page_addr
                    let page_data = flash[flash_offset..flash_offset + FLASH_PAGE_SIZE].to_vec();
                    let word_count = FLASH_PAGE_SIZE / 4;
                    // re-borrow to avoid borrow checker
                    let flash_ctrl = self.flash_ctrl_regs(is_primary);
                    for i in 0..word_count {
                        let byte_offset = i * 4;
                        let value = u32::from_le_bytes(
                            page_data[byte_offset..byte_offset + 4].try_into().unwrap(),
                        );
                        flash_ctrl.buffer[i].set(value);
                    }
                }
                2 => {
                    // Write page: copy from memory at page_addr to flash buffer
                    let word_count = FLASH_PAGE_SIZE / 4;

                    let flash_ctrl = self.flash_ctrl_regs(is_primary);
                    // Changed from upstream: the range has no `.into_iter()`,
                    // for clippy.
                    let buffer: Vec<u8> = (0..word_count)
                        .flat_map(|i| flash_ctrl.buffer[i].get().to_le_bytes())
                        .collect();

                    // re-borrow to avoid borrow checker conflicts
                    let flash = if is_primary {
                        &mut self.primary_flash
                    } else {
                        &mut self.secondary_flash
                    };
                    flash[flash_offset..flash_offset + FLASH_PAGE_SIZE].copy_from_slice(&buffer);
                }
                3 => {
                    // Erase page: fill with 0xFF
                    flash[flash_offset..flash_offset + FLASH_PAGE_SIZE].fill(0xFF);
                }
                _ => {
                    // Changed from upstream: `op` is an inline format
                    // argument in the `println!`, for clippy.
                    error = 1; // Treat invalid operation as read error
                    println!("[flash-ctrl] FAILED: Invalid operation code {op}");
                }
            }
        }

        // Clear START bit and set DONE
        let regs = self.flash_ctrl_regs(is_primary);
        regs.fl_control.modify(FlashControl::START::CLEAR);
        regs.op_status
            .modify(FlashOpStatus::DONE::SET + FlashOpStatus::ERR.val(error));
        regs.fl_interrupt_state.set(2); // event interrupt
    }

    /// `ModelFpgaSubsystem::step`, reduced to its call to `handle_flash`.
    fn step(&mut self) {
        self.handle_flash();
    }
}

/// Models the parts of a VCK190 that the flash driver uses: the wrapper
/// registers and the host. The host runs one pass of its step loop for every
/// `period` register accesses by the MCU.
struct Board {
    host: RefCell<FpgaHost>,
    /// Number of register accesses by the MCU for each host step, or `None`
    /// while no host is running.
    period: Cell<Option<u32>>,
    accesses: Cell<u32>,
    /// When set, `McuBus` clears the operation field of each value that the
    /// MCU writes to FL_CONTROL. The host does not know operation 0, so it
    /// reports an error.
    drop_op: Cell<bool>,
}

impl Board {
    fn new(period: Option<u32>) -> Self {
        let host = FpgaHost::new();
        // Reset values from caliptra_fpga_realtime_regs.rdl.
        for is_primary in [true, false] {
            let regs = host.flash_ctrl_regs(is_primary);
            regs.page_size.set(256);
            regs.ctrl_regwen.set(1);
            regs.flash_size.set(0x0100_0000);
            for word in &regs.buffer {
                word.set(0xffff_ffff);
            }
        }
        Self {
            host: RefCell::new(host),
            period: Cell::new(period),
            accesses: Cell::new(0),
            drop_op: Cell::new(false),
        }
    }

    /// Returns the MCU's access to the registers of the controller at `addr`
    /// on the AXI bus.
    fn mcu(&self, addr: usize) -> McuBus<'_> {
        let offset = addr.checked_sub(WRAPPER_ADDR).unwrap();
        assert!(offset + size_of::<FlashCtrlRegs>() <= WRAPPER_SIZE);
        let wrapper = self.host.borrow().wrapper.ptr.cast::<u8>();
        let base = wrapper.wrapping_add(offset).expose_provenance();
        McuBus {
            // SAFETY: `base` is the address of a controller's registers in the
            // wrapper's storage, which is allocated for as long as `self` is.
            // Nothing else accesses them as the MCU. The host and the tests
            // access them on this thread.
            regs: unsafe { FpgaMmioBlock::new(base) },
            board: self,
        }
    }

    /// Returns the flash of the controller at `addr`, built in the same way as
    /// the on-device test in `target/veer/tests/flash` builds it.
    fn flash(&self, addr: usize) -> impl Flash<Error = util_error::ErrorCode> + '_ {
        BlockingFlash {
            driver: SoftFlash::new(FpgaFlashCtrl::new(self.mcu(addr))),
            blocking: NoWait,
        }
    }

    /// Called before each register access by the MCU. Runs one host step on
    /// every `period`-th call.
    fn tick(&self) {
        let Some(period) = self.period.get() else {
            return;
        };
        let accesses = self.accesses.get() + 1;
        if accesses < period {
            self.accesses.set(accesses);
        } else {
            self.accesses.set(0);
            self.host.borrow_mut().step();
        }
    }
}

/// Register access from the MCU to one controller.
struct McuBus<'a> {
    regs: FpgaMmioBlock,
    board: &'a Board,
}

impl Mmio for McuBus<'_> {
    fn read(&self, reg: Reg) -> u32 {
        self.board.tick();
        self.regs.read(reg)
    }

    fn write(&mut self, reg: Reg, mut value: u32) {
        self.board.tick();
        if self.board.drop_op.get() && reg == Reg::FlControl {
            value &= !(FlashControl::OP.mask << FlashControl::OP.shift);
        }
        self.regs.write(reg, value);
    }
}

impl FpgaMmio for McuBus<'_> {
    fn read_buf(&self, word: BufWord) -> u32 {
        self.board.tick();
        self.regs.read_buf(word)
    }

    fn write_buf(&mut self, word: BufWord, value: u32) {
        self.board.tick();
        self.regs.write_buf(word, value);
    }
}

fn addr(offset: usize) -> FlashAddress {
    FlashAddress::new(offset as u32)
}

fn block() -> PowerOf2Usize {
    PowerOf2Usize::new(ERASE_SIZE).unwrap()
}

/// Returns one byte for each offset in `offsets`. The bytes differ from page
/// to page.
fn pattern(offsets: Range<usize>) -> Vec<u8> {
    offsets.map(|i| (i % 251) as u8).collect()
}

#[test]
fn round_trip_with_fast_and_slow_hosts() {
    // With a period of 1, the host runs before every register access by the
    // MCU. With larger periods, the MCU polls several times before the host
    // responds.
    for period in [1, 2, 3, 5, 8, 13] {
        let board = Board::new(Some(period));
        let mut flash = board.flash(FPGA_PRIMARY_FLASH_CTRL_ADDR);

        let (size, page_size, erase_sizes) = flash.geometry().unwrap();
        {
            let host = board.host.borrow();
            assert_eq!(size.get(), host.primary_flash.len());
            assert_eq!(
                size.get() as u32,
                host.flash_ctrl_regs(true).flash_size.get()
            );
        }
        assert_eq!(page_size, block());
        assert_eq!(erase_sizes, ERASE_SIZE as u32);

        // The flash is initially erased, so fill three blocks with zeros before
        // erasing the middle one.
        board.host.borrow_mut().primary_flash[..3 * ERASE_SIZE].fill(0);
        flash.erase(addr(ERASE_SIZE), block()).unwrap();
        {
            let stored = &board.host.borrow().primary_flash;
            assert_eq!(stored[..ERASE_SIZE], [0; ERASE_SIZE]);
            assert_eq!(stored[ERASE_SIZE..2 * ERASE_SIZE], [0xff; ERASE_SIZE]);
            assert_eq!(stored[2 * ERASE_SIZE..3 * ERASE_SIZE], [0; ERASE_SIZE]);
        }

        // The data starts and ends in the middle of a word, and spans four
        // pages.
        let start = ERASE_SIZE + 0x1b3;
        let data = pattern(0..700);
        flash.program(addr(start), &data).unwrap();
        // Programming can only clear bits.
        flash.program(addr(start), &[0x0f; 5]).unwrap();
        let mut expected = data;
        expected[..5].iter_mut().for_each(|byte| *byte &= 0x0f);
        {
            let stored = &board.host.borrow().primary_flash;
            assert_eq!(stored[ERASE_SIZE..start], [0xff; 0x1b3]);
            assert_eq!(stored[start..start + 700], expected);
            assert_eq!(stored[start + 700..start + 800], [0xff; 100]);
        }

        let mut buf = [0; 700];
        flash.read(addr(start), &mut buf).unwrap();
        assert_eq!(buf[..], expected);
    }
}

#[test]
fn each_controller_has_its_own_flash() {
    let board = Board::new(Some(3));
    let mut primary = board.flash(FPGA_PRIMARY_FLASH_CTRL_ADDR);
    let mut secondary = board.flash(FPGA_SECONDARY_FLASH_CTRL_ADDR);

    primary.program(addr(0x10), b"primary").unwrap();
    secondary.program(addr(0x10), b"secondary").unwrap();
    {
        let host = board.host.borrow();
        assert_eq!(host.primary_flash[0x10..0x19], *b"primary\xff\xff");
        assert_eq!(host.secondary_flash[0x10..0x19], *b"secondary");
    }

    let mut buf = [0; 9];
    primary.read(addr(0x10), &mut buf).unwrap();
    assert_eq!(buf, *b"primary\xff\xff");
    secondary.read(addr(0x10), &mut buf).unwrap();
    assert_eq!(buf, *b"secondary");
}

#[test]
fn accesses_the_last_page_and_rejects_out_of_bounds() {
    let board = Board::new(Some(2));
    let mut flash = board.flash(FPGA_PRIMARY_FLASH_CTRL_ADDR);
    let size = DEFAULT_FLASH_SIZE;

    let data = pattern(0..PAGE_SIZE);
    flash.program(addr(size - PAGE_SIZE), &data).unwrap();
    assert_eq!(board.host.borrow().primary_flash[size - PAGE_SIZE..], data);
    let mut buf = [0; PAGE_SIZE];
    flash.read(addr(size - PAGE_SIZE), &mut buf).unwrap();
    assert_eq!(buf[..], data);
    flash.erase(addr(size - ERASE_SIZE), block()).unwrap();
    assert_eq!(
        board.host.borrow().primary_flash[size - ERASE_SIZE..],
        [0xff; ERASE_SIZE]
    );

    // The host extends its storage to fit any page number, so the driver must
    // reject accesses past the end of the flash.
    let out_of_bounds = Err(FLASH_GENERIC_ADDR_OUT_OF_BOUNDS);
    assert_eq!(flash.read(addr(size - 1), &mut [0; 2]), out_of_bounds);
    assert_eq!(flash.program(addr(size), &[0]), out_of_bounds);
    assert_eq!(flash.erase(addr(size), block()), out_of_bounds);
    assert_eq!(board.host.borrow().primary_flash.len(), size);
}

#[test]
fn page_buffer_rejects_ranges_outside_the_page() {
    let board = Board::new(Some(1));
    let mut controller = FpgaFlashCtrl::new(board.mcu(FPGA_PRIMARY_FLASH_CTRL_ADDR));
    let out_of_bounds = Err(FLASH_GENERIC_ADDR_OUT_OF_BOUNDS);
    assert_eq!(controller.read_buffer(PAGE_SIZE - 1, &mut [0; 1]), Ok(()));
    assert_eq!(
        controller.read_buffer(PAGE_SIZE - 1, &mut [0; 2]),
        out_of_bounds
    );
    assert_eq!(
        controller.read_buffer(usize::MAX, &mut [0; 2]),
        out_of_bounds
    );
    assert_eq!(controller.program_buffer(PAGE_SIZE, &[0]), out_of_bounds);
}

#[test]
fn ignores_stale_status() {
    let board = Board::new(Some(5));
    board.host.borrow_mut().primary_flash[..ERASE_SIZE].copy_from_slice(&pattern(0..ERASE_SIZE));
    let mut flash = board.flash(FPGA_PRIMARY_FLASH_CTRL_ADDR);

    // Set DONE, as it would be if the host completed an operation just after
    // the driver timed out.
    let host = board.host.borrow();
    host.flash_ctrl_regs(true)
        .op_status
        .write(FlashOpStatus::DONE::SET);
    drop(host);

    let mut buf = [0; 8];
    flash.read(addr(0x300), &mut buf).unwrap();
    assert_eq!(buf[..], pattern(0x300..0x308));
}

#[test]
fn reports_host_errors() {
    let board = Board::new(Some(3));
    board.host.borrow_mut().primary_flash[..ERASE_SIZE].copy_from_slice(&pattern(0..ERASE_SIZE));
    let mut flash = board.flash(FPGA_PRIMARY_FLASH_CTRL_ADDR);

    // The host reports an error only for an unknown operation.
    board.drop_op.set(true);
    let failed = Err(FLASH_VEER_OP_FAILED);
    assert_eq!(flash.read(addr(0), &mut [0; 4]), failed);
    assert_eq!(flash.program(addr(0), &[0; 4]), failed);
    assert_eq!(flash.erase(addr(0), block()), failed);

    // The next operation succeeds after a failure.
    board.drop_op.set(false);
    let mut buf = [0; 4];
    assert_eq!(flash.read(addr(0), &mut buf), Ok(()));
    assert_eq!(buf[..], pattern(0..4));
}

#[test]
fn times_out_without_a_host() {
    let board = Board::new(None);
    board.host.borrow_mut().primary_flash[..ERASE_SIZE].copy_from_slice(&pattern(0..ERASE_SIZE));
    let mut flash = board.flash(FPGA_PRIMARY_FLASH_CTRL_ADDR);

    assert_eq!(flash.erase(addr(0), block()), Err(FLASH_VEER_TIMEOUT));

    // Start the host. It must not run the erase that timed out.
    board.period.set(Some(1));
    let mut buf = [0; ERASE_SIZE];
    assert_eq!(flash.read(addr(0), &mut buf), Ok(()));
    assert_eq!(buf[..], pattern(0..ERASE_SIZE));
}

#[test]
fn disables_interrupts() {
    let board = Board::new(Some(1));
    // Enable both interrupts, as earlier code could have left them.
    let host = board.host.borrow();
    host.flash_ctrl_regs(true).fl_interrupt_enable.set(3);
    drop(host);

    let _flash = board.flash(FPGA_PRIMARY_FLASH_CTRL_ADDR);
    let host = board.host.borrow();
    assert_eq!(host.flash_ctrl_regs(true).fl_interrupt_enable.get(), 0);
}
