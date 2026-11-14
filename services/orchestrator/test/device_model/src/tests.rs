// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::*;

// Macros, not functions: a script is borrowed for the device's lifetime, and
// only a plain literal is promoted to `'static`, not a function call.
macro_rules! boots {
    ($after:expr) => {
        Behaviour::Boots {
            after_millis: $after,
        }
    };
}

macro_rules! dies {
    ($after:expr, $up:expr) => {
        Behaviour::BootsThenFails {
            after_millis: $after,
            up_millis: $up,
        }
    };
}

fn up(at_millis: u64) -> Transition {
    Transition {
        at_millis,
        ready: true,
    }
}

fn down(at_millis: u64) -> Transition {
    Transition {
        at_millis,
        ready: false,
    }
}

#[test]
fn a_new_device_is_held_and_not_ready() {
    let mut d = DeviceModel::new(&[boots!(10)]);

    assert_eq!(d.phase(), Phase::Held);
    assert!(!d.ready());
    assert_eq!(d.releases(), 0);
    assert_eq!(d.next_event_at(), None);
    assert_eq!(d.take_transition(), None);
    // Polling does not release it.
    assert_eq!(d.poll(1_000), Phase::Held);
}

#[test]
fn it_boots_after_the_delay_counted_from_the_release() {
    let mut d = DeviceModel::new(&[boots!(50)]);
    d.set_reset(false, 100);

    assert_eq!(d.poll(100), Phase::Booting);
    assert_eq!(d.next_event_at(), Some(150));
    assert_eq!(d.poll(149), Phase::Booting);
    assert!(!d.ready());

    assert_eq!(d.poll(150), Phase::Ready);
    assert!(d.ready());
    assert_eq!(d.take_transition(), Some(up(150)));
    assert_eq!(d.next_event_at(), None);
}

// The caller's polling is coarse; the boot still happened at its due time.
#[test]
fn a_late_poll_stamps_the_due_time_not_the_poll_time() {
    let mut d = DeviceModel::new(&[boots!(50)]);
    d.set_reset(false, 0);

    assert_eq!(d.poll(1_000), Phase::Ready);
    assert_eq!(d.take_transition(), Some(up(50)));
}

#[test]
fn a_zero_delay_is_ready_at_the_release() {
    let mut d = DeviceModel::new(&[boots!(0)]);
    d.set_reset(false, 10);

    assert_eq!(d.poll(10), Phase::Ready);
    assert_eq!(d.take_transition(), Some(up(10)));
}

#[test]
fn a_hanging_device_never_reports_ready() {
    let mut d = DeviceModel::new(&[Behaviour::Hang]);
    d.set_reset(false, 0);

    assert_eq!(d.poll(u64::MAX), Phase::Hung);
    assert!(!d.ready());
    assert_eq!(d.next_event_at(), None);
    assert_eq!(d.take_transition(), None);
}

// Reset mid-boot throws the attempt away: the delay starts over from the
// next release.
#[test]
fn a_reset_mid_boot_restarts_the_delay() {
    let mut d = DeviceModel::new(&[boots!(50)]);
    d.set_reset(false, 0);
    d.set_reset(true, 30);

    assert_eq!(d.poll(60), Phase::Held);
    assert!(!d.ready());

    d.set_reset(false, 40);
    assert_eq!(d.releases(), 2);
    assert_eq!(d.poll(89), Phase::Booting);
    assert_eq!(d.poll(90), Phase::Ready);
    // The first attempt never reported, so there is exactly one rise.
    assert_eq!(d.take_transition(), Some(up(90)));
    assert_eq!(d.take_transition(), None);
}

#[test]
fn asserting_reset_drops_ready() {
    let mut d = DeviceModel::new(&[boots!(10)]);
    d.set_reset(false, 0);
    d.poll(10);
    assert_eq!(d.take_transition(), Some(up(10)));

    d.set_reset(true, 25);

    assert_eq!(d.phase(), Phase::Held);
    assert!(!d.ready());
    assert_eq!(d.take_transition(), Some(down(25)));
}

// A pulse that rose and fell between two polls is still there to be drained.
#[test]
fn a_pulse_between_polls_is_not_lost() {
    let mut d = DeviceModel::new(&[boots!(10)]);
    d.set_reset(false, 0);

    // The caller never polled while the device was up.
    d.set_reset(true, 15);

    assert!(!d.ready());
    assert_eq!(d.take_transition(), Some(up(10)));
    assert_eq!(d.take_transition(), Some(down(15)));
}

// Each release is a new attempt; the script says what each one does.
#[test]
fn the_script_is_indexed_by_release_and_the_last_entry_repeats() {
    let mut d = DeviceModel::new(&[Behaviour::Hang, boots!(20)]);

    d.set_reset(false, 0);
    assert_eq!(d.poll(1_000), Phase::Hung, "first release hangs");

    d.set_reset(true, 5);
    d.set_reset(false, 10);
    assert_eq!(d.poll(29), Phase::Booting);
    assert_eq!(d.poll(30), Phase::Ready, "second release boots");

    d.set_reset(true, 40);
    d.set_reset(false, 50);
    assert_eq!(d.poll(70), Phase::Ready, "third repeats the last entry");
    assert_eq!(d.releases(), 3);
}

