// Utu - ASUS Laptop Control Centre for Ubuntu
// Copyright (C) 2026 Baraka Malila — GPL-3.0-or-later

pub mod mode_strip;
pub mod systems_monitor;

use gtk4 as gtk;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use self::mode_strip::{ModeStripModel, ModeStripMsg};
use self::systems_monitor::{SystemsMonitorModel, SystemsMonitorMsg};
use crate::sys_paths::*;

const IMG_TUF: &[u8] = include_bytes!("../../../assets/img/tuf.png");
const IMG_ROG: &[u8] = include_bytes!("../../../assets/img/rog.png");
const IMG_ZENBOOK: &[u8] = include_bytes!("../../../assets/img/zenbook.png");
const IMG_VIVOBOOK: &[u8] = include_bytes!("../../../assets/img/vivobook.png");
const IMG_PROART: &[u8] = include_bytes!("../../../assets/img/proart.png");
const IMG_EXPERTBOOK: &[u8] = include_bytes!("../../../assets/img/expertbook.png");

pub struct HomeModel {
    product_label: gtk::Label,
    gpu_badge: gtk::Label,
    battery_pct_label: gtk::Label,
    battery_status_label: gtk::Label,
    battery_limit_label: gtk::Label,
    battery_health_label: gtk::Label,
    battery_bar: gtk::ProgressBar,
    laptop_image: gtk::Picture,
    systems_monitor: relm4::Controller<SystemsMonitorModel>,
    mode_strip: relm4::Controller<ModeStripModel>,
}

#[derive(Debug)]
pub enum HomeOutput {
    NavigateToHardware,
    Error(String),
}

#[derive(Debug)]
pub enum HomeMsg {
    Refresh,
}

#[derive(Debug)]
pub enum HomeCmd {
    Loaded {
        product: String,
        gpu_mode: String,
        img_bytes: &'static [u8],
    },
    BatteryRefreshed {
        pct: u8,
        status: String,
        limit: u8,
        health_pct: u8,
    },
}

#[relm4::component(pub)]
impl Component for HomeModel {
    type Init = ();
    type Input = HomeMsg;
    type Output = HomeOutput;
    type CommandOutput = HomeCmd;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_vexpand: true,

            append = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 32,
                set_margin_top: 24,
                set_margin_start: 32,
                set_margin_end: 32,
                set_vexpand: true,

                // ── Left column (~40%) ───────────────────────────────────
                append = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 16,
                    set_width_request: 320,

                    append = &model.laptop_image.clone(),

                    append = &model.product_label.clone(),

