# Operations

## List a mock device

Run `usbwatch list` for a deterministic JSON Lines record. Pass
`--fixture PATH` to load a recorded session.

## Inspect descriptors

Run `usbwatch inspect --vid 1234 --pid 5678 --fixture fixtures/device-session.json`.
IDs are hexadecimal, with or without a `0x` prefix.

## Replay a session

Run `usbwatch replay fixtures/device-session.json`. Each line is one ordered
event. Use `--max-events` to lower the configured bound.

Errors exit two. The command never opens a device path and never sends network
traffic.
