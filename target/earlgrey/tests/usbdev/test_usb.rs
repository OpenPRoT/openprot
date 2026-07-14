// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
use earlgrey_util::device_id::format_device_id;
use lc_ctrl::LcCtrl;
use pw_status::{Error, Result, StatusCode};
use zerocopy::IntoBytes;

use test_usb_codegen::{handle, signals};
use userspace::time::Instant;
use userspace::{entry, syscall};

use aligned::{Aligned, A4};
use hal_usb::driver::{UsbDriver, UsbEvent, UsbPacket};
use hal_usb::{Direction, StringDescriptorRef, USB_CLASS_VENDOR};
use usb_driver::{EpIn, EpOut, UsbConfig};
use usb_stack::{
    //UsbActionRun,
    DescriptorSource,
    UsbAction,
};

const USB_VENDOR_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(1);
const USB_PRODUCT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(2);
const USB_SERIAL_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(3);
const USB_TEST_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(4);

static DEVICE_DESC: hal_usb::DeviceDescriptor = hal_usb::DeviceDescriptor {
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
const CONFIG_DESC: hal_usb::ConfigDescriptor = hal_usb::ConfigDescriptor {
    configuration_value: 1,
    max_power: 250,
    self_powered: false,
    remote_wakeup: false,
    interfaces: &[hal_usb::InterfaceDescriptor {
        name: USB_TEST_HANDLE,
        interface_number: 1,
        alternate_setting: 0,
        interface_class: USB_CLASS_VENDOR,
        interface_sub_class: 0xFF,
        interface_protocol: 1,
        func_descs: &[],
        endpoints: &[
            hal_usb::EndpointDescriptor {
                direction: Direction::DeviceToHost,
                endpoint_num: 1,
                interval: 0,
                max_packet_size: 64,
                transfer_type: hal_usb::TransferType::Bulk,
            },
            hal_usb::EndpointDescriptor {
                direction: Direction::HostToDevice,
                endpoint_num: 2,
                interval: 0,
                max_packet_size: 64,
                transfer_type: hal_usb::TransferType::Bulk,
            },
        ],
    }],
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
const USB_TEST: hal_usb::StringDescriptorRef =
    hal_usb::string_descriptor!("USB Test Interface").as_ref();

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
    // We advertise that we are self-powered and do not support remote wakeup.
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
            USB_TEST_HANDLE => Some(USB_TEST),
            _ => None,
        }
    }
}

const CONTROL_EP_OUT_NUM: u8 = 0;

fn handle_usb() -> Result<()> {
    let mut serial_num_buffer = Aligned::<A4, _>([0_u8; 130]);

    let lc_ctrl = unsafe { LcCtrl::new() };
    let device_id: [u32; 8] = lc_ctrl.regs().device_id().read().into();

    let mut serial_ascii = [0u8; 64];
    let serial_str = format_device_id(&device_id, &mut serial_ascii).ok_or(Error::Internal)?;
    pw_log::info!("Serial Number: {}", serial_str);

    let device_id_bytes = device_id.as_bytes();

    let descriptors = MyDescriptors {
        serial_desc_bytes: hal_usb::hex_utf16_descriptor_aligned(
            &mut serial_num_buffer,
            device_id_bytes,
        )
        .map_err(|_| Error::Internal)?,
        product_desc_bytes: PRODUCT_ID_DEFAULT,
    };
    const USB_EP_IN: EpIn = EpIn {
        num: 1,
        buf_pool_size: 8,
    };
    const USB_EP_OUT: EpOut = EpOut {
        num: 2,
        set_nak: true,
    };

    const USB_CONFIG: UsbConfig = UsbConfig::new(&[USB_EP_IN], &[USB_EP_OUT]);
    let mut usb = usb_driver::Usb::new(unsafe { usbdev::Usbdev::new() }, USB_CONFIG);
    let mut ep0 = usb_stack::SimpleEp0::new();
    let mut ep0_action: UsbAction<'_> = UsbAction::None;

    loop {
        let wait_return = syscall::object_wait(
            handle::USBDEV_INTERRUPTS,
            signals::USBDEV_PKT_RECEIVED
                | signals::USBDEV_PKT_SENT
                | signals::USBDEV_DISCONNECTED
                | signals::USBDEV_HOST_LOST
                | signals::USBDEV_LINK_RESET
                | signals::USBDEV_LINK_SUSPEND
                | signals::USBDEV_LINK_RESUME
                | signals::USBDEV_AV_OUT_EMPTY
                | signals::USBDEV_RX_FULL
                | signals::USBDEV_AV_OVERFLOW
                //| signals::USBDEV_LINK_IN_ERR
                | signals::USBDEV_RX_CRC_ERR
                | signals::USBDEV_RX_PID_ERR
                | signals::USBDEV_RX_BITSTUFF_ERR
                | signals::USBDEV_FRAME
                //| signals::USBDEV_POWERED
                //| signals::USBDEV_LINK_OUT_ERR
                | signals::USBDEV_AV_SETUP_EMPTY,
            Instant::MAX,
        )?;

        if wait_return.user_data != 0 {
            pw_log::error!("Incorrect WaitReturn values");
            return Err(Error::Unknown);
        }
        while let Some(event) = usb.poll() {
            match event {
                UsbEvent::SetupPacket { pkt, endpoint } => {
                    if endpoint == 0 {
                        let dir = pkt.request().direction() as u32;
                        let ty = pkt.request().request_type() as u32;
                        let recip = pkt.request().recipient() as u32;
                        let bmreq = (dir << 7) | (ty << 5) | (recip);
                        let request = pkt.request().request();
                        pw_log::debug!(
                            "SETUP: {:02x} {:02x} val={:04x} idx={:04x} len={:04x}",
                            bmreq as u32,
                            request as u8,
                            pkt.value() as u16,
                            pkt.index() as u16,
                            pkt.length() as u16,
                        );

                        ep0_action = ep0
                            .handle_event(event, &descriptors)
                            .unwrap_or(UsbAction::None);
                    } else {
                        pw_log::debug!("Setup on bad EP {}", endpoint as u8);
                    }
                }

                UsbEvent::DataOutPacket(pkt) => match u8::try_from(pkt.endpoint_index()).unwrap() {
                    CONTROL_EP_OUT_NUM => {
                        pw_log::debug!("OUT on control ep");
                    }
                    ep => {
                        pw_log::debug!(
                            "Unhandled OUT on EP {} len={}",
                            ep as u8,
                            pkt.len() as usize
                        );
                    }
                },
                UsbEvent::UsbReset => {
                    pw_log::debug!("USB reset");
                }
                _ => {
                    ep0_action.merge(
                        ep0.handle_event(event, &descriptors)
                            .unwrap_or(UsbAction::None),
                    );
                }
            }
            ep0_action.run(&mut usb);
        }
        let _ = syscall::interrupt_ack(handle::USBDEV_INTERRUPTS, wait_return.pending_signals);
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

#[entry]
fn entry() -> Result<()> {
    pw_log::info!("🔄 RUNNING");
    usb_setup_pinmux();
    let ret = handle_usb();

    if ret.is_err() {
        pw_log::error!("❌ FAIL: {}", ret.status_code() as u32);
    } else {
        pw_log::info!("✅ PASS");
    }

    ret
}

util_panic::make_panic_handler!();
