// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The records the eRoT keeps across resets.

use crate::layout::{ImageLayout, Region};

/// Names one record the eRoT keeps across resets. Like
/// [`SlotId`], the value is a name and not an index: ids only have to be
/// unique inside one [`RecordLayout`], and the board decides what each one
/// means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordId(pub u8);

/// Where one eRoT-owned record lives: the SVN floor, the lockdown latch, a
/// pending-update record. A record is not an image, so it is never booted,
/// never recovered, and never described by an [`ImageLayout`].
///
/// The store has to be one only the eRoT can write. That is a hardware
/// property the board guarantees and this crate cannot check.
#[derive(Debug, Clone, Copy)]
pub struct RecordRegion {
    id: RecordId,
    region: Region,
}

impl RecordRegion {
    /// Declares where one record lives. Const, so a bad board table fails the
    /// build.
    #[must_use]
    pub const fn new(id: RecordId, region: Region) -> Self {
        Self { id, region }
    }

    /// This region's id, unique within the layout (checked by
    /// [`RecordLayout::new`]).
    #[must_use]
    pub const fn id(&self) -> RecordId {
        self.id
    }

    /// Where this record lives in the eRoT's store.
    #[must_use]
    pub const fn region(&self) -> Region {
        self.region
    }
}

/// Every eRoT-owned record in one store, checked against each other and
/// against the images sharing that store.
///
/// A layout describes one store. Pairing it with that store, and
/// guaranteeing that only the eRoT can write it, is board wiring this crate
/// cannot check.
///
/// Offsets are counted from the start of the store, the same way
/// [`Region`] counts a device's images from the start of that device's
/// area. Records and images can only be compared when they are counted from
/// the same place, which is what the `images` argument of
/// [`new`](Self::new) says: pass the layout of the images in this store,
/// or `None` when the records have a store to themselves.
#[derive(Debug, Clone, Copy)]
pub struct RecordLayout {
    regions: &'static [RecordRegion],
}

impl RecordLayout {
    /// Declares the records in one store. Const, so a board that puts the
    /// SVN floor inside a slot fails the build instead of corrupting the
    /// slot at the first commit.
    ///
    /// # Panics
    ///
    /// Panics if the layout is empty, if two regions share an id, if two
    /// regions overlap, or if a region overlaps one of `images`.
    ///
    /// The overlap check only covers addresses. Two regions that share a
    /// flash erase block still wipe each other, and this crate does not
    /// know the erase block size, so a board checks that in its own const
    /// fence.
    #[must_use]
    pub const fn new(regions: &'static [RecordRegion], images: Option<ImageLayout>) -> Self {
        assert!(!regions.is_empty(), "a record layout must hold a record");
        let mut s = 0;
        while s < regions.len() {
            if let Some(images) = images {
                let slots = images.slots();
                let mut i = 0;
                while i < slots.len() {
                    assert!(
                        !regions[s].region.overlaps(&slots[i].region()),
                        "a record must not overlap a slot"
                    );
                    i += 1;
                }
                if let Some(golden) = images.golden() {
                    assert!(
                        !regions[s].region.overlaps(&golden.region()),
                        "a record must not overlap the golden image"
                    );
                }
            }
            let mut t = s + 1;
            while t < regions.len() {
                assert!(
                    regions[s].id.0 != regions[t].id.0,
                    "record ids must be unique within a layout"
                );
                assert!(
                    !regions[s].region.overlaps(&regions[t].region),
                    "records must not overlap"
                );
                t += 1;
            }
            s += 1;
        }
        Self { regions }
    }

    /// Where the named record lives, or `None` when the layout does not
    /// declare it.
    #[must_use]
    pub const fn region(&self, id: RecordId) -> Option<Region> {
        let mut s = 0;
        while s < self.regions.len() {
            if self.regions[s].id.0 == id.0 {
                return Some(self.regions[s].region);
            }
            s += 1;
        }
        None
    }

    /// Every region the layout declares, in declaration order.
    #[must_use]
    pub const fn regions(&self) -> &'static [RecordRegion] {
        self.regions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Golden, Slot, SlotId};

    // Board tables run the constructors at compile time, where a
    // rejection is a build error nobody can assert on. These tests call
    // them at runtime to prove the reject paths actually fire.

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

    const LAYOUT: ImageLayout = ImageLayout::new(const { &[slot(0), slot(1)] }, Some(GOLDEN));

    /// Records sit above every slot `slot` can place, so the two only
    /// collide when a test means them to.
    const FLOOR: RecordRegion = RecordRegion::new(RecordId(0), Region::new(0xE000_0000, SLOT_LEN));
    const LATCH: RecordRegion = RecordRegion::new(RecordId(1), Region::new(0xE010_0000, SLOT_LEN));

    #[test]
    fn finds_a_record_by_id() {
        let records = RecordLayout::new(const { &[FLOOR, LATCH] }, Some(LAYOUT));
        assert_eq!(records.region(RecordId(1)), Some(LATCH.region()));
        assert_eq!(records.regions().len(), 2);
    }

    #[test]
    fn an_undeclared_record_has_no_region() {
        let records = RecordLayout::new(const { &[FLOOR] }, Some(LAYOUT));
        assert_eq!(records.region(RecordId(7)), None);
    }

    /// Records in a store of their own have no images to be checked against.
    #[test]
    fn accepts_records_without_images() {
        let records = RecordLayout::new(
            const { &[RecordRegion::new(RecordId(0), Region::new(0, 4096))] },
            None,
        );
        assert_eq!(records.region(RecordId(0)).map(|r| r.len()), Some(4096));
    }

    #[test]
    #[should_panic(expected = "record ids must be unique")]
    fn rejects_duplicate_record_ids() {
        let _ = RecordLayout::new(
            const {
                &[
                    RecordRegion::new(RecordId(0), Region::new(0xE000_0000, SLOT_LEN)),
                    RecordRegion::new(RecordId(0), Region::new(0xE010_0000, SLOT_LEN)),
                ]
            },
            None,
        );
    }

    #[test]
    #[should_panic(expected = "records must not overlap")]
    fn rejects_overlapping_records() {
        let _ = RecordLayout::new(
            const {
                &[
                    RecordRegion::new(RecordId(0), Region::new(0xE000_0000, SLOT_LEN)),
                    RecordRegion::new(RecordId(1), Region::new(0xE000_0000, SLOT_LEN)),
                ]
            },
            None,
        );
    }

    #[test]
    #[should_panic(expected = "a record must not overlap a slot")]
    fn rejects_a_record_inside_a_slot() {
        let _ = RecordLayout::new(
            const { &[RecordRegion::new(RecordId(0), Region::new(0, SLOT_LEN))] },
            Some(LAYOUT),
        );
    }

    #[test]
    #[should_panic(expected = "a record must not overlap the golden image")]
    fn rejects_a_record_inside_the_golden_image() {
        let _ = RecordLayout::new(
            const {
                &[RecordRegion::new(
                    RecordId(0),
                    Region::new(0xF000_0000, SLOT_LEN),
                )]
            },
            Some(LAYOUT),
        );
    }

    #[test]
    #[should_panic(expected = "a record layout must hold a record")]
    fn rejects_an_empty_record_layout() {
        let _ = RecordLayout::new(&[], None);
    }
}
