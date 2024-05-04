use std::{fs, str::FromStr, time::Duration};

use anyhow::{anyhow, Result};
use gdk::{glib::Cast, RGBA};
use glib::MainContext;
use gtk::prelude::{BoxExt, LabelExt, StyleContextExt, WidgetExt};
use tracing::error;

use crate::utils::gtkiconloader::IconName;
use crate::{
    constants::TriBool,
    datahodler::channel::DualChannel,
    utils::fileutils,
    widgets::chart::{Chart, LineType, Series},
};
use crate::{statusbar::WidgetShareInfo, utils::gtkiconloader};

use super::{temp, Block};

const CPU_BOOST_PATH: &str = "/sys/devices/system/cpu/cpufreq/boost";
const CPU_NO_TURBO_PATH: &str = "/sys/devices/system/cpu/intel_pstate/no_turbo";

#[derive(Clone)]
pub enum CpuIn {}

#[derive(Clone)]
pub enum CpuOut {
    Frequencies(Vec<f64>),
    UtilizationAvg(f64, f64),
    Utilizations(Vec<f64>),
    CpuTemp(f64),
}

pub struct CpuBlock {
    dualchannel: DualChannel<CpuOut, CpuIn>,
}

impl CpuBlock {
    pub fn new() -> Self {
        let dualchannel = DualChannel::new(30);

        CpuBlock { dualchannel }
    }
}

impl Block for CpuBlock {
    type Out = CpuOut;

    type In = CpuIn;

    fn run(&mut self) -> anyhow::Result<()> {
        let mut cputime = read_proc_stat()?;
        let cores = cputime.1.len();

        if cores == 0 {
            return Err(anyhow!("/proc/stat reported zero cores"));
        }

        let mut temp_file = temp::match_type_dir("x86_pkg_temp").map(|mut p| {
            p.push("temp");
            p
        });

        let sender = self.dualchannel.get_out_sender();
        glib::timeout_add_seconds_local(1, move || {
            let freqs = read_frequencies().expect("unable to read frequencies");
            sender.send(CpuOut::Frequencies(freqs)).unwrap();

            // Compute utilizations
            let new_cputime = read_proc_stat().unwrap();
            let utilization_avg = new_cputime.0.utilization_user_and_system(cputime.0);
            sender
                .send(CpuOut::UtilizationAvg(utilization_avg.0, utilization_avg.1))
                .unwrap();
            let mut utilizations = Vec::new();
            if new_cputime.1.len() != cores {}
            for i in 0..cores {
                utilizations.push(new_cputime.1[i].utilization(cputime.1[i]));
            }
            sender.send(CpuOut::Utilizations(utilizations)).unwrap();

            cputime = new_cputime;

            glib::ControlFlow::Continue
        });

        let sender = self.dualchannel.get_out_sender();
        glib::timeout_add_local(Duration::from_millis(1600), move || {
            if let Ok(temp_path) = temp_file.as_ref() {
                let temp = temp::read_type_temp(temp_path);
                if let Ok(temp) = temp {
                    sender.send(CpuOut::CpuTemp(temp)).unwrap();
                }
            };

            glib::ControlFlow::Continue
        });

        Ok(())
    }

    fn widget(&self, _: &WidgetShareInfo) -> gtk::Widget {
        let mut receiver = self.dualchannel.get_out_receiver();

        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(false)
            .build();

        let icon = gtkiconloader::load_font_icon(IconName::CPU);

        let right_holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .build();

        let label_holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(true)
            .vexpand(false)
            .build();

        let utilization_label = gtk::Label::builder().build();
        utilization_label.style_context().add_class("cpu-util");
        let freq_label = gtkiconloader::load_font_icon(IconName::Empty);
        freq_label.style_context().add_class("cpu-freq");
        let temp_label = gtk::Label::builder().build();
        temp_label.style_context().add_class("cpu-temp");

        label_holder.pack_start(&utilization_label, false, false, 0);

        label_holder.pack_start(&temp_label, false, false, 0);
        label_holder.pack_start(&freq_label, false, false, 0);

        right_holder.pack_start(&label_holder, false, false, 0);

        let user_serie = Series::new("cpu_user", 100., 50, RGBA::new(0.5, 0.8, 1.0, 0.6));
        let system_serie = Series::new("cpu_system", 100., 50, RGBA::new(0.6, 0.6, 0.1, 0.6));
        /*         let cpu_temp = Series::new("cpu_temp", 100., 30, RGBA::new(1.0, 0.3, 0.1, 0.6))
        .with_baseline(crate::widgets::chart::BaselineType::FixedPercent(0.))
        .with_height_percent(1.)
        .with_line_type(LineType::Line); */
        let chart = Chart::builder()
            .with_width(30)
            .with_line_width(1.)
            .with_series(system_serie.clone())
            .with_series(user_serie.clone());
        chart.draw_in_seconds(2);

        right_holder.pack_end(&chart.drawing_box, true, true, 0);

        holder.pack_start(&icon, false, false, 0);
        holder.pack_end(&right_holder, false, false, 0);

        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                if let Ok(msg) = receiver.recv().await {
                    match msg {
                        CpuOut::Frequencies(freqs) => {
                            let max = freqs
                                .iter()
                                .max_by(|f1, f2| f64::total_cmp(&f1, &f2))
                                .unwrap_or(&0.);

                            let icon = if max < &(1. * 1e9) {
                                gtkiconloader::load_label(IconName::FreqShell)
                            } else if max < &(2. * 1e9) {
                                gtkiconloader::load_label(IconName::FreqSnail)
                            } else if max < &(3. * 1e9) {
                                gtkiconloader::load_label(IconName::FreqTurtle)
                            } else {
                                gtkiconloader::load_label(IconName::FreqRabbit)
                            };

                            freq_label.set_label(icon)
                        }
                        CpuOut::UtilizationAvg(user, system) => {
                            system_serie.add_value(system * 100.);
                            user_serie.add_value(user * 100.);
                            utilization_label
                                .set_label(format!("{:.1}%", (system + user) * 100.).as_str());
                        }
                        CpuOut::Utilizations(_) => {}
                        CpuOut::CpuTemp(temp) => {
                            temp_label.set_label(format!("{:.1}C", temp).as_str())
                        }
                    }
                }
            }
        });

        holder.upcast()
    }
}

