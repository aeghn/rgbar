use gdk::RGBA;
use glib::Continue;

use std::collections::HashMap;

use gtk::prelude::{GtkWindowExt, WidgetExtManual, BoxExt};
use gtk::traits::ContainerExt;
use gtk::traits::StyleContextExt;
use gtk::traits::WidgetExt;
use gtk::ApplicationWindow;
use gtk::Orientation;
use tracing::error;

use crate::blocks::manager::BlockManager;
use crate::{blocks, widgets};
use crate::blocks::{BlockWidget, Block};

pub struct StatusBar {
    window_map: HashMap<i32, ApplicationWindow>,
    application: gtk::Application,
    block_manager: BlockManager
}

impl StatusBar {
    pub fn new(application: &gtk::Application) -> Self {
        let mut vec: Vec<Box<dyn BlockWidget>> = vec![];

        let time_but = blocks::time::TimeModule {};
        vec.push(Box::new(time_but));
       // let hypr_box = blocks::hyprstatus::HyprStatus {};
       // vec.push(Box::new(hypr_box));
       
       let block_manager = BlockManager::launch();

        StatusBar {
            window_map: HashMap::new(),
            application: application.clone(),
            block_manager
        }
    }

    pub fn new_window(&self, monitor_num: i32) -> ApplicationWindow {
        let window = crate::window::create_window(&self.application, monitor_num);

        self.build_widgets(&window);

        window
    }

    pub fn handle_monitors(&mut self) {
        let screen = gdk::Screen::default().expect("Failed to get the default screen.");

        self.check_monitors(&screen);
    }

    pub fn check_monitors(&mut self, screen: &gdk::Screen) {
        let monitor_count = screen.display().n_monitors();
        for i in 0..monitor_count {
            if self.window_map.contains_key(&i) {
                continue;
            }

            error!("new window {:?}", i);
            let win = self.new_window(i);

            self.window_map.insert(i, win);
        }

        let new_keys: Vec<i32> = self.window_map.keys().map(|i| i.clone()).collect();
        for key in new_keys {
            match screen.display().monitor(key) {
                None => {
                    if let Some(win) = self.window_map.remove(&key) {
                        error!("destroy: {:?}", key);
                        win.close();
                    }
                },
                Some(_) => {}
            }
        }
    }

    fn build_widgets(&self, window: &ApplicationWindow) {
        let bar = gtk::Box::new(Orientation::Horizontal, 10);
        bar.style_context().add_class("bar");

        let netspeed = self.block_manager.netspeed_worker.widget();
        bar.pack_start(&netspeed, false, false, 3);

/*         let upload_serie = Series::new("upload", 3000, 120, RGBA::new(1.0, 0., 0., 0.), SeriesType::Line);
        
        {
            let usender = upload_serie.sender.clone();
            let (utx, urx) = flume::unbounded();
            let (dtx, drx) = flume::unbounded();
            blocks::netspeed(&utx, &dtx);

            glib::MainContext::ref_thread_default().spawn_local(async move {
                loop {
                    match urx.recv_async().await {
                        Ok(msg) => {
                            usender.send(widgets::chart::SeriesMsg::AddValue(msg)).unwrap();
                        },
                        Err(_) => todo!(),
                    }
                }
            });
        }

        {
            let us = upload_serie.clone();

            glib::MainContext::ref_thread_default().spawn_local(async move {
                us.loop_receive().await
            });
        } */
/* 
        let net_box = widgets::chart::Chart::new(vec![upload_serie]);
        bar.pack_end(&net_box.drawing_area, false, true, 0); */

        window.add(&bar);

        let window = window.clone();
        glib::idle_add_local(move || {
            window.show_all();
            Continue(false)
        });
    }
}
