# hs-probe-firmware

A CMSIS-DAP firmware for [hs-probe](https://github.com/korken89/hs-probe). This includes support
for DAPv1 and DAPv2 over high-speed (480 MBit/s) USB 2.0.

## Building the firmware

```
cd firmware
cargo build --release
```

## Loading the firmware

The HS-Probe supports `dfu-util` and can have its firmware loaded via it. To generate the bin, run:

```console
cd firmware
cargo objcopy --release -- -O binary firmware.bin
```

And load it into the HS-Probe with:

```console
dfu-util -a 0 -s 0x08000000:leave -D firmware.bin
```

It will automatically restart into DFU mode and load the firmware.

## Feature flags

One can update the feature flags

## Licence

Firmware is licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
