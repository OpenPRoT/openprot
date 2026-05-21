# OpenTitan Pigweed target

Example project of the Pigweed kernel running on the OpenTitan Earl Grey chip.

## Building

To build the IPC test, run:

.. code-block:: console

    bazelisk build //target/earlgrey/ipc/user:ipc

## Running

To run the IPC test, run

.. tab-set::

    .. tab-item:: CW310
        .. code-block:: console

            bazelisk run //target/earlgrey/ipc/user:ipc_runner_hyper310

    .. tab-item:: CW340
        .. code-block:: console

            bazelisk run //target/earlgrey/ipc/user:ipc_runner_hyper340

    .. tab-item:: Verilator
        .. code-block:: console

            bazelisk run //target/earlgrey/ipc/user:ipc_runner_verilator

    .. tab-item:: QEMU
        .. code-block:: console

            bazelisk run //target/earlgrey/tests/ipc/user:ipc_runner_qemu_test

## Testing

To run the unittests, run

.. tab-set::

    .. tab-item:: CW310
        .. code-block:: console

            bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/unittest_runner:hyper310_test

    .. tab-item:: CW340
        .. code-block:: console

            bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/unittest_runner:hyper340_test

    .. tab-item:: QEMU
        .. code-block:: console

            bazelisk test //target/earlgrey/tests/ipc/user:ipc_runner_qemu_test

        This test runs in approximately 2.6 seconds under QEMU.
        It is included in the ``earlgrey_qemu_tests`` workflow and runs on every PR via the ``ci`` group.

## Adding a QEMU lane to a new earlgrey test

1. **Confirm an existing verilator target** — find the ``opentitan_test`` (or ``opentitan_runner``) rule
   with ``interface = "verilator"`` in the target's ``BUILD.bazel``.

2. **Add a sibling QEMU test target** in the same ``BUILD.bazel``:

   .. code-block:: python

       opentitan_test(
           name = "<test>_qemu_test",
           interface = "qemu",
           tags = ["qemu"],
           target = ":<image_target>",
           timeout = "moderate",
       )

3. **Omit** ``ecdsa_key``, ``spx_key``, and ``nightly_test`` — those are not needed for QEMU tests.

4. **No workflow wiring needed.** The ``earlgrey_qemu_tests`` build uses
   ``--build_tag_filters=+qemu --test_tag_filters=+qemu`` and targets ``//target/earlgrey/...``,
   so any ``opentitan_test`` target tagged ``qemu`` under ``target/earlgrey/`` is automatically
   picked up and run on every PR via the ``ci`` group.

5. **Canonical example:** ``target/earlgrey/tests/ipc/user/BUILD.bazel`` — see the
   ``ipc_runner_qemu_test`` target.

## VS Code setup

.. _rust-analyzer: https://rust-analyzer.github.io/

.. code-block:: console

   bazelisk run @rules_rust//tools/rust_analyzer:gen_rust_project -- //target/...
