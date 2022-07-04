pub mod time;
pub mod hyprstatus;
pub mod battery;
pub mod netspeed;

use glib::IsA;
use gtk::Widget;


pub trait Module<W>
    where W: IsA<Widget> {
    fn into_widget(self) -> W;
}
