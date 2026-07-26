// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use earlgrey_gpio::EarlGreyGpio;
use earlgrey_pinout::swstraps::SwStraps;
use earlgrey_pinout::Pinout;
use pw_status::Result;
use userspace::entry;
use userspace::time::{sleep_until, Clock, Duration, SystemClock};
use util_panic as _;

const DELAY_10MS: Duration = Duration::from_millis(10);

fn sleep_10ms() {
    let _ = sleep_until(SystemClock::now() + DELAY_10MS);
}

fn run_swstraps_test() -> Result<()> {
    // SAFETY: EarlGreyGpio::new() initializes MMIO access to the GPIO and Pinmux peripherals;
    // safe in this single-threaded test environment.
    let mut gpio = unsafe { EarlGreyGpio::new() };
    SwStraps::configure(&mut gpio).map_err(|_| pw_status::Error::Internal)?;

    pw_log::info!("🔄 RUNNING SWSTRAPS TEST");

    let mut last_reported: Option<u32> = None;

    loop {
        let mut current =
            SwStraps::read_straps(&mut gpio).map_err(|_| pw_status::Error::Internal)?;

        if Some(current) != last_reported {
            // A change is observed (or initial read after boot).
            // Sleep 10ms and read again in a loop until the value is stable.
            loop {
                sleep_10ms();
                let next =
                    SwStraps::read_straps(&mut gpio).map_err(|_| pw_status::Error::Internal)?;
                if next == current {
                    break;
                }
                current = next;
            }

            if Some(current) != last_reported {
                pw_log::info!("SW_STRAP = {:#04x}", current);
                last_reported = Some(current);
            }
        }

        sleep_10ms();
    }
}

#[entry]
fn entry() -> Result<()> {
    run_swstraps_test()
}
