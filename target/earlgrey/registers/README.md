# Earlgrey register definitions

These register definitions were created by running kor's `topgen-extract`
utility (from his usergit) against the `earlgrey_1.0.0` branch.

```bash
cargo run -- chip_to_ureg \
    ~/opentitan/earlgrey_1.0.0 \
    top_earlgrey \
    ~/openprot/target/earlgrey/registers
```

I ran topgen-extract with [PR#15](https://github.com/chipsalliance/caliptra-ureg/pull/15) applied to caliptra-ureg.
