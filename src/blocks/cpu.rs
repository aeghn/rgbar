use std::{
    fs::{self, File},
    io::BufReader,
    str::FromStr, ops::Add,
};

use anyhow::{anyhow, Error, Result};
use gdk::RGBA;
use glib::{Continue, MainContext, Cast};
use gtk::{false_, prelude::{BoxExt, LabelExt}};

use crate::{
    constants::TriBool,
    datahodler::channel::{DualChannel, MReceiver, SSender},
    utils::fileutils, widgets::chart::{Chart, Series, LineType},
};
use crate::widgets::chart::DrawDirection;

use super::Block;

const CPU_BOOST_PATH: &str = "/sys/devices/system/cpu/cpufreq/boost";
const CPU_NO_TURBO_PATH: &str = "/sys/devices/system/cpu/intel_pstate/no_turbo";

#[derive(Clone)]
pub enum CpuBM {}

#[derive(Clone)]
pub enum CpuWM {
    Turbo(TriBool),
    Frequencies(Vec<f64>),
    UtilizationAvg(f64),
    Utilizations(Vec<f64>),
}

pub struct CpuBlock {
    dualchannel: DualChannel<CpuWM, CpuBM>,
}

impl CpuBlock {
    pub fn new() -> Self {
        let dualchannel = DualChannel::new(30);

        CpuBlock { dualchannel }
    }
}

impl Block for CpuBlock {
    type WM = CpuWM;

    type BM = CpuBM;

    fn loop_receive(&mut self) -> anyhow::Result<()> {
        let sender = self.dualchannel.get_out_sender();

        let mut cputime = read_proc_stat()?;
        let cores = cputime.1.len();

        if cores == 0 {
            return Err(anyhow!("/proc/stat reported zero cores"));
        }

        glib::timeout_add_seconds_local(1, move || {
            let freqs = read_frequencies().expect("unable to read frequencies");
            sender.send(CpuWM::Frequencies(freqs)).unwrap();

            // Compute utilizations
            let new_cputime = read_proc_stat().unwrap();
            let utilization_avg = new_cputime.0.utilization(cputime.0);
            sender.send(CpuWM::UtilizationAvg(utilization_avg)).unwrap();
            let mut utilizations = Vec::new();
            if new_cputime.1.len() != cores {}
            for i in 0..cores {
                utilizations.push(new_cputime.1[i].utilization(cputime.1[i]));
            }
            sender.send(CpuWM::Utilizations(utilizations)).unwrap();

            cputime = new_cputime;

            // Read boost state on intel CPUs
            sender
                .send(CpuWM::Turbo(
                    boost_status()
                        .map(|e| if e { TriBool::True } else { TriBool::False })
                        .unwrap_or(TriBool::Unknown),
                ))
                .unwrap();

            Continue(true)
        });

        Ok(())
    }

    fn get_channel(&self) -> (&SSender<Self::BM>, &MReceiver<Self::WM>) {
        self.dualchannel.get_reveled()
    }

    fn widget(&self) -> gtk::Widget {
        let holder = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).build();

        let label = gtk::Label::builder().label("CPU: ").build();

        let prefix = "CPU: ";
        let mut turbo_str = String::new();
        let mut freq = String::new();
        let mut label_str = String::new();

        let mut receiver = self.dualchannel.get_out_receiver();
        
        let series = Series::new("cpu", 100., 60, RGBA::WHITE, false);
        let chart = Chart::builder().height(20).width(60).with_series(series.clone(), DrawDirection::DownTop).build();
        holder.pack_start(&label, false, false, 0);
        holder.pack_end(&chart.drawing_area, false, false, 0);

        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                if let Ok(msg) = receiver.recv().await {
                    match msg {
                        CpuWM::Turbo(turbo) => {
                            let new = match turbo {
                                TriBool::True => "T",
                                TriBool::False => "",
                                TriBool::Unknown => "TError",
                            };

                            let new = format!("{} {}", prefix, new);
                            if new.as_str() != label_str.as_str() {
                                label.set_label(&new);
                            }
                            label_str = new;
                        },
                        CpuWM::Frequencies(freq) => {

                        },
                        CpuWM::UtilizationAvg(avg) => {
                            series.add_value(avg * 100.);                            
                        },
                        CpuWM::Utilizations(_) => {},
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