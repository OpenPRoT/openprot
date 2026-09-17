// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Schema for the per-board device table. Board device tables
//! (`target/<board>/devices.rs`) declare the values; no concrete line or
//! device is named here.
//!
//! Invariants are enforced in the `const fn` constructors, so an invalid
//! table is a build error and there is no validate step to forget. Checks
//! on board-defined types belong next to the table that gives them
//! meaning (`services/orchestrator/test/devices.rs` shows the pattern).

#![cfg_attr(not(test), no_std)]

/// One boot checkpoint: a signal the orchestrator waits for, and how long
/// it waits. Retry policy is deliberately not table data: a retry
/// re-resets the device and re-runs the whole walk, so budgets are
/// per boot attempt and owned by the orchestrator state machine.
///
/// The signal is a board-defined id — the schema attaches no meaning to
/// it and names no signal kinds. Each board defines its own vocabulary (a
/// small enum: a GPIO line, a progress-register threshold, a message-path
/// readiness) and gives it meaning in its `EvidenceReader`. The id is a
/// defunctionalized evidence check: data in the table instead of a
/// function, so the table stays printable, comparable, const-checkable —
/// and could one day be generated instead of written.
///
/// Fields are private so a checkpoint that violates the schema is
/// unrepresentable: [`new`](Self::new) is the only way in, and it checks.
#[derive(Debug, Clone, Copy)]
pub struct BootCheckpoint<G> {
    name: &'static str,
    signal: G,
    timeout: core::time::Duration,
}

impl<G> BootCheckpoint<G> {
    /// Declares a checkpoint. `const`, so board tables run the checks at
    /// build time.
    ///
    /// # Panics
    ///
    /// Panics — a build error in const context — if `name` is empty or
    /// `timeout` is zero.
    #[must_use]
    pub const fn new(name: &'static str, signal: G, timeout: core::time::Duration) -> Self {
        assert!(!name.is_empty(), "checkpoint name must not be empty");
        assert!(!timeout.is_zero(), "checkpoint timeout must not be zero");
        Self {
            name,
            signal,
            timeout,
        }
    }

    /// Names the checkpoint in failure reports ("bl1", "kernel", …).
    /// Unique within a device's checkpoint list.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Board-defined signal id, resolved by the board's `EvidenceReader`
    /// (in `orchestrator-capabilities`). An id rather than a function, so
    /// the table stays pure data — the type-level docs say why.
    #[must_use]
    pub const fn signal(&self) -> &G {
        &self.signal
    }

    /// Window for one attempt at this checkpoint. Expiry is the boot
    /// walk's own judgment; hung devices report nothing.
    ///
    /// The orchestrator state machine never sees this value — it is
    /// clockless. The walk consumes the windows and reports expiry as a
    /// failed attempt; a component's whole boot timeout is nothing more
    /// than its walk over these windows, in order.
    #[must_use]
    pub const fn timeout(&self) -> core::time::Duration {
        self.timeout
    }
}

/// Names one slot in one device's layout. The value is a name, not an
/// index: ids only have to be unique inside one layout, which
/// [`ImageLayout::new`] checks. They do not have to be in order, next to
/// each other, or start at zero, and slot 0 on the BMC has nothing to do
/// with slot 0 on the NIC. Recovery order comes from the order slots are
/// declared in, not from the id values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotId(pub u8);

/// A byte range in the store that holds one device's images. Offsets are
/// counted from the start of that device's area, so the platform driver is
/// the one that knows which part holds it and where the area starts.
///
/// Base and length describe any store the eRoT addresses directly. That is
/// flash today, and an EEPROM or a memory-mapped part would have the same
/// shape. An image with no address, streamed or held inside a self-updating
/// device, is not described this way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    base: u32,
    len: u32,
}

impl Region {
    /// Declares a region. Const, so a bad board table fails the build
    /// instead of the boot.
    ///
    /// # Panics
    ///
    /// Panics if `len` is zero, or if `base + len` would run past the end
    /// of the 32-bit offset range.
    #[must_use]
    pub const fn new(base: u32, len: u32) -> Self {
        assert!(len > 0, "region length must not be zero");
        assert!(
            base.checked_add(len).is_some(),
            "region must not run past the end of the offset space"
        );
        Self { base, len }
    }

