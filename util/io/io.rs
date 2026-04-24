#![no_std]

use util_error::{ErrorCode, ErrorModule};

pub const IO_GENERIC: ErrorModule = ErrorModule::new(0x494F); // ascii: IO
pub const IO_GENERIC_READ_OUT_OF_BOUNDS: ErrorCode =
    IO_GENERIC.from_pw(1, pw_status::Error::OutOfRange);

/// Trait for random access read
pub trait RandomRead {
    fn read(&mut self, start_addr: usize, dst: &mut [u8]) -> Result<(), ErrorCode>;
    fn size(&self) -> usize;
}

impl RandomRead for &[u8] {
    fn read(&mut self, start_addr: usize, dst: &mut [u8]) -> Result<(), ErrorCode> {
        // Explicit wrapping add. Overflows are expected to
        // be detected in the indexing operation
        let end_addr = start_addr.wrapping_add(dst.len());
        let src = self
            .get(start_addr..end_addr)
            .ok_or(IO_GENERIC_READ_OUT_OF_BOUNDS)?;
        dst.copy_from_slice(src);
        Ok(())
    }
    fn size(&self) -> usize {
        self.len()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_read() {
        let mut src: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut dst: [u8; 3] = [0; 3];
        assert!(src.read(6, &mut dst).is_ok());
        assert_eq!(&dst, &[7, 8, 9]);
        assert_eq!(RandomRead::size(&src), 9);
    }

    #[test]
    fn should_fail() {
        let mut src: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut dst: [u8; 3] = [0; 3];
        assert!(src.read(7, &mut dst).is_err());
    }

    #[test]
    fn invalid_start_address_should_not_panic() {
        let mut src: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8];
        let mut dst: [u8; 4] = [0; 4];
        // Set `start_addr` so that adding `dst.len()` causes it to
        // wrap around and become smaller than `src.len()`
        let start_addr: usize = usize::MAX - (dst.len() - 1);
        // Should not panic
        let result = src.read(start_addr, &mut dst);
        assert!(result.is_err());
    }
}
