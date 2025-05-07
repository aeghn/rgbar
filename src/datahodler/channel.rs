use async_broadcast::{Receiver, Sender};
use chin_tools::{ anyhow::aanyhow, AResult};

#[derive(Clone)]
pub struct DualChannel<OutMsg: Clone, InMsg: Clone> {
    pub out_sender: Sender<OutMsg>,
    pub out_recevier: Receiver<OutMsg>,
    pub in_sender: async_channel::Sender<InMsg>,
    pub in_recevier: async_channel::Receiver<InMsg>,
}

impl<OutMsg: Clone, InMsg: Clone> DualChannel<OutMsg, InMsg> {
    pub fn new(cap: usize) -> Self {
        let (mut otx, orx) = async_broadcast::broadcast(cap);
        let (itx, irx) = async_channel::unbounded();
        otx.set_overflow(true);

        Self {
            out_sender: otx,
            out_recevier: orx,
            in_sender: itx,
            in_recevier: irx,
        }
    }

    pub fn get_out_sender(&self) -> MSender<OutMsg> {
        self.out_sender.clone().into()
    }

    pub fn get_out_receiver(&self) -> MReceiver<OutMsg> {
        self.out_recevier.clone().into()
    }

    pub fn get_in_receiver(&self) -> SReceiver<InMsg> {
        self.in_recevier.clone()
    }

    pub fn get_in_sender(&self) -> SSender<InMsg> {
        self.in_sender.clone()
    }
}

#[derive(Clone)]
pub struct MSender<Msg: Clone> {
    sender: Sender<Msg>,
}

impl<Msg: Clone> MSender<Msg> {
    pub fn send(&self, msg: Msg) -> AResult<Option<Msg>> {
        self.sender
            .try_broadcast(msg)
            .map_err(|err| aanyhow!("unable to send: {}", err))
    }
}

impl<Msg: Clone> From<async_broadcast::Sender<Msg>> for MSender<Msg> {
    fn from(value: async_broadcast::Sender<Msg>) -> Self {
        MSender { sender: value }
    }
}

pub type MReceiver<Msg> = async_broadcast::Receiver<Msg>;

pub type SReceiver<Msg> = async_channel::Receiver<Msg>;
pub type SSender<Msg> = async_channel::Sender<Msg>;
