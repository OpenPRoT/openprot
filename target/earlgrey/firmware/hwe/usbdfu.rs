// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
#![allow(dead_code)]

use pw_status::Error;
use usbdfu_codegen::{handle, signals};
use userspace::time::Instant;
use userspace::{entry, syscall};

use aligned::{Aligned, A4};
use hal_usb::{ConfigDescriptor, DeviceDescriptor, StringDescriptorRef};

use hal_usb::driver::UsbDriver;
use usb_driver::UsbConfig;
use usb_stack::{DescriptorSource, UsbAction, UsbClass};

use earlgrey_util::error as eg_error;
use earlgrey_util::PersoCertificate;
use earlgrey_util::tags::{BootSlot};
use earlgrey_util::tags::ManifestIdentifier;
use earlgrey_sysmgr_client::{SysmgrClient, BootInfo};
use hal_flash::{Flash, FlashAddress};
use services_flash_client::FlashIpcClient;
use util_error::{self as error, ErrorCode};
use util_ipc::IpcChannel;
use zerocopy::{FromBytes};

use protocol_usb_dfu::{DfuBuilder, DfuClass, DfuHandler, DfuResult, DfuStatus};

const USB_VENDOR_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(1);
const USB_PRODUCT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(2);
const USB_SERIAL_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(3);
const DFU_FIRMWARE_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(4);
const DFU_UDS_CERT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(5);
const DFU_CDI0_CERT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(6);
const DFU_CDI1_CERT_HANDLE: hal_usb::StringHandle = hal_usb::StringHandle(7);

const DFU_BUILDER: DfuBuilder = DfuBuilder::new(
    0,    // interface_num
    1,    // alt_settings (1 for now)
    2048, // transfer_size
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
        DFU_BUILDER.interface( 0, DFU_FIRMWARE_HANDLE, &[]),
        DFU_BUILDER.interface( 1, DFU_UDS_CERT_HANDLE, &[]),
        DFU_BUILDER.interface( 2, DFU_CDI0_CERT_HANDLE, &[]),
        DFU_BUILDER.interface( 3, DFU_CDI1_CERT_HANDLE,
            &[DFU_BUILDER.functional_descriptor()]),
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
    hal_usb::string_descriptor!("Earlgrey DFU").as_ref();
const DFU_FIRMWARE: hal_usb::StringDescriptorRef = hal_usb::string_descriptor!("Firmware").as_ref();
const DFU_UDS_CERT: hal_usb::StringDescriptorRef = hal_usb::string_descriptor!("UDS Certificate").as_ref();
const DFU_CDI0_CERT: hal_usb::StringDescriptorRef = hal_usb::string_descriptor!("CDI0 Certificate").as_ref();
const DFU_CDI1_CERT: hal_usb::StringDescriptorRef = hal_usb::string_descriptor!("CDI1 Certificate").as_ref();

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
            DFU_FIRMWARE_HANDLE => Some(DFU_FIRMWARE),
            DFU_UDS_CERT_HANDLE => Some(DFU_UDS_CERT),
            DFU_CDI0_CERT_HANDLE => Some(DFU_CDI0_CERT),
            DFU_CDI1_CERT_HANDLE => Some(DFU_CDI1_CERT),
            _ => None,
        }
    }
}

