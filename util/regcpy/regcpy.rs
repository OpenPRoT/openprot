// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # regcpy
//! Helper functions for copying byte-slices between main memory and MMIO spaces.
#![cfg_attr(not(test), no_std)]

// TODO(caliptra-ureg#17): Upstream these helpers into caliptra-ureg.

use aligned::Aligned;
use aligned::A4;
use core::cmp::min;
use zerocopy::FromBytes;
use zerocopy::Unalign;

#[inline(never)]
pub fn copy_to_reg_unaligned(
    reg: &ureg::RegRef<
        impl ureg::WritableReg<WriteVal = u32, Raw = u32> + ureg::ResettableReg<Raw = u32>,
        impl ureg::MmioMut + Copy,
    >,
    src: &[u8],
) {
    let (words, rem_bytes): (&[Unalign<u32>], &[u8]) = FromBytes::ref_from_prefix(src).unwrap();

    for word in words {
        reg.write(|_| word.get());
    }
    if let Some(last_word) = last_word(rem_bytes) {
        reg.write(|_| last_word);
    }
}

#[inline(never)]
pub fn copy_to_reg(
    reg: &ureg::RegRef<
        impl ureg::WritableReg<WriteVal = u32, Raw = u32> + ureg::ResettableReg<Raw = u32>,
        impl ureg::MmioMut + Copy,
    >,
    src: &Aligned<A4, [u8]>,
) {
    // Convert to regular slice; optimizer should be smart enough to realize it's still aligned.
    let (words, rem_bytes): (&[u32], &[u8]) = FromBytes::ref_from_prefix(src.as_ref()).unwrap();

    for word in words {
        reg.write(|_| *word);
    }
    if let Some(last_word) = last_word(rem_bytes) {
        reg.write(|_| last_word);
    }
}

#[inline(never)]
pub fn copy_to_reg_array<const LEN: usize>(
    array: &ureg::Array<
        LEN,
        ureg::RegRef<
            impl ureg::WritableReg<WriteVal = u32, Raw = u32> + ureg::ResettableReg<Raw = u32>,
            impl ureg::MmioMut + Copy,
        >,
    >,
    src: &Aligned<A4, [u8]>,
) {
    // Convert to regular slice; optimizer should be smart enough to realize it's still aligned.
    let (words, rem_bytes): (&[u32], &[u8]) = FromBytes::ref_from_prefix(src.as_ref()).unwrap();

    let words_to_copy = min(LEN, words.len());

    #[allow(clippy::needless_range_loop)] // optimizes better
    for i in 0..words_to_copy {
        array.at(i).write(|_| words[i]);
    }
    let Some(reg) = array.get(words.len()) else {
        return;
    };
    let Some(last_word) = last_word(rem_bytes) else {
        return;
    };
    reg.write(|_| last_word);
}

#[inline(never)]
pub fn copy_to_reg_array_unaligned<const LEN: usize>(
    array: &ureg::Array<
        LEN,
        ureg::RegRef<
            impl ureg::WritableReg<WriteVal = u32, Raw = u32> + ureg::ResettableReg<Raw = u32>,
            impl ureg::MmioMut + Copy,
        >,
    >,
    src: &[u8],
) {
    let (words, rem_bytes): (&[Unalign<u32>], &[u8]) = FromBytes::ref_from_prefix(src).unwrap();

    let words_to_copy = min(LEN, words.len());

    #[allow(clippy::needless_range_loop)] // optimizes better
    for i in 0..words_to_copy {
        array.at(i).write(|_| words[i].get());
    }
    let Some(reg) = array.get(words.len()) else {
        return;
    };
    let Some(last_word) = last_word(rem_bytes) else {
        return;
    };
    reg.write(|_| last_word);
}

