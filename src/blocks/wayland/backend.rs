use crate::datahodler::channel::{MReceiver, SSender};

use crate::statusbar::WidgetShareInfo;
use crate::util::{self};
use chin_tools::utils::idutils;
use chin_tools::wayland::{CurrentStatus, WLEvent, WLOutput, WLWindow, WLWorkspace};
use chin_tools::wrapper::anyhow::AResult;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::util::gtk_icon_loader::GtkIconLoader;
use gtk::prelude::{ContainerExt, ImageExt, LabelExt, WidgetExtManual};
use gtk::traits::WidgetExt;
use gtk::traits::{BoxExt, ButtonExt, StyleContextExt};
use gtk::Label;
use tracing::error;

use super::{InEvent, OutEvent};

#[derive(Clone, Debug)]
pub struct WorkspaceWidget {
    workspace: WLWorkspace,
    button: gtk::Button,
}

#[derive(Clone)]
pub struct WaylandWidget {
    workspace_widget_map: Rc<RefCell<HashMap<u64, WorkspaceWidget>>>,
    ws_box: gtk::Box,
    title_container: (gtk::Image, Label),
    out_receiver: MReceiver<OutEvent>,
    in_sender: SSender<InEvent>,
    pub holder: gtk::Box,
    current_status: Rc<RefCell<CurrentStatus>>,
    pub icon_loader: GtkIconLoader,
    output_name: String,
}

impl WaylandWidget {
    pub fn new(
        in_sender: &SSender<InEvent>,
        out_receiver: &MReceiver<OutEvent>,
        share_info: &WidgetShareInfo,
    ) -> AResult<Self> {
        let holder = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        let ws_box = Self::create_workspace_container();

        let title_container = Self::create_active_window_container();

        holder.style_context().add_class("wm");

        holder.pack_start(&ws_box, false, false, 0);

        holder.pack_start(&title_container.0, false, false, 0);
        holder.pack_start(&title_container.1, false, false, 0);

        let icon_loader = util::gtk_icon_loader::GtkIconLoader::new();

        let current_status = CurrentStatus::new(chin_tools::wayland::WLCompositor::Niri)?;

        let output_name = share_info
            .plug_name
            .as_ref()
            .map_or_else(|| idutils::generate_uuid(), |s| s.to_owned());

        Ok(WaylandWidget {
            workspace_widget_map: Default::default(),
            ws_box,
            title_container,
            out_receiver: out_receiver.clone(),
            in_sender: in_sender.clone(),
            holder,
            current_status: Rc::new(RefCell::new(current_status)),
            icon_loader,
            output_name,
        })
    }

    pub async fn receive_out_events(&self) {
        let mut receiver = self.out_receiver.clone();

        self.in_sender.send(InEvent::NewBar).await.unwrap();

        loop {
            match receiver.recv().await {
                Ok(msg) => match msg {
                    OutEvent::WLEvent(event) => match event {
                        WLEvent::WorkspaceFocused(ws) => self.on_workspace_focused(&ws),
                        WLEvent::WorkspaceDeleted(ws) => {
                            self.on_workspace_delete(&ws);
                        }
                        WLEvent::WorkspaceAdded(ws) => {
                            self.on_workspace_added(&ws);
                        }
                        WLEvent::WorkspaceChanged(ws) => self.on_workspace_changed(&ws),
                        WLEvent::WindowFocused(window) => self.on_active_window_changed(&window),
                        WLEvent::MonitorFocused(output) => self.on_active_monitor_changed(&output),
                    },
                    OutEvent::AllWorkspaces(vec) => self.update_all_workspaces(vec),
                },
                Err(err) => {
                    error!("unable to receive message: {}", err)
                }
            }
        }
    }

    fn update_all_workspaces(&self, mut wss: Vec<WLWorkspace>) {
        wss.sort_by(|e1, e2| e2.get_name().cmp(&e1.get_name()));
        for ele in wss.iter() {
            self.on_workspace_added(ele);
        }

        if let Some(ws) = wss.iter().find(|e| e.is_active()) {
            self.on_workspace_focused(ws);
        }

        self.holder.show_all();
    }

    fn on_workspace_changed(&self, workspace: &WLWorkspace) {
        if let Some(ws) = self.find_workspace_widget(&workspace) {
            ws.button.set_label(&workspace.get_name());
        } else {
            self.on_workspace_added(workspace);
        }
    }

    fn on_workspace_focused(&self, workspace: &WLWorkspace) {
        {
            let old = &self.current_status.borrow().workspace;
            if let Some(ww) = self.find_workspace_widget(old) {
                let style = ww.button.style_context();
                if style.has_class("ws-focus") {
                    style.remove_class("ws-focus");
                }
            }
        }

        if let Some(ww) = self.find_workspace_widget(workspace) {
            let style = ww.button.style_context();
            if !style.has_class("ws-focus") {
                style.add_class("ws-focus");
            }
        }

        self.current_status.borrow_mut().workspace = workspace.clone();
    }

    fn on_workspace_delete(&self, workspace: &WLWorkspace) {
        if let Some(ws) = self
            .workspace_widget_map
            .borrow_mut()
            .remove(&workspace.get_id())
        {
            self.ws_box.remove(&ws.button);
        }
    }

