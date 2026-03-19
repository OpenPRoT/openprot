// Licensed under the Apache-2.0 license

//! I2C Slave Echo Application
//!
//! Sits on I2C2 at address 0x42 and implements a simple register model:
//!
//! - **Write** `[reg, value]` — stores `value` at `reg` and prints the access.
//! - **Write** `[reg]` followed by **Read** — returns the stored value for `reg`
//!   and prints the access.
//!
//! # Protocol
//!
//! ```text
//! Master writes [reg, val]  →  slave stores reg_map[reg] = val
//!                              prints: "WRITE reg=0xRR val=0xVV"
//!
//! Master writes [reg]       →  slave sets read pointer to reg
//! Master reads N bytes      →  slave returns reg_map[reg..reg+N]
//!                              prints: "READ  reg=0xRR val=0xVV"
//! ```
//!
//! # Hardware
//!
//! The AST1060 I2C2 controller is used in simultaneous master+slave mode:
//! master operations use the I2CM* register set, slave operations use I2CS*.

#![no_main]
#![no_std]

use app_i2c_slave_echo::handle;
use i2c_api::{BusIndex, I2cAddress, I2cTargetClient, SlaveEventKind};
use i2c_client::IpcI2cClient;
use pw_status::Result;
use userspace::entry;

/// I2C bus index (I2C2).
const SLAVE_BUS: BusIndex = BusIndex::BUS_2;

/// Slave address.
const SLAVE_ADDR: u8 = 0x42;

/// Number of registers in the echo register map.
const REG_MAP_SIZE: usize = 256;

fn slave_echo_loop() -> Result<()> {
    let mut client = IpcI2cClient::new(handle::I2C);

    let addr = I2cAddress::new(SLAVE_ADDR).map_err(|_| pw_status::Error::InvalidArgument)?;

    // Configure and enable slave mode.
    client
        .configure_target_address(SLAVE_BUS, addr)
        .map_err(|_| pw_status::Error::Internal)?;
    client
        .enable_receive(SLAVE_BUS)
        .map_err(|_| pw_status::Error::Internal)?;

    pw_log::info!(
        "I2C slave echo running on bus {} at address 0x{:02x}",
        SLAVE_BUS.value() as u32,
        SLAVE_ADDR as u32,
    );

    // Register map — all bytes initialised to zero.
    let mut reg_map = [0u8; REG_MAP_SIZE];
    // Current register pointer (set by a 1-byte write, used for reads).
    let mut reg_ptr: u8 = 0;

    // Pre-load the initial read response.
    if let Err(_) = client.slave_set_response(SLAVE_BUS, &[reg_map[reg_ptr as usize]]) {
        pw_log::error!("Initial slave_set_response failed");
    }

    let mut rx_buf = [0u8; 32];

    loop {
        match client.slave_wait_event(SLAVE_BUS, &mut rx_buf) {
            Ok((SlaveEventKind::DataReceived, n)) => {
                match n {
                    0 => {
                        // Zero-length write — ignore (e.g. probe).
                    }
                    1 => {
                        // Register pointer set; update read response.
                        reg_ptr = rx_buf[0];
                        let val = reg_map[reg_ptr as usize];
                         pw_log::info!(
                             "READ  reg=0x{:02x} val=0x{:02x}",
                             reg_ptr as u32,
                             val as u32,
                         );
                        if let Err(_) = client.slave_set_response(SLAVE_BUS, &[val]) {
                            pw_log::error!("slave_set_response failed");
                        }
                    }
                    _ => {
                        // Write: byte 0 = register address, byte 1 = value.
                        reg_ptr = rx_buf[0];
                        let val = rx_buf[1];
                        reg_map[reg_ptr as usize] = val;
                         pw_log::info!(
                             "WRITE reg=0x{:02x} val=0x{:02x}",
                             reg_ptr as u32,
                             val as u32,
                         );
                         //Update read response in case master reads back immediately.
                        if let Err(_) = client.slave_set_response(SLAVE_BUS, &[val]) {
                            pw_log::error!("slave_set_response failed");
                        }
                    }
                }
            }
            Ok((SlaveEventKind::ReadRequest, _)) => {
                // Master read our pre-loaded response; log it.
                let val = reg_map[reg_ptr as usize];
                // pw_log::info!(
                //     "READ  reg=0x{:02x} val=0x{:02x}",
                //     reg_ptr as u32,
                //     val as u32,
                // );
                let _ = val;
            }
            Ok((SlaveEventKind::Stop, _)) => {
                // Stop condition — nothing to do.
            }
            Err(_) => {
                // Timeout or error — keep looping.
            }
        }
    }
}

#[entry]
fn entry() -> ! {
    pw_log::info!("I2C slave echo starting");
    let _ = slave_echo_loop();
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
