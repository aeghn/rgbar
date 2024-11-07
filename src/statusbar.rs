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
use crate::util::gdk_util::get_monitor_plug_name;

pub struct StatusBar {
    window_map: HashMap<i32, ApplicationWindow>,
    application: gtk::Application,
    block_manager: BlockManager,
}

#[derive(Clone, Default)]
pub struct WidgetShareInfo {
    pub monitor_num: i32,
    pub plug_name: Option<String>,
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

    pub fn new_window(&self, monitor_num: i32, plug_name: Option<String>) -> ApplicationWindow {
        let window = crate::window::create_window(&self.application, monitor_num.clone());

        let share_info = WidgetShareInfo {
            monitor_num,
            plug_name,
        };

        self.build_widgets(&window, share_info);

        window
    }

    pub fn handle_monitors(&mut self) {
        let screen = gdk::Screen::default().expect("Failed to get the default screen.");

        self.check_monitors(&screen);
    }

    pub fn check_monitors(&mut self, screen: &gdk::Screen) {
        let display = screen.display();
        for monitor_num in 0..display.n_monitors() {
            if self.window_map.contains_key(&monitor_num) {
                continue;
            }
            let plug_name = get_monitor_plug_name(&display, monitor_num).map(|e| e.to_string());

            let win = self.new_window(monitor_num, plug_name);

            self.window_map.insert(monitor_num, win);
        }

        let new_monitors: Vec<i32> = self.window_map.keys().map(|i| i.clone()).collect();
        for monitor_num in new_monitors {
            match display.monitor(monitor_num) {
                None => {
                    if let Some(win) = self.window_map.remove(&monitor_num) {
                        error!("destroy: {:?}", monitor_num);
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

        #[cfg(feature = "hyprland")]
        {
            let hyprstatus = self.block_manager.hypr_block.widget(share_info);
            bar.pack_start(&hyprstatus, false, false, 0);
        }

        let wayland = self.block_manager.wayland_block.widget(share_info);
        bar.pack_start(&wayland, false, false, 0);

        window.add(&bar);
        bar.show_all();

        let window = window.clone();
        glib::idle_add_local_once(move || {
            window.show();
        });
    }
}
