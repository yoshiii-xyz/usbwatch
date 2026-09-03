# Limits

- Maximum descriptor stream: 64 KiB.
- Maximum control-transfer data: 64 KiB.
- Maximum session JSON: 8 MiB.
- Maximum devices: 256.
- Maximum replay events: 4096.
- Only recorded JSON and the built-in mock device are supported.
- The parser does not implement every class-specific descriptor.
- No real USB enumeration, control transfer, kernel-driver interaction, or
  cross-platform backend is included.
- Human usability and physical-device validation are excluded.
