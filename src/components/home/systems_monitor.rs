// Utu - ASUS Laptop Control Centre for Ubuntu
// Copyright (C) 2026 Baraka Malila — GPL-3.0-or-later
// Live CPU/GPU/Fan/RAM/Disk metric section — thin accent lines below each row.

use gtk4 as gtk;
use gtk4::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::sys_paths::*;

pub struct SystemsMonitorModel {
    cpu_freq_label: gtk::Label,
    cpu_temp_label: gtk::Label,
    gpu_temp_label: gtk::Label,
    fan_cpu_label: gtk::Label,
    fan_gpu_label: gtk::Label,
    ram_label: gtk::Label,
    disk_label: gtk::Label,
}

#[derive(Debug)]
pub enum SystemsMonitorMsg {}

#[derive(Debug)]
pub enum SystemsMonitorCmd {
    Refreshed {
        cpu_freq: String,
        cpu_temp: String,
        gpu_temp: String,
        fan_cpu: String,
        fan_gpu: String,
        ram: String,
        disk: String,
    },
}

fn metric_row(label_text: &str) -> (gtk::Box, gtk::Label) {
    let value = gtk::Label::new(Some("…"));
    value.set_halign(gtk::Align::End);
    value.set_hexpand(true);
    value.add_css_class("numeric");

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let lbl = gtk::Label::new(Some(label_text));
    lbl.set_halign(gtk::Align::Start);
    lbl.add_css_class("dim-label");
    header.append(&lbl);
    header.append(&value);

    let line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    line.add_css_class("metric-line");
    line.set_hexpand(true);

    let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
    row.append(&header);
    row.append(&line);

    (row, value)
}

fn metric_section(title: &str, rows: Vec<(gtk::Box, gtk::Label)>) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let title_lbl = gtk::Label::new(Some(title));
    title_lbl.set_halign(gtk::Align::Start);
    title_lbl.add_css_class("heading");
    section.append(&title_lbl);
    for (row, _) in rows {
        section.append(&row);
    }
    section
}

#[relm4::component(pub)]
impl Component for SystemsMonitorModel {
    type Init = ();
    type Input = SystemsMonitorMsg;
    type Output = ();
    type CommandOutput = SystemsMonitorCmd;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,

            append = &gtk::Label {
                set_label: &t!("home_systems_monitor"),
                set_halign: gtk::Align::Start,
                add_css_class: "title-2",
            },

            append = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 24,
                set_homogeneous: true,

