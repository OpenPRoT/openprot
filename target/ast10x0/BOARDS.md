# AST1060 Board Support

At this time, OpenPRoT examples support the AST1060 Test Harness board.

The Customer EVB for the AST1060 is not specifically supported at this time, but is not necessarily incompatible with this firwmare.

## Test Harness Board

The Test Harness board is an ASpeed developed board for testing purposes, and is physically smaller than the EVB and has a different feature set.

### Features

* Two SoC daughter cards, each of which consists of
  * An ASPeed AST1060 SoC
  * A SPI NOR part
  * Headers to control SRST# and FWSPICK
  * Additional support logic
  * An M.2 formfactor edge connector (not electrically M.2)
* A backplane board
  * Headers for most IO channels and point-to-point links between the cards
  * Four SPI NOR parts for testing SPI monitor capabilities
  * UART level converters and DB-9 headers
  * SATA and DC jack for power

The purpose of the board is to allow one AST1060 to serve as a target and one as an initiator. This capability is leveraged in OpenPRoT testing, allowing for I2C, MCTP, and SPDM testing on device as both requester and responder.

### Configuration

OpenPRoT demos expect the board to be configured in a specific manner, both for firmware and scripted control. The demos are built around using a Raspberry Pi to control GPIOs and the UARTs.

To begin, orient the board with the DB-9 connectors facing you.

GPIO connections should be as follows:

| Raspberry Pi Pin | Target Device | Target Pin     |
|------------------|---------------|----------------|
| 23               | A             | SRST (PIN1)    |
| 18               | A             | FWSPICK (PIN2) |
| 25               | B             | SRST (PIN1)    |
| 24               | B             | FWSPICK (PIN2) |

For I2C communication, connect pins 1 and 2 of J15. This links I2C2 between the devices.

Connect both UARTs to the Pi. Either use a USB to Serial adapter, or use a USB UART directly to the 3 pin header directly above FWSPICK.

### Manual Execution

To manually run tests on the target devices, the uart_test_exec.py script is run on the Pi. By default it targets device A (on the left side of the board) but with command line arguments it can also target device B. Passing the script the UART to use and binary, it will reset the device and upload the binary. It is possible to feed in the ELF binary as well to ensure that Pigweed tokens are decoded on the fly.

### Automation

OpenPRoT test automation is derived from the uart_test_exec.py script and similarly expects SSH access to a Pi connected to the Harness in the above manner.
