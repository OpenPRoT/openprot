// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! High-level blocking flash interface.

#![cfg_attr(not(test), no_std)]

use core::{cmp::min, num::NonZero};
pub use hal_flash_driver::FlashAddress;
use hal_flash_driver::FlashDriver;
use util_error::ErrorCode;
use util_io::RandomRead;
use util_types::{Blocking, PowerOf2Usize};

/// High-level flash interface.
///
/// This trait provides a simplified, blocking interface for flash operations.
pub trait Flash {
    /// Returns the geometry of the flash.
    ///
    /// Returns a tuple of (total_size, page_size, erasable_sizes_bitmap).
    fn geometry(&self) -> (NonZero<usize>, PowerOf2Usize, u32);

    /// Reads data from flash.
    ///
    /// # Arguments
    /// * `start_addr`: The address to start reading from.
    /// * `buf`: The buffer to read data into.
    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), ErrorCode>;

    /// Erases a block of flash.
    ///
    /// # Arguments
    /// * `start_addr`: The start address of the block to erase.
    /// * `size`: The size of the block to erase.
    fn erase(&mut self, start_addr: FlashAddress, size: PowerOf2Usize) -> Result<(), ErrorCode>;

    /// Programs data into flash.
    ///
    /// # Arguments
    /// * `start_addr`: The address to start programming at.
    /// * `data`: The data to program.
    fn program(&mut self, start_addr: FlashAddress, data: &[u8]) -> Result<(), ErrorCode>;

    /// Returns a `RandomRead` implementation for this flash.
    fn random_reader(&mut self) -> impl RandomRead
    where
        Self: Sized,
    {
        FlashRandomReader(self)
    }
}

impl<F: Flash> Flash for &mut F {
    #[inline(always)]
    fn geometry(&self) -> (NonZero<usize>, PowerOf2Usize, u32) {
        (**self).geometry()
    }
    #[inline(always)]
    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), ErrorCode> {
        (**self).read(start_addr, buf)
    }
    #[inline(always)]
    fn program(&mut self, start_addr: FlashAddress, data: &[u8]) -> Result<(), ErrorCode> {
        (**self).program(start_addr, data)
    }
    #[inline(always)]
    fn erase(&mut self, start_addr: FlashAddress, size: PowerOf2Usize) -> Result<(), ErrorCode> {
        (**self).erase(start_addr, size)
    }
}

/// A trait that can be used to constrain the page-size of the flash.
///
/// If you just need to read the page size at runtime, use `Flash::geometry()` instead.
pub trait FlashPageSize {
    /// The size of a flash page in bytes.
    const PAGE_SIZE: usize;
}

/// A blocking flash implementation that wraps a `FlashDriver`.
///
/// This struct uses a `Blocking` implementation to wait for asynchronous flash
/// operations to complete.
pub struct BlockingFlash<TDriver: FlashDriver, TBlocking: Blocking> {
    /// The underlying flash driver.
    pub driver: TDriver,
    /// The blocking mechanism used to wait for operations.
    pub blocking: TBlocking,
}

impl<TDriver: FlashDriver, TBlocking: Blocking> FlashPageSize
    for BlockingFlash<TDriver, TBlocking>
{
    const PAGE_SIZE: usize = 1 << (TDriver::ERASABLE_SIZES_BITMAP.trailing_zeros());
}

impl<TDriver: FlashDriver, TBlocking: Blocking> Flash for BlockingFlash<TDriver, TBlocking> {
    fn geometry(&self) -> (NonZero<usize>, PowerOf2Usize, u32) {
        let page_size = PowerOf2Usize::new(1 << (TDriver::ERASABLE_SIZES_BITMAP.trailing_zeros())).unwrap();
        (self.driver.size(), page_size, TDriver::ERASABLE_SIZES_BITMAP)
    }
    fn read(&mut self, start_addr: FlashAddress, mut buf: &mut [u8]) -> Result<(), ErrorCode> {
        let mut addr = start_addr;
        let align_skip_len = (addr.offset() & (TDriver::READ_ALIGNMENT as u32 - 1)) as usize;
        if (align_skip_len) != 0 {
            assert!(TDriver::READ_ALIGNMENT <= 16);
            let mut tmp = [0_u8; 16];
            let prefix_count = min(TDriver::READ_ALIGNMENT - align_skip_len, buf.len());
            self.driver
                .read(addr & !(TDriver::READ_ALIGNMENT - 1), &mut tmp)?;
            buf[..prefix_count].copy_from_slice(&tmp[align_skip_len..][..prefix_count]);
            buf = &mut buf[prefix_count..];
            addr += prefix_count;
        }
        for buf_chunk in buf.chunks_mut(TDriver::MAX_READ_SIZE) {
            self.driver.read(addr, buf_chunk)?;
            addr += buf_chunk.len();
        }
        Ok(())
    }
    fn erase(&mut self, start_addr: FlashAddress, size: PowerOf2Usize) -> Result<(), ErrorCode> {
        self.driver.start_erase(start_addr, size)?;
        self.blocking.wait_for_notification();
        self.driver.complete_op()
    }
    fn program(&mut self, start_addr: FlashAddress, mut data: &[u8]) -> Result<(), ErrorCode> {
        assert!(
            TDriver::PROGRAM_WINDOW_SIZE.count_ones() == 1,
            "TDriver::PROGRAM_WINDOW_SIZE must be a power of 2"
        );
        let window_mask = TDriver::PROGRAM_WINDOW_SIZE - 1;
        let mut addr = start_addr;
        while !data.is_empty() {
            let chunk = &data[..min(
                data.len(),
                TDriver::PROGRAM_WINDOW_SIZE - ((addr.offset() & window_mask as u32) as usize),
            )];
            self.driver.start_program(addr, chunk)?;
            self.blocking.wait_for_notification();
            self.driver.complete_op()?;
            data = &data[chunk.len()..];
            addr += chunk.len();
        }
        Ok(())
    }
}

