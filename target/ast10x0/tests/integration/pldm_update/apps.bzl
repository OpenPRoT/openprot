# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""The apps in the update scenario, shared with its negative cases.

Labels rather than file names, because each negative scenario is a package
of its own and builds the same sources from there.
"""

HERE = "//target/ast10x0/tests/integration/pldm_update:"

APPS = [
    # Both MCTP servers and the loopback between them: the wire.
    {
        "deps": [
            "//services/mctp/api:mctp_api",
            "//services/mctp/server:mctp_server_lib",
            "//services/mctp/transport-loopback:mctp_transport_loopback",
            "@pigweed//pw_kernel/userspace",
            "@pigweed//pw_log/rust:pw_log",
            "@pigweed//pw_status/rust:pw_status",
            "@rust_crates//:mctp",
        ],
        "name": "mctp_bus",
        "src": HERE + "mctp_bus_main.rs",
    },
    # The RoT the firmware device answers to.
    {
        "deps": [
            "//services/orchestrator/adapters/walk:orchestrator_checkpoint_walk",
            "//services/orchestrator/capabilities:orchestrator_capabilities",
            "//services/orchestrator/config:orchestrator_config",
            "//services/orchestrator/driver:orchestrator_driver",
            "//services/orchestrator/sm:orchestrator_sm",
            "//util/io",
            "@pigweed//pw_kernel/userspace",
            "@pigweed//pw_log/rust:pw_log",
            "@pigweed//pw_status/rust:pw_status",
        ],
        "name": "orchestrator",
        "src": HERE + "orchestrator_main.rs",
    },
    # The DSP0267 firmware device.
    {
        "deps": [
            "//services/mctp/client-ipc:mctp_client_ipc",
            "//services/pldm:pldm_service",
            "@pigweed//pw_kernel/userspace",
            "@pigweed//pw_log/rust:pw_log",
            "@pigweed//pw_status/rust:pw_status",
            "@rust_crates//:pldm-common",
            "@rust_crates//:pldm-interface",
        ],
        "name": "pldm_fd",
        "src": HERE + "pldm_fd_main.rs",
    },
    # The update agent, the BMC's half of the update.
    {
        "deps": [
            "//services/mctp/client-ipc:mctp_client_ipc",
            "//services/pldm:pldm_service",
            "@pigweed//pw_kernel/userspace",
            "@pigweed//pw_log/rust:pw_log",
            "@pigweed//pw_status/rust:pw_status",
            "@rust_crates//:pldm-common",
        ],
        "name": "pldm_ua",
        "src": HERE + "pldm_ua_main.rs",
    },
]

SYSTEM_CONFIG = HERE + "system_config"
TARGET_SRC = HERE + "target.rs"
