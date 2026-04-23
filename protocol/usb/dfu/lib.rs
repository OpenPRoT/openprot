//! USB Device Firmware Upgrade (DFU) 1.1 implementation.

#![no_std]

use hal_usb::driver::{UsbDriver, UsbEvent, UsbPacket};
use hal_usb::{
    Direction, FunctionalDescriptor, InterfaceDescriptor, Recipient, Request, RequestType,
    SetupPacket, StringHandle,
};
use usb_stack::{UsbAction, UsbClass, EMPTY};

/// DFU specific constants.
pub const USB_CLASS_APP_SPECIFIC: u8 = 0xFE;
pub const USB_SUBCLASS_DFU: u8 = 0x01;
pub const USB_PROTOCOL_DFU: u8 = 0x02;

pub const DFU_DESCRIPTOR_TYPE: u8 = 0x21;

/// DFU specific requests.
pub const DFU_DETACH: Request = Request::new(
    Direction::HostToDevice,
    RequestType::Class,
    Recipient::Interface,
    0,
);
pub const DFU_DNLOAD: Request = Request::new(
    Direction::HostToDevice,
    RequestType::Class,
    Recipient::Interface,
    1,
);
pub const DFU_UPLOAD: Request = Request::new(
    Direction::DeviceToHost,
    RequestType::Class,
    Recipient::Interface,
    2,
);
pub const DFU_GETSTATUS: Request = Request::new(
    Direction::DeviceToHost,
    RequestType::Class,
    Recipient::Interface,
    3,
);
pub const DFU_CLRSTATUS: Request = Request::new(
    Direction::HostToDevice,
    RequestType::Class,
    Recipient::Interface,
    4,
);
pub const DFU_GETSTATE: Request = Request::new(
    Direction::DeviceToHost,
    RequestType::Class,
    Recipient::Interface,
    5,
);
pub const DFU_ABORT: Request = Request::new(
    Direction::HostToDevice,
    RequestType::Class,
    Recipient::Interface,
    6,
);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DfuState {
    AppIdle = 0,
    AppDetach = 1,
    DfuIdle = 2,
    DfuDnloadSync = 3,
    DfuDnloadBusy = 4,
    DfuDnloadIdle = 5,
    DfuManifestSync = 6,
    DfuManifest = 7,
    DfuManifestWaitReset = 8,
    DfuUploadIdle = 9,
    DfuError = 10,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DfuStatus {
    Ok = 0,
    ErrTarget = 1,
    ErrFile = 2,
    ErrWrite = 3,
    ErrErase = 4,
    ErrCheckChunked = 5,
    ErrProg = 6,
    ErrVerify = 7,
    ErrAddress = 8,
    ErrNotDone = 9,
    ErrFirmware = 10,
    ErrVendor = 11,
    ErrUsbr = 12,
    ErrPor = 13,
    ErrUnknown = 14,
    ErrStalledPkt = 15,
}

pub enum DfuResult {
    Ok,
    Err(DfuStatus),
}

/// A trait for providing the backend storage for the DFU implementation.
pub trait DfuHandler {
    /// Write a block of data to the device.
    fn dnload(&mut self, alt: u8, block_num: u16, data: &[u8]) -> DfuResult;
    /// Read a block of data from the device.
    fn upload(&mut self, alt: u8, block_num: u16, data: &mut [u8]) -> Result<usize, DfuStatus>;
    /// Finalize the download.
    fn manifest(&mut self) -> DfuResult;
    /// Abort the current operation.
    fn abort(&mut self);
}

/// A builder for DFU class configuration.
#[derive(Copy, Clone)]
pub struct DfuBuilder {
    pub interface_num: u8,
    pub alt_settings: u8,
    pub transfer_size: u16,
    pub attributes: u8,
    pub detach_timeout: u16,
}

impl DfuBuilder {
    pub const fn new(interface_num: u8, alt_settings: u8, transfer_size: u16) -> Self {
        Self {
            interface_num,
            alt_settings,
            transfer_size,
            attributes: 0x07, // bitCanDnload | bitCanUpload | bitManifestationTolerant
            detach_timeout: 0,
        }
    }

    pub const fn functional_descriptor(&self) -> FunctionalDescriptor {
        FunctionalDescriptor::raw(
            DFU_DESCRIPTOR_TYPE,
            &[
                self.attributes,
                (self.detach_timeout & 0xFF) as u8,
                ((self.detach_timeout >> 8) & 0xFF) as u8,
                (self.transfer_size & 0xFF) as u8,
                ((self.transfer_size >> 8) & 0xFF) as u8,
                0x10, // bcdDFUVersion 1.1 (0x0110)
                0x01,
            ],
        )
    }

    pub const fn interface(
        &self,
        alt: u8,
        name: StringHandle,
        func_descs: &'static [FunctionalDescriptor],
    ) -> InterfaceDescriptor {
        InterfaceDescriptor {
            name,
            interface_number: self.interface_num,
            alternate_setting: alt,
            interface_class: USB_CLASS_APP_SPECIFIC,
            interface_sub_class: USB_SUBCLASS_DFU,
            interface_protocol: USB_PROTOCOL_DFU,
            func_descs,
            endpoints: &[],
        }
    }
}

pub struct DfuClass<H, const BLOCK_SIZE: usize>
where
    H: DfuHandler,
{
    config: DfuBuilder,
    handler: H,
    state: DfuState,
    status: DfuStatus,
    alt: u8,
    expecting_dnload: bool,
    block_num: u16,
    buffer: [u8; BLOCK_SIZE],
    transfer_offset: usize,
    transfer_total: usize,
}

impl<H, const BLOCK_SIZE: usize> DfuClass<H, BLOCK_SIZE>
where
    H: DfuHandler,
{
    pub fn new(config: DfuBuilder, handler: H) -> Self {
        assert!(BLOCK_SIZE >= config.transfer_size as usize);
        Self {
            config,
            handler,
            state: DfuState::DfuIdle,
            status: DfuStatus::Ok,
            alt: 0,
            expecting_dnload: false,
            block_num: 0,
            buffer: [0u8; BLOCK_SIZE],
            transfer_offset: 0,
            transfer_total: 0,
        }
    }

    /// Polls the send buffer and initiates IN transfers if data is available.
    pub fn poll<D: UsbDriver>(&mut self, driver: &mut D) {
        if (self.state == DfuState::DfuUploadIdle || self.state == DfuState::DfuIdle)
            && self.transfer_offset < self.transfer_total
        {
            let data = &self.buffer[self.transfer_offset..self.transfer_total];
            let n = driver.transfer_in_unaligned(0, data, true);
            self.transfer_offset += n;

            if self.transfer_offset == self.transfer_total {
                if self.transfer_total < self.config.transfer_size as usize {
                    self.state = DfuState::DfuIdle;
                } else {
                    self.state = DfuState::DfuUploadIdle;
                }
            }
        }
    }

    fn handle_setup<'a>(&'a mut self, pkt: SetupPacket) -> (UsbAction<'a>, bool) {
        if pkt.request().recipient() != Recipient::Interface
            || (pkt.index() as u8) != self.config.interface_num
        {
            return (UsbAction::None, false);
        }

        match pkt.request() {
            DFU_DETACH => {
                // In DFU mode, DETACH is a no-op or transitions back to APP mode.
                // We'll just ACK it.
                (
                    UsbAction::TransferIn {
                        endpoint: 0,
                        data: EMPTY,
                        zlp: true,
                    },
                    true,
                )
            }
            DFU_DNLOAD => {
                if self.state == DfuState::DfuError {
                    return (UsbAction::StallInAndOut { endpoint: 0 }, true);
                }
                let len = pkt.length() as usize;
                if len > BLOCK_SIZE {
                    self.state = DfuState::DfuError;
                    self.status = DfuStatus::ErrStalledPkt;
                    return (UsbAction::StallInAndOut { endpoint: 0 }, true);
                }
                self.block_num = pkt.value();
                self.transfer_offset = 0;
                self.transfer_total = len;

                if len == 0 {
                    // Transition to manifest sync
                    self.state = DfuState::DfuManifestSync;
                    (
                        UsbAction::TransferIn {
                            endpoint: 0,
                            data: EMPTY,
                            zlp: true,
                        },
                        true,
                    )
                } else {
                    self.expecting_dnload = true;
                    self.state = DfuState::DfuDnloadSync;
                    (UsbAction::None, true)
                }
            }
            DFU_UPLOAD => {
                if self.state != DfuState::DfuIdle && self.state != DfuState::DfuUploadIdle {
                    return (UsbAction::StallInAndOut { endpoint: 0 }, true);
                }
                self.block_num = pkt.value();
                match self
                    .handler
                    .upload(self.alt, self.block_num, &mut self.buffer)
                {
                    Ok(n) => {
                        self.transfer_offset = 0;
                        self.transfer_total = n;
                        // poll_transmit will handle the actual transfer
                        (UsbAction::None, true)
                    }
                    Err(s) => {
                        self.state = DfuState::DfuError;
                        self.status = s;
                        (UsbAction::StallInAndOut { endpoint: 0 }, true)
                    }
                }
            }
            DFU_GETSTATUS => {
                // status[0]: bStatus
                // status[1-3]: bwPollTimeout (24-bit, little endian)
                // status[4]: bState
                // status[5]: iString
                self.buffer[0] = self.status as u8;
                self.buffer[1] = 0; // bwPollTimeout = 0
                self.buffer[2] = 0;
                self.buffer[3] = 0;
                self.buffer[4] = self.state as u8;
                self.buffer[5] = 0;

                // State transitions after GETSTATUS
                match self.state {
                    DfuState::DfuDnloadSync => self.state = DfuState::DfuDnloadIdle,
                    DfuState::DfuManifestSync => {
                        match self.handler.manifest() {
                            DfuResult::Ok => self.state = DfuState::DfuIdle, // ManifestationTolerant = 1
                            DfuResult::Err(s) => {
                                self.state = DfuState::DfuError;
                                self.status = s;
                            }
                        }
                    }
                    _ => {}
                }

                (
                    UsbAction::TransferInUnaligned {
                        endpoint: 0,
                        data: &self.buffer[..6],
                        zlp: true,
                    },
                    true,
                )
            }
            DFU_CLRSTATUS => {
                self.state = DfuState::DfuIdle;
                self.status = DfuStatus::Ok;
                (
                    UsbAction::TransferIn {
                        endpoint: 0,
                        data: EMPTY,
                        zlp: true,
                    },
                    true,
                )
            }
            DFU_GETSTATE => {
                self.buffer[0] = self.state as u8;
                (
                    UsbAction::TransferInUnaligned {
                        endpoint: 0,
                        data: &self.buffer[..1],
                        zlp: true,
                    },
                    true,
                )
            }
            DFU_ABORT => {
                self.handler.abort();
                self.state = DfuState::DfuIdle;
                self.status = DfuStatus::Ok;
                self.transfer_total = 0;
                self.transfer_offset = 0;
                (
                    UsbAction::TransferIn {
                        endpoint: 0,
                        data: EMPTY,
                        zlp: true,
                    },
                    true,
                )
            }
            Request::INTERFACE_SET_INTERFACE => {
                self.alt = pkt.value() as u8;
                (
                    UsbAction::TransferIn {
                        endpoint: 0,
                        data: EMPTY,
                        zlp: true,
                    },
                    true,
                )
            }
            _ => (UsbAction::None, false),
        }
    }

    fn handle_control_out<'a>(&'a mut self, pkt: impl UsbPacket) -> UsbAction<'a> {
        if !self.expecting_dnload {
            return UsbAction::StallInAndOut { endpoint: 0 };
        }

        let data = pkt.copy_to_unaligned(&mut self.buffer[self.transfer_offset..]);
        self.transfer_offset += data.len();

        if self.transfer_offset >= self.transfer_total {
            self.expecting_dnload = false;
            match self.handler.dnload(
                self.alt,
                self.block_num,
                &self.buffer[..self.transfer_total],
            ) {
                DfuResult::Ok => UsbAction::TransferIn {
                    endpoint: 0,
                    data: EMPTY,
                    zlp: true,
                },
                DfuResult::Err(s) => {
                    self.state = DfuState::DfuError;
                    self.status = s;
                    UsbAction::StallInAndOut { endpoint: 0 }
                }
            }
        } else {
            UsbAction::None
        }
    }
}

