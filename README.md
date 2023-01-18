# TUNNEL-Firmware

Firmware for the TUNNEL Through the Wall capstone project.

## Building

Due to conditional compilation, you must specify what features to use which
control panic handling. The tree options are `RTT`, `HALT`, and `QEMU`. `RTT`
is good for on-board debugging. `QEMU` is good for emulator debugging. `HALT`
is good for performance.

The typical build command will be:
```shell
cargo build --features RTT
cargo run --features RTT
```

## QEMU Usage

```shell
qemu-system-arm -cpu cortex-m4 \
  -machine netduinoplus2 \
  -nographic \
  -semihosting-config enable=on,target=native \
  -kernel target/thumbv7em-none-eabihf/debug/tunnel_firmware

```