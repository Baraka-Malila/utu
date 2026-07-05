// Utu — ASUS Laptop Control for Ubuntu (fork of Ayuz by Guido Philipp)
// SPDX-License-Identifier: GPL-3.0-or-later

/// D-Bus proxy for GNOME SettingsDaemon screen brightness.
/// Replaces the KDE PowerDevil proxy from the upstream Ayuz project.
#[zbus::proxy(
    interface = "org.gnome.SettingsDaemon.Power.Screen",
    default_service = "org.gnome.SettingsDaemon.Power",
    default_path = "/org/gnome/SettingsDaemon/Power"
)]
pub trait GnomeBrightnessControl {
    #[zbus(property, name = "Brightness")]
    fn brightness(&self) -> zbus::Result<i32>;

    #[zbus(property, name = "Brightness")]
    fn set_brightness(&self, value: i32) -> zbus::Result<()>;
}

/// Returns the current brightness (0–100). Returns `Err` if
/// GNOME SettingsDaemon is not running (e.g. non-GNOME session).
pub async fn get_brightness() -> Result<i32, String> {
    let conn = zbus::Connection::session().await.map_err(|e| e.to_string())?;
    let proxy = GnomeBrightnessControlProxy::new(&conn)
        .await
        .map_err(|e| e.to_string())?;
    proxy.brightness().await.map_err(|e| e.to_string())
}

/// Sets screen brightness to an absolute value (1–100).
pub async fn set_brightness_absolute(value: i32) -> Result<(), String> {
    let conn = zbus::Connection::session().await.map_err(|e| e.to_string())?;
    let proxy = GnomeBrightnessControlProxy::new(&conn)
        .await
        .map_err(|e| e.to_string())?;
    proxy
        .set_brightness(value.clamp(1, 100))
        .await
        .map_err(|e| e.to_string())
}

/// Adjusts screen brightness by `delta_percent` (e.g. +5 or -5), clamped
/// to 1–100. Returns `Err` if GNOME SettingsDaemon is unreachable.
pub async fn adjust_brightness_relative(delta_percent: i32) -> Result<(), String> {
    let conn = zbus::Connection::session().await.map_err(|e| e.to_string())?;
    let proxy = GnomeBrightnessControlProxy::new(&conn)
        .await
        .map_err(|e| e.to_string())?;
    let cur = proxy.brightness().await.map_err(|e| e.to_string())?;
    let next = (cur + delta_percent).clamp(1, 100);
    proxy.set_brightness(next).await.map_err(|e| e.to_string())
}

/// Returns 100 — GNOME uses a 0–100 scale so the max is always 100.
/// Provided for API compatibility with callers that previously asked KDE
/// for a variable `brightness_max`.
pub fn brightness_max() -> i32 {
    100
}
