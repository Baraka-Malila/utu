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

use gtk4 as gtk;
use gtk4::prelude::*;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::config::AppConfig;
use crate::services::dbus;
use crate::services::dbus::GfxMode;

pub struct GpuModel {
    supergfxctl_available: bool,
    current_mode: GfxMode,
    display_modes: Vec<GfxMode>,
}

#[derive(Debug)]
pub enum GpuMsg {
    ChangeMode(GfxMode),
    LoadProfile(u32),
}

#[derive(Debug)]
pub enum GpuCommandOutput {
    SupergfxctlChecked(bool),
    InitModeAndSupported(GfxMode, Vec<GfxMode>),
    ModeSet(GfxMode),
    Error(String),
}

// Static metadata for the three modes we surface in the UI.
// NvidiaNoModeset is labeled "Dedicated" — it's the closest to a discrete-only mode.
const MODE_META: &[(GfxMode, &str, &str, &str)] = &[
    (GfxMode::Integrated,      "computer-symbolic",     "Integrated", "iGPU only · Best battery"),
    (GfxMode::Hybrid,          "view-refresh-symbolic", "Hybrid",     "Auto-switch · Balanced"),
    (GfxMode::NvidiaNoModeset, "input-gaming-symbolic", "Dedicated",  "dGPU always · Max perf"),
];

fn gpu_mode_card(
    icon: &str,
    name: &str,
    desc: &str,
    active: bool,
    sender: ComponentSender<GpuModel>,
    mode: GfxMode,
) -> gtk::Button {
    let icon_w = gtk::Image::from_icon_name(icon);
    icon_w.set_pixel_size(32);

    let name_l = gtk::Label::new(Some(name));
    name_l.add_css_class("body");
    name_l.set_halign(gtk::Align::Center);

    let desc_l = gtk::Label::new(Some(desc));
    desc_l.add_css_class("caption");
    desc_l.add_css_class("dim-label");
    desc_l.set_wrap(true);
    desc_l.set_halign(gtk::Align::Center);
    desc_l.set_max_width_chars(14);

    let inner = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(12)
        .margin_end(12)
        .halign(gtk::Align::Center)
        .build();
    inner.append(&icon_w);
    inner.append(&name_l);
    inner.append(&desc_l);

    let btn = gtk::Button::new();
    btn.set_child(Some(&inner));
    btn.add_css_class("flat");
    btn.add_css_class("mode-card");
    if active {
        btn.add_css_class("mode-card-active");
    }
    btn.connect_clicked(move |_| {
        sender.input(GpuMsg::ChangeMode(mode));
    });
    btn
}

#[relm4::component(pub)]
impl Component for GpuModel {
    type Init = ();
    type Input = GpuMsg;
    type Output = String;
    type CommandOutput = GpuCommandOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let saved_mode = GfxMode::from(AppConfig::load().active_profile().gpu_mode);

        let model = GpuModel {
            supergfxctl_available: false,
            current_mode: saved_mode,
            display_modes: Vec::new(),
        };

        let widgets = view_output!();
        model.rebuild_cards(&root, &sender);

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let available = dbus::check_supergfxctl_available().await;
                    out.emit(GpuCommandOutput::SupergfxctlChecked(available));

                    if !available {
                        return;
                    }

                    let current = match dbus::get_gpu_mode().await {
                        Ok(m) => m,
                        Err(e) => {
                            out.emit(GpuCommandOutput::Error(e));
                            return;
                        }
                    };
                    let supported = match dbus::get_supported_gpu_modes().await {
                        Ok(v) => v,
                        Err(e) => {
                            out.emit(GpuCommandOutput::Error(e));
                            return;
                        }
                    };
                    out.emit(GpuCommandOutput::InitModeAndSupported(current, supported));
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: GpuMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            GpuMsg::LoadProfile(mode_u32) => {
                if !self.supergfxctl_available {
                    return;
                }
                let mode = GfxMode::from(mode_u32);
                self.current_mode = mode;
                self.rebuild_cards(root, &sender);
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_gpu_mode(mode).await {
                                Ok(m) => out.emit(GpuCommandOutput::ModeSet(m)),
                                Err(e) => out.emit(GpuCommandOutput::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            GpuMsg::ChangeMode(mode) => {
                if mode == self.current_mode {
                    return;
                }
                self.current_mode = mode;
                AppConfig::update(|c| c.active_profile_mut().gpu_mode = mode as u32);
                self.rebuild_cards(root, &sender);
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_gpu_mode(mode).await {
                                Ok(m) => out.emit(GpuCommandOutput::ModeSet(m)),
                                Err(e) => out.emit(GpuCommandOutput::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: GpuCommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            GpuCommandOutput::SupergfxctlChecked(available) => {
                self.supergfxctl_available = available;
            }
            GpuCommandOutput::InitModeAndSupported(current, supported) => {
                let mut modes = vec![current];
                for m in supported {
                    if !modes.contains(&m) {
                        modes.push(m);
                    }
                }
                self.display_modes = modes;
                self.current_mode = current;
                self.rebuild_cards(root, &sender);
            }
            GpuCommandOutput::ModeSet(mode) => {
                tracing::info!(
                    "{}",
                    t!("gpu_mode_set", mode = t!(mode.i18n_key()).to_string())
                );
                let dialog = adw::AlertDialog::builder()
                    .heading(&*t!("gpu_restart_title"))
                    .body(&*t!("gpu_restart_body"))
                    .build();
                dialog.add_response("later", &t!("gpu_restart_later"));
                dialog.add_response("now", &t!("gpu_restart_now"));
                dialog.set_response_appearance("now", adw::ResponseAppearance::Suggested);
                dialog.set_default_response(Some("later"));
                dialog.set_close_response("later");
                dialog.connect_response(None, |_, response| {
                    if response == "now" {
                        let _ = std::process::Command::new("pkexec")
                            .args(["systemctl", "restart", "gdm3"])
                            .spawn();
                    }
                });
                // Present on the active window — GTK finds it via the default display.
                dialog.present(None::<&gtk::Widget>);
            }
            GpuCommandOutput::Error(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

impl GpuModel {
    fn rebuild_cards(&self, root: &gtk::Box, sender: &ComponentSender<Self>) {
        while let Some(c) = root.last_child() {
            root.remove(&c);
        }

        if !self.supergfxctl_available && self.display_modes.is_empty() {
            let warning = gtk::Label::new(Some(&t!("supergfxctl_missing_warning")));
            warning.add_css_class("error");
            warning.set_wrap(true);
            warning.set_xalign(0.0);
            root.append(&warning);
            return;
        }

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.set_homogeneous(true);

        for (mode, icon, name, desc) in MODE_META {
            // Only show modes that the daemon reports as supported, or current mode.
            // When display_modes is empty (not yet loaded), show all three.
            let show = self.display_modes.is_empty()
                || self.display_modes.contains(mode)
                || *mode == self.current_mode;
            if !show {
                continue;
            }
            let active = *mode == self.current_mode;
            let card = gpu_mode_card(icon, name, desc, active, sender.clone(), *mode);
            row.append(&card);
        }
        root.append(&row);

        if !self.display_modes.is_empty() {
            let note = gtk::Label::new(Some(&t!("gpu_reboot_warning")));
            note.add_css_class("dim-label");
            note.add_css_class("caption");
            note.set_wrap(true);
            note.set_xalign(0.0);
            root.append(&note);
        }
    }
}
