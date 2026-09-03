use std::process::Command;

use usbwatch::{
    ControlRequest, ControlTransfer, EventKind, HotplugEvent, MAX_DESCRIPTOR_BYTES, MAX_EVENTS,
    MockBackend, MockDevice, PermissionState, default_session, enumerate_interfaces,
    inspect_device, parse_descriptors, replay_session,
};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_usbwatch"))
}

fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/device-session.json")
}

#[test]
fn inspect_command_enumerates_interfaces_and_endpoints() {
    let output = binary()
        .args(["inspect", "--vid", "1234", "--pid", "5678", "--fixture"])
        .arg(fixture())
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["vid"], 4660);
    assert_eq!(
        json["interfaces"][0]["endpoints"].as_array().unwrap().len(),
        2
    );
}

#[test]
fn list_is_json_lines() {
    let output = binary().args(["list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(lines[0]).unwrap()["id"],
        "mock-keyboard"
    );
}

#[test]
fn replay_includes_hotplug_and_control_transfer() {
    let output = binary().args(["replay"]).arg(fixture()).output().unwrap();
    assert!(output.status.success());
    let records: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(records.len(), 4);
    assert_eq!(records[1]["event"], "control.transfer");
    assert_eq!(records[1]["data_bytes"], 4);
    assert_eq!(records[2]["status"], "detached");
    assert_eq!(records[3]["status"], "device-disappeared");
}

#[test]
fn permission_error_and_disappearance_are_explicit() {
    let mut session = default_session();
    session.devices.push(MockDevice {
        id: "denied".to_owned(),
        vid: 0x1111,
        pid: 0x2222,
        product: "Denied".to_owned(),
        descriptor_hex: session.devices[0].descriptor_hex.clone(),
        permission: PermissionState::Denied,
    });
    session.events = vec![
        HotplugEvent {
            sequence: 1,
            kind: EventKind::Attach,
            device_id: "denied".to_owned(),
            detail: None,
            transfer: None,
        },
        HotplugEvent {
            sequence: 2,
            kind: EventKind::ControlTransfer,
            device_id: "mock-keyboard".to_owned(),
            detail: None,
            transfer: Some(ControlTransfer {
                request: ControlRequest {
                    bm_request_type: 128,
                    b_request: 6,
                    w_value: 0x100,
                    w_index: 0,
                    w_length: 4,
                },
                data_hex: "01020304".to_owned(),
                status: "ok".to_owned(),
            }),
        },
    ];
    let records = replay_session(&session, MAX_EVENTS).unwrap();
    assert_eq!(records[0].status, "permission-denied");
    assert_eq!(records[1].status, "device-disappeared");
}

#[test]
fn malformed_descriptor_lengths_are_rejected() {
    assert!(parse_descriptors(&[0, 1]).is_err());
    assert!(parse_descriptors(&[18, 1, 0]).is_err());
    assert!(parse_descriptors(&vec![2u8; MAX_DESCRIPTOR_BYTES + 1]).is_err());
}

#[test]
fn mock_backend_supports_concurrent_reads() {
    let backend = MockBackend::new(&default_session()).unwrap();
    let workers: Vec<_> = (0..16)
        .map(|_| {
            let backend = backend.clone();
            std::thread::spawn(move || backend.inspect(0x1234, 0x5678).unwrap().descriptor_count)
        })
        .collect();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), 5);
    }
}

#[test]
fn output_is_bounded_by_event_limit() {
    let mut session = default_session();
    session.events = (0..=MAX_EVENTS as u64)
        .map(|sequence| HotplugEvent {
            sequence,
            kind: EventKind::Attach,
            device_id: "mock-keyboard".to_owned(),
            detail: None,
            transfer: None,
        })
        .collect();
    assert!(replay_session(&session, MAX_EVENTS).is_err());
}

#[test]
fn library_inspection_rejects_permission_denial() {
    let mut device = default_session().devices.remove(0);
    device.permission = PermissionState::Denied;
    assert!(inspect_device(&device).is_err());
}

#[test]
fn interface_parser_is_stable() {
    let session = default_session();
    let bytes = usbwatch::parse_hex_bytes(&session.devices[0].descriptor_hex, MAX_DESCRIPTOR_BYTES)
        .unwrap();
    let first = enumerate_interfaces(&bytes).unwrap();
    let second = enumerate_interfaces(&bytes).unwrap();
    assert_eq!(first, second);
}