#[inline(never)]
pub fn copy_from_reg<const LEN: usize>(
    dest: &mut Aligned<A4, [u8]>,
    reg: &ureg::RegRef<impl ureg::ReadableReg<ReadVal = u32, Raw = u32>, impl ureg::Mmio + Copy>,
) {
    let (words, rem_bytes): (&mut [u32], &mut [u8]) = FromBytes::mut_from_prefix(dest).unwrap();
    let words_to_copy = min(LEN, words.len());

    #[allow(clippy::needless_range_loop)]
    for i in 0..words_to_copy {
        words[i] = reg.read();
    }

    if words_to_copy < LEN {
        set_rem_bytes(rem_bytes, || reg.read());
    }
}

#[inline(never)]
pub fn copy_from_reg_array<const LEN: usize>(
    dest: &mut Aligned<A4, [u8]>,
    array: &ureg::Array<
        LEN,
        ureg::RegRef<impl ureg::ReadableReg<ReadVal = u32, Raw = u32>, impl ureg::Mmio + Copy>,
    >,
) {
    let (words, rem_bytes): (&mut [u32], &mut [u8]) = FromBytes::mut_from_prefix(dest).unwrap();
    let words_to_copy = min(LEN, words.len());

    #[allow(clippy::needless_range_loop)]
    for i in 0..words_to_copy {
        words[i] = array.at(i).read();
    }

    if words_to_copy < LEN {
        set_rem_bytes(rem_bytes, || array.at(words_to_copy).read());
    }
}

#[inline(never)]
pub fn copy_from_reg_array_unaligned<const LEN: usize>(
    dest: &mut [u8],
    array: &ureg::Array<
        LEN,
        ureg::RegRef<impl ureg::ReadableReg<ReadVal = u32, Raw = u32>, impl ureg::Mmio + Copy>,
    >,
) {
    let (words, rem_bytes): (&mut [Unalign<u32>], &mut [u8]) =
        FromBytes::mut_from_prefix(dest).unwrap();
    let words_to_copy = min(LEN, words.len());

    #[allow(clippy::needless_range_loop)]
    for i in 0..words_to_copy {
        words[i].set(array.at(i).read());
    }

    if words_to_copy < LEN {
        set_rem_bytes(rem_bytes, || array.at(words_to_copy).read());
    }
}

#[inline(never)]
pub fn copy_from_reg_unaligned(
    dest: &mut [u8],
    reg: &ureg::RegRef<impl ureg::ReadableReg<ReadVal = u32, Raw = u32>, impl ureg::MmioMut + Copy>,
) {
    let (words, rem_bytes): (&mut [Unalign<u32>], &mut [u8]) =
        FromBytes::mut_from_prefix(dest).unwrap();

    for word in words {
        word.set(reg.read())
    }
    set_rem_bytes(rem_bytes, || reg.read());
}

#[inline(always)]
fn last_word(rem_bytes: &[u8]) -> Option<u32> {
    if rem_bytes.is_empty() {
        None
    } else {
        Some(u32::from_le_bytes([
            rem_bytes[0],
            rem_bytes.get(1).copied().unwrap_or_default(),
            rem_bytes.get(2).copied().unwrap_or_default(),
            0,
        ]))
    }
}

#[inline(always)]
fn set_rem_bytes(rem_bytes: &mut [u8], get_word: impl FnOnce() -> u32) {
    if rem_bytes.is_empty() {
        return;
    }
    let word = get_word();
    rem_bytes[0] = word as u8;
    if rem_bytes.len() == 1 {
        return;
    }
    rem_bytes[1] = (word >> 8) as u8;
    if rem_bytes.len() == 2 {
        return;
    }
    rem_bytes[2] = (word >> 16) as u8;
}

#[cfg(test)]
mod test {
    use super::*;

    use core::cell::RefCell;
    use core::mem::transmute_copy;
    use std::collections::HashMap;
    use std::collections::VecDeque;
    use std::rc::Rc;

    use ureg::Mmio;
    use ureg::MmioMut;
    use ureg::ReadWriteReg32;
    use ureg::RegRef;
    use ureg::UintType;

