//! CHIP-8 virtual machine: memory, registers, stack, display, keypad.
//!
//! This module is completely independent from the frontend (window, audio, input).
//! It exposes a minimal API:
//!
//! - [`Chip8::new`] to instantiate a fresh machine (font set preloaded in RAM).
//! - [`Chip8::load_rom`] to copy a program into memory.
//! - [`Chip8::tick`] to fetch / decode / execute one instruction.
//! - [`Chip8::decrement_timers`] to be called at 60 Hz.
//! - The `display` and `keypad` fields, exposed publicly so the frontend can
//!   read the screen and write key states.

pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;
pub const DISPLAY_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT;

const MEMORY_SIZE: usize = 4096;
const NUM_REGISTERS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;

/// Programs start at 0x200 in RAM. The first 512 bytes were historically
/// reserved for the interpreter on the original COSMAC VIP machine.
const PROGRAM_START: u16 = 0x200;

/// Address where the built-in font sprites are loaded. Conventionally 0x50.
const FONT_BASE: u16 = 0x50;

/// 16 hexadecimal digit sprites (0..F), 5 bytes each.
/// Each byte = one row of 8 pixels. A character is 8x5 pixels.
/// Programs access these via the FX29 opcode.
const FONT_SET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];


pub struct Chip8 {
    /// 4 KB of RAM. Holds the program (from 0x200), sprites, and data.
    memory: [u8; MEMORY_SIZE],

    /// 16 general-purpose 8-bit registers V0..VF. VF doubles as a flag
    /// register (carry, borrow, sprite collision, etc.).
    v: [u8; NUM_REGISTERS],

    /// 16-bit index register. Typically points to a memory address
    /// (sprite to draw, base address for FX55/FX65, etc.).
    i: u16,

    /// Program counter: address of the next instruction to execute.
    /// Starts at 0x200, advances by 2 each tick (instructions are 2 bytes).
    pc: u16,

    /// Call stack for subroutines (CALL/RET). Up to 16 nested calls.
    stack: [u16; STACK_SIZE],

    /// Stack pointer: index of the next free slot.
    /// `stack[sp - 1]` is the top of the stack; `sp == 0` means empty.
    sp: u8,

    /// 8-bit timer decremented at 60 Hz while > 0.
    /// Programs read it via FX07 to measure time (animations, cooldowns).
    delay_timer: u8,

    /// 8-bit timer, same behavior as `delay_timer`, but the machine
    /// must emit a beep while it's > 0. Set via FX18.
    sound_timer: u8,

    /// Monochrome video memory: 64 x 32 = 2048 pixels, indexed as
    /// `display[y * DISPLAY_WIDTH + x]`. `true` = pixel on, `false` = off.
    pub display: [bool; DISPLAY_SIZE],

    /// Hex keypad state (16 keys, 0..F). `true` = pressed.
    pub keypad: [bool; NUM_KEYS],
}


impl Chip8 {
    /// Builds a fresh machine with the font set preloaded at FONT_BASE.
    pub fn new() -> Self {
        let mut memory = [0u8; MEMORY_SIZE];
        let base = FONT_BASE as usize;
        memory[base..base + FONT_SET.len()].copy_from_slice(&FONT_SET);

        Self {
            memory,
            v: [0; NUM_REGISTERS],
            i: 0,
            pc: PROGRAM_START,
            stack: [0; STACK_SIZE],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            display: [false; DISPLAY_SIZE],
            keypad: [false; NUM_KEYS],
        }
    }

    /// Copies a program into memory starting at PROGRAM_START (0x200).
    pub fn load_rom(&mut self, rom: &[u8]) {
        let start = PROGRAM_START as usize;
        self.memory[start..start + rom.len()].copy_from_slice(rom);
    }

    /// Decrements both timers by 1 (saturating at 0).
    /// Must be called at 60 Hz, independent of CPU speed.
    pub fn decrement_timers(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }

