
HTTP client

<https://crates.io/crates/reqwless>

WebSocket

<https://crates.io/crates/embedded-websocket>


<https://crates.io/crates/xtensa-lx-rt>
```toml
[profile.dev.package.xtensa-lx-rt]
opt-level = 'z'
```

<https://github.com/esp-rs/esp-pacs/tree/main/esp32s3>

<https://github.com/esp-rs/esp-hal>
<https://github.com/esp-rs/esp-idf-hal>

## Tools

### espup

```bash
cargo binstall espup
```

``` bash
$ espup help
Tool for installing and maintaining Espressif Rust ecosystem.


Usage: espup <COMMAND>

Commands:
  completions  Generate completions for the given shell
  install      Installs Espressif Rust ecosystem
  uninstall    Uninstalls Espressif Rust ecosystem
  update       Updates Xtensa Rust toolchain
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

Install

```bash
espup install
```

+ Setup env <https://docs.espressif.com/projects/rust/book/installation/riscv-and-xtensa.html#3-set-up-the-environment-variables>


Install more tools

```bash
brew install cmake ninja dfu-util
```

```bash
cargo binstall ldproxy
```

```bash
cargo binstall esp-generate
```

```bash
cargo binstall espflash
```
---

## Wifi

```bash
$ networksetup -getinfo Wi-Fi
DHCP Configuration
IP address: 192.168.1.137
Subnet mask: 255.255.255.0
Router: 192.168.1.254
Client ID:
IPv6: Automatic
IPv6 IP address: none
IPv6 Router: none
Wi-Fi ID: 84:2f:57:62:67:54
```

---