fn get_certificate(flash: &mut FlashIpcClient, n: u8, data: &mut [u8]) -> Result<usize, DfuStatus> {
    pw_log::info!("Reading certificate {}", n as usize);
    let (partition, mut n) = match n {
        0 => (0, 0), // The UDS (dice) cert is located in bank=0, page=9.
        1 => (1, 0), // The CDI (dice) certs are located in bank=1, page=9.
        2 => (1, 1),
        _ => return Err(DfuStatus::ErrFile),
    };
    let mut offset = 0usize;
    let mut buf = [0u8; 1024];
    loop {
        let sz = core::cmp::min(2048 - offset, buf.len());
        flash.read(FlashAddress::info(partition, 9, offset as u32), &mut buf[..sz]).map_err(|_| DfuStatus::ErrUnknown)?;
        match PersoCertificate::from_bytes(&buf) {
            Ok((cert, _)) => {
                if n == 0 {
                    let len = cert.certificate.len();
                    pw_log::info!("Found cert: {} bytes", len as usize);
                    data[..len].copy_from_slice(cert.certificate);
                    return Ok(len);
                }
                offset += (cert.obj_size + 7) & !7;
                n -= 1;
            }
            Err(_) => break,
        }
    }
    Err(DfuStatus::ErrUnknown)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FwUpdateState {
    Idle,
    RomExt,
    Application,
    Done,
}

struct FwUpdate {
    state: FwUpdateState,
    next_erase: u32,
    start_block: u32,
    rom_ext: BootSlot,
    rom_ext_start: u32,
    rom_ext_end: u32,
    app: BootSlot,
    app_start: u32,
    app_end: u32,
}

impl FwUpdate {
    fn new(info: &BootInfo) -> Result<Self, ErrorCode> {
        let rom_ext= info.rom_ext.boot_slot.opposite().ok_or(eg_error::EG_ERROR_BOOT_SLOT_UNKNOWN)?;
        let rom_ext_start= FwUpdate::addr(rom_ext);
        let app = info.app.boot_slot.opposite().ok_or(eg_error::EG_ERROR_BOOT_SLOT_UNKNOWN)?;
        let app_start= FwUpdate::addr(app) + info.rom_ext.size;

        Ok(FwUpdate {
            state: FwUpdateState::Idle,
            next_erase: 0,
            start_block: 0,
            rom_ext,
            rom_ext_start,
            rom_ext_end: rom_ext_start + info.rom_ext.size,
            app,
            app_start,
            app_end: app_start + info.app.size,
        })
    }

    fn addr(slot: BootSlot) -> u32 {
        match slot {
            BootSlot::SlotA => 0,
            BootSlot::SlotB => 0x80000,
            _ => unreachable!(),
        }
    }
}

struct MyDfuHandler {
    flash: FlashIpcClient,
    sysmgr: SysmgrClient,
    update: FwUpdate,
}

impl MyDfuHandler {
    fn flash_erase(&mut self, mut start: u32, end: u32) -> Result<(), ErrorCode> {
        while start < end {
            self.flash.erase_page(FlashAddress::data(start))?;
            start += 2048;
        }
        Ok(())
    }

    fn flash_fw_block(&mut self, block_num: u32, data: &[u8]) -> Result<(), DfuStatus> {
        if block_num == self.update.next_erase {
            let id = ManifestIdentifier::read_from_bytes(&data[0x334..0x338]).unwrap();
            match id {
                ManifestIdentifier::ROM_EXT => {
                    pw_log::info!("Flashing ROM_EXT region at {}-{}",
                        self.update.rom_ext_start as u32,
                        self.update.rom_ext_end as u32,
                    );
                    self.flash_erase(self.update.rom_ext_start, self.update.rom_ext_end).map_err(|_| DfuStatus::ErrErase)?;
                    self.update.state = FwUpdateState::RomExt;
                    self.update.next_erase = 32;
                    self.update.start_block = block_num;
                }
                ManifestIdentifier::APPLICATION => {
                    pw_log::info!("Flashing Application region at {}-{}",
                        self.update.app_start as u32,
                        self.update.app_end as u32,
                    );
                    self.flash_erase(self.update.app_start, self.update.app_end).map_err(|_| DfuStatus::ErrErase)?;
                    self.update.state = FwUpdateState::Application;
                    self.update.start_block = block_num;
                }
                _ => {
                    pw_log::error!("Unknown manifest ID: {:08x}", id.0 as u32);
                    return Err(DfuStatus::ErrUnknown);
                }
            }
        }

        let block = block_num - self.update.start_block;
        match self.update.state {
            FwUpdateState::RomExt => {
                self.flash.program(FlashAddress::data(self.update.rom_ext_start+block*2048), data).map_err(|_| DfuStatus::ErrProg)?;
            }
            FwUpdateState::Application => {
                self.flash.program(FlashAddress::data(self.update.app_start+block*2048), data).map_err(|_| DfuStatus::ErrProg)?;
            }
            _ => { return Err(DfuStatus::ErrUnknown); }
        }

        if data.len() < 2048 {
            self.update.state = FwUpdateState::Done;
        }
        Ok(())
    }
}

impl DfuHandler for MyDfuHandler {
    fn dnload(&mut self, alt: u8, block_num: u16, data: &[u8]) -> DfuResult {
        pw_log::info!(
            "DNLOAD: alt={}, block={}, len={}",
            alt,
            block_num,
            data.len()
        );
        if alt == 0 {
           match self.flash_fw_block(block_num as u32, data) {
               Ok(()) => DfuResult::Ok,
               Err(e) => DfuResult::Err(e),
           }
        } else {
            DfuResult::Err(DfuStatus::ErrFile)
        }
    }

    fn upload(&mut self, alt: u8, block_num: u16, data: &mut [u8]) -> Result<usize, DfuStatus> {
        pw_log::info!(
            "UPLOAD: alt={}, block={}, len={}",
            alt,
            block_num,
            data.len()
        );
        match alt {
            1|2|3 => get_certificate(&mut self.flash, alt-1, data),
            _ => Err(DfuStatus::ErrFile),
        }
    }

    fn manifest(&mut self) -> DfuResult {
        pw_log::info!("MANIFEST");
        if self.update.state == FwUpdateState::Done {
            // TODO: check for errors.
            let _ = self.sysmgr.set_boot_policy(self.update.app, BootSlot::Unspecified);
            let _ = self.sysmgr.request_reboot();
        }
        DfuResult::Ok
    }

    fn abort(&mut self) {
        pw_log::info!("ABORT");
    }
}

fn handle_usb() -> Result<(), ErrorCode> {
    let mut serial_num_buffer = Aligned::<A4, _>([0_u8; 130]);
    let descriptors = MyDescriptors {
        serial_desc_bytes: hal_usb::hex_utf16_descriptor_aligned(
            &mut serial_num_buffer,
            b"DFU-12345",
        )
        .unwrap(),
        product_desc_bytes: PRODUCT_ID_DEFAULT,
    };

    const USB_CONFIG: UsbConfig = UsbConfig::new(&[], &[]);

    let mut usb = usb_driver::Usb::new(unsafe { usbdev::Usbdev::new() }, USB_CONFIG);
    let mut ep0 = usb_stack::SimpleEp0::new();
    let sysmgr = SysmgrClient::new(IpcChannel::new(handle::SYSMGR_SERVICE));
    let info = sysmgr.get_boot_info()?;
    let mut dfu = DfuClass::<_, 2048>::new(DFU_BUILDER, MyDfuHandler {
        flash: FlashIpcClient::new(IpcChannel::new(handle::FLASH_SERVICE))?,
        sysmgr,
        update: FwUpdate::new(&info)?,
    });

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
                | signals::USBDEV_RX_CRC_ERR
                | signals::USBDEV_RX_PID_ERR
                | signals::USBDEV_RX_BITSTUFF_ERR
                | signals::USBDEV_FRAME
                | signals::USBDEV_AV_SETUP_EMPTY,
            Instant::MAX,
        ).map_err(ErrorCode::kernel_error)?;

        if wait_return.user_data != 0 {
            pw_log::error!("Incorrect WaitReturn values");
            return Err(error::KERNEL_ERROR_UNKNOWN);
        }

        while let Some(event) = usb.poll() {
            let mut action = match dfu.handle_event(event) {
                Ok(a) => a,
                Err(e) => ep0.handle_event(e, &descriptors).unwrap_or(UsbAction::None),
            };
            action.run(&mut usb);
        }

        // Initiate any pending transmissions (e.g. UPLOAD blocks)
        dfu.poll(&mut usb);
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
fn entry() -> ! {
    usb_setup_pinmux();
    let ret = handle_usb().map_err(|e| {
        pw_log::error!("usbdfu failed: {:08x}", u32::from(e) as u32);
        Error::Unknown
    });
    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in {}", module_path!() as &str);
    loop {}
}
