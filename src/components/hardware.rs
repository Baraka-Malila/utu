// Utu - ASUS Laptop Control Centre for Ubuntu
// Copyright (C) 2026 Baraka Malila — GPL-3.0-or-later
// Hardware detail page — Overview tab with live metrics + placeholder tabs.

use gtk4 as gtk;
use gtk4::prelude::*;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

pub struct HardwareModel {
    bios_label: gtk::Label,
    kernel_label: gtk::Label,
    cpu_name_label: gtk::Label,
    cpu_temp_label: gtk::Label,
    cpu_util_bar: gtk::ProgressBar,
    ram_bar: gtk::ProgressBar,
    ram_label: gtk::Label,
    gpu_name_label: gtk::Label,
    gpu_temp_label: gtk::Label,
    disk_label: gtk::Label,
    view_switcher: adw::ViewSwitcher,
    view_stack: adw::ViewStack,
}

#[derive(Debug)]
pub enum HardwareMsg {}

#[derive(Debug)]
pub enum HardwareCmd {
    StaticLoaded {
        bios: String,
        kernel: String,
        cpu_name: String,
        gpu_name: String,
    },
    MetricsRefreshed {
        cpu_temp: String,
        cpu_util: f64,
        gpu_temp: String,
        ram_used_gb: f64,
        ram_total_gb: f64,
        disk_used: String,
    },
}

#[relm4::component(pub)]
impl Component for HardwareModel {
    type Init = ();
    type Input = HardwareMsg;
    type Output = String;
    type CommandOutput = HardwareCmd;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_vexpand: true,

            append = &model.view_switcher.clone(),

            append = &gtk::ScrolledWindow {
                set_vexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_child = Some(&model.view_stack.clone()),
            },
        }
    }

    fn init(_: (), _root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let bios_label = gtk::Label::new(Some("…"));
        let kernel_label = gtk::Label::new(Some("…"));
        let cpu_name_label = gtk::Label::new(Some("…"));
        let cpu_temp_label = gtk::Label::new(Some("…°C"));
        let cpu_util_bar = gtk::ProgressBar::new();
        cpu_util_bar.set_hexpand(true);
        let ram_bar = gtk::ProgressBar::new();
        ram_bar.set_hexpand(true);
        let ram_label = gtk::Label::new(Some("…"));
        let gpu_name_label = gtk::Label::new(Some("…"));
        let gpu_temp_label = gtk::Label::new(Some("…°C"));
        let disk_label = gtk::Label::new(Some("…"));

        let view_stack = adw::ViewStack::new();
        let view_switcher = adw::ViewSwitcher::new();
        view_switcher.set_stack(Some(&view_stack));
        view_switcher.set_policy(adw::ViewSwitcherPolicy::Wide);

        let overview = build_overview(
            &bios_label,
            &kernel_label,
            &cpu_name_label,
            &cpu_temp_label,
            &cpu_util_bar,
            &gpu_name_label,
            &gpu_temp_label,
            &ram_bar,
            &ram_label,
            &disk_label,
        );
        view_stack.add_titled(&overview, Some("overview"), &t!("hardware_overview"));

        for (id, title) in &[
            ("cpu", t!("hardware_tab_cpu").as_ref()),
            ("gpu", t!("hardware_tab_gpu").as_ref()),
            ("memory", t!("hardware_tab_memory").as_ref()),
            ("storage", t!("hardware_tab_storage").as_ref()),
            ("thermals", t!("hardware_tab_thermals").as_ref()),
        ] {
            let placeholder = gtk::Label::new(Some("Coming in Phase 5"));
            placeholder.set_valign(gtk::Align::Center);
            placeholder.set_halign(gtk::Align::Center);
            placeholder.add_css_class("dim-label");
            view_stack.add_titled(&placeholder, Some(id), title);
        }

        let model = HardwareModel {
            bios_label,
            kernel_label,
            cpu_name_label,
            cpu_temp_label,
            cpu_util_bar,
            ram_bar,
            ram_label,
            gpu_name_label,
            gpu_temp_label,
            disk_label,
            view_switcher,
            view_stack,
        };
        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let bios =
                        tokio::fs::read_to_string("/sys/class/dmi/id/bios_version")
                            .await
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default();
                    let kernel = tokio::process::Command::new("uname")
                        .arg("-r")
                        .output()
                        .await
                        .map(|o| {
                            String::from_utf8_lossy(&o.stdout).trim().to_string()
                        })
                        .unwrap_or_default();
                    let cpu_name = tokio::fs::read_to_string("/proc/cpuinfo")
                        .await
                        .map(|s| {
                            s.lines()
                                .find(|l| l.starts_with("model name"))
                                .and_then(|l| l.split(':').nth(1))
                                .map(|s| s.trim().to_string())
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();
                    let gpu_name = detect_gpu_name().await;
                    out.emit(HardwareCmd::StaticLoaded {
                        bios,
                        kernel,
                        cpu_name,
                        gpu_name,
                    });
                })
                .drop_on_shutdown()
        });

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    loop {
                        out.emit(fetch_metrics().await);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _: HardwareMsg, _: ComponentSender<Self>, _: &Self::Root) {}

    fn update_cmd(
        &mut self,
        msg: HardwareCmd,
        _: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match msg {
            HardwareCmd::StaticLoaded {
                bios,
                kernel,
                cpu_name,
                gpu_name,
            } => {
                self.bios_label.set_label(&bios);
                self.kernel_label.set_label(&kernel);
                self.cpu_name_label.set_label(&cpu_name);
                self.gpu_name_label.set_label(&gpu_name);
            }
            HardwareCmd::MetricsRefreshed {
                cpu_temp,
                cpu_util,
                gpu_temp,
                ram_used_gb,
                ram_total_gb,
                disk_used,
            } => {
                self.cpu_temp_label.set_label(&cpu_temp);
                self.cpu_util_bar.set_fraction(cpu_util);
                self.gpu_temp_label.set_label(&gpu_temp);
                self.ram_bar
                    .set_fraction(ram_used_gb / ram_total_gb.max(1.0));
                self.ram_label.set_label(&format!(
                    "{:.1} / {:.0} GB",
                    ram_used_gb, ram_total_gb
                ));
                self.disk_label.set_label(&disk_used);
            }
        }
    }
}

