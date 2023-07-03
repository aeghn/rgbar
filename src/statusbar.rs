use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk::ApplicationWindow;
use gtk::Orientation;
use gtk::traits::ContainerExt;
use gtk::traits::StyleContextExt;
use gtk::traits::WidgetExt;


use crate::blocks;
use crate::blocks::Module;


pub struct StatusBar {
    window_map: HashMap<i32, ApplicationWindow>,
    modules: Vec<Box<dyn Module>>,
    application: gtk::Application
}

impl StatusBar {
    pub fn new(application: &gtk::Application) -> Self {
        let mut vec: Vec<Box<dyn Module>> = vec![];

        let time_but = blocks::time::TimeModule{};
        vec.push(Box::new(time_but));
         let hypr_box =  blocks::hyprstatus::HyprStatus{};
        vec.push(Box::new(hypr_box ));
        let bat_box = blocks::battery::BatteryModule{};
        vec.push(Box::new(bat_box));
        let net_box = blocks::netspeed::NetspeedModule{};  
        vec.push(Box::new(net_box));

        StatusBar { 
            window_map: HashMap::new(), 
            modules: vec,
            application: application.clone(),
        }
    }

    pub fn new_window(&self, monitor_num: i32) -> ApplicationWindow {
        let window = crate::window::create_window(&self.application, monitor_num);
        
        self.build_widgets(&window);

        window
    }

    pub fn handle_monitors(&mut self) {
        let screen = gdk::Screen::default().expect("Failed to get the default screen.");
    
        let monitor_count = screen.display().n_monitors();
        self.check_monitors(monitor_count);
    }
    

    pub fn check_monitors(&mut self, monitor_count: i32) {
        for i in 0..monitor_count {
            if self.window_map.contains_key(&i) {
                continue;
            }

            let win = self.new_window(i);

            self.window_map.insert(i, win);
        }
    }

    fn build_widgets(&self, window: &ApplicationWindow) {
        let bar = gtk::Box::new(Orientation::Horizontal, 10);
        bar.style_context().add_class("bar");
        
        self.modules.iter().for_each(|m| {
            m.put_into_bar(&bar);
        });
    
        window.add(&bar);
    
        window.show_all();
    }
}