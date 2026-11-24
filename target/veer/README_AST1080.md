# Booting OpenPRoT on the AST1080 Caliptra Subsystem MCU

## Build

### OpenPRoT

Build: bazel build --config=k_veer --//target/veer:target_type=ast1080 //target/veer/tests/i3c_service:i3c_service_image

### ASpeed Support

Clone: https://github.com/wmaroneAMD/aspeed-mcu-fw-hub/
Checkout: openprot-insert
Build: bazel build //ast1080/ast1080a0/evb:image-openprot

Output: bazel-bin/ast1080/ast1080a0/evb/ast1080-openprot.bin

## Load on EVB (Linux)

* Connect a Dediprog to J67
* Pull J1205 high
* Toggle SRST#
* ./dpcmd -e
* ./dpcmd -p ast1080-openprot.bin
* Pull J1205 low
* Toggle SRST#

## I3C Interactions

Currently in progress...
