// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use super::*;

/// Writes `data` to a uniquely named file under the test's temp dir and
/// returns its path. `name` only has to be unique within one test.
fn payload(name: &str, data: &[u8]) -> PathBuf {
    let dir = std::env::var("TEST_TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let path = PathBuf::from(dir).join(format!("flash-image-{name}.bin"));
    fs::write(&path, data).unwrap();
    path
}

fn args(flash_size: u32, slots: Vec<(SlotId, Placement)>) -> Args {
    Args {
        flash_size,
        slots,
        golden: None,
        output: PathBuf::from("unused"),
    }
}

#[test]
fn parse_region_takes_decimal_and_hex() {
    let p = parse_region("0x10:32=some/path").unwrap();
    assert_eq!(p.region.base(), 0x10);
    assert_eq!(p.region.len(), 32);
    assert_eq!(p.payload, PathBuf::from("some/path"));
}

#[test]
fn parse_slot_reads_the_id_first() {
    let (id, p) = parse_slot("3:0:16=x").unwrap();
    assert_eq!(id.0, 3);
    assert_eq!(p.region.len(), 16);
}

#[test]
fn a_spec_without_a_payload_is_refused() {
    assert!(parse_region("0:16").is_err());
}

#[test]
fn a_spec_without_a_length_is_refused() {
    assert!(parse_region("0=x").is_err());
}

#[test]
fn an_unknown_argument_is_refused() {
    let argv = ["--rubbish".to_string(), "1".to_string()];
    assert!(parse_args(argv.into_iter()).is_err());
}

#[test]
fn a_flag_without_its_value_is_refused() {
    let argv = ["--flash-size".to_string()];
    assert!(parse_args(argv.into_iter()).is_err());
}

#[test]
fn payloads_land_at_their_slot_base_and_the_rest_stays_erased() {
    let a = payload("land-a", b"AAAA");
    let b = payload("land-b", b"BB");
    let image = build(&args(
        32,
        vec![
            (
                SlotId(0),
                Placement {
                    region: Region::new(0, 16),
                    payload: a,
                },
            ),
            (
                SlotId(1),
                Placement {
                    region: Region::new(16, 16),
                    payload: b,
                },
            ),
        ],
    ))
    .unwrap();

    assert_eq!(&image[0..4], b"AAAA");
    assert_eq!(&image[4..16], &[ERASED; 12]);
    assert_eq!(&image[16..18], b"BB");
    assert_eq!(&image[18..32], &[ERASED; 14]);
}

#[test]
fn declaration_order_does_not_have_to_match_address_order() {
    let high = payload("order-high", b"H");
    let low = payload("order-low", b"L");
    let image = build(&args(
        32,
        vec![
            (
                SlotId(7),
                Placement {
                    region: Region::new(16, 16),
                    payload: high,
                },
            ),
            (
                SlotId(2),
                Placement {
                    region: Region::new(0, 16),
                    payload: low,
                },
            ),
        ],
    ))
    .unwrap();

    assert_eq!(image[0], b'L');
    assert_eq!(image[16], b'H');
}

#[test]
fn a_payload_larger_than_its_slot_is_refused() {
    let big = payload("too-big", b"AAAAAAAA");
    let err = build(&args(
        32,
        vec![(
            SlotId(0),
            Placement {
                region: Region::new(0, 4),
                payload: big,
            },
        )],
    ))
    .unwrap_err();
    assert!(err.contains("does not fit"), "{err}");
}

#[test]
fn a_slot_past_the_end_of_the_flash_is_refused() {
    let p = payload("past-end", b"A");
    let err = build(&args(
        16,
        vec![(
            SlotId(0),
            Placement {
                region: Region::new(8, 16),
                payload: p,
            },
        )],
    ))
    .unwrap_err();
    assert!(err.contains("past the"), "{err}");
}

#[test]
fn a_missing_payload_is_refused() {
    let err = build(&args(
        16,
        vec![(
            SlotId(0),
            Placement {
                region: Region::new(0, 8),
                payload: PathBuf::from("no/such/file"),
            },
        )],
    ))
    .unwrap_err();
    assert!(err.contains("no/such/file"), "{err}");
}

#[test]
fn the_golden_image_is_written_too() {
    let slot = payload("golden-slot", b"S");
    let golden = payload("golden-image", b"G");
    let image = build(&Args {
        flash_size: 32,
        slots: vec![(
            SlotId(0),
            Placement {
                region: Region::new(0, 16),
                payload: slot,
            },
        )],
        golden: Some(Placement {
            region: Region::new(16, 16),
            payload: golden,
        }),
        output: PathBuf::from("unused"),
    })
    .unwrap();

    assert_eq!(image[0], b'S');
    assert_eq!(image[16], b'G');
}

/// `ImageLayout::new` is what rejects these, not any check in this tool.
/// The point of the panic is that a test image cannot describe a layout
/// the orchestrator would refuse to boot.
#[test]
#[should_panic(expected = "slots must not overlap")]
fn overlapping_slots_are_rejected_by_the_layout() {
    let a = payload("overlap-a", b"A");
    let b = payload("overlap-b", b"B");
    let _ = build(&args(
        64,
        vec![
            (
                SlotId(0),
                Placement {
                    region: Region::new(0, 32),
                    payload: a,
                },
            ),
            (
                SlotId(1),
                Placement {
                    region: Region::new(16, 32),
                    payload: b,
                },
            ),
        ],
    ));
}

#[test]
#[should_panic(expected = "slot ids must be unique")]
fn duplicate_slot_ids_are_rejected_by_the_layout() {
    let a = payload("dup-a", b"A");
    let b = payload("dup-b", b"B");
    let _ = build(&args(
        64,
        vec![
            (
                SlotId(0),
                Placement {
                    region: Region::new(0, 16),
                    payload: a,
                },
            ),
            (
                SlotId(0),
                Placement {
                    region: Region::new(16, 16),
                    payload: b,
                },
            ),
        ],
    ));
}

#[test]
#[should_panic(expected = "region length must not be zero")]
fn a_zero_length_region_is_rejected_by_the_layout() {
    let _ = parse_region("0:0=x");
}
