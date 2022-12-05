# TUNNEL-Firmware

Firmware for the TUNNEL Through the Wall capstone project.

## QEMU Usage

```shell
qemu-system-arm -cpu cortex-m4 \
  -machine netduinoplus2 \
  -nographic \
  -semihosting-config enable=on,target=native \
  -kernel target/thumbv7em-none-eabihf/debug/tunnel_firmware

```