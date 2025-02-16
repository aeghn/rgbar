use chin_tools::wayland::{WLCompositor, WLWindow};
use chin_tools::wrapper::anyhow::AResult;
use gdk::glib::Propagation;
use gtk::Label;

use std::collections::HashMap;
use std::sync::Arc;

use crate::config::Config;
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
}

impl WindowWidget {
    pub fn new(window: WLWindow, icon_loader: &GtkIconLoader) -> Self {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        let icon = gtk::Image::builder().build();
        icon.style_context().add_class("wmw-icon");

        let event_box = gtk::EventBox::builder().child(&icon).build();

        let title = gtk::Label::builder().build();
        title.style_context().add_class("wmw-title");
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

        container.pack_start(&event_box, false, false, 0);
        container.pack_start(&title, false, false, 0);
        container.show_all();

        if let Some(app_id) = window.get_app_id() {
            if let Some(img) = icon_loader.load_from_name(&app_id) {
                icon.set_from_pixbuf(Some(&img));
            } else {
                tracing::warn!("unable to get icon for {}", app_id);
            }
        }

        {
            let window = window.clone();
            event_box.connect_button_release_event(move |_, event| match event.button() {
                1 => {
                    let _ = window.focus();
                    Propagation::Stop
                }
                _ => Propagation::Proceed,
            });
        }

        Self {
            window,
            container,
            title,
        }
    }

    pub fn update_window(&mut self, window: WLWindow) {
        if let Some(title) = window.get_title() {
            self.title.set_label(&title);
        }
        self.window = window;
    }

    pub fn on_focus(&self, flag: bool) {
        if flag {
            self.title.set_label(
                self.window
                    .get_title()
                    .as_ref()
                    .map_or("Unknown Title", |v| v),
            );
            self.title.show();
            self.container.style_context().add_class("wmw-focus")
        } else {
            self.title.set_label("");
            self.title.hide();
            self.container.style_context().remove_class("wmw-focus")
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
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        // https://stackoverflow.com/questions/50120555/gtk-stack-wont-change-visible-child-inside-an-event-callback-function
        // > I do not have Granite installed so I can't reproduce the given example. Does Granite.Widgets.Welcome get shown after instantiation? If not, and I quote, "Note that the child widget has to be visible itself (see show) in order to become the visible child of this.". Try to instantiate it first, call show on it and then add it to the Gtk.Stack. It should work.
        container.show_all();
        Self {
            widget_map: Default::default(),
            container,
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
        if self.focused_id != id {
            if let Some(id) = self.focused_id {
                if let Some(w) = self.widget_map.get(&id) {
                    w.on_focus(false);
                }
            }
        }
        if let Some(id) = id {
            if let Some(w) = self.widget_map.get(&id) {
                w.on_focus(true);
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
    pub workspace_containers: HashMap<u64, WindowContainer>,
    pub current_window_id: Option<u64>,
    pub current_workspace_id: i64,
}

impl WindowContainerManager {
    pub fn new() -> AResult<Self> {
        let stack = gtk::Stack::builder()
            .build();

        stack.add_named(
            &gtk::Label::new(Some("This workspace's windows container is missing.")),
            "missing",
        );

        let current_window_id = None;
        let containers: HashMap<u64, WindowContainer> = Default::default();

        Ok(Self {
            stack,
            workspace_containers: containers,
            current_window_id,
            current_workspace_id: -1,
        })
    }

    pub fn init(mut self, workspace_ids: Vec<u64>) -> AResult<Self> {
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

        for wsid in workspace_ids {
            if !containers.contains_key(&wsid) {
                containers.insert(wsid, WindowContainer::new(wsid));
            }
        }

        self.current_window_id = current_window_id;
        for (_, w) in containers.into_iter() {
            self.on_workspace_overwrite(w);
        }

        Ok(self)
    }

    pub fn on_workspace_overwrite(&mut self, container: WindowContainer) {
        tracing::debug!("[WIN] workspace overwrite {:?}", container.workspace_id);
        let cont = container.container.clone();
        let wsid = container.workspace_id;
        let stack = self.stack.clone();

        stack.add_named(&cont, wsid.to_string().as_str());

        self.workspace_containers
            .insert(container.workspace_id, container);
    }

    pub fn on_workspace_delete(&mut self, workspace_id: u64) {
        tracing::debug!("[WIN] workspace delete {:?}", workspace_id);
        if let Some(old) = self.workspace_containers.remove(&workspace_id) {
            self.stack.remove(&old.container);
        }
    }

    pub fn on_workspace_change(&mut self, workspace_id: u64) {
        tracing::debug!("workspace change {:?}", workspace_id);
        if self.current_workspace_id != workspace_id as i64 {
            if !self.workspace_containers.contains_key(&workspace_id) {
                tracing::warn!(
                    "A workspace should create first before visit it {}",
                    workspace_id
                );

                self.on_workspace_overwrite(WindowContainer::new(workspace_id));
            }

            if let Some(_) = self.workspace_containers.get(&workspace_id) {
                let stack = self.stack.clone();
                glib::idle_add_local_once(move || {
                    tracing::debug!("[WIN] set visble child: {}", workspace_id);
                    stack.set_visible_child_name(&workspace_id.to_string().as_str());
                });
                self.current_workspace_id = workspace_id as i64;
            }
        }
    }

    pub fn on_window_delete(&mut self, window_id: u64) {
        tracing::debug!("{:?}", window_id);

        for (_, wc) in self.workspace_containers.iter_mut() {
            wc.on_window_delete(window_id);
        }
    }

    pub fn on_window_overwrite(&mut self, window: WLWindow) {
        tracing::debug!("{:?}", window);

        if let Some(wc) = window
            .get_workspace_id()
            .and_then(|w| self.workspace_containers.get_mut(&w))
        {
            wc.on_window_overwrite(window);
        } else {
            let mut container = WindowContainer::new(window.get_id());
            container.on_window_overwrite(window);
            self.on_workspace_overwrite(container);
        }
    }

    pub fn on_window_change_focus(&mut self, window: Option<WLWindow>) {
        tracing::debug!("{:?}", window);

        for (_, wc) in self.workspace_containers.iter_mut() {
            self.current_window_id.map(|id| {
                wc.deal_with_window_id(id, |e| {
                    e.on_focus(false);
                })
            });
        }

        if let Some(win) = window {
            if let Some(wsid) = win.get_workspace_id() {
                self.workspace_containers
                    .get_mut(&wsid)
                    .map(|e| e.on_change_focus(Some(win.get_id())));
                self.on_workspace_change(wsid);
            }
            self.current_window_id.replace(win.get_id());
        }
    }
}
