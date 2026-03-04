README.md is a bit outdated, but before tackling that the following information will inform its rewrite:

All tests for AST1060 have been done against a pair of boards on the ASPeed PRoT Fixture 3.0, which features two AST1060 boards on M.2 formfactor daughtercards.

# Appearance and Organization

When the board is facing you with the DB-9 connectors pointed in your direction:

- The LEFT board has a JTAG pigtail installed. The RIGHT does not.
- It is *highly recommended* that the DB-9 interfaces be replaced with direct 3-pin USB to UART headers.

  Pin 1 -> TX

  Pin 2 -> RX

  Pin 3 -> GND

The two pins directly below UART are FWSPICK, which when driven HIGH will force the ROM into UART bootloader mode.
The two pins directly to the RIGHT are #SRST, which when driven LOW will put the processor into RESET.

# Caveats

There is no mechanism for external writing to internal SPI. Rewriting it requires a firmware capable of doing so.

# Operation

The board allows the sequence of SRST# low -> FWSPICK high -> SRST# high to reset and put the device into UART boot mode. For remote management of these pins, a Raspberry Pi is recommended as it provides the environment required to use uart_test_exec.py.

## PI Wires
```
LEFT board: FWSPICK Pin 2 -> RPi GPIO 18
LEFT board: SRST# Pin 1 -> RPi GPIO 23
```

These GPIO pins are coded into uart_test_exec.py as defaults.

For the RIGHT board, choose your own and use the `--fwspick-pin` and `--srst-pin` command-line arguments. Recommended pins are GPIO 24 for FWSPICK and GPIO25 for SRST#

## Command line examples

Basic example that uploads i2c_uart.bin to the device:
```
python3 ./uart_test_exec.py /dev/serial/by-id/usb-FTDI_FT232R_USB_UART_AB80D84B-if00-port0 ./i2c_uart.bin
```

Basic example that uploads a reference firmware to a secondary device:
```
python3 ./uart_test_exec.py --fwspick-pin 24 --srst-pin 25 /dev/serial/by-id/usb-FTDI_FT232R_USB_UART_ABSCEVVK-if00-port0 uart_zephyr.bin
```

### Advanced examples with Pigweed support

The uart_test_exec.py script supports parsing of Pigweed tokenized output.

To make this work, copy the ELF binary and py_tokenizer to the same machine as uart_test_exec.py and set the path to it in the environment variable PW_TOK_ROOT:
```
export PW_TOK_ROOT=/home/amd/pw_tokenizer
```
Then ensure the ELF binary is also accessible to the script, and run it:
```
python3 ./uart_test_exec.py --elf ./i2c.elf --notok /dev/serial/by-id/usb-FTDI_FT232R_USB_UART_AB80D84B-if00-port0 ./i2c_uart.bin
```
The --notok arguments hides the raw base64 tokens. Run
```
uart_test_exec.py --help
```
For more capabilities.
