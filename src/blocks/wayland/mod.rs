use chin_tools::wayland::{into_wl_event, WLCompositor, WLEvent, WLWorkspace, WLWindow, CurrentStatus, WLOutput};
use chin_tools::wrapper::anyhow::AResult;
use gdk::glib::Cast;
use gtk::{Widget, Image, Label};

use crate::datahodler::channel::{DualChannel, MReceiver, SSender};
use crate::statusbar::WidgetShareInfo;
use gio::{DataInputStream, SocketClient};
use glib::{MainContext, Priority};

use super::Block;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;


use crate::util::{self};
use chin_tools::utils::idutils;

use crate::util::gtk_icon_loader::GtkIconLoader;
use gtk::prelude::{ContainerExt, ImageExt, LabelExt, WidgetExtManual};
use gtk::traits::WidgetExt;
use gtk::traits::{BoxExt, ButtonExt, StyleContextExt};
use tracing::error;



#[derive(Clone)]
pub enum OutEvent {
    WLEvent(WLEvent),
    AllWorkspaces(Vec<WLWorkspace>),
}

#[derive(Clone)]
pub enum InEvent {
    NewBar,
}

pub struct WaylandBlock {
    dualchannel: DualChannel<OutEvent, InEvent>,
}

impl WaylandBlock {
    pub fn new() -> Self {
        Self {
            dualchannel: DualChannel::new(30),
        }
    }

    fn new_client() -> AResult<Vec<WLWorkspace>> {
        let com = WLCompositor::current()?;
        let workspaces = com.get_all_workspaces()?;

        Ok(workspaces)
    }
}

impl Block for WaylandBlock {
    type Out = OutEvent;
    type In = InEvent;

    fn run(&mut self) -> anyhow::Result<()> {
        let sender = self.dualchannel.get_out_sender();

        std::thread::spawn(move || {
            let mut all_windows = chin_tools::wayland::niri::model::Window::get_all()
                .unwrap()
                .into_iter()
                .map(|e| (e.id, e))
                .collect();
            let mut all_workspace = chin_tools::wayland::niri::model::Workspace::get_all()
                .unwrap()
                .into_iter()
                .map(|e| (e.id, e))
                .collect();

            chin_tools::wayland::niri::event_stream::handle_event_stream(|event| {
                let events = into_wl_event(event, &mut all_workspace, &mut all_windows);
                if let Some(events) = events {
                    for ele in events {
                        let _ = sender.send(OutEvent::WLEvent(ele));
                    }
                }
            })
        });

        let in_receiver = self.dualchannel.get_in_receiver();
        let sender = self.dualchannel.get_out_sender();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                match in_receiver.recv().await {
                    Ok(msg) => match msg {
                        InEvent::NewBar => {
                            let all = Self::new_client().unwrap();
                            sender.send(OutEvent::AllWorkspaces(all)).unwrap();
                        }
                    },
                    Err(_) => todo!(),
                }
            }
        });

        Ok(())
    }

    fn widget(&self, share_info: &WidgetShareInfo) -> Widget {
        let in_sender = self.dualchannel.get_in_sender();
        let out_receiver = self.dualchannel.get_out_receiver();

        let mut wayland_widget = WaylandWidget::new(&in_sender, &out_receiver, share_info).unwrap();
        let holder = wayland_widget.holder.clone();

        MainContext::ref_thread_default().spawn_local(async move {
            wayland_widget.receive_out_events().await;
        });

        holder.upcast()
    }
}


#[derive(Clone, Debug)]
pub struct WorkspaceWidget {
    workspace: WLWorkspace,
    button: gtk::Button,
}

pub struct WaylandWidget {
    workspace_widget_map: HashMap<u64, WorkspaceWidget>,
    ws_box: gtk::Box,
    title_container: (gtk::Image, Label),
    out_receiver: MReceiver<OutEvent>,
    in_sender: SSender<InEvent>,
    pub holder: gtk::Box,
    current_status: CurrentStatus,
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

