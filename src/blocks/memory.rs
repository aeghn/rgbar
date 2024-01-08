use std::cmp::min;
use std::str::FromStr;

use super::prelude::*;
use crate::util::read_file;

pub async fn run(config: &Config, api: &CommonApi) -> Result<()> {
    let mut actions = api.get_actions()?;
    api.set_default_actions(&[(MouseButton::Left, None, "toggle_format")])?;

    let mut format = config.format.with_default(
        " $icon $mem_used.eng(prefix:Mi)/$mem_total.eng(prefix:Mi)($mem_used_percents.eng(w:2)) ",
    )?;
    let mut format_alt = match &config.format_alt {
        Some(f) => Some(f.with_default("")?),
        None => None,
    };

    let mut timer = config.interval.timer();

    loop {
        let mem_state = Memstate::new().await?;

        let mem_total = mem_state.mem_total as f64 * 1024.;
        let mem_free = mem_state.mem_free as f64 * 1024.;

        // TODO: possibly remove this as it is confusing to have `mem_total_used` and `mem_used`
        // htop and such only display equivalent of `mem_used`
        let mem_total_used = mem_total - mem_free;

        // dev note: difference between avail and free:
        // https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id=34e431b0ae398fc54ea69ff85ec700722c9da773
        // same logic as htop
        let mem_avail = if mem_state.mem_available != 0 {
            min(mem_state.mem_available, mem_state.mem_total)
        } else {
            mem_state.mem_free
        } as f64
            * 1024.;

        // While zfs_arc_cache can be considered "available" memory,
        // it can only free a maximum of (zfs_arc_cache - zfs_arc_min) amount.
        // see https://github.com/htop-dev/htop/pull/1003
        let zfs_shrinkable_size = mem_state
            .zfs_arc_cache
            .saturating_sub(mem_state.zfs_arc_min) as f64;
        let mem_avail = mem_avail + zfs_shrinkable_size;

        let pagecache = mem_state.pagecache as f64 * 1024.;
        let reclaimable = mem_state.s_reclaimable as f64 * 1024.;
        let shmem = mem_state.shmem as f64 * 1024.;

        // See https://lore.kernel.org/lkml/1455827801-13082-1-git-send-email-hannes@cmpxchg.org/
        let cached = pagecache + reclaimable - shmem + zfs_shrinkable_size;

        let buffers = mem_state.buffers as f64 * 1024.;

        // same logic as htop
        let used_diff = mem_free + buffers + pagecache + reclaimable;
        let mem_used = if mem_total >= used_diff {
            mem_total - used_diff
        } else {
            mem_total - mem_free
        };

        // account for ZFS ARC cache
        let mem_used = mem_used - zfs_shrinkable_size;

        let swap_total = mem_state.swap_total as f64 * 1024.;
        let swap_free = mem_state.swap_free as f64 * 1024.;
        let swap_cached = mem_state.swap_cached as f64 * 1024.;
        let swap_used = swap_total - swap_free - swap_cached;

        let mem_state = match mem_used / mem_total * 100. {
            x if x > config.critical_mem => State::Critical,
            x if x > config.warning_mem => State::Warning,
            _ => State::Idle,
        };

        let swap_state = match swap_used / swap_total * 100. {
            x if x > config.critical_swap => State::Critical,
            x if x > config.warning_swap => State::Warning,
            _ => State::Idle,
        };
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
    async fn new() -> Result<Self> {
        // Reference: https://www.kernel.org/doc/Documentation/filesystems/proc.txt

        let mut file = BufReader::new(
            File::open("/proc/meminfo")
                .await
                .error("/proc/meminfo does not exist")?,
        );

        let mut mem_state = Memstate::default();
        let mut line = String::new();

        while file
            .read_line(&mut line)
            .await
            .error("failed to read /proc/meminfo")?
            != 0
        {
            let mut words = line.split_whitespace();

            let name = match words.next() {
                Some(name) => name,
                None => {
                    line.clear();
                    continue;
                }
            };
            let val = words
                .next()
                .and_then(|x| u64::from_str(x).ok())
                .error("failed to parse /proc/meminfo")?;

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

            line.clear();
        }

        // For ZFS
        if let Ok(arcstats) = read_file("/proc/spl/kstat/zfs/arcstats").await {
            let size_re = regex!(r"size\s+\d+\s+(\d+)");
            let size = &size_re
                .captures(&arcstats)
                .error("failed to find zfs_arc_cache size")?[1];
            mem_state.zfs_arc_cache = size.parse().error("failed to parse zfs_arc_cache size")?;
            let c_min_re = regex!(r"c_min\s+\d+\s+(\d+)");
            let c_min = &c_min_re
                .captures(&arcstats)
                .error("failed to find zfs_arc_min size")?[1];
            mem_state.zfs_arc_min = c_min.parse().error("failed to parse zfs_arc_min size")?;
        }

        Ok(mem_state)
    }
}
