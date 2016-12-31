pub trait Interrupt {
    fn as_str() -> &'static str;
    fn native_vector() -> u16;
    fn emulation_vector() -> u16;
    fn has_signature() -> bool;
    fn set_break() -> bool;
    fn set_interrupt_disable() -> bool;
}

pub struct Break;
pub struct Coprocessor;
pub struct Irq;
pub struct Nmi;

impl Interrupt for Break {
    fn as_str() -> &'static str {
        "BRK"
    }

    fn native_vector() -> u16 {
        0xFFE6
    }

    fn emulation_vector() -> u16 {
        0xFFFE
    }

    fn has_signature() -> bool {
        true
    }

    fn set_break() -> bool {
        true
    }

    fn set_interrupt_disable() -> bool {
        true
    }
}

impl Interrupt for Coprocessor {
    fn as_str() -> &'static str {
        "COP"
    }

    fn native_vector() -> u16 {
        0xFFE5
    }

    fn emulation_vector() -> u16 {
        0xFFF4
    }

    fn has_signature() -> bool {
        true
    }

    fn set_break() -> bool {
        false
    }

    fn set_interrupt_disable() -> bool {
        true
    }
}

impl Interrupt for Irq {
    fn as_str() -> &'static str {
        "IRQ"
    }

    fn native_vector() -> u16 {
        0xFFEE
    }

    fn emulation_vector() -> u16 {
        0xFFFE
    }

    fn has_signature() -> bool {
        false
    }

    fn set_break() -> bool {
        false
    }

    fn set_interrupt_disable() -> bool {
        true
    }
}

impl Interrupt for Nmi {
    fn as_str() -> &'static str {
        "NMI"
    }

    fn native_vector() -> u16 {
        0xFFEA
    }

    fn emulation_vector() -> u16 {
        0xFFFA
    }

    fn has_signature() -> bool {
        false
    }

    fn set_break() -> bool {
        false
    }

    fn set_interrupt_disable() -> bool {
        false
    }
}