        let title_container = Self::create_window_container();

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
            current_status,
            icon_loader,
            output_name,
        })
    }

    pub async fn receive_out_events(&mut self) {
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

    fn update_all_workspaces(&mut self, mut wss: Vec<WLWorkspace>) {
        wss.sort_by(|e1, e2| e2.get_name().cmp(&e1.get_name()));
        for ele in wss.iter() {
            self.on_workspace_added(ele);
        }

        if let Some(ws) = wss.iter().find(|e| e.is_active()) {
            self.on_workspace_focused(ws);
        }

        self.holder.show_all();
    }

    fn on_workspace_changed(&mut self, workspace: &WLWorkspace) {
        if let Some(ws) = self.find_workspace_widget(&workspace) {
            ws.button.set_label(&workspace.get_name());
        } else {
            self.on_workspace_added(workspace);
        }
    }

    fn on_workspace_focused(&mut self, workspace: &WLWorkspace) {
        {
            let old = &self.current_status.workspace;
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

        self.current_status.workspace = workspace.clone();
    }

    fn on_workspace_delete(&mut self, workspace: &WLWorkspace) {
        if let Some(ws) = self
            .workspace_widget_map
            .remove(&workspace.get_id())
        {
            self.ws_box.remove(&ws.button);
        }
    }

    fn on_active_window_changed(&mut self, window: &Option<WLWindow>) {
        self.current_status.window = window.clone();

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

    fn on_active_monitor_changed(&mut self, output: &WLOutput) {
        self.current_status.output = output.clone();
        let c = self
            .workspace_widget_map
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

    fn on_workspace_added(&mut self, workspace: &WLWorkspace) {
        if let None = self.find_workspace_widget(workspace) {
            tracing::error!("add workspace: {:?}", workspace);
            let widget = Self::create_workspace_button(workspace);
            self.workspace_widget_map
                .insert(workspace.get_id(), widget.clone());
            self.ws_box.pack_end(&widget.button, false, false, 0);
        }
        self.reorder_workspaces();
    }

    fn reorder_workspaces(&mut self) {
        let workspaces = self.ws_box.clone();
        for (_, w) in self.workspace_widget_map.iter().filter(|(_, e)| {
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

    fn find_workspace_widget(&self, workspace: &WLWorkspace) -> Option<WorkspaceWidget> {
        self.workspace_widget_map
            .iter()
            .find(|(_, ww)| workspace.get_id() == ww.workspace.get_id())
            .map(|(_, ww)| ww.clone())
    }

    fn create_workspace_container() -> gtk::Box {
        let ws_container = gtk::Box::builder().build();
        ws_container.style_context().add_class("wss");

        ws_container
    }

    fn create_window_container() -> (gtk::Image, gtk::Label) {

        (image, label)
    }
}




pub struct WindowWidget {
    pub window: WLWindow,
    pub container: gtk::Box,
    pub title: Label,
    pub icon: Image,
}

impl WindowWidget {
    pub fn new(window: WLWindow, icon_loader: &GtkIconLoader) -> Self {
        let container = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).build();

        let icon = gtk::Image::builder().build();
        icon.style_context().add_class("wm-cw-icon");
        let title = gtk::Label::builder().build();
        title.style_context().add_class("wm-cw-title");

        title.set_single_line_mode(true);
        title.set_ellipsize(gdk::pango::EllipsizeMode::End);
        title.set_lines(1);
        title.set_line_wrap(true);
        title.set_line_wrap_mode(gdk::pango::WrapMode::Char);
        if let Some(app_id) = window.get_app_id() {
            if let Some(img) = icon_loader.load_from_name(&app_id) {
                icon.set_from_pixbuf(Some(&img));
            } else {
                // todo
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
}

pub struct WindowContainerManager {
    pub stack: gtk::StackSwitcher,
    pub containers: HashMap<u64, WindowContainer>
}

impl WindowContainerManager {
    pub fn new() -> Self {
        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::SlideUpDown)
            .build();
        Self {
            stack: gtk::StackSwitcher::builder().stack(&stack).build(),
            containers: Default::default(),
        }
    }

    pub fn on_workspace_overwrite(&mut self, container: WindowContainer) {
        if let Some(old) = self.containers.remove(&container.workspace_id) {
            self.stack.remove(&old.container);
        }
        self.stack.add(&container.container);
        self.containers.insert(container.workspace_id, container);
    }

    pub fn on_workspace_change(&mut self, id: u64) {
        self.stack.set_transition_type();
        self.stack.set_transition_duration(1000);
    }
}

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
            container: gtk::Box::builder().orientation(gtk::Orientation::Horizontal).build(),
            focused_id: None,
            icon_loader: util::gtk_icon_loader::GtkIconLoader::new(),
            workspace_id,
        }
    }

    pub fn on_window_overwrite(&mut self, window: WLWindow) {
        let mut change_focus = None;
        if let Some(win) = self.widget_map.get_mut(&window.get_id())  {
            if window.is_focused() && ! win.window.is_focused() {
                change_focus = Some(window.get_id())
            }
            if win.window.get_title() != window.get_title() {
                win.update_window(window);
            }
        } else {
            if window.is_focused() {
                change_focus = Some(window.get_id())
            }
            self.widget_map.insert(window.get_id(), WindowWidget::new(window, &self.icon_loader));
            self.on_reorder()
        }

        if change_focus.is_some() {
            self.on_change_focus(change_focus)
        }
    }

    pub fn on_window_delete(&mut self, window: u64) {
        if let Some(window) = self.widget_map.remove(&window) {
            self.container.remove(&window.container);
        }
    }

    pub fn on_change_focus(&mut self, id: Option<u64>) {
        if self.focused_id != id {
            if let Some(id) = self.focused_id {
                if let Some(w) = self.widget_map.get(&id) {
                    w.title.hide()
                }

                if let Some(w) = self.widget_map.get(&id) {
                    w.title.show()
                }
            }
        }
    }

    pub fn on_reorder(&self) {
        let mut ids:Vec<(u64, &gtk::Box)> = self.widget_map.iter().map(|(i, w)|  (*i, &w.container)).collect();

        ids.sort_by(|e1, e2| e1.0.cmp(&e2.0));

        for (id, (_, b)) in ids.into_iter().enumerate() {
            self.container.reorder_child( b, id  as i32);
        }
    }
}
