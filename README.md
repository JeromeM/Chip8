# chip8

A CHIP-8 emulator written in Rust, with a winit + pixels frontend.
Implements all 35 standard CHIP-8 opcodes, 60 Hz timers, and keyboard input.

## Run

```bash
# default: roms/ibm-logo.ch8
cargo run

# with a specific ROM
cargo run -- roms/pong.ch8
```

`Esc` quits the emulator.

## Controls

The CHIP-8 hex keypad is mapped to the top-left 4x4 block of your keyboard,
**by physical position** (works on AZERTY and QWERTY):

```
Your keyboard          CHIP-8 keypad
┌───┬───┬───┬───┐      ┌───┬───┬───┬───┐
│ 1 │ 2 │ 3 │ 4 │      │ 1 │ 2 │ 3 │ C │
├───┼───┼───┼───┤      ├───┼───┼───┼───┤
│ Q │ W │ E │ R │  →   │ 4 │ 5 │ 6 │ D │
├───┼───┼───┼───┤      ├───┼───┼───┼───┤
│ A │ S │ D │ F │      │ 7 │ 8 │ 9 │ E │
├───┼───┼───┼───┤      ├───┼───┼───┼───┤
│ Z │ X │ C │ V │      │ A │ 0 │ B │ F │
└───┴───┴───┴───┘      └───┴───┴───┴───┘
```

> Note: input mapping uses macOS scancodes. Windows / Linux would need
> different values in `scancode_to_chip8`.

## Project layout

```
src/main.rs    Window, event loop, input, rendering, frame pacing.
src/chip8.rs   The virtual machine: memory, registers, opcodes, display.
roms/          Drop ROMs here (.ch8 files).
```

## Further reading

- `INSTRUCTIONS.md` — reference of all 35 CHIP-8 opcodes (FR).
- `DECODING.md` — visual cheat sheet for opcode/nibble decoding (FR).
