use chrono::{DateTime, Local};
use glib::Continue;
use gtk::traits::BoxExt;
use gtk::traits::ButtonExt;
use gtk::traits::StyleContextExt;
use gtk::traits::WidgetExt;
use gtk::Widget;
use super::Module;

pub struct TimeModule {}

fn get_wes_time() -> String {
    let now: DateTime<Local> = Local::now();
    now.format("%y-%m-%d %H:%M:%S").to_string()
}

impl Module for TimeModule {
    fn to_widget(&self) -> Widget {
        let date = gtk::Button::with_label(&get_wes_time());
        date.style_context().add_class("block");

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        glib::timeout_add_seconds_local(3, move ||{
            let _date = Local::now();
            let _ = tx.send(format!("{}", get_wes_time()));
            Continue(true)
        });

        {
            let date = date.clone();
            rx.attach(None, move |s| {
                date.set_label(s.as_str());
                glib::Continue(true)
            });
        }

        glib::Cast::upcast::<Widget>(date)
    }

    fn put_into_bar(&self, bar: &gtk::Box) {
        bar.pack_end(&self.to_widget(), false, false, 0);
    }
}
