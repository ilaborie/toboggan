# Other client

For every tasks, create a plan.md in the folder of the project that describe the code do be done.
Ultrathink for making this plan.

all code should be conform the clean code standard.

`mise check` should be OK

You can look at the existing web and wasm client as source of inspiration.

Try to keep the code simple.

## Toboggan desktop

In the `./toboggan-desktop` folder implement a toboggan client using the `iced` crate.


## Toboggan ios

In the `./toboggan-ios` folder implement an iOS client.
This should be a native iOS UI using Swift UI, but the core code should be writen in Rust
Using uniffi-rs to provide Switf API.

## Toboggan TUI

In the `./toboggan-tui` folder implement a toboggan client using the `ratatui` crate.
