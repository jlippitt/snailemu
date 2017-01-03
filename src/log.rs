static mut TRACE_MODE: bool = false;

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
    unsafe { TRACE_MODE }
}

pub fn enable_trace_mode() {
    unsafe { TRACE_MODE = true };
}
