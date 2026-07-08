// Utu - ASUS Laptop Control Centre for Ubuntu
// Copyright (C) 2026 Guido Philipp, Baraka Malila
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see https://www.gnu.org/licenses/.

use gtk4::glib;
use gtk4::prelude::*;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::commands::pkexec_write_sysfs;
use crate::sys_paths::SYS_MEM_SLEEP;
use crate::services::config::AppConfig;
use crate::services::dbus;

pub struct BatteryModel {
    asusd_available: bool,
    maintenance_mode_active: bool,
    deep_sleep_active: bool,
    deep_sleep_supported: bool,
}

#[derive(Debug)]
pub enum BatteryMsg {
    ToggleMaintenanceMode(bool),
    TriggerOneShot,
    ToggleDeepSleep(bool),
    LoadProfile(bool),
}

#[derive(Debug)]
pub enum BatteryCommandOutput {
    AsusdChecked(bool),
    ChargeLimitSet(u8),
    Error(String),
    TimerElapsed,
    InitValue(u8),
    InitDeepSleep(bool),
    DeepSleepSet(bool),
    DeepSleepSupported(bool),
}

#[relm4::component(pub)]
impl Component for BatteryModel {
    type Init = ();
    type Input = BatteryMsg;
    type Output = String;
    type CommandOutput = BatteryCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &glib::markup_escape_text(&t!("battery_group_title")),
            set_description: Some(&t!("battery_group_desc")),

            #[template]
            add = &crate::components::widgets::DaemonWarningLabel {
                #[watch]
                set_visible: !model.asusd_available,
                set_label: &t!("asusd_missing_warning"),
            },

            add = &adw::SwitchRow {
                set_title: &t!("battery_maintenance_title"),
                set_subtitle: &t!("battery_maintenance_subtitle"),
                #[watch]
                set_active: model.maintenance_mode_active,
                #[watch]
                set_sensitive: model.asusd_available,
                connect_active_notify[sender] => move |switch| {
                    sender.input(BatteryMsg::ToggleMaintenanceMode(switch.is_active()));
                },
            },

            add = &adw::ActionRow {
                set_title: &t!("battery_one_shot_label"),
                set_subtitle: "",
                add_prefix = &gtk4::Image {
                    set_icon_name: Some("battery-full-charging-symbolic"),
                    set_pixel_size: 16,
                },
                add_suffix = &gtk4::Button {
                    set_label: &t!("battery_one_shot_label"),
                    add_css_class: "suggested-action",
                    set_valign: gtk4::Align::Center,
                    #[watch]
                    set_sensitive: model.asusd_available && model.maintenance_mode_active,
                    connect_clicked[sender] => move |_| {
                        sender.input(BatteryMsg::TriggerOneShot);
                    },
                },
            },

