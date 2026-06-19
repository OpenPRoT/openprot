// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

#[macro_export]
macro_rules! make_panic_handler {
    () => {
        #[panic_handler]
        fn panic_handler(info: &core::panic::PanicInfo) -> ! {
            if let Some(location) = info.location() {
                util_panic::panic_is_possible(
                    location.file().as_ptr(),
                    location.file().len(),
                    location.line(),
                    location.column(),
                );
            } else {
                util_panic::panic_is_possible(core::ptr::null(), 0, 0, 0);
            }
        }
    };
}

// This panic_is_possible function is the hook used by the panic detector
// to identify the presence a panic handler.
#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn panic_is_possible(
    filename: *const u8,
    filename_len: usize,
    line: u32,
    col: u32,
) -> ! {
    // The arguments to this function are reverse-engineered
    // from the machine code by static analysis to
    // display a list of all the panic call-sites to the
    // user in the rust_binary_no_panics_test error message.
    // See pw_kernel/tooling/panic_detector/check_panic.rs for more details.

    // If this symbol exists in the binary, panics are
    // possible. Presubmit tests can ensure that this symbol
    // does not exist in the final binary.  Do not rename or
    // remove this function.
    core::hint::black_box(filename);
    core::hint::black_box(filename_len);
    core::hint::black_box(line);
    core::hint::black_box(col);

    #[expect(clippy::empty_loop)]
    loop {}
}