// Read frequencies (read in MHz, store in Hz)
fn read_frequencies() -> Result<Vec<f64>> {
    let freqs: Vec<f64> = fileutils::read_lines("/proc/cpuinfo")
        .expect("Unable to read /proc/cpuinfo")
        .filter_map(|f| {
            if let Ok(line) = f {
                if line.starts_with("cpu MHz") {
                    let slice = line
                        .trim_end()
                        .trim_start_matches(|c: char| !c.is_ascii_digit());
                    return Some(
                        f64::from_str(slice).expect("failed to parse /proc/cpuinfo") * 1e6,
                    );
                }
            };
            None
        })
        .collect();

    Ok(freqs)
}

#[derive(Debug, Clone, Copy)]
struct CpuTime {
    idle: u64,
    non_idle: u64,
    user: u64,
    system_total: u64,
}

impl CpuTime {
    fn from_str(s: &str) -> Option<Self> {
        let mut s = s.trim().split_ascii_whitespace();
        let user = u64::from_str(s.next()?).ok()?;
        let nice = u64::from_str(s.next()?).ok()?;
        let system = u64::from_str(s.next()?).ok()?;
        let idle = u64::from_str(s.next()?).ok()?;
        let iowait = u64::from_str(s.next()?).ok()?;
        let irq = u64::from_str(s.next()?).ok()?;
        let softirq = u64::from_str(s.next()?).ok()?;

        let system_total = nice + system + irq + softirq;
        Some(Self {
            idle: idle + iowait,
            non_idle: user + system_total,
            user,
            system_total,
        })
    }

    fn utilization(&self, old: Self) -> f64 {
        let elapsed = (self.idle + self.non_idle).saturating_sub(old.idle + old.non_idle);
        if elapsed == 0 {
            0.0
        } else {
            ((self.non_idle - old.non_idle) as f64 / elapsed as f64).clamp(0., 1.)
        }
    }

    fn utilization_user_and_system(&self, old: Self) -> (f64, f64) {
        let elapsed = (self.idle + self.non_idle).saturating_sub(old.idle + old.non_idle);
        if elapsed == 0 {
            (0.0, 0.)
        } else {
            (
                ((self.user - old.user) as f64 / elapsed as f64).clamp(0., 1.),
                ((self.system_total - old.system_total) as f64 / elapsed as f64).clamp(0., 1.),
            )
        }
    }
}

fn read_proc_stat() -> Result<(CpuTime, Vec<CpuTime>)> {
    let mut utilizations = Vec::with_capacity(32);
    let mut total = None;

    fileutils::read_lines("/proc/stat")?.for_each(|l| {
        {
            if let Ok(line) = l {
                // Total time
                let data = line.trim_start_matches(|c: char| !c.is_ascii_whitespace());
                if line.starts_with("cpu ") {
                    if let Some(r) = CpuTime::from_str(data) {
                        total = Some(r);
                    }
                } else if line.starts_with("cpu") {
                    if let Some(r) = CpuTime::from_str(data) {
                        utilizations.push(r);
                    }
                }
            }
        }
    });

    Ok((total.unwrap(), utilizations))
}

/// Read the cpu turbo boost status from kernel sys interface
/// or intel pstate interface
fn boost_status() -> Option<bool> {
    if let Ok(boost) = fs::read_to_string(CPU_BOOST_PATH) {
        Some(boost.starts_with('1'))
    } else if let Ok(no_turbo) = fs::read_to_string(CPU_NO_TURBO_PATH) {
        Some(no_turbo.starts_with('0'))
    } else {
        None
    }
}
