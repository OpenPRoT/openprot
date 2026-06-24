# MCTP Server Hardware Test

This test runs the MCTP echo flow between two physical AST1060 EVBs connected
over I2C. One board acts as the MCTP server + requester; the other acts as the
peer server + responder. Both boards flash, boot, and exchange echo packets
simultaneously. The test runs indefinitely or until timeout. There is currently
no success or failure case other than inspecting the logging behaviour.

## Prerequisites

### Hardware

- Two AST1060 EVBs wired I2C bus-to-bus (Bus 2 on each board)
- A Raspberry Pi acting as the test fixture, connected to both boards via:
  - Two USB-to-UART adapters (`/dev/ttyUSB0` for board A, `/dev/ttyUSB1` for board B)
  - GPIO lines for SRST and FWSPICK on each board (see `evb_config.toml`)

### Pi software

The Pi needs Python 3 and `pyserial`. If it is not already installed:

```
pip install pyserial
```

### SSH access from your workstation to the Pi

The test runner connects to the Pi over SSH using **key-based authentication**
(no passwords). To set this up the first time:

1. **Generate a named SSH key** on your workstation:

   ```
   ssh-keygen -t ed25519 -f ~/.ssh/ast_pi
   ```

   You will be prompted for a passphrase.

2. **Copy your public key to the Pi:**

   ```
   ssh-copy-id -i ~/.ssh/ast_pi.pub <your-username>@<pi-hostname>
   ```

   You will be prompted for your password associated with your username on
   the pi host. After this, SSH from your workstation using this key will not
   require the Pi password.

3. **Configure SSH to use this key for the Pi host.** Add the following to
   `~/.ssh/config` on your workstation (create the file if it does not exist):

   ```
   Host <pi-hostname>
       IdentityFile ~/.ssh/ast_pi
       IdentitiesOnly yes
   ```

   This tells SSH to use your named key — and only that key — when connecting
   to the Pi, regardless of which keys your agent currently has loaded.

4. **Add the key to your SSH agent** so the test runner can use it without
   prompting for the passphrase mid-run:

   ```
   ssh-add ~/.ssh/ast_pi
   ```

   You will be prompted for the passphrase once per login session. The agent
   holds the unlocked key in memory until you log out or explicitly remove it.

5. **Verify it works:**

   ```
   ssh <your-username>@<pi-hostname> echo ok
   ```

   You should see `ok` printed without any prompts.

## Running the test

```
bazelisk test \
    --config=k_ast1060_evb \
    --test_env=AST1060_EVB_PI_HOST=<pi-hostname> \
    --test_output=streamed \
    -- //target/ast10x0/tests/mctp/server:mctp_server_test
```

Replace `<pi-hostname>` with the hostname or IP address of your Pi fixture.

The `--test_output=streamed` flag lets you watch the UART output from both
boards in real time as the test runs. The test has no pass/fail sentinel and
runs until you interrupt it with `Ctrl-C` or until Bazel's `eternal` timeout
(3600 seconds) is reached.

### What happens when you run it

1. Bazel builds the firmware images for both boards.
2. `test_runner.py` on your workstation acquires an exclusive lock on the Pi
   (`/tmp/ast1060_evb.lock`) so that concurrent test runs from other users do
   not interfere.
3. The firmware binaries and `pi_test_runner.py` are SCP'd to the Pi.
4. `pi_test_runner.py` sequences the GPIO lines on each board to enter UART
   bootloader mode, uploads the firmware, then streams raw UART bytes back to
   your workstation.
5. UART output is detokenized on the host machine (log messages are decoded
   from their compact tokenized form) and printed to your terminal.
6. The test runs until you interrupt it (`Ctrl-C`) or the `eternal` timeout
   expires.

>Using `Ctrl-C` to stop the test kills the test process without giving it a
>chance to clean up the lockfile. If the lockfile remains untouched for 60
>seconds, it is considered stale. Running tests touch the lock file every 10
>seconds to maintain their lock. As a result of this, you may see a message like
>
>```
>RUNNER: removing stale lock held by jsmith 37m 14s ago, last touched 2089s ago...
>
>````


## If the Pi is already locked

The test runner prints who holds the lock and how long they have held it:

```
RUNNER: Pi locked by jsmith 2m 34s ago, last touched 8s ago; waiting...
```


It will wait up to 120 seconds for the lock to become available. If the previous
test run crashed without releasing the lock, the runner detects that the lock
file has not been touched in over 60 seconds, removes it automatically, and
proceeds:

```
RUNNER: removing stale lock held by jsmith 5m 12s ago, last touched 75s ago...
```

If you need to clear the lock manually for any reason:

```
ssh <pi-hostname> rm -f /tmp/ast1060_evb.lock
```

## Hardware configuration

GPIO pin assignments and serial port paths are defined in
[`target/ast10x0/harness/evb_config.toml`](../../../harness/evb_config.toml):

```toml
[gpio]
srst_pin = 23       # BCM pin connected to board A SRST
fwspick_pin = 18    # BCM pin connected to board A FWSPICK

[uart]
serial_port = "/dev/ttyUSB0"   # Board A serial port
baudrate = 115200

[device_b]
srst_pin = 25       # BCM pin connected to board B SRST
fwspick_pin = 24    # BCM pin connected to board B FWSPICK
serial_port = "/dev/ttyUSB1"   # Board B serial port
```

If your Pi wiring differs, edit `evb_config.toml` before running the test.

>Different Pi hosts might have different configurations. Currently there is no
>association between a config and its host built into the harness.

## Troubleshooting

>We are currently aware of an issue where occasionally the spliced UART streams
>corrupt each other so that one line from each becomes unreadable.

**`ssh: connect to host ... port 22: Connection refused`**
: The Pi is unreachable. Check that the hostname is correct and the Pi is powered on.

**`Permission denied (publickey)`**
: Key-based SSH auth is not set up, or the key is not loaded in your agent.
  Follow the [SSH setup](#ssh-access-from-your-workstation-to-the-pi) steps
  above, and make sure you have run `ssh-add ~/.ssh/ast_pi` this session.

**`RUNNER: timeout acquiring Pi lock after 120s`**
: Another test has been running for over 2 minutes whilst touching the lockfile.
  Wait until the current test is done using the hardware or contact the lock holder shown
  in the waiting messages.

**`Error: could not open /dev/ttyUSB0`**
: The USB-to-UART adapter is not detected on the Pi. Check USB connections and
  verify the device appears with `ls /dev/ttyUSB*` on the Pi.
