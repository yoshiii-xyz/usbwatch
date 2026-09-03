# Design

The library has four layers: bounded hex decoding, descriptor framing,
interface and endpoint enumeration, and recorded event replay. A session is
validated before any output is produced. Device IDs are unique, descriptor
streams are capped at 64 KiB, transfer payloads at 64 KiB, and sessions at 8
MiB.

The mock backend stores an immutable device list behind an `Arc<Mutex<_>>`.
This exercises concurrent inspection without making a hardware or kernel
assumption. Replay sorts by sequence and turns permission and disappearance
conditions into explicit statuses.
