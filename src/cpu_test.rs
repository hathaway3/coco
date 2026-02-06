use crate::*;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec;
use spin::Mutex;

fn create_core() -> Core {
    // 64K RAM
    let ram = Box::leak(vec![0u8; 0x10000].into_boxed_slice());
    let sam = Arc::new(Mutex::new(Sam::new()));
    let vdg = Arc::new(Mutex::new(Vdg::with_ram(0)));
    let pia1 = Arc::new(Mutex::new(Pia1::new()));
    let pia0 = Arc::new(Mutex::new(Pia0::new(pia1.clone())));

    Core::new(ram, sam, vdg, pia0, pia1, 0xFFFF, None)
}

#[test]
fn test_lda_immediate() {
    let mut core = create_core();
    // LDA #$55 -> 86 55
    // 86 = LDA Immediate
    let prog = [0x86, 0x55];
    core.load_bytes(&prog, 0x1000).unwrap();
    core.reg.pc = 0x1000;

    core.exec_one().unwrap();

    assert_eq!(core.reg.a, 0x55);
    // LDA #$55 is 2 bytes
    assert_eq!(core.reg.pc, 0x1002);
}

#[test]
fn test_ldb_extended() {
    let mut core = create_core();
    // LDB $2000 -> F6 20 00 (Extended addressing)
    // Store $99 at $2000 first
    core.load_bytes(&[0x99], 0x2000).unwrap();

    let prog = [0xF6, 0x20, 0x00];
    core.load_bytes(&prog, 0x1000).unwrap();
    core.reg.pc = 0x1000;

    core.exec_one().unwrap();

    assert_eq!(core.reg.b, 0x99);
    assert_eq!(core.reg.pc, 0x1003);
}

#[test]
fn test_add_set_flags() {
    let mut core = create_core();
    // ADDA #$01 -> 8B 01
    // Initial A = 0
    let prog = [0x8B, 0x01];
    core.load_bytes(&prog, 0x1000).unwrap();
    core.reg.pc = 0x1000;

    core.exec_one().unwrap();

    assert_eq!(core.reg.a, 0x01);
    assert!(!core.reg.cc.is_set(registers::CCBit::Z));
    assert!(!core.reg.cc.is_set(registers::CCBit::N));
}