    /// Offset of the first byte, from the start of the device's firmware
    /// partition.
    #[must_use]
    pub const fn base(&self) -> u32 {
        self.base
    }

    /// Length in bytes: how much the region holds, not the size of the
    /// image now in it.
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.len
    }

    /// Offset one past the last byte.
    #[must_use]
    pub const fn end(&self) -> u32 {
        self.base + self.len
    }

    /// Whether the two regions share a byte.
    const fn overlaps(&self, other: &Region) -> bool {
        self.base < other.end() && other.base < self.end()
    }
}

/// One slot in a device's layout. The number of slots is data, so a layout
/// is A/B, a single slot, or any other count depending on what the board
/// declares, and no layout shape is named in code.
///
/// Every slot can be written and booted. The golden image can be neither,
/// and it is not a slot, so there is no flag to set and nothing to check.
#[derive(Debug, Clone, Copy)]
pub struct Slot {
    id: SlotId,
    region: Region,
}

impl Slot {
    /// Declares one slot. Const, so board tables still build at compile
    /// time. Rules that cover a whole layout (unique ids, no overlap) are
    /// checked by [`ImageLayout::new`], which sees the whole list.
    #[must_use]
    pub const fn new(id: SlotId, region: Region) -> Self {
        Self { id, region }
    }

    /// This slot's id, unique within the layout (checked by
    /// [`ImageLayout::new`]).
    #[must_use]
    pub const fn id(&self) -> SlotId {
        self.id
    }

    /// Where this slot lives in the device's image store.
    #[must_use]
    pub const fn region(&self) -> Region {
        self.region
    }
}

/// The golden image: the last image recovery falls back to, and the one
/// image the eRoT never writes.
///
/// It is not a slot, so code that walks the slot list to pick a write
/// target, or to check the SVN floor, never sees it. The floor has to skip
/// it. A golden image's security version is fixed when the board is made,
/// so once the floor moves past that version, checking the floor would make
/// the last image that still boots unbootable.
///
/// Skipping the floor is only safe while the image really cannot be
/// written, which the board has to guarantee in hardware: a write-protect
/// pin, a locked flash block, or a separate part. This crate cannot check
/// that.
#[derive(Debug, Clone, Copy)]
pub struct Golden {
    region: Region,
}

impl Golden {
    /// Declares a layout's golden image.
    #[must_use]
    pub const fn new(region: Region) -> Self {
        Self { region }
    }

    /// Where the golden image lives in the device's image store.
    #[must_use]
    pub const fn region(&self) -> Region {
        self.region
    }
}

/// Every image of one device that the eRoT can address: the slots it
/// writes, plus the golden image. Slots are declared in recovery order:
/// recovery tries them from top to bottom and falls back to the golden
/// image last.
///
/// A device has a layout when the eRoT owns its flash and writes its
/// images. A device that takes its own updates over PLDM and picks what it
/// boots has no layout, and the eRoT never names a byte range for it. That
/// is all [`DeviceConfig::layout`] being an `Option` says; how the eRoT
/// talks to such a device is decided elsewhere.
#[derive(Debug, Clone, Copy)]
pub struct ImageLayout {
    slots: &'static [Slot],
    golden: Golden,
}

impl ImageLayout {
    /// Declares a device's images. Const, so a bad layout fails the
    /// build.
    ///
    /// # Panics
    ///
    /// Panics if two slots share an id, or if two regions overlap, the
    /// golden image included. Overlapping regions would let an update to
    /// one image corrupt another.
    ///
    /// The overlap check only covers addresses. Two regions that share a
    /// flash erase block still wipe each other even though they do not
    /// overlap, and this crate does not know the erase block size, so a
    /// board checks that in its own const fence.
    #[must_use]
    pub const fn new(slots: &'static [Slot], golden: Golden) -> Self {
        let mut s = 0;
        while s < slots.len() {
            assert!(
                !slots[s].region.overlaps(&golden.region),
                "a slot must not overlap the golden image"
            );
            let mut t = s + 1;
            while t < slots.len() {
                assert!(
                    slots[s].id.0 != slots[t].id.0,
                    "slot ids must be unique within a layout"
                );
                assert!(
                    !slots[s].region.overlaps(&slots[t].region),
                    "slots must not overlap"
                );
                t += 1;
            }
            s += 1;
        }
        Self { slots, golden }
    }

