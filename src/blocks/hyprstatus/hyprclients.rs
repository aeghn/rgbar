use std::hash::{Hash, Hasher};
use std::process::Command;

use tracing::error;

#[derive(Clone)]
pub struct HyprClient {
    pub class: String,
    pub title: String,
    pub address: String,
    pub mapped: bool,
    pub hidden: bool,
    pub pid: i64,
    pub xwayland: bool,
    pub workspace_id: i64,
}

#[derive(Clone, Debug)]
pub struct HyprMonitor {
    pub id: Option<i64>,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct HyprWorkspace {
    pub id: Option<i32>,
    pub name: String,
    pub monitor: HyprMonitor,
}

impl Hash for HyprWorkspace {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq<Self> for HyprWorkspace {
    fn eq(&self, other: &Self) -> bool {
        return self.id == other.id;
    }
}

impl Eq for HyprWorkspace {}

impl HyprWorkspace {
    pub fn get_bar_name(&self) -> String {
        if self.monitor.name != "eDP-1" {
            format!("* {}", self.name)
        } else {
            self.name.to_string()
        }
    }
}

pub fn get_clients() -> anyhow::Result<Vec<HyprClient>> {
    let output = Command::new("hyprctl").arg("clients").arg("-j").output()?;
    error!("hypr clients: {:.?}", output);

    let mut vec: Vec<HyprClient> = vec![];

    let out = String::from_utf8(output.stdout).unwrap();

    let json = serde_json::from_str::<serde_json::Value>(out.as_str()).unwrap();

    if let Some(array) = json.as_array() {
        for e in array {
            let class = e.get("class").unwrap().as_str().unwrap();
            let monitor = e.get("monitor").unwrap().as_i64().unwrap();
            if monitor == -1 {
                continue;
            }

            vec.push(HyprClient {
                class: class.to_string(),
                title: e.get("title").unwrap().as_str().unwrap().to_string(),
                address: e.get("address").unwrap().as_str().unwrap().to_string(),
                mapped: e.get("mapped").unwrap().as_bool().unwrap(),
                hidden: e.get("hidden").unwrap().as_bool().unwrap(),
                pid: e.get("pid").unwrap().as_i64().unwrap(),
                xwayland: e.get("xwayland").unwrap().as_bool().unwrap(),
                workspace_id: e
                    .get("workspace")
                    .unwrap()
                    .get("id")
                    .unwrap()
                    .as_i64()
                    .unwrap(),
            })
        }
    }
    Ok(vec)
}

pub fn get_active_client() -> Option<HyprClient> {
    let output = Command::new("hyprctl")
        .arg("activewindow")
        .arg("-j")
        .output()
        .unwrap();

    let _vec: Vec<HyprClient> = vec![];

    let out = String::from_utf8(output.stdout).unwrap();

    let json = serde_json::from_str::<serde_json::Value>(out.as_str()).unwrap();

    if let Some(e) = json.as_object() {
        Some(HyprClient {
            class: e.get("class")?.as_str()?.to_string(),
            title: e.get("title")?.as_str()?.to_string(),
            address: e.get("address")?.as_str()?.to_string(),
            mapped: e.get("mapped")?.as_bool()?,
            hidden: e.get("hidden")?.as_bool()?,
            pid: e.get("pid")?.as_i64()?,
            xwayland: e.get("xwayland")?.as_bool()?,
            workspace_id: e.get("workspace")?.get("id")?.as_i64()?,
        })
    } else {
        None
    }
}

pub fn get_monitors() -> anyhow::Result<(Vec<HyprMonitor>, i32)> {
    let output = Command::new("hyprctl")
        .arg("monitors")
        .arg("-j")
        .output()
        .unwrap();

    let _vec: Vec<HyprClient> = vec![];

    let command_output = String::from_utf8(output.stdout).unwrap();

    let json = serde_json::from_str::<serde_json::Value>(command_output.as_str()).unwrap();
    let array = json.as_array().unwrap();

    let mut res = vec![];
    let mut cursor = 0;
    array.iter().for_each(|e| {
        let id = e.get("id").unwrap().as_i64().unwrap() as i32;

        if e.get("focused").unwrap().as_bool().unwrap() {
            cursor = id.clone();
        }

        res.push(HyprMonitor {
            id: Some(id as i64),
            name: e.get("name").unwrap().as_str().unwrap().to_string(),
        });
    });

    Ok((res, cursor))
}

pub fn get_workspaces() -> anyhow::Result<Vec<HyprWorkspace>> {
    let output = Command::new("hyprctl")
        .arg("workspaces")
        .arg("-j")
        .output()
        .unwrap();

    let out = String::from_utf8(output.stdout).unwrap();

    let json = serde_json::from_str::<serde_json::Value>(out.as_str()).unwrap();

    let mut vec = vec![];
    if let Some(arr) = json.as_array() {
        arr.iter().for_each(|e| {
            vec.push(HyprWorkspace {
                id: e.get("id").unwrap().as_i64().map(|i| i as i32),
                name: e.get("name").unwrap().as_str().unwrap().to_string(),
                monitor: HyprMonitor {
                    id: e.get("monitorID").unwrap().as_i64(),
                    name: e.get("monitor").unwrap().as_str().unwrap().to_string(),
                },
            });
        })
    }

    Ok(vec)
}

pub fn get_active_workspace() -> anyhow::Result<HyprWorkspace> {
    let output = Command::new("hyprctl")
        .arg("activeworkspace")
        .arg("-j")
        .output()?;

    let out = String::from_utf8(output.stdout).unwrap();

    let json = serde_json::from_str::<serde_json::Value>(out.as_str()).unwrap();

    match json.as_object() {
        Some(e) => Ok(HyprWorkspace {
            id: e.get("id").unwrap().as_i64().map(|i| i as i32),
            name: e.get("name").unwrap().as_str().unwrap().to_string(),
            monitor: HyprMonitor {
                id: e.get("monitorID").unwrap().as_i64(),
                name: e.get("monitor").unwrap().as_str().unwrap().to_string(),
            },
        }),
        None => Err(anyhow::anyhow!("unable to get active workspace")),
    }
}
