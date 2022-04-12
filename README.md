# Padme core

## Pixel As Dot-Matrix Emulator

padme-core is a GB emulator core. It itself doesn't rely on libstd or on dynamic memory, which makes it easier to use in any embedded platforms or for WASM.

## TODO

- [x] no_std
- [x] Timer
- [x] DMA
- [x] CPU Dissassembler
- [x] Pixel Processor Unit with fifo
- [x] External Screen
- [x] External Serial port
- [x] Joypad
- [ ] Add unit tests for each module
- [ ] Add integration tests and quick examples
- [ ] Joypad interrupts
- [ ] MBC
- [ ] Sound