struct FlashRandomReader<'a, F: Flash>(&'a mut F);
impl<F: Flash> RandomRead for FlashRandomReader<'_, F> {
    fn read(&mut self, start_addr: usize, buf: &mut [u8]) -> Result<(), ErrorCode> {
        self.0.read(FlashAddress::new(0, start_addr as u32), buf)
    }
    fn size(&self) -> usize {
        self.0.geometry().0.get()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub struct FakeBlocking();
    impl Blocking for FakeBlocking {
        fn wait_for_notification(&self) {}
    }

    #[derive(Clone)]
    pub struct FakeFlashDriver {
        pub data: Vec<u8>,
        pub check_err_result: Result<(), ErrorCode>,
    }
    impl FakeFlashDriver {
        pub fn new(data: Vec<u8>) -> Self {
            Self {
                data,
                check_err_result: Ok(()),
            }
        }
    }
    impl FlashDriver for FakeFlashDriver {
        const ERASABLE_SIZES_BITMAP: u32 = 1 << 11;
        const PROGRAM_WINDOW_SIZE: usize = 64;
        const MAX_READ_SIZE: usize = 4096;
        const READ_ALIGNMENT: usize = 4;
        const PROGRAM_ALIGNMENT: usize = 8;

        fn size(&self) -> NonZero<usize> {
            NonZero::new(self.data.len()).unwrap()
        }
        fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), ErrorCode> {
            let start_addr = start_addr.offset() as usize;
            assert!(start_addr.checked_add(buf.len()).unwrap() <= self.data.len());
            assert!(buf.len() <= Self::MAX_READ_SIZE);
            assert!(start_addr % Self::READ_ALIGNMENT == 0);
            buf.copy_from_slice(&self.data[start_addr..][..buf.len()]);
            Ok(())
        }
        fn start_erase(&mut self, start_addr: FlashAddress, size: PowerOf2Usize) -> Result<(), ErrorCode> {
            let start_addr = start_addr.offset() as usize;
            assert_eq!(size.get(), 2048);
            assert!(start_addr.checked_add(size.get()).unwrap() <= self.data.len());
            assert!(start_addr % size.get() == 0);
            self.data[start_addr..][..size.get()].fill(0xff);
            Ok(())
        }
        fn start_program(
            &mut self,
            start_addr: FlashAddress,
            data: &[u8],
        ) -> Result<(), ErrorCode> {
            let start_addr = start_addr.offset() as usize;
            assert!(start_addr.checked_add(data.len()).unwrap() <= self.data.len());
            assert!(
                data.len() <= Self::PROGRAM_WINDOW_SIZE,
                "Program window violation"
            );
            let end_addr = start_addr.wrapping_add(data.len());
            assert!(
                start_addr / Self::PROGRAM_WINDOW_SIZE
                    == (end_addr - 1) / Self::PROGRAM_WINDOW_SIZE,
                "Program window violation"
            );
            for (dest, src) in self.data[start_addr..end_addr].iter_mut().zip(data) {
                *dest &= *src;
            }
            Ok(())
        }
        fn is_busy(&mut self) -> bool {
            false
        }
        fn complete_op(&mut self) -> Result<(), ErrorCode> {
            self.check_err_result
        }
    }

    #[test]
    #[should_panic(expected = "Program window violation")]
    pub fn test_fake_flash_program_window_violation_0() {
        let mut flash_driver = FakeFlashDriver::new((0..255).collect());
        flash_driver.start_program(FlashAddress::new(0, 0x3c), &[0x42; 5]).unwrap();
    }

    #[test]
    #[should_panic(expected = "Program window violation")]
    pub fn test_fake_flash_program_window_violation_1() {
        let mut flash_driver = FakeFlashDriver::new((0..255).collect());
        flash_driver.start_program(FlashAddress::new(0, 0x0), &[0; 68]).unwrap();
    }

    #[test]
    pub fn test_fake_flash_full_program_window() {
        let mut flash_driver = FakeFlashDriver::new((0..255).collect());
        flash_driver.start_program(FlashAddress::new(0, 0x40), &[0; 0x40]).unwrap();
        assert_eq!(flash_driver.data[0x40..0x80], [0; 0x40]);
    }

    #[test]
    pub fn test_size() {
        let flash_driver = FakeFlashDriver::new((0..255).collect());
        let mut flash = BlockingFlash {
            driver: flash_driver,
            blocking: FakeBlocking(),
        };

        assert_eq!(flash.geometry().0.get(), 255);
        assert_eq!(flash.random_reader().size(), 255);
    }

    #[test]
    pub fn test_read() {
        let flash_driver = FakeFlashDriver::new((0..255).collect());

        let mut flash = BlockingFlash {
            driver: flash_driver,
            blocking: FakeBlocking(),
        };

        let mut buf = [0_u8; 4];
        flash.read(FlashAddress::new(0, 0), &mut buf).unwrap();
        assert_eq!(buf, [0_u8, 1, 2, 3]);

        let mut buf = [0_u8; 4];
        flash.read(FlashAddress::new(0, 1), &mut buf).unwrap();
        assert_eq!(buf, [1, 2, 3, 4]);

        let mut buf = [0_u8; 4];
        flash.read(FlashAddress::new(0, 2), &mut buf).unwrap();
        assert_eq!(buf, [2, 3, 4, 5]);

        let mut buf = [0_u8; 4];
        flash.random_reader().read(2, &mut buf).unwrap();
        assert_eq!(buf, [2, 3, 4, 5]);

        let mut buf = [0_u8; 4];
        flash.read(FlashAddress::new(0, 3), &mut buf).unwrap();
        assert_eq!(buf, [3, 4, 5, 6]);

        let mut buf = [0_u8; 6];
        flash.read(FlashAddress::new(0, 3), &mut buf).unwrap();
        assert_eq!(buf, [3, 4, 5, 6, 7, 8]);

        for i in 0..32 {
            let mut buf = [0_u8; 32];
            flash.read(FlashAddress::new(0, 0), &mut buf[..i]).unwrap();
            assert_eq!(&buf[..i], &flash.driver.data[..i]);
        }

        for i in 0..32 {
            let mut buf = [0_u8; 32];
            flash.read(FlashAddress::new(0, 32 - i as u32), &mut buf[..i]).unwrap();
            assert_eq!(&buf[..i], &flash.driver.data[32 - i..32]);
        }
    }

    #[test]
    pub fn test_erase() {
        let mut flash = BlockingFlash {
            driver: FakeFlashDriver::new(vec![0x42; 0x4000]),
            blocking: FakeBlocking(),
        };
        flash.erase(FlashAddress::new(0, 0x0800), PowerOf2Usize::new(2048).unwrap()).unwrap();
        assert_eq!(flash.driver.data[0x0000..0x0800], [0x42; 0x0800]);
        assert_eq!(flash.driver.data[0x0800..0x1000], [0xff; 0x0800]);
        assert_eq!(flash.driver.data[0x1000..0x4000], [0x42; 0x3000]);

        flash.erase(FlashAddress::new(0, 0x3000), PowerOf2Usize::new(2048).unwrap()).unwrap();
        assert_eq!(flash.driver.data[0x0000..0x0800], [0x42; 0x0800]);
        assert_eq!(flash.driver.data[0x0800..0x1000], [0xff; 0x0800]);
        assert_eq!(flash.driver.data[0x1000..0x3000], [0x42; 0x2000]);
        assert_eq!(flash.driver.data[0x3000..0x3800], [0xff; 0x0800]);
        assert_eq!(flash.driver.data[0x3800..0x4000], [0x42; 0x0800]);
    }

    #[test]
    pub fn test_program() {
        let mut flash = BlockingFlash {
            driver: FakeFlashDriver::new(vec![0xff; 8192]),
            blocking: FakeBlocking(),
        };

        flash
            .program(FlashAddress::new(0, 0x3c), &[0x10, 0x11, 0x12, 0x13, 0x14, 0x15])
            .unwrap();
        assert_eq!(
            flash.driver.data[0x38..0x44],
            [0xff, 0xff, 0xff, 0xff, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0xff, 0xff]
        );

        flash.program(FlashAddress::new(0, 0x40), &[0x24, 0x25]).unwrap();
        assert_eq!(
            flash.driver.data[0x38..0x44],
            [0xff, 0xff, 0xff, 0xff, 0x10, 0x11, 0x12, 0x13, 0x04, 0x05, 0xff, 0xff]
        );
    }
}
