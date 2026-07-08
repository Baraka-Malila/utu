// Utu - ASUS Laptop Control Centre for Ubuntu
// Copyright (C) 2026 Baraka Malila — GPL-3.0-or-later

use gtk4 as gtk;
use gtk4::prelude::*;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::config::AppConfig;
use crate::services::theme;

pub const ACCENT_PRESETS: &[(&str, &str, &str)] = &[
    ("appearance_amber", "#e8a800", "amber"),
    ("appearance_crimson", "#e8002d", "crimson"),
    ("appearance_teal", "#00bcd4", "teal"),
];

pub struct AppearanceModel {
    active_accent: String,
    swatches_box: gtk::Box,
}

#[derive(Debug)]
pub enum AppearanceMsg {
    SetAccent(String),
}

fn swatch_button(
    hex: &str,
    label_key: &str,
    active: bool,
    sender: ComponentSender<AppearanceModel>,
) -> gtk::Button {
    let btn = gtk::Button::new();
    btn.set_tooltip_text(Some(&t!(label_key)));
    btn.add_css_class("accent-swatch");
    if active {
        btn.add_css_class("accent-swatch-active");
    }

    let provider = gtk4::CssProvider::new();
    provider.load_from_string(&format!(
        "button.accent-swatch {{ background-color: {hex}; min-width: 48px; min-height: 48px; \
         border-radius: 50%; padding: 0; }}"
    ));
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 2,
        );
    }
    btn.add_css_class("mode-card");
    btn.add_css_class("flat");

    let hex_owned = hex.to_string();
    btn.connect_clicked(move |_| {
        sender.input(AppearanceMsg::SetAccent(hex_owned.clone()));
    });
    btn
}

#[relm4::component(pub)]
impl Component for AppearanceModel {
    type Init = ();
    type Input = AppearanceMsg;
    type Output = String;
    type CommandOutput = ();

    view! {
        adw::PreferencesPage {
            set_title: &t!("appearance_page_title"),

            add = &adw::PreferencesGroup {
                set_title: &t!("appearance_accent_title"),

                add = &model.swatches_box.clone(),
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let active_accent = AppConfig::load().accent_hex.clone();

        let swatches_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(16)
            .margin_top(12)
            .margin_bottom(12)
            .halign(gtk::Align::Center)
            .build();

        let model = AppearanceModel {
            active_accent: active_accent.clone(),
            swatches_box,
        };
        let widgets = view_output!();
        model.rebuild_swatches(&sender);
        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        msg: AppearanceMsg,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AppearanceMsg::SetAccent(hex) => {
                if hex == self.active_accent {
                    return;
                }
                self.active_accent = hex.clone();
                AppConfig::update(|c| c.accent_hex = hex.clone());
                theme::apply_accent(&hex);
                self.rebuild_swatches(&sender);
            }
        }
    }

    fn update_cmd(
        &mut self,
        _msg: (),
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
    }
}

impl AppearanceModel {
    fn rebuild_swatches(&self, sender: &ComponentSender<Self>) {
        while let Some(child) = self.swatches_box.first_child() {
            self.swatches_box.remove(&child);
        }
        for &(key, hex, _id) in ACCENT_PRESETS {
            let active = hex == self.active_accent.as_str();
            let btn = swatch_button(hex, key, active, sender.clone());
            self.swatches_box.append(&btn);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accent_names_are_non_empty() {
        for (name, hex, _) in ACCENT_PRESETS {
            assert!(!name.is_empty());
            assert!(hex.starts_with('#'));
        }
    }
}
