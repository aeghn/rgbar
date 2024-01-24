use crate::datahodler::channel::{MReceiver, SSender};

mod audio;
pub mod battery;
pub mod cpu;
pub mod manager;
pub mod memory;
pub mod netspeed;

pub mod hyprstatus;
pub mod time;

pub trait BlockWidget {
    fn widget(&self) -> gtk::Widget;
    fn put_into_bar(&self, bar: &gtk::Box);
}

pub trait Block {
    type Out;
    type In;

    fn run(&mut self) -> anyhow::Result<()>;

    fn get_channel(&self) -> (&SSender<Self::In>, &MReceiver<Self::Out>);

    fn widget(&self) -> gtk::Widget;
}
