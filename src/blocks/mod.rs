use crate::datahodler::channel::{MReceiver, SSender};

mod audio;
pub mod battery;
pub mod cpu;
pub mod manager;
pub mod memory;
pub mod netspeed;
pub mod time;

pub trait BlockWidget {
    fn widget(&self) -> gtk::Widget;
    fn put_into_bar(&self, bar: &gtk::Box);
}

pub trait Block {
    type WM;
    type BM;

    fn loop_receive(&mut self) -> anyhow::Result<()>;

    fn get_channel(&self) -> (&SSender<Self::BM>, &MReceiver<Self::WM>);

    fn widget(&self) -> gtk::Widget;
}
