use crate::statusbar::WidgetShareInfo;

mod audio;
#[allow(dead_code)]
pub mod battery;
#[allow(dead_code)]
pub mod cpu;
pub mod manager;
#[allow(dead_code)]
pub mod memory;
pub mod netspeed;

pub mod hyprstatus;
pub mod time;
pub mod temp;

pub trait Block {
    type Out;
    type In;

    fn run(&mut self) -> anyhow::Result<()>;

    fn widget(&self, share_info: &WidgetShareInfo) -> gtk::Widget;
}
