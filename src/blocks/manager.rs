use super::{
    netspeed::NetspeedBlock,
    Block, time::TimeBlock, cpu::CpuBlock, battery::BatteryModule,
};

pub struct BlockManager {
    pub net_block: NetspeedBlock,
    pub time_block: TimeBlock,
    pub cpu_block: CpuBlock,
    pub battery_block: BatteryModule,
}

impl BlockManager {
    pub fn launch() -> BlockManager {
        let mut net_block = NetspeedBlock::new();
        net_block.loop_receive();

        let mut time_block = TimeBlock::new();
        time_block.loop_receive();

        let mut cpu_block = CpuBlock::new();
        cpu_block.loop_receive();

        let mut battery_block = BatteryModule::new();
        battery_block.loop_receive();

        BlockManager { net_block, time_block, cpu_block, battery_block }
    }
}