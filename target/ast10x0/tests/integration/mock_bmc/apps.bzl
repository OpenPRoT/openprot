# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""The apps in the boot scenario, shared with its negative case.

Labels rather than file names, because the negative scenario is a package
of its own and builds the same sources from there.
"""

HERE = "//target/ast10x0/tests/integration/mock_bmc:"

MOCK_BMC_DEPS = [
    "//hal/blocking",
    "//services/test/mock-bmc:mock_bmc",
    "@pigweed//pw_kernel/userspace",
    "@pigweed//pw_log/rust:pw_log",
    "@pigweed//pw_status/rust:pw_status",
    "@rust_crates//:embedded-hal",
]

ORCHESTRATOR_DEPS = [
    "//services/orchestrator/adapters/walk:orchestrator_checkpoint_walk",
    "//services/orchestrator/capabilities:orchestrator_capabilities",
    "//services/orchestrator/config:orchestrator_config",
    "//services/orchestrator/driver:orchestrator_driver",
    "//services/orchestrator/sm:orchestrator_sm",
    "//util/io",
    "@pigweed//pw_kernel/userspace",
    "@pigweed//pw_log/rust:pw_log",
    "@pigweed//pw_status/rust:pw_status",
]

APPS = [
    # The managed device: drives its ready line in response to its reset line.
    {
        "deps": MOCK_BMC_DEPS,
        "name": "mock_bmc",
        "src": HERE + "mock_bmc_main.rs",
    },
    # The RoT: releases the device and judges whether it came up.
    {
        "deps": ORCHESTRATOR_DEPS,
        "name": "orchestrator",
        "src": HERE + "orchestrator_main.rs",
    },
]

SYSTEM_CONFIG = HERE + "system_config"
TARGET_SRC = HERE + "target.rs"
