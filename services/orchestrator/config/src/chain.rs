// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The views the orchestrator and the platform driver take of the device
//! table. Both are derived here, so the table is the only place a board
//! states what its components are.

use crate::device::DeviceConfig;
use openprot_orchestrator_sm::{ComponentAttrs, ComponentId};

/// The entries `chain_of` validated. The field is private, so the only
/// constructor is the const-validated path and holders can trust the
/// invariants without rechecking.
pub struct ChainEntries<const N: usize>([(ComponentId, ComponentAttrs); N]);

impl<const N: usize> ChainEntries<N> {
    /// The validated entries, one per device in table order.
    pub const fn entries(&self) -> &[(ComponentId, ComponentAttrs); N] {
        &self.0
    }
}

/// Table order assigns the ids: `devices[i]` is `ComponentId::new(i)`. The
/// board's per-component arrays are indexed the same way, so the table is
/// also what keeps those arrays lined up with the chain.
///
/// # Panics
///
/// Panics, a build error in const context, if the table is empty or holds
/// more than `u8::MAX` devices (`Chain` takes neither), or if a `depends_on`
/// names anything other than an earlier device in the table.
#[must_use]
pub const fn chain_of<R, P, const N: usize>(devices: &[DeviceConfig<R, P>; N]) -> ChainEntries<N> {
    assert!(N > 0, "a device table needs at least one device");
    assert!(
        N <= u8::MAX as usize,
        "a device table holds at most u8::MAX devices"
    );
    let mut i = 0;
    while i < N {
        if let Some(on) = devices[i].attrs().depends_on {
            let on = on.get() as usize;
            assert!(
                on < i,
                "depends_on must name an earlier device in the table"
            );
        }
        i += 1;
    }

    // Const arrays need a default; the loop below overwrites every element.
    let mut chain = [(ComponentId::new(0), ComponentAttrs::passive_required()); N];
    let mut i = 0;
    while i < N {
        chain[i] = (ComponentId::new(i as u8), devices[i].attrs());
        i += 1;
    }
    ChainEntries(chain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::BootCheckpoint;
    use core::time::Duration;
    use openprot_orchestrator_sm::{Chain, FailurePolicy};

    const CP: BootCheckpoint<u8> = BootCheckpoint::new("up", 1, Duration::from_millis(10));

    const fn dev(attrs: ComponentAttrs) -> DeviceConfig<u8, u8> {
        DeviceConfig::new("dev", 0, &[CP], None, attrs)
    }

    #[test]
    fn ids_come_from_table_order() {
        let table = [
            dev(ComponentAttrs::passive_required()),
            dev(ComponentAttrs::active_isolable()),
        ];
        let validated = chain_of(&table);
        let entries = validated.entries();

        assert_eq!(entries[0].0, ComponentId::new(0));
        assert_eq!(entries[1].0, ComponentId::new(1));
        assert_eq!(entries[1].1.failure_policy, FailurePolicy::Isolable);
    }

    #[test]
    fn the_derived_chain_is_one_the_machine_accepts() {
        let table = [
            dev(ComponentAttrs::passive_required()),
            dev(ComponentAttrs::active_isolable()),
        ];
        let validated = chain_of(&table);
        let entries: heapless::Vec<_, 2> =
            heapless::Vec::from_slice(validated.entries()).expect("same length as the table");

        assert!(Chain::<2>::try_from(entries).is_ok());
    }

    #[test]
    #[should_panic(expected = "at least one device")]
    fn rejects_an_empty_table() {
        let table: [DeviceConfig<u8, u8>; 0] = [];
        let _ = chain_of(&table);
    }

    #[test]
    #[should_panic(expected = "at most u8::MAX")]
    fn rejects_a_table_longer_than_the_chain_takes() {
        let table = [dev(ComponentAttrs::passive_required()); 256];
        let _ = chain_of(&table);
    }

    #[test]
    #[should_panic(expected = "depends_on must name an earlier device")]
    fn rejects_a_dependency_outside_the_table() {
        let mut attrs = ComponentAttrs::passive_required();
        attrs.depends_on = Some(ComponentId::new(4));
        let _ = chain_of(&[dev(attrs)]);
    }

    #[test]
    #[should_panic(expected = "depends_on must name an earlier device")]
    fn rejects_a_self_dependency() {
        let mut attrs = ComponentAttrs::passive_required();
        attrs.depends_on = Some(ComponentId::new(0));
        let _ = chain_of(&[dev(attrs)]);
    }

    #[test]
    #[should_panic(expected = "depends_on must name an earlier device")]
    fn rejects_a_forward_dependency() {
        let mut first = ComponentAttrs::passive_required();
        first.depends_on = Some(ComponentId::new(1));
        let _ = chain_of(&[dev(first), dev(ComponentAttrs::passive_required())]);
    }
}
