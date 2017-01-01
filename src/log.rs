static mut trace_mode: bool = false;

#[macro_export]
macro_rules! info {
    ($($args:tt)+) => {{
        println!($($args)+)
    }}
}

#[macro_export]
macro_rules! warn {
    ($($args:tt)+) => {{
        println!($($args)+)
    }}
}

#[macro_export]
macro_rules! debug {
    ($($args:tt)+) => {{
        if cfg!(debug_assertions) && ::log::trace_mode_enabled() {
            println!($($args)+)
        }
    }}
}

pub fn trace_mode_enabled() -> bool {
    unsafe { trace_mode }
}

pub fn enable_trace_mode() {
    unsafe { trace_mode = true };
}
