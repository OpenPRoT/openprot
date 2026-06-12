// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use aligned::{Aligned, A4};
use pw_status::Error;
use usbmgr_codegen::{handle, signals};
use userspace::syscall::Signals;
use userspace::time::Instant;
use userspace::{process_entry, syscall};

use hal_usb::driver::UsbDriver;
use hal_usb::driver::UsbEvent;
use hal_usb::{ConfigDescriptor, DeviceDescriptor, StringDescriptorRef};
use usb_driver::UsbConfig;
use usb_stack::{DescriptorSource, UsbAction, UsbClass};

use protocol_usb_cdc_acm::{CdcAcm, CdcAcmBuilder};

use util_error::ErrorCode;
use util_zfmt::{render::render_event, FixedBuf};
use zfmt::Zfmt;

const USB_VENDOR_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(1);
const USB_PRODUCT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(2);
const USB_SERIAL_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(3);
const USB_CDC_COMM_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(4);
const USB_CDC_DATA_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(5);

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
        match handle {
            USB_VENDOR_HANDLE => Some(VENDOR_ID),
            USB_PRODUCT_HANDLE => Some(self.product_desc_bytes),
            USB_SERIAL_HANDLE => Some(self.serial_desc_bytes),
            USB_CDC_COMM_HANDLE => Some(USB_COMM),
            USB_CDC_DATA_HANDLE => Some(USB_DATA),
            _ => None,
        }
    }
}

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

fn handle_usb() -> Result<(), ErrorCode> {
    let mut log_events_pending = false;
    let mut log_cursor = 0u64;
    let mut event = [0u8; 256];
    let mut serial_num_buffer = Aligned::<A4, _>([0_u8; 130]);
    let descriptors = MyDescriptors {
        serial_desc_bytes: hal_usb::hex_utf16_descriptor_aligned(
            &mut serial_num_buffer,
            b"12345678",
        )
        .unwrap(),
        product_desc_bytes: PRODUCT_ID_DEFAULT,
    };

    const USB_CONFIG: UsbConfig = UsbConfig::new(&CDC_BUILDER.eps().0, &CDC_BUILDER.eps().1);

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
        | signals::USBDEV_POWERED
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
                    let dir = pkt.request().direction() as u32;
                    let ty = pkt.request().request_type() as u32;
                    let recip = pkt.request().recipient() as u32;
                    let bmreq = (dir << 7) | (ty << 5) | (recip);
                    let request = pkt.request().request();
                    util_zfmt::debug!(UsbSetup {
                        bmreq: bmreq as u8,
                        request,
                        value: pkt.value(),
                        index: pkt.index(),
                        length: pkt.length(),
                    });
                }
                let mut action = match cdc_acm.handle_event(event) {
                    Ok(a) => a,
                    Err(e) => ep0.handle_event(e, &descriptors).unwrap_or(UsbAction::None),
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
    }
}

fn usb_setup_pinmux() {
    use top_earlgrey::{PinmuxInsel, PinmuxPeripheralIn};
    let mut pinmux = unsafe { pinmux::PinmuxAon::new() };

    pinmux
        .regs_mut()
        .mio_periph_insel()
        .at(PinmuxPeripheralIn::UsbdevSense as usize)
        .modify(|_| (PinmuxInsel::ConstantOne as u32).into());
}

#[process_entry("usbmgr")]
fn entry() -> Result<(), Error> {
    pw_log::info!("🔄 RUNNING");
    usb_setup_pinmux();
    match handle_usb() {
        Ok(()) => Ok(()),
        Err(e) => {
            let e = u32::from(e);
            pw_log::error!("usbmgr FAIL: {}", e as u32);
            // TODO: make an appropiate conversion function.
            Err(Error::try_from(e & 0x1f)?)
        }
    }
}
