#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct AlertHandler {
    _priv: (),
}
impl AlertHandler {
    pub const PTR: *mut u32 = 0x40150000 as *mut u32;
    #[doc = r" # Safety"]
    #[doc = r""]
    #[doc = r" Caller must ensure that all concurrent use of this"]
    #[doc = r" peripheral in the firmware is done so in a compatible"]
    #[doc = r" way. The simplest way to enforce this is to only call"]
    #[doc = r" this function once."]
    #[inline(always)]
    pub unsafe fn new() -> Self {
        Self { _priv: () }
    }
    #[doc = r" Returns a register block that can be used to read"]
    #[doc = r" registers from this peripheral, but cannot write."]
    #[inline(always)]
    pub fn regs(&self) -> RegisterBlock<ureg::RealMmio<'_>> {
        RegisterBlock {
            ptr: Self::PTR,
            mmio: core::default::Default::default(),
        }
    }
    #[doc = r" Return a register block that can be used to read and"]
    #[doc = r" write this peripheral's registers."]
    #[inline(always)]
    pub fn regs_mut(&mut self) -> RegisterBlock<ureg::RealMmioMut<'_>> {
        RegisterBlock {
            ptr: Self::PTR,
            mmio: core::default::Default::default(),
        }
    }
}
#[derive(Clone, Copy)]
pub struct RegisterBlock<TMmio: ureg::Mmio + core::borrow::Borrow<TMmio>> {
    ptr: *mut u32,
    mmio: TMmio,
}
impl<TMmio: ureg::Mmio + core::default::Default> RegisterBlock<TMmio> {
    #[doc = r" # Safety"]
    #[doc = r""]
    #[doc = r" The caller is responsible for ensuring that ptr is valid for"]
    #[doc = r" volatile reads and writes at any of the offsets in this register"]
    #[doc = r" block."]
    #[inline(always)]
    pub unsafe fn new(ptr: *mut u32) -> Self {
        Self {
            ptr,
            mmio: core::default::Default::default(),
        }
    }
}
impl<TMmio: ureg::Mmio> RegisterBlock<TMmio> {
    #[doc = r" # Safety"]
    #[doc = r""]
    #[doc = r" The caller is responsible for ensuring that ptr is valid for"]
    #[doc = r" volatile reads and writes at any of the offsets in this register"]
    #[doc = r" block."]
    #[inline(always)]
    pub unsafe fn new_with_mmio(ptr: *mut u32, mmio: TMmio) -> Self {
        Self { ptr, mmio }
    }
    #[doc = "Interrupt State Register\n\nRead value: [`regs::IntrStateReadVal`]; Write value: [`regs::IntrStateWriteVal`]"]
    #[inline(always)]
    pub fn intr_state(&self) -> ureg::RegRef<crate::meta::IntrState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Enable Register\n\nRead value: [`regs::IntrEnableReadVal`]; Write value: [`regs::IntrEnableWriteVal`]"]
    #[inline(always)]
    pub fn intr_enable(&self) -> ureg::RegRef<crate::meta::IntrEnable, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Test Register\n\nRead value: [`regs::IntrTestReadVal`]; Write value: [`regs::IntrTestWriteVal`]"]
    #[inline(always)]
    pub fn intr_test(&self) -> ureg::RegRef<crate::meta::IntrTest, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for !!PING_TIMEOUT_CYC_SHADOWED and !!PING_TIMER_EN_SHADOWED.\n\nRead value: [`regs::PingTimerRegwenReadVal`]; Write value: [`regs::PingTimerRegwenWriteVal`]"]
    #[inline(always)]
    pub fn ping_timer_regwen(&self) -> ureg::RegRef<crate::meta::PingTimerRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Ping timeout cycle count.\n\nRead value: [`regs::PingTimeoutCycShadowedReadVal`]; Write value: [`regs::PingTimeoutCycShadowedWriteVal`]"]
    #[inline(always)]
    pub fn ping_timeout_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::PingTimeoutCycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Ping timer enable.\n\nRead value: [`regs::PingTimerEnShadowedReadVal`]; Write value: [`regs::PingTimerEnShadowedWriteVal`]"]
    #[inline(always)]
    pub fn ping_timer_en_shadowed(&self) -> ureg::RegRef<crate::meta::PingTimerEnShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for alert enable bits.\n\nRead value: [`regs::AlertRegwenReadVal`]; Write value: [`regs::AlertRegwenWriteVal`]"]
    #[inline(always)]
    pub fn alert_regwen(&self) -> ureg::Array<65, ureg::RegRef<crate::meta::AlertRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable register for alerts.\n\nRead value: [`regs::AlertEnShadowedReadVal`]; Write value: [`regs::AlertEnShadowedWriteVal`]"]
    #[inline(always)]
    pub fn alert_en_shadowed(
        &self,
    ) -> ureg::Array<65, ureg::RegRef<crate::meta::AlertEnShadowed, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x11c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Class assignment of alerts.\n\nRead value: [`regs::AlertClassShadowedReadVal`]; Write value: [`regs::AlertClassShadowedWriteVal`]"]
    #[inline(always)]
    pub fn alert_class_shadowed(
        &self,
    ) -> ureg::Array<65, ureg::RegRef<crate::meta::AlertClassShadowed, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x220 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Alert Cause Register\n\nRead value: [`regs::AlertCauseReadVal`]; Write value: [`regs::AlertCauseWriteVal`]"]
    #[inline(always)]
    pub fn alert_cause(&self) -> ureg::Array<65, ureg::RegRef<crate::meta::AlertCause, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x324 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for alert enable bits.\n\nRead value: [`regs::LocAlertRegwenReadVal`]; Write value: [`regs::LocAlertRegwenWriteVal`]"]
    #[inline(always)]
    pub fn loc_alert_regwen(
        &self,
    ) -> ureg::Array<7, ureg::RegRef<crate::meta::LocAlertRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x428 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable register for the local alerts\n\"alert pingfail\" (0), \"escalation pingfail\" (1),\n\"alert integfail\" (2), \"escalation integfail\" (3),\n\"bus integrity failure\" (4), \"shadow reg update error\" (5)\nand \"shadow reg storage error\" (6).\n\nRead value: [`regs::LocAlertEnShadowedReadVal`]; Write value: [`regs::LocAlertEnShadowedWriteVal`]"]
    #[inline(always)]
    pub fn loc_alert_en_shadowed(
        &self,
    ) -> ureg::Array<7, ureg::RegRef<crate::meta::LocAlertEnShadowed, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x444 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Class assignment of the local alerts\n\"alert pingfail\" (0), \"escalation pingfail\" (1),\n\"alert integfail\" (2), \"escalation integfail\" (3),\n\"bus integrity failure\" (4), \"shadow reg update error\" (5)\nand \"shadow reg storage error\" (6).\n\nRead value: [`regs::LocAlertClassShadowedReadVal`]; Write value: [`regs::LocAlertClassShadowedWriteVal`]"]
    #[inline(always)]
    pub fn loc_alert_class_shadowed(
        &self,
    ) -> ureg::Array<7, ureg::RegRef<crate::meta::LocAlertClassShadowed, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x460 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Alert Cause Register for the local alerts\n\"alert pingfail\" (0), \"escalation pingfail\" (1),\n\"alert integfail\" (2), \"escalation integfail\" (3),\n\"bus integrity failure\" (4), \"shadow reg update error\" (5)\nand \"shadow reg storage error\" (6).\n\nRead value: [`regs::LocAlertCauseReadVal`]; Write value: [`regs::LocAlertCauseWriteVal`]"]
    #[inline(always)]
    pub fn loc_alert_cause(
        &self,
    ) -> ureg::Array<7, ureg::RegRef<crate::meta::LocAlertCause, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x47c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Lock bit for Class A configuration.\n\nRead value: [`regs::ClassaRegwenReadVal`]; Write value: [`regs::ClassaRegwenWriteVal`]"]
    #[inline(always)]
    pub fn classa_regwen(&self) -> ureg::RegRef<crate::meta::ClassaRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x498 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Escalation control register for alert Class A. Can not be modified if !!CLASSA_REGWEN is false.\n\nRead value: [`regs::ClassaCtrlShadowedReadVal`]; Write value: [`regs::ClassaCtrlShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classa_ctrl_shadowed(&self) -> ureg::RegRef<crate::meta::ClassaCtrlShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x49c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Clear enable for escalation protocol of Class A alerts.\n\nRead value: [`regs::ClassaClrRegwenReadVal`]; Write value: [`regs::ClassaClrRegwenWriteVal`]"]
    #[inline(always)]
    pub fn classa_clr_regwen(&self) -> ureg::RegRef<crate::meta::ClassaClrRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4a0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Clear for escalation protocol of Class A.\n\nRead value: [`regs::ClassaClrShadowedReadVal`]; Write value: [`regs::ClassaClrShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classa_clr_shadowed(&self) -> ureg::RegRef<crate::meta::ClassaClrShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4a4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Current accumulation value for alert Class A. Software can clear this register\nwith a write to !!CLASSA_CLR_SHADOWED register unless !!CLASSA_CLR_REGWEN is false.\n\nRead value: [`regs::ClassaAccumCntReadVal`]; Write value: [`regs::ClassaAccumCntWriteVal`]"]
    #[inline(always)]
    pub fn classa_accum_cnt(&self) -> ureg::RegRef<crate::meta::ClassaAccumCnt, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4a8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Accumulation threshold value for alert Class A.\n\nRead value: [`regs::ClassaAccumThreshShadowedReadVal`]; Write value: [`regs::ClassaAccumThreshShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classa_accum_thresh_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassaAccumThreshShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4ac / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt timeout in cycles.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classa_timeout_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassaTimeoutCycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4b0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Crashdump trigger configuration for Class A.\n\nRead value: [`regs::ClassaCrashdumpTriggerShadowedReadVal`]; Write value: [`regs::ClassaCrashdumpTriggerShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classa_crashdump_trigger_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassaCrashdumpTriggerShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4b4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 0 for Class A.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classa_phase0_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassaPhase0CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4b8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 1 for Class A.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classa_phase1_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassaPhase1CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4bc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 2 for Class A.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classa_phase2_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassaPhase2CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 3 for Class A.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classa_phase3_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassaPhase3CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Escalation counter in cycles for Class A.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classa_esc_cnt(&self) -> ureg::RegRef<crate::meta::ClassaEscCnt, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Current escalation state of Class A. See also !!CLASSA_ESC_CNT.\n\nRead value: [`regs::ClassaStateReadVal`]; Write value: [`regs::ClassaStateWriteVal`]"]
    #[inline(always)]
    pub fn classa_state(&self) -> ureg::RegRef<crate::meta::ClassaState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4cc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Lock bit for Class B configuration.\n\nRead value: [`regs::ClassbRegwenReadVal`]; Write value: [`regs::ClassbRegwenWriteVal`]"]
    #[inline(always)]
    pub fn classb_regwen(&self) -> ureg::RegRef<crate::meta::ClassbRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4d0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Escalation control register for alert Class B. Can not be modified if !!CLASSB_REGWEN is false.\n\nRead value: [`regs::ClassbCtrlShadowedReadVal`]; Write value: [`regs::ClassbCtrlShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classb_ctrl_shadowed(&self) -> ureg::RegRef<crate::meta::ClassbCtrlShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4d4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Clear enable for escalation protocol of Class B alerts.\n\nRead value: [`regs::ClassbClrRegwenReadVal`]; Write value: [`regs::ClassbClrRegwenWriteVal`]"]
    #[inline(always)]
    pub fn classb_clr_regwen(&self) -> ureg::RegRef<crate::meta::ClassbClrRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4d8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Clear for escalation protocol of Class B.\n\nRead value: [`regs::ClassbClrShadowedReadVal`]; Write value: [`regs::ClassbClrShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classb_clr_shadowed(&self) -> ureg::RegRef<crate::meta::ClassbClrShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4dc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Current accumulation value for alert Class B. Software can clear this register\nwith a write to !!CLASSB_CLR_SHADOWED register unless !!CLASSB_CLR_REGWEN is false.\n\nRead value: [`regs::ClassbAccumCntReadVal`]; Write value: [`regs::ClassbAccumCntWriteVal`]"]
    #[inline(always)]
    pub fn classb_accum_cnt(&self) -> ureg::RegRef<crate::meta::ClassbAccumCnt, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4e0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Accumulation threshold value for alert Class B.\n\nRead value: [`regs::ClassbAccumThreshShadowedReadVal`]; Write value: [`regs::ClassbAccumThreshShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classb_accum_thresh_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassbAccumThreshShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4e4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt timeout in cycles.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classb_timeout_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassbTimeoutCycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4e8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Crashdump trigger configuration for Class B.\n\nRead value: [`regs::ClassbCrashdumpTriggerShadowedReadVal`]; Write value: [`regs::ClassbCrashdumpTriggerShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classb_crashdump_trigger_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassbCrashdumpTriggerShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4ec / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 0 for Class B.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classb_phase0_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassbPhase0CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4f0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 1 for Class B.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classb_phase1_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassbPhase1CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4f4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 2 for Class B.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classb_phase2_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassbPhase2CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4f8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 3 for Class B.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classb_phase3_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassbPhase3CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4fc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Escalation counter in cycles for Class B.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classb_esc_cnt(&self) -> ureg::RegRef<crate::meta::ClassbEscCnt, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x500 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Current escalation state of Class B. See also !!CLASSB_ESC_CNT.\n\nRead value: [`regs::ClassbStateReadVal`]; Write value: [`regs::ClassbStateWriteVal`]"]
    #[inline(always)]
    pub fn classb_state(&self) -> ureg::RegRef<crate::meta::ClassbState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x504 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Lock bit for Class C configuration.\n\nRead value: [`regs::ClasscRegwenReadVal`]; Write value: [`regs::ClasscRegwenWriteVal`]"]
    #[inline(always)]
    pub fn classc_regwen(&self) -> ureg::RegRef<crate::meta::ClasscRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x508 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Escalation control register for alert Class C. Can not be modified if !!CLASSC_REGWEN is false.\n\nRead value: [`regs::ClasscCtrlShadowedReadVal`]; Write value: [`regs::ClasscCtrlShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classc_ctrl_shadowed(&self) -> ureg::RegRef<crate::meta::ClasscCtrlShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x50c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Clear enable for escalation protocol of Class C alerts.\n\nRead value: [`regs::ClasscClrRegwenReadVal`]; Write value: [`regs::ClasscClrRegwenWriteVal`]"]
    #[inline(always)]
    pub fn classc_clr_regwen(&self) -> ureg::RegRef<crate::meta::ClasscClrRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x510 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Clear for escalation protocol of Class C.\n\nRead value: [`regs::ClasscClrShadowedReadVal`]; Write value: [`regs::ClasscClrShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classc_clr_shadowed(&self) -> ureg::RegRef<crate::meta::ClasscClrShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x514 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Current accumulation value for alert Class C. Software can clear this register\nwith a write to !!CLASSC_CLR_SHADOWED register unless !!CLASSC_CLR_REGWEN is false.\n\nRead value: [`regs::ClasscAccumCntReadVal`]; Write value: [`regs::ClasscAccumCntWriteVal`]"]
    #[inline(always)]
    pub fn classc_accum_cnt(&self) -> ureg::RegRef<crate::meta::ClasscAccumCnt, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x518 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Accumulation threshold value for alert Class C.\n\nRead value: [`regs::ClasscAccumThreshShadowedReadVal`]; Write value: [`regs::ClasscAccumThreshShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classc_accum_thresh_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClasscAccumThreshShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x51c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt timeout in cycles.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classc_timeout_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClasscTimeoutCycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x520 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Crashdump trigger configuration for Class C.\n\nRead value: [`regs::ClasscCrashdumpTriggerShadowedReadVal`]; Write value: [`regs::ClasscCrashdumpTriggerShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classc_crashdump_trigger_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClasscCrashdumpTriggerShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x524 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 0 for Class C.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classc_phase0_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClasscPhase0CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x528 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 1 for Class C.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classc_phase1_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClasscPhase1CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x52c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 2 for Class C.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classc_phase2_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClasscPhase2CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x530 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 3 for Class C.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classc_phase3_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClasscPhase3CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x534 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Escalation counter in cycles for Class C.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classc_esc_cnt(&self) -> ureg::RegRef<crate::meta::ClasscEscCnt, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x538 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Current escalation state of Class C. See also !!CLASSC_ESC_CNT.\n\nRead value: [`regs::ClasscStateReadVal`]; Write value: [`regs::ClasscStateWriteVal`]"]
    #[inline(always)]
    pub fn classc_state(&self) -> ureg::RegRef<crate::meta::ClasscState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x53c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Lock bit for Class D configuration.\n\nRead value: [`regs::ClassdRegwenReadVal`]; Write value: [`regs::ClassdRegwenWriteVal`]"]
    #[inline(always)]
    pub fn classd_regwen(&self) -> ureg::RegRef<crate::meta::ClassdRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x540 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Escalation control register for alert Class D. Can not be modified if !!CLASSD_REGWEN is false.\n\nRead value: [`regs::ClassdCtrlShadowedReadVal`]; Write value: [`regs::ClassdCtrlShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classd_ctrl_shadowed(&self) -> ureg::RegRef<crate::meta::ClassdCtrlShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x544 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Clear enable for escalation protocol of Class D alerts.\n\nRead value: [`regs::ClassdClrRegwenReadVal`]; Write value: [`regs::ClassdClrRegwenWriteVal`]"]
    #[inline(always)]
    pub fn classd_clr_regwen(&self) -> ureg::RegRef<crate::meta::ClassdClrRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x548 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Clear for escalation protocol of Class D.\n\nRead value: [`regs::ClassdClrShadowedReadVal`]; Write value: [`regs::ClassdClrShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classd_clr_shadowed(&self) -> ureg::RegRef<crate::meta::ClassdClrShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x54c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Current accumulation value for alert Class D. Software can clear this register\nwith a write to !!CLASSD_CLR_SHADOWED register unless !!CLASSD_CLR_REGWEN is false.\n\nRead value: [`regs::ClassdAccumCntReadVal`]; Write value: [`regs::ClassdAccumCntWriteVal`]"]
    #[inline(always)]
    pub fn classd_accum_cnt(&self) -> ureg::RegRef<crate::meta::ClassdAccumCnt, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x550 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Accumulation threshold value for alert Class D.\n\nRead value: [`regs::ClassdAccumThreshShadowedReadVal`]; Write value: [`regs::ClassdAccumThreshShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classd_accum_thresh_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassdAccumThreshShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x554 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt timeout in cycles.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classd_timeout_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassdTimeoutCycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x558 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Crashdump trigger configuration for Class D.\n\nRead value: [`regs::ClassdCrashdumpTriggerShadowedReadVal`]; Write value: [`regs::ClassdCrashdumpTriggerShadowedWriteVal`]"]
    #[inline(always)]
    pub fn classd_crashdump_trigger_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassdCrashdumpTriggerShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x55c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 0 for Class D.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classd_phase0_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassdPhase0CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x560 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 1 for Class D.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classd_phase1_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassdPhase1CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x564 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 2 for Class D.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classd_phase2_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassdPhase2CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x568 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Duration of escalation phase 3 for Class D.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classd_phase3_cyc_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ClassdPhase3CycShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x56c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Escalation counter in cycles for Class D.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn classd_esc_cnt(&self) -> ureg::RegRef<crate::meta::ClassdEscCnt, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x570 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Current escalation state of Class D. See also !!CLASSD_ESC_CNT.\n\nRead value: [`regs::ClassdStateReadVal`]; Write value: [`regs::ClassdStateWriteVal`]"]
    #[inline(always)]
    pub fn classd_state(&self) -> ureg::RegRef<crate::meta::ClassdState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x574 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
}
pub mod regs {
    #![doc = r" Types that represent the values held by registers."]
    #[derive(Clone, Copy)]
    pub struct AlertCauseReadVal(u32);
    impl AlertCauseReadVal {
        #[doc = "Cause bit "]
        #[inline(always)]
        pub fn a(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AlertCauseWriteVal {
            AlertCauseWriteVal(self.0)
        }
    }
    impl From<u32> for AlertCauseReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertCauseReadVal> for u32 {
        #[inline(always)]
        fn from(val: AlertCauseReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertCauseWriteVal(u32);
    impl AlertCauseWriteVal {
        #[doc = "Cause bit "]
        #[inline(always)]
        pub fn a_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
    }
    impl From<u32> for AlertCauseWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertCauseWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AlertCauseWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertClassShadowedReadVal(u32);
    impl AlertClassShadowedReadVal {
        #[doc = "Classification "]
        #[inline(always)]
        pub fn class_a(&self) -> super::enums::Class {
            super::enums::Class::try_from((self.0 >> 0) & 3).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AlertClassShadowedWriteVal {
            AlertClassShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for AlertClassShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertClassShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: AlertClassShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertClassShadowedWriteVal(u32);
    impl AlertClassShadowedWriteVal {
        #[doc = "Classification "]
        #[inline(always)]
        pub fn class_a(
            self,
            f: impl FnOnce(super::enums::selector::ClassSelector) -> super::enums::Class,
        ) -> Self {
            Self(
                (self.0 & !(3 << 0)) | (u32::from(f(super::enums::selector::ClassSelector())) << 0),
            )
        }
    }
    impl From<u32> for AlertClassShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertClassShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AlertClassShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertEnShadowedReadVal(u32);
    impl AlertEnShadowedReadVal {
        #[doc = "Alert enable bit.\n\nNote that the alert pinging mechanism will only ping alerts that have been enabled and locked."]
        #[inline(always)]
        pub fn en_a(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AlertEnShadowedWriteVal {
            AlertEnShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for AlertEnShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertEnShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: AlertEnShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertEnShadowedWriteVal(u32);
    impl AlertEnShadowedWriteVal {
        #[doc = "Alert enable bit.\n\nNote that the alert pinging mechanism will only ping alerts that have been enabled and locked."]
        #[inline(always)]
        pub fn en_a(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for AlertEnShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertEnShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AlertEnShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertRegwenReadVal(u32);
    impl AlertRegwenReadVal {
        #[doc = "Alert configuration write enable bit.\nIf this is cleared to 0, the corresponding !!ALERT_EN_SHADOWED\nand !!ALERT_CLASS_SHADOWED bits are not writable anymore.\n\nNote that the alert pinging mechanism will only ping alerts that have been enabled and locked."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AlertRegwenWriteVal {
            AlertRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for AlertRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: AlertRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertRegwenWriteVal(u32);
    impl AlertRegwenWriteVal {
        #[doc = "Alert configuration write enable bit.\nIf this is cleared to 0, the corresponding !!ALERT_EN_SHADOWED\nand !!ALERT_CLASS_SHADOWED bits are not writable anymore.\n\nNote that the alert pinging mechanism will only ping alerts that have been enabled and locked."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for AlertRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AlertRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaAccumCntReadVal(u32);
    impl ClassaAccumCntReadVal {
        #[inline(always)]
        pub fn classa_accum_cnt(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
    }
    impl From<u32> for ClassaAccumCntReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaAccumCntReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaAccumCntReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaAccumThreshShadowedReadVal(u32);
    impl ClassaAccumThreshShadowedReadVal {
        #[doc = "Once the accumulation value register is equal to the threshold escalation will\nbe triggered on the next alert occurrence within this class A begins. Note that this\nregister can not be modified if !!CLASSA_REGWEN is false."]
        #[inline(always)]
        pub fn classa_accum_thresh_shadowed(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassaAccumThreshShadowedWriteVal {
            ClassaAccumThreshShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClassaAccumThreshShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaAccumThreshShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaAccumThreshShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaAccumThreshShadowedWriteVal(u32);
    impl ClassaAccumThreshShadowedWriteVal {
        #[doc = "Once the accumulation value register is equal to the threshold escalation will\nbe triggered on the next alert occurrence within this class A begins. Note that this\nregister can not be modified if !!CLASSA_REGWEN is false."]
        #[inline(always)]
        pub fn classa_accum_thresh_shadowed(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for ClassaAccumThreshShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaAccumThreshShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaAccumThreshShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaClrRegwenReadVal(u32);
    impl ClassaClrRegwenReadVal {
        #[doc = "Register defaults to true, can only be cleared. This register is set\nto false by the hardware if the escalation protocol has been triggered and the bit\n!!CLASSA_CTRL_SHADOWED.LOCK is true."]
        #[inline(always)]
        pub fn classa_clr_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassaClrRegwenWriteVal {
            ClassaClrRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ClassaClrRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaClrRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaClrRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaClrRegwenWriteVal(u32);
    impl ClassaClrRegwenWriteVal {
        #[doc = "Register defaults to true, can only be cleared. This register is set\nto false by the hardware if the escalation protocol has been triggered and the bit\n!!CLASSA_CTRL_SHADOWED.LOCK is true."]
        #[inline(always)]
        pub fn classa_clr_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ClassaClrRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaClrRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaClrRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaClrShadowedReadVal(u32);
    impl ClassaClrShadowedReadVal {
        #[doc = "Writing 1 to this register clears the accumulator and aborts escalation\n(if it has been triggered). This clear is disabled if !!CLASSA_CLR_REGWEN is false."]
        #[inline(always)]
        pub fn classa_clr_shadowed(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassaClrShadowedWriteVal {
            ClassaClrShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClassaClrShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaClrShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaClrShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaClrShadowedWriteVal(u32);
    impl ClassaClrShadowedWriteVal {
        #[doc = "Writing 1 to this register clears the accumulator and aborts escalation\n(if it has been triggered). This clear is disabled if !!CLASSA_CLR_REGWEN is false."]
        #[inline(always)]
        pub fn classa_clr_shadowed(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for ClassaClrShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaClrShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaClrShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaCrashdumpTriggerShadowedReadVal(u32);
    impl ClassaCrashdumpTriggerShadowedReadVal {
        #[doc = "Determine in which escalation phase to capture the crashdump containing all alert cause CSRs and escalation\ntimer states. It is recommended to capture the crashdump upon entering the first escalation phase\nthat activates a countermeasure with many side-effects (e.g. life cycle state scrapping) in order\nto prevent spurious alert events from masking the original alert causes.\nNote that this register can not be modified if !!CLASSA_REGWEN is false."]
        #[inline(always)]
        pub fn classa_crashdump_trigger_shadowed(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassaCrashdumpTriggerShadowedWriteVal {
            ClassaCrashdumpTriggerShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClassaCrashdumpTriggerShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaCrashdumpTriggerShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaCrashdumpTriggerShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaCrashdumpTriggerShadowedWriteVal(u32);
    impl ClassaCrashdumpTriggerShadowedWriteVal {
        #[doc = "Determine in which escalation phase to capture the crashdump containing all alert cause CSRs and escalation\ntimer states. It is recommended to capture the crashdump upon entering the first escalation phase\nthat activates a countermeasure with many side-effects (e.g. life cycle state scrapping) in order\nto prevent spurious alert events from masking the original alert causes.\nNote that this register can not be modified if !!CLASSA_REGWEN is false."]
        #[inline(always)]
        pub fn classa_crashdump_trigger_shadowed(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for ClassaCrashdumpTriggerShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaCrashdumpTriggerShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaCrashdumpTriggerShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaCtrlShadowedReadVal(u32);
    impl ClassaCtrlShadowedReadVal {
        #[doc = "Enable escalation mechanisms (accumulation and\ninterrupt timeout) for Class A. Note that interrupts can fire\nregardless of whether the escalation mechanisms are enabled for\nthis class or not."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable automatic locking of escalation counter for class A.\nIf true, there is no way to stop the escalation protocol for class A\nonce it has been triggered."]
        #[inline(always)]
        pub fn lock(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable escalation signal 0 for Class A"]
        #[inline(always)]
        pub fn en_e0(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Enable escalation signal 1 for Class A"]
        #[inline(always)]
        pub fn en_e1(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Enable escalation signal 2 for Class A"]
        #[inline(always)]
        pub fn en_e2(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Enable escalation signal 3 for Class A"]
        #[inline(always)]
        pub fn en_e3(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Determines in which escalation phase escalation signal 0 shall be asserted."]
        #[inline(always)]
        pub fn map_e0(&self) -> u32 {
            (self.0 >> 6) & 3
        }
        #[doc = "Determines in which escalation phase escalation signal 1 shall be asserted."]
        #[inline(always)]
        pub fn map_e1(&self) -> u32 {
            (self.0 >> 8) & 3
        }
        #[doc = "Determines in which escalation phase escalation signal 2 shall be asserted."]
        #[inline(always)]
        pub fn map_e2(&self) -> u32 {
            (self.0 >> 10) & 3
        }
        #[doc = "Determines in which escalation phase escalation signal 3 shall be asserted."]
        #[inline(always)]
        pub fn map_e3(&self) -> u32 {
            (self.0 >> 12) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassaCtrlShadowedWriteVal {
            ClassaCtrlShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClassaCtrlShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaCtrlShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaCtrlShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaCtrlShadowedWriteVal(u32);
    impl ClassaCtrlShadowedWriteVal {
        #[doc = "Enable escalation mechanisms (accumulation and\ninterrupt timeout) for Class A. Note that interrupts can fire\nregardless of whether the escalation mechanisms are enabled for\nthis class or not."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable automatic locking of escalation counter for class A.\nIf true, there is no way to stop the escalation protocol for class A\nonce it has been triggered."]
        #[inline(always)]
        pub fn lock(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable escalation signal 0 for Class A"]
        #[inline(always)]
        pub fn en_e0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Enable escalation signal 1 for Class A"]
        #[inline(always)]
        pub fn en_e1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Enable escalation signal 2 for Class A"]
        #[inline(always)]
        pub fn en_e2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Enable escalation signal 3 for Class A"]
        #[inline(always)]
        pub fn en_e3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Determines in which escalation phase escalation signal 0 shall be asserted."]
        #[inline(always)]
        pub fn map_e0(self, val: u32) -> Self {
            Self((self.0 & !(3 << 6)) | ((val & 3) << 6))
        }
        #[doc = "Determines in which escalation phase escalation signal 1 shall be asserted."]
        #[inline(always)]
        pub fn map_e1(self, val: u32) -> Self {
            Self((self.0 & !(3 << 8)) | ((val & 3) << 8))
        }
        #[doc = "Determines in which escalation phase escalation signal 2 shall be asserted."]
        #[inline(always)]
        pub fn map_e2(self, val: u32) -> Self {
            Self((self.0 & !(3 << 10)) | ((val & 3) << 10))
        }
        #[doc = "Determines in which escalation phase escalation signal 3 shall be asserted."]
        #[inline(always)]
        pub fn map_e3(self, val: u32) -> Self {
            Self((self.0 & !(3 << 12)) | ((val & 3) << 12))
        }
    }
    impl From<u32> for ClassaCtrlShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaCtrlShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaCtrlShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaRegwenReadVal(u32);
    impl ClassaRegwenReadVal {
        #[doc = "Class configuration enable bit.\nIf this is cleared to 0, the corresponding class configuration\nregisters cannot be written anymore."]
        #[inline(always)]
        pub fn classa_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassaRegwenWriteVal {
            ClassaRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ClassaRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaRegwenWriteVal(u32);
    impl ClassaRegwenWriteVal {
        #[doc = "Class configuration enable bit.\nIf this is cleared to 0, the corresponding class configuration\nregisters cannot be written anymore."]
        #[inline(always)]
        pub fn classa_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ClassaRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassaStateReadVal(u32);
    impl ClassaStateReadVal {
        #[inline(always)]
        pub fn classa_state(&self) -> super::enums::ClassxState {
            super::enums::ClassxState::try_from((self.0 >> 0) & 7).unwrap()
        }
    }
    impl From<u32> for ClassaStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassaStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassaStateReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbAccumCntReadVal(u32);
    impl ClassbAccumCntReadVal {
        #[inline(always)]
        pub fn classb_accum_cnt(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
    }
    impl From<u32> for ClassbAccumCntReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbAccumCntReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbAccumCntReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbAccumThreshShadowedReadVal(u32);
    impl ClassbAccumThreshShadowedReadVal {
        #[doc = "Once the accumulation value register is equal to the threshold escalation will\nbe triggered on the next alert occurrence within this class B begins. Note that this\nregister can not be modified if !!CLASSB_REGWEN is false."]
        #[inline(always)]
        pub fn classb_accum_thresh_shadowed(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassbAccumThreshShadowedWriteVal {
            ClassbAccumThreshShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClassbAccumThreshShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbAccumThreshShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbAccumThreshShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbAccumThreshShadowedWriteVal(u32);
    impl ClassbAccumThreshShadowedWriteVal {
        #[doc = "Once the accumulation value register is equal to the threshold escalation will\nbe triggered on the next alert occurrence within this class B begins. Note that this\nregister can not be modified if !!CLASSB_REGWEN is false."]
        #[inline(always)]
        pub fn classb_accum_thresh_shadowed(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for ClassbAccumThreshShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbAccumThreshShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbAccumThreshShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbClrRegwenReadVal(u32);
    impl ClassbClrRegwenReadVal {
        #[doc = "Register defaults to true, can only be cleared. This register is set\nto false by the hardware if the escalation protocol has been triggered and the bit\n!!CLASSB_CTRL_SHADOWED.LOCK is true."]
        #[inline(always)]
        pub fn classb_clr_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassbClrRegwenWriteVal {
            ClassbClrRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ClassbClrRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbClrRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbClrRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbClrRegwenWriteVal(u32);
    impl ClassbClrRegwenWriteVal {
        #[doc = "Register defaults to true, can only be cleared. This register is set\nto false by the hardware if the escalation protocol has been triggered and the bit\n!!CLASSB_CTRL_SHADOWED.LOCK is true."]
        #[inline(always)]
        pub fn classb_clr_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ClassbClrRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbClrRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbClrRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbClrShadowedReadVal(u32);
    impl ClassbClrShadowedReadVal {
        #[doc = "Writing 1 to this register clears the accumulator and aborts escalation\n(if it has been triggered). This clear is disabled if !!CLASSB_CLR_REGWEN is false."]
        #[inline(always)]
        pub fn classb_clr_shadowed(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassbClrShadowedWriteVal {
            ClassbClrShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClassbClrShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbClrShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbClrShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbClrShadowedWriteVal(u32);
    impl ClassbClrShadowedWriteVal {
        #[doc = "Writing 1 to this register clears the accumulator and aborts escalation\n(if it has been triggered). This clear is disabled if !!CLASSB_CLR_REGWEN is false."]
        #[inline(always)]
        pub fn classb_clr_shadowed(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for ClassbClrShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbClrShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbClrShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbCrashdumpTriggerShadowedReadVal(u32);
    impl ClassbCrashdumpTriggerShadowedReadVal {
        #[doc = "Determine in which escalation phase to capture the crashdump containing all alert cause CSRs and escalation\ntimer states. It is recommended to capture the crashdump upon entering the first escalation phase\nthat activates a countermeasure with many side-effects (e.g. life cycle state scrapping) in order\nto prevent spurious alert events from masking the original alert causes.\nNote that this register can not be modified if !!CLASSB_REGWEN is false."]
        #[inline(always)]
        pub fn classb_crashdump_trigger_shadowed(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassbCrashdumpTriggerShadowedWriteVal {
            ClassbCrashdumpTriggerShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClassbCrashdumpTriggerShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbCrashdumpTriggerShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbCrashdumpTriggerShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbCrashdumpTriggerShadowedWriteVal(u32);
    impl ClassbCrashdumpTriggerShadowedWriteVal {
        #[doc = "Determine in which escalation phase to capture the crashdump containing all alert cause CSRs and escalation\ntimer states. It is recommended to capture the crashdump upon entering the first escalation phase\nthat activates a countermeasure with many side-effects (e.g. life cycle state scrapping) in order\nto prevent spurious alert events from masking the original alert causes.\nNote that this register can not be modified if !!CLASSB_REGWEN is false."]
        #[inline(always)]
        pub fn classb_crashdump_trigger_shadowed(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for ClassbCrashdumpTriggerShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbCrashdumpTriggerShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbCrashdumpTriggerShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbCtrlShadowedReadVal(u32);
    impl ClassbCtrlShadowedReadVal {
        #[doc = "Enable escalation mechanisms (accumulation and\ninterrupt timeout) for Class B. Note that interrupts can fire\nregardless of whether the escalation mechanisms are enabled for\nthis class or not."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable automatic locking of escalation counter for class B.\nIf true, there is no way to stop the escalation protocol for class B\nonce it has been triggered."]
        #[inline(always)]
        pub fn lock(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable escalation signal 0 for Class B"]
        #[inline(always)]
        pub fn en_e0(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Enable escalation signal 1 for Class B"]
        #[inline(always)]
        pub fn en_e1(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Enable escalation signal 2 for Class B"]
        #[inline(always)]
        pub fn en_e2(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Enable escalation signal 3 for Class B"]
        #[inline(always)]
        pub fn en_e3(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Determines in which escalation phase escalation signal 0 shall be asserted."]
        #[inline(always)]
        pub fn map_e0(&self) -> u32 {
            (self.0 >> 6) & 3
        }
        #[doc = "Determines in which escalation phase escalation signal 1 shall be asserted."]
        #[inline(always)]
        pub fn map_e1(&self) -> u32 {
            (self.0 >> 8) & 3
        }
        #[doc = "Determines in which escalation phase escalation signal 2 shall be asserted."]
        #[inline(always)]
        pub fn map_e2(&self) -> u32 {
            (self.0 >> 10) & 3
        }
        #[doc = "Determines in which escalation phase escalation signal 3 shall be asserted."]
        #[inline(always)]
        pub fn map_e3(&self) -> u32 {
            (self.0 >> 12) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassbCtrlShadowedWriteVal {
            ClassbCtrlShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClassbCtrlShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbCtrlShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbCtrlShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbCtrlShadowedWriteVal(u32);
    impl ClassbCtrlShadowedWriteVal {
        #[doc = "Enable escalation mechanisms (accumulation and\ninterrupt timeout) for Class B. Note that interrupts can fire\nregardless of whether the escalation mechanisms are enabled for\nthis class or not."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable automatic locking of escalation counter for class B.\nIf true, there is no way to stop the escalation protocol for class B\nonce it has been triggered."]
        #[inline(always)]
        pub fn lock(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable escalation signal 0 for Class B"]
        #[inline(always)]
        pub fn en_e0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Enable escalation signal 1 for Class B"]
        #[inline(always)]
        pub fn en_e1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Enable escalation signal 2 for Class B"]
        #[inline(always)]
        pub fn en_e2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Enable escalation signal 3 for Class B"]
        #[inline(always)]
        pub fn en_e3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Determines in which escalation phase escalation signal 0 shall be asserted."]
        #[inline(always)]
        pub fn map_e0(self, val: u32) -> Self {
            Self((self.0 & !(3 << 6)) | ((val & 3) << 6))
        }
        #[doc = "Determines in which escalation phase escalation signal 1 shall be asserted."]
        #[inline(always)]
        pub fn map_e1(self, val: u32) -> Self {
            Self((self.0 & !(3 << 8)) | ((val & 3) << 8))
        }
        #[doc = "Determines in which escalation phase escalation signal 2 shall be asserted."]
        #[inline(always)]
        pub fn map_e2(self, val: u32) -> Self {
            Self((self.0 & !(3 << 10)) | ((val & 3) << 10))
        }
        #[doc = "Determines in which escalation phase escalation signal 3 shall be asserted."]
        #[inline(always)]
        pub fn map_e3(self, val: u32) -> Self {
            Self((self.0 & !(3 << 12)) | ((val & 3) << 12))
        }
    }
    impl From<u32> for ClassbCtrlShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbCtrlShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbCtrlShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbRegwenReadVal(u32);
    impl ClassbRegwenReadVal {
        #[doc = "Class configuration enable bit.\nIf this is cleared to 0, the corresponding class configuration\nregisters cannot be written anymore."]
        #[inline(always)]
        pub fn classb_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassbRegwenWriteVal {
            ClassbRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ClassbRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbRegwenWriteVal(u32);
    impl ClassbRegwenWriteVal {
        #[doc = "Class configuration enable bit.\nIf this is cleared to 0, the corresponding class configuration\nregisters cannot be written anymore."]
        #[inline(always)]
        pub fn classb_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ClassbRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassbStateReadVal(u32);
    impl ClassbStateReadVal {
        #[inline(always)]
        pub fn classb_state(&self) -> super::enums::ClassxState {
            super::enums::ClassxState::try_from((self.0 >> 0) & 7).unwrap()
        }
    }
    impl From<u32> for ClassbStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassbStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassbStateReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscAccumCntReadVal(u32);
    impl ClasscAccumCntReadVal {
        #[inline(always)]
        pub fn classc_accum_cnt(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
    }
    impl From<u32> for ClasscAccumCntReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscAccumCntReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscAccumCntReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscAccumThreshShadowedReadVal(u32);
    impl ClasscAccumThreshShadowedReadVal {
        #[doc = "Once the accumulation value register is equal to the threshold escalation will\nbe triggered on the next alert occurrence within this class C begins. Note that this\nregister can not be modified if !!CLASSC_REGWEN is false."]
        #[inline(always)]
        pub fn classc_accum_thresh_shadowed(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClasscAccumThreshShadowedWriteVal {
            ClasscAccumThreshShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClasscAccumThreshShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscAccumThreshShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscAccumThreshShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscAccumThreshShadowedWriteVal(u32);
    impl ClasscAccumThreshShadowedWriteVal {
        #[doc = "Once the accumulation value register is equal to the threshold escalation will\nbe triggered on the next alert occurrence within this class C begins. Note that this\nregister can not be modified if !!CLASSC_REGWEN is false."]
        #[inline(always)]
        pub fn classc_accum_thresh_shadowed(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for ClasscAccumThreshShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscAccumThreshShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscAccumThreshShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscClrRegwenReadVal(u32);
    impl ClasscClrRegwenReadVal {
        #[doc = "Register defaults to true, can only be cleared. This register is set\nto false by the hardware if the escalation protocol has been triggered and the bit\n!!CLASSC_CTRL_SHADOWED.LOCK is true."]
        #[inline(always)]
        pub fn classc_clr_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClasscClrRegwenWriteVal {
            ClasscClrRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ClasscClrRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscClrRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscClrRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscClrRegwenWriteVal(u32);
    impl ClasscClrRegwenWriteVal {
        #[doc = "Register defaults to true, can only be cleared. This register is set\nto false by the hardware if the escalation protocol has been triggered and the bit\n!!CLASSC_CTRL_SHADOWED.LOCK is true."]
        #[inline(always)]
        pub fn classc_clr_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ClasscClrRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscClrRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscClrRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscClrShadowedReadVal(u32);
    impl ClasscClrShadowedReadVal {
        #[doc = "Writing 1 to this register clears the accumulator and aborts escalation\n(if it has been triggered). This clear is disabled if !!CLASSC_CLR_REGWEN is false."]
        #[inline(always)]
        pub fn classc_clr_shadowed(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClasscClrShadowedWriteVal {
            ClasscClrShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClasscClrShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscClrShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscClrShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscClrShadowedWriteVal(u32);
    impl ClasscClrShadowedWriteVal {
        #[doc = "Writing 1 to this register clears the accumulator and aborts escalation\n(if it has been triggered). This clear is disabled if !!CLASSC_CLR_REGWEN is false."]
        #[inline(always)]
        pub fn classc_clr_shadowed(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for ClasscClrShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscClrShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscClrShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscCrashdumpTriggerShadowedReadVal(u32);
    impl ClasscCrashdumpTriggerShadowedReadVal {
        #[doc = "Determine in which escalation phase to capture the crashdump containing all alert cause CSRs and escalation\ntimer states. It is recommended to capture the crashdump upon entering the first escalation phase\nthat activates a countermeasure with many side-effects (e.g. life cycle state scrapping) in order\nto prevent spurious alert events from masking the original alert causes.\nNote that this register can not be modified if !!CLASSC_REGWEN is false."]
        #[inline(always)]
        pub fn classc_crashdump_trigger_shadowed(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClasscCrashdumpTriggerShadowedWriteVal {
            ClasscCrashdumpTriggerShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClasscCrashdumpTriggerShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscCrashdumpTriggerShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscCrashdumpTriggerShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscCrashdumpTriggerShadowedWriteVal(u32);
    impl ClasscCrashdumpTriggerShadowedWriteVal {
        #[doc = "Determine in which escalation phase to capture the crashdump containing all alert cause CSRs and escalation\ntimer states. It is recommended to capture the crashdump upon entering the first escalation phase\nthat activates a countermeasure with many side-effects (e.g. life cycle state scrapping) in order\nto prevent spurious alert events from masking the original alert causes.\nNote that this register can not be modified if !!CLASSC_REGWEN is false."]
        #[inline(always)]
        pub fn classc_crashdump_trigger_shadowed(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for ClasscCrashdumpTriggerShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscCrashdumpTriggerShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscCrashdumpTriggerShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscCtrlShadowedReadVal(u32);
    impl ClasscCtrlShadowedReadVal {
        #[doc = "Enable escalation mechanisms (accumulation and\ninterrupt timeout) for Class C. Note that interrupts can fire\nregardless of whether the escalation mechanisms are enabled for\nthis class or not."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable automatic locking of escalation counter for class C.\nIf true, there is no way to stop the escalation protocol for class C\nonce it has been triggered."]
        #[inline(always)]
        pub fn lock(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable escalation signal 0 for Class C"]
        #[inline(always)]
        pub fn en_e0(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Enable escalation signal 1 for Class C"]
        #[inline(always)]
        pub fn en_e1(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Enable escalation signal 2 for Class C"]
        #[inline(always)]
        pub fn en_e2(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Enable escalation signal 3 for Class C"]
        #[inline(always)]
        pub fn en_e3(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Determines in which escalation phase escalation signal 0 shall be asserted."]
        #[inline(always)]
        pub fn map_e0(&self) -> u32 {
            (self.0 >> 6) & 3
        }
        #[doc = "Determines in which escalation phase escalation signal 1 shall be asserted."]
        #[inline(always)]
        pub fn map_e1(&self) -> u32 {
            (self.0 >> 8) & 3
        }
        #[doc = "Determines in which escalation phase escalation signal 2 shall be asserted."]
        #[inline(always)]
        pub fn map_e2(&self) -> u32 {
            (self.0 >> 10) & 3
        }
        #[doc = "Determines in which escalation phase escalation signal 3 shall be asserted."]
        #[inline(always)]
        pub fn map_e3(&self) -> u32 {
            (self.0 >> 12) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClasscCtrlShadowedWriteVal {
            ClasscCtrlShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClasscCtrlShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscCtrlShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscCtrlShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscCtrlShadowedWriteVal(u32);
    impl ClasscCtrlShadowedWriteVal {
        #[doc = "Enable escalation mechanisms (accumulation and\ninterrupt timeout) for Class C. Note that interrupts can fire\nregardless of whether the escalation mechanisms are enabled for\nthis class or not."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable automatic locking of escalation counter for class C.\nIf true, there is no way to stop the escalation protocol for class C\nonce it has been triggered."]
        #[inline(always)]
        pub fn lock(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable escalation signal 0 for Class C"]
        #[inline(always)]
        pub fn en_e0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Enable escalation signal 1 for Class C"]
        #[inline(always)]
        pub fn en_e1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Enable escalation signal 2 for Class C"]
        #[inline(always)]
        pub fn en_e2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Enable escalation signal 3 for Class C"]
        #[inline(always)]
        pub fn en_e3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Determines in which escalation phase escalation signal 0 shall be asserted."]
        #[inline(always)]
        pub fn map_e0(self, val: u32) -> Self {
            Self((self.0 & !(3 << 6)) | ((val & 3) << 6))
        }
        #[doc = "Determines in which escalation phase escalation signal 1 shall be asserted."]
        #[inline(always)]
        pub fn map_e1(self, val: u32) -> Self {
            Self((self.0 & !(3 << 8)) | ((val & 3) << 8))
        }
        #[doc = "Determines in which escalation phase escalation signal 2 shall be asserted."]
        #[inline(always)]
        pub fn map_e2(self, val: u32) -> Self {
            Self((self.0 & !(3 << 10)) | ((val & 3) << 10))
        }
        #[doc = "Determines in which escalation phase escalation signal 3 shall be asserted."]
        #[inline(always)]
        pub fn map_e3(self, val: u32) -> Self {
            Self((self.0 & !(3 << 12)) | ((val & 3) << 12))
        }
    }
    impl From<u32> for ClasscCtrlShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscCtrlShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscCtrlShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscRegwenReadVal(u32);
    impl ClasscRegwenReadVal {
        #[doc = "Class configuration enable bit.\nIf this is cleared to 0, the corresponding class configuration\nregisters cannot be written anymore."]
        #[inline(always)]
        pub fn classc_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClasscRegwenWriteVal {
            ClasscRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ClasscRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscRegwenWriteVal(u32);
    impl ClasscRegwenWriteVal {
        #[doc = "Class configuration enable bit.\nIf this is cleared to 0, the corresponding class configuration\nregisters cannot be written anymore."]
        #[inline(always)]
        pub fn classc_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ClasscRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClasscStateReadVal(u32);
    impl ClasscStateReadVal {
        #[inline(always)]
        pub fn classc_state(&self) -> super::enums::ClassxState {
            super::enums::ClassxState::try_from((self.0 >> 0) & 7).unwrap()
        }
    }
    impl From<u32> for ClasscStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClasscStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClasscStateReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdAccumCntReadVal(u32);
    impl ClassdAccumCntReadVal {
        #[inline(always)]
        pub fn classd_accum_cnt(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
    }
    impl From<u32> for ClassdAccumCntReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdAccumCntReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdAccumCntReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdAccumThreshShadowedReadVal(u32);
    impl ClassdAccumThreshShadowedReadVal {
        #[doc = "Once the accumulation value register is equal to the threshold escalation will\nbe triggered on the next alert occurrence within this class D begins. Note that this\nregister can not be modified if !!CLASSD_REGWEN is false."]
        #[inline(always)]
        pub fn classd_accum_thresh_shadowed(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassdAccumThreshShadowedWriteVal {
            ClassdAccumThreshShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClassdAccumThreshShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdAccumThreshShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdAccumThreshShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdAccumThreshShadowedWriteVal(u32);
    impl ClassdAccumThreshShadowedWriteVal {
        #[doc = "Once the accumulation value register is equal to the threshold escalation will\nbe triggered on the next alert occurrence within this class D begins. Note that this\nregister can not be modified if !!CLASSD_REGWEN is false."]
        #[inline(always)]
        pub fn classd_accum_thresh_shadowed(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for ClassdAccumThreshShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdAccumThreshShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdAccumThreshShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdClrRegwenReadVal(u32);
    impl ClassdClrRegwenReadVal {
        #[doc = "Register defaults to true, can only be cleared. This register is set\nto false by the hardware if the escalation protocol has been triggered and the bit\n!!CLASSD_CTRL_SHADOWED.LOCK is true."]
        #[inline(always)]
        pub fn classd_clr_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassdClrRegwenWriteVal {
            ClassdClrRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ClassdClrRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdClrRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdClrRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdClrRegwenWriteVal(u32);
    impl ClassdClrRegwenWriteVal {
        #[doc = "Register defaults to true, can only be cleared. This register is set\nto false by the hardware if the escalation protocol has been triggered and the bit\n!!CLASSD_CTRL_SHADOWED.LOCK is true."]
        #[inline(always)]
        pub fn classd_clr_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ClassdClrRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdClrRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdClrRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdClrShadowedReadVal(u32);
    impl ClassdClrShadowedReadVal {
        #[doc = "Writing 1 to this register clears the accumulator and aborts escalation\n(if it has been triggered). This clear is disabled if !!CLASSD_CLR_REGWEN is false."]
        #[inline(always)]
        pub fn classd_clr_shadowed(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassdClrShadowedWriteVal {
            ClassdClrShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClassdClrShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdClrShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdClrShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdClrShadowedWriteVal(u32);
    impl ClassdClrShadowedWriteVal {
        #[doc = "Writing 1 to this register clears the accumulator and aborts escalation\n(if it has been triggered). This clear is disabled if !!CLASSD_CLR_REGWEN is false."]
        #[inline(always)]
        pub fn classd_clr_shadowed(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for ClassdClrShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdClrShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdClrShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdCrashdumpTriggerShadowedReadVal(u32);
    impl ClassdCrashdumpTriggerShadowedReadVal {
        #[doc = "Determine in which escalation phase to capture the crashdump containing all alert cause CSRs and escalation\ntimer states. It is recommended to capture the crashdump upon entering the first escalation phase\nthat activates a countermeasure with many side-effects (e.g. life cycle state scrapping) in order\nto prevent spurious alert events from masking the original alert causes.\nNote that this register can not be modified if !!CLASSD_REGWEN is false."]
        #[inline(always)]
        pub fn classd_crashdump_trigger_shadowed(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassdCrashdumpTriggerShadowedWriteVal {
            ClassdCrashdumpTriggerShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClassdCrashdumpTriggerShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdCrashdumpTriggerShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdCrashdumpTriggerShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdCrashdumpTriggerShadowedWriteVal(u32);
    impl ClassdCrashdumpTriggerShadowedWriteVal {
        #[doc = "Determine in which escalation phase to capture the crashdump containing all alert cause CSRs and escalation\ntimer states. It is recommended to capture the crashdump upon entering the first escalation phase\nthat activates a countermeasure with many side-effects (e.g. life cycle state scrapping) in order\nto prevent spurious alert events from masking the original alert causes.\nNote that this register can not be modified if !!CLASSD_REGWEN is false."]
        #[inline(always)]
        pub fn classd_crashdump_trigger_shadowed(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for ClassdCrashdumpTriggerShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdCrashdumpTriggerShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdCrashdumpTriggerShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdCtrlShadowedReadVal(u32);
    impl ClassdCtrlShadowedReadVal {
        #[doc = "Enable escalation mechanisms (accumulation and\ninterrupt timeout) for Class D. Note that interrupts can fire\nregardless of whether the escalation mechanisms are enabled for\nthis class or not."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable automatic locking of escalation counter for class D.\nIf true, there is no way to stop the escalation protocol for class D\nonce it has been triggered."]
        #[inline(always)]
        pub fn lock(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable escalation signal 0 for Class D"]
        #[inline(always)]
        pub fn en_e0(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Enable escalation signal 1 for Class D"]
        #[inline(always)]
        pub fn en_e1(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Enable escalation signal 2 for Class D"]
        #[inline(always)]
        pub fn en_e2(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Enable escalation signal 3 for Class D"]
        #[inline(always)]
        pub fn en_e3(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Determines in which escalation phase escalation signal 0 shall be asserted."]
        #[inline(always)]
        pub fn map_e0(&self) -> u32 {
            (self.0 >> 6) & 3
        }
        #[doc = "Determines in which escalation phase escalation signal 1 shall be asserted."]
        #[inline(always)]
        pub fn map_e1(&self) -> u32 {
            (self.0 >> 8) & 3
        }
        #[doc = "Determines in which escalation phase escalation signal 2 shall be asserted."]
        #[inline(always)]
        pub fn map_e2(&self) -> u32 {
            (self.0 >> 10) & 3
        }
        #[doc = "Determines in which escalation phase escalation signal 3 shall be asserted."]
        #[inline(always)]
        pub fn map_e3(&self) -> u32 {
            (self.0 >> 12) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassdCtrlShadowedWriteVal {
            ClassdCtrlShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ClassdCtrlShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdCtrlShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdCtrlShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdCtrlShadowedWriteVal(u32);
    impl ClassdCtrlShadowedWriteVal {
        #[doc = "Enable escalation mechanisms (accumulation and\ninterrupt timeout) for Class D. Note that interrupts can fire\nregardless of whether the escalation mechanisms are enabled for\nthis class or not."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable automatic locking of escalation counter for class D.\nIf true, there is no way to stop the escalation protocol for class D\nonce it has been triggered."]
        #[inline(always)]
        pub fn lock(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable escalation signal 0 for Class D"]
        #[inline(always)]
        pub fn en_e0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Enable escalation signal 1 for Class D"]
        #[inline(always)]
        pub fn en_e1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Enable escalation signal 2 for Class D"]
        #[inline(always)]
        pub fn en_e2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Enable escalation signal 3 for Class D"]
        #[inline(always)]
        pub fn en_e3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Determines in which escalation phase escalation signal 0 shall be asserted."]
        #[inline(always)]
        pub fn map_e0(self, val: u32) -> Self {
            Self((self.0 & !(3 << 6)) | ((val & 3) << 6))
        }
        #[doc = "Determines in which escalation phase escalation signal 1 shall be asserted."]
        #[inline(always)]
        pub fn map_e1(self, val: u32) -> Self {
            Self((self.0 & !(3 << 8)) | ((val & 3) << 8))
        }
        #[doc = "Determines in which escalation phase escalation signal 2 shall be asserted."]
        #[inline(always)]
        pub fn map_e2(self, val: u32) -> Self {
            Self((self.0 & !(3 << 10)) | ((val & 3) << 10))
        }
        #[doc = "Determines in which escalation phase escalation signal 3 shall be asserted."]
        #[inline(always)]
        pub fn map_e3(self, val: u32) -> Self {
            Self((self.0 & !(3 << 12)) | ((val & 3) << 12))
        }
    }
    impl From<u32> for ClassdCtrlShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdCtrlShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdCtrlShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdRegwenReadVal(u32);
    impl ClassdRegwenReadVal {
        #[doc = "Class configuration enable bit.\nIf this is cleared to 0, the corresponding class configuration\nregisters cannot be written anymore."]
        #[inline(always)]
        pub fn classd_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClassdRegwenWriteVal {
            ClassdRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ClassdRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdRegwenWriteVal(u32);
    impl ClassdRegwenWriteVal {
        #[doc = "Class configuration enable bit.\nIf this is cleared to 0, the corresponding class configuration\nregisters cannot be written anymore."]
        #[inline(always)]
        pub fn classd_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ClassdRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClassdStateReadVal(u32);
    impl ClassdStateReadVal {
        #[inline(always)]
        pub fn classd_state(&self) -> super::enums::ClassxState {
            super::enums::ClassxState::try_from((self.0 >> 0) & 7).unwrap()
        }
    }
    impl From<u32> for ClassdStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClassdStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClassdStateReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.classa is set."]
        #[inline(always)]
        pub fn classa(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.classb is set."]
        #[inline(always)]
        pub fn classb(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.classc is set."]
        #[inline(always)]
        pub fn classc(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.classd is set."]
        #[inline(always)]
        pub fn classd(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IntrEnableWriteVal {
            IntrEnableWriteVal(self.0)
        }
    }
    impl From<u32> for IntrEnableReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrEnableReadVal> for u32 {
        #[inline(always)]
        fn from(val: IntrEnableReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableWriteVal(u32);
    impl IntrEnableWriteVal {
        #[doc = "Enable interrupt when !!INTR_STATE.classa is set."]
        #[inline(always)]
        pub fn classa(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.classb is set."]
        #[inline(always)]
        pub fn classb(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.classc is set."]
        #[inline(always)]
        pub fn classc(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.classd is set."]
        #[inline(always)]
        pub fn classd(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
    }
    impl From<u32> for IntrEnableWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrEnableWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntrEnableWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrStateReadVal(u32);
    impl IntrStateReadVal {
        #[doc = "Interrupt state bit of Class A. Set by HW in case an alert within this class triggered. Defaults true, write one to clear."]
        #[inline(always)]
        pub fn classa(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt state bit of Class B. Set by HW in case an alert within this class triggered. Defaults true, write one to clear."]
        #[inline(always)]
        pub fn classb(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt state bit of Class C. Set by HW in case an alert within this class triggered. Defaults true, write one to clear."]
        #[inline(always)]
        pub fn classc(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt state bit of Class D. Set by HW in case an alert within this class triggered. Defaults true, write one to clear."]
        #[inline(always)]
        pub fn classd(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IntrStateWriteVal {
            IntrStateWriteVal(self.0)
        }
    }
    impl From<u32> for IntrStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: IntrStateReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrStateWriteVal(u32);
    impl IntrStateWriteVal {
        #[doc = "Interrupt state bit of Class A. Set by HW in case an alert within this class triggered. Defaults true, write one to clear."]
        #[inline(always)]
        pub fn classa_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Interrupt state bit of Class B. Set by HW in case an alert within this class triggered. Defaults true, write one to clear."]
        #[inline(always)]
        pub fn classb_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "Interrupt state bit of Class C. Set by HW in case an alert within this class triggered. Defaults true, write one to clear."]
        #[inline(always)]
        pub fn classc_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "Interrupt state bit of Class D. Set by HW in case an alert within this class triggered. Defaults true, write one to clear."]
        #[inline(always)]
        pub fn classd_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
    }
    impl From<u32> for IntrStateWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrStateWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntrStateWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrTestWriteVal(u32);
    impl IntrTestWriteVal {
        #[doc = "Write 1 to force !!INTR_STATE.classa to 1."]
        #[inline(always)]
        pub fn classa(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.classb to 1."]
        #[inline(always)]
        pub fn classb(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to force !!INTR_STATE.classc to 1."]
        #[inline(always)]
        pub fn classc(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Write 1 to force !!INTR_STATE.classd to 1."]
        #[inline(always)]
        pub fn classd(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
    }
    impl From<u32> for IntrTestWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrTestWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntrTestWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct LocAlertCauseReadVal(u32);
    impl LocAlertCauseReadVal {
        #[doc = "Cause bit "]
        #[inline(always)]
        pub fn la(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> LocAlertCauseWriteVal {
            LocAlertCauseWriteVal(self.0)
        }
    }
    impl From<u32> for LocAlertCauseReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<LocAlertCauseReadVal> for u32 {
        #[inline(always)]
        fn from(val: LocAlertCauseReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct LocAlertCauseWriteVal(u32);
    impl LocAlertCauseWriteVal {
        #[doc = "Cause bit "]
        #[inline(always)]
        pub fn la_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
    }
    impl From<u32> for LocAlertCauseWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<LocAlertCauseWriteVal> for u32 {
        #[inline(always)]
        fn from(val: LocAlertCauseWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct LocAlertClassShadowedReadVal(u32);
    impl LocAlertClassShadowedReadVal {
        #[doc = "Classification "]
        #[inline(always)]
        pub fn class_la(&self) -> super::enums::Class {
            super::enums::Class::try_from((self.0 >> 0) & 3).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> LocAlertClassShadowedWriteVal {
            LocAlertClassShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for LocAlertClassShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<LocAlertClassShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: LocAlertClassShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct LocAlertClassShadowedWriteVal(u32);
    impl LocAlertClassShadowedWriteVal {
        #[doc = "Classification "]
        #[inline(always)]
        pub fn class_la(
            self,
            f: impl FnOnce(super::enums::selector::ClassSelector) -> super::enums::Class,
        ) -> Self {
            Self(
                (self.0 & !(3 << 0)) | (u32::from(f(super::enums::selector::ClassSelector())) << 0),
            )
        }
    }
    impl From<u32> for LocAlertClassShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<LocAlertClassShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: LocAlertClassShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct LocAlertEnShadowedReadVal(u32);
    impl LocAlertEnShadowedReadVal {
        #[doc = "Alert enable bit.\n\nNote that the alert pinging mechanism will only ping alerts that have been enabled and locked."]
        #[inline(always)]
        pub fn en_la(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> LocAlertEnShadowedWriteVal {
            LocAlertEnShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for LocAlertEnShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<LocAlertEnShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: LocAlertEnShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct LocAlertEnShadowedWriteVal(u32);
    impl LocAlertEnShadowedWriteVal {
        #[doc = "Alert enable bit.\n\nNote that the alert pinging mechanism will only ping alerts that have been enabled and locked."]
        #[inline(always)]
        pub fn en_la(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for LocAlertEnShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<LocAlertEnShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: LocAlertEnShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct LocAlertRegwenReadVal(u32);
    impl LocAlertRegwenReadVal {
        #[doc = "Alert configuration write enable bit.\nIf this is cleared to 0, the corresponding !!LOC_ALERT_EN_SHADOWED\nand !!LOC_ALERT_CLASS_SHADOWED bits are not writable anymore.\n\nNote that the alert pinging mechanism will only ping alerts that have been enabled and locked."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> LocAlertRegwenWriteVal {
            LocAlertRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for LocAlertRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<LocAlertRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: LocAlertRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct LocAlertRegwenWriteVal(u32);
    impl LocAlertRegwenWriteVal {
        #[doc = "Alert configuration write enable bit.\nIf this is cleared to 0, the corresponding !!LOC_ALERT_EN_SHADOWED\nand !!LOC_ALERT_CLASS_SHADOWED bits are not writable anymore.\n\nNote that the alert pinging mechanism will only ping alerts that have been enabled and locked."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for LocAlertRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<LocAlertRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: LocAlertRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PingTimeoutCycShadowedReadVal(u32);
    impl PingTimeoutCycShadowedReadVal {
        #[doc = "Timeout value in cycles.\nIf an alert receiver or an escalation sender does not respond to a ping within this timeout window, a pingfail alert will be raised.\nIt is recommended to set this value to the equivalent of 256 cycles of the slowest alert sender clock domain in the system (or greater)."]
        #[inline(always)]
        pub fn ping_timeout_cyc_shadowed(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> PingTimeoutCycShadowedWriteVal {
            PingTimeoutCycShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for PingTimeoutCycShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PingTimeoutCycShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: PingTimeoutCycShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PingTimeoutCycShadowedWriteVal(u32);
    impl PingTimeoutCycShadowedWriteVal {
        #[doc = "Timeout value in cycles.\nIf an alert receiver or an escalation sender does not respond to a ping within this timeout window, a pingfail alert will be raised.\nIt is recommended to set this value to the equivalent of 256 cycles of the slowest alert sender clock domain in the system (or greater)."]
        #[inline(always)]
        pub fn ping_timeout_cyc_shadowed(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for PingTimeoutCycShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PingTimeoutCycShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: PingTimeoutCycShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PingTimerEnShadowedReadVal(u32);
    impl PingTimerEnShadowedReadVal {
        #[doc = "Setting this to 1 enables the ping timer mechanism.\nThis bit cannot be cleared to 0 once it has been set to 1.\n\nNote that the alert pinging mechanism will only ping alerts that have been enabled and locked."]
        #[inline(always)]
        pub fn ping_timer_en_shadowed(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> PingTimerEnShadowedWriteVal {
            PingTimerEnShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for PingTimerEnShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PingTimerEnShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: PingTimerEnShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PingTimerEnShadowedWriteVal(u32);
    impl PingTimerEnShadowedWriteVal {
        #[doc = "Setting this to 1 enables the ping timer mechanism.\nThis bit cannot be cleared to 0 once it has been set to 1.\n\nNote that the alert pinging mechanism will only ping alerts that have been enabled and locked."]
        #[inline(always)]
        pub fn ping_timer_en_shadowed_set(self) -> Self {
            Self(self.0 | (1 << 0))
        }
    }
    impl From<u32> for PingTimerEnShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PingTimerEnShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: PingTimerEnShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PingTimerRegwenReadVal(u32);
    impl PingTimerRegwenReadVal {
        #[doc = "When true, the !!PING_TIMEOUT_CYC_SHADOWED and !!PING_TIMER_EN_SHADOWED registers can be modified.\nWhen false, they become read-only. Defaults true, write one to clear.\nThis should be cleared once the alert handler has been configured and the ping\ntimer mechanism has been kicked off."]
        #[inline(always)]
        pub fn ping_timer_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> PingTimerRegwenWriteVal {
            PingTimerRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for PingTimerRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PingTimerRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: PingTimerRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PingTimerRegwenWriteVal(u32);
    impl PingTimerRegwenWriteVal {
        #[doc = "When true, the !!PING_TIMEOUT_CYC_SHADOWED and !!PING_TIMER_EN_SHADOWED registers can be modified.\nWhen false, they become read-only. Defaults true, write one to clear.\nThis should be cleared once the alert handler has been configured and the ping\ntimer mechanism has been kicked off."]
        #[inline(always)]
        pub fn ping_timer_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for PingTimerRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PingTimerRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: PingTimerRegwenWriteVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum ClassxState {
        Idle = 0,
        Timeout = 1,
        Fsmerror = 2,
        Terminal = 3,
        Phase0 = 4,
        Phase1 = 5,
        Phase2 = 6,
        Phase3 = 7,
    }
    impl ClassxState {
        #[inline(always)]
        pub fn idle(&self) -> bool {
            *self == Self::Idle
        }
        #[inline(always)]
        pub fn timeout(&self) -> bool {
            *self == Self::Timeout
        }
        #[inline(always)]
        pub fn fsm_error(&self) -> bool {
            *self == Self::Fsmerror
        }
        #[inline(always)]
        pub fn terminal(&self) -> bool {
            *self == Self::Terminal
        }
        #[inline(always)]
        pub fn phase0(&self) -> bool {
            *self == Self::Phase0
        }
        #[inline(always)]
        pub fn phase1(&self) -> bool {
            *self == Self::Phase1
        }
        #[inline(always)]
        pub fn phase2(&self) -> bool {
            *self == Self::Phase2
        }
        #[inline(always)]
        pub fn phase3(&self) -> bool {
            *self == Self::Phase3
        }
    }
    impl TryFrom<u32> for ClassxState {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<ClassxState, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, ClassxState>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<ClassxState> for u32 {
        fn from(val: ClassxState) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Class {
        Classa = 0,
        Classb = 1,
        Classc = 2,
        Classd = 3,
    }
    impl Class {
        #[inline(always)]
        pub fn class_a(&self) -> bool {
            *self == Self::Classa
        }
        #[inline(always)]
        pub fn class_b(&self) -> bool {
            *self == Self::Classb
        }
        #[inline(always)]
        pub fn class_c(&self) -> bool {
            *self == Self::Classc
        }
        #[inline(always)]
        pub fn class_d(&self) -> bool {
            *self == Self::Classd
        }
    }
    impl TryFrom<u32> for Class {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Class, ()> {
            if val < 4 {
                Ok(unsafe { core::mem::transmute::<u32, Class>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Class> for u32 {
        fn from(val: Class) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct ClassxStateSelector();
        impl ClassxStateSelector {
            #[inline(always)]
            pub fn idle(&self) -> super::ClassxState {
                super::ClassxState::Idle
            }
            #[inline(always)]
            pub fn timeout(&self) -> super::ClassxState {
                super::ClassxState::Timeout
            }
            #[inline(always)]
            pub fn fsm_error(&self) -> super::ClassxState {
                super::ClassxState::Fsmerror
            }
            #[inline(always)]
            pub fn terminal(&self) -> super::ClassxState {
                super::ClassxState::Terminal
            }
            #[inline(always)]
            pub fn phase0(&self) -> super::ClassxState {
                super::ClassxState::Phase0
            }
            #[inline(always)]
            pub fn phase1(&self) -> super::ClassxState {
                super::ClassxState::Phase1
            }
            #[inline(always)]
            pub fn phase2(&self) -> super::ClassxState {
                super::ClassxState::Phase2
            }
            #[inline(always)]
            pub fn phase3(&self) -> super::ClassxState {
                super::ClassxState::Phase3
            }
        }
        pub struct ClassSelector();
        impl ClassSelector {
            #[inline(always)]
            pub fn class_a(&self) -> super::Class {
                super::Class::Classa
            }
            #[inline(always)]
            pub fn class_b(&self) -> super::Class {
                super::Class::Classb
            }
            #[inline(always)]
            pub fn class_c(&self) -> super::Class {
                super::Class::Classc
            }
            #[inline(always)]
            pub fn class_d(&self) -> super::Class {
                super::Class::Classd
            }
        }
    }
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type IntrState =
        ureg::ReadWriteReg32<0, crate::regs::IntrStateReadVal, crate::regs::IntrStateWriteVal>;
    pub type IntrEnable =
        ureg::ReadWriteReg32<0, crate::regs::IntrEnableReadVal, crate::regs::IntrEnableWriteVal>;
    pub type IntrTest = ureg::WriteOnlyReg32<0, crate::regs::IntrTestWriteVal>;
    pub type PingTimerRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::PingTimerRegwenReadVal,
        crate::regs::PingTimerRegwenWriteVal,
    >;
    pub type PingTimeoutCycShadowed = ureg::ReadWriteReg32<
        0x100,
        crate::regs::PingTimeoutCycShadowedReadVal,
        crate::regs::PingTimeoutCycShadowedWriteVal,
    >;
    pub type PingTimerEnShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::PingTimerEnShadowedReadVal,
        crate::regs::PingTimerEnShadowedWriteVal,
    >;
    pub type AlertRegwen =
        ureg::ReadWriteReg32<1, crate::regs::AlertRegwenReadVal, crate::regs::AlertRegwenWriteVal>;
    pub type AlertEnShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::AlertEnShadowedReadVal,
        crate::regs::AlertEnShadowedWriteVal,
    >;
    pub type AlertClassShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::AlertClassShadowedReadVal,
        crate::regs::AlertClassShadowedWriteVal,
    >;
    pub type AlertCause =
        ureg::ReadWriteReg32<0, crate::regs::AlertCauseReadVal, crate::regs::AlertCauseWriteVal>;
    pub type LocAlertRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::LocAlertRegwenReadVal,
        crate::regs::LocAlertRegwenWriteVal,
    >;
    pub type LocAlertEnShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::LocAlertEnShadowedReadVal,
        crate::regs::LocAlertEnShadowedWriteVal,
    >;
    pub type LocAlertClassShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::LocAlertClassShadowedReadVal,
        crate::regs::LocAlertClassShadowedWriteVal,
    >;
    pub type LocAlertCause = ureg::ReadWriteReg32<
        0,
        crate::regs::LocAlertCauseReadVal,
        crate::regs::LocAlertCauseWriteVal,
    >;
    pub type ClassaRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ClassaRegwenReadVal,
        crate::regs::ClassaRegwenWriteVal,
    >;
    pub type ClassaCtrlShadowed = ureg::ReadWriteReg32<
        0x393c,
        crate::regs::ClassaCtrlShadowedReadVal,
        crate::regs::ClassaCtrlShadowedWriteVal,
    >;
    pub type ClassaClrRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ClassaClrRegwenReadVal,
        crate::regs::ClassaClrRegwenWriteVal,
    >;
    pub type ClassaClrShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::ClassaClrShadowedReadVal,
        crate::regs::ClassaClrShadowedWriteVal,
    >;
    pub type ClassaAccumCnt = ureg::ReadOnlyReg32<crate::regs::ClassaAccumCntReadVal>;
    pub type ClassaAccumThreshShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::ClassaAccumThreshShadowedReadVal,
        crate::regs::ClassaAccumThreshShadowedWriteVal,
    >;
    pub type ClassaTimeoutCycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassaCrashdumpTriggerShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::ClassaCrashdumpTriggerShadowedReadVal,
        crate::regs::ClassaCrashdumpTriggerShadowedWriteVal,
    >;
    pub type ClassaPhase0CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassaPhase1CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassaPhase2CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassaPhase3CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassaEscCnt = ureg::ReadOnlyReg32<u32>;
    pub type ClassaState = ureg::ReadOnlyReg32<crate::regs::ClassaStateReadVal>;
    pub type ClassbRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ClassbRegwenReadVal,
        crate::regs::ClassbRegwenWriteVal,
    >;
    pub type ClassbCtrlShadowed = ureg::ReadWriteReg32<
        0x393c,
        crate::regs::ClassbCtrlShadowedReadVal,
        crate::regs::ClassbCtrlShadowedWriteVal,
    >;
    pub type ClassbClrRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ClassbClrRegwenReadVal,
        crate::regs::ClassbClrRegwenWriteVal,
    >;
    pub type ClassbClrShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::ClassbClrShadowedReadVal,
        crate::regs::ClassbClrShadowedWriteVal,
    >;
    pub type ClassbAccumCnt = ureg::ReadOnlyReg32<crate::regs::ClassbAccumCntReadVal>;
    pub type ClassbAccumThreshShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::ClassbAccumThreshShadowedReadVal,
        crate::regs::ClassbAccumThreshShadowedWriteVal,
    >;
    pub type ClassbTimeoutCycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassbCrashdumpTriggerShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::ClassbCrashdumpTriggerShadowedReadVal,
        crate::regs::ClassbCrashdumpTriggerShadowedWriteVal,
    >;
    pub type ClassbPhase0CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassbPhase1CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassbPhase2CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassbPhase3CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassbEscCnt = ureg::ReadOnlyReg32<u32>;
    pub type ClassbState = ureg::ReadOnlyReg32<crate::regs::ClassbStateReadVal>;
    pub type ClasscRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ClasscRegwenReadVal,
        crate::regs::ClasscRegwenWriteVal,
    >;
    pub type ClasscCtrlShadowed = ureg::ReadWriteReg32<
        0x393c,
        crate::regs::ClasscCtrlShadowedReadVal,
        crate::regs::ClasscCtrlShadowedWriteVal,
    >;
    pub type ClasscClrRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ClasscClrRegwenReadVal,
        crate::regs::ClasscClrRegwenWriteVal,
    >;
    pub type ClasscClrShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::ClasscClrShadowedReadVal,
        crate::regs::ClasscClrShadowedWriteVal,
    >;
    pub type ClasscAccumCnt = ureg::ReadOnlyReg32<crate::regs::ClasscAccumCntReadVal>;
    pub type ClasscAccumThreshShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::ClasscAccumThreshShadowedReadVal,
        crate::regs::ClasscAccumThreshShadowedWriteVal,
    >;
    pub type ClasscTimeoutCycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClasscCrashdumpTriggerShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::ClasscCrashdumpTriggerShadowedReadVal,
        crate::regs::ClasscCrashdumpTriggerShadowedWriteVal,
    >;
    pub type ClasscPhase0CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClasscPhase1CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClasscPhase2CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClasscPhase3CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClasscEscCnt = ureg::ReadOnlyReg32<u32>;
    pub type ClasscState = ureg::ReadOnlyReg32<crate::regs::ClasscStateReadVal>;
    pub type ClassdRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ClassdRegwenReadVal,
        crate::regs::ClassdRegwenWriteVal,
    >;
    pub type ClassdCtrlShadowed = ureg::ReadWriteReg32<
        0x393c,
        crate::regs::ClassdCtrlShadowedReadVal,
        crate::regs::ClassdCtrlShadowedWriteVal,
    >;
    pub type ClassdClrRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ClassdClrRegwenReadVal,
        crate::regs::ClassdClrRegwenWriteVal,
    >;
    pub type ClassdClrShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::ClassdClrShadowedReadVal,
        crate::regs::ClassdClrShadowedWriteVal,
    >;
    pub type ClassdAccumCnt = ureg::ReadOnlyReg32<crate::regs::ClassdAccumCntReadVal>;
    pub type ClassdAccumThreshShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::ClassdAccumThreshShadowedReadVal,
        crate::regs::ClassdAccumThreshShadowedWriteVal,
    >;
    pub type ClassdTimeoutCycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassdCrashdumpTriggerShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::ClassdCrashdumpTriggerShadowedReadVal,
        crate::regs::ClassdCrashdumpTriggerShadowedWriteVal,
    >;
    pub type ClassdPhase0CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassdPhase1CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassdPhase2CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassdPhase3CycShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ClassdEscCnt = ureg::ReadOnlyReg32<u32>;
    pub type ClassdState = ureg::ReadOnlyReg32<crate::regs::ClassdStateReadVal>;
}
