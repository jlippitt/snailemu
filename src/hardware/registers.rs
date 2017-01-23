use std::rc::Rc;
use super::hardware::HardwareBus;
use super::io_port::IoPort;
use super::joypad::{Joypad, JOYPAD_COUNT};
use super::ppu::Ppu;
use util::byte_access::ByteAccess;

const CHIP_VERSION: u8 = 0x02;

const JOYPAD_AUTO_READ_LINES: u8 = 3;

pub struct HardwareRegs {
    io_port: Rc<IoPort>,
    cpu_action: CpuAction,
    vblank: bool,
    hblank: bool,
    nmi: NmiRegs,
    irq: IrqRegs,
    multiplication: MultiplicationRegs,
    division: DivisionRegs,
    joypad: JoypadRegs,
    dma_channel_mask: u8
}

bitflags! {
    flags CpuAction: u8 {
        const NMI = 0x80,
        const IRQ = 0x40,
        const DMA = 0x20
    }
}

struct NmiRegs {
    enabled: bool,
    active: bool
}

struct IrqRegs {
    enabled: IrqCondition,
    row: u16,
    column: u16,
    active: bool
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum IrqCondition {
    Never,
    MatchRow,
    MatchColumn,
    MatchRowAndColumn
}

struct MultiplicationRegs {
    lhs: u8,
    result: u16
}

struct DivisionRegs {
    lhs: u16,
    result: u16
}

struct JoypadRegs {
    auto_read_enabled: bool,
    auto_read_active: u8,
    button_state: [u16; JOYPAD_COUNT]
}

impl HardwareRegs {
    pub fn new(io_port: Rc<IoPort>) -> HardwareRegs {
        HardwareRegs {
            io_port: io_port,
            cpu_action: CpuAction::empty(),
            vblank: false,
            hblank: false,
            nmi: NmiRegs {
                enabled: false,
                active: false
            },
            irq: IrqRegs {
                enabled: IrqCondition::Never,
                row: 0,
                column: 0,
                active: false
            },
            multiplication: MultiplicationRegs {
                lhs: 0xFF,
                result: 0x0000
            },
            division: DivisionRegs {
                lhs: 0xFFFF,
                result: 0x0000
            },
            joypad: JoypadRegs {
                auto_read_enabled: false,
                auto_read_active: 0,
                button_state: [0; JOYPAD_COUNT]
            },
            dma_channel_mask: 0x00
        }
    }

    pub fn update(&mut self, ppu: &mut Ppu, joypad: &Joypad) {
        let old_vblank = self.vblank;

        self.vblank = ppu.vblank();
        self.hblank = ppu.hblank();

        // During VBlank transition, set NMI flag and (if it is enabled) trigger NMI
        if self.vblank != old_vblank {
            self.nmi.active = self.vblank;

            if self.nmi.active {
                // Start of VBlank
                if self.nmi.enabled {
                    self.cpu_action.insert(NMI);
                }

                if self.joypad.auto_read_enabled {
                    self.joypad.auto_read_active = JOYPAD_AUTO_READ_LINES;
                    self.joypad.button_state = joypad.read_button_state();
                    debug!("Joypad auto read: {:04X}", self.joypad.button_state[0]);
                }
            }
        }

        if self.irq.enabled != IrqCondition::Never && !self.irq.active {
            let position = ppu.position();

            let timer_condition = match self.irq.enabled {
                IrqCondition::MatchRow => {
                    position.v() == self.irq.row && position.h() == 0
                },
                IrqCondition::MatchColumn => {
                    position.h() == self.irq.column
                },
                IrqCondition::MatchRowAndColumn => {
                    position.v() == self.irq.row && position.h() == self.irq.column
                },
                _ => unreachable!()
            };

            if timer_condition {
                self.irq.active = true;
                self.cpu_action.insert(IRQ);
            }
        }

        if self.joypad.auto_read_active > 0 {
            self.joypad.auto_read_active -= 1;
        }

        if self.io_port.triggered() {
            ppu.store_position();
            self.io_port.reset_trigger();
        }
    }

