Source: https://github.com/opsnull/rust-slint-printdemo/blob/main/mcu-board-support/gt911.rs

References:
- GT911: https://github.com/enelson1001/rust-esp32s3-lvgl-clickme/blob/master/src/gt911.rs
- TT21100 driver implementation: https://github.com/jessebraham/tt21100/blob/main/tt21100/src/lib.rs
- TT21100 example: https://github.com/sambenko/esp-box-tt21100-example/blob/main/src/main.rs
- GT911 register list: https://github.com/STMicroelectronics/stm32-gt911/blob/main/gt911_reg.h
- esp-idf C GT911 driver: https://github.com/espressif/esp-bsp/blob/master/components/lcd_touch/esp_lcd_touch_gt911/esp_lcd_touch_gt911.c

Notes:

1. ESP32-S3-BOX-3's GT911 I2C address is 0x14, not the default 0x5d (discovered through testing); reference: https://github.com/espressif/esp-bsp/blob/master/components/lcd_touch/esp_lcd_touch_gt911/include/esp_lcd_touch_gt911.h#L34C1-L40C4
2. Using an older version of embedded_hal library (0.2.5 version, not the new 1.0.0 version) for compatibility with other libraries;
3. Since Touch and LCD share the reset pin, and LCD initialization has already set the reset, Touch doesn't need reset - removed related logic, otherwise it would cause LCD to display white screen;
4. Custom Error, wrapping other types of Error for easier error reporting;
5. Added IRQ pin and IRQ pin-based data_available() method, which Slint will subsequently call to determine if there are touch events;

Known Issues:

1. One touch produces multiple touch events (see serial logs), so filtering needs to be done at the application layer like Slint.


---

TODO see <https://github.com/sambenko/esp-box-tt21100-example/blob/main/src/main.rs>
https://github.com/enelson1001/rust-esp32s3-lvgl-clickme/blob/master/src/gt911.rs
https://github.com/opsnull/rust-slint-printdemo/blob/main/mcu-board-support/gt911.rs

https://github.com/ImplFerris/esp32-projects/tree/main

https://github.com/espressif/idf-extra-components?tab=readme-ov-file