    /// The slots the eRoT writes, in declaration order. Empty when the
    /// golden image is the device's only image.
    #[must_use]
    pub const fn slots(&self) -> &'static [Slot] {
        self.slots
    }

    /// The golden image.
    #[must_use]
    pub const fn golden(&self) -> Golden {
        self.golden
    }

    /// How many images recovery can try: every slot, then the golden
    /// image.
    #[must_use]
    pub const fn image_count(&self) -> usize {
        self.slots.len() + 1
    }
}

/// Fails the build if `max_retry` is too small for a device to boot every
/// image. Recovery restores one image per attempt, and the orchestrator
/// counts the restore before it decides whether to boot, so the last
/// restore it allows is never booted. A device with two slots and a golden
/// image therefore needs four attempts for the golden image to run, not
/// three.
///
/// This is a floor, not the exact number. A board whose driver retries the
/// same image before stepping to the next one needs more attempts than
/// this. Devices with no layout are skipped, because the eRoT has no images
/// to step through for them.
///
/// Boards call it from a const fence, so a budget that is too small fails
/// the build:
///
/// ```ignore
/// const _: () = assert_retry_reaches_every_image(MAX_RETRY, MANAGED_DEVICES);
/// ```
///
/// # Panics
///
/// Panics if `max_retry` is not greater than a device's image count.
pub const fn assert_retry_reaches_every_image<R, G>(max_retry: u8, devices: &[DeviceConfig<R, G>]) {
    let mut i = 0;
    while i < devices.len() {
        match devices[i].layout() {
            Some(layout) => assert!(
                max_retry as usize > layout.image_count(),
                "max_retry is too small to boot every image of a device"
            ),
            None => {}
        }
        i += 1;
    }
}

/// One managed downstream device, as declared by the board config.
///
/// Generic over the board's reset signal type `R` (which must match the
/// `ResetId` of the reset controller behind the board's `BootControl`
/// implementation) and its boot-signal vocabulary `G`, for the same
/// reason: signal ids are board-specific.
///
/// Deliberately says nothing about attestation or commit requirements:
/// those follow from what kind of device this is (iRoT-backed or
/// symbiont, the orchestrator's `ComponentKind`), not from a table
/// setting — a second knob would only let the two disagree.
///
/// Fields are private so a device entry that violates the schema is
/// unrepresentable: [`new`](Self::new) is the only way in, and it checks.
#[derive(Debug, Clone, Copy)]
pub struct DeviceConfig<R, G: 'static> {
    name: &'static str,
    reset_signal: R,
    checkpoints: &'static [BootCheckpoint<G>],
    layout: Option<ImageLayout>,
}

impl<R, G> DeviceConfig<R, G> {
    /// Declares a managed device. `const`, so board tables run the checks
    /// at build time.
    ///
    /// # Panics
    ///
    /// Panics — a build error in const context — if `name` is empty, if
    /// `checkpoints` is empty, if two checkpoints share a name (failure
    /// reports identify a checkpoint by name; a duplicate would make them
    /// ambiguous). Layout rules are checked by [`ImageLayout::new`].
    #[must_use]
    pub const fn new(
        name: &'static str,
        reset_signal: R,
        checkpoints: &'static [BootCheckpoint<G>],
        layout: Option<ImageLayout>,
    ) -> Self {
        assert!(!name.is_empty(), "device name must not be empty");
        assert!(
            !checkpoints.is_empty(),
            "device must declare at least one boot checkpoint"
        );
        let mut c = 0;
        while c < checkpoints.len() {
            let mut d = c + 1;
            while d < checkpoints.len() {
                assert!(
                    !str_eq(checkpoints[c].name, checkpoints[d].name),
                    "checkpoint names must be unique per device"
                );
                d += 1;
            }
            c += 1;
        }
        Self {
            name,
            reset_signal,
            checkpoints,
            layout,
        }
    }

    /// The device's name in reports and logs.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Reset signal id, passed to HalBootControl::new.
    #[must_use]
    pub const fn reset_signal(&self) -> &R {
        &self.reset_signal
    }

