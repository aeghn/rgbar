use std::collections::HashMap;

use chin_tools::wayland::{WLCompositor, WLOutput, WLWorkspace};
use chin_tools::wrapper::anyhow::AResult;
use gtk::prelude::ContainerExt;
use gtk::traits::WidgetExt;
use gtk::traits::{BoxExt, ButtonExt, StyleContextExt};

#[derive(Debug)]
pub struct WorkspaceWidget {
    workspace: WLWorkspace,
    button: gtk::Button,
}

#[derive(Debug)]
pub struct WorkspaceContainer {
    workspace_widget_map: HashMap<u64, WorkspaceWidget>,
    pub holder: gtk::Box,
    output_name: String,
    current_workspace_id: Option<u64>,
}

impl WorkspaceContainer {
    pub fn new(output_name: String) -> AResult<Self> {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        holder.style_context().add_class("wss");

        Ok(Self {
            workspace_widget_map: Default::default(),
            holder,
            output_name,
            current_workspace_id: Default::default(),
        })
    }

    pub fn get_workspace_ids(&self) -> Vec<u64> {
        self.workspace_widget_map.keys().map(|e| *e).collect()
    }

    pub fn init(mut self) -> AResult<Self> {
        let mut current_workspace_id = None;
        let mut containers: Vec<WLWorkspace> = Default::default();

        for workspace in WLCompositor::current()?.get_all_workspaces()? {
            if workspace.is_focused() {
                current_workspace_id.replace(workspace.get_id());
            }

            containers.push(workspace);
        }
        self.update_all_workspaces(containers);

        Ok(self)
    }

    pub fn update_all_workspaces(&mut self, mut wss: Vec<WLWorkspace>) {
        wss.sort_by(|e1, e2| e2.get_name().cmp(&e1.get_name()));
        for ele in wss.iter() {
            self.on_workspace_added(ele);
        }

        if let Some(ws) = wss.iter().find(|e| e.is_active()) {
            self.on_workspace_focused(ws);
        }

        self.holder.show_all();
    }

    pub fn on_workspace_changed(&mut self, workspace: &WLWorkspace) {
        if let Some(ws) = self.workspace_widget_map.get(&workspace.get_id()) {
            ws.button.set_label(&workspace.get_name());
        } else {
            self.on_workspace_added(workspace);
        }
    }

    pub fn on_workspace_focused(&mut self, workspace: &WLWorkspace) {
        {
            if let Some(ww) = self
                .current_workspace_id
                .and_then(|old| self.workspace_widget_map.get(&old))
            {
                let style = ww.button.style_context();
                if style.has_class("ws-focus") {
                    style.remove_class("ws-focus");
                }
            }
        }

        if let Some(ww) = self.workspace_widget_map.get(&workspace.get_id()) {
            let style = ww.button.style_context();
            if !style.has_class("ws-focus") {
                style.add_class("ws-focus");
            }
        }

        self.workspace_widget_map.values().for_each(|e| {
            if e.workspace.get_id() != workspace.get_id() {
                e.button.hide()
            } else {
                e.button.show()
            }
        });

        self.current_workspace_id.replace(workspace.get_id());
    }

    pub fn on_workspace_delete(&mut self, workspace: &WLWorkspace) {
        if let Some(ws) = self.workspace_widget_map.remove(&workspace.get_id()) {
            self.holder.remove(&ws.button);
        }
    }

    pub fn on_active_monitor_changed(&mut self, output: &WLOutput) {
        tracing::error!("[WS] not implmented {:?}", output);
    }

    pub fn on_workspace_added(&mut self, workspace: &WLWorkspace) {
        tracing::debug!(
            "[WS] workspace added wsid2idx: {} -> {}",
            workspace.get_id(),
            workspace.get_name()
        );
        if let None = self.workspace_widget_map.get(&workspace.get_id()) {
            let widget = Self::workspace_container(workspace);
            self.holder.pack_end(&widget.button, false, false, 0);
            self.workspace_widget_map.insert(workspace.get_id(), widget);
        }
        self.reorder_workspaces();
    }

    fn reorder_workspaces(&mut self) {
        let workspaces = self.holder.clone();
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
                a.widget_name().cmp(&b.widget_name())
            }
        });

        children.iter().rev().enumerate().for_each(|(i, widget)| {
            widget.show();
            workspaces.reorder_child(widget, i as i32)
        });
    }

    fn workspace_container(workspace: &WLWorkspace) -> WorkspaceWidget {
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
}