fn info_row(label: &str, value: &gtk::Label) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(label);
    row.set_selectable(false);
    value.set_valign(gtk::Align::Center);
    value.add_css_class("dim-label");
    row.add_suffix(value);
    row
}

fn build_overview(
    bios_label: &gtk::Label,
    kernel_label: &gtk::Label,
    cpu_name_label: &gtk::Label,
    cpu_temp_label: &gtk::Label,
    cpu_util_bar: &gtk::ProgressBar,
    gpu_name_label: &gtk::Label,
    gpu_temp_label: &gtk::Label,
    ram_bar: &gtk::ProgressBar,
    ram_label: &gtk::Label,
    disk_label: &gtk::Label,
) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.set_margin_top(16);
    page.set_margin_bottom(24);
    page.set_margin_start(32);
    page.set_margin_end(32);

    let system_group = adw::PreferencesGroup::new();
    system_group.set_title(&t!("hardware_kernel"));
    system_group.add(&info_row("BIOS", bios_label));
    system_group.add(&info_row("Kernel", kernel_label));
    page.append(&system_group);

    let cpu_group = adw::PreferencesGroup::new();
    cpu_group.set_title(&t!("hardware_processor"));
    cpu_group.add(&info_row("Model", cpu_name_label));
    let temp_row = adw::ActionRow::new();
    temp_row.set_title("Temperature");
    temp_row.set_selectable(false);
    cpu_temp_label.set_valign(gtk::Align::Center);
    cpu_temp_label.add_css_class("dim-label");
    temp_row.add_suffix(cpu_temp_label);
    cpu_group.add(&temp_row);
    let util_row = adw::ActionRow::new();
    util_row.set_title("Utilisation");
    util_row.set_selectable(false);
    cpu_util_bar.set_valign(gtk::Align::Center);
    util_row.add_suffix(cpu_util_bar);
    cpu_group.add(&util_row);
    page.append(&cpu_group);

    let gpu_group = adw::PreferencesGroup::new();
    gpu_group.set_title(&t!("hardware_graphics"));
    gpu_group.add(&info_row("Model", gpu_name_label));
    let gpu_temp_row = adw::ActionRow::new();
    gpu_temp_row.set_title("Temperature");
    gpu_temp_row.set_selectable(false);
    gpu_temp_label.set_valign(gtk::Align::Center);
    gpu_temp_label.add_css_class("dim-label");
    gpu_temp_row.add_suffix(gpu_temp_label);
    gpu_group.add(&gpu_temp_row);
    page.append(&gpu_group);

    let mem_group = adw::PreferencesGroup::new();
    mem_group.set_title(&t!("hardware_memory"));
    let ram_row = adw::ActionRow::new();
    ram_row.set_title("Usage");
    ram_row.set_selectable(false);
    ram_label.set_valign(gtk::Align::Center);
    ram_label.add_css_class("dim-label");
    ram_bar.set_valign(gtk::Align::Center);
    ram_row.add_suffix(ram_label);
    ram_row.add_suffix(ram_bar);
    mem_group.add(&ram_row);
    page.append(&mem_group);

    let storage_group = adw::PreferencesGroup::new();
    storage_group.set_title(&t!("hardware_storage"));
    let disk_row = adw::ActionRow::new();
    disk_row.set_title("Used");
    disk_row.set_selectable(false);
    disk_label.set_valign(gtk::Align::Center);
    disk_label.add_css_class("dim-label");
    disk_row.add_suffix(disk_label);
    storage_group.add(&disk_row);
    page.append(&storage_group);

    page
}

