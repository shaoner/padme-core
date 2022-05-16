# Padme core

![crates.io](https://img.shields.io/crates/v/padme_core.svg)
![build status](https://github.com/alexlren/estel_secp256k1/actions/workflows/ci.yaml/badge.svg)


## Pixel As Dot-Matrix Emulator

padme-core is a Gameboy emulator engine. It itself doesn't rely on libstd or on dynamic memory, which makes it easier to use in any embedded platforms or [web assembly](https://github.com/alexlren/padme-browser).

<img src="/docs/demo.gif" width="300"/>

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

1. Create your hardware components: a screen, a speaker and a serial output

These components should be specific to your platform.

```rust
use padme_core::{AudioSpeaker, Button, Pixel, Rom, Screen, SerialOutput, System};

struct MyScreen {
    // ... your framebuffer implementation
}

impl Screen for MyScreen {
    fn set_pixel(&mut self, pixel: &Pixel, x: u8, y: u8) {
        // add pixel to your framebuffer
    }
}

struct MySpeaker {
    // ... your audio buffer implementation
}

impl AudioSpeaker for MySpeaker {
    fn set_samples(&mut self, left: f32, right: f32) {
        // add samples for left and right channels
    }
}

struct MySerialConsole {
}

impl SerialOutput for MySerialConsole {
    fn putchar(&mut self, ch: u8) {
        // add char to 
    }
}
```

Alternatively, if you don't need / want some of these components, it's possible to use empty versions:

```rust
use padme::default::{NoScreen, NoSerial, NoSpeaker};
```

2. Load a rom

```rust
use padme_core::Rom;

let bin: Vec<u8> = std::fs::read("some_game.gb").expect("could not find game");
let mut rom = Rom::load(bin).unwrap();
```

3. Create your emulator and run it

```rust
use std::time::Instant;
use std::thread::sleep;

let mut emulator = System::new(rom, MyScreen, MySerialConsole, MySpeaker);
// Set the number of frame per seconds
// This also sets the number of cycles needed per frame given the fixed CPU clock frequency
emulator.set_frame_rate(60);

while running {
    // We need to know how much time it took to display a frame
    let t0 = Instant::now();
    // This executes all the cycles needed to display one frame
    emulator.update_frame();
    // Deal with button inputs
    emulator.set_button(Button::A, a_pressed);
    emulator.set_button(Button::B, b_pressed);
    emulator.set_button(Button::Start, start_pressed);
    emulator.set_button(Button::Select, select_pressed);
    emulator.set_button(Button::Up, up_pressed);
    emulator.set_button(Button::Down, down_pressed);
    emulator.set_button(Button::Left, left_pressed);
    emulator.set_button(Button::Right, right_pressed);
    // Now we just need to wait the remaining time before the next frame
    // This is because we need to keep ~60 frames / second
    let frame_time = t0.elapsed();
    let min_frame_time = emulator.min_frame_time();
    if frame_time < min_frame_time {
        sleep(min_frame_time - frame_time);
    }
}
```

Alternatively, you may want to execute the steps yourself:

```rust
use padme_core::{CLOCK_SPEED};

let cycles_per_frame = CLOCK_SPEED / 60;
let mut cycles = 0u32;
while cycles < cycles_per_frame {
    cycles += emulator.step() as u32;
}
emulator.screen().update();
```

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
- [x] Integration tests
- [x] Audio processor unit

## Todo

- [ ] Add support for MBC2, MBC4, MBC5, MBC6, MBC7
- [ ] Add unit tests for each module
