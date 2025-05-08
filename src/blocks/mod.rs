use chin_tools::AResult;

use crate::window::WidgetShareInfo;

mod audio;
#[allow(dead_code)]
pub mod battery;
#[allow(dead_code)]
pub mod cpu;
pub mod manager;
#[allow(dead_code)]
pub mod memory;
pub mod netspeed;

#[cfg(feature = "hyprland")]
pub mod hyprstatus;
pub mod temp;
pub mod time;
pub mod wayland;

pub trait Block {
    type Out;
    type In;

    fn run(&mut self) -> AResult<()>;

    fn widget(&self, share_info: &WidgetShareInfo) -> gtk::Widget;
}
