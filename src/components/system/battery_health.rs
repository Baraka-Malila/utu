// Utu - ASUS Laptop Control Centre for Ubuntu
// Copyright (C) 2026 Baraka Malila — GPL-3.0-or-later

use gtk4 as gtk;
use gtk4::prelude::*;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;
use serde::{Deserialize, Serialize};

const ENERGY_FULL: &str = "/sys/class/power_supply/BAT0/energy_full";
const ENERGY_DESIGN: &str = "/sys/class/power_supply/BAT0/energy_full_design";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HealthEntry {
    pub date: String,
    pub health_pct: f64,
    pub full_wh: f64,
    pub design_wh: f64,
}

pub struct BatteryHealth {
    entries: Vec<HealthEntry>,
    current_pct: f64,
    current_full_wh: f64,
    design_wh: f64,
    drawing_area: gtk::DrawingArea,
    summary_label: gtk::Label,
}

#[derive(Debug)]
pub enum BatteryHealthMsg {}

#[derive(Debug)]
pub enum BatteryHealthCmd {
    Loaded {
        entries: Vec<HealthEntry>,
        pct: f64,
        full_wh: f64,
        design_wh: f64,
    },
}

fn health_data_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("io.github", "baraka_malila", "utu")
        .map(|d| d.data_local_dir().join("battery_health.json"))
}

async fn load_and_update() -> BatteryHealthCmd {
    let full_uj = tokio::fs::read_to_string(ENERGY_FULL)
        .await
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let design_uj = tokio::fs::read_to_string(ENERGY_DESIGN)
        .await
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(1);

    let full_wh = full_uj as f64 / 1_000_000.0;
    let design_wh = design_uj as f64 / 1_000_000.0;
    let pct = (full_wh / design_wh * 100.0).clamp(0.0, 100.0);

    let today = chrono_today();
    let path = health_data_path();

    let mut entries: Vec<HealthEntry> = path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let last_date = entries.last().map(|e| e.date.as_str()).unwrap_or("");
    if last_date != today {
        entries.push(HealthEntry {
            date: today.to_string(),
            health_pct: pct,
            full_wh,
            design_wh,
        });
        if let Some(p) = &path {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(
                p,
                serde_json::to_string_pretty(&entries).unwrap_or_default(),
            );
        }
    }

    BatteryHealthCmd::Loaded {
        entries,
        pct,
        full_wh,
        design_wh,
    }
}

fn chrono_today() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let (y, m, d) = epoch_days_to_ymd(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn epoch_days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let yd = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            366
        } else {
            365
        };
        if days < yd {
            break;
        }
        days -= yd;
        year += 1;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let months = [
        31u64,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for &mdays in &months {
        if days < mdays {
            break;
        }
        days -= mdays;
        month += 1;
    }
    (year, month, days + 1)
}

#[relm4::component(pub)]
impl Component for BatteryHealth {
    type Init = ();
    type Input = BatteryHealthMsg;
    type Output = String;
    type CommandOutput = BatteryHealthCmd;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("battery_health_title"),

            add = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_margin_top: 8,
                set_margin_bottom: 8,
                set_margin_start: 8,
                set_margin_end: 8,

                append = &model.drawing_area.clone(),
                append = &model.summary_label.clone(),
            },
        }
    }

    fn init(_: (), _root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let drawing_area = gtk::DrawingArea::new();
        drawing_area.set_height_request(180);
        drawing_area.add_css_class("health-graph");

        let summary_label = gtk::Label::new(Some(&t!("battery_health_loading")));
        summary_label.add_css_class("dim-label");
        summary_label.set_halign(gtk::Align::Start);

        let model = BatteryHealth {
            entries: Vec::new(),
            current_pct: 0.0,
            current_full_wh: 0.0,
            design_wh: 0.0,
            drawing_area,
            summary_label,
        };
        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    out.emit(load_and_update().await);
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        _msg: BatteryHealthMsg,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
    }

    fn update_cmd(
        &mut self,
        msg: BatteryHealthCmd,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        let BatteryHealthCmd::Loaded {
            entries,
            pct,
            full_wh,
            design_wh,
        } = msg;
        self.entries = entries;
        self.current_pct = pct;
        self.current_full_wh = full_wh;
        self.design_wh = design_wh;

        if self.entries.len() < 2 {
            self.summary_label
                .set_label(&t!("battery_health_insufficient"));
        } else {
            self.summary_label.set_label(&t!(
                "battery_health_summary",
                pct = format!("{:.1}", pct),
                design = format!("{:.0}", design_wh),
                now = format!("{:.1}", full_wh)
            ));
        }

        let entries = self.entries.clone();
        self.drawing_area.set_draw_func(move |_, cr, w, h| {
            draw_health_graph(cr, w, h, &entries);
        });
        self.drawing_area.queue_draw();
    }
}

fn draw_health_graph(cr: &gtk4::cairo::Context, w: i32, h: i32, entries: &[HealthEntry]) {
    let wf = w as f64;
    let hf = h as f64;

    // Grid lines at 25%, 50%, 75%
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.08);
    cr.set_line_width(1.0);
    for &pct in &[25.0f64, 50.0, 75.0] {
        let y = hf - (pct / 100.0) * hf;
        cr.move_to(0.0, y);
        cr.line_to(wf, y);
        let _ = cr.stroke();
    }

    if entries.len() < 2 {
        return;
    }

    let min_pct = entries
        .iter()
        .map(|e| e.health_pct)
        .fold(f64::INFINITY, f64::min);
    let max_pct = entries
        .iter()
        .map(|e| e.health_pct)
        .fold(f64::NEG_INFINITY, f64::max);
    let range = (max_pct - min_pct).max(1.0);
    let n = entries.len() as f64;

    // Accent line (amber #e8a800)
    cr.set_source_rgb(0.91, 0.659, 0.0);
    cr.set_line_width(2.0);
    for (i, entry) in entries.iter().enumerate() {
        let x = (i as f64 / (n - 1.0)) * wf;
        let y = hf - ((entry.health_pct - min_pct) / range) * (hf - 20.0) - 10.0;
        if i == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    let _ = cr.stroke();

    // Data point squares
    cr.set_source_rgb(1.0, 1.0, 1.0);
    for (i, entry) in entries.iter().enumerate() {
        let x = (i as f64 / (n - 1.0)) * wf;
        let y = hf - ((entry.health_pct - min_pct) / range) * (hf - 20.0) - 10.0;
        cr.rectangle(x - 2.0, y - 2.0, 4.0, 4.0);
        let _ = cr.fill();
    }
}
