// Utu - ASUS Laptop Control Centre for Ubuntu
// Copyright (C) 2026 Baraka Malila — GPL-3.0-or-later
// 6-card horizontal strip: Quiet · Balanced · Performance | Integrated · Hybrid · Dedicated

use gtk4 as gtk;
use gtk4::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::config::AppConfig;
use crate::services::dbus::{self, FanProfile, GfxMode};

pub struct ModeStripModel {
    active_thermal: FanProfile,
    active_gpu: GfxMode,
}

#[derive(Debug)]
pub enum ModeStripMsg {
    SetThermal(FanProfile),
    SetGpu(GfxMode),
}

#[derive(Debug)]
pub enum ModeStripCmd {
    Init { thermal: FanProfile, gpu: GfxMode },
    Error(String),
}

fn strip_card(
    icon: &str,
    label: &str,
    active: bool,
    sender_fn: impl Fn() + 'static,
) -> gtk::Button {
    let img = gtk::Image::from_icon_name(icon);
    img.set_pixel_size(24);
    let lbl = gtk::Label::new(Some(label));
    lbl.add_css_class("caption");
    let inner = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(8)
        .margin_end(8)
        .halign(gtk::Align::Center)
        .build();
    inner.append(&img);
    inner.append(&lbl);

    let btn = gtk::Button::new();
    btn.set_child(Some(&inner));
    btn.add_css_class("home-mode-card");
    if active {
        btn.add_css_class("home-mode-card-active");
    }
    btn.connect_clicked(move |_| sender_fn());
    btn
}

#[relm4::component(pub)]
impl Component for ModeStripModel {
    type Init = ();
    type Input = ModeStripMsg;
    type Output = ();
    type CommandOutput = ModeStripCmd;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,

            append = &gtk::Label {
                set_label: &t!("home_operating_mode"),
                set_halign: gtk::Align::Start,
                add_css_class: "title-2",
            },
        }
    }

    fn init(_: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let config_gpu = crate::services::config::AppConfig::load().active_profile().gpu_mode;
        let model = ModeStripModel {
            active_thermal: FanProfile::Balanced,
            active_gpu: GfxMode::from(config_gpu),
        };
        let widgets = view_output!();

        build_strip(&root, &model, &sender);

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let thermal =
                        dbus::get_fan_profile().await.unwrap_or(FanProfile::Balanced);
                    let gpu = dbus::get_gpu_mode().await.unwrap_or(GfxMode::Hybrid);
                    out.emit(ModeStripCmd::Init { thermal, gpu });
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        msg: ModeStripMsg,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            ModeStripMsg::SetThermal(p) => {
                self.active_thermal = p;
                AppConfig::update(|c| c.active_profile_mut().fan_profile = p as u32);
                rebuild_strip(root, self, &sender);
                sender.command(move |_, shutdown| {
                    shutdown
                        .register(async move {
                            let _ = dbus::set_fan_profile(p).await;
                        })
                        .drop_on_shutdown()
                });
            }
            ModeStripMsg::SetGpu(m) => {
                self.active_gpu = m;
                AppConfig::update(|c| c.active_profile_mut().gpu_mode = m as u32);
                rebuild_strip(root, self, &sender);
                sender.command(move |_, shutdown| {
                    shutdown
                        .register(async move {
                            let _ = dbus::set_gpu_mode(m).await;
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: ModeStripCmd,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            ModeStripCmd::Init { thermal, gpu } => {
                self.active_thermal = thermal;
                self.active_gpu = gpu;
                rebuild_strip(root, self, &sender);
            }
            ModeStripCmd::Error(e) => {
                tracing::warn!("ModeStrip: {}", e);
            }
        }
    }
}

fn rebuild_strip(
    root: &gtk::Box,
    model: &ModeStripModel,
    sender: &ComponentSender<ModeStripModel>,
) {
    while let Some(c) = root.last_child() {
        if c.downcast_ref::<gtk::Label>().is_some() {
            break;
        }
        root.remove(&c);
    }
    build_strip(root, model, sender);
}

fn build_strip(
    root: &gtk::Box,
    model: &ModeStripModel,
    sender: &ComponentSender<ModeStripModel>,
) {
    let strip = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    strip.set_homogeneous(true);

    let thermal_cards = [
        (
            FanProfile::Quiet,
            "power-profile-power-saver-symbolic",
            "Quiet",
        ),
        (
            FanProfile::Balanced,
            "power-profile-balanced-symbolic",
            "Balanced",
        ),
        (
            FanProfile::Performance,
            "power-profile-performance-symbolic",
            "Performance",
        ),
    ];
    for (p, icon, label) in &thermal_cards {
        let s = sender.clone();
        let pv = *p;
        strip.append(&strip_card(
            icon,
            label,
            *p == model.active_thermal,
            move || s.input(ModeStripMsg::SetThermal(pv)),
        ));
    }

    let sep = gtk::Separator::new(gtk::Orientation::Vertical);
    sep.set_margin_top(8);
    sep.set_margin_bottom(8);
    strip.append(&sep);

    let gpu_cards = [
        (GfxMode::Integrated, "computer-symbolic", "Integrated"),
        (GfxMode::Hybrid, "view-refresh-symbolic", "Hybrid"),
        (GfxMode::AsusMuxDiscreet, "input-gaming-symbolic", "Dedicated"),
    ];
    for (m, icon, label) in &gpu_cards {
        let s = sender.clone();
        let mv = *m;
        strip.append(&strip_card(
            icon,
            label,
            *m == model.active_gpu,
            move || s.input(ModeStripMsg::SetGpu(mv)),
        ));
    }

    root.append(&strip);
}
