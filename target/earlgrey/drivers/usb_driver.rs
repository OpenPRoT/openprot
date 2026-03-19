#![no_std]

use aligned::A4;
use aligned::Aligned;
use core::cmp::min;
use util_regcpy::copy_to_reg_array;
use console::traceln;
use hal_usb::SetupPacket;
use hal_usb::driver::UsbDriver;
use hal_usb::driver::UsbEvent;
use hal_usb::driver::UsbPacket;
use zerocopy::IntoBytes;

const MAX_PACKET_SIZE: usize = 64;
const BUFFER_SLOT_SIZE_WORDS: usize = MAX_PACKET_SIZE / 4;
const BUFFER_SLOT_COUNT: usize = 32;

const EMPTY_A4: &Aligned<A4, [u8]> = &Aligned([]);

use buf_pool::BufId;
use buf_pool::BufPool;
use buf_pool::BuffPoolAllocator;
use core::cmp;
use ureg::RealMmio;

use crate::transmit_queue::TransmitQueues;

pub struct PacketHandle<TMmio: ureg::Mmio + Copy> {
    // A reference to 16 words of packet data in the peripheral MMIO memory.
    data: ureg::Array<16, ureg::RegRef<ureg::ReadWriteReg32<0, u32, u32>, TMmio>>,
    // The length of the packet in bytes
    packet_len: u16,
    ep: u8,
}

impl<TMmio: ureg::Mmio + Copy> UsbPacket for PacketHandle<TMmio> {
    fn endpoint_index(&self) -> usize {
        self.ep.into()
    }
    fn len(&self) -> usize {
        self.packet_len.into()
    }

    fn copy_to_uninit(self, dest: &mut [core::mem::MaybeUninit<u32>]) -> &Aligned<A4, [u8]> {
        #![allow(clippy::needless_range_loop)]

        // TODO: Are we sure we want to silently truncate if dest isn't big enough?
        let word_len = min(min(dest.len(), MAX_PACKET_SIZE / 4), self.len().div_ceil(4));
        for i in 0..word_len {
            dest[i].write(self.data.at(i).read());
        }
        //let result = unsafe { mutask_subtle::slice_assume_init(&dest[..word_len]) };

        // This is feature(maybe_uninit_slice).
        let result = &dest[..word_len];
        let result = unsafe { &*(result as *const [core::mem::MaybeUninit<u32>] as *const [u32]) };

        // TODO: add a Aligned::try_from() function to the aligned crate and use it here with unwrap.
        unsafe {
            core::mem::transmute::<&[u8], &Aligned<A4, [u8]>>(
                &result.as_bytes()[..min(self.len(), word_len * 4)],
            )
        }
    }

