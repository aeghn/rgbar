use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use gtk::gdk::Monitor;
use log::info;

use crate::prelude::*;
use crate::util::gdk_util::get_monitor_plug_name;
use crate::window::RGBWindow;

use crate::blocks::manager::BlockManager;

pub struct RGBApplication {
    window_map: Rc<RefCell<HashMap<i32, RGBWindow>>>,
    application: gtk::Application,
    block_manager: BlockManager,
}

#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub monitor: Monitor,
    pub num: i32,
    pub plug_name: Option<String>,
}

impl RGBApplication {
    pub fn new(application: &gtk::Application) -> AResult<Self> {
        let block_manager = BlockManager::launch()?;

        Ok(RGBApplication {
            window_map: Default::default(),
            application: application.clone(),
            block_manager,
        })
    }

    fn init_window(
        app: &Rc<RefCell<RGBApplication>>,
        display: &Display,
        monitor: &Monitor,
        monitor_num: i32,
    ) {
        let app = app.clone();
        let display = display.clone();
        let monitor = monitor.clone();
        let mut count = 0;
        timeout_add_local(Duration::from_millis(256), move || {
            let plug_name = get_monitor_plug_name(&display, monitor_num).map(|e| e.to_string());

            if plug_name.is_none() && count < 8 {
                return ControlFlow::Continue;
            }

            count += 1;
            let monitor_info = MonitorInfo {
                monitor: monitor.clone(),
                num: monitor_num,
                plug_name,
            };

            let window = RGBWindow::new(&app.borrow().application, &monitor_info).unwrap();

            window.inject_widgets(&app.borrow().block_manager);

            ControlFlow::Break
        });
    }

    fn init_monitor(
        app: &Rc<RefCell<RGBApplication>>,
        display: &Display,
        monitor: Option<&Monitor>,
    ) {
        for monitor_num in 0..display.n_monitors() {
            let display = display.clone();
            let mon = display.monitor(monitor_num);

            if monitor == mon.as_ref() || monitor.is_none() {
                info!("init monitor: {}", monitor_num);
                if let Some(mon) = mon {
                    Self::init_window(app, &display, &mon, monitor_num);
                }
            }
        }
    }

    pub fn monitor_monitors(screen: &Screen, app: &Application) -> EResult {
        let app = RGBApplication::new(app).unwrap();
        let app: Rc<RefCell<RGBApplication>> = Rc::new(RefCell::new(app));

        let display = screen.display();

        Self::init_monitor(&app, &display, None);

        display.connect_monitor_added(move |display, monitor| {
            info!("display connected");
            Self::init_monitor(&app, display, Some(monitor));
        });

        Ok(())
    }
}
