# ESP32-S3 Hardware Specifications

This document outlines the hardware specifications for the ESP32-S3 microcontroller used in the toboggan-esp32 implementation.

## Microcontroller

| Parameter | Value |
|-----------|-------|
| **Type** | ESP32-S3 |
| **CPU** | Dual-Core Xtensa® 32bit LX7 up to 240 MHz |

## Memory

| Type | Specification |
|------|---------------|
| **SRAM** | 512 KB |
| **ROM** | 384 KB |
| **PSRAM** | Octal SPI, 16 MB |
| **PSRAM Speed** | 120 MHz (Experimental Feature) |
| **External Flash** | Quad SPI, 16 MB |

## AI Feature

| Component | Specification |
|-----------|---------------|
| **AI Algorithm** | Neural Network, Acoustic algorithm, etc. |
| **Computing Acceleration** | Vector, Complex number, FFT, etc. |

## Wireless

| Technology | Specification |
|------------|---------------|
| **Wi-Fi** | 2.4 GHz, IEEE 802.11b/g/n |
| **Bluetooth® LE** | Bluetooth® 5 LE and Bluetooth® mesh |

## Display

| Parameter | Value |
|-----------|-------|
| **Display Type** | 2.4-inch LCD |
| **Display Resolution** | 240 x 320 pixels |
| **Display Interface** | SPI |
| **Interface Speed** | 40 MHz |
| **Touch Type** | Capacitive |
| **Touch Points** | 10 |
| **Driver IC** | ILI9342C |

## Audio

### Audio Input

| Parameter | Value |
|-----------|-------|
| **Microphone Type** | Dual Mic |
| **ADC Model** | ES7210 |
| **Mute** | Supported |

### Audio Output

| Parameter | Value |
|-----------|-------|
| **Speaker Model** | 8 Ohm 1 W |
| **PA Model** | NS4150 |
| **Codec Model** | ES8311 |

## Sensor

| Parameter | Value |
|-----------|-------|
| **Sensor Type** | 3-axis Gyroscope, 3-axis Accelerometer |
| **Sensor Model** | ICM-42607-P |

## Interface

| Type | Usage |
|------|-------|
| **USB Type-C** | Power, USB download/JTAG debug, general USB device functions |
| **Goldfinger** | I/O Expansion |

## User Interface

| Component | Specification |
|-----------|---------------|
| **Onboard Buttons** | Reset, Boot, Mute |
| **Onboard LEDs** | Power LED, Mute LED |

## OS / SDK

| Parameter | Value |
|-----------|-------|
| **Original OS** | FreeRTOS |
| **SDK** | ESP-IDF |

## Physical Specifications

| Parameter | Value |
|-----------|-------|
| **Dimensions** | 61 x 66 x 16.6 mm |
| **Weight** | 292 g |

## Power

| Parameter | Value |
|-----------|-------|
| **USB-C Power** | 5 V - 2.0 A Input |
| **Battery** | N/A |

## ESP32-S3-BOX-3-DOCK

The dock provides additional connectivity options for the ESP32-S3 development board.

| Type | Number | Details | Usage |
|------|--------|---------|-------|
| **12-pin Female Header** | 2 | 8 I/O (Pmod™ Compatible), 3.3 V Power Output | GPIO, I2C, SPI, UART, RMT, LEDC, etc. |
| **USB Type-A** | 1 | 5 V Power Output, USB Host | Connect to diverse USB devices such as USB camera, USB disk, and other HID devices |
| **USB Type-C** | 1 | 5 V Power Input | 5 V power input only |
| **PCIe Connector** | 1 | 36 Pin, 1.00 mm (.0394") pitch, Accepts .062" (1.60 mm) card | Vertical mounting goldfinger |

### Dock Pinout Layout

The ESP32-S3-BOX-3-DOCK provides two 12-pin Pmod™ compatible headers (J1 and J2) with the following pinout:

#### J1 Header (Middle)
| Pin | Signal | Type | Pin | Signal | Type |
|-----|--------|------|-----|--------|------|
| 1 | 3V3 | Power | 7 | 3V3 | Power |
| 2 | GND | Ground | 8 | GND | Ground |
| 3 | GPIO41 | GPIO | 9 | GPIO40 | GPIO |
| 4 | GPIO38 | GPIO | 10 | GPIO39 | GPIO |
| 5 | GPIO19 | GPIO | 11 | GPIO20 | GPIO |
| 6 | GPIO21 | GPIO | 12 | GPIO42 | GPIO |

#### J2 Header (Side)  
| Pin | Signal | Type | Pin | Signal | Type |
|-----|--------|------|-----|--------|------|
| 1 | 3V3 | Power | 7 | 3V3 | Power |
| 2 | GND | Ground | 8 | GND | Ground |
| 3 | GPIO43 | GPIO | 9 | GPIO44 | GPIO |
| 4 | GPIO11 | GPIO | 10 | GPIO12 | GPIO |
| 5 | GPIO14 | GPIO | 11 | GPIO9 | GPIO |
| 6 | GPIO10 | GPIO | 12 | GPIO13 | GPIO |

#### Special Pins
- **U0TXD**: GPIO43 (UART0 Transmit)
- **U0RXD**: GPIO44 (UART0 Receive)
- **USB-**: GPIO19 (USB Data Negative)
- **USB+**: GPIO20 (USB Data Positive)

#### Pin Capabilities Legend
- **PWM Capable Pin**: Pins marked with wave symbol (~) support PWM output
- **GPIOX**: General Purpose Input/Output pins
- **JTAG**: JTAG debugging interface pins
- **SERIAL**: Serial communication pins for debug/programming
- **GND**: Ground connections
- **PWR**: Power rails (3V3 and 5V)

---

*This specification is based on the ESP32-S3 development board and dock used for the toboggan-esp32 embedded presentation system.*