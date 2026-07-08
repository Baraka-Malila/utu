// Utu - ASUS Laptop Control Centre for Ubuntu
// Copyright (C) 2026 Baraka Malila — GPL-3.0-or-later

use gtk4 as gtk;
use gtk4::prelude::*;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::dbus;

pub struct AboutModel {
    asusd_running: bool,
    supergfxd_running: bool,
    asusd_dot: gtk::Label,
    supergfxd_dot: gtk::Label,
}

#[derive(Debug)]
pub enum AboutMsg {}

#[derive(Debug)]
pub enum AboutCmd {
    DaemonStatus { asusd: bool, supergfxd: bool },
}

fn make_dot(running: bool) -> gtk::Label {
    let lbl = gtk::Label::new(Some(if running { "●" } else { "○" }));
    if running {
        lbl.add_css_class("success");
    } else {
        lbl.add_css_class("error");
    }
    lbl
}

fn update_dot(dot: &gtk::Label, running: bool) {
    dot.set_text(if running { "●" } else { "○" });
    if running {
        dot.remove_css_class("error");
        dot.add_css_class("success");
    } else {
        dot.remove_css_class("success");
        dot.add_css_class("error");
    }
}

#[relm4::component(pub)]
impl Component for AboutModel {
    type Init = ();
    type Input = AboutMsg;
    type Output = String;
    type CommandOutput = AboutCmd;

    view! {
        adw::PreferencesPage {
            set_title: &t!("tab_about"),

            add = &adw::PreferencesGroup {
                add = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 24,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_vexpand: true,
                    set_margin_top: 48,
                    set_margin_bottom: 48,

                    append = &gtk::Image {
                        set_icon_name: Some("preferences-system-symbolic"),
                        set_pixel_size: 96,
                        add_css_class: "dim-label",
                    },

                    append = &gtk::Label {
                        set_label: &t!("about_version", version = env!("CARGO_PKG_VERSION")),
                        add_css_class: "title-1",
                    },

                    append = &gtk::Label {
                        set_label: &t!("about_built_on"),
                        add_css_class: "dim-label",
                    },

                    append = &gtk::Button {
                        set_label: "GitHub",
                        add_css_class: "pill",
                        set_halign: gtk::Align::Center,
                        connect_clicked => |_| {
                            let _ = std::process::Command::new("xdg-open")
                                .arg("https://github.com/Baraka-Malila/utu")
                                .spawn();
                        },
                    },

                    append = &gtk::Label {
                        set_label: &t!("about_license"),
                        add_css_class: "dim-label",
                        add_css_class: "caption",
                    },

                    append = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 24,
                        set_halign: gtk::Align::Center,
                        set_margin_top: 16,

                        append = &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 8,
                            append = &gtk::Label { set_label: "asusd" },
                            append = &model.asusd_dot.clone(),
                        },

                        append = &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 8,
                            append = &gtk::Label { set_label: "supergfxd" },
                            append = &model.supergfxd_dot.clone(),
                        },
                    },
                },
            },
        }
    }

    fn init(_: (), _root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = AboutModel {
            asusd_running: false,
            supergfxd_running: false,
            asusd_dot: make_dot(false),
            supergfxd_dot: make_dot(false),
        };
        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let asusd = dbus::check_asusd_available().await;
                    let supergfxd = dbus::check_supergfxctl_available().await;
                    out.emit(AboutCmd::DaemonStatus { asusd, supergfxd });
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _msg: AboutMsg, _sender: ComponentSender<Self>, _root: &Self::Root) {}

    fn update_cmd(
        &mut self,
        msg: AboutCmd,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        let AboutCmd::DaemonStatus { asusd, supergfxd } = msg;
        self.asusd_running = asusd;
        self.supergfxd_running = supergfxd;
        update_dot(&self.asusd_dot, asusd);
        update_dot(&self.supergfxd_dot, supergfxd);
    }
}
