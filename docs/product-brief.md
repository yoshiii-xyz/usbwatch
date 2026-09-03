# Product brief

## Mission

Let developers inspect USB descriptors, interfaces, endpoints, hotplug events,
and selected control transfers from a safe CLI and library.

## MVP

The Linux-first MVP is software-only and works on recorded JSON sessions:

```text
usbwatch list
usbwatch inspect --vid VID --pid PID
usbwatch replay fixtures/device-session.json
```

The default mock device makes `list` and `inspect` runnable without hardware.
The fixture backend adds deterministic hotplug and control-transfer replay.

## Completion boundary

Descriptor parsing, malformed-length handling, interface and endpoint
enumeration, replay status handling, bounded output, JSON Lines output, and
concurrent mock reads are tested. Real devices, kernel drivers, permissions on
actual device nodes, and cross-platform behavior are not claimed.
