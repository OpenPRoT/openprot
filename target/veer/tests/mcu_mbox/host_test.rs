// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Sends commands to the two MCU mailboxes and checks the answers of the app
//! in `responder.rs`, which uses the `veer_mcu_mbox` driver.
//!
//! The host takes the place of the SoC agent and writes the registers of a
//! mailbox itself ([`start`] and [`finish`]). The hardware model's
//! `mailbox_execute` reaches only mailbox 0. It also goes on when the lock
//! register reads 1, which means that the lock is held, so on the VCK190 it
//! loses a command that it starts while the hardware is clearing the SRAM
//! after the command before.
//!
//! On the emulator and on the VCK190 the hardware model reaches the
//! mailboxes with the AXI user of the MCU, so the MCI does not raise its
//! notification for a command. The app polls the mailboxes and does not use
//! the notification.
//!
//! The test does not cover an ECC error or a requester that releases a
//! mailbox before the answer. The host tests of the driver cover both.

use std::time::{Duration, Instant};

use caliptra_mcu_hw_model::{McuHwModel, McuManager};
use test_boot_files::BootFiles;

/// Logged by `responder.rs` when it has released the locks of the mailboxes.
const READY: &str = "The MCU mailboxes are ready";

/// The commands of `responder.rs`.
const INVERT: u32 = 0x494e_5654;
const FILL: u32 = 0x4649_4c4c;
const USER: u32 = 0x5553_4552;
const SIZE: u32 = 0x5349_5a45;
const DONE: u32 = 0x444f_4e45;

/// A command that `responder.rs` does not know.
const UNKNOWN: u32 = 0x1;

/// Values of `mbox_cmd_status`.
const CMD_BUSY: u32 = 0;
const DATA_READY: u32 = 1;
const CMD_COMPLETE: u32 = 2;
const CMD_FAILURE: u32 = 3;

/// The largest request and the largest response of the test, in bytes: the
/// size of the smaller SRAM of the two products, the VCK190's.
const MAX_LENGTH: usize = 16 * 1024;

/// The largest SRAM that a mailbox can have, in bytes.
const MAX_SRAM_SIZE: usize = 2 * 1024 * 1024;

const TIMEOUT: Duration = Duration::from_secs(120);

/// How long the test waits for the lock of a mailbox or for an answer.
const MAILBOX_TIMEOUT: Duration = Duration::from_secs(30);

/// Calls the closure with the registers of MCU mailbox `$index`. The two
/// mailboxes have register blocks of different types with the same methods.
macro_rules! with_mbox {
    ($hw:expr, $index:expr, |$mbox:ident| $body:expr) => {
        match $index {
            0 => $hw.mcu_manager().with_mbox0(|$mbox| $body),
            1 => $hw.mcu_manager().with_mbox1(|$mbox| $body),
            index => panic!("there is no MCU mailbox {index}"),
        }
    };
}

/// The answer of the MCU to a command.
#[derive(Debug, PartialEq)]
struct Answer {
    /// `mbox_cmd_status`.
    status: u32,
    /// The first `mbox_dlen` bytes of the SRAM.
    data: Vec<u8>,
}

impl Answer {
    fn data(data: Vec<u8>) -> Self {
        Self {
            status: DATA_READY,
            data,
        }
    }

    fn failure() -> Self {
        Self {
            status: CMD_FAILURE,
            data: Vec::new(),
        }
    }
}

/// Runs the model until `done` returns true or [`TIMEOUT`] passes, and
/// returns what the MCU logged.
fn run_until<M: McuHwModel>(hw: &mut M, mut done: impl FnMut(&mut M, &str) -> bool) -> String {
    let start = Instant::now();
    let mut log = String::new();
    hw.step_until(|m| {
        let out = m.output().take(usize::MAX);
        print!("{out}");
        log.push_str(&out);
        done(m, &log) || start.elapsed() > TIMEOUT
    });
    log
}

/// Reads the lock register of mailbox `index` until the read takes the lock.
fn take_lock<M: McuHwModel>(hw: &mut M, index: usize) {
    let start = Instant::now();
    while with_mbox!(hw, index, |mbox| mbox.mbox_lock().read().lock()) {
        assert!(
            start.elapsed() < MAILBOX_TIMEOUT,
            "the lock of mailbox {index} was not released"
        );
        hw.step();
    }
}

