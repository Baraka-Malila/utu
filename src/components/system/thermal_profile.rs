// Utu - ASUS Laptop Control Centre for Ubuntu
// Copyright (C) 2026 Baraka Malila — GPL-3.0-or-later

use gtk4 as gtk;
use gtk4::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::config::AppConfig;
use crate::services::dbus::{self, FanProfile};

pub struct ThermalProfileModel {
    pub active: FanProfile,
    pub asusd_available: bool,
}

#[derive(Debug)]
pub enum ThermalProfileMsg {
    Select(FanProfile),
    LoadProfile(FanProfile),
}

#[derive(Debug)]
pub enum ThermalProfileCmd {
    AsusdChecked(bool),
    ProfileSet(FanProfile),
    Error(String),
}

impl ThermalProfileModel {
    pub fn apply_profile(&mut self, p: FanProfile) {
        self.active = p;
    }
}

fn mode_card(
    icon: &str,
    name: &str,
    desc: &str,
    active: bool,
    sender: ComponentSender<ThermalProfileModel>,
    profile: FanProfile,
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
        sender.input(ThermalProfileMsg::Select(profile));
    });
    btn
}

#[relm4::component(pub)]
impl Component for ThermalProfileModel {
    type Init = ();
    type Input = ThermalProfileMsg;
    type Output = String;
    type CommandOutput = ThermalProfileCmd;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,
        }
    }

    fn init(_: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = ThermalProfileModel {
            active: FanProfile::from(AppConfig::load().active_profile().fan_profile),
            asusd_available: false,
        };

        let widgets = view_output!();
        model.rebuild_cards(&root, &sender);

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let avail = dbus::check_asusd_available().await;
                    out.emit(ThermalProfileCmd::AsusdChecked(avail));
                    if !avail {
                        return;
                    }
                    match dbus::get_fan_profile().await {
                        Ok(p) => out.emit(ThermalProfileCmd::ProfileSet(p)),
                        Err(e) => out.emit(ThermalProfileCmd::Error(e)),
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: ThermalProfileMsg, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ThermalProfileMsg::Select(p) => {
                if p == self.active {
                    return;
                }
                self.active = p;
                AppConfig::update(|c| c.active_profile_mut().fan_profile = p as u32);
                self.rebuild_cards(root, &sender);
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_fan_profile(p).await {
                                Ok(actual) => out.emit(ThermalProfileCmd::ProfileSet(actual)),
                                Err(e) => out.emit(ThermalProfileCmd::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            ThermalProfileMsg::LoadProfile(p) => {
                self.active = p;
                self.rebuild_cards(root, &sender);
                if !self.asusd_available {
                    return;
                }
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_fan_profile(p).await {
                                Ok(actual) => out.emit(ThermalProfileCmd::ProfileSet(actual)),
                                Err(e) => out.emit(ThermalProfileCmd::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: ThermalProfileCmd,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            ThermalProfileCmd::AsusdChecked(a) => {
                self.asusd_available = a;
            }
            ThermalProfileCmd::ProfileSet(p) => {
                self.active = p;
                self.rebuild_cards(root, &sender);
            }
            ThermalProfileCmd::Error(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

impl ThermalProfileModel {
    fn rebuild_cards(&self, root: &gtk::Box, sender: &ComponentSender<Self>) {
        while let Some(c) = root.last_child() {
            root.remove(&c);
        }

        let card_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        card_row.set_homogeneous(true);

        let profiles = [
            (
                FanProfile::Quiet,
                "power-profile-power-saver-symbolic",
                t!("thermal_profile_quiet_name").to_string(),
                t!("thermal_profile_quiet_desc").to_string(),
            ),
            (
                FanProfile::Balanced,
                "power-profile-balanced-symbolic",
                t!("thermal_profile_balanced_name").to_string(),
                t!("thermal_profile_balanced_desc").to_string(),
            ),
            (
                FanProfile::Performance,
                "power-profile-performance-symbolic",
                t!("thermal_profile_performance_name").to_string(),
                t!("thermal_profile_performance_desc").to_string(),
            ),
        ];

        for (profile, icon, name, desc) in &profiles {
            let card = mode_card(
                icon,
                name,
                desc,
                *profile == self.active,
                sender.clone(),
                *profile,
            );
            card_row.append(&card);
        }

        root.append(&card_row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn apply_profile_updates_active() {
        let mut model = ThermalProfileModel {
            active: FanProfile::Balanced,
            asusd_available: true,
        };
        model.apply_profile(FanProfile::Quiet);
        assert_eq!(model.active, FanProfile::Quiet);
    }
}
