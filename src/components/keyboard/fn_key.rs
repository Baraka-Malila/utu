// Utu - ASUS Laptop Control Centre for Ubuntu
// Copyright (C) 2026 Guido Philipp, Baraka Malila — GPL-3.0-or-later

use gtk4 as gtk;
use gtk4::prelude::*;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::commands::{run_command_blocking, which_exists};
use crate::services::config::AppConfig;

fn build_grub_script(locked: bool) -> String {
    let value = if locked { "0" } else { "1" };
    format!(
        r#"sed -i 's/ asus_wmi\.fnlock_default=[^ "]*//g' /etc/default/grub && \
sed -i 's/\(GRUB_CMDLINE_LINUX_DEFAULT="[^"]*\)"/\1 asus_wmi.fnlock_default={}"/g' /etc/default/grub && \
update-grub"#,
        value
    )
}

fn fn_mode_card(
    icon: &str,
    name: &str,
    desc: &str,
    active: bool,
    sensitive: bool,
    sender: ComponentSender<FnKeyModel>,
    locked: bool,
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
    btn.set_sensitive(sensitive);
    if active {
        btn.add_css_class("mode-card-active");
    }
    btn.connect_clicked(move |_| {
        sender.input(FnKeyMsg::ToggleLocked(locked));
    });
    btn
}

pub struct FnKeyModel {
    locked: bool,
    update_grub_available: bool,
    cards_box: gtk::Box,
    status_label: gtk::Label,
}

#[derive(Debug)]
pub enum FnKeyMsg {
    ToggleLocked(bool),
    LoadProfile(bool),
}

#[derive(Debug)]
pub enum FnKeyCommandOutput {
    UpdateGrubChecked(bool),
    Set(bool),
    Error(String),
}

#[relm4::component(pub)]
impl Component for FnKeyModel {
    type Init = ();
    type Input = FnKeyMsg;
    type Output = String;
    type CommandOutput = FnKeyCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("fn_key_group_title"),
            set_description: Some(&t!("fn_key_group_desc")),

            #[template]
            add = &crate::components::widgets::DaemonWarningLabel {
                #[watch]
                set_visible: !model.update_grub_available,
                set_label: &t!("fn_key_update_grub_missing_warning"),
            },

            add = &model.cards_box.clone(),
            add = &model.status_label.clone(),
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let locked = AppConfig::load().active_profile().input_fn_key_locked;

        let cards_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .homogeneous(true)
            .margin_top(8)
            .margin_bottom(4)
            .build();

        let status_label = gtk::Label::new(Some(&t!("fn_key_hint_subtitle")));
        status_label.add_css_class("caption");
        status_label.add_css_class("dim-label");
        status_label.set_halign(gtk::Align::Start);
        status_label.set_margin_bottom(8);
        status_label.set_margin_start(4);

        let model = FnKeyModel {
            locked,
            update_grub_available: false,
            cards_box,
            status_label,
        };
        let widgets = view_output!();

        model.rebuild_cards(&sender);

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let ok = which_exists("update-grub").await;
                    out.send(FnKeyCommandOutput::UpdateGrubChecked(ok)).ok();
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: FnKeyMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            FnKeyMsg::ToggleLocked(locked) => {
                if locked == self.locked {
                    return;
                }
                self.locked = locked;
                self.rebuild_cards(&sender);

                let script = build_grub_script(locked);
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match run_command_blocking("pkexec", &["bash", "-c", &script]).await {
                                Ok(()) => out.emit(FnKeyCommandOutput::Set(locked)),
                                Err(e) => out.emit(FnKeyCommandOutput::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            FnKeyMsg::LoadProfile(locked) => {
                if locked == self.locked {
                    return;
                }
                self.locked = locked;
                self.rebuild_cards(&sender);

                let script = build_grub_script(locked);
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match run_command_blocking("pkexec", &["bash", "-c", &script]).await {
                                Ok(()) => out.emit(FnKeyCommandOutput::Set(locked)),
                                Err(e) => out.emit(FnKeyCommandOutput::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: FnKeyCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            FnKeyCommandOutput::UpdateGrubChecked(ok) => {
                self.update_grub_available = ok;
                self.rebuild_cards(&sender);
            }
            FnKeyCommandOutput::Set(locked) => {
                AppConfig::update(|c| c.active_profile_mut().input_fn_key_locked = locked);
                let mode = if locked {
                    t!("fn_key_mode_locked")
                } else {
                    t!("fn_key_mode_normal")
                };
                self.status_label
                    .set_label(&t!("fn_key_saved", mode = mode));
            }
            FnKeyCommandOutput::Error(e) => {
                self.status_label
                    .set_label(&t!("fn_key_save_error", error = e.clone()));
                let _ = sender.output(e);
            }
        }
    }
}

impl FnKeyModel {
    fn rebuild_cards(&self, sender: &ComponentSender<Self>) {
        while let Some(child) = self.cards_box.first_child() {
            self.cards_box.remove(&child);
        }

        let avail = self.update_grub_available;
        let locked = self.locked;

        let locked_card = fn_mode_card(
            "input-keyboard-symbolic",
            &t!("fn_key_locked_title"),
            &t!("fn_key_locked_desc"),
            locked,
            avail,
            sender.clone(),
            true,
        );
        let normal_card = fn_mode_card(
            "go-home-symbolic",
            &t!("fn_key_normal_title"),
            &t!("fn_key_normal_desc"),
            !locked,
            avail,
            sender.clone(),
            false,
        );

        self.cards_box.append(&locked_card);
        self.cards_box.append(&normal_card);
    }
}