            add = &adw::SwitchRow {
                set_title: &t!("battery_deep_sleep_title"),
                #[watch]
                set_subtitle: &if model.deep_sleep_supported {
                    t!("battery_deep_sleep_subtitle")
                } else {
                    t!("battery_deep_sleep_not_supported")
                },
                #[watch]
                set_sensitive: model.deep_sleep_supported,
                #[watch]
                set_active: model.deep_sleep_active,
                connect_active_notify[sender] => move |switch| {
                    sender.input(BatteryMsg::ToggleDeepSleep(switch.is_active()));
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = BatteryModel {
            asusd_available: false,
            maintenance_mode_active: false,
            deep_sleep_active: false,
            deep_sleep_supported: false,
        };
        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let available = dbus::check_asusd_available().await;
                    out.emit(BatteryCommandOutput::AsusdChecked(available));
                    if available {
                        match dbus::get_charge_limit().await {
                            Ok(val) => out.emit(BatteryCommandOutput::InitValue(val)),
                            Err(e) => out.emit(BatteryCommandOutput::Error(e)),
                        }
                    }
                })
                .drop_on_shutdown()
        });

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    match tokio::fs::read_to_string(SYS_MEM_SLEEP).await {
                        Ok(content) => {
                            out.emit(BatteryCommandOutput::InitDeepSleep(
                                content.contains("[deep]"),
                            ));
                            out.emit(BatteryCommandOutput::DeepSleepSupported(
                                content.contains("deep"),
                            ));
                        }
                        Err(e) => {
                            out.emit(BatteryCommandOutput::Error(
                                t!("error_mem_sleep_read", error = e.to_string()).to_string(),
                            ));
                        }
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: BatteryMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            BatteryMsg::ToggleMaintenanceMode(active) => {
                if active == self.maintenance_mode_active {
                    return;
                }
                self.maintenance_mode_active = active;
                let limit: u8 = if active { 80 } else { 100 };
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_charge_limit(limit).await {
                                Ok(v) => out.emit(BatteryCommandOutput::ChargeLimitSet(v)),
                                Err(e) => out.emit(BatteryCommandOutput::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            BatteryMsg::TriggerOneShot => {
                // Set to 100% now; auto-revert to 80% after 24h.
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_charge_limit(100).await {
                                Ok(v) => out.emit(BatteryCommandOutput::ChargeLimitSet(v)),
                                Err(e) => {
                                    out.emit(BatteryCommandOutput::Error(e));
                                    return;
                                }
                            }
                            tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60))
                                .await;
                            out.emit(BatteryCommandOutput::TimerElapsed);
                        })
                        .drop_on_shutdown()
                });
            }
            BatteryMsg::ToggleDeepSleep(active) => {
                if active && !self.deep_sleep_supported {
                    let _ = sender.output(t!("battery_deep_sleep_not_supported").to_string());
                    return;
                }
                if active == self.deep_sleep_active {
                    return;
                }
                self.deep_sleep_active = active;
                AppConfig::update(|c| c.active_profile_mut().battery_deep_sleep_active = active);
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            let value = if active { "deep" } else { "s2idle" };
                            match pkexec_write_sysfs(SYS_MEM_SLEEP, value).await {
                                Ok(()) => out.emit(BatteryCommandOutput::DeepSleepSet(active)),
                                Err(e) => out.emit(BatteryCommandOutput::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            BatteryMsg::LoadProfile(active) => {
                if self.deep_sleep_active == active || !self.deep_sleep_supported {
                    return;
                }
                self.deep_sleep_active = active;
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            let value = if active { "deep" } else { "s2idle" };
                            match pkexec_write_sysfs(SYS_MEM_SLEEP, value).await {
                                Ok(()) => out.emit(BatteryCommandOutput::DeepSleepSet(active)),
                                Err(e) => out.emit(BatteryCommandOutput::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: BatteryCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            BatteryCommandOutput::AsusdChecked(available) => {
                self.asusd_available = available;
            }
            BatteryCommandOutput::InitValue(val) => {
                self.maintenance_mode_active = val != 100;
            }
            BatteryCommandOutput::InitDeepSleep(active) => {
                self.deep_sleep_active = active;
            }
            BatteryCommandOutput::DeepSleepSupported(supported) => {
                self.deep_sleep_supported = supported;
                if !supported {
                    self.deep_sleep_active = false;
                }
            }
            BatteryCommandOutput::DeepSleepSet(active) => {
                tracing::info!(
                    "{}",
                    t!(
                        "battery_deep_sleep_set",
                        value = if active && self.deep_sleep_supported {
                            "deep"
                        } else {
                            "s2idle"
                        }
                    )
                );
            }
            BatteryCommandOutput::ChargeLimitSet(val) => {
                tracing::info!(
                    "{}",
                    t!("battery_charge_limit_set", value = val.to_string())
                );
            }
            BatteryCommandOutput::TimerElapsed => {
                // One-shot 24h expired — revert to 80%.
                sender.command(|out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_charge_limit(80).await {
                                Ok(v) => out.emit(BatteryCommandOutput::ChargeLimitSet(v)),
                                Err(e) => out.emit(BatteryCommandOutput::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            BatteryCommandOutput::Error(e) => {
                let _ = sender.output(e);
            }
        }
    }
}
