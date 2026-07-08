// Utu - ASUS Laptop Control Centre for Ubuntu
// Copyright (C) 2026 Baraka Malila — GPL-3.0-or-later

use gtk4 as gtk;
use gtk4::prelude::*;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::dbus;

pub struct ChargeLimit {
    limit: u8,
    asusd_available: bool,
    slider: gtk::Scale,
    value_label: gtk::Label,
}

#[derive(Debug)]
pub enum ChargeLimitMsg {
    SliderChanged(u8),
    Commit,
}

#[derive(Debug)]
pub enum ChargeLimitCmd {
    Loaded(u8),
    Set(u8),
    Error(String),
}

impl ChargeLimit {
    pub fn clamp_limit(v: u8) -> u8 {
        v.clamp(20, 100)
    }
}

#[relm4::component(pub)]
impl Component for ChargeLimit {
    type Init = ();
    type Input = ChargeLimitMsg;
    type Output = String;
    type CommandOutput = ChargeLimitCmd;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("battery_page_charge_title"),
            set_description: Some(&t!("battery_page_charge_subtitle")),

            add = &adw::ActionRow {
                set_title: "",

                add_suffix = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,
                    set_hexpand: true,
                    set_valign: gtk::Align::Center,

                    append = &model.slider.clone(),
                    append = &model.value_label.clone(),
                },
            },
        }
    }

    fn init(_: (), _root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let slider = gtk::Scale::with_range(gtk::Orientation::Horizontal, 20.0, 100.0, 1.0);
        slider.set_hexpand(true);
        slider.set_value(80.0);
        slider.set_draw_value(false);

        let value_label = gtk::Label::new(Some("80%"));
        value_label.set_width_chars(4);
        value_label.set_xalign(1.0);
        value_label.add_css_class("dim-label");

        {
            let sender = sender.clone();
            let lbl = value_label.clone();
            slider.connect_value_changed(move |s| {
                let v = s.value() as u8;
                lbl.set_label(&format!("{}%", v));
                sender.input(ChargeLimitMsg::SliderChanged(v));
            });
        }
        // Commit to daemon when the user releases the slider.
        {
            let sender = sender.clone();
            slider.connect_change_value(move |_, _, _| {
                sender.input(ChargeLimitMsg::Commit);
                gtk4::glib::Propagation::Proceed
            });
        }

        let model = ChargeLimit {
            limit: 80,
            asusd_available: false,
            slider,
            value_label,
        };
        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    if !dbus::check_asusd_available().await {
                        return;
                    }
                    match dbus::get_charge_limit().await {
                        Ok(v) => out.emit(ChargeLimitCmd::Loaded(v)),
                        Err(e) => out.emit(ChargeLimitCmd::Error(e)),
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: ChargeLimitMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            ChargeLimitMsg::SliderChanged(v) => {
                self.limit = Self::clamp_limit(v);
            }
            ChargeLimitMsg::Commit => {
                if !self.asusd_available {
                    return;
                }
                let limit = self.limit;
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_charge_limit(limit).await {
                                Ok(v) => out.emit(ChargeLimitCmd::Set(v)),
                                Err(e) => out.emit(ChargeLimitCmd::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: ChargeLimitCmd,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ChargeLimitCmd::Loaded(v) | ChargeLimitCmd::Set(v) => {
                self.asusd_available = true;
                self.limit = v;
                self.slider.set_value(v as f64);
                self.value_label.set_label(&format!("{}%", v));
            }
            ChargeLimitCmd::Error(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clamp_to_range() {
        assert_eq!(ChargeLimit::clamp_limit(15), 20);
        assert_eq!(ChargeLimit::clamp_limit(50), 50);
        assert_eq!(ChargeLimit::clamp_limit(110), 100);
    }
}