async fn detect_gpu_name() -> String {
    if let Ok(out) = tokio::process::Command::new("lspci").output().await {
        let output = String::from_utf8_lossy(&out.stdout);
        for line in output.lines() {
            let lo = line.to_lowercase();
            if lo.contains("vga") || lo.contains("3d") || lo.contains("display") {
                if lo.contains("nvidia") || lo.contains("amd") || lo.contains("intel") {
                    if let Some(part) = line.split(':').nth(2) {
                        return part.trim().to_string();
                    }
                }
            }
        }
    }
    "Unknown GPU".to_string()
}

async fn fetch_metrics() -> HardwareCmd {
    let cpu_temp = tokio::fs::read_to_string(
        "/sys/class/thermal/thermal_zone0/temp",
    )
    .await
    .ok()
    .and_then(|s| s.trim().parse::<i32>().ok())
    .map(|mt| format!("{}°C", mt / 1000))
    .unwrap_or_else(|| "?°C".to_string());

    let cpu_util = read_cpu_util().await;
    let gpu_temp = read_gpu_temp().await;

    let (ram_used_gb, ram_total_gb) =
        tokio::fs::read_to_string("/proc/meminfo")
            .await
            .map(|s| {
                let mut total = 0u64;
                let mut avail = 0u64;
                for line in s.lines() {
                    if line.starts_with("MemTotal:") {
                        total = line
                            .split_whitespace()
                            .nth(1)
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0);
                    } else if line.starts_with("MemAvailable:") {
                        avail = line
                            .split_whitespace()
                            .nth(1)
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0);
                    }
                }
                let used = total.saturating_sub(avail);
                (used as f64 / 1_048_576.0, total as f64 / 1_048_576.0)
            })
            .unwrap_or((0.0, 1.0));

    let disk_used = tokio::process::Command::new("df")
        .args(["-h", "/"])
        .output()
        .await
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .nth(1)
                .and_then(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
                .unwrap_or_else(|| "N/A".to_string())
        })
        .unwrap_or_else(|_| "N/A".to_string());

    HardwareCmd::MetricsRefreshed {
        cpu_temp,
        cpu_util,
        gpu_temp,
        ram_used_gb,
        ram_total_gb,
        disk_used,
    }
}

async fn read_cpu_util() -> f64 {
    // Read /proc/stat twice with a short gap to calculate utilisation
    async fn read_stat() -> Option<(u64, u64)> {
        let s = tokio::fs::read_to_string("/proc/stat").await.ok()?;
        let line = s.lines().next()?;
        let vals: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|v| v.parse().ok())
            .collect();
        if vals.len() < 4 {
            return None;
        }
        let idle = vals[3] + vals.get(4).copied().unwrap_or(0);
        let total: u64 = vals.iter().sum();
        Some((idle, total))
    }

    let before = read_stat().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let after = read_stat().await;

    if let (Some((idle0, total0)), Some((idle1, total1))) = (before, after) {
        let d_total = total1.saturating_sub(total0);
        let d_idle = idle1.saturating_sub(idle0);
        if d_total > 0 {
            return 1.0 - (d_idle as f64 / d_total as f64);
        }
    }
    0.0
}

async fn read_gpu_temp() -> String {
    for entry in std::fs::read_dir("/sys/class/hwmon")
        .into_iter()
        .flatten()
        .flatten()
    {
        let name_path = entry.path().join("name");
        if let Ok(name) = std::fs::read_to_string(&name_path) {
            let n = name.trim().to_lowercase();
            if n.contains("nvidia") || n.contains("amdgpu") || n.contains("radeon") {
                if let Ok(t) =
                    std::fs::read_to_string(entry.path().join("temp1_input"))
                {
                    if let Ok(mt) = t.trim().parse::<i32>() {
                        return format!("{}°C", mt / 1000);
                    }
                }
            }
        }
    }
    "?°C".to_string()
}
