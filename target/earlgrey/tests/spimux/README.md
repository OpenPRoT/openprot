# EarlGrey SPIMux E2E Switching and Electrical Verification Test

This test package verifies the SPI MUX switching and reset sequencing logic (`SpiMuxHandler::switch_mux_to`) on OpenTitan EarlGrey hardware.

## Architecture & Design Principles
1. **MUX State as Single Source of Truth (SSOT)**:
   - The physical MUX pins (`SPI_MUX_CTRL` / `IOB8`, `SPI_MUX_EN_N` / `IOB7`, `SPI_RESET_N` / `IOA7`) dictate which external SPI EPROM is connected to `SpiHost0`.
2. **Safe Switching Sequence**:
   - `switch_mux_to` executes a 3-step safe switching sequence:
     1. **Assert Reset**: Pull `SPI_RESET_N` LOW (`0V`) to reset EPROM chips and prevent runt pulses / glitches during switching.
      2. **Switch MUX**: Drive `SPI_MUX_CTRL` LOW (`0V`) for `HostCpu0Earlgrey1` (Host CPU connects to Flash 0, Earlgrey connects to Flash 1) or HIGH (`3.3V`) for `HostCpu1Earlgrey0` (Host CPU connects to Flash 1, Earlgrey connects to Flash 0).
     3. **Release Reset**: Pull `SPI_RESET_N` HIGH (`3.3V`) and enable MUX (`SPI_MUX_EN_N = LOW`), allowing the newly connected EPROM to wake up cleanly.
3. **Quiesce & Handshake**:
   - `platform` coordinates with `flash_service` via IPC notices (`switch_mux_notice` / `switch_mux_fin_notice`) to drain and lock the bus before hardware MUX switching occurs.

## Running Tests
To run the E2E test on an FPGA board (CW340 / `hyper340`):
```bash
bazelisk test --test_output=all //target/earlgrey/tests/spimux:spimux_hyper340_test
```
On FPGA platforms (`hyper310` and `hyper340`), the host harness (`host_spimux_check`) directly samples the DUT GPIO pins via HyperDebug to verify that `SPI_MUX_CTRL`, `SPI_MUX_EN_N`, and `SPI_RESET_N` match the expected electrical voltage levels at every stage of MUX switching.
