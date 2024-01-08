use glib::MainContext;

use super::{
    netspeed::{self, NetspeedBlock, NetspeedBM, NetspeedWM},
    Block,
};

pub struct BlockManager {
    pub netspeed_worker: NetspeedBlock,
}

impl BlockManager {
    pub fn launch() -> BlockManager {
        let mut netspeed_worker = NetspeedBlock::new();
        netspeed_worker.loop_receive();


        BlockManager { netspeed_worker }
    }
}
