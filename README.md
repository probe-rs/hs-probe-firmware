# hs-probe-firmware

A CMSIS-DAP firmware for [hs-probe](https://github.com/korken89/hs-probe). This includes support
for DAPv1 and DAPv2 over high-speed (480 MBit/s) USB 2.0.

## Building the firmware

```
cargo build --release
```

## Loading the firmware

The HS-Probe supports `dfu-util` and can have its firmware loaded via it. To
generate the bin, install
[cargo-binutils](https://github.com/rust-embedded/cargo-binutils) and run:

```console
cargo objcopy --release -- -O binary firmware.bin
```

And load it into the HS-Probe with:

```console
dfu-util -a 0 -s 0x08000000:leave -D firmware.bin
```

It will automatically restart into DFU mode and load the firmware.

## Feature flags

The following feature flags exists:

* `turbo`, this will the MCU speed to 216 MHz instead of the current default of 72 MHz.
* ...

To build with features, the following command is used:

```console
cargo build --release --features turbo,...,...
```

## Special thanks

We would like to give special thanks to:

- [Vadim Kaushan (@disasm)](https://github.com/disasm) for the USB implementation and helping bring the probe up.
- [Adam Greig (@adamgreig)](https://github.com/adamgreig) for the SWD implementation and helping bring the probe up.
- [Emil Fresk (@korken89)](https://github.com/korken89) for the hardware design.
- [Noah Huesser (@yatekii)](https://github.com/yatekii) for the `probe-rs` initiative and helping bring the probe up.

## Licence

Firmware is licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
