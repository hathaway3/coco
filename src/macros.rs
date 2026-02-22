
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

macro_rules! acia_dbg {
    ($($e:expr),+ $(,)?) => {{}};
}

macro_rules! line_err {
    ($line:expr, $kind:expr, $msg:expr $(,)?) => {
        Error::new($kind, None, "")
    };
}

macro_rules! general_err {
    ($($msg:expr),* $(,)?) => {
        Error::new($crate::ErrorKind::General, None, "")
    };
}

macro_rules! syntax_err {
    ($msg:expr $(,)?) => {
        Error::new($crate::ErrorKind::Syntax, None, "")
    };
}

macro_rules! syntax_err_line {
    ($line:expr, $msg:expr $(,)?) => {
        Error::new($crate::ErrorKind::Syntax, None, "")
    };
}

macro_rules! syntax_err_ctx {
    ($ctx:expr,$msg:expr $(,)?) => {
        Error::new($crate::ErrorKind::Syntax, $ctx, "")
    };
}

macro_rules! instruction_invalid {
    ($ctx:expr, $($msg:expr),* $(,)?) => {
        Error::new($crate::ErrorKind::Runtime, $ctx, "")
    };
}

macro_rules! runtime_err {
    ($ctx:expr, $($msg:expr),* $(,)?) => {
        Error::new($crate::ErrorKind::Runtime, $ctx, "")
    };
}

macro_rules! err {
    ($kind:expr,$ctx:expr, $($msg:expr),* $(,)?) => {
        Error::new($kind, $ctx, "")
    };
}

macro_rules! within_usize_bound {
    ($val:expr,$bound:expr) => {
        ((($val) as usize) < (($bound) as usize))
    };
}

macro_rules! break_on_error {
    ($result: expr) => {
        if ($result).is_err() {
            break;
        }
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
