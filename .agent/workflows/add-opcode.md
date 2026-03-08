---
description: How to safely add a new DriveWire opcode to the Rust emulator
---

1. **Identify the Opcode**: Check [.agent/knowledge/drivewire-protocol.md](../knowledge/drivewire-protocol.md) or the spec for the command byte.
2. **Locate I/O Provider**: Identify the module handling DriveWire (likely a future `dw_acia` or similar).
3. **Define Constant**: Add the opcode to a relevant constants module.
4. **Implement logic**:
   - Add a match arm for the new opcode.
   - Use non-blocking I/O where possible.
   - **Performance**: Ensure the handler doesn't stall the emulator core for more than a few cycles.
5. **Verify**: Add a test case to the Rust test suite simulating the protocol handshake.