    /// Runs one fetch / decode / execute cycle.
    pub fn tick(&mut self) {
        // --- FETCH ---
        // CHIP-8 instructions are 16 bits, stored big-endian across 2 bytes.
        let high = self.memory[self.pc as usize] as u16;
        let low = self.memory[(self.pc as usize) + 1] as u16;
        let opcode: u16 = (high << 8) | low;
        self.pc += 2;

        // --- DECODE ---
        // Split the opcode into its 4 nibbles plus common immediate forms.
        let nibbles = (
            (opcode & 0xF000) >> 12,
            (opcode & 0x0F00) >> 8,
            (opcode & 0x00F0) >> 4,
            (opcode & 0x000F),
        );
        let nnn = opcode & 0x0FFF;             // 12-bit immediate (address)
        let nn = (opcode & 0x00FF) as u8;      // 8-bit immediate
        let x = nibbles.1 as usize;            // Vx register index
        let y = nibbles.2 as usize;            // Vy register index
        let n = nibbles.3 as usize;            // 4-bit immediate (sprite height)

        // --- EXECUTE ---
        match nibbles {
            // 00E0 — Clear the screen.
            (0x0, 0x0, 0xE, 0x0) => {
                self.display = [false; DISPLAY_SIZE];
            }

            // 00EE — RET: pop return address into pc.
            (0x0, 0x0, 0xE, 0xE) => {
                self.sp -= 1;
                self.pc = self.stack[self.sp as usize];
            }

            // 1NNN — Jump to NNN.
            (0x1, _, _, _) => {
                self.pc = nnn;
            }

            // 2NNN — CALL: push pc, then jump to NNN.
            (0x2, _, _, _) => {
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = nnn;
            }

            // 3XNN — Skip next instruction if Vx == NN.
            (0x3, _, _, _) => {
                if self.v[x] == nn {
                    self.pc += 2;
                }
            }

            // 4XNN — Skip next instruction if Vx != NN.
            (0x4, _, _, _) => {
                if self.v[x] != nn {
                    self.pc += 2;
                }
            }

            // 5XY0 — Skip next instruction if Vx == Vy.
            (0x5, _, _, 0x0) => {
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            }

            // 6XNN — Vx = NN.
            (0x6, _, _, _) => {
                self.v[x] = nn;
            }

            // 7XNN — Vx += NN (no carry flag, wraps on u8 overflow).
            (0x7, _, _, _) => {
                self.v[x] = self.v[x].wrapping_add(nn);
            }

            // 8XY0 — Vx = Vy.
            (0x8, _, _, 0x0) => {
                self.v[x] = self.v[y];
            }

            // 8XY1 — Vx = Vx | Vy.
            (0x8, _, _, 0x1) => {
                self.v[x] |= self.v[y];
            }

            // 8XY2 — Vx = Vx & Vy.
            (0x8, _, _, 0x2) => {
                self.v[x] &= self.v[y];
            }

            // 8XY3 — Vx = Vx ^ Vy.
            (0x8, _, _, 0x3) => {
                self.v[x] ^= self.v[y];
            }

            // 8XY4 — Vx += Vy, VF = carry. Write Vx before VF so the flag
            // survives when x happens to equal 0xF.
            (0x8, _, _, 0x4) => {
                let (result, overflow) = self.v[x].overflowing_add(self.v[y]);
                self.v[x] = result;
                self.v[0xF] = overflow as u8;
            }

            // 8XY5 — Vx -= Vy, VF = 1 if no borrow (Vx >= Vy).
            (0x8, _, _, 0x5) => {
                let (result, borrow) = self.v[x].overflowing_sub(self.v[y]);
                self.v[x] = result;
                self.v[0xF] = if borrow { 0 } else { 1 };
            }

            // 8XY6 — Vx >>= 1, VF = lost LSB. SUPER-CHIP variant: Vy is ignored.
            (0x8, _, _, 0x6) => {
                let lsb = self.v[x] & 0x1;
                self.v[x] >>= 1;
                self.v[0xF] = lsb;
            }

            // 8XY7 — Vx = Vy - Vx, VF = 1 if no borrow (Vy >= Vx).
            (0x8, _, _, 0x7) => {
                let (result, borrow) = self.v[y].overflowing_sub(self.v[x]);
                self.v[x] = result;
                self.v[0xF] = if borrow { 0 } else { 1 };
            }

            // 8XYE — Vx <<= 1, VF = lost MSB. SUPER-CHIP variant: Vy is ignored.
            (0x8, _, _, 0xE) => {
                let msb = (self.v[x] & 0x80) >> 7;
                self.v[x] <<= 1;
                self.v[0xF] = msb;
            }

            // 9XY0 — Skip next instruction if Vx != Vy.
            (0x9, _, _, 0x0) => {
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
            }

            // ANNN — I = NNN.
            (0xA, _, _, _) => {
                self.i = nnn;
            }

            // BNNN — Jump to NNN + V0.
            (0xB, _, _, _) => {
                self.pc = nnn + self.v[0] as u16;
            }

            // CXNN — Vx = random_byte & NN.
            (0xC, _, _, _) => {
                self.v[x] = rand::random::<u8>() & nn;
            }

            // DXYN — Draw an N-row sprite at (Vx, Vy) from address I, XORing
            // onto the screen. VF = 1 if any lit pixel was turned off.
            // Initial position wraps modulo screen, but overflowing pixels
            // at the right/bottom edge are clipped, not wrapped.
            (0xD, _, _, _) => {
                let start_x = (self.v[x] as usize) % DISPLAY_WIDTH;
                let start_y = (self.v[y] as usize) % DISPLAY_HEIGHT;
                self.v[0xF] = 0;

                for row in 0..n {
                    let sprite_byte = self.memory[(self.i as usize) + row];

                    for col in 0..8 {
                        let bit = (sprite_byte >> (7 - col)) & 1;
                        if bit == 0 {
                            continue;
                        }

                        let sx = start_x + col;
                        let sy = start_y + row;
                        if sx >= DISPLAY_WIDTH || sy >= DISPLAY_HEIGHT {
                            continue;
                        }

                        let idx = sy * DISPLAY_WIDTH + sx;
                        if self.display[idx] {
                            self.v[0xF] = 1;
                        }
                        self.display[idx] ^= true;
                    }
                }
            }

            // EX9E — Skip next instruction if the key with code Vx is pressed.
            (0xE, _, 0x9, 0xE) => {
                if self.keypad[self.v[x] as usize] {
                    self.pc += 2;
                }
            }

            // EXA1 — Skip next instruction if the key with code Vx is NOT pressed.
            (0xE, _, 0xA, 0x1) => {
                if !self.keypad[self.v[x] as usize] {
                    self.pc += 2;
                }
            }

            // FX07 — Vx = delay_timer.
            (0xF, _, 0x0, 0x7) => {
                self.v[x] = self.delay_timer;
            }

            // FX0A — Block until a key is pressed, then store its code in Vx.
            // If no key is pressed, rewind pc by 2 so this instruction runs
            // again next tick (spinning in place until a press is detected).
            (0xF, _, 0x0, 0xA) => {
                match (0..NUM_KEYS).find(|&i| self.keypad[i]) {
                    Some(key) => self.v[x] = key as u8,
                    None => self.pc -= 2,
                }
            }

            // FX15 — delay_timer = Vx.
            (0xF, _, 0x1, 0x5) => {
                self.delay_timer = self.v[x];
            }

            // FX18 — sound_timer = Vx.
            (0xF, _, 0x1, 0x8) => {
                self.sound_timer = self.v[x];
            }

            // FX1E — I += Vx (wraps on u16 overflow, VF is not affected).
            (0xF, _, 0x1, 0xE) => {
                self.i = self.i.wrapping_add(self.v[x] as u16);
            }

            // FX29 — I = address of the font sprite for digit Vx (5 bytes each).
            (0xF, _, 0x2, 0x9) => {
                self.i = FONT_BASE + (self.v[x] as u16) * 5;
            }

            // FX33 — Store BCD of Vx into memory[I..I+3] (hundreds, tens, units).
            (0xF, _, 0x3, 0x3) => {
                let val = self.v[x];
                let i = self.i as usize;
                self.memory[i] = val / 100;
                self.memory[i + 1] = (val / 10) % 10;
                self.memory[i + 2] = val % 10;
            }

            // FX55 — Store V0..=Vx into memory[I..]. SUPER-CHIP variant: I is unchanged.
            (0xF, _, 0x5, 0x5) => {
                let base = self.i as usize;
                for offset in 0..=x {
                    self.memory[base + offset] = self.v[offset];
                }
            }

            // FX65 — Load V0..=Vx from memory[I..]. SUPER-CHIP variant: I is unchanged.
            (0xF, _, 0x6, 0x5) => {
                let base = self.i as usize;
                for offset in 0..=x {
                    self.v[offset] = self.memory[base + offset];
                }
            }

            _ => {
                eprintln!(
                    "Unimplemented opcode: {:04X} (at pc = {:#X})",
                    opcode,
                    self.pc - 2
                );
            }
        }
    }
}
