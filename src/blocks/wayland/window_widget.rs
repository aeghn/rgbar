use chin_tools::wayland::{WLCompositor, WLWindow};
use chin_tools::wrapper::anyhow::AResult;
use gtk::{false_, Image, Label};

use std::collections::HashMap;

use crate::util;

use crate::util::gtk_icon_loader::GtkIconLoader;
use gtk::prelude::{ContainerExt, ImageExt, LabelExt, StackExt};
use gtk::traits::WidgetExt;
use gtk::traits::{BoxExt, StyleContextExt};

#[derive(Debug)]
pub struct WindowWidget {
    pub window: WLWindow,
    pub container: gtk::Box,
    pub title: Label,
    pub icon: Image,
}

impl WindowWidget {
    pub fn new(window: WLWindow, icon_loader: &GtkIconLoader) -> Self {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        let icon = gtk::Image::builder().build();
        icon.style_context().add_class("wm-cw-icon");

        let title = gtk::Label::builder().build();
        title.style_context().add_class("wm-cw-title");
        if window.is_focused() {
            title.set_label(
                window
                    .get_title()
                    .unwrap_or("Unknown Title".to_string())
                    .as_str(),
            );
        }

        title.set_single_line_mode(true);
        title.set_ellipsize(gdk::pango::EllipsizeMode::End);
        title.set_lines(1);
        title.set_line_wrap(true);
        title.set_line_wrap_mode(gdk::pango::WrapMode::Char);

        container.pack_start(&icon, false, false, 0);
        container.pack_start(&title, false, false, 0);
        container.show_all();

        if let Some(app_id) = window.get_app_id() {
            if let Some(img) = icon_loader.load_from_name(&app_id) {
                icon.set_from_pixbuf(Some(&img));
            } else {
                tracing::warn!("unable to get icon for {}", app_id);
            }
        }

        Self {
            window,
            container,
            title,
            icon,
        }
    }

    pub fn update_window(&mut self, window: WLWindow) {
        if let Some(title) = window.get_title() {
            self.title.set_label(&title);
        }
        self.window = window;
    }

    pub fn on_focus(&self, flag: bool) {
        tracing::error!(
            "on change focus {} {:?} -- {}",
            self.window.get_id(),
            self.window.get_title(),
            flag
        );
        if flag {
            self.title.set_label(
                self.window
                    .get_title()
                    .as_ref()
                    .map_or("Unknown Title", |v| v),
            );
            self.title.show();
        } else {
            self.title.set_label("");
            self.title.hide();
        }
    }
}

#[derive(Debug)]
pub struct WindowContainer {
    pub workspace_id: u64,
    pub widget_map: HashMap<u64, WindowWidget>,
    pub container: gtk::Box,
    pub focused_id: Option<u64>,
    pub icon_loader: GtkIconLoader,
}

impl WindowContainer {
    pub fn new(workspace_id: u64) -> Self {
        Self {
            widget_map: Default::default(),
            container: gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .build(),
            focused_id: None,
            icon_loader: util::gtk_icon_loader::GtkIconLoader::new(),
            workspace_id,
        }
    }

    pub fn on_window_overwrite(&mut self, window: WLWindow) {
        tracing::debug!("{:?}", window);
        let mut change_focus = None;
        if let Some(win) = self.widget_map.get_mut(&window.get_id()) {
            if window.is_focused() && !win.window.is_focused() {
                change_focus = Some(window.get_id())
            }
            if win.window.get_title() != window.get_title() {
                win.update_window(window);
            }
        } else {
            if window.is_focused() {
                change_focus = Some(window.get_id())
            }
            let window_widget = WindowWidget::new(window, &self.icon_loader);
            self.container.add(&window_widget.container);
            self.widget_map
                .insert(window_widget.window.get_id(), window_widget);

            self.on_reorder()
        }

        if change_focus.is_some() {
            self.on_change_focus(change_focus)
        }
    }

    pub fn on_window_delete(&mut self, window: u64) {
        tracing::debug!("{:?}", window);
        if let Some(window) = self.widget_map.remove(&window) {
            self.container.remove(&window.container);
        }
    }

