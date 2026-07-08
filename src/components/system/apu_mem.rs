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
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::config::AppConfig;
use crate::services::dbus;

fn label_for_value(v: i32) -> String {
    if v == 0 {
        t!("apu_mem_value_auto").to_string()
    } else {
        t!("apu_mem_value_gb", size = v).to_string()
    }
}

fn pill_strip(options: &[&str]) -> (gtk::Box, Vec<gtk::ToggleButton>) {
    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    hbox.add_css_class("pill-strip");
    hbox.add_css_class("linked");
    let mut btns: Vec<gtk::ToggleButton> = Vec::new();
    for &opt in options {
        let btn = gtk::ToggleButton::with_label(opt);
        hbox.append(&btn);
        btns.push(btn);
    }
    // Radio-group: deactivate all others when one is toggled on.
    for i in 0..btns.len() {
        let others: Vec<gtk::ToggleButton> = btns
            .iter()
            .enumerate()
            .filter(|&(j, _)| j != i)
            .map(|(_, b)| b.clone())
            .collect();
        btns[i].connect_toggled(move |b| {
            if b.is_active() {
                for o in &others {
                    o.set_active(false);
                }
            }
        });
    }
    (hbox, btns)
}

pub struct ApuMemModel {
    available: bool,
    current_value: i32,
    display_options: Vec<i32>,
    pill_btns: Vec<gtk::ToggleButton>,
}

#[derive(Debug)]
pub enum ApuMemMsg {
    ChangeValue(usize),
    LoadProfile(i32),
}

#[derive(Debug)]
pub enum ApuMemCommandOutput {
    NotAvailable,
    Init(Vec<i32>, i32),
    ValueSet(i32),
    Error(String),
}

#[relm4::component(pub)]
impl Component for ApuMemModel {
    type Init = ();
    type Input = ApuMemMsg;
    type Output = String;
    type CommandOutput = ApuMemCommandOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,

            append = &gtk::Label {
                set_label: &t!("apu_mem_group_title"),
                set_halign: gtk::Align::Start,
                add_css_class: "body",
            },

            append = &gtk::Label {
                set_label: &t!("apu_mem_subtitle"),
                set_halign: gtk::Align::Start,
                add_css_class: "caption",
                add_css_class: "dim-label",
                set_wrap: true,
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let saved_value = AppConfig::load().active_profile().apu_mem;

        // Build pill strip with a placeholder label until daemon responds.
        let labels = [label_for_value(saved_value)];
        let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        let (strip, pill_btns) = pill_strip(&label_refs);
        if let Some(btn) = pill_btns.first() {
            btn.set_active(true);
        }
        // Wire each button to emit ChangeValue(index).
        for (i, btn) in pill_btns.iter().enumerate() {
            let sender = sender.clone();
            btn.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(ApuMemMsg::ChangeValue(i));
                }
            });
        }
        strip.set_sensitive(false);
        root.append(&strip);

        let model = ApuMemModel {
            available: false,
            current_value: saved_value,
            display_options: Vec::new(),
            pill_btns,
        };

        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let options = match dbus::get_apu_mem_options().await {
                        Ok(v) if !v.is_empty() => v,
                        _ => {
                            out.emit(ApuMemCommandOutput::NotAvailable);
                            return;
                        }
                    };
                    let current = match dbus::get_apu_mem().await {
                        Ok(v) => v,
                        Err(_) => {
                            out.emit(ApuMemCommandOutput::NotAvailable);
                            return;
                        }
                    };
                    out.emit(ApuMemCommandOutput::Init(options, current));
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: ApuMemMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            ApuMemMsg::LoadProfile(value) => {
                if !self.available {
                    return;
                }
                self.current_value = value;
                self.sync_pill_selection(value);
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_apu_mem(value).await {
                                Ok(v) => out.emit(ApuMemCommandOutput::ValueSet(v)),
                                Err(e) => out.emit(ApuMemCommandOutput::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            ApuMemMsg::ChangeValue(idx) => {
                let Some(&value) = self.display_options.get(idx) else {
                    return;
                };
                if value == self.current_value {
                    return;
                }
                self.current_value = value;
                AppConfig::update(|c| c.active_profile_mut().apu_mem = value);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_apu_mem(value).await {
                                Ok(v) => out.emit(ApuMemCommandOutput::ValueSet(v)),
                                Err(e) => out.emit(ApuMemCommandOutput::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: ApuMemCommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            ApuMemCommandOutput::NotAvailable => {
                // Keep strip disabled and visible — shows "not supported on this device"
            }
            ApuMemCommandOutput::Init(options, current) => {
                self.available = true;
                self.display_options = options;
                self.current_value = current;

                // Rebuild pill strip with the real options.
                self.rebuild_strip(root, &sender);
            }
            ApuMemCommandOutput::ValueSet(v) => {
                tracing::info!("{}", t!("apu_mem_set", value = label_for_value(v)));
            }
            ApuMemCommandOutput::Error(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

impl ApuMemModel {
    fn sync_pill_selection(&self, value: i32) {
        if let Some(idx) = self.display_options.iter().position(|&v| v == value) {
            for (i, btn) in self.pill_btns.iter().enumerate() {
                btn.set_active(i == idx);
            }
        }
    }

    fn rebuild_strip(&mut self, root: &gtk::Box, sender: &ComponentSender<Self>) {
        // Remove any existing strip (last child after the two labels).
        // Labels are at indices 0 and 1; the strip is at 2 if it exists.
        let children: Vec<_> = {
            let mut v = Vec::new();
            let mut child = root.first_child();
            while let Some(c) = child {
                child = c.next_sibling();
                v.push(c);
            }
            v
        };
        // Remove everything after the first two label widgets.
        for c in children.into_iter().skip(2) {
            root.remove(&c);
        }

        let labels: Vec<String> = self.display_options.iter().map(|&v| label_for_value(v)).collect();
        let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        let (strip, btns) = pill_strip(&label_refs);
        strip.set_sensitive(self.available);

        // Wire callbacks with new indices.
        for (i, btn) in btns.iter().enumerate() {
            let sender = sender.clone();
            btn.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(ApuMemMsg::ChangeValue(i));
                }
            });
        }

        self.pill_btns = btns;
        self.sync_pill_selection(self.current_value);
        root.append(&strip);

        if self.available {
            let note = gtk::Label::new(Some(&t!("apu_mem_reboot_warning")));
            note.add_css_class("caption");
            note.add_css_class("dim-label");
            note.set_wrap(true);
            note.set_xalign(0.0);
            root.append(&note);
        }
    }
}
