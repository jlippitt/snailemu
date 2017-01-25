#[macro_use]
extern crate bitflags;

extern crate sdl2;
extern crate sdl2_sys;

#[macro_use]
mod log;

mod cpu;
mod hardware;
mod util;

use cpu::Cpu;
use hardware::{Apu, Hardware, Hdma, IoPort, Joypad, Ppu, Rom, Screen, Wram};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::env;
use std::path::Path;
use std::rc::Rc;

fn main() {
    let rom_path = env::args_os().nth(1).unwrap();
    let rom = Rom::new(Path::new(&rom_path));

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let io_port = Rc::new(IoPort::new());

    let ppu = Ppu::new(Screen::new(&video_subsystem), io_port.clone());

    let hardware = Hardware::new(rom, Wram::new(), ppu, Apu::new(), Joypad::new(), io_port);

    let mut cpu = Cpu::new(hardware);

    'outer: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'outer,
                Event::KeyDown { keycode: Some(Keycode::T), .. } => log::enable_trace_mode(),
                _ => cpu.hardware_mut().joypad_mut().handle_event(event)
            }
        }

        cpu.tick();
    }
}
