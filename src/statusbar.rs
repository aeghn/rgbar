use std::collections::HashMap;

use gtk::prelude::{BoxExt, GtkWindowExt};
use gtk::traits::ContainerExt;
use gtk::traits::StyleContextExt;
use gtk::traits::WidgetExt;
use gtk::ApplicationWindow;
use gtk::Orientation;
use tracing::error;

use crate::blocks::manager::BlockManager;
use crate::blocks::Block;

pub struct StatusBar {
    window_map: HashMap<i32, ApplicationWindow>,
    application: gtk::Application,
    block_manager: BlockManager,
}

#[derive(Clone, Default)]
pub struct WidgetShareInfo {
    pub monitor: i32,
}

impl StatusBar {
    pub fn new(application: &gtk::Application) -> Self {
        let block_manager = BlockManager::launch();

        StatusBar {
            window_map: HashMap::new(),
            application: application.clone(),
            block_manager,
        }
    }

    pub fn new_window(&self, monitor_num: i32) -> ApplicationWindow {
        let window = crate::window::create_window(&self.application, monitor_num.clone());

        let mut share_info = WidgetShareInfo::default();

        share_info.monitor = monitor_num.clone();

        self.build_widgets(&window, share_info);

        window
    }

    pub fn handle_monitors(&mut self) {
        let screen = gdk::Screen::default().expect("Failed to get the default screen.");

        self.check_monitors(&screen);
    }

    pub fn check_monitors(&mut self, screen: &gdk::Screen) {
        tracing::info!("Screen {:?}", screen);
        let monitor_count = screen.display().n_monitors();
        for i in 0..monitor_count {
            if self.window_map.contains_key(&i) {
                continue;
            }

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
                }
                Some(_) => {}
            }
        }
    }

    fn build_widgets(&self, window: &ApplicationWindow, share_info: WidgetShareInfo) {
        let bar = gtk::Box::new(Orientation::Horizontal, 10);

        bar.style_context().add_class("bar");
        let share_info = &share_info;

        let time = self.block_manager.time_block.widget(share_info);
        time.style_context().add_class("block");
        bar.pack_end(&time, false, false, 0);

        let battery = self.block_manager.battery_block.widget(share_info);
        battery.style_context().add_class("block");
        bar.pack_end(&battery, false, false, 0);

        let volume = self.block_manager.vol_block.widget(share_info);
        volume.style_context().add_class("block");
        bar.pack_end(&volume, false, false, 0);

        let cpu = self.block_manager.cpu_block.widget(share_info);
        cpu.style_context().add_class("block");
        bar.pack_end(&cpu, false, false, 0);

        let memory = self.block_manager.memory_block.widget(share_info);
        memory.style_context().add_class("block");
        bar.pack_end(&memory, false, false, 0);

        let netspeed = self.block_manager.net_block.widget(share_info);
        netspeed.style_context().add_class("block");
        bar.pack_end(&netspeed, false, false, 0);

        let hyprstatus = self.block_manager.hypr_block.widget(share_info);
        bar.pack_start(&hyprstatus, false, false, 0);

        window.add(&bar);
        bar.show_all();

        let window = window.clone();
        glib::idle_add_local_once(move || {
            window.show();
        });
    }
}