    fn uint_val<T: ureg::Uint>(val: T) -> u64 {
        // nosemgrep
        unsafe {
            // SAFETY: The underlying source type is the same as the destination type.
            match T::TYPE {
                ureg::UintType::U8 => core::mem::transmute_copy::<T, u8>(&val).into(),
                ureg::UintType::U16 => core::mem::transmute_copy::<T, u16>(&val).into(),
                ureg::UintType::U32 => core::mem::transmute_copy::<T, u32>(&val).into(),
                ureg::UintType::U64 => core::mem::transmute_copy::<T, u64>(&val),
            }
        }
    }

    #[derive(Clone, Default)]
    struct FakeMmio {
        fifos: Rc<RefCell<HashMap<usize, VecDeque<u32>>>>,
        write_log: Rc<RefCell<Vec<(usize, u64)>>>,
    }
    impl FakeMmio {
        fn fifo_push(&self, addr: usize, val: u32) {
            self.fifos
                .borrow_mut()
                .entry(addr)
                .or_default()
                .push_back(val);
        }
        fn take_log(&self) -> Vec<(usize, u64)> {
            core::mem::take(&mut *self.write_log.borrow_mut())
        }
        fn log(&self) -> Vec<(usize, u64)> {
            self.write_log.borrow().clone()
        }
    }
    impl Mmio for FakeMmio {
        unsafe fn read_volatile<T: ureg::Uint>(&self, src: *const T) -> T {
            let addr = src as usize;
            let Some(val) = self.fifos.borrow_mut().entry(addr).or_default().pop_front() else {
                panic!("Unexpected read from addr 0x{addr:x}")
            };
            if T::TYPE != UintType::U32 {
                panic!("Read must be of type u32");
            }
            // nosemgrep
            unsafe {
                // SAFETY: the type `T` is u32.
                transmute_copy::<u32, T>(&val)
            }
        }
    }
    impl MmioMut for FakeMmio {
        unsafe fn write_volatile<T: ureg::Uint>(&self, dst: *mut T, src: T) {
            self.write_log
                .borrow_mut()
                .push((dst as usize, uint_val(src)));
        }
    }

    #[test]
    #[should_panic(expected = "Unexpected read from addr 0x40404040")]
    pub fn test_fake_mmio_read_unexpected_addr() {
        let mmio = FakeMmio::default();
        // nosemgrep
        let fifo_reg = unsafe {
            // SAFETY: the backend is FakeMmio.
            RegRef::<ReadWriteReg32<0, u32, u32>, _>::new_with_mmio(0x4040_4040 as *mut _, &mmio)
        };
        fifo_reg.read();
    }

    #[test]
    #[should_panic(expected = "Read must be of type u32")]
    pub fn test_fake_mmio_read_unexpected_type() {
        let addr: usize = 0x4040;
        let mmio = FakeMmio::default();
        mmio.fifo_push(addr, 42);
        // nosemgrep
        unsafe {
            // SAFETY: the backend is FakeMmio.
            mmio.read_volatile(addr as *const u64)
        };
    }

    #[test]
    pub fn test_fake_mmio_read() {
        let addr = 0x4040_4040;
        let mmio = FakeMmio::default();
        // nosemgrep
        let fifo_reg = unsafe {
            // SAFETY: the backend is FakeMmio.
            RegRef::<ReadWriteReg32<0, u32, u32>, _>::new_with_mmio(addr as *mut _, &mmio)
        };
        mmio.fifo_push(addr, 0xba5e_ba11);
        mmio.fifo_push(addr, 0x1234_5678);
        assert_eq!(fifo_reg.read(), 0xba5e_ba11);
        assert_eq!(fifo_reg.read(), 0x1234_5678);
    }

