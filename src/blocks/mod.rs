pub mod time;
pub mod battery;
pub mod netspeed;
pub mod hyprstatus;


pub trait Module {
    fn to_widget(&self) -> gtk::Widget;
    fn put_into_bar(&self, bar: &gtk::Box);
}