    /// Boot checkpoints, in the order the device passes them. The device
    /// counts as booted when the last one is reached; a checkpoint whose
    /// window expires fails the attempt — whether to retry or recover is
    /// the orchestrator's decision, not table data.
    #[must_use]
    pub const fn checkpoints(&self) -> &'static [BootCheckpoint<G>] {
        self.checkpoints
    }

    /// This device's images, when the eRoT owns its flash. `None` when the
    /// device takes its own updates, where the eRoT never addresses a byte
    /// range.
    #[must_use]
    pub const fn layout(&self) -> Option<ImageLayout> {
        self.layout
    }
}

// `==` on `&str` is not const; compare bytes by hand.
const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;

    // Board tables run the constructors at compile time, where a
    // rejection is a build error nobody can assert on. These tests call
    // them at runtime to prove the reject paths actually fire.

    const BOOT_COMPLETE: BootCheckpoint<u8> =
        BootCheckpoint::new("boot-complete", 0, Duration::from_secs(1));

    // Same name, different signal: each checkpoint is individually valid,
    // so the pair only trips the device-level duplicate check.
    const BOOT_COMPLETE_DUPLICATE_NAME: BootCheckpoint<u8> =
        BootCheckpoint::new("boot-complete", 1, Duration::from_secs(1));

    /// Slots are one megabyte each; the values only have to be distinct
    /// and non-overlapping.
    const SLOT_LEN: u32 = 0x10_0000;

    /// An ordinary slot, placed by id so two of them never overlap by
    /// accident.
    const fn slot(id: u8) -> Slot {
        Slot::new(SlotId(id), Region::new(id as u32 * SLOT_LEN, SLOT_LEN))
    }

    /// The golden image, above every slot `slot` can place.
    const GOLDEN: Golden = Golden::new(Region::new(0xF000_0000, SLOT_LEN));

    const LAYOUT: ImageLayout = ImageLayout::new(const { &[slot(0), slot(1)] }, GOLDEN);

    #[test]
    fn accepts_a_valid_table() {
        let device = DeviceConfig::new("dev", 0u8, &[BOOT_COMPLETE], Some(LAYOUT));
        assert_eq!(device.name(), "dev");
        assert_eq!(*device.reset_signal(), 0);
        assert_eq!(device.checkpoints().len(), 1);
        assert_eq!(device.checkpoints()[0].name(), "boot-complete");
        assert_eq!(*device.checkpoints()[0].signal(), 0);
        assert_eq!(device.checkpoints()[0].timeout(), Duration::from_secs(1));

        let layout = device.layout().expect("declared above");
        assert_eq!(layout.slots().len(), 2);
        assert_eq!(layout.slots()[1].region().base(), SLOT_LEN);
        assert_eq!(layout.slots()[1].region().end(), 2 * SLOT_LEN);
        assert_eq!(layout.golden().region().base(), 0xF000_0000);
    }

    /// A device that owns its own images declares no layout at all, golden
    /// included: the eRoT never addresses a byte range for it.
    #[test]
    fn accepts_a_device_without_a_layout() {
        let device = DeviceConfig::new("dev", 0u8, &[BOOT_COMPLETE], None);
        assert!(device.layout().is_none());
    }

    /// The golden image may be a device's only image.
    #[test]
    fn accepts_a_layout_with_no_slots() {
        let layout = ImageLayout::new(&[], GOLDEN);
        assert!(layout.slots().is_empty());
    }

    #[test]
    #[should_panic(expected = "checkpoint names must be unique")]
    fn rejects_duplicate_checkpoint_names() {
        let _ = DeviceConfig::new(
            "dev",
            0u8,
            &[BOOT_COMPLETE, BOOT_COMPLETE_DUPLICATE_NAME],
            None,
        );
    }

    #[test]
    #[should_panic(expected = "device name must not be empty")]
    fn rejects_an_empty_device_name() {
        let _ = DeviceConfig::new("", 0u8, &[BOOT_COMPLETE], None);
    }

    #[test]
    #[should_panic(expected = "at least one boot checkpoint")]
    fn rejects_an_empty_checkpoint_list() {
        let _ = DeviceConfig::new("dev", 0u8, &[] as &[BootCheckpoint<u8>], None);
    }

    #[test]
    #[should_panic(expected = "slot ids must be unique")]
    fn rejects_duplicate_slot_ids() {
        let _ = ImageLayout::new(
            const {
                &[
                    Slot::new(SlotId(0), Region::new(0, SLOT_LEN)),
                    Slot::new(SlotId(0), Region::new(SLOT_LEN, SLOT_LEN)),
                ]
            },
            GOLDEN,
        );
    }

    #[test]
    #[should_panic(expected = "slots must not overlap")]
    fn rejects_overlapping_slots() {
        let _ = ImageLayout::new(
            const {
                &[
                    Slot::new(SlotId(0), Region::new(0, SLOT_LEN)),
                    Slot::new(SlotId(1), Region::new(SLOT_LEN - 1, SLOT_LEN)),
                ]
            },
            GOLDEN,
        );
    }

    #[test]
    #[should_panic(expected = "must not overlap the golden image")]
    fn rejects_a_slot_overlapping_the_golden_image() {
        let _ = ImageLayout::new(
            const { &[Slot::new(SlotId(0), Region::new(0xF000_0000, SLOT_LEN))] },
            GOLDEN,
        );
    }

    /// Regions that touch do not overlap: `end` is one past the last
    /// byte.
    #[test]
    fn accepts_adjacent_regions() {
        let layout = ImageLayout::new(
            const {
                &[
                    Slot::new(SlotId(0), Region::new(0, SLOT_LEN)),
                    Slot::new(SlotId(1), Region::new(SLOT_LEN, SLOT_LEN)),
                ]
            },
            GOLDEN,
        );
        assert_eq!(
            layout.slots()[0].region().end(),
            layout.slots()[1].region().base()
        );
    }

    #[test]
    fn counts_every_slot_and_the_golden_image() {
        assert_eq!(LAYOUT.image_count(), 3);
        assert_eq!(ImageLayout::new(&[], GOLDEN).image_count(), 1);
    }

    /// Two slots and a golden image need four attempts: three restores
    /// plus the one the last restore would otherwise never get.
    #[test]
    fn accepts_a_retry_budget_that_boots_the_golden_image() {
        const DEVICES: &[DeviceConfig<u8, u8>] =
            &[DeviceConfig::new("dev", 0, &[BOOT_COMPLETE], Some(LAYOUT))];
        assert_retry_reaches_every_image(4, DEVICES);
    }

    /// A device with no layout has no images to step through, so any
    /// budget covers it.
    #[test]
    fn accepts_any_retry_budget_for_a_device_without_a_layout() {
        const DEVICES: &[DeviceConfig<u8, u8>] =
            &[DeviceConfig::new("dev", 0, &[BOOT_COMPLETE], None)];
        assert_retry_reaches_every_image(0, DEVICES);
    }

    #[test]
    /// Three attempts restore the golden image and stop before booting it.
    #[should_panic(expected = "max_retry is too small")]
    fn rejects_a_retry_budget_that_never_boots_the_golden_image() {
        const DEVICES: &[DeviceConfig<u8, u8>] =
            &[DeviceConfig::new("dev", 0, &[BOOT_COMPLETE], Some(LAYOUT))];
        assert_retry_reaches_every_image(3, DEVICES);
    }

    #[test]
    #[should_panic(expected = "region length must not be zero")]
    fn rejects_a_zero_length_region() {
        let _ = Region::new(0, 0);
    }

    #[test]
    #[should_panic(expected = "must not run past the end of the offset space")]
    fn rejects_a_region_past_the_end_of_the_offset_space() {
        let _ = Region::new(u32::MAX, 1);
    }

    #[test]
    #[should_panic(expected = "checkpoint name must not be empty")]
    fn rejects_an_empty_checkpoint_name() {
        let _ = BootCheckpoint::new("", 0u8, Duration::from_secs(1));
    }

    #[test]
    #[should_panic(expected = "checkpoint timeout must not be zero")]
    fn rejects_a_zero_checkpoint_timeout() {
        let _ = BootCheckpoint::new("boot-complete", 0u8, Duration::ZERO);
    }
}
