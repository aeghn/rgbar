pub mod battery;
pub mod hyprstatus;
pub mod netspeed;
pub mod time;
mod audio;

pub trait Module {
    fn to_widget(&self) -> gtk::Widget;
    fn put_into_bar(&self, bar: &gtk::Box);
}
