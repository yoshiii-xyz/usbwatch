# Research record

## Decision

The MVP uses the standard USB descriptor framing and a small own-data model
instead of binding to a host USB library. This keeps the parser testable with
fixtures and makes the hardware gap explicit. The first parser recognizes the
standard device, configuration, interface, and endpoint descriptor lengths and
preserves unknown descriptor types as raw bounded records.

The Rust USB documentation describes descriptor blocks and the relationship
between interfaces and endpoints. The control request fields used by the
fixture follow the host-facing request model. These references support the
protocol boundary, not a claim of complete USB compliance.

## Sources

- USB descriptor data types: https://docs.rs/usb/latest/usb/
- USB descriptor parsing model: https://docs.rs/usbh/latest/usbh/descriptor/
- USB endpoint definitions: https://docs.rs/usb-device/latest/usb_device/endpoint/
- USB control request fields: https://docs.rs/usb-device/latest/usb_device/control/struct.Request.html