                append = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 16,
                },

                append = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 16,
                },
            },
        }
    }

    fn init(_: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let (cpu_freq_row, cpu_freq_label) = metric_row("Frequency");
        let (cpu_temp_row, cpu_temp_label) = metric_row("Temperature");
        let (gpu_temp_row, gpu_temp_label) = metric_row("Temperature");
        let (fan_cpu_row, fan_cpu_label) = metric_row("CPU Fan");
        let (fan_gpu_row, fan_gpu_label) = metric_row("GPU Fan");
        let (ram_row, ram_label) = metric_row("RAM");
        let (disk_row, disk_label) = metric_row("Disk");

        let cpu_section = metric_section(
            "CPU",
            vec![
                (cpu_freq_row, cpu_freq_label.clone()),
                (cpu_temp_row, cpu_temp_label.clone()),
            ],
        );
        let fan_section = metric_section(
            "Fan",
            vec![
                (fan_cpu_row, fan_cpu_label.clone()),
                (fan_gpu_row, fan_gpu_label.clone()),
            ],
        );
        let gpu_section =
            metric_section("GPU", vec![(gpu_temp_row, gpu_temp_label.clone())]);
        let storage_section = metric_section(
            "Storage",
            vec![
                (ram_row, ram_label.clone()),
                (disk_row, disk_label.clone()),
            ],
        );

        let model = SystemsMonitorModel {
            cpu_freq_label,
            cpu_temp_label,
            gpu_temp_label,
            fan_cpu_label,
            fan_gpu_label,
            ram_label,
            disk_label,
        };
        let widgets = view_output!();

        // Populate the two columns in the grid box.
        // root: Box[Vertical] > Label, Box[Horizontal] > Box[left], Box[right]
        if let Some(grid) = root.last_child().and_then(|w| w.downcast::<gtk::Box>().ok()) {
            if let (Some(left), Some(right)) = (grid.first_child(), grid.last_child()) {
                if let (Ok(l), Ok(r)) =
                    (left.downcast::<gtk::Box>(), right.downcast::<gtk::Box>())
                {
                    l.append(&cpu_section);
                    l.append(&fan_section);
                    r.append(&gpu_section);
                    r.append(&storage_section);
                }
            }
        }

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

    fn update(&mut self, _msg: SystemsMonitorMsg, _: ComponentSender<Self>, _: &Self::Root) {}

    fn update_cmd(
        &mut self,
        msg: SystemsMonitorCmd,
        _: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        let SystemsMonitorCmd::Refreshed {
            cpu_freq,
            cpu_temp,
            gpu_temp,
            fan_cpu,
            fan_gpu,
            ram,
            disk,
        } = msg;
        self.cpu_freq_label.set_label(&cpu_freq);
        self.cpu_temp_label.set_label(&cpu_temp);
        self.gpu_temp_label.set_label(&gpu_temp);
        self.fan_cpu_label.set_label(&fan_cpu);
        self.fan_gpu_label.set_label(&fan_gpu);
        self.ram_label.set_label(&ram);
        self.disk_label.set_label(&disk);
    }
}

async fn fetch_metrics() -> SystemsMonitorCmd {
    let cpu_freq = tokio::fs::read_to_string(
        "/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq",
    )
    .await
    .ok()
    .and_then(|s| s.trim().parse::<u64>().ok())
    .map(|khz| format!("{:.1} GHz", khz as f64 / 1_000_000.0))
    .unwrap_or_else(|| "? GHz".to_string());

    let cpu_temp = tokio::fs::read_to_string(SYS_THERMAL_ZONE0_TEMP)
        .await
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
        .map(|mt| format!("{}°C", mt / 1000))
        .unwrap_or_else(|| "?°C".to_string());

    let gpu_temp = read_gpu_temp().await;
    let (fan_cpu, fan_gpu) = read_fan_rpms().await;

    let ram = tokio::fs::read_to_string(SYS_MEM_INFO)
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
            if total > 0 {
                format!("{} / {} GB", (total - avail) / 1_048_576, total / 1_048_576)
            } else {
                "N/A".to_string()
            }
        })
        .unwrap_or_else(|_| "N/A".to_string());

    let disk = tokio::process::Command::new("df")
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

    SystemsMonitorCmd::Refreshed {
        cpu_freq,
        cpu_temp,
        gpu_temp,
        fan_cpu,
        fan_gpu,
        ram,
        disk,
    }
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

async fn read_fan_rpms() -> (String, String) {
    let mut cpu_rpm = None;
    let mut gpu_rpm = None;
    for entry in std::fs::read_dir("/sys/class/hwmon")
        .into_iter()
        .flatten()
        .flatten()
    {
        let name_path = entry.path().join("name");
        if let Ok(name) = std::fs::read_to_string(&name_path) {
            if name.trim().contains("asus") {
                if let Ok(f1) =
                    std::fs::read_to_string(entry.path().join("fan1_input"))
                {
                    cpu_rpm =
                        f1.trim().parse::<u32>().ok().map(|r| format!("{} RPM", r));
                }
                if let Ok(f2) =
                    std::fs::read_to_string(entry.path().join("fan2_input"))
                {
                    gpu_rpm =
                        f2.trim().parse::<u32>().ok().map(|r| format!("{} RPM", r));
                }
            }
        }
    }
    (
        cpu_rpm.unwrap_or_else(|| "? RPM".to_string()),
        gpu_rpm.unwrap_or_else(|| "? RPM".to_string()),
    )
}
