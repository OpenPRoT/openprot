// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_main]
#![no_std]

use i2c_api::SlaveEvent;
use i2c_client::I2cClient;
use i2c_client_ipc::IpcTransport;
use mctp_lib::config;
use openprot_mctp_server::Server;
use openprot_mctp_transport_i2c::{I2cSender, MctpI2cReceiver};

use mctp_server_runtime::Channel;
use pw_status::Error;
use pw_status::Result;
use userspace::entry;
use userspace::syscall::{self, Signals};

use app_mctp_server_peer::handle;

const OWN_EID: u8 = 9;
const OWN_I2C_ADDR: u8 = 0x42;
const REMOTE_I2C_ADDR: u8 = 0x10;
const I2C_RX_MAX: usize = config::MAX_PAYLOAD;

fn mctp_server_loop() -> Result<()> {
    pw_log::info!("MCTP server peer starting");
    let sender = I2cSender::new(
        I2cClient::new(IpcTransport::new(handle::I2C)),
        OWN_I2C_ADDR,
        REMOTE_I2C_ADDR,
    );
    let mut i2c_rx_client = I2cClient::new(IpcTransport::new(handle::I2C));
    let i2c_receiver = MctpI2cReceiver::new(OWN_I2C_ADDR);

    if i2c_rx_client.configure_slave(OWN_I2C_ADDR).is_err() {
        pw_log::error!("configure_slave failed");
        return Err(Error::Internal);
    }
    if i2c_rx_client.enable_slave().is_err() {
        pw_log::error!("enable_slave failed");
        return Err(Error::Internal);
    }
    if i2c_rx_client.enable_notification().is_err() {
        pw_log::error!("enable_notification failed");
        return Err(Error::Internal);
    }

    let mut server = Server::<_, 16>::new(mctp::Eid(OWN_EID), 0, sender);
    let mut i2c_rx_buf = [0u8; I2C_RX_MAX];
    let mut channels = [Channel::new(handle::MCTP)];

    mctp_server_runtime::run(
        handle::WG,
        &mut channels,
        handle::I2C,
        Signals::USER,
        &mut server,
        |server| {
            match i2c_rx_client.slave_receive(&mut i2c_rx_buf) {
                Ok(event) => {
                    if event.kind == SlaveEvent::DataReceived && event.data_len > 0 {
                        if let Ok((pkt, _)) = i2c_receiver.decode(&i2c_rx_buf[..event.data_len]) {
                            let _ = server.inbound(pkt);
                        } else {
                            pw_log::error!("i2c frame decode failed");
                        }
                    }
                }
                Err(_) => {
                    pw_log::error!("slave_receive failed");
                }
            }
        },
    )
}

#[entry]
fn entry() {
    if let Err(e) = mctp_server_loop() {
        pw_log::error!("mctp_server peer exiting with error");
        let _ = syscall::process_exit(e as u32);
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

