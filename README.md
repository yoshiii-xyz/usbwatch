# usbwatch

usbwatch is a Linux-first USB inspection core for recorded descriptors and
mock sessions. It parses bounded USB descriptor streams, enumerates interfaces
and endpoints, and replays hotplug and control-transfer records without
opening a real device.

Status: released v0.1.0 software core. Real hardware, kernel-driver
interaction, and cross-platform behavior are outside this release boundary.

## Install

```text
cargo install usbwatch --version 0.1.0
```

## Quick start

```text
usbwatch list
usbwatch inspect --vid 1234 --pid 5678 --fixture fixtures/device-session.json
usbwatch replay fixtures/device-session.json
```

`list` prints one JSON object per device. `inspect` prints a JSON report with
raw descriptor summaries, interfaces, and endpoints. `replay` prints one JSON
object per recorded event in sequence order.

## What it solves

The software core makes USB protocol inspection testable when no device or
host USB permission is available. A recorded session describes mock devices,
hotplug events, and selected control transfers. The parser follows the common
USB descriptor framing of a length byte followed by a type byte.

## Output and errors

Successful commands exit zero. Invalid input, permission states, unknown
devices, malformed descriptor lengths, and resource-limit violations are
reported on stderr and exit two. Replay keeps permission errors and device
disappearance as explicit event statuses.

## Safety and privacy

The MVP reads only the supplied fixture and does not enumerate host devices or
perform USB I/O. It retains no state and sends no data over the network.
Recorded transfer data is bounded by 64 KiB and is printed only when the
fixture requests replay output.

## Limits and non-goals

See [docs/limits.md](docs/limits.md). The project is not a consumer USB
application, a kernel driver, a real-device compatibility promise, or a
cross-platform backend.

## Testing and development

See [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/release.md](docs/release.md)
for the bounded local and hosted validation commands.

## Release and support status

Version 0.1.0 is a released software core. Hardware and operating-system gaps
remain explicit until the required environments are available.

## License

MIT. See [LICENSE](LICENSE).
