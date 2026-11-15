# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""The apps every async_sim scenario builds, shared by label.

Labels rather than file names, because each scenario is a package of its own
and builds the same sources from there.
"""

HERE = "//target/ast10x0/tests/orchestrator/async_sim:"

APPS = [
    # The system under test: the orchestrator core and its async executor.
    {
        "deps": [
            "//services/orchestrator/server:orchestrator_server",
            "//services/orchestrator/sm:orchestrator_sm",
            "//util/ipc",
            "//util/requester",
            "//util/transport",
            "@pigweed//pw_kernel/userspace",
            "@pigweed//pw_log/rust:pw_log",
            "@pigweed//pw_status/rust:pw_status",
            "@rust_crates//:heapless",
        ],
        "name": "orchestrator",
        "src": HERE + "orchestrator_main.rs",
    },
    # Verifies images after a latency of its own choosing (deferred replies).
    {
        "deps": [
            "//util/service",
            "@pigweed//pw_kernel/userspace",
            "@pigweed//pw_log/rust:pw_log",
            "@pigweed//pw_status/rust:pw_status",
        ],
        "name": "verifier",
        "src": HERE + "verifier_main.rs",
    },
    # The managed devices: each follows a boot script through a device model.
    {
        "deps": [
            "//services/orchestrator/test/device_model:device_model",
            "//util/service",
            "@pigweed//pw_kernel/userspace",
            "@pigweed//pw_log/rust:pw_log",
            "@pigweed//pw_status/rust:pw_status",
            "@rust_crates//:heapless",
        ],
        "name": "devices",
        "src": HERE + "devices_main.rs",
    },
]

SYSTEM_CONFIG = HERE + "system_config"
TARGET_SRC = HERE + "target.rs"
