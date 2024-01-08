use std::{rc::Rc, cell::RefCell};

use anyhow::anyhow;
use async_broadcast::{Receiver, Sender};

pub struct DualChannel<OutMsg: Clone, InMsg> {
    pub out_sender: Sender<OutMsg>,
    pub out_recevier: Receiver<OutMsg>,
    pub in_sender: async_channel::Sender<InMsg>,
    pub in_recevier: async_channel::Receiver<InMsg>
}

impl<OutMsg: Clone, InMsg> DualChannel<OutMsg, InMsg> {
    pub fn new(
        cap: usize,
    ) -> Self {
        let (mut otx, orx) = async_broadcast::broadcast(cap);
        let (mut itx, irx) = async_channel::unbounded();
        otx.set_overflow(true);

        Self {
            out_sender: otx,
            out_recevier: orx,
            in_sender: itx,
            in_recevier: irx
        }
    }

    pub fn get_reveled(&self) -> (&SSender<InMsg>, &MReceiver<OutMsg>) {
        (&self.in_sender, &self.out_recevier)
    }

    pub fn output(&self, msg: OutMsg) -> anyhow::Result<Option<OutMsg>> {
        self.out_sender.try_broadcast(msg).map_err(|err| {anyhow!("unable to send: {}", err)})
    }

    pub async fn recv_async(&self) -> anyhow::Result<InMsg> {
        self.in_recevier.recv().await.map_err(|err| {anyhow!("unable to recevie: {}", err)})
    }

    pub fn get_out_sender(&self) -> MSender<OutMsg> {
        self.out_sender.clone().into()
    }

    pub fn get_out_receiver(&self) -> MReceiver<OutMsg> {
        self.out_recevier.clone().into()
    }

    pub fn get_in_recevier(&self) -> SReceiver<InMsg> {
        self.in_recevier.clone()
    }
}


#[derive(Clone)]
pub struct MSender<Msg: Clone> {
    sender: Sender<Msg>
}

impl <Msg: Clone> MSender<Msg> {
    pub fn send(&self, msg: Msg) -> anyhow::Result<Option<Msg>> {
        self.sender.try_broadcast(msg).map_err(|err| {anyhow!("unable to send: {}", err)})
    }
}

impl <Msg: Clone> From<async_broadcast::Sender<Msg>> for MSender<Msg> {
    fn from(value: async_broadcast::Sender<Msg>) -> Self {
        MSender { sender: value }
    }
}

pub type MReceiver<Msg: Clone> = async_broadcast::Receiver<Msg>;

pub type SReceiver<Msg> = async_channel::Receiver<Msg>;
pub type SSender<Msg> = async_channel::Sender<Msg>;

