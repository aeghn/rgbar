use crate::datahodler::channel::{SSender, MReceiver};

pub mod battery;
pub mod hyprstatus;
pub mod netspeed;
pub mod time;
mod audio;
pub mod manager;

pub trait BlockWidget {
    fn widget(&self) -> gtk::Widget;
    fn put_into_bar(&self, bar: &gtk::Box);
}

pub trait Block {
    type WM;
    type BM;

    fn loop_receive(&mut self);

    fn get_channel(&self) -> (&SSender<Self::BM>, &MReceiver<Self::WM>);

    fn widget(&self) -> gtk::Widget;
}