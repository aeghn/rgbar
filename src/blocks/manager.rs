use super::{
    battery::BatteryBlock, cpu::CpuBlock, memory::MemoryBlock, netspeed::NetspeedBlock,
    time::TimeBlock, Block,
};

pub struct BlockManager {
    pub net_block: NetspeedBlock,
    pub time_block: TimeBlock,
    pub cpu_block: CpuBlock,
    pub battery_block: BatteryBlock,
    pub memory_block: MemoryBlock,
}

impl BlockManager {
    pub fn launch() -> BlockManager {
        let mut net_block = NetspeedBlock::new();
        net_block.loop_receive().unwrap();

        let mut time_block = TimeBlock::new();
        time_block.loop_receive().unwrap();

        let mut cpu_block = CpuBlock::new();
        cpu_block.loop_receive().unwrap();

        let mut battery_block = BatteryBlock::new();
        battery_block.loop_receive().unwrap();

        let mut memory_block = MemoryBlock::new();
        memory_block.loop_receive().unwrap();

        BlockManager {
            net_block,
            time_block,
            cpu_block,
            battery_block,
            memory_block,
        }
    }
}
