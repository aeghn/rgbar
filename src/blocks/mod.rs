pub mod time;
pub mod hyprstatus;
pub mod battery;
pub mod netspeed;




pub trait Module {
    fn into_widget(&self) -> gtk::Widget;
    fn put_into_bar(&self, bar: &gtk::Box);
}