    pub fn cpu_action_ready(&self) -> bool {
        !self.cpu_action.is_empty()
    }

    pub fn check_and_reset_nmi(&mut self) -> bool {
        if self.cpu_action.contains(NMI) {
            self.cpu_action.remove(NMI);
            true
        } else {
            false
        }
    }

    pub fn check_and_reset_irq(&mut self) -> bool {
        if self.cpu_action.contains(IRQ) {
            self.cpu_action.remove(IRQ);
            true
        } else {
            false
        }
    }

    pub fn check_and_reset_dma(&mut self) -> Option<u8> {
        if self.cpu_action.contains(DMA) {
            self.cpu_action.remove(DMA);
            let channel_mask = self.dma_channel_mask;
            self.dma_channel_mask = 0x00;
            Some(channel_mask)
        } else {
            None
        }
    }
}

impl HardwareBus for HardwareRegs {
    fn read(&mut self, offset: usize) -> u8 {
        match offset {
            0x10 => {
                let nmi = if self.nmi.active { 0x80 } else { 0x00 };
                self.nmi.active = false;
                nmi | CHIP_VERSION
            },
            0x11 => {
                let irq = if self.irq.active { 0x80 } else { 0x00 };
                self.irq.active = false;
                irq
            },
            0x12 => {
                let mut value = 0x00;
                if self.vblank {
                    value |= 0x80;
                }
                if self.hblank {
                    value |= 0x40;
                }
                if self.joypad.auto_read_active > 0 {
                    value |= 0x01;
                }
                value
            },
            0x13 => self.io_port.value(),
            0x14 => self.division.result.lower(),
            0x15 => self.division.result.upper(),
            0x16 => self.multiplication.result.lower(),
            0x17 => self.multiplication.result.upper(),
            0x18 => self.joypad.button_state[0].lower(),
            0x19 => self.joypad.button_state[0].upper(),
            0x1A => self.joypad.button_state[1].lower(),
            0x1B => self.joypad.button_state[1].upper(),
            0x1C => self.joypad.button_state[2].lower(),
            0x1D => self.joypad.button_state[2].upper(),
            0x1E => self.joypad.button_state[3].lower(),
            0x1F => self.joypad.button_state[3].upper(),
            _ => 0x00 // TODO: Open bus
        }
    }

    fn write(&mut self, offset: usize, value: u8) {
        match offset {
            0x00 => {
                self.nmi.enabled = value & 0x80 != 0;
                self.joypad.auto_read_enabled = value & 0x01 != 0;

                self.irq.enabled = match value & 0x30 {
                    0x10 => IrqCondition::MatchColumn,
                    0x20 => IrqCondition::MatchRow,
                    0x30 => IrqCondition::MatchRowAndColumn,
                    _ => IrqCondition::Never
                };
            },
            0x01 => self.io_port.set_value(value),
            0x02 => self.multiplication.lhs = value,
            0x03 => self.multiplication.result = (self.multiplication.lhs as u16) * (value as u16),
            0x04 => self.division.lhs.set_lower(value),
            0x05 => self.division.lhs.set_upper(value),
            0x06 => {
                // Multiplication result is used to store remainder
                if value != 0 {
                    self.division.result = self.division.lhs / (value as u16);
                    self.multiplication.result = self.division.lhs % (value as u16);
                } else {
                    self.division.result = 0xFFFF;
                    self.multiplication.result = self.division.lhs;
                }
            },
            0x07 => self.irq.column.set_lower(value),
            0x08 => self.irq.column.set_upper(value & 0x01),
            0x09 => self.irq.row.set_lower(value),
            0x0A => self.irq.row.set_upper(value & 0x01),
            0x0B => {
                self.dma_channel_mask = value;
                if value != 0x00 {
                    self.cpu_action.insert(DMA);
                }
            }
            _ => ()
        }
    }
}