                    append = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        append = &model.gpu_badge.clone() -> gtk::Label {
                            add_css_class: "pill",
                            add_css_class: "accent",
                        },
                    },

                    // Battery section
                    append = &adw::PreferencesGroup {
                        add = &adw::ActionRow {
                            set_title: "",
                            add_suffix = &model.battery_pct_label.clone(),
                        },
                        add = &adw::ActionRow {
                            set_title: "",
                            add_suffix = &model.battery_bar.clone() -> gtk::ProgressBar {
                                set_hexpand: true,
                                set_valign: gtk::Align::Center,
                            },
                        },
                        add = &adw::ActionRow {
                            set_title: "",
                            add_suffix = &model.battery_status_label.clone(),
                        },
                        add = &adw::ActionRow {
                            set_title: "",
                            add_suffix = &model.battery_limit_label.clone(),
                        },
                        add = &adw::ActionRow {
                            set_title: "",
                            add_suffix = &model.battery_health_label.clone(),
                        },
                    },
                },

                // ── Right column (~60%) ──────────────────────────────────
                append = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 24,
                    set_hexpand: true,

                    append = &model.systems_monitor.widget().clone(),

                    append = &model.mode_strip.widget().clone(),
                },
            },

            // Footer glow strip
            append = &gtk::Box {
                add_css_class: "glow-footer",
                set_hexpand: true,
                set_height_request: 4,
            },
        }
    }

    fn init(_: (), _root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let product_label = gtk::Label::new(Some("ASUS Laptop"));
        product_label.add_css_class("title-1");
        product_label.set_halign(gtk::Align::Start);

        let gpu_badge = gtk::Label::new(Some("GPU Mode: Hybrid"));
        let battery_pct_label = gtk::Label::new(Some("…%"));
        battery_pct_label.add_css_class("title-2");
        let battery_status_label = gtk::Label::new(Some("…"));
        battery_status_label.add_css_class("dim-label");
        let battery_limit_label = gtk::Label::new(Some("…"));
        battery_limit_label.add_css_class("dim-label");
        let battery_health_label = gtk::Label::new(Some("…"));
        battery_health_label.add_css_class("dim-label");

        let battery_bar = gtk::ProgressBar::new();
        battery_bar.set_hexpand(true);
        battery_bar.set_valign(gtk::Align::Center);

        let laptop_image = gtk::Picture::new();
        laptop_image.set_width_request(300);
        laptop_image.set_height_request(200);
        laptop_image.set_can_shrink(true);
        laptop_image.set_content_fit(gtk::ContentFit::Contain);
        laptop_image.set_valign(gtk::Align::Center);
        laptop_image.set_tooltip_text(Some(&t!("home_click_for_hardware")));

        let systems_monitor = SystemsMonitorModel::builder()
            .launch(())
            .forward(sender.input_sender(), |_: ()| HomeMsg::Refresh);
        let mode_strip = ModeStripModel::builder()
            .launch(())
            .forward(sender.input_sender(), |_: ()| HomeMsg::Refresh);

        let model = HomeModel {
            product_label,
            gpu_badge,
            battery_pct_label,
            battery_status_label,
            battery_limit_label,
            battery_health_label,
            battery_bar,
            laptop_image,
            systems_monitor,
            mode_strip,
        };

        let widgets = view_output!();

        // Wire click on laptop image → NavigateToHardware
        {
            let gesture = gtk::GestureClick::new();
            let out = sender.output_sender().clone();
            gesture.connect_released(move |_, _, _, _| {
                let _ = out.send(HomeOutput::NavigateToHardware);
            });
            model.laptop_image.add_controller(gesture);
        }

        // Load static info (product name, GPU mode)
        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let product = tokio::fs::read_to_string(SYS_PRODUCT_NAME)
                        .await
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| "ASUS Laptop".to_string());

                    let gpu_mode = crate::services::dbus::get_gpu_mode()
                        .await
                        .map(|m| format!("{:?}", m))
                        .unwrap_or_else(|_| "Hybrid".to_string());

                    let img_bytes = select_laptop_img(&product);
                    out.emit(HomeCmd::Loaded {
                        product,
                        gpu_mode,
                        img_bytes,
                    });
                })
                .drop_on_shutdown()
        });

        // Poll battery every 60 seconds
        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    loop {
                        out.emit(fetch_battery().await);
                        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _msg: HomeMsg, _sender: ComponentSender<Self>, _root: &Self::Root) {}

    fn update_cmd(
        &mut self,
        msg: HomeCmd,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            HomeCmd::Loaded {
                product,
                gpu_mode,
                img_bytes,
            } => {
                self.product_label.set_label(&product);
                self.gpu_badge.set_label(&t!("home_gpu_mode_badge", mode = gpu_mode));

                if let Ok(texture) = load_texture(img_bytes) {
                    self.laptop_image.set_paintable(Some(&texture));
                } else if let Some(display) = gdk::Display::default() {
                    let theme = gtk::IconTheme::for_display(&display);
                    let icon = theme.lookup_icon(
                        "computer-symbolic",
                        &[],
                        192,
                        1,
                        gtk::TextDirection::None,
                        gtk::IconLookupFlags::empty(),
                    );
                    self.laptop_image.set_paintable(Some(&icon));
                }
            }
            HomeCmd::BatteryRefreshed {
                pct,
                status,
                limit,
                health_pct,
            } => {
                self.battery_pct_label.set_label(&format!("{}%", pct));
                self.battery_bar.set_fraction(pct as f64 / 100.0);
                self.battery_status_label.set_label(&status);
                self.battery_limit_label
                    .set_label(&t!("home_battery_limit", pct = limit));
                self.battery_health_label
                    .set_label(&t!("home_battery_health_short", pct = health_pct));
            }
        }
    }
}

fn select_laptop_img(product_name: &str) -> &'static [u8] {
    let p = product_name.to_uppercase();
    if p.contains("ROG") {
        IMG_ROG
    } else if p.contains("TUF") {
        IMG_TUF
    } else if p.contains("ZENBOOK") || p.contains("ZEN") {
        IMG_ZENBOOK
    } else if p.contains("VIVOBOOK") || p.contains("VIVO") {
        IMG_VIVOBOOK
    } else if p.contains("PROART") {
        IMG_PROART
    } else if p.contains("EXPERTBOOK") || p.contains("EXPERT") {
        IMG_EXPERTBOOK
    } else {
        IMG_TUF
    }
}

fn load_texture(bytes: &'static [u8]) -> Result<gdk::Texture, glib::Error> {
    let glib_bytes = glib::Bytes::from_static(bytes);
    gdk::Texture::from_bytes(&glib_bytes)
}

async fn fetch_battery() -> HomeCmd {
    let pct = tokio::fs::read_to_string(SYS_BATTERY0_CAPACITY)
        .await
        .ok()
        .or_else(|| {
            std::fs::read_to_string(SYS_BATTERY1_CAPACITY).ok()
        })
        .and_then(|s| s.trim().parse::<u8>().ok())
        .unwrap_or(0);

    let status_raw = tokio::fs::read_to_string("/sys/class/power_supply/BAT0/status")
        .await
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

    let status = match status_raw.as_str() {
        "Charging" => t!("home_status_charging").to_string(),
        "Discharging" => t!("home_status_discharging").to_string(),
        "Full" => t!("home_status_full").to_string(),
        other => other.to_string(),
    };

    let limit = tokio::fs::read_to_string(
        "/sys/class/power_supply/BAT0/charge_control_end_threshold",
    )
    .await
    .ok()
    .and_then(|s| s.trim().parse::<u8>().ok())
    .unwrap_or(100);

    let energy_full = tokio::fs::read_to_string("/sys/class/power_supply/BAT0/energy_full")
        .await
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(1);

    let energy_design =
        tokio::fs::read_to_string("/sys/class/power_supply/BAT0/energy_full_design")
            .await
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(1);

    let health_pct = ((energy_full as f64 / energy_design as f64) * 100.0).min(100.0) as u8;

    HomeCmd::BatteryRefreshed {
        pct,
        status,
        limit,
        health_pct,
    }
}
