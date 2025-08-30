# Impl for ESP32

<https://github.com/espressif/esp-box>

ESP32-S3-BOX-3B

[esp-rs](https://github.com/esp-rs)

--- 
Software ver: v1.2.4
ESP-IDF Ver: v5.2-dev-2802-g165955de
SR lang: English
Board: S3_BOX_3
MAC: b4:3a:45:f3:a5:6c
---

Chip type: `ESP32-S3`

## Rust Link

<https://docs.espressif.com/projects/rust/book/>

<https://github.com/esp-rs>
<https://github.com/esp-rs/esp-hal>
<https://github.com/esp-rs/awesome-esp-rust>




## Update firmware

Using [esptool](https://github.com/espressif/esptool)
[esptool doc](https://docs.espressif.com/projects/esptool/en/latest/esp32/)


Read mac address

```bash
$ esptool read-mac
esptool v5.0.2
Connected to ESP32-S3 on /dev/cu.usbmodem2101:
Chip type:          ESP32-S3 (QFN56) (revision v0.2)
Features:           Wi-Fi, BT 5 (LE), Dual Core + LP Core, 240MHz, Embedded PSRAM 16MB (AP_1v8)
Crystal frequency:  40MHz
USB mode:           USB-Serial/JTAG
MAC:                b4:3a:45:f3:a5:6c

Stub flasher running.

MAC:                b4:3a:45:f3:a5:6c

Hard resetting via RTS pin...
```

Or Flash id

```bash
$ esptool v5.0.2
Connected to ESP32-S3 on /dev/cu.usbmodem2101:
Chip type:          ESP32-S3 (QFN56) (revision v0.2)
Features:           Wi-Fi, BT 5 (LE), Dual Core + LP Core, 240MHz, Embedded PSRAM 16MB (AP_1v8)
Crystal frequency:  40MHz
USB mode:           USB-Serial/JTAG
MAC:                b4:3a:45:f3:a5:6c

Stub flasher running.

Flash Memory Information:
=========================
Manufacturer: c8
Device: 6018
Detected flash size: 16MB
Flash type set in eFuse: quad (4 data lines)
Flash voltage set by eFuse: 3.3V

Hard resetting via RTS pin...
```

Update Firmware

```bash
$ esptool write-flash 0x0 ~/Downloads/ESP-BOX_Demo_EN_V0.5.0.bin
```

Download here: <https://github.com/espressif/esp-box/releases>

## Demo

Can upload demo and install with [esp-launchpad](https://espressif.github.io/esp-launchpad/?flashConfigURL=https://espressif.github.io/esp-box/launchpad.toml)

Connect, select the demo, Flash, and reset device when installed

## Programming IDF

<https://docs.espressif.com/projects/esp-idf/en/latest/esp32s3/index.html>


## Rust

[Rust Book]()
[Training](https://docs.espressif.com/projects/rust/no_std-training/)

<https://docs.espressif.com/projects/rust/>

[Maze beginner](https://jamesmcm.github.io/blog/beginner-rust-esp32-lcdsnake/)
[Examples](https://github.com/sambenko/esp32s3-box-examples)


---

core, alloc, std : <https://google.github.io/comprehensive-rust/bare-metal/no_std.html>