impl<H, const BLOCK_SIZE: usize> UsbClass for DfuClass<H, BLOCK_SIZE>
where
    H: DfuHandler,
{
    fn handle_event<'a, P: UsbPacket>(
        &'a mut self,
        event: UsbEvent<P>,
    ) -> Result<UsbAction<'a>, UsbEvent<P>> {
        match event {
            UsbEvent::SetupPacket { pkt, endpoint } if endpoint == 0 => {
                let (action, claimed) = self.handle_setup(pkt);
                if claimed {
                    Ok(action)
                } else {
                    Err(UsbEvent::SetupPacket { pkt, endpoint })
                }
            }
            UsbEvent::DataOutPacket(pkt) if pkt.endpoint_index() == 0 => {
                Ok(self.handle_control_out(pkt))
            }
            _ => Err(event),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aligned::{Aligned, A4};
    use hal_usb::driver::UsbEvent;
    use usb_stack::testing::FakeUsbPacket;

    struct MockHandler {
        dnload_called: bool,
        upload_called: bool,
        manifest_called: bool,
        abort_called: bool,
    }

    impl MockHandler {
        fn new() -> Self {
            Self {
                dnload_called: false,
                upload_called: false,
                manifest_called: false,
                abort_called: false,
            }
        }
    }

    impl DfuHandler for MockHandler {
        fn dnload(&mut self, _alt: u8, _block_num: u16, _data: &[u8]) -> DfuResult {
            self.dnload_called = true;
            DfuResult::Ok
        }
        fn upload(
            &mut self,
            _alt: u8,
            _block_num: u16,
            data: &mut [u8],
        ) -> Result<usize, DfuStatus> {
            self.upload_called = true;
            data[0] = 0xAA;
            Ok(1)
        }
        fn manifest(&mut self) -> DfuResult {
            self.manifest_called = true;
            DfuResult::Ok
        }
        fn abort(&mut self) {
            self.abort_called = true;
        }
    }

    fn setup_packet(req: Request, val: u16, idx: u16, len: u16) -> SetupPacket {
        SetupPacket::new([
            (u16::from(req) as u32) | ((val as u32) << 16),
            (idx as u32) | ((len as u32) << 16),
        ])
    }

    #[test]
    fn test_dfu_dnload_sequence() {
        let config = DfuBuilder::new(1, 1, 64);
        let mut dfu = DfuClass::<_, 64>::new(config, MockHandler::new());

        // 1. DNLOAD request (block 0, 4 bytes)
        let setup = setup_packet(DFU_DNLOAD, 0, 1, 4);
        let event: UsbEvent<FakeUsbPacket<'_>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup,
        };
        let action = dfu.handle_event(event).map_err(|_| ()).unwrap();
        assert!(matches!(action, UsbAction::None));
        assert_eq!(dfu.state, DfuState::DfuDnloadSync);

        // 2. Data OUT packet
        let data = [1, 2, 3, 4];
        let pkt = FakeUsbPacket { ep: 0, data: &data };
        let action = dfu
            .handle_event(UsbEvent::DataOutPacket(pkt))
            .map_err(|_| ())
            .unwrap();
        assert!(matches!(action, UsbAction::TransferIn { endpoint: 0, .. }));
        assert!(dfu.handler.dnload_called);

        // 3. GETSTATUS
        let setup = setup_packet(DFU_GETSTATUS, 0, 1, 6);
        let event: UsbEvent<FakeUsbPacket<'_>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup,
        };
        let action = dfu.handle_event(event).map_err(|_| ()).unwrap();
        assert!(matches!(action, UsbAction::TransferInUnaligned { .. }));
        assert_eq!(dfu.state, DfuState::DfuDnloadIdle);

        // 4. DNLOAD ZLP (Manifestation)
        let setup = setup_packet(DFU_DNLOAD, 1, 1, 0);
        let event: UsbEvent<FakeUsbPacket<'_>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup,
        };
        let action = dfu.handle_event(event).map_err(|_| ()).unwrap();
        assert!(matches!(action, UsbAction::TransferIn { endpoint: 0, .. }));
        assert_eq!(dfu.state, DfuState::DfuManifestSync);

        // 5. GETSTATUS (Manifest)
        let setup = setup_packet(DFU_GETSTATUS, 0, 1, 6);
        let event: UsbEvent<FakeUsbPacket<'_>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup,
        };
        let _ = dfu.handle_event(event).map_err(|_| ()).unwrap();
        assert!(dfu.handler.manifest_called);
        assert_eq!(dfu.state, DfuState::DfuIdle);
    }

    #[test]
    fn test_dfu_upload() {
        let config = DfuBuilder::new(1, 1, 64);
        let mut dfu = DfuClass::<_, 64>::new(config, MockHandler::new());

        let setup = setup_packet(DFU_UPLOAD, 0, 1, 64);
        let event: UsbEvent<FakeUsbPacket<'_>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup,
        };
        let action = dfu.handle_event(event).map_err(|_| ()).unwrap();
        assert!(matches!(action, UsbAction::None));

        // Mock driver for poll_transmit
        struct MockDriver {
            transferred: usize,
        }
        impl UsbDriver for MockDriver {
            const MAX_PACKET_SIZE: usize = 64;
            type Packet<'a> = FakeUsbPacket<'a>;
            fn transfer_in(&mut self, _ep: u8, _data: &Aligned<A4, [u8]>, _zlp: bool) -> usize {
                0
            }
            fn transfer_in_unaligned(&mut self, _ep: u8, data: &[u8], _zlp: bool) -> usize {
                self.transferred = data.len();
                data.len()
            }
            fn stall(&mut self, _ep: u8, _stall: bool) {}
            fn is_stalled(&mut self, _ep: u8) -> bool {
                false
            }
            fn set_address(&mut self, _addr: u8) {}
            fn poll(&mut self) -> Option<UsbEvent<Self::Packet<'_>>> {
                None
            }
        }

        let mut driver = MockDriver { transferred: 0 };
        dfu.poll_transmit(&mut driver);
        assert_eq!(driver.transferred, 1);
        assert!(dfu.handler.upload_called);
        assert_eq!(dfu.state, DfuState::DfuIdle); // n < transfer_size
    }

    #[test]
    fn test_dfu_abort() {
        let config = DfuBuilder::new(1, 1, 64);
        let mut dfu = DfuClass::<_, 64>::new(config, MockHandler::new());
        dfu.state = DfuState::DfuDnloadIdle;

        let setup = setup_packet(DFU_ABORT, 0, 1, 0);
        let event: UsbEvent<FakeUsbPacket<'_>> = UsbEvent::SetupPacket {
            endpoint: 0,
            pkt: setup,
        };
        let action = dfu.handle_event(event).map_err(|_| ()).unwrap();
        assert!(matches!(action, UsbAction::TransferIn { endpoint: 0, .. }));
        assert!(dfu.handler.abort_called);
        assert_eq!(dfu.state, DfuState::DfuIdle);
    }
}
