use std::collections::HashMap;

use chin_tools::wayland::{WLCompositor, WLOutput, WLWorkspace};
use chin_tools::wrapper::anyhow::AResult;
pub use gtk::traits::{BoxExt, LabelExt, StyleContextExt, WidgetExt};


#[derive(Debug)]
pub struct WorkspaceWidget {
    workspace: WLWorkspace,
}

impl WorkspaceWidget {
    pub fn new(workspace: WLWorkspace) -> WorkspaceWidget {
        WorkspaceWidget {
            workspace,
        }
    }
}

#[derive(Debug)]
pub struct WorkspaceContainer {
    workspace_widget_map: HashMap<usize, WorkspaceWidget>,
    pub holder: gtk::Box,
    indicator: gtk::Label,
    output_name: String,
    current_workspace_id: Option<usize>,
}

impl WorkspaceContainer {
    pub fn new(output_name: String) -> AResult<Self> {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        let indicator = gtk::Label::builder().build();
        indicator.style_context().add_class("ws");

        holder.style_context().add_class("wss");
        holder.pack_start(&indicator, false, true, 0);

        Ok(Self {
            workspace_widget_map: Default::default(),
            holder,
            output_name,
            current_workspace_id: Default::default(),
            indicator,
        })
    }

    pub fn init(mut self) -> AResult<Self> {
        tracing::info!("monitor_name: {}", self.output_name);
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

    pub fn on_workspace_added(&mut self, workspace: &WLWorkspace) {
        self.workspace_widget_map
            .insert(workspace.get_id(), WorkspaceWidget::new(workspace.clone()));
        self.update_view();
    }

    pub fn on_workspace_delete(&mut self, workspace: &WLWorkspace) {
        self.workspace_widget_map.remove(&workspace.get_id());
        self.update_view();
    }

    pub fn on_workspace_changed(&mut self, workspace: &WLWorkspace) {
        self.workspace_widget_map
            .get_mut(&workspace.get_id())
            .map(|e| e.workspace = workspace.clone());
        self.update_view();
    }

    pub fn on_workspace_focused(&mut self, workspace: &WLWorkspace) {
        self.current_workspace_id.replace(workspace.get_id());
        self.update_view();
    }

    pub fn on_active_monitor_changed(&mut self, output: &WLOutput) {
        tracing::error!("[WS] not implmented {:?}", output);
    }

    fn update_view(&self) {
        let indicator = self
            .current_workspace_id
            .and_then(|e| self.workspace_widget_map.get(&e))
            .map_or("Unknown".to_owned(), |e| e.workspace.get_name());
        self.indicator
            .set_label(format!("{}/{}", indicator, self.workspace_widget_map.len()).as_str());

        // let mut line_size = self.lines.children().len();

        // let mut wss: Vec<&WorkspaceWidget> =
        //     self.workspace_widget_map.values().into_iter().collect();
        // wss.sort_by(|e1, e2| e1.workspace.get_id().cmp(&e2.workspace.get_id()));

        // let mut current_index = None;
        // for (index, ws) in wss.iter().enumerate() {
        //     if Some(ws.workspace.get_id()) == self.current_workspace_id {
        //         let end = if index + 2 >= wss.len() {
        //             wss.len() - 1
        //         } else {
        //             index + 2
        //         };

        //         let start = index.saturating_sub(2);
        //         let cur = index;
        //         current_index.replace((start, cur, end));
        //         break;
        //     }
        // }

        // while wss.len() > line_size && line_size < 5 {
        //     let indicator = gtk::Separator::builder()
        //         .height_request(5)
        //         .width_request(20)
        //         .build();
        //     indicator.style_context().add_class("ws-line");
        //     self.lines.pack_end(&indicator, true, false, 1);
        //     line_size += 1;
        // }
        // while wss.len() < line_size {
        //     self.lines.children().get(0).map(|e| self.lines.remove(e));
        //     line_size -= 1;
        // }
        // let children = self.lines.children();
        // if let Some((start, cur, end)) = current_index {
        //     for i in 0..=(end - start) {
        //         if cur - start == i {
        //             children
        //                 .get(i)
        //                 .map(|e| e.style_context().add_class("ws-line-focus"));
        //         } else {
        //             children
        //                 .get(i)
        //                 .map(|e| e.style_context().remove_class("ws-line-focus"));
        //         }
        //     }
        // }
    }

    pub fn get_workspace_ids(&self) -> Vec<usize> {
        self.workspace_widget_map.keys().map(|e| *e).collect()
    }
}
