# `regcpy`

The `regcpy` crate provides utility functions for copying byte slices
between memory and MMIO register spaces.  These functions are helpful
in dealing with registers that represent FIFOs or memory buffers within
a peripheral.  Regardless of the alignment requirements of the source
or destination byte-slice, these functions permit a slice of arbitrary
length.

The following functions are provided:

- `copy_to_reg`: Copy an aligned byte-slice to a single register.
- `copy_to_reg_array`: Copy an aligned byte-slice to a register array.
- `copy_to_reg_unaligned`: Copy an unaligned byte slice to a single register.
- `copy_to_reg_array_unaligned`: Copy an unaligned byte slice to a register array.
- `copy_from_reg`: Copy from a register to an aligned byte slice.
- `copy_from_reg_array`: Copy from a register array to an aligned byte slice.
- `copy_from_reg_unaligned`: Copy from a register to an unaligned byte slice.
- `copy_from_reg_array_unaligned`: Copy from a register array to an unaligned byte slice.
