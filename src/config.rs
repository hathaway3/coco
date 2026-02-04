// use super::*;

pub struct Args {
    pub debug: bool,
    pub trace: bool,
    pub no_auto_sym: bool,
    pub verbose: bool,
    pub break_start: bool,
}

pub static mut ARGS: Args = Args {
    debug: false,
    trace: false,
    no_auto_sym: true,
    verbose: false,
    break_start: false,
};

pub fn init() {}

pub fn auto_load_syms() -> bool {
    unsafe { !ARGS.no_auto_sym && ARGS.debug }
}

pub fn debug() -> bool {
    unsafe { ARGS.debug }
}

pub fn trace() -> bool {
    unsafe { ARGS.trace }
}

pub fn help_humans() -> bool {
    unsafe { ARGS.debug || ARGS.trace }
}

pub fn verbose() -> bool {
    unsafe { ARGS.verbose }
}
