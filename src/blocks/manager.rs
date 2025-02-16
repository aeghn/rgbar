use std::sync::Arc;

use crate::config::Config;

use super::{
    audio::PulseBlock, battery::BatteryBlock, cpu::CpuBlock, memory::MemoryBlock,
    netspeed::NetspeedBlock, time::TimeBlock, wayland::WaylandBlock, Block,
};

pub struct BlockManager {
    pub net_block: NetspeedBlock,
    pub time_block: TimeBlock,
    pub cpu_block: CpuBlock,
    pub battery_block: BatteryBlock,
    pub memory_block: MemoryBlock,
    pub wayland_block: WaylandBlock,
    pub vol_block: PulseBlock,
}

impl BlockManager {
    pub fn launch() -> BlockManager {
        let mut net_block = NetspeedBlock::new();
        net_block.run().unwrap();

        let mut time_block = TimeBlock::new();
        time_block.run().unwrap();

        let mut cpu_block = CpuBlock::new();
        cpu_block.run().unwrap();

        let mut battery_block = BatteryBlock::new();
        battery_block.run().unwrap();

        let mut memory_block = MemoryBlock::new();
        memory_block.run().unwrap();

        let mut vol_block = PulseBlock::new();
        vol_block.run().unwrap();

        let mut wayland_block = WaylandBlock::new();
        wayland_block.run().unwrap();

        BlockManager {
            net_block,
            time_block,
            cpu_block,
            battery_block,
            memory_block,
            wayland_block,
            vol_block,
        }
    }
}
