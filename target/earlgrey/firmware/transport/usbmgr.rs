// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! USB Manager Service.
//!
//! This service manages the USB interface for the OpenPRoT Earlgrey target.
//! It configures a composite USB device exposing:
//! 1. A CDC-ACM (Virtual COM Port) interface used for redirecting system logs.
//! 2. A DFU (Device Firmware Upgrade) interface for firmware updates and
//!    retrieving device certificates (UDS, CDI0, CDI1).
//!
//! The service runs in a loop, waiting for USB interrupts to drive the USB stack,
//! and logger events to forward log messages over the CDC-ACM interface.

#![no_std]
#![no_main]

use aligned::{Aligned, A4};
use pw_status::Error;
use usbmgr_codegen::{handle, signals};
use userspace::syscall::Signals;
use userspace::time::Instant;
use userspace::{process_entry, syscall};
use zerocopy::IntoBytes;

use hal_usb::driver::{UsbDriver, UsbEvent};
use hal_usb::{ConfigDescriptor, DeviceDescriptor, SetupPacket, StringDescriptorRef};
use usb_driver::UsbConfig;
use usb_stack::{DescriptorSource, UsbAction, UsbClass};

mod dfu;
use dfu::{EarlgreyDfuHandler, DFU_CDI0_CERT, DFU_CDI1_CERT, DFU_FIRMWARE, DFU_UDS_CERT};
use earlgrey_sysmgr_client::SysmgrClient;
use protocol_usb_cdc_acm::{CdcAcm, CdcAcmBuilder};
use protocol_usb_dfu::{DfuBuilder, DfuClass};
use services_flash_client::FlashIpcClient;
use util_ipc::IpcHandle;

use util_error::{AsStatus, ErrorCode};
use util_zfmt::messages::{ProcessExit, ProcessStart};
use util_zfmt::{render::render_event, FixedBuf};
use zfmt::Zfmt;

const USB_VENDOR_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(1);
const USB_PRODUCT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(2);
const USB_SERIAL_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(3);
const USB_CDC_COMM_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(4);
const USB_CDC_DATA_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(5);
const DFU_FIRMWARE_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(6);
const DFU_UDS_CERT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(7);
const DFU_CDI0_CERT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(8);
const DFU_CDI1_CERT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(9);

// The serial number size is 2 bytes (USB descriptor header) + 32 bytes of
// serial number * (2 for hex encoding) * (2 bytes per UTF16 character).
const USB_SERIAL_SIZE: usize = 2 + 32 * 2 * 2;

const DFU_BUILDER: DfuBuilder = DfuBuilder::new(
    2,    // interface_num (2, after CDC-ACM's 0 and 1)
    4,    // alt_settings
    2048, // transfer_size
);

const CDC_BUILDER: CdcAcmBuilder = CdcAcmBuilder::new(
    0, // comm_if: Communication Interface index
    1, // data_if: Data Interface index
    1, // comm_ep: Communication IN endpoint (Interrupt)
    2, // data_out_ep: Data OUT endpoint (Bulk)
    3, // data_in_ep: Data IN endpoint (Bulk)
);

const DEVICE_DESC: DeviceDescriptor = DeviceDescriptor {
    device_class: hal_usb::DeviceClass::SPECIFIED_BY_INTERFACE,
    device_sub_class: 0x00,
    device_protocol: 0x00,
    max_packet_size: 64,
    vendor_id: 0x18d1,  // Google, Inc.
    product_id: 0x503a, // STWG USB Fullspeed IP.
    device_release_num: 0x0100,
    manufacturer: USB_VENDOR_HANDLE,
    product: USB_PRODUCT_HANDLE,
    serial_num: USB_SERIAL_HANDLE,
};

const CONFIG_DESC: ConfigDescriptor = ConfigDescriptor {
    configuration_value: 1,
    max_power: 250,
    self_powered: false,
    remote_wakeup: false,
    interfaces: &[
        CDC_BUILDER.comm_interface(
            USB_CDC_COMM_HANDLE,
            &CDC_BUILDER.comm_func_descs(),
            &CDC_BUILDER.comm_endpoints(),
        ),
        CDC_BUILDER.data_interface(USB_CDC_DATA_HANDLE, &CDC_BUILDER.data_endpoints()),
        DFU_BUILDER.interface(0, DFU_FIRMWARE_HANDLE, &[]),
        DFU_BUILDER.interface(1, DFU_UDS_CERT_HANDLE, &[]),
        DFU_BUILDER.interface(2, DFU_CDI0_CERT_HANDLE, &[]),
        DFU_BUILDER.interface(
            3,
            DFU_CDI1_CERT_HANDLE,
            &[DFU_BUILDER.functional_descriptor()],
        ),
    ],
};

const STRING_DESC_0: hal_usb::StringDescriptor0 = hal_usb::StringDescriptor0 {
    langs: &[
        // English - United States
        0x0409,
    ],
};

