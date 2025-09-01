# Links

API for the device
[esp-idf-svc](https://docs.esp-rs.org/esp-idf-svc/esp_idf_svc/index.html)

Raw bindings
[esp-idf-sys](https://docs.esp-rs.org/esp-idf-sys/esp_idf_sys/index.html)


[esp-idf-hal](https://docs.esp-rs.org/esp-idf-hal/esp_idf_hal/)


([Obsolete demo)](https://github.com/ivmarkov/rust-esp32-std-demo)


# Hardware


320x240 ST7789 TFT display
Crate: <https://crates.io/crates/st7789>

TT21100 capacitive touch screen
Crate: <https://crates.io/crates/tt21100>

## Example

- <https://github.com/georgik/esp32-spooky-maze-game>
- <https://github.com/georgik/esp32-rust-multi-target-template>

- <https://lilymara.xyz/posts/2023/01/images-esp32/>

## Flash

```bash
$ espflash /dev/cu.usbmodem14101 target/xtensa-esp32s3-espidf/debug/hello
```

## Logs

```bash
$ espmonitor /dev/cu.usbmodem14101
```

## LED

```rust
 let mut led = peripherals.pins.gpio5.into_output().unwrap();
  let n = 1;

  while n == 1 {
    led.set_high().unwrap();
    thread::sleep(Duration::from_millis(1000));

    led.set_low().unwrap();
    thread::sleep(Duration::from_millis(1000));

    println!("blink");
  }
```