    #[test]
    #[should_panic(expected = "Unexpected read from addr 0x40404040")]
    pub fn test_fake_mmio_read_fifo_exhausted() {
        let addr = 0x4040_4040;
        let mmio = FakeMmio::default();
        // nosemgrep
        let fifo_reg = unsafe {
            // SAFETY: the backend is FakeMmio.
            RegRef::<ReadWriteReg32<0, u32, u32>, _>::new_with_mmio(addr as *mut _, &mmio)
        };
        mmio.fifo_push(addr, 0xba5e_ba11);
        mmio.fifo_push(addr, 0x1234_5678);
        assert_eq!(fifo_reg.read(), 0xba5e_ba11);
        assert_eq!(fifo_reg.read(), 0x1234_5678);
        fifo_reg.read();
    }

    #[test]
    #[rustfmt::skip]
    pub fn test_fake_mmio_write() {
        let mmio = FakeMmio::default();
        // nosemgrep
        let fifo_reg = unsafe {
            // SAFETY: the backend is FakeMmio.
            RegRef::<ReadWriteReg32<0, u32, u32>, _>::new_with_mmio(0x4040_4040 as *mut _, &mmio)
        };
        // nosemgrep
        let fifo_reg2 = unsafe {
            // SAFETY: the backend is FakeMmio.
            RegRef::<ReadWriteReg32<0, u32, u32>, _>::new_with_mmio(0x5050_5050 as *mut _, &mmio)
        };

        assert_eq!(
            mmio.log(),
            vec![],
        );
        fifo_reg.write(|_| 0xba5e_ba11);
        assert_eq!(
            mmio.log(),
            vec![
                (0x4040_4040, 0xba5e_ba11),
            ],
        );
        fifo_reg.write(|_| 0xabba_abba);
        assert_eq!(
            mmio.log(),
            vec![
                (0x4040_4040, 0xba5e_ba11),
                (0x4040_4040, 0xabba_abba),
            ],
        );
        fifo_reg2.write(|_| 0x1234_5678);
        assert_eq!(
            mmio.log(),
            vec![
                (0x4040_4040, 0xba5e_ba11),
                (0x4040_4040, 0xabba_abba),
                (0x5050_5050, 0x1234_5678),
            ],
        );
    }

