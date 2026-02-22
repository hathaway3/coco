use core::sync::atomic::{AtomicBool, Ordering};

pub struct Args {
    pub debug: AtomicBool,
    pub trace: AtomicBool,
    pub no_auto_sym: AtomicBool,
    pub verbose: AtomicBool,
    pub break_start: AtomicBool,
}

pub static ARGS: Args = Args {
    debug: AtomicBool::new(false),
    trace: AtomicBool::new(false),
    no_auto_sym: AtomicBool::new(true),
    verbose: AtomicBool::new(false),
    break_start: AtomicBool::new(false),
};

pub fn auto_load_syms() -> bool {
    !ARGS.no_auto_sym.load(Ordering::Relaxed) && ARGS.debug.load(Ordering::Relaxed)
}

pub fn debug() -> bool {
    ARGS.debug.load(Ordering::Relaxed)
}

pub fn trace() -> bool {
    ARGS.trace.load(Ordering::Relaxed)
}

pub fn help_humans() -> bool {
    ARGS.debug.load(Ordering::Relaxed) || ARGS.trace.load(Ordering::Relaxed)
}

pub fn verbose() -> bool {
    ARGS.verbose.load(Ordering::Relaxed)
}
