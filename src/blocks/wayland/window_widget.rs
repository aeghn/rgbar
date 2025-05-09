use chin_tools::wayland::{WLWindow, WLWindowBehaiver, WLWindowId, WLWorkspace, WLWorkspaceId};
use chin_tools::wrapper::anyhow::AResult;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

use crate::prelude::*;
use crate::util;

#[derive(Debug, PartialEq)]
pub struct WindowWidget {
    pub window: WLWindow,
    pub gbox: gtk::Box,
    pub title: Label,
    pub dirty: bool,
}

impl Deref for WindowWidget {
    type Target = WLWindow;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
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
            title.set_label(window.get_title().unwrap_or("Unknown Title"));
        }

        title.set_single_line_mode(true);
        title.set_ellipsize(EllipsizeMode::End);
        title.set_lines(1);
        title.set_line_wrap(true);
        title.set_line_wrap_mode(WrapMode::Char);

        container.pack_start(&event_box, false, false, 0);
        container.pack_start(&title, false, false, 0);
        container.show_all();

        if let Some(app_id) = window.get_app_id() {
            if let Some(img) = icon_loader.load_named_pixbuf(&app_id) {
                icon.set_from_surface(img.create_surface(2, None::<&Window>).as_ref());
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
            gbox: container,
            title,
            dirty: true,
        }
    }

    pub fn update_data(&mut self, window: WLWindow) -> bool {
        if self.window != window {
            self.window = window;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn update_view(&mut self) {
        if self.dirty {
            if let Some(title) = self.window.get_title() {
                self.title.set_label(&title);
            }
            if self.window.is_focused() {
                self.title.set_label(
                    self.window
                        .get_title()
                        .as_ref()
                        .map_or("Unknown Title", |v| v),
                );
                self.title.show();
                self.gbox.style_context().add_class("wmw-focus")
            } else {
                self.title.set_text("");
                self.title.hide();
                self.gbox.style_context().remove_class("wmw-focus")
            }
            self.dirty = false;
        }
    }
}

#[derive(Debug)]
pub struct WindowContainer {
    pub workspace_id: WLWindowId,
    pub widget_map: HashMap<WLWindowId, WindowWidget>,
    pub gbox: gtk::Box,
    focused_id: Option<WLWindowId>,
    icon_loader: GtkIconLoader,
    dirty: bool,
    to_remove: Vec<gtk::Box>,
}

impl WindowContainer {
    pub fn new(workspace_id: WLWindowId) -> Self {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        // https://stackoverflow.com/questions/50120555/gtk-stack-wont-change-visible-child-inside-an-event-callback-function
        // > I do not have Granite installed so I can't reproduce the given example. Does Granite.Widgets.Welcome get shown after instantiation? If not, and I quote, "Note that the child widget has to be visible itself (see show) in order to become the visible child of this.". Try to instantiate it first, call show on it and then add it to the Gtk.Stack. It should work.
        container.show_all();
        Self {
            widget_map: Default::default(),
            gbox: container,
            focused_id: None,
            icon_loader: util::gtk_icon_loader::GtkIconLoader::new(),
            workspace_id,
            dirty: true,
            to_remove: Default::default(),
        }
    }

    pub fn on_window_overwrite(&mut self, window: WLWindow) -> bool {
        if let Some(win) = self.widget_map.get_mut(&window.get_id()) {
            let dirty = win.update_data(window);
            self.dirty = self.dirty || dirty;
            return self.dirty;
        } else {
            let ww = WindowWidget::new(window, &self.icon_loader);
            self.gbox.add(&ww.gbox);
            self.widget_map.insert(ww.get_id(), ww);

            return true;
        }
    }

    pub fn on_window_delete(&mut self, window: WLWindowId) -> bool {
        if let Some(win) = self.widget_map.remove(&window) {
            self.dirty = true;
            self.to_remove.push(win.gbox);
            return true;
        }
        false
    }

    pub fn update_view(&mut self) {
        if self.dirty {
            let mut wws: Vec<&mut WindowWidget> = self.widget_map.iter_mut().map(|(_, w)| w).collect();

            for r in &self.to_remove {
                self.gbox.remove(r);
            }

            wws.sort_by(|e1, e2| e1.get_title().cmp(&e2.get_title()));

            for (id, ww) in wws.into_iter().enumerate() {
                ww.update_view();
                self.gbox.reorder_child(&ww.gbox, id as i32);
            }
            self.gbox.show_all();

            self.dirty = false;
        }
    }
}

pub struct WindowContainerManager {
    pub stack: gtk::Stack,
    pub workspace_containers: HashMap<WLWorkspaceId, WindowContainer>,
    current_workspace_id: Option<WLWorkspaceId>,
}

impl WindowContainerManager {
    pub fn new() -> AResult<Self> {
        let stack = gtk::Stack::builder().build();

        stack.add_named(
            &gtk::Label::new(Some("This workspace's windows container is missing.")),
            "missing",
        );

        let containers: HashMap<WLWindowId, WindowContainer> = Default::default();

        Ok(Self {
            stack,
            workspace_containers: containers,
            current_workspace_id: Default::default(),
        })
    }
    pub fn on_workspace_overwrite(&mut self, workspace: &WLWorkspace) {
        if workspace.is_focused {
            self.current_workspace_id.replace(workspace.id);
        }
    }

    pub fn on_workspace_delete(&mut self, workspace_id: &WLWindowId) {
        if let Some(old) = self.workspace_containers.remove(&workspace_id) {
            self.stack.remove(&old.gbox);
        }
    }

    pub fn on_window_overwrite(&mut self, window: &WLWindow) {
        if window.is_focused {
            if let Some(id) = window.workspace_id {
                self.current_workspace_id.replace(id);
            }
        }
        if let Some(wc) = window
            .get_workspace_id()
            .and_then(|w| self.workspace_containers.get_mut(&w))
        {
            wc.on_window_overwrite(window.clone());
        } else {
            let mut container = WindowContainer::new(window.get_id());
            container.on_window_overwrite(window.clone());
            if let Some(id) = window.get_workspace_id() {
                self.stack.add_named(&container.gbox, &id.to_string());
                self.workspace_containers.insert(id, container);
            }
        }
    }

    pub fn on_window_delete(&mut self, window_id: &WLWindowId) {
        for (_, wc) in self.workspace_containers.iter_mut() {
            wc.on_window_delete(window_id.clone());
        }
    }

    pub fn update_view(&mut self) {
        let stack = self.stack.clone();

        if let Some(wc) = self
            .current_workspace_id
            .and_then(|e| self.workspace_containers.get_mut(&e))
        {
            wc.update_view();
            let container = wc.gbox.clone();
            idle_add_local_once(move || {
                stack.set_visible_child(&container);
                stack.show_all();
            });
        } else {
            idle_add_local_once(move || {
                stack.hide();
            });
        };
    }
}