    fn copy_to(self, dest: &mut [u32]) -> &Aligned<A4, [u8]> {
        #![allow(clippy::needless_range_loop)]

        // TODO: Are we sure we want to silently truncate if dest isn't big enough?
        let word_len = min(min(dest.len(), MAX_PACKET_SIZE / 4), self.len().div_ceil(4));
        for i in 0..word_len {
            dest[i] = self.data.at(i).read();
        }
        // TODO: add a Aligned::try_from() function to the aligned crate and use it here with unwrap.
        unsafe {
            core::mem::transmute::<&[u8], &Aligned<A4, [u8]>>(
                &dest.as_bytes()[..min(self.len(), dest.as_bytes().len())],
            )
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NextInPacket {
    buf_id: u8,
    len: u8,
}
impl NextInPacket {
    const NONE: Self = Self {
        buf_id: 0xff,
        len: 0xff,
    };
}

#[derive(Clone, Copy)]
pub struct EpIn {
    pub num: u8,
    pub buf_pool_size: u32,
}

#[derive(Clone, Copy)]
pub struct EpOut {
    pub num: u8,
    /// If true, hardware will NAK OUT transfers on this endpoint after the first
    /// until software re-enables by setting `rxenable_out`
    pub set_nak: bool,
}

const NB_EP: usize = 12;

pub struct Usb {
    mmio: usbdev::Usbdev,

    // buffer pool for SETUP transfers from host, technically common, but only used for EP0.
    buf_pool_setup: BufPool,
    // buffer pool for OUT transfers from host, common for all EPs.
    buf_pool_out: BufPool,

    // buffer pools for IN transfers.
    buf_pools_in: [BufPool; NB_EP],

    transmit_queues: TransmitQueues,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UsbConfig {
    buf_pool_setup: BufPool,
    buf_pool_out: BufPool,
    buf_pools_in: [BufPool; NB_EP],
    in_mask: u32,
    out_mask: u32,
    set_nak_mask: u32,
}
impl UsbConfig {
    /// Construct a USB config. This should typically be done within a const
    /// block so any errors become compile-time panics.
    ///
    /// # Panic
    ///
    /// This function will panic if the supplied endpoints are invalid.
    #[inline(always)]
    pub const fn new(eps_in: &[EpIn], eps_out: &[EpOut]) -> Self {
        let mut buf_pool_allocator = BuffPoolAllocator::new();
        let buf_pool_setup = buf_pool_allocator.new_bufpool(4).unwrap();
        let buf_pool_out = buf_pool_allocator.new_bufpool(12).unwrap();

        let mut buf_pools_in = [BufPool::EMPTY; NB_EP];
        let mut i = 0;
        while i < eps_in.len() {
            let ep = &eps_in[i];
            if ep.num == 0 || ep.num as usize > NB_EP {
                panic!("Invalid endpoint number");
            }
            let Some(new_pool) = buf_pool_allocator.new_bufpool(ep.buf_pool_size) else {
                panic!("Bufpool allocation overflow");
            };
            buf_pools_in[ep.num as usize] = new_pool;
            i += 1;
        }
        let Some(new_pool) = buf_pool_allocator.remainder_bufpool() else {
            panic!("Bufpool allocation overflow");
        };
        buf_pools_in[0] = new_pool;

        let in_mask: u32 = {
            let mut v = 1; // always enable EP0
            let mut i = 0;
            while i < eps_in.len() {
                v |= 1 << eps_in[i].num;
                i += 1;
            }
            v
        };

        let out_mask: u32 = {
            let mut v = 1; // always enable EP0
            let mut i = 0;
            while i < eps_out.len() {
                v |= 1 << eps_out[i].num;
                i += 1;
            }
            v
        };

        let set_nak_mask: u32 = {
            let mut v = 0u32;
            let mut i = 0;
            while i < eps_out.len() {
                if eps_out[i].set_nak {
                    v |= 1 << eps_out[i].num;
                }
                i += 1;
            }

            // Writes to `rxenable_out` are not atomic - therefore we must
            // guarantee that `set_nak_out` is only set for up to a single
            // endpoint (github.com/lowRISC/opentitan/issues/27434)
            assert!(
                v.count_ones() <= 1,
                "set_nak_out can be enabled on at most one endpoint"
            );
            v
        };

        Self {
            buf_pool_setup,
            buf_pool_out,
            buf_pools_in,
            in_mask,
            out_mask,
            set_nak_mask,
        }
    }
}

impl Usb {
    pub fn new(mmio: usbdev::Usbdev, config: UsbConfig) -> Self {
        let mut result = Self {
            mmio,
            buf_pool_setup: config.buf_pool_setup,
            buf_pool_out: config.buf_pool_out,
            buf_pools_in: config.buf_pools_in,
            transmit_queues: TransmitQueues::new(),
        };
        result.init(&config);
        result
    }
    fn init(&mut self, config: &UsbConfig) {
        self.fill_setup_buffer_fifo();
        self.fill_out_buffer_fifo();

        let regs = self.mmio.regs_mut();

        regs.ep_in_enable0().write(|_| config.in_mask.into());
        regs.ep_out_enable0().write(|_| config.out_mask.into());
        regs.rxenable_out0().write(|_| config.out_mask.into());
        regs.set_nak_out0().write(|_| config.set_nak_mask.into());

        regs.rxenable_setup0().write(|w| w.setup0(true));
        regs.intr_enable().write(|w| {
            w.pkt_received(true)
                .pkt_sent(true)
                .disconnected(true)
                .host_lost(true)
                .link_reset(true)
                .link_suspend(true)
                .link_resume(true)
                .av_out_empty(true)
                .rx_full(true)
                .av_overflow(true)
                .link_in_err(false)
                .rx_crc_err(false)
                .rx_pid_err(false)
                .rx_bitstuff_err(false)
                .frame(false)
                .powered(true)
                .link_out_err(false)
                .av_setup_empty(true)
        });
        regs.usbctrl().modify(|w| w.enable(true));

        let stat = regs.usbstat().read();
        traceln!(
            "Usb out_depth={} setup_depth={}",
            stat.av_out_depth(),
            stat.av_setup_depth()
        );
    }

    fn reset_in(&mut self) {
        let regs = self.mmio.regs_mut();
        for (i, pool) in &mut self.buf_pools_in.iter_mut().enumerate() {
            pool.reset();
            let configin = regs.configin().at(i);
            if configin.read().pend() {
                // link reset will cancel any pending transactions. Since we're
                // resetting the pool/queue state there's nothing else to do but
                // clear the notification.
                configin.write(|w| w.pend_clear());
            }
        }
        self.transmit_queues.reset();
    }

    fn fill_setup_buffer_fifo(&mut self) {
        // Setup buffers for incoming SETUP packets from host.
        while !self.mmio.regs().usbstat().read().av_setup_full() {
            let Some(buf_id) = self.buf_pool_setup.take() else {
                break;
            };
            self.mmio
                .regs_mut()
                .avsetupbuffer()
                .write(|w| w.buffer(buf_id.into()));
        }
    }
    fn fill_out_buffer_fifo(&mut self) {
        // Setup buffers for incoming OUT packets from host.
        while !self.mmio.regs().usbstat().read().av_out_full() {
            let Some(buf_id) = self.buf_pool_out.take() else {
                break;
            };
            self.mmio
                .regs_mut()
                .avoutbuffer()
                .write(|w| w.buffer(buf_id.into()));
        }
    }

    /// Flush packets bufferred to transmit on `endpoint` starting with `buf_id`.
    fn clear_ep_tx_queue(&mut self, endpoint: u32, mut buf_id: BufId) {
        let buf_pool_in = &mut self.buf_pools_in[usize::try_from(endpoint).unwrap()];
        buf_pool_in.put(buf_id);
        while let Some(pkt) = self.transmit_queues.deque_next_packet(endpoint, buf_id) {
            buf_id = BufId(pkt.buf_id.into());
            buf_pool_in.put(buf_id);
        }
    }

    /// Resume accepting OUT transfers on this endpoint.
    ///
    /// This should be called after processing an OUT transfer on a given endpoint
    /// that was configured with `EpOut { set_nak: true }`. The `set_nak` option causes
    /// the hardware to automatically NAK subsequent OUT transactions until this function
    /// is called to re-enable reception.
    pub fn set_rxenable(&mut self, ep_num: u8) {
        self.mmio
            .regs_mut()
            .rxenable_out0()
            .modify(|w| bit_setval(u32::from(w), ep_num.into(), true).into());
    }
}

#[inline(always)]
fn bit_setval(bits: u32, index: usize, value: bool) -> u32 {
    let mask = 1 << index;
    if value { bits | mask } else { bits & !mask }
}

impl UsbDriver for Usb {
    const MAX_PACKET_SIZE: usize = 64;
    type Packet<'a> = PacketHandle<RealMmio<'a>>;

    #[inline(always)]
    fn stall_in(&mut self, endpoint_idx: u8, stalled: bool) {
        self.mmio
            .regs_mut()
            .in_stall0()
            .modify(|w| bit_setval(u32::from(w), endpoint_idx.into(), stalled).into());
    }
    #[inline(always)]
    fn stall_out(&mut self, endpoint_idx: u8, stalled: bool) {
        self.mmio
            .regs_mut()
            .out_stall0()
            .modify(|w| bit_setval(u32::from(w), endpoint_idx.into(), stalled).into());
    }

    /// Store data in peripheral buffer that will be transferred when the host requests it.
    #[inline(never)]
    fn transfer_in(&mut self, endpoint: u8, mut data: &Aligned<A4, [u8]>, zlp: bool) -> usize {
        let mut bytes_queued = 0;
        let zlp = zlp && (data.len() % MAX_PACKET_SIZE) == 0;
        loop {
            if data.is_empty() && !zlp {
                break;
            }
            let regs = self.mmio.regs_mut();
            let Some(configin_reg) = regs.configin().get(endpoint.into()) else {
                // Fault?
                return 0;
            };

            let pkt = &data[..cmp::min(MAX_PACKET_SIZE, data.len())];
            if pkt.len() == MAX_PACKET_SIZE {
                data = &data[pkt.len()..];
            } else {
                data = EMPTY_A4;
            }

            let buf_pool = self.buf_pools_in.get_mut(usize::from(endpoint)).unwrap();

            // Check to see if we have enough buffers to send both
            // the last data packet and a ZLP if necessary. If not, leave last
            // data packet unsent so caller knows to retry transfer
            if zlp && pkt.len() == MAX_PACKET_SIZE && buf_pool.len() < 2 {
                traceln!("Couldn't find buf in pool for last IN + ZLP");
                break;
            }

            let Some(buf_id) = buf_pool.take() else {
                traceln!("Couldn't find buf in pool for next IN");
                break;
            };

            let Some(buffer) = regs
                .buffer()
                .get_sub_array::<BUFFER_SLOT_SIZE_WORDS>(buf_id.offset())
            else {
                // Shouldn't fail to get buffer offset
                unreachable!();
            };
            copy_to_reg_array(&buffer, pkt);

            match self.transmit_queues.queue(
                endpoint.into(),
                NextInPacket {
                    buf_id: u32::from(buf_id) as u8,
                    len: pkt.len() as u8,
                },
            ) {
                TransmitQueueAction::None => {}
                TransmitQueueAction::SendNow => {
                    if configin_reg.read().rdy() {
                        traceln!("WARN: Packet already queued in hardware");
                    }
                    configin_reg.write(|w| {
                        w.buffer(buf_id.into())
                            .rdy(true)
                            .size(u32::try_from(pkt.len()).unwrap())
                    });
                }
            }
            bytes_queued += pkt.len();

            if pkt.is_empty() {
                break;
            }
        }
        bytes_queued
    }
    fn set_address(&mut self, address: u8) {
        self.mmio
            .regs_mut()
            .usbctrl()
            .modify(|w| w.device_address(address.into()));
    }

    #[inline(never)]
    fn poll(&mut self) -> Option<UsbEvent<PacketHandle<RealMmio<'_>>>> {
        let intr = self.mmio.regs_mut().intr_state().read();

        // TODO: use count_leading_zeros() to iterate over the pending interrupts
        if intr.pkt_received() {
            let fifo_entry = self.mmio.regs_mut().rxfifo().read();

            if fifo_entry.setup() {
                self.fill_setup_buffer_fifo();

                if let Some(configin_reg) = self
                    .mmio
                    .regs_mut()
                    .configin()
                    .get(fifo_entry.ep() as usize)
                {
                    let configin = configin_reg.read();
                    if configin.pend() {
                        // Previous transmission was cancelled by an incoming setup packet
                        configin_reg.write(|w| w.pend_clear());
                        self.clear_ep_tx_queue(fifo_entry.ep(), BufId(configin.buffer()));
                    }
                }
            } else {
                self.fill_out_buffer_fifo();
            }

            let offset = usize::try_from(fifo_entry.buffer()).unwrap() * BUFFER_SLOT_SIZE_WORDS;
            let Some(pkt_buffer) = self
                .mmio
                .regs()
                .into_buffer()
                .get_sub_array::<BUFFER_SLOT_SIZE_WORDS>(offset)
            else {
                return Some(UsbEvent::ErrorUnexpectedBufId);
            };

            if fifo_entry.setup() {
                let buf_id = BufId(fifo_entry.buffer());

                // Return the buffer back to the pool, but don't call
                // self.fill_setup_buffer_fifo() yet, as the caller to poll()
                // may look at this data from the returned event, and we don't want
                // the peripheral to change it while they're reading the data
                // (because the returned Event is exclusively holding self, it won't be possible
                // to call fill_setup_buffer_fifo() until after they lose the event).
                self.buf_pool_setup.put(buf_id);

                let ep = u8::try_from(fifo_entry.ep()).unwrap();
                let pkt_handle = PacketHandle {
                    data: pkt_buffer,
                    // These unwraps will optimize out
                    ep,
                    packet_len: u16::try_from(fifo_entry.size()).unwrap(),
                };
                let mut pkt_words = [0_u32; 2];
                pkt_handle.copy_to(&mut pkt_words);
                return Some(UsbEvent::SetupPacket {
                    endpoint: ep,
                    pkt: SetupPacket::new(pkt_words),
                });
            } else {
                let buf_id = BufId(fifo_entry.buffer());
                self.buf_pool_out.put(buf_id);
                return Some(UsbEvent::DataOutPacket(PacketHandle {
                    data: pkt_buffer,
                    // These unwraps will optimize out
                    ep: u8::try_from(fifo_entry.ep()).unwrap(),
                    packet_len: u16::try_from(fifo_entry.size()).unwrap(),
                }));
            }
        }
        if intr.pkt_sent() {
            let regs = self.mmio.regs_mut();
            loop {
                let endpoint_bits: u32 = regs.in_sent0().read().into();
                if endpoint_bits == 0 {
                    break;
                }
                let endpoint_id = endpoint_bits.trailing_zeros();

                // Ensure we don't get interrupted about this packet again (w1c)
                regs.in_sent0().write(|_| (1 << endpoint_id).into());

                let Some(configin_reg) = regs.configin().get(usize::try_from(endpoint_id).unwrap())
                else {
                    // TODO: Log weird hardware behavior?
                    continue;
                };
                let configin = configin_reg.read();

                if configin.rdy() {
                    // TODO: Log weird hardware behavior...
                    continue;
                }

                let buf_pool = self
                    .buf_pools_in
                    .get_mut(usize::try_from(endpoint_id).unwrap())
                    .unwrap();
                let sent_buf_id = BufId(configin.buffer());
                buf_pool.put(sent_buf_id);

                if let Some(next_pkt) = self
                    .transmit_queues
                    .deque_next_packet(endpoint_id, sent_buf_id)
                {
                    // We have more packets for this endpoint already in the
                    // peripheral SRAM; let's tell the hardware to prep the next
                    // one for sending.
                    configin_reg.write(|w| {
                        w.buffer(next_pkt.buf_id.into())
                            .size(next_pkt.len.into())
                            .rdy(true)
                    });
                }
                return Some(UsbEvent::PacketSent {
                    endpoint: endpoint_id,
                });
            }
        }
        if intr.host_lost() {
            self.mmio
                .regs_mut()
                .intr_state()
                .write(|w| w.host_lost_clear());
            return Some(UsbEvent::LinkDown);
        }
        if intr.powered() {
            self.mmio
                .regs_mut()
                .intr_state()
                .write(|w| w.powered_clear());
            return Some(UsbEvent::VBus);
        }
        if intr.disconnected() {
            self.mmio
                .regs_mut()
                .intr_state()
                .write(|w| w.disconnected_clear());
            return Some(UsbEvent::VBusLost);
        }
        if intr.link_reset() {
            self.reset_in();
            self.mmio
                .regs_mut()
                .intr_state()
                .write(|w| w.link_reset_clear());
            return Some(UsbEvent::UsbReset);
        }
        if intr.av_overflow() {
            self.mmio
                .regs_mut()
                .intr_state()
                .write(|w| w.av_overflow_clear());
            traceln!("av_overflow");
        }
        if intr.link_suspend() {
            self.mmio
                .regs_mut()
                .intr_state()
                .write(|w| w.link_suspend_clear());
        }
        if intr.link_resume() {
            self.mmio
                .regs_mut()
                .intr_state()
                .write(|w| w.link_resume_clear());
        }
        if intr.av_out_empty() {
            traceln!("av_out_empty");
            self.fill_out_buffer_fifo();
        }
        if intr.av_setup_empty() {
            traceln!("av_setup_empty");
            self.fill_setup_buffer_fifo();
        }
        if intr.rx_full() {
            traceln!("rx_full");
        }

        None
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum TransmitQueueAction {
    None,
    SendNow,
}

pub mod transmit_queue {
    use super::*;

    pub struct TransmitQueues {
        slots: [NextInPacket; BUFFER_SLOT_COUNT],

        /// Indexed by endpoint num, this is the slot index of the last packet
        /// queued for transmission on that endpoint.
        last_pkt_idx: [Option<u8>; NB_EP],
    }
    impl TransmitQueues {
        pub const fn new() -> Self {
            Self {
                slots: [NextInPacket::NONE; BUFFER_SLOT_COUNT],
                last_pkt_idx: [None; NB_EP],
            }
        }
        pub fn reset(&mut self) {
            *self = Self::new()
        }

        #[must_use]
        #[inline(always)]
        pub fn queue(&mut self, ep_id: u32, pkt: NextInPacket) -> TransmitQueueAction {
            let ep_id = usize::try_from(ep_id).unwrap();
            let last_pkt_idx = &mut self.last_pkt_idx[ep_id];
            let result = if let Some(last_pkt_idx) = *last_pkt_idx
                && let Some(entry) = self.slots.get_mut(usize::from(last_pkt_idx))
            {
                *entry = pkt;
                TransmitQueueAction::None
            } else {
                if let Some(entry) = self.slots.get_mut(usize::from(pkt.buf_id)) {
                    *entry = NextInPacket::NONE;
                }
                TransmitQueueAction::SendNow
            };
            *last_pkt_idx = Some(pkt.buf_id);
            result
        }

        #[inline(always)]
        pub fn deque_next_packet(
            &mut self,
            ep_id: u32,
            sent_buf_id: BufId,
        ) -> Option<NextInPacket> {
            let ep_id = usize::try_from(ep_id).unwrap();
            let sent_buf_id = usize::from(sent_buf_id);
            let next_pkt = &mut self.slots[sent_buf_id];
            if usize::from(next_pkt.buf_id) >= BUFFER_SLOT_COUNT {
                self.last_pkt_idx[ep_id] = None;
                return None;
            }
            Some(core::mem::replace(next_pkt, NextInPacket::NONE))
        }
    }
    impl Default for TransmitQueues {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn test_transmit_queues() {
            let mut queues = TransmitQueues::new();
            assert_eq!(
                queues.queue(0, NextInPacket { buf_id: 4, len: 64 }),
                TransmitQueueAction::SendNow
            );
            assert_eq!(
                queues.queue(0, NextInPacket { buf_id: 5, len: 64 }),
                TransmitQueueAction::None
            );
            assert_eq!(
                queues.queue(
                    1,
                    NextInPacket {
                        buf_id: 10,
                        len: 64
                    }
                ),
                TransmitQueueAction::SendNow
            );
            assert_eq!(
                queues.queue(
                    1,
                    NextInPacket {
                        buf_id: 11,
                        len: 64
                    }
                ),
                TransmitQueueAction::None
            );
            assert_eq!(
                queues.queue(0, NextInPacket { buf_id: 6, len: 0 }),
                TransmitQueueAction::None
            );
            assert_eq!(
                queues.queue(1, NextInPacket { buf_id: 12, len: 3 }),
                TransmitQueueAction::None
            );

            assert_eq!(
                queues.deque_next_packet(1, BufId(10)),
                Some(NextInPacket {
                    buf_id: 11,
                    len: 64
                })
            );
            assert_eq!(
                queues.deque_next_packet(0, BufId(4)),
                Some(NextInPacket { buf_id: 5, len: 64 })
            );
            assert_eq!(
                queues.deque_next_packet(0, BufId(5)),
                Some(NextInPacket { buf_id: 6, len: 0 })
            );
            assert_eq!(queues.deque_next_packet(0, BufId(6)), None);

            assert_eq!(
                queues.queue(
                    1,
                    NextInPacket {
                        buf_id: 10,
                        len: 33
                    }
                ),
                TransmitQueueAction::None
            );
            assert_eq!(
                queues.queue(0, NextInPacket { buf_id: 4, len: 64 }),
                TransmitQueueAction::SendNow
            );
            assert_eq!(
                queues.queue(0, NextInPacket { buf_id: 5, len: 9 }),
                TransmitQueueAction::None
            );

            assert_eq!(
                queues.deque_next_packet(1, BufId(11)),
                Some(NextInPacket { buf_id: 12, len: 3 })
            );
            assert_eq!(
                queues.deque_next_packet(1, BufId(12)),
                Some(NextInPacket {
                    buf_id: 10,
                    len: 33
                })
            );
            assert_eq!(
                queues.deque_next_packet(0, BufId(4)),
                Some(NextInPacket { buf_id: 5, len: 9 })
            );
            assert_eq!(queues.deque_next_packet(0, BufId(5)), None);
            assert_eq!(queues.deque_next_packet(1, BufId(10)), None);

            // Make sure we cleaned up after ourselves...
            assert!(queues.slots.iter().all(|s| *s == NextInPacket::NONE));
            assert!(queues.last_pkt_idx.iter().all(|i| i.is_none()));
        }
    }
}

pub mod buf_pool {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    #[repr(transparent)]
    pub struct BufId(pub u32);
    impl BufId {
        pub const fn offset(&self) -> usize {
            self.0 as usize * BUFFER_SLOT_SIZE_WORDS
        }
    }
    impl From<BufId> for u32 {
        fn from(value: BufId) -> Self {
            value.0
        }
    }
    impl From<BufId> for usize {
        fn from(value: BufId) -> Self {
            usize::try_from(value.0).unwrap()
        }
    }
    impl From<u32> for BufId {
        fn from(value: u32) -> Self {
            Self(value)
        }
    }

    pub struct BuffPoolAllocator {
        allocated_buf_ids: u32,
    }
    impl Default for BuffPoolAllocator {
        fn default() -> Self {
            Self::new()
        }
    }
    impl BuffPoolAllocator {
        pub const fn new() -> Self {
            Self {
                allocated_buf_ids: 0,
            }
        }
        pub const fn new_bufpool(&mut self, len: u32) -> Option<BufPool> {
            let start_id = self.allocated_buf_ids.trailing_ones();
            if len == 0 || start_id + len > 32 {
                return None;
            }
            let mask = (((1_u64 << len) - 1) << start_id) as u32;
            self.allocated_buf_ids |= mask;
            Some(BufPool {
                init_value: mask,
                available_bufs: mask,
            })
        }
        pub const fn remainder_bufpool(mut self) -> Option<BufPool> {
            let left = self.allocated_buf_ids.leading_zeros();
            self.new_bufpool(left)
        }
    }

    #[cfg(test)]
    mod test_buff_pool_allocator {
        use super::*;

        #[test]
        fn test_next() {
            let mut allocator = BuffPoolAllocator::new();
            assert_eq!(
                allocator.new_bufpool(1),
                Some(BufPool {
                    available_bufs: 0b01,
                    init_value: 0b01
                })
            );
            assert_eq!(
                allocator.new_bufpool(1),
                Some(BufPool {
                    available_bufs: 0b10,
                    init_value: 0b10,
                })
            );
            assert_eq!(
                allocator.new_bufpool(2),
                Some(BufPool {
                    available_bufs: 0b1100,
                    init_value: 0b1100,
                })
            );
            assert_eq!(allocator.new_bufpool(0), None);
            assert_eq!(allocator.new_bufpool(30), None);
            assert_eq!(
                allocator.new_bufpool(28),
                Some(BufPool {
                    available_bufs: (0xffff_ffffu64 << 4) as u32,
                    init_value: (0xffff_ffffu64 << 4) as u32,
                })
            );
            assert_eq!(allocator.new_bufpool(1), None);
        }
        #[test]
        fn test_remainder() {
            let mut allocator = BuffPoolAllocator::new();
            assert_eq!(
                allocator.new_bufpool(1),
                Some(BufPool {
                    available_bufs: 0b01,
                    init_value: 0b01,
                })
            );
            assert_eq!(
                allocator.new_bufpool(1),
                Some(BufPool {
                    available_bufs: 0b10,
                    init_value: 0b10,
                })
            );
            assert_eq!(
                allocator.new_bufpool(2),
                Some(BufPool {
                    available_bufs: 0b1100,
                    init_value: 0b1100,
                })
            );
            assert_eq!(
                allocator.remainder_bufpool(),
                Some(BufPool {
                    available_bufs: (0xffff_ffffu64 << 4) as u32,
                    init_value: (0xffff_ffffu64 << 4) as u32,
                })
            );
        }
    }

    #[derive(PartialEq, Eq, Debug, Clone, Copy)]
    pub struct BufPool {
        // bitset of bufs that are currently available for taking with take().
        available_bufs: u32,
        init_value: u32,
    }
    impl BufPool {
        pub const EMPTY: Self = Self {
            available_bufs: 0,
            init_value: 0,
        };

        #[cfg(test)]
        pub const fn new(start_id: usize, len: usize) -> Self {
            assert!(start_id < 32);
            assert!(len > 0);
            assert!(start_id + len <= 32);
            let available_bufs = (((1_u64 << len) - 1) << start_id) as u32;
            Self {
                available_bufs,
                init_value: available_bufs,
            }
        }
        pub fn reset(&mut self) {
            self.available_bufs = self.init_value;
        }
        pub fn take(&mut self) -> Option<BufId> {
            if self.is_empty() {
                return None;
            }
            let buf_id = self.available_bufs.trailing_zeros();
            let mask = 1 << buf_id;
            debug_assert!((self.available_bufs & mask) != 0);
            self.available_bufs &= !mask;
            Some(BufId(buf_id))
        }
        pub fn put(&mut self, buf_id: BufId) {
            let mask = 1 << u32::from(buf_id);
            debug_assert!((self.available_bufs & mask) == 0);
            self.available_bufs |= mask;
        }

        pub fn len(&self) -> usize {
            usize::try_from(self.available_bufs.count_ones()).unwrap()
        }

        pub fn is_empty(&self) -> bool {
            self.available_bufs == 0
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn test_full_size() {
            let mut pool = BufPool::new(0, 32);
            assert_eq!(pool.available_bufs, 0xffff_ffff);
            for i in 0..32 {
                assert_eq!(Some(BufId::from(i)), pool.take());
            }
            assert_eq!(None, pool.take());
            pool.put(5.into());
            pool.put(7.into());
            pool.put(3.into());
            assert_eq!(Some(3.into()), pool.take());
            assert_eq!(Some(5.into()), pool.take());
            assert_eq!(Some(7.into()), pool.take());
            assert_eq!(None, pool.take());
            assert_eq!(None, pool.take());
        }

        #[test]
        fn test_5_bits() {
            let mut pool = BufPool::new(4, 5);
            assert_eq!(pool.available_bufs, 0x0000_01f0);
            assert_eq!(Some(BufId::from(4)), pool.take());
            assert_eq!(Some(BufId::from(5)), pool.take());
            assert_eq!(Some(BufId::from(6)), pool.take());
            assert_eq!(Some(BufId::from(7)), pool.take());
            assert_eq!(Some(BufId::from(8)), pool.take());
            assert_eq!(None, pool.take());

            pool.put(5.into());
            pool.put(6.into());
            assert_eq!(Some(5.into()), pool.take());
            assert_eq!(Some(6.into()), pool.take());
            assert_eq!(None, pool.take());
        }

        #[test]
        fn test_config() {
            assert_eq!(
                UsbConfig::new(
                    &[
                        EpIn {
                            num: 1,
                            buf_pool_size: 3,
                        },
                        EpIn {
                            num: 3,
                            buf_pool_size: 5,
                        },
                    ],
                    &[
                        EpOut {
                            num: 2,
                            set_nak: true
                        },
                        EpOut {
                            num: 4,
                            set_nak: false
                        },
                    ]
                ),
                UsbConfig {
                    buf_pool_setup: BufPool::new(0, 4),
                    buf_pool_out: BufPool::new(4, 12),
                    buf_pools_in: [
                        BufPool::new(24, 8),
                        BufPool::new(16, 3),
                        BufPool::EMPTY,
                        BufPool::new(19, 5),
                        BufPool::EMPTY,
                        BufPool::EMPTY,
                        BufPool::EMPTY,
                        BufPool::EMPTY,
                        BufPool::EMPTY,
                        BufPool::EMPTY,
                        BufPool::EMPTY,
                        BufPool::EMPTY,
                    ],
                    in_mask: 0b01011,
                    out_mask: 0b10101,
                    set_nak_mask: 0b100,
                },
            );
        }

        #[test]
        #[should_panic]
        fn test_config_too_many_set_nak() {
            UsbConfig::new(
                &[EpIn {
                    num: 1,
                    buf_pool_size: 3,
                }],
                &[
                    EpOut {
                        num: 2,
                        set_nak: true,
                    },
                    EpOut {
                        num: 4,
                        set_nak: true,
                    },
                ],
            );
        }
    }
}
