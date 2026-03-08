# DriveWire 4.0 Protocol Reference

## Opcode Table (Core)

| Opcode | Name | Description | Length |
|--------|------|-------------|--------|
| $5A | `OP_DWINIT` | Initialization handshake | 1 |
| $D2 | `OP_READEX` | Fast sector read (256 bytes) | 1+1+2 (Op, Unit, LSN) |
| $D3 | `OP_WRITE` | Sector write (256 bytes) | 1+1+2 (Op, Unit, LSN) |
| $72 | `OP_REREAD` | Repeat last read | 1 |
| $77 | `OP_REWRITE` | Repeat last write | 1 |
| $D6 | `OP_RFM` | Remote File Manager sub-ops | 1+SubOp |

## RFM (Remote File Manager) Sub-Operations

$D0=CREATE, $D1=OPEN, $D2=READ, $D3=WRITE, $D4=READLN, $D5=WRITLN, $D6=GETSTT, $D7=SETSTT, $D8=CLOSE, $D9=DELETE, $DA=MAKDIR, $DB=CHGDIR, $DC=DELDIR.

## Protocol Handshakes

### DWINIT
1. CoCo sends `$5A`
2. Server responds with `$5A`
3. CoCo sends `$5A`

### READEX
1. CoCo sends `$D2` + Unit + LSN (2 bytes)
2. Server sends 256 bytes of data
3. CoCo sends 1 byte checksum
4. Server responds with `$00` (ACK) or `$FF` (NAK)

## Error Codes

| Code | Label | Meaning |
|------|-------|---------|
| 0 | `E_NONE` | Success |
| 240 | `E_UNIT` | Invalid drive number |
| 242 | `E_WP` | Write protected |
| 243 | `E_CRC` | CRC/Checksum error |
| 244 | `E_READ` | Read error |
| 246 | `E_NOTRDY` | Drive not ready / Connection failed |