    #[test]
    #[rustfmt::skip]
    pub fn test_copy_to_reg_unaligned() {
        let mmio = FakeMmio::default();
        // nosemgrep
        let fifo_reg = unsafe {
            // SAFETY: the backend is FakeMmio.
            RegRef::<ReadWriteReg32<0, u32, u32>, &FakeMmio>::new_with_mmio(
                0x4040_4040 as *mut _,
                &mmio,
            )
        };

        copy_to_reg_unaligned(&fifo_reg, &[]);
        assert_eq!(
            mmio.take_log(),
            vec![],
        );
        copy_to_reg_unaligned(&fifo_reg, &[0x12]);
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x0000_0012)],
        );

        copy_to_reg_unaligned(&fifo_reg, &[0x12, 0x34]);
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x0000_3412)],
        );

        copy_to_reg_unaligned(&fifo_reg, &[0x12, 0x34, 0x56]);
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x0056_3412)],
        );
        copy_to_reg_unaligned(&fifo_reg, &[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x7856_3412)],
        );
        copy_to_reg_unaligned(&fifo_reg, &[0x12, 0x34, 0x56, 0x78, 0x9a]);
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4040, 0x0000_009a),
            ],
        );
        copy_to_reg_unaligned(&fifo_reg, &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]);
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4040, 0x0000_bc9a),
            ],
        );
        copy_to_reg_unaligned(&fifo_reg, &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]);
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4040, 0xf0de_bc9a),
            ],
        );
        copy_to_reg_unaligned(&fifo_reg, &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0xdd]);
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4040, 0xf0de_bc9a),
                (0x4040_4040, 0x0000_00dd),
            ],
        );
    }

    #[test]
    #[rustfmt::skip]
    pub fn test_copy_to_reg() {
        let mmio = FakeMmio::default();
        // nosemgrep
        let fifo_reg = unsafe {
            // SAFETY: the backend is FakeMmio.
            RegRef::<ReadWriteReg32<0, u32, u32>, &FakeMmio>::new_with_mmio(
                0x4040_4040 as *mut _,
                &mmio,
            )
        };

        copy_to_reg(&fifo_reg, &Aligned([]));
        assert_eq!(
            mmio.take_log(),
            vec![],
        );
        copy_to_reg(&fifo_reg, &Aligned([0x12]));
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x0000_0012)],
        );
        copy_to_reg(&fifo_reg, &Aligned([0x12, 0x34]));
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x0000_3412)],
        );
        copy_to_reg(&fifo_reg, &Aligned([0x12, 0x34, 0x56]));
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x0056_3412)],
        );
        copy_to_reg(&fifo_reg, &Aligned([0x12, 0x34, 0x56, 0x78]));
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x7856_3412)],
        );
        copy_to_reg(&fifo_reg, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9a]));
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4040, 0x0000_009a),
            ],
        );
        copy_to_reg(&fifo_reg, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]));
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4040, 0x0000_bc9a),
            ],
        );
        copy_to_reg(&fifo_reg, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]));
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4040, 0xf0de_bc9a),
            ],
        );
        copy_to_reg(&fifo_reg, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0xdd]));
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4040, 0xf0de_bc9a),
                (0x4040_4040, 0x0000_00dd),
            ],
        );
    }

    #[test]
    #[rustfmt::skip]
    pub fn test_copy_to_reg_array() {
        let mmio = FakeMmio::default();
        // nosemgrep
        let reg_array = unsafe {
            // SAFETY: the backend is FakeMmio.
            ureg::Array::<3, RegRef<ReadWriteReg32<0, u32, u32>, &FakeMmio>>::new_with_mmio(
                0x4040_4040 as *mut _,
                &mmio,
            )
        };

        copy_to_reg_array(&reg_array, &Aligned([]));
        assert_eq!(
            mmio.take_log(),
            vec![],
        );

        copy_to_reg_array(&reg_array, &Aligned([0x12]));
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x0000_0012)],
        );

        copy_to_reg_array(&reg_array, &Aligned([0x12, 0x34]));
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x0000_3412)],
        );

        copy_to_reg_array(&reg_array, &Aligned([0x12, 0x34, 0x56]));
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x0056_3412)],
        );

        copy_to_reg_array(&reg_array, &Aligned([0x12, 0x34, 0x56, 0x78]));
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x7856_3412)],
        );

        copy_to_reg_array(&reg_array, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9a]));
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4044, 0x0000_009a),
            ],
        );

        copy_to_reg_array(&reg_array, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]));
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4044, 0x0000_bc9a),
            ],
        );

        copy_to_reg_array(&reg_array, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]));
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4044, 0xf0de_bc9a),
            ],
        );

        copy_to_reg_array(&reg_array, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0xdd]));
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4044, 0xf0de_bc9a),
                (0x4040_4048, 0x0000_00dd),
            ],
        );
    }

    #[test]
    #[rustfmt::skip]
    pub fn test_copy_to_reg_array_unaligned() {
        let mmio = FakeMmio::default();
        // nosemgrep
        let reg_array = unsafe {
            // SAFETY: the backend is FakeMmio.
            ureg::Array::<3, RegRef<ReadWriteReg32<0, u32, u32>, &FakeMmio>>::new_with_mmio(
                0x4040_4040 as *mut _,
                &mmio,
            )
        };

        copy_to_reg_array_unaligned(&reg_array, &[]);
        assert_eq!(
            mmio.take_log(),
            vec![],
        );

        copy_to_reg_array_unaligned(&reg_array, &[0x12]);
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x0000_0012)],
        );

        copy_to_reg_array_unaligned(&reg_array, &[0x12, 0x34]);
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x0000_3412)],
        );

        copy_to_reg_array_unaligned(&reg_array, &[0x12, 0x34, 0x56]);
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x0056_3412)],
        );

        copy_to_reg_array_unaligned(&reg_array, &[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(
            mmio.take_log(),
            vec![(0x4040_4040, 0x7856_3412)],
        );

        copy_to_reg_array_unaligned(&reg_array, &[0x12, 0x34, 0x56, 0x78, 0x9a]);
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4044, 0x0000_009a),
            ],
        );

        copy_to_reg_array_unaligned(&reg_array, &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]);
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4044, 0x0000_bc9a),
            ],
        );

        copy_to_reg_array_unaligned(&reg_array, &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]);
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4044, 0xf0de_bc9a),
            ],
        );

        copy_to_reg_array_unaligned(&reg_array, &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0xdd]);
        assert_eq!(
            mmio.take_log(),
            vec![
                (0x4040_4040, 0x7856_3412),
                (0x4040_4044, 0xf0de_bc9a),
                (0x4040_4048, 0x0000_00dd),
            ],
        );
    }

    #[test]
    #[rustfmt::skip]
    pub fn test_copy_from_reg() {
        let mmio = FakeMmio::default();
        // nosemgrep
        let fifo_reg = unsafe {
            // SAFETY: the backend is FakeMmio.
            RegRef::<ReadWriteReg32<0, u32, u32>, &FakeMmio>::new_with_mmio(
                0x4040_4040 as *mut _,
                &mmio,
            )
        };

        copy_from_reg::<10>(&mut Aligned([]), &fifo_reg);
        assert_eq!(
            mmio.take_log(),
            vec![],
        );

        mmio.fifo_push(0x4040_4040, 0x0000_0012);
        let mut result = Aligned::<A4, _>([0_u8; 1]);
        copy_from_reg::<10>(&mut result, &fifo_reg);
        assert_eq!(&result, &Aligned([0x12]));

        mmio.fifo_push(0x4040_4040, 0x0000_3412);
        let mut result = Aligned::<A4, _>([0_u8; 2]);
        copy_from_reg::<10>(&mut result, &fifo_reg);
        assert_eq!(&result, &Aligned([0x12, 0x34]));

        mmio.fifo_push(0x4040_4040, 0x0056_3412);
        let mut result = Aligned::<A4, _>([0_u8; 3]);
        copy_from_reg::<10>(&mut result, &fifo_reg);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56]));

        mmio.fifo_push(0x4040_4040, 0x78563412);
        let mut result = Aligned::<A4, _>([0_u8; 4]);
        copy_from_reg::<10>(&mut result, &fifo_reg);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56, 0x78]));

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4040, 0x0000_009A);
        let mut result = Aligned::<A4, _>([0_u8; 5]);
        copy_from_reg::<10>(&mut result, &fifo_reg);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9A]));

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4040, 0x0000_BC9A);
        let mut result = Aligned::<A4, _>([0_u8; 6]);
        copy_from_reg::<10>(&mut result, &fifo_reg);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]));

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4040, 0x00DE_BC9A);
        let mut result = Aligned::<A4, _>([0_u8; 7]);
        copy_from_reg::<10>(&mut result, &fifo_reg);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE]));

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4040, 0xF0DE_BC9A);
        let mut result = Aligned::<A4, _>([0_u8; 8]);
        copy_from_reg::<10>(&mut result, &fifo_reg);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]));

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4040, 0xF0DE_BC9A);
        mmio.fifo_push(0x4040_4040, 0x0000_00DD);
        let mut result = Aligned::<A4, _>([0_u8; 9]);
        copy_from_reg::<10>(&mut result, &fifo_reg);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0xDD]));
    }

    #[test]
    #[rustfmt::skip]
    pub fn test_copy_from_reg_array() {
        let mmio = FakeMmio::default();
        // nosemgrep
        let reg_array = unsafe {
            // SAFETY: the backend is FakeMmio.
            ureg::Array::<3, RegRef<ReadWriteReg32<0, u32, u32>, &FakeMmio>>::new_with_mmio(
                0x4040_4040 as *mut _,
                &mmio,
            )
        };

        copy_from_reg_array(&mut Aligned([]), &reg_array);
        assert_eq!(
            mmio.take_log(),
            vec![],
        );

        mmio.fifo_push(0x4040_4040, 0x0000_0012);
        let mut result = Aligned::<A4, _>([0_u8; 1]);
        copy_from_reg_array(&mut result, &reg_array);
        assert_eq!(&result, &Aligned([0x12]));

        mmio.fifo_push(0x4040_4040, 0x0000_3412);
        let mut result = Aligned::<A4, _>([0_u8; 2]);
        copy_from_reg_array(&mut result, &reg_array);
        assert_eq!(&result, &Aligned([0x12, 0x34]));

        mmio.fifo_push(0x4040_4040, 0x0056_3412);
        let mut result = Aligned::<A4, _>([0_u8; 3]);
        copy_from_reg_array(&mut result, &reg_array);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56]));

        mmio.fifo_push(0x4040_4040, 0x78563412);
        let mut result = Aligned::<A4, _>([0_u8; 4]);
        copy_from_reg_array(&mut result, &reg_array);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56, 0x78]));

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4044, 0x0000_009A);
        let mut result = Aligned::<A4, _>([0_u8; 5]);
        copy_from_reg_array(&mut result, &reg_array);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9A]));

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4044, 0x0000_BC9A);
        let mut result = Aligned::<A4, _>([0_u8; 6]);
        copy_from_reg_array(&mut result, &reg_array);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]));

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4044, 0x00DE_BC9A);
        let mut result = Aligned::<A4, _>([0_u8; 7]);
        copy_from_reg_array(&mut result, &reg_array);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE]));

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4044, 0xF0DE_BC9A);
        let mut result = Aligned::<A4, _>([0_u8; 8]);
        copy_from_reg_array(&mut result, &reg_array);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]));

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4044, 0xF0DE_BC9A);
        mmio.fifo_push(0x4040_4048, 0x0000_00DD);
        let mut result = Aligned::<A4, _>([0_u8; 9]);
        copy_from_reg_array(&mut result, &reg_array);
        assert_eq!(&result, &Aligned([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0xDD]));
    }

    #[test]
    #[rustfmt::skip]
    pub fn test_copy_from_reg_array_unaligned() {
        let mmio = FakeMmio::default();
        // nosemgrep
        let reg_array = unsafe {
            // SAFETY: the backend is FakeMmio.
            ureg::Array::<3, RegRef<ReadWriteReg32<0, u32, u32>, &FakeMmio>>::new_with_mmio(
                0x4040_4040 as *mut _,
                &mmio,
            )
        };

        copy_from_reg_array_unaligned(&mut [], &reg_array);
        assert_eq!(
            mmio.take_log(),
            vec![],
        );

        mmio.fifo_push(0x4040_4040, 0x0000_0012);
        let mut result = [0_u8; 1];
        copy_from_reg_array_unaligned(&mut result, &reg_array);
        assert_eq!(result, [0x12]);

        mmio.fifo_push(0x4040_4040, 0x0000_3412);
        let mut result = [0_u8; 2];
        copy_from_reg_array_unaligned(&mut result, &reg_array);
        assert_eq!(result, [0x12, 0x34]);

        mmio.fifo_push(0x4040_4040, 0x0056_3412);
        let mut result = [0_u8; 3];
        copy_from_reg_array_unaligned(&mut result, &reg_array);
        assert_eq!(result, [0x12, 0x34, 0x56]);

        mmio.fifo_push(0x4040_4040, 0x78563412);
        let mut result = [0_u8; 4];
        copy_from_reg_array_unaligned(&mut result, &reg_array);
        assert_eq!(result, [0x12, 0x34, 0x56, 0x78]);

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4044, 0x0000_009A);
        let mut result = [0_u8; 5];
        copy_from_reg_array_unaligned(&mut result, &reg_array);
        assert_eq!(result, [0x12, 0x34, 0x56, 0x78, 0x9A]);

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4044, 0x0000_BC9A);
        let mut result = [0_u8; 6];
        copy_from_reg_array_unaligned(&mut result, &reg_array);
        assert_eq!(result, [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]);

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4044, 0x00DE_BC9A);
        let mut result = [0_u8; 7];
        copy_from_reg_array_unaligned(&mut result, &reg_array);
        assert_eq!(result, [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE]);

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4044, 0xF0DE_BC9A);
        let mut result = [0_u8; 8];
        copy_from_reg_array_unaligned(&mut result, &reg_array);
        assert_eq!(result, [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);

        mmio.fifo_push(0x4040_4040, 0x78563412);
        mmio.fifo_push(0x4040_4044, 0xF0DE_BC9A);
        mmio.fifo_push(0x4040_4048, 0x0000_00DD);
        let mut result = [0_u8; 9];
        copy_from_reg_array_unaligned(&mut result, &reg_array);
        assert_eq!(result, [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0xDD]);
    }

    #[test]
    pub fn test_copy_from_reg_unaligned() {
        let addr: usize = 0x4040_4040;
        let mmio = FakeMmio::default();
        // nosemgrep
        let fifo_reg = unsafe {
            // SAFETY: the backend is FakeMmio.
            RegRef::<ReadWriteReg32<0, u32, u32>, &FakeMmio>::new_with_mmio(addr as *mut _, &mmio)
        };
        copy_from_reg_unaligned(&mut [], &fifo_reg);

        mmio.fifo_push(addr, 0x7856_3412);
        let mut result = [0_u8; 1];
        copy_from_reg_unaligned(&mut result, &fifo_reg);
        assert_eq!(result, [0x12]);

        mmio.fifo_push(addr, 0x7856_3412);
        let mut result = [0_u8; 2];
        copy_from_reg_unaligned(&mut result, &fifo_reg);
        assert_eq!(result, [0x12, 0x34]);

        mmio.fifo_push(addr, 0x7856_3412);
        let mut result = [0_u8; 3];
        copy_from_reg_unaligned(&mut result, &fifo_reg);
        assert_eq!(result, [0x12, 0x34, 0x56]);

        mmio.fifo_push(addr, 0x7856_3412);
        let mut result = [0_u8; 4];
        copy_from_reg_unaligned(&mut result, &fifo_reg);
        assert_eq!(result, [0x12, 0x34, 0x56, 0x78]);

        mmio.fifo_push(addr, 0x7856_3412);
        mmio.fifo_push(addr, 0xf0de_bc9a);
        let mut result = [0_u8; 5];
        copy_from_reg_unaligned(&mut result, &fifo_reg);
        assert_eq!(result, [0x12, 0x34, 0x56, 0x78, 0x9a]);

        mmio.fifo_push(addr, 0x7856_3412);
        mmio.fifo_push(addr, 0xf0de_bc9a);
        let mut result = [0_u8; 6];
        copy_from_reg_unaligned(&mut result, &fifo_reg);
        assert_eq!(result, [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]);

        mmio.fifo_push(addr, 0x7856_3412);
        mmio.fifo_push(addr, 0xf0de_bc9a);
        let mut result = [0_u8; 7];
        copy_from_reg_unaligned(&mut result, &fifo_reg);
        assert_eq!(result, [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde]);

        mmio.fifo_push(addr, 0x7856_3412);
        mmio.fifo_push(addr, 0xf0de_bc9a);
        let mut result = [0_u8; 8];
        copy_from_reg_unaligned(&mut result, &fifo_reg);
        assert_eq!(result, [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]);

        mmio.fifo_push(addr, 0x7856_3412);
        mmio.fifo_push(addr, 0xf0de_bc9a);
        mmio.fifo_push(addr, 0x2c);
        let mut result = [0_u8; 9];
        copy_from_reg_unaligned(&mut result, &fifo_reg);
        assert_eq!(
            result,
            [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x2c]
        );
    }
}
