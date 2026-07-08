// Utu - ASUS Laptop Control Centre for Ubuntu
// Copyright (C) 2026 Guido Philipp, Baraka Malila — GPL-3.0-or-later

use gtk4 as gtk;
use gtk4::prelude::*;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::config::AppConfig;

/// Controls when the keyboard backlight idle-timeout is active.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) enum TimeoutMode {
    #[default]
    Never,
    /// Timeout on both AC and battery.
    BatteryAndAc,
    /// Timeout only when running on battery.
    BatteryOnly,
}

impl From<u32> for TimeoutMode {
    fn from(v: u32) -> Self {
        match v {
            1 => Self::BatteryAndAc,
            2 => Self::BatteryOnly,
            _ => Self::Never,
        }
    }
}

pub struct BacklightIdleModel {
    timeout_mode: TimeoutMode,
    cards_box: gtk::Box,
}

#[derive(Debug)]
pub enum BacklightIdleMsg {
    ChangeMode(TimeoutMode),
    /// Mode index from config (u32 for backward compat with app.rs).
    LoadProfile(u32),
    /// Ambient settings changed — reserved for future swayidle integration.
    AmbientChanged,
}

#[derive(Debug)]
pub enum BacklightIdleCommandOutput {
    Error(String),
}

fn idle_mode_card(
    icon: &str,
    name: &str,
    desc: &str,
    active: bool,
    sender: ComponentSender<BacklightIdleModel>,
    mode: TimeoutMode,
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
        sender.input(BacklightIdleMsg::ChangeMode(mode));
    });
    btn
}

#[relm4::component(pub)]
impl Component for BacklightIdleModel {
    type Init = ();
    type Input = BacklightIdleMsg;
    type Output = String;
    type CommandOutput = BacklightIdleCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("sleep_group_title"),
            set_description: Some(&t!("sleep_group_desc")),

            add = &model.cards_box.clone(),
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let timeout_mode =
            TimeoutMode::from(AppConfig::load().active_profile().kbd_timeout_mode);

        let cards_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .homogeneous(true)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let model = BacklightIdleModel {
            timeout_mode,
            cards_box,
        };
        let widgets = view_output!();
        model.rebuild_cards(&sender);
        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        msg: BacklightIdleMsg,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            BacklightIdleMsg::ChangeMode(mode) => {
                if mode == self.timeout_mode {
                    return;
                }
                self.timeout_mode = mode;
                AppConfig::update(|c| c.active_profile_mut().kbd_timeout_mode = mode as u32);
                self.rebuild_cards(&sender);
            }
            BacklightIdleMsg::LoadProfile(mode_u32) => {
                let mode = TimeoutMode::from(mode_u32);
                if mode == self.timeout_mode {
                    return;
                }
                self.timeout_mode = mode;
                self.rebuild_cards(&sender);
            }
            BacklightIdleMsg::AmbientChanged => {}
        }
    }

    fn update_cmd(
        &mut self,
        msg: BacklightIdleCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            BacklightIdleCommandOutput::Error(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

impl BacklightIdleModel {
    fn rebuild_cards(&self, sender: &ComponentSender<Self>) {
        while let Some(child) = self.cards_box.first_child() {
            self.cards_box.remove(&child);
        }

        let modes: &[(&str, &str, &str, TimeoutMode)] = &[
            (
                "display-brightness-symbolic",
                &t!("sleep_mode_never_title"),
                &t!("keyboard_idle_never_desc"),
                TimeoutMode::Never,
            ),
            (
                "preferences-system-time-symbolic",
                &t!("sleep_mode_always_title"),
                &t!("keyboard_idle_always_desc"),
                TimeoutMode::BatteryAndAc,
            ),
            (
                "battery-symbolic",
                &t!("sleep_mode_battery_title"),
                &t!("keyboard_idle_battery_desc"),
                TimeoutMode::BatteryOnly,
            ),
        ];

        for &(icon, name, desc, mode) in modes {
            let card =
                idle_mode_card(icon, name, desc, self.timeout_mode == mode, sender.clone(), mode);
            self.cards_box.append(&card);
        }
    }
}
