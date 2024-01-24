use std::cmp::min;
use std::str::FromStr;

use anyhow::Result;
use gdk::RGBA;
use glib::{Cast, MainContext};
use gtk::prelude::{BoxExt, StyleContextExt, WidgetExt};

use crate::datahodler::channel::DualChannel;
use crate::utils::gtk_icon_loader::IconName;
use crate::utils::{fileutils, gtk_icon_loader};
use crate::widgets::chart::{Chart, DrawDirection, LineType, Series};

use super::Block;

#[derive(Clone)]
pub enum MemoryOut {
    MemoryInfo(u64, u64), // USED / TOTAL
}

#[derive(Clone)]
pub enum MemoryIn {}

pub struct MemoryBlock {
    dualchannel: DualChannel<MemoryOut, MemoryIn>,
}

impl MemoryBlock {
    pub fn new() -> Self {
        MemoryBlock {
            dualchannel: DualChannel::new(100),
        }
    }
}

impl Block for MemoryBlock {
    type Out = MemoryOut;

    type In = MemoryIn;

    fn run(&mut self) -> anyhow::Result<()> {
        let sender = self.dualchannel.get_out_sender();

        glib::timeout_add_seconds_local(1, move || {
            let mem_state = Memstate::new().unwrap();

            let mem_total = mem_state.mem_total * 1024;
            let mem_free = mem_state.mem_free * 1024;

            // TODO: possibly remove this as it is confusing to have `mem_total_used` and `mem_used`
            // htop and such only display equivalent of `mem_used`
            let mem_total_used = mem_total - mem_free;

            sender
                .send(MemoryOut::MemoryInfo(mem_total, mem_total_used))
                .unwrap();

            // dev note: difference between avail and free:
            // https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=34e431b0ae398fc54ea69ff85ec700722c9da773
            // same logic as htop
            let _mem_avail = if mem_state.mem_available != 0 {
                min(mem_state.mem_available, mem_state.mem_total)
            } else {
                mem_state.mem_free
            } as f64
                * 1024.;

            let swap_total = mem_state.swap_total * 1024;
            let swap_free = mem_state.swap_free * 1024;
            let swap_cached = mem_state.swap_cached * 1024;
            let _swap_used = swap_total - swap_free - swap_cached;

            glib::ControlFlow::Continue
        });

        Ok(())
    }

    fn get_channel(
        &self,
    ) -> (
        &crate::datahodler::channel::SSender<Self::In>,
        &crate::datahodler::channel::MReceiver<Self::Out>,
    ) {
        todo!()
    }

    fn widget(&self) -> gtk::Widget {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(false)
            .build();

        let image = gtk_icon_loader::load_image(IconName::RAM);

        let label = gtk::Label::builder().label("MEM: ").build();
        label.style_context().add_class("cpu-mem-label");

        let _label_str = String::new();

        let mut receiver = self.dualchannel.get_out_receiver();

        let series = Series::new("mem", 100., 40, RGBA::new(1.0, 1.0, 0.3, 1.), false);
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
                        MemoryOut::MemoryInfo(total, used) => {
                            series.add_value((used * 100 / total) as f64);
                        }
                    }
                }
            }
        });

        holder.upcast()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Memstate {
    mem_total: u64,
    mem_free: u64,
    mem_available: u64,
    buffers: u64,
    pagecache: u64,
    s_reclaimable: u64,
    shmem: u64,
    swap_total: u64,
    swap_free: u64,
    swap_cached: u64,
    zfs_arc_cache: u64,
    zfs_arc_min: u64,
}

impl Memstate {
    fn new() -> Result<Self> {
        // Reference: https://www.kernel.org/doc/Documentation/filesystems/proc.txt

        let mut mem_state = Memstate::default();

        fileutils::read_lines("/proc/meminfo")
            .expect("unable to open /proc/meminfo ?")
            .for_each(|line| {
                let line = line.unwrap_or("".to_string());

                let mut words = line.split_whitespace();

                let name = match words.next() {
                    Some(name) => name,
                    None => {
                        return;
                    }
                };
                let val = words
                    .next()
                    .and_then(|x| u64::from_str(x).ok())
                    .expect("failed to parse /proc/meminfo");

                match name {
                    "MemTotal:" => mem_state.mem_total = val,
                    "MemFree:" => mem_state.mem_free = val,
                    "MemAvailable:" => mem_state.mem_available = val,
                    "Buffers:" => mem_state.buffers = val,
                    "Cached:" => mem_state.pagecache = val,
                    "SReclaimable:" => mem_state.s_reclaimable = val,
                    "Shmem:" => mem_state.shmem = val,
                    "SwapTotal:" => mem_state.swap_total = val,
                    "SwapFree:" => mem_state.swap_free = val,
                    "SwapCached:" => mem_state.swap_cached = val,
                    _ => (),
                }
            });
        Ok(mem_state)
    }
}