const VENDOR_ID: hal_usb::StringDescriptorRef = hal_usb::string_descriptor!("Google Inc.").as_ref();
const PRODUCT_ID_DEFAULT: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("OpenPRoT Earlgrey").as_ref();
const USB_COMM: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("CDC Comm Interface").as_ref();
const USB_DATA: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("CDC Data Interface").as_ref();

/// Implements `DescriptorSource` to provide USB descriptors.
///
/// Dynamically generates the serial number descriptor using the chip's unique device ID.
struct MyDescriptors<'a> {
    serial_desc_bytes: StringDescriptorRef<'a>,
    product_desc_bytes: StringDescriptorRef<'a>,
}

impl DescriptorSource for MyDescriptors<'_> {
    const DEVICE_DESC_BYTES: &'static Aligned<A4, [u8]> = &Aligned(DEVICE_DESC.serialize());
    const CONFIG_DESC_BYTES: &'static Aligned<A4, [u8]> =
        &Aligned(CONFIG_DESC.serialize::<{ CONFIG_DESC.total_size() }>());
    const STRING_DESC_0_BYTES: &'static Aligned<A4, [u8]> =
        &Aligned(STRING_DESC_0.serialize::<{ STRING_DESC_0.total_size() }>());
    const DEVICE_STATUS: Aligned<A4, [u8; 2]> = Aligned([1u8, 0]);

    fn get_string(
        &self,
        handle: hal_usb::StringHandle,
        _lang: u16,
    ) -> Option<hal_usb::StringDescriptorRef<'_>> {
        let h = handle.0;
        if h == USB_VENDOR_HANDLE.0 {
            Some(VENDOR_ID)
        } else if h == USB_PRODUCT_HANDLE.0 {
            Some(self.product_desc_bytes)
        } else if h == USB_SERIAL_HANDLE.0 {
            Some(self.serial_desc_bytes)
        } else if h == USB_CDC_COMM_HANDLE.0 {
            Some(USB_COMM)
        } else if h == USB_CDC_DATA_HANDLE.0 {
            Some(USB_DATA)
        } else if h == DFU_FIRMWARE_HANDLE.0 {
            Some(DFU_FIRMWARE)
        } else if h == DFU_UDS_CERT_HANDLE.0 {
            Some(DFU_UDS_CERT)
        } else if h == DFU_CDI0_CERT_HANDLE.0 {
            Some(DFU_CDI0_CERT)
        } else if h == DFU_CDI1_CERT_HANDLE.0 {
            Some(DFU_CDI1_CERT)
        } else {
            None
        }
    }
}

/// Helper struct to format USB Setup packets for zfmt logging.
#[derive(Zfmt)]
#[zfmt(
    format = "UsbSetup: {bmreq:02x} {request:02x} val={value:04x} idx={index:04x} len={length:04x}"
)]
pub struct UsbSetup {
    pub bmreq: u8,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub length: u16,
}

impl From<SetupPacket> for UsbSetup {
    fn from(pkt: SetupPacket) -> Self {
        let dir = pkt.request().direction() as u32;
        let ty = pkt.request().request_type() as u32;
        let recip = pkt.request().recipient() as u32;
        let bmreq = (dir << 7) | (ty << 5) | (recip);
        let request = pkt.request().request();
        UsbSetup {
            bmreq: bmreq as u8,
            request,
            value: pkt.value(),
            index: pkt.index(),
            length: pkt.length(),
        }
    }
}

/// Main USB event loop.
///
/// This function:
/// 1. Retrieves boot info to obtain the unique chip ID for the serial number.
/// 2. Initializes the USB driver, DFU class, and CDC-ACM class.
/// 3. Registers interrupts and logger signals with a wait group.
/// 4. Enters the main loop:
///    - Handles USB interrupt signals and dispatches events to CDC-ACM, DFU, or Ep0.
///    - Handles logger signals and forwards pending log messages to the CDC-ACM TX queue.
///    - Drives CDC-ACM transmission and DFU state machine.
fn handle_usb() -> Result<(), ErrorCode> {
    let mut log_events_pending = false;
    let mut log_cursor = 0u64;
    let mut event = [0u8; 256];

    // Get the boot info (includes device_id) so we can create the
    // serial number string descriptor.
    let sysmgr = SysmgrClient::new(IpcHandle::new(handle::SYSMGR_USB));
    let boot_info = sysmgr.get_boot_info()?;

    let mut serial_num_buffer = Aligned::<A4, _>([0_u8; USB_SERIAL_SIZE]);
    let descriptors = MyDescriptors {
        serial_desc_bytes: hal_usb::hex_utf16_descriptor_aligned(
            &mut serial_num_buffer,
            boot_info.chip.device_id.as_bytes(),
        )
        .unwrap(),
        product_desc_bytes: PRODUCT_ID_DEFAULT,
    };

    const USB_CONFIG: UsbConfig = UsbConfig::new(&CDC_BUILDER.eps().0, &CDC_BUILDER.eps().1);

    let flash = FlashIpcClient::new(IpcHandle::new(handle::FLASH_USB))?;
    let _spi_flash = FlashIpcClient::new(IpcHandle::new(handle::SPI_FLASH_USB))?;
    let dfu_handler = EarlgreyDfuHandler::new(flash, sysmgr, &boot_info)?;
    let mut dfu = DfuClass::<_, 2048>::new(DFU_BUILDER, dfu_handler);

    let mut usb = usb_driver::Usb::new(unsafe { usbdev::Usbdev::new() }, USB_CONFIG);
    let mut ep0 = usb_stack::SimpleEp0::new();
    let mut cdc_acm = CdcAcm::<256, 256>::new(CDC_BUILDER);

    syscall::wait_group_add(
        handle::USB_WAIT_GROUP,
        handle::LOGGER_USB,
        Signals::USER,
        handle::LOGGER_USB as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    let usb_intr_mask = signals::USBDEV_PKT_RECEIVED
        | signals::USBDEV_PKT_SENT
        | signals::USBDEV_DISCONNECTED
        | signals::USBDEV_HOST_LOST
        | signals::USBDEV_LINK_RESET
        | signals::USBDEV_LINK_SUSPEND
        | signals::USBDEV_LINK_RESUME
        | signals::USBDEV_AV_OUT_EMPTY
        | signals::USBDEV_RX_FULL
        | signals::USBDEV_AV_OVERFLOW
        | signals::USBDEV_AV_SETUP_EMPTY;

    syscall::wait_group_add(
        handle::USB_WAIT_GROUP,
        handle::USBDEV_INTERRUPTS,
        usb_intr_mask,
        handle::USBDEV_INTERRUPTS as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    loop {
        let wait_return =
            syscall::object_wait(handle::USB_WAIT_GROUP, Signals::READABLE, Instant::MAX)
                .map_err(ErrorCode::kernel_error)?;

        let wakeup = wait_return.user_data as u32;

        if wakeup == handle::USBDEV_INTERRUPTS {
            while let Some(event) = usb.poll() {
                if let UsbEvent::SetupPacket { pkt, .. } = event {
                    util_zfmt::debug!(UsbSetup::from(pkt));
                }
                let mut action = match cdc_acm.handle_event(event) {
                    Ok(a) => a,
                    Err(event) => match dfu.handle_event(event) {
                        Ok(a) => a,
                        Err(e) => ep0.handle_event(e, &descriptors).unwrap_or(UsbAction::None),
                    },
                };
                action.run(&mut usb);
            }
            let _ = syscall::interrupt_ack(handle::USBDEV_INTERRUPTS, wait_return.pending_signals);
        } else if wakeup == handle::LOGGER_USB {
            // If we got a wakeup signal from the logger task, ack it and note that we have events
            // pending.
            util_zfmt::logger().clear_notifier()?;
            log_events_pending = true;
        }

        // TODO: this just echos CDC-ACM input back into the output.
        // Decide what to do with input.
        while let Some(byte) = cdc_acm.rx_queue.pop() {
            let _ = cdc_acm.tx_queue.push(byte);
        }

        // If the CDC-ACM queue is empty and if we have more log events,
        // process them.
        if log_events_pending && cdc_acm.tx_queue.is_empty() {
            let (cursor, ev) = util_zfmt::logger().get_event(log_cursor, &mut event)?;
            log_cursor = cursor;
            if ev.is_empty() {
                // No more events.
                log_events_pending = false;
            } else {
                // Render to text and advance the cursor.
                let mut buf = FixedBuf::<254>::new();
                if let Some(len) = render_event(ev, &mut buf) {
                    let _ = cdc_acm.tx_queue.push_slice(buf.as_slice());
                    let _ = cdc_acm.tx_queue.push_slice(b"\r\n");
                    log_cursor += len as u64;
                }
            }
        }

        // Drive the CDC-ACM transmitter.
        cdc_acm.poll_transmit(&mut usb);
        dfu.poll(&mut usb);
    }
}

/// Configures pinmux for the USB device.
///
/// Currently configures USB sense (VBUS detect) to constant high.
fn usb_setup_pinmux() {
    // TODO: move pinmux setup into the platform task.
    use top_earlgrey::{PinmuxInsel, PinmuxPeripheralIn};
    let mut pinmux = unsafe { pinmux::PinmuxAon::new() };

    pinmux
        .regs_mut()
        .mio_periph_insel()
        .at(PinmuxPeripheralIn::UsbdevSense as usize)
        .modify(|_| (PinmuxInsel::ConstantOne as u32).into());
}

/// USB manager server entry point.
fn usbmgr_server() -> Result<(), ErrorCode> {
    usb_setup_pinmux();
    handle_usb()
}

/// Process entry point for the `usbmgr` task.
#[process_entry("usbmgr")]
fn entry() -> Result<(), Error> {
    util_zfmt::info!(ProcessStart { name: "usbmgr" });
    let ret = usbmgr_server();
    util_zfmt::error!(ProcessExit {
        name: "usbmgr",
        status: ret.as_status()
    });

    Err(Error::Unknown)
}
