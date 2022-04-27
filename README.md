# Padme core

## Pixel As Dot-Matrix Emulator

padme-core is a GB emulator core. It itself doesn't rely on libstd or on dynamic memory, which makes it easier to use in any embedded platforms or for WASM.

## Tests

For fast unit / integration / doc tests:

```
cargo test
```

For more expensive tests, you can use:

```
cargo test -- --ignored
```

or run all tests with:

```
cargo test -- --include-ignored
```

## Examples

To see some implementations, check out [padme-demo](https://github.com/alexlren/padme-demo), a desktop demo or [padme-browser](https://github.com/alexlren/padme-browser), a web assembly version.

## Features

- [x] no_std
- [x] Timer
- [x] DMA
- [x] CPU Dissassembler
- [x] Pixel Processor Unit with fifo
- [x] External Screen
- [x] External Serial port
- [x] Joypad
- [x] Rom, MBC1, MBC3
- [x] Add some integration tests
- [ ] Add support for MBC2, MBC4, MBC5, MBC6, MBC7
- [ ] Add unit tests for each module
- [ ] Audio processor unit
