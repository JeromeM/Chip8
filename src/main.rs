mod chip8;

use std::process;
use std::time::{Duration, Instant};

use chip8::{Chip8, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

/// Render scale: CHIP-8's logical 64x32 surface is scaled by this factor
/// to fit the OS window.
const SCALE: u32 = 10;

/// CHIP-8 CPU speed (instructions per second). 700 Hz is the accepted
/// default for most modern ROMs.
const CPU_HZ: u32 = 700;

/// Timer / render tick rate (60 Hz).
const TIMER_HZ: u32 = 60;

/// CPU cycles to run per 60 Hz frame (≈ 11).
const CYCLES_PER_FRAME: u32 = CPU_HZ / TIMER_HZ;

/// Duration of one 60 Hz frame.
const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TIMER_HZ as u64);

/// ROM loaded when no CLI argument is provided.
const DEFAULT_ROM_PATH: &str = "roms/ibm-logo.ch8";

/// Maps a physical key (by macOS scancode) to a CHIP-8 keypad index (0..F).
/// Returns `None` for any unmapped key.
///
/// We use the scancode rather than the VirtualKeyCode because, on macOS with
/// an AZERTY layout, winit reports inconsistent VirtualKeyCodes for the top
/// number row (e.g. unshifted "1" arrives as Key7). Scancodes represent the
/// physical key position, independent of layout.
///
/// ⚠️ These scancode values are **macOS-specific**. Windows/Linux would differ.
///
///   Physical position →  CHIP-8 keypad
///   1 2 3 4           →  1 2 3 C
///   Q W E R           →  4 5 6 D
///   A S D F           →  7 8 9 E
///   Z X C V           →  A 0 B F
fn scancode_to_chip8(scancode: u32) -> Option<usize> {
    Some(match scancode {
        18 => 0x1, 19 => 0x2, 20 => 0x3, 21 => 0xC, // 1 2 3 4 row
        12 => 0x4, 13 => 0x5, 14 => 0x6, 15 => 0xD, // Q W E R row
         0 => 0x7,  1 => 0x8,  2 => 0x9,  3 => 0xE, // A S D F row
         6 => 0xA,  7 => 0x0,  8 => 0xB,  9 => 0xF, // Z X C V row
        _ => return None,
    })
}

fn main() {
    env_logger::init();

    // Read the ROM from the first CLI argument, falling back to DEFAULT_ROM_PATH.
    let rom_path: String = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_ROM_PATH.to_string());
    let rom: Vec<u8> = match std::fs::read(&rom_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Cannot read ROM {}: {}", rom_path, e);
            process::exit(1);
        }
    };

    let event_loop = EventLoop::new();

    let window_width = DISPLAY_WIDTH as u32 * SCALE;
    let window_height = DISPLAY_HEIGHT as u32 * SCALE;

    let window = WindowBuilder::new()
        .with_title("CHIP-8")
        .with_inner_size(LogicalSize::new(window_width, window_height))
        .build(&event_loop)
        .unwrap();

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let mut pixels =
        Pixels::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32, surface_texture).unwrap();

    let mut emulator = Chip8::new();
    emulator.load_rom(&rom);

    // Timestamp of the last completed 60 Hz frame.
    let mut last_frame = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                control_flow.set_exit();
            }

            // Keyboard input: update emulator.keypad on press/release.
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    input: KeyboardInput { virtual_keycode, state, scancode, .. },
                    ..
                },
                ..
            } => {
                // Escape quits the emulator (VirtualKeyCode works fine for this).
                if virtual_keycode == Some(VirtualKeyCode::Escape)
                    && state == ElementState::Pressed
                {
                    control_flow.set_exit();
                }
                // CHIP-8 keypad mapping goes through scancode to stay
                // layout-independent (AZERTY / QWERTY / etc.).
                if let Some(chip8_key) = scancode_to_chip8(scancode) {
                    emulator.keypad[chip8_key] = state == ElementState::Pressed;
                }
            }

            Event::MainEventsCleared => {
                // Throttle to 60 Hz: when enough time has elapsed, run a batch
                // of CPU cycles, tick the timers, and request a redraw.
                if last_frame.elapsed() >= FRAME_DURATION {
                    for _ in 0..CYCLES_PER_FRAME {
                        emulator.tick();
                    }
                    emulator.decrement_timers();
                    last_frame = Instant::now();
                    window.request_redraw();
                }
            }

            Event::RedrawRequested(_) => {
                let frame = pixels.frame_mut();
                for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
                    let color = if emulator.display[i] {
                        [0xff, 0xff, 0xff, 0xff]
                    } else {
                        [0x00, 0x00, 0x00, 0xff]
                    };
                    pixel.copy_from_slice(&color);
                }
                pixels.render().unwrap();
            }

            _ => {}
        }
    });
}
