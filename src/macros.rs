#[macro_export]
macro_rules! verbose_println {
    ($($p:expr),+ $(,)?) => {{}};
}

#[macro_export]
macro_rules! println {
    ($($p:expr),*) => {{}};
}

#[macro_export]
macro_rules! print {
    ($($p:expr),*) => {{}};
}

#[macro_export]
macro_rules! info {
    ($($p:expr),+ $(,)?) => {{}};
}

#[macro_export]
macro_rules! warn {
    ($($p:expr),+ $(,)?) => {{}};
}

macro_rules! line_err {
    ($line:expr, $kind:expr, $($arg:tt)*) => {
        crate::error::Error::new($kind, None, &alloc::format!($($arg)*))
    };
}

macro_rules! general_err {
    ($($arg:tt)*) => {
        crate::error::Error::new($crate::error::ErrorKind::General, None, &alloc::format!($($arg)*))
    };
}

macro_rules! syntax_err {
    ($($arg:tt)*) => {
        crate::error::Error::new($crate::error::ErrorKind::Syntax, None, &alloc::format!($($arg)*))
    };
}

macro_rules! syntax_err_line {
    ($line:expr, $($arg:tt)*) => {
        crate::error::Error::new($crate::error::ErrorKind::Syntax, None, &alloc::format!($($arg)*))
    };
}

macro_rules! syntax_err_ctx {
    ($ctx:expr, $($arg:tt)*) => {
        crate::error::Error::new($crate::error::ErrorKind::Syntax, $ctx, &alloc::format!($($arg)*))
    };
}

macro_rules! runtime_err {
    ($ctx:expr, $($arg:tt)*) => {
        crate::error::Error::new($crate::error::ErrorKind::Runtime, $ctx, &alloc::format!($($arg)*))
    };
}

#[macro_export]
macro_rules! alt_screen_buffer {
    () => {{}};
}

#[macro_export]
macro_rules! main_screen_buffer {
    () => {{}};
}

#[macro_export]
macro_rules! xor {
    ($a: expr, $b: expr) => {
        ((($a) && !($b)) || (!($a) && ($b)))
    };
}

#[macro_export]
macro_rules! bit {
    ($a: expr, $b: expr) => {
        (((($a) as u32) & (1 << ($b) as u32)) != 0)
    };
}

#[macro_export]
macro_rules! clear_screen {
    () => {{}};
}

#[macro_export]
macro_rules! blue {
    ($msg:expr) => {
        $msg
    };
}

#[macro_export]
macro_rules! red {
    ($msg:expr) => {
        $msg
    };
}

#[macro_export]
macro_rules! green {
    ($msg:expr) => {
        $msg
    };
}

#[macro_export]
macro_rules! yellow {
    ($msg:expr) => {
        $msg
    };
}

#[macro_export]
macro_rules! color {
    ($color: literal, $msg: expr) => {
        $msg
    };
}
