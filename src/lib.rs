//! Bounded USB descriptor parsing and deterministic recorded-session replay.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub const MAX_DESCRIPTOR_BYTES: usize = 64 * 1024;
pub const MAX_SESSION_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_DEVICES: usize = 256;
pub const MAX_EVENTS: usize = 4096;
pub const MAX_TRANSFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsbError(pub String);

impl fmt::Display for UsbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UsbError {}

impl From<std::io::Error> for UsbError {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}

impl From<serde_json::Error> for UsbError {
    fn from(value: serde_json::Error) -> Self {
        Self(format!("invalid session JSON: {value}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    #[default]
    Ok,
    Denied,
    Disappeared,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MockDevice {
    pub id: String,
    pub vid: u16,
    pub pid: u16,
    #[serde(default)]
    pub product: String,
    pub descriptor_hex: String,
    #[serde(default)]
    pub permission: PermissionState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Attach,
    Detach,
    ControlTransfer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControlRequest {
    #[serde(rename = "bmRequestType")]
    pub bm_request_type: u8,
    #[serde(rename = "bRequest")]
    pub b_request: u8,
    #[serde(rename = "wValue")]
    pub w_value: u16,
    #[serde(rename = "wIndex")]
    pub w_index: u16,
    #[serde(rename = "wLength")]
    pub w_length: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControlTransfer {
    pub request: ControlRequest,
    #[serde(default)]
    pub data_hex: String,
    #[serde(default = "default_transfer_status")]
    pub status: String,
}

fn default_transfer_status() -> String {
    "ok".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HotplugEvent {
    pub sequence: u64,
    pub kind: EventKind,
    pub device_id: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub transfer: Option<ControlTransfer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub devices: Vec<MockDevice>,
    #[serde(default)]
    pub events: Vec<HotplugEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Descriptor {
    pub offset: usize,
    pub length: usize,
    pub descriptor_type: u8,
    pub bytes_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndpointSummary {
    pub address: u8,
    pub number: u8,
    pub direction: String,
    pub transfer_type: String,
    pub max_packet_size: u16,
    pub interval: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InterfaceSummary {
    pub number: u8,
    pub alternate_setting: u8,
    pub class_code: u8,
    pub subclass_code: u8,
    pub protocol: u8,
    pub declared_endpoints: u8,
    pub endpoints: Vec<EndpointSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Inspection {
    pub id: String,
    pub vid: u16,
    pub pid: u16,
    pub product: String,
    pub permission: String,
    pub descriptor_count: usize,
    pub descriptors: Vec<Descriptor>,
    pub interfaces: Vec<InterfaceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListRecord {
    pub id: String,
    pub vid: u16,
    pub pid: u16,
    pub product: String,
    pub permission: String,
    pub descriptor_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayRecord {
    pub sequence: u64,
    pub event: String,
    pub device_id: String,
    pub status: String,
    pub data_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

pub fn parse_hex_bytes(input: &str, max_bytes: usize) -> Result<Vec<u8>, UsbError> {
    let compact: String = input
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect();
    if compact.len() % 2 != 0 {
        return Err(UsbError("hex input has an odd number of digits".to_owned()));
    }
    if compact.len() / 2 > max_bytes {
        return Err(UsbError(format!(
            "hex input exceeds {max_bytes} byte limit"
        )));
    }
    hex::decode(compact).map_err(|_| UsbError("hex input contains a non-hex digit".to_owned()))
}

pub fn parse_hex_u16(input: &str) -> Result<u16, String> {
    let normalized = input
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    u16::from_str_radix(normalized, 16).map_err(|_| format!("invalid USB ID: {input}"))
}

pub fn parse_descriptors(bytes: &[u8]) -> Result<Vec<Descriptor>, UsbError> {
    if bytes.len() > MAX_DESCRIPTOR_BYTES {
        return Err(UsbError(format!(
            "descriptor stream exceeds {MAX_DESCRIPTOR_BYTES} byte limit"
        )));
    }
    let mut descriptors = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let remaining = bytes.len() - offset;
        if remaining < 2 {
            return Err(UsbError(format!(
                "descriptor at offset {offset} lacks its header"
            )));
        }
        let length = bytes[offset] as usize;
        if length < 2 {
            return Err(UsbError(format!(
                "descriptor at offset {offset} has invalid length {length}"
            )));
        }
        if length > remaining {
            return Err(UsbError(format!(
                "descriptor at offset {offset} extends past the stream"
            )));
        }
        let descriptor_type = bytes[offset + 1];
        let minimum = match descriptor_type {
            1 => 18,
            2 => 9,
            4 => 9,
            5 => 7,
            _ => 2,
        };
        if length < minimum {
            return Err(UsbError(format!(
                "descriptor type {descriptor_type} at offset {offset} is shorter than {minimum} bytes"
            )));
        }
        descriptors.push(Descriptor {
            offset,
            length,
            descriptor_type,
            bytes_hex: hex::encode(&bytes[offset..offset + length]),
        });
        offset += length;
    }
    Ok(descriptors)
}

pub fn enumerate_interfaces(bytes: &[u8]) -> Result<Vec<InterfaceSummary>, UsbError> {
    let descriptors = parse_descriptors(bytes)?;
    let mut interfaces = Vec::new();
    let mut current_interface = None;
    for descriptor in descriptors {
        let raw = &bytes[descriptor.offset..descriptor.offset + descriptor.length];
        match descriptor.descriptor_type {
            4 => {
                interfaces.push(InterfaceSummary {
                    number: raw[2],
                    alternate_setting: raw[3],
                    class_code: raw[5],
                    subclass_code: raw[6],
                    protocol: raw[7],
                    declared_endpoints: raw[4],
                    endpoints: Vec::new(),
                });
                current_interface = Some(interfaces.len() - 1);
            }
            5 => {
                if let Some(index) = current_interface {
                    let transfer_type = match raw[3] & 0x03 {
                        0 => "control",
                        1 => "isochronous",
                        2 => "bulk",
                        3 => "interrupt",
                        _ => "reserved",
                    };
                    interfaces[index].endpoints.push(EndpointSummary {
                        address: raw[2],
                        number: raw[2] & 0x0f,
                        direction: if raw[2] & 0x80 == 0 { "out" } else { "in" }.to_owned(),
                        transfer_type: transfer_type.to_owned(),
                        max_packet_size: u16::from_le_bytes([raw[4], raw[5]]) & 0x07ff,
                        interval: raw[6],
                    });
                }
            }
            _ => {}
        }
    }
    Ok(interfaces)
}

fn permission_label(permission: &PermissionState) -> &'static str {
    match permission {
        PermissionState::Ok => "ok",
        PermissionState::Denied => "permission-denied",
        PermissionState::Disappeared => "device-disappeared",
    }
}

fn checked_descriptor_bytes(device: &MockDevice) -> Result<Vec<u8>, UsbError> {
    let bytes = parse_hex_bytes(&device.descriptor_hex, MAX_DESCRIPTOR_BYTES)?;
    if bytes.is_empty() {
        return Err(UsbError(format!(
            "device {} has no descriptor bytes",
            device.id
        )));
    }
    Ok(bytes)
}

pub fn inspect_device(device: &MockDevice) -> Result<Inspection, UsbError> {
    if device.permission != PermissionState::Ok {
        return Err(UsbError(format!(
            "device {} is {}",
            device.id,
            permission_label(&device.permission)
        )));
    }
    let bytes = checked_descriptor_bytes(device)?;
    let descriptors = parse_descriptors(&bytes)?;
    let device_descriptor = descriptors
        .iter()
        .find(|descriptor| descriptor.descriptor_type == 1)
        .ok_or_else(|| UsbError("descriptor stream has no device descriptor".to_owned()))?;
    let raw = &bytes[device_descriptor.offset..device_descriptor.offset + device_descriptor.length];
    if raw.len() < 18 {
        return Err(UsbError("device descriptor is incomplete".to_owned()));
    }
    let parsed_vid = u16::from_le_bytes([raw[8], raw[9]]);
    let parsed_pid = u16::from_le_bytes([raw[10], raw[11]]);
    if parsed_vid != device.vid || parsed_pid != device.pid {
        return Err(UsbError(format!(
            "device ID does not match descriptor: expected {:04x}:{:04x}, found {:04x}:{:04x}",
            device.vid, device.pid, parsed_vid, parsed_pid
        )));
    }
    Ok(Inspection {
        id: device.id.clone(),
        vid: device.vid,
        pid: device.pid,
        product: device.product.clone(),
        permission: permission_label(&device.permission).to_owned(),
        descriptor_count: descriptors.len(),
        descriptors,
        interfaces: enumerate_interfaces(&bytes)?,
    })
}

pub fn list_devices(session: &Session) -> Result<Vec<ListRecord>, UsbError> {
    validate_session(session)?;
    session
        .devices
        .iter()
        .map(|device| {
            let descriptor_count = if device.permission == PermissionState::Ok {
                parse_descriptors(&checked_descriptor_bytes(device)?)?.len()
            } else {
                0
            };
            Ok(ListRecord {
                id: device.id.clone(),
                vid: device.vid,
                pid: device.pid,
                product: device.product.clone(),
                permission: permission_label(&device.permission).to_owned(),
                descriptor_count,
            })
        })
        .collect()
}

pub fn replay_session(session: &Session, max_events: usize) -> Result<Vec<ReplayRecord>, UsbError> {
    validate_session(session)?;
    if max_events == 0 || max_events > MAX_EVENTS {
        return Err(UsbError(format!(
            "max-events must be between 1 and {MAX_EVENTS}"
        )));
    }
    if session.events.len() > max_events {
        return Err(UsbError(format!(
            "session has {} events, above the configured limit {max_events}",
            session.events.len()
        )));
    }
    let devices: BTreeMap<_, _> = session
        .devices
        .iter()
        .map(|device| (device.id.as_str(), device))
        .collect();
    let mut attached = BTreeMap::new();
    let mut events = session.events.clone();
    events.sort_by_key(|event| event.sequence);
    let mut output = Vec::with_capacity(events.len());
    for event in events {
        let device = devices.get(event.device_id.as_str()).ok_or_else(|| {
            UsbError(format!(
                "event references unknown device {}",
                event.device_id
            ))
        })?;
        let mut record = ReplayRecord {
            sequence: event.sequence,
            event: match event.kind {
                EventKind::Attach => "device.attached",
                EventKind::Detach => "device.detached",
                EventKind::ControlTransfer => "control.transfer",
            }
            .to_owned(),
            device_id: event.device_id.clone(),
            status: "ok".to_owned(),
            data_bytes: 0,
            detail: event.detail.clone(),
        };
        match event.kind {
            EventKind::Attach => match device.permission {
                PermissionState::Ok => {
                    attached.insert(device.id.clone(), true);
                    record.status = "attached".to_owned();
                }
                PermissionState::Denied => record.status = "permission-denied".to_owned(),
                PermissionState::Disappeared => record.status = "device-disappeared".to_owned(),
            },
            EventKind::Detach => {
                attached.insert(device.id.clone(), false);
                record.status = "detached".to_owned();
            }
            EventKind::ControlTransfer => {
                let transfer = event
                    .transfer
                    .as_ref()
                    .ok_or_else(|| UsbError("control.transfer event has no transfer".to_owned()))?;
                let data = parse_hex_bytes(&transfer.data_hex, MAX_TRANSFER_BYTES)?;
                record.data_bytes = data.len();
                if device.permission == PermissionState::Denied {
                    record.status = "permission-denied".to_owned();
                } else if device.permission == PermissionState::Disappeared
                    || !attached.get(device.id.as_str()).copied().unwrap_or(false)
                {
                    record.status = "device-disappeared".to_owned();
                } else {
                    record.status = transfer.status.clone();
                }
            }
        }
        output.push(record);
    }
    Ok(output)
}

pub fn validate_session(session: &Session) -> Result<(), UsbError> {
    if session.devices.len() > MAX_DEVICES {
        return Err(UsbError(format!(
            "session exceeds {MAX_DEVICES} device limit"
        )));
    }
    if session.events.len() > MAX_EVENTS {
        return Err(UsbError(format!(
            "session exceeds {MAX_EVENTS} event limit"
        )));
    }
    let mut ids = BTreeMap::new();
    for device in &session.devices {
        if device.id.is_empty() || device.id.len() > 128 {
            return Err(UsbError("device ID must contain 1 to 128 bytes".to_owned()));
        }
        if ids.insert(device.id.clone(), ()).is_some() {
            return Err(UsbError(format!("duplicate device ID {}", device.id)));
        }
        let _ = parse_hex_bytes(&device.descriptor_hex, MAX_DESCRIPTOR_BYTES)?;
    }
    for event in &session.events {
        if event.device_id.is_empty() || event.device_id.len() > 128 {
            return Err(UsbError("event device ID is outside its bounds".to_owned()));
        }
        if let Some(transfer) = &event.transfer {
            let _ = parse_hex_bytes(&transfer.data_hex, MAX_TRANSFER_BYTES)?;
            if transfer.request.w_length as usize > MAX_TRANSFER_BYTES {
                return Err(UsbError(
                    "control transfer length exceeds the output limit".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

pub fn load_session(path: &Path) -> Result<Session, UsbError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        return Err(UsbError(format!(
            "session path is not a regular file: {}",
            path.display()
        )));
    }
    if metadata.len() as usize > MAX_SESSION_BYTES {
        return Err(UsbError(format!(
            "session exceeds {MAX_SESSION_BYTES} byte limit"
        )));
    }
    let data = fs::read(path)?;
    let session: Session = serde_json::from_slice(&data)?;
    validate_session(&session)?;
    Ok(session)
}

pub fn default_session() -> Session {
    Session {
        devices: vec![MockDevice {
            id: "mock-keyboard".to_owned(),
            vid: 0x1234,
            pid: 0x5678,
            product: "Mock Keyboard".to_owned(),
            descriptor_hex: "1201000200000040341278560001010203010902200001010080320904000002030101000705810308000a07050202400000".to_owned(),
            permission: PermissionState::Ok,
        }],
        events: vec![],
    }
}

#[derive(Clone)]
pub struct MockBackend {
    devices: Arc<Mutex<Vec<MockDevice>>>,
}

impl MockBackend {
    pub fn new(session: &Session) -> Result<Self, UsbError> {
        validate_session(session)?;
        Ok(Self {
            devices: Arc::new(Mutex::new(session.devices.clone())),
        })
    }

    pub fn list(&self) -> Result<Vec<MockDevice>, UsbError> {
        self.devices
            .lock()
            .map(|devices| devices.clone())
            .map_err(|_| UsbError("mock backend lock was poisoned".to_owned()))
    }

    pub fn inspect(&self, vid: u16, pid: u16) -> Result<Inspection, UsbError> {
        let device = self
            .list()?
            .into_iter()
            .find(|device| device.vid == vid && device.pid == pid)
            .ok_or_else(|| UsbError(format!("no mock device {:04x}:{:04x}", vid, pid)))?;
        inspect_device(&device)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw() -> Vec<u8> {
        parse_hex_bytes(
            "1201000200000040341278560001010203010902200001010080320904000002030101000705810308000a07050202400000",
            MAX_DESCRIPTOR_BYTES,
        )
        .unwrap()
    }

    #[test]
    fn parses_descriptor_stream() {
        let descriptors = parse_descriptors(&raw()).unwrap();
        assert_eq!(descriptors.len(), 5);
        assert_eq!(descriptors[0].descriptor_type, 1);
    }

    #[test]
    fn rejects_truncated_descriptor() {
        let error = parse_descriptors(&[7, 5, 1]).unwrap_err();
        assert!(error.0.contains("extends past"));
    }

    #[test]
    fn enumerates_interface_endpoints() {
        let interfaces = enumerate_interfaces(&raw()).unwrap();
        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].endpoints.len(), 2);
        assert_eq!(interfaces[0].endpoints[0].direction, "in");
        assert_eq!(interfaces[0].endpoints[1].transfer_type, "bulk");
    }

    #[test]
    fn mock_backend_can_be_read_concurrently() {
        let backend = MockBackend::new(&default_session()).unwrap();
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let backend = backend.clone();
                std::thread::spawn(move || backend.list().unwrap().len())
            })
            .collect();
        for thread in threads {
            assert_eq!(thread.join().unwrap(), 1);
        }
    }
}
