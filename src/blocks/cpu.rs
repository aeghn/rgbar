use std::{
    fs::{self},
    str::FromStr,
};

use anyhow::{anyhow, Result};
use gdk::RGBA;
use glib::{Cast, MainContext};
use gtk::prelude::{BoxExt, LabelExt, StyleContextExt, WidgetExt};

use crate::utils::gtk_icon_loader;
use crate::utils::gtk_icon_loader::IconName;
use crate::widgets::chart::DrawDirection;
use crate::{
    constants::TriBool,
    datahodler::channel::{DualChannel, MReceiver, SSender},
    utils::fileutils,
    widgets::chart::{Chart, LineType, Series},
};

use super::Block;

const CPU_BOOST_PATH: &str = "/sys/devices/system/cpu/cpufreq/boost";
const CPU_NO_TURBO_PATH: &str = "/sys/devices/system/cpu/intel_pstate/no_turbo";

#[derive(Clone)]
pub enum CpuIn {}

#[derive(Clone)]
pub enum CpuOut {
    Turbo(TriBool),
    Frequencies(Vec<f64>),
    UtilizationAvg(f64),
    Utilizations(Vec<f64>),
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
        let sender = self.dualchannel.get_out_sender();

        let mut cputime = read_proc_stat()?;
        let cores = cputime.1.len();

        if cores == 0 {
            return Err(anyhow!("/proc/stat reported zero cores"));
        }

        glib::timeout_add_seconds_local(1, move || {
            let freqs = read_frequencies().expect("unable to read frequencies");
            sender.send(CpuOut::Frequencies(freqs)).unwrap();

            // Compute utilizations
            let new_cputime = read_proc_stat().unwrap();
            let utilization_avg = new_cputime.0.utilization(cputime.0);
            sender.send(CpuOut::UtilizationAvg(utilization_avg)).unwrap();
            let mut utilizations = Vec::new();
            if new_cputime.1.len() != cores {}
            for i in 0..cores {
                utilizations.push(new_cputime.1[i].utilization(cputime.1[i]));
            }
            sender.send(CpuOut::Utilizations(utilizations)).unwrap();

            cputime = new_cputime;

            // Read boost state on intel CPUs
            sender
                .send(CpuOut::Turbo(
                    boost_status()
                        .map(|e| if e { TriBool::True } else { TriBool::False })
                        .unwrap_or(TriBool::Unknown),
                ))
                .unwrap();

            glib::ControlFlow::Continue
        });

        Ok(())
    }

    fn get_channel(&self) -> (&SSender<Self::In>, &MReceiver<Self::Out>) {
        self.dualchannel.get_reveled()
    }

    fn widget(&self) -> gtk::Widget {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(false)
            .build();

        let image = gtk_icon_loader::load_image(IconName::CPU);

        let label = gtk::Label::builder().label("CPU: ").build();
        label.style_context().add_class("cpu-mem-label");

        let _turbo_str = String::new();
        let _freq = String::new();
        let mut label_str = String::new();

        let mut receiver = self.dualchannel.get_out_receiver();

        let series = Series::new("cpu", 100., 30, RGBA::new(1.0, 0.4, 0.4, 1.0), false);
        let chart = Chart::builder()
            .width(60)
            .line_width(2)
            .with_series(series.clone(), DrawDirection::DownTop)
            .line_type(LineType::Line)
            .build();
        chart.draw_in_seconds(1);
        chart.drawing_box.style_context().add_class("chart-border");
        holder.pack_start(&image, false, false, 0);
        holder.pack_end(&chart.drawing_box, false, false, 0);

        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                if let Ok(msg) = receiver.recv().await {
                    match msg {
                        CpuOut::Turbo(turbo) => {
                            let new = match turbo {
                                TriBool::True => "T",
                                TriBool::False => "N",
                                TriBool::Unknown => "",
                            };

                            let new = format!("{}", new);
                            if new.as_str() != label_str.as_str() {
                                label.set_label(&new);
                            }
                            label_str = new;
                        }
                        CpuOut::Frequencies(_freq) => {}
                        CpuOut::UtilizationAvg(avg) => {
                            series.add_value(avg * 100.);
                        }
                        CpuOut::Utilizations(_) => {}
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

        Some(Self {
            idle: idle + iowait,
            non_idle: user + nice + system + irq + softirq,
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