    pub fn on_change_focus(&mut self, id: Option<u64>) {
        tracing::debug!("{:?}", id);
        if self.focused_id != id {
            if let Some(id) = self.focused_id {
                if let Some(w) = self.widget_map.get(&id) {
                    w.on_focus(false);
                }
            }
            if let Some(id) = id {
                if let Some(w) = self.widget_map.get(&id) {
                    w.on_focus(true);
                }
            }
        }
        self.focused_id = id;
    }

    pub fn on_reorder(&self) {
        tracing::debug!("reorder");
        let mut ids: Vec<(u64, &gtk::Box)> = self
            .widget_map
            .iter()
            .map(|(i, w)| (*i, &w.container))
            .collect();

        ids.sort_by(|e1, e2| e1.0.cmp(&e2.0));

        for (id, (_, b)) in ids.into_iter().enumerate() {
            self.container.reorder_child(b, id as i32);
        }
    }

    pub fn deal_with_window_id<F>(&self, id: u64, func: F)
    where
        F: Fn(&WindowWidget),
    {
        if let Some(win) = self.widget_map.get(&id) {
            func(win)
        }
    }
}

pub struct WindowContainerManager {
    pub stack: gtk::Stack,
    pub containers: HashMap<u64, WindowContainer>,
    pub current_window_id: Option<u64>,
    pub is_hide: bool,
}

impl WindowContainerManager {
    pub fn new() -> AResult<Self> {
        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::SlideUpDown)
            .build();

        let current_window_id = None;
        let containers: HashMap<u64, WindowContainer> = Default::default();

        Ok(Self {
            stack,
            containers,
            current_window_id,
            is_hide: false,
        })
    }

    pub fn init(mut self) -> AResult<Self> {
        let mut current_window_id = None;
        let mut containers: HashMap<u64, WindowContainer> = Default::default();

        for window in WLCompositor::current()?.get_all_windows()? {
            if let Some(wsid) = window.get_workspace_id() {
                if window.is_focused() {
                    current_window_id.replace(window.get_id());
                }
                containers
                    .entry(wsid)
                    .or_insert(WindowContainer::new(wsid))
                    .on_window_overwrite(window);
            }
        }

        self.current_window_id = current_window_id;
        for (_, w) in containers.into_iter() {
            self.on_workspace_overwrite(w);
        }

        Ok(self)
    }

    pub fn on_workspace_overwrite(&mut self, container: WindowContainer) {
        tracing::debug!("{:?}", container);
        self.on_workspace_delete(container.workspace_id);
        self.stack.add_named(
            &container.container,
            container.workspace_id.to_string().as_str(),
        );
        self.containers.insert(container.workspace_id, container);
    }

    pub fn on_workspace_delete(&mut self, workspace_id: u64) {
        tracing::debug!("{:?}", workspace_id);
        if let Some(old) = self.containers.remove(&workspace_id) {
            self.stack.remove(&old.container);
        }
    }

    pub fn on_workspace_change(&mut self, workspace_id: u64) {
        tracing::debug!("{:?}", workspace_id);
        if self.containers.contains_key(&workspace_id) {
            self.stack
                .set_visible_child_name(workspace_id.to_string().as_str());
            if self.is_hide {
                self.stack.show();
            }
        } else {
            self.stack.hide();
            self.is_hide = true;
        }
    }

    pub fn on_window_delete(&mut self, window_id: u64) {
        tracing::debug!("{:?}", window_id);

        for (_, wc) in self.containers.iter_mut() {
            wc.on_window_delete(window_id);
        }
    }

    pub fn on_window_overwrite(&mut self, window: WLWindow) {
        tracing::debug!("{:?}", window);

        if let Some(wc) = window
            .get_workspace_id()
            .and_then(|w| self.containers.get_mut(&w))
        {
            wc.on_window_overwrite(window);
        }
    }

    pub fn on_window_change_focus(&mut self, window: Option<WLWindow>) {
        tracing::debug!("{:?}", window);

        for (_, wc) in self.containers.iter_mut() {
            self.current_window_id.map(|id| {
                wc.deal_with_window_id(id, |e| {
                    e.on_focus(false);
                })
            });
        }

        if let Some(ww) = window {
            if let Some(wsid) = ww.get_workspace_id() {
                self.containers
                    .get_mut(&wsid)
                    .map(|e| e.on_change_focus(Some(ww.get_id())));
            }
        }
    }
}
