// Utu — ASUS Laptop Control for Ubuntu (fork of Ayuz by Guido Philipp)
// SPDX-License-Identifier: GPL-3.0-or-later

use evdev::{Device, EventSummary, KeyCode};

use crate::app::AppMsg;
use crate::services::evdev_runner::open_event_stream;

/// KEY_PROG1 (148) — the physical Armoury Crate button on ASUS laptops.
/// Emitted by asus-nb-wmi via the `Asus WMI hotkeys` evdev node.
const ARMOURY_KEYCODES: &[u16] = &[148];

fn find_armoury_device() -> Option<Device> {
    let mut fallback: Option<Device> = None;

    for (_, device) in evdev::enumerate() {
        let name = device.name().unwrap_or_default().to_lowercase();
        let is_asus = name.contains("asus") && (name.contains("wmi") || name.contains("hotkey"));

        if let Some(keys) = device.supported_keys() {
            let has_key = ARMOURY_KEYCODES
                .iter()
                .any(|&c| keys.contains(KeyCode::new(c)));
            if is_asus && has_key {
                return Some(device);
            }
            if has_key && fallback.is_none() {
                fallback = Some(device);
            }
        }
    }
    fallback
}

/// Watches the Armoury Crate button and emits [`AppMsg::ShowWindow`] on each
/// press. Returns immediately if no device advertising KEY_PROG1 is found.
pub async fn run(sender: relm4::Sender<AppMsg>) {
    let Some(device) = find_armoury_device() else {
        tracing::info!("armoury_key: no device found — Armoury Crate key unavailable");
        return;
    };

    if let Some(name) = device.name() {
        tracing::info!("armoury_key: listening on {name}");
    }

    let Some(mut stream) = open_event_stream(device) else {
        return;
    };

    loop {
        let event = match stream.next_event().await {
            Ok(ev) => ev,
            Err(e) => {
                tracing::warn!("armoury_key: event read error: {e}");
                break;
            }
        };

        if let EventSummary::Key(_, key, 1) = event.destructure() {
            if ARMOURY_KEYCODES.contains(&key.code()) {
                sender.emit(AppMsg::ShowWindow);
            }
        }
    }
}
