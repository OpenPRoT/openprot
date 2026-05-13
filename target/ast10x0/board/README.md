# ast10x0_board

Board-level integration crate for AST10x0 platforms.

This is the empty scaffold. The crate exists so that board-specific
descriptors, monitor wiring, and SPIM routing helpers have a stable
home as they get added on top of the SCU + SPIMONITOR primitives in
`peripherals`.

## Build

```
bazelisk build --config=virt_ast10x0 //target/ast10x0/board:ast10x0_board
```
