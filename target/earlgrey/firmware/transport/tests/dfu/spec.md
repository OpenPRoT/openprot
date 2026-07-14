# Transport firmware tests

The first tests we'll implement are DFU firmware-update and ownership transfer
tests.  These tests will use DFU to upload a signed version of our own
`//target/earlgrey/tests/bootinfo` binary signed with a dummy key.  There will
be two versions of this test:
- One version will contain an ownership transfer configuration and we'll expect
  the boot the boot_test after DFU manifestation.
- One version will not contain an ownership transfer configuration and we'll
  expect to reboot and restart the transport firmware.

We will need to update our signing rules.  Examine the changes
to rules/signing.bzl and rules/manifest.bzl in opentitan repo
commit 43dd42d68d52ea8a6d930b66ccf3db491a64ac88.  We'll want to
modify our signing rules (or write new rule) to accomplish the same type of
manifest header manipulation in this code base.

We will need to create keys for the `dummy` owner.  We can copy the keys
from the opentitan repo in `sw/device/silicon_creator/lib/ownership/keys/dummy`.
We'll place a local copy in //target/earlgrey/signing/keys/dummy.

This is a complex test that involves sequencing firmware updates to the device
under test.  As such, we should write a test harness.
We'll examine our own DFU test in //target/earlgrey/tests/usbdfu/host_usb_dfu_check.rs.
The test should load the transport firmware (probably via the bazel
opentitan_test rule), then use the test harness to sequence the DFU update and
examine console output to look for pass/fail criterial.  Consider refactoring
host_usb_dfu_check.rs to refactor out reusable components (like the DFU
implementation) into target/earlgrey/testutil.
