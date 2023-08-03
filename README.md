EUsb
===========

![Rust](https://github.com/ZR233/eusb/workflows/Rust/badge.svg)

Rust bindings for the [Libusb C-library](https://libusb.info/) for communicate with usb device.

This repository includes three crates:

- [![Crate](https://img.shields.io/crates/v/eusb.svg)](https://crates.io/crates/eusb)
  [![docs.rs](https://docs.rs/eusb/badge.svg)](https://docs.rs/eusb)
  `eusb`: A easy use async Rust lib. 
- [![Crate](https://img.shields.io/crates/v/libusb-src.svg)](https://crates.io/crates/libusb-src)
  [![docs.rs](https://docs.rs/libusb-src/badge.svg)](https://docs.rs/libusb-src)
  `libusb-src`: A crate for compiling the Libusb library.
- `example`: example usages.


Support platform
--------------


| Linux | Windows | macOS | android |
|:-----:|:-------:|:-----:|:-------:|
|  ✔️   |   ✔️    |  ✔️   |   ✔️    |

LICENSE
--------
See [LICENSE.md](./LICENSE.md)