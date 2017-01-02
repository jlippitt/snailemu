use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use super::hardware::HardwareBus;

pub const JOYPAD_COUNT: usize = 4;

pub struct Joypad {
    button_state: [ButtonState; JOYPAD_COUNT],
    button_indexes: [usize; 2],
    latch: bool
}

bitflags! {
    pub flags ButtonState: u16 {
        const B = 0x8000,
        const Y = 0x4000,
        const SELECT = 0x2000,
        const START = 0x1000,
        const UP = 0x0800,
        const DOWN = 0x0400,
        const LEFT = 0x0200,
        const RIGHT = 0x0100,
        const A = 0x0080,
        const X = 0x0040,
        const L = 0x0020,
        const R = 0x0010
    }
}

fn keycode_to_button(keycode: Keycode) -> ButtonState {
    // All very subject to change
    match keycode {
        Keycode::Z => B,
        Keycode::A => Y,
        Keycode::Space => SELECT,
        Keycode::Return => START,
        Keycode::Up => UP,
        Keycode::Down => DOWN,
        Keycode::Left => LEFT,
        Keycode::Right => RIGHT,
        Keycode::X => A,
        Keycode::S => X,
        Keycode::Q => L,
        Keycode::W => R,
        _ => ButtonState::empty()
    }
}

impl Joypad {
    pub fn new() -> Joypad {
        Joypad {
            button_state: [ButtonState::empty(); 4],
            button_indexes: [0, 0],
            latch: false
        }
    }

    pub fn read_button_state(&self) -> [u16; JOYPAD_COUNT] {
        [
            self.button_state[0].bits(),
            self.button_state[1].bits(),
            self.button_state[2].bits(),
            self.button_state[3].bits()
        ]
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::KeyDown { keycode: Some(keycode), .. } => {
                self.button_state[0].insert(keycode_to_button(keycode));
            },
            Event::KeyUp { keycode: Some(keycode), .. } => {
                self.button_state[0].remove(keycode_to_button(keycode));
            },
            _ => ()
        };
    }

    fn read_data_line_state(&mut self, port_offset: usize) -> u8 {
        let button_index = self.button_indexes[port_offset];

        if button_index < 16 {
            let mask = 0x8000 >> button_index;
            let data_line_1_bit = (self.button_state[port_offset].bits() & mask) != 0;
            let data_line_2_bit = (self.button_state[port_offset + 2].bits() & mask) != 0;
            self.button_indexes[port_offset] += 1;
            ((data_line_2_bit as u8) << 1) | (data_line_1_bit as u8)
        } else {
            0x03
        }
    }
}

impl HardwareBus for Joypad {
    fn read(&mut self, offset: usize) -> u8 {
        let value = match offset {
            0x16 => self.read_data_line_state(0),
            0x17 => 0x1C | self.read_data_line_state(1),
            _ => 0x00 // TODO: Open bus
        };
        debug!("NES joypad read: $40{:02X} => ${:02X}", offset, value);
        value
    }

    fn write(&mut self, offset: usize, value: u8) {
        debug!("NES joypad write: $40{:02X} <= ${:02X}", offset, value);
        match offset {
            0x16 => {
                let old_latch = self.latch;
                self.latch = value & 0x01 != 0;
                if self.latch && !old_latch {
                    self.button_indexes[0] = 0;
                    self.button_indexes[1] = 0;
                }
            },
            _ => ()
        };
    }
}