/// Takes the lock of mailbox `index` and starts `command` with `length` as
/// the length and `data` as the request data.
fn start<M: McuHwModel>(hw: &mut M, index: usize, command: u32, length: usize, data: &[u8]) {
    take_lock(hw, index);
    with_mbox!(hw, index, |mbox| {
        mbox.mbox_cmd().write(|_| command);
        mbox.mbox_dlen().write(|_| length as u32);
        for (i, chunk) in data.chunks(4).enumerate() {
            let mut word = [0u8; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            mbox.mbox_sram().at(i).write(|_| u32::from_le_bytes(word));
        }
        mbox.mbox_execute().write(|w| w.execute(true));
    });
}

/// Waits for the answer to the command in mailbox `index`, reads it and
/// releases the mailbox.
fn finish<M: McuHwModel>(hw: &mut M, index: usize) -> Answer {
    let start = Instant::now();
    let status = loop {
        let status = with_mbox!(hw, index, |mbox| u32::from(
            mbox.mbox_cmd_status().read().status()
        ));
        if status != CMD_BUSY {
            break status;
        }
        assert!(
            start.elapsed() < MAILBOX_TIMEOUT,
            "the command in mailbox {index} was not answered"
        );
        hw.step();
    };
    let data = with_mbox!(hw, index, |mbox| {
        let length = mbox.mbox_dlen().read() as usize;
        assert!(length <= MAX_LENGTH, "the answer has {length} bytes");
        let mut data = Vec::new();
        for i in 0..length.div_ceil(4) {
            data.extend_from_slice(&mbox.mbox_sram().at(i).read().to_le_bytes());
        }
        data.truncate(length);
        mbox.mbox_execute().write(|w| w.execute(false));
        data
    });
    Answer { status, data }
}

/// Sends `command` with `request` to mailbox `index` and returns the answer.
fn execute<M: McuHwModel>(hw: &mut M, index: usize, command: u32, request: &[u8]) -> Answer {
    start(hw, index, command, request.len(), request);
    finish(hw, index)
}

/// The data that `responder.rs` answers `INVERT` of `request` with.
fn inverted(request: &[u8]) -> Vec<u8> {
    request.iter().map(|byte| !byte).collect()
}

/// Checks the answers of the app to commands in mailbox `index`.
fn check_mailbox<M: McuHwModel>(hw: &mut M, index: usize) {
    // The app released the lock that the MCU holds after a reset, so the
    // host can take it. The size of the SRAM that the app uses is the size
    // that the MCI reports. The emulator reports 0.
    let answer = execute(hw, index, SIZE, &[]);
    assert_eq!(answer.status, DATA_READY, "SIZE failed");
    let sram_size = u32::from_le_bytes(answer.data.try_into().expect("SIZE returned no word"));
    let sram_size = sram_size as usize;
    let reported_kib = hw.mcu_manager().with_mci(|mci| {
        let config = mci.hw_config0().read();
        match index {
            0 => config.mcu_mbox0_sram_size(),
            _ => config.mcu_mbox1_sram_size(),
        }
    });
    println!("mailbox {index}: the app uses {sram_size} bytes, the MCI reports {reported_kib} KiB");
    if reported_kib != 0 {
        assert_eq!(
            sram_size,
            reported_kib as usize * 1024,
            "the system config has the wrong size for the SRAM of mailbox {index}"
        );
    }
    assert!(
        sram_size >= MAX_LENGTH,
        "the SRAM is smaller than the test expects"
    );

    // Request data comes back with every bit inverted.
    for len in [0, 1, 2, 3, 4, 5, 255, 256, 257, 1000, 4099, MAX_LENGTH] {
        let request: Vec<u8> = (0..len).map(|i| (i * 7 + 3 + index) as u8).collect();
        assert_eq!(
            execute(hw, index, INVERT, &request),
            Answer::data(inverted(&request)),
            "wrong answer to INVERT of {len} bytes"
        );
    }

    // A response can be longer than its request.
    for len in [1, 6, 4096, MAX_LENGTH] {
        assert_eq!(
            execute(hw, index, FILL, &(len as u32).to_le_bytes()),
            Answer::data((0..len).map(|i| i as u8).collect()),
            "wrong answer to FILL of {len} bytes"
        );
    }

    // When the host releases the mailbox, the emulator or the hardware
    // clears what the last response left in the SRAM.
    assert_eq!(
        execute(hw, index, FILL, &8u32.to_le_bytes()),
        Answer::data((0..8).collect())
    );
    take_lock(hw, index);
    let words = with_mbox!(hw, index, |mbox| {
        let words = (mbox.mbox_sram().at(0).read(), mbox.mbox_sram().at(1).read());
        mbox.mbox_execute().write(|w| w.execute(false));
        words
    });
    assert_eq!(words, (0, 0), "the SRAM kept the last response");

    // The app sees the AXI user that holds the lock.
    start(hw, index, USER, 0, &[]);
    let user = with_mbox!(hw, index, |mbox| mbox.mbox_user().read());
    assert_eq!(finish(hw, index), Answer::data(user.to_le_bytes().to_vec()));

    assert_eq!(
        execute(hw, index, DONE, &[1, 2, 3]),
        Answer {
            status: CMD_COMPLETE,
            data: Vec::new()
        },
        "wrong answer to DONE"
    );

    assert_eq!(
        execute(hw, index, UNKNOWN, &[]),
        Answer::failure(),
        "wrong answer to an unknown command"
    );

    // A command whose length is larger than the SRAM fails. The emulator's
    // SRAM has the largest size that a mailbox can have, and the emulator
    // stops on a length that is larger.
    if sram_size < MAX_SRAM_SIZE {
        start(hw, index, INVERT, sram_size + 1, &[]);
        assert_eq!(
            finish(hw, index),
            Answer::failure(),
            "wrong answer to a command that is larger than the SRAM"
        );
    }

    // The mailbox works after commands that failed.
    assert_eq!(
        execute(hw, index, INVERT, &[0x0f]),
        Answer::data(vec![0xf0])
    );
}

#[test]
fn answers_the_commands_of_a_requester() {
    let files = BootFiles::from_env();
    let mut hw =
        caliptra_mcu_hw_model::new(files.init_params()).expect("hardware model failed to boot");

    let log = run_until(&mut hw, |_, log| log.contains(READY));
    assert!(log.contains(READY), "the app did not start");

    check_mailbox(&mut hw, 0);
    check_mailbox(&mut hw, 1);

    // The app answers a command in each mailbox while the other mailbox
    // holds a command.
    start(&mut hw, 0, INVERT, 2, &[1, 2]);
    start(&mut hw, 1, INVERT, 3, &[3, 4, 5]);
    assert_eq!(finish(&mut hw, 1), Answer::data(inverted(&[3, 4, 5])));
    assert_eq!(finish(&mut hw, 0), Answer::data(inverted(&[1, 2])));
}
