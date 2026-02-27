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

## Testing

To run the unittests, run

.. tab-set::

    .. tab-item:: CW310
        .. code-block:: console

            bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/unittest_runner:hyper310_test

    .. tab-item:: CW340
        .. code-block:: console

            bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/unittest_runner:hyper340_test

## VS Code setup

.. _rust-analyzer: https://rust-analyzer.github.io/

.. code-block:: console

   bazelisk run @rules_rust//tools/rust_analyzer:gen_rust_project -- //target/...