#[test]
fn a_device_can_boot_and_then_die() {
    let mut d = DeviceModel::new(&[dies!(20, 30)]);
    d.set_reset(false, 0);

    assert_eq!(d.poll(20), Phase::Ready);
    assert_eq!(d.next_event_at(), Some(50));
    assert_eq!(d.poll(49), Phase::Ready);

    assert_eq!(d.poll(50), Phase::Down);
    assert!(!d.ready());
    assert_eq!(d.next_event_at(), None);
    assert_eq!(d.take_transition(), Some(up(20)));
    assert_eq!(d.take_transition(), Some(down(50)));
}

#[test]
fn a_late_poll_applies_both_changes_in_order() {
    let mut d = DeviceModel::new(&[dies!(20, 30)]);
    d.set_reset(false, 0);

    assert_eq!(d.poll(1_000), Phase::Down);
    assert_eq!(d.take_transition(), Some(up(20)));
    assert_eq!(d.take_transition(), Some(down(50)));
}

#[test]
fn a_dead_device_comes_back_only_through_reset() {
    let mut d = DeviceModel::new(&[dies!(10, 10), boots!(10)]);
    d.set_reset(false, 0);
    assert_eq!(d.poll(100), Phase::Down);

    // Nothing but a reset brings it back.
    assert_eq!(d.poll(10_000), Phase::Down);

    d.set_reset(true, 200);
    d.set_reset(false, 210);
    assert_eq!(d.poll(220), Phase::Ready);
    assert_eq!(d.poll(1_000_000), Phase::Ready, "the second entry stays up");
}

#[test]
fn next_event_at_follows_the_phase() {
    let mut d = DeviceModel::new(&[dies!(10, 5), Behaviour::Hang, boots!(1)]);
    assert_eq!(d.next_event_at(), None, "held");

    d.set_reset(false, 0);
    assert_eq!(d.next_event_at(), Some(10), "booting");
    d.poll(10);
    assert_eq!(d.next_event_at(), Some(15), "ready, about to fail");
    d.poll(15);
    assert_eq!(d.next_event_at(), None, "down");

    d.set_reset(true, 20);
    d.set_reset(false, 20);
    assert_eq!(d.next_event_at(), None, "hung");

    d.set_reset(true, 30);
    d.set_reset(false, 30);
    d.poll(31);
    assert_eq!(d.phase(), Phase::Ready);
    assert_eq!(d.next_event_at(), None, "ready and stable");
}

// A caller may set the line every wake; only a real edge counts.
#[test]
fn repeating_a_level_changes_nothing() {
    let mut d = DeviceModel::new(&[boots!(10)]);

    d.set_reset(true, 0);
    d.set_reset(true, 1);
    assert_eq!(d.releases(), 0);

    d.set_reset(false, 2);
    d.set_reset(false, 3);
    d.set_reset(false, 4);
    assert_eq!(d.releases(), 1, "released once");
    assert_eq!(
        d.poll(12),
        Phase::Ready,
        "the delay still runs from the first release"
    );
}

#[test]
fn an_empty_script_hangs() {
    let mut d = DeviceModel::new(&[]);
    d.set_reset(false, 0);

    assert_eq!(d.poll(1_000), Phase::Hung);
}

#[test]
fn transitions_beyond_the_queue_are_counted_not_kept() {
    let mut d = DeviceModel::new(&[dies!(1, 1)]);
    // Three boot-then-die cycles, never drained: six transitions, room for four.
    for cycle in 0..3u64 {
        let t = cycle * 100;
        d.set_reset(true, t);
        d.set_reset(false, t);
        d.poll(t + 50);
    }

    assert_eq!(d.dropped(), 2);
    assert_eq!(d.take_transition(), Some(up(1)));
    assert_eq!(d.take_transition(), Some(down(2)));
    assert_eq!(d.take_transition(), Some(up(101)));
    assert_eq!(d.take_transition(), Some(down(102)));
    assert_eq!(d.take_transition(), None);
}

#[test]
fn the_queue_is_usable_again_once_drained() {
    let mut d = DeviceModel::new(&[dies!(1, 1)]);
    for cycle in 0..3u64 {
        let t = cycle * 100;
        d.set_reset(true, t);
        d.set_reset(false, t);
        d.poll(t + 50);
    }
    while d.take_transition().is_some() {}

    d.set_reset(true, 1_000);
    d.set_reset(false, 1_000);
    d.poll(1_050);

    assert_eq!(d.take_transition(), Some(up(1_001)));
    assert_eq!(d.take_transition(), Some(down(1_002)));
}

#[test]
fn a_clock_that_goes_backwards_does_not_panic() {
    let mut d = DeviceModel::new(&[boots!(50)]);
    d.set_reset(false, 100);

    assert_eq!(d.poll(40), Phase::Booting, "earlier than the release");
    d.set_reset(true, 10);
    assert_eq!(d.phase(), Phase::Held);
}

#[test]
fn a_huge_delay_saturates_instead_of_overflowing() {
    let mut d = DeviceModel::new(&[boots!(u64::MAX)]);
    d.set_reset(false, 100);

    assert_eq!(d.next_event_at(), Some(u64::MAX));
    assert_eq!(d.poll(u64::MAX - 1), Phase::Booting);
}
