// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use demo_task_codegen::handle;
use pw_status::Result;
use registers::rv_timer::RvTimer;
use userspace::time::{sleep_until, Instant};
use userspace::{entry, syscall};
use zfmt::{log_info, FlatAdapter, FlatSend, ZfmtStr, ZfmtU64};

#[inline(always)]
fn rv_timer_value(rv_timer: &RvTimer) -> u64 {
    let regs = rv_timer.regs();
    loop {
        let hi1 = regs.timer_v_upper0().read();
        let low = regs.timer_v_lower0().read();
        let hi2 = regs.timer_v_upper0().read();
        if hi1 == hi2 {
            return ((hi1 as u64) << 32) | (low as u64);
        }
    }
}

#[derive(zfmt::Zfmt)]
#[zfmt(format = "sensor_status active={active} battery_mv={battery_mv} error_code={error_code}")]
pub struct SensorStatus {
    pub active: bool,
    pub battery_mv: u16,
    pub error_code: i8,
}

#[derive(zfmt::Zfmt)]
#[zfmt(format = "sensor_cal gain_x1000={gain_x1000} offset_x1000={offset_x1000}")]
pub struct SensorCalibration {
    pub gain_x1000: u32,
    pub offset_x1000: i32,
}

#[derive(zfmt::Zfmt)]
#[zfmt(format = "task_notification task={task_name} priority={priority}")]
pub struct TaskNotification<'a> {
    pub task_name: &'a str,
    pub priority: u32,
}

#[derive(zfmt::Zfmt)]
#[zfmt(format = "interned_event category={category} level={level}")]
pub struct InternedEvent {
    pub category: ZfmtStr,
    pub level: u8,
}

#[derive(zfmt::Zfmt)]
pub enum SystemAlert {
    #[zfmt(format = "system_alert status=ok")]
    Ok,
    #[zfmt(format = "system_alert status=warning code={code}")]
    Warning { code: u32 },
    #[zfmt(format = "system_alert status=error reason={reason}")]
    Error { reason: ZfmtStr },
}

struct IpcLogger {
    rv_timer: RvTimer,
}

impl FlatSend for IpcLogger {
    fn timestamp(&self) -> ZfmtU64 {
        ZfmtU64::from_u64(rv_timer_value(&self.rv_timer))
    }
    fn send(&self, data: &[u8]) {
        let mut recv_buf = [0u8; 1];
        let tx_res = syscall::channel_transact(handle::IPC, data, &mut recv_buf[..0], Instant::MAX);
        if let Err(e) = tx_res {
            pw_log::error!("channel_transact failed: {}", e as u32);
        }
    }
}

static LOGGER: FlatAdapter<IpcLogger, 256> = FlatAdapter::new(IpcLogger {
    rv_timer: unsafe { RvTimer::new() },
});

#[entry]
fn entry() -> Result<()> {
    pw_log::info!("Demo task started");

    let mut next_wake_time = syscall::debug_clock_now().ticks() + 1_500_000;

    // Iteration 0: SensorStatus (Booleans, Signed/Unsigned integers)
    let sleep_res = sleep_until(Instant::from_ticks(next_wake_time));
    if let Err(e) = sleep_res {
        pw_log::error!("sleep failed: {}", e as u32);
    }
    log_info!(
        LOGGER,
        SensorStatus {
            active: true,
            battery_mv: 3300,
            error_code: -1
        }
    );
    next_wake_time = syscall::debug_clock_now().ticks() + 1_500_000;

    // Iteration 1: SensorCalibration (Fixed-point arithmetic values)
    let sleep_res = sleep_until(Instant::from_ticks(next_wake_time));
    if let Err(e) = sleep_res {
        pw_log::error!("sleep failed: {}", e as u32);
    }
    log_info!(
        LOGGER,
        SensorCalibration {
            gain_x1000: 1230,
            offset_x1000: -50
        }
    );
    next_wake_time = syscall::debug_clock_now().ticks() + 1_500_000;

    // Iteration 2: TaskNotification (Variable-length string references - Tier 2)
    let sleep_res = sleep_until(Instant::from_ticks(next_wake_time));
    if let Err(e) = sleep_res {
        pw_log::error!("sleep failed: {}", e as u32);
    }
    log_info!(
        LOGGER,
        TaskNotification {
            task_name: "demo_worker",
            priority: 2
        }
    );
    next_wake_time = syscall::debug_clock_now().ticks() + 1_500_000;

    // Iteration 3: InternedEvent (Compile-time interned strings using ZfmtStr)
    let sleep_res = sleep_until(Instant::from_ticks(next_wake_time));
    if let Err(e) = sleep_res {
        pw_log::error!("sleep failed: {}", e as u32);
    }
    log_info!(
        LOGGER,
        InternedEvent {
            category: ZfmtStr::new(zfmt::zfmt_str!("hardware_driver")),
            level: 4
        }
    );
    next_wake_time = syscall::debug_clock_now().ticks() + 1_500_000;

    // Iteration 4: SystemAlert (Enum dispatcher with struct payloads and ZfmtStr)
    let sleep_res = sleep_until(Instant::from_ticks(next_wake_time));
    if let Err(e) = sleep_res {
        pw_log::error!("sleep failed: {}", e as u32);
    }
    log_info!(
        LOGGER,
        SystemAlert::Error {
            reason: ZfmtStr::new(zfmt::zfmt_str!("out_of_memory"))
        }
    );

    pw_log::info!("Demo task finished");
    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in demo_task");
    loop {}
}
