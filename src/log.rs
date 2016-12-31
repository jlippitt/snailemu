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
        if cfg!(debug_assertions) {
            println!($($args)+)
        }
    }}
}
