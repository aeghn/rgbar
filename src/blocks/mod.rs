use crate::statusbar::WidgetShareInfo;

mod audio;
pub mod battery;
pub mod cpu;
pub mod manager;
pub mod memory;
pub mod netspeed;

pub mod hyprstatus;
pub mod time;

pub trait Block {
    type Out;
    type In;

    fn run(&mut self) -> anyhow::Result<()>;

    fn widget(&self, share_info: &WidgetShareInfo) -> gtk::Widget;
}