    /*     fn on_workspace_reset(&self, workspaces: Vec<WLWorkspace>) {
        let mut wss: HashMap<u64, WLWorkspace> =
            workspaces.into_iter().map(|e| (e.get_id(), e)).collect();

        for ws in self.workspace_widget_map.borrow_mut().iter_mut() {
            if let Some(w) = wss.remove(&ws.workspace.get_id()) {
                ws.workspace = w.clone();
                ws.button.set_label(w.get_name().as_str());
            } else {
                self.ws_box.remove(&ws.button);
            }
        }

        tracing::error!("WSS:{:?}", wss);
        for w in wss.values() {
            self.append_workspace(w);
        }

        self.reorder_workspaces();
    } */

    fn on_active_window_changed(&self, window: &Option<WLWindow>) {
        self.current_status.borrow_mut().window = window.clone();

        if let Some(Some(title)) = window.as_ref().map(|e| e.get_title()) {
            self.title_container.1.set_label(&title);
            self.title_container.1.show();
        } else {
            self.title_container.1.hide();
        }

        if let Some(Some(title)) = window.as_ref().map(|e: &WLWindow| e.get_app_id()) {
            if let Some(img) = self.icon_loader.load_from_name(&&title) {
                self.title_container.0.set_from_pixbuf(Some(&img));
                self.title_container.0.show();
            } else {
                self.title_container.0.hide();
            }
        } else {
            self.title_container.0.hide();
        }
    }

    fn on_active_monitor_changed(&self, output: &WLOutput) {
        self.current_status.borrow_mut().output = output.clone();
        let c = self
            .workspace_widget_map
            .borrow()
            .iter()
            .find(|(_, e)| {
                e.workspace.is_active()
                    && e.workspace
                        .get_output_name()
                        .map(|e| e.as_str() == output.get_name())
                        .unwrap_or(false)
            })
            .map(|(_, w)| w.workspace.clone());
        if let Some(c) = c {
            self.on_workspace_focused(&c);
        }
    }

    fn on_workspace_added(&self, workspace: &WLWorkspace) {
        if let None = self.find_workspace_widget(workspace) {
            tracing::error!("add workspace: {:?}", workspace);
            let widget = Self::create_workspace_button(workspace);
            self.workspace_widget_map
                .borrow_mut()
                .insert(workspace.get_id(), widget.clone());
            self.ws_box.pack_end(&widget.button, false, false, 0);
        }
        self.reorder_workspaces();
    }

    fn reorder_workspaces(&self) {
        let workspaces = self.ws_box.clone();
        for (_, w) in self.workspace_widget_map.borrow().iter().filter(|(_, e)| {
            e.workspace
                .get_output_name()
                .as_ref()
                .map_or(true, |e| e != &self.output_name)
        }) {
            workspaces.remove(&w.button);
        }

        let mut children = workspaces.children();
        children.sort_by(|a, b| {
            let ai = isize::from_str_radix(a.widget_name().as_str(), 10);
            let bi = isize::from_str_radix(b.widget_name().as_str(), 10);

            if let (Ok(ai), Ok(bi)) = (ai, bi) {
                isize::cmp(&ai, &bi)
            } else {
                tracing::error!("widget name: {:?}", a.widget_name());
                a.widget_name().cmp(&b.widget_name())
            }
        });

        children.iter().rev().enumerate().for_each(|(i, widget)| {
            widget.show();
            workspaces.reorder_child(widget, i as i32)
        });
    }

    fn create_workspace_button(workspace: &WLWorkspace) -> WorkspaceWidget {
        let workspace_button = gtk::Button::builder()
            .label(workspace.get_name().as_str())
            .name(workspace.get_id().to_string())
            .build();

        workspace_button.style_context().add_class("ws");

        {
            let ws = workspace.clone();
            workspace_button.connect_clicked(move |_| {
                let _ = ws.focus();
            });
        }

        WorkspaceWidget {
            workspace: workspace.clone(),
            button: workspace_button,
        }
    }

    fn create_workspace_container() -> gtk::Box {
        let ws_container = gtk::Box::builder().build();
        ws_container.style_context().add_class("wss");

        ws_container
    }

    fn create_active_window_container() -> (gtk::Image, gtk::Label) {
        let image = gtk::Image::builder().build();
        image.style_context().add_class("wm-cw-icon");
        let label = gtk::Label::builder().build();
        label.style_context().add_class("wm-cw-title");

        label.set_single_line_mode(true);
        label.set_ellipsize(gdk::pango::EllipsizeMode::End);
        label.set_lines(1);
        label.set_line_wrap(true);
        label.set_line_wrap_mode(gdk::pango::WrapMode::Char);

        (image, label)
    }

    fn find_workspace_widget(&self, workspace: &WLWorkspace) -> Option<WorkspaceWidget> {
        self.workspace_widget_map
            .borrow()
            .iter()
            .find(|(_, ww)| workspace.get_id() == ww.workspace.get_id())
            .map(|(_, ww)| ww.clone())
    }
}
