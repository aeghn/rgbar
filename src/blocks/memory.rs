use std::cmp::min;
use std::str::FromStr;

use crate::prelude::*;
use chin_tools::AResult;



use crate::statusbar::WidgetShareInfo;
use crate::util::gtk_icon_loader::StatusName;
use crate::util::{fileutil, gtk_icon_loader};
use crate::widgets::chart::{Chart, Column};

use super::Block;

#[derive(Clone)]
pub enum MemoryOut {
    MemoryUsedAndCache(usize, usize, usize), // USED / Cache / total
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

    fn run(&mut self) -> AResult<()> {
        let sender = self.dualchannel.get_out_sender();

        timeout_add_seconds_local(1, move || {
            let mem_state = Memstate::new().unwrap();

            let mem_total = mem_state.mem_total * 1024;

            // TODO: possibly remove this as it is confusing to have `mem_total_used` and `mem_used`
            // htop and such only display equivalent of `mem_used`
            let mem_used = mem_total - mem_state.mem_available * 1024;
            let mem_cache = mem_state.pagecache * 1024;

            sender
                .send(MemoryOut::MemoryUsedAndCache(
                    mem_used, mem_cache, mem_total,
                ))
                .unwrap();

            // dev note: difference between avail and free:
            // https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=34e431b0ae398fc54ea69ff85ec700722c9da773
            // same logic as htop
            let _mem_avail = ((if mem_state.mem_available != 0 {
                min(mem_state.mem_available, mem_state.mem_total)
            } else {
                mem_state.mem_free
            }) as f64)
                * 1024.0;

            let swap_total = mem_state.swap_total * 1024;
            let swap_free = mem_state.swap_free * 1024;
            let swap_cached = mem_state.swap_cached * 1024;
            let _swap_used = swap_total - swap_free - swap_cached;

            ControlFlow::Continue
        });

        Ok(())
    }

    fn widget(&self, _share_info: &WidgetShareInfo) -> gtk::Widget {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(false)
            .build();

        let icon = gtk_icon_loader::load_fixed_status_image(StatusName::RAM);

        let mut receiver = self.dualchannel.get_out_receiver();

        let mem_columns = Column::new("mem", 100.0, 30, RGBA::new(0.2, 0.2, 0.2, 0.6));
        let cache_columns = Column::new("cache", 100.0, 30, RGBA::new(0.5, 0.5, 0.5, 0.6));
        let chart = Chart::builder()
            .with_width(30)
            .with_line_width(1.0)
            .with_columns(mem_columns.clone())
            .with_columns(cache_columns.clone());
        chart.draw_in_seconds(1);

        holder.pack_start(&icon, false, false, 0);
        holder.pack_end(&chart.drawing_box, false, false, 0);

        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                if let Ok(msg) = receiver.recv().await {
                    match msg {
                        MemoryOut::MemoryUsedAndCache(used, cache, total) => {
                            cache_columns.add_value(((cache * 100) / total) as f64);
                            mem_columns.add_value(((used * 100) / total) as f64);
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
    mem_total: usize,
    mem_free: usize,
    mem_available: usize,
    buffers: usize,
    pagecache: usize,
    s_reclaimable: usize,
    shmem: usize,
    swap_total: usize,
    swap_free: usize,
    swap_cached: usize,
    zfs_arc_cache: usize,
    zfs_arc_min: usize,
}

impl Memstate {
    fn new() -> AResult<Self> {
        // Reference: https://www.kernel.org/doc/Documentation/filesystems/proc.txt

        let mut mem_state = Memstate::default();

        fileutil::read_lines("/proc/meminfo")
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
                    .and_then(|x| usize::from_str(x).ok())
                    .expect("failed to parse /proc/meminfo");

                match name {
                    "MemTotal:" => {
                        mem_state.mem_total = val;
                    }
                    "MemFree:" => {
                        mem_state.mem_free = val;
                    }
                    "MemAvailable:" => {
                        mem_state.mem_available = val;
                    }
                    "Buffers:" => {
                        mem_state.buffers = val;
                    }
                    "Cached:" => {
                        mem_state.pagecache = val;
                    }
                    "SReclaimable:" => {
                        mem_state.s_reclaimable = val;
                    }
                    "Shmem:" => {
                        mem_state.shmem = val;
                    }
                    "SwapTotal:" => {
                        mem_state.swap_total = val;
                    }
                    "SwapFree:" => {
                        mem_state.swap_free = val;
                    }
                    "SwapCached:" => {
                        mem_state.swap_cached = val;
                    }
                    _ => (),
                }
            });
        Ok(mem_state)
    }
}
