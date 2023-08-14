use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::process::Command;

#[derive(Clone, Debug)]
pub struct HyprWorkspace {
    pub id: i64,
    pub monitor: String,
    pub name: String,
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
        if self.monitor != "eDP-1" {
            format!(" {}", self.id)
        } else {
            self.id.to_string()
        }
    }
}

#[derive(Clone)]
pub struct HyprClient {
    pub class: String,
    pub title: String,
    pub address: String,
    pub mapped: bool,
    pub hidden: bool,
    pub pid: i64,
    pub xwayland: bool,
    pub workspace: HyprWorkspace,
}

pub fn get_clients() -> Result<Vec<HyprClient>, String> {
    let output = Command::new("hyprctl")
        .arg("clients")
        .arg("-j")
        .output()
        .unwrap();

    let monitors = get_monitors();

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
                workspace: HyprWorkspace {
                    id: e
                        .get("workspace")
                        .unwrap()
                        .get("id")
                        .unwrap()
                        .as_i64()
                        .unwrap(),
                    monitor: monitors.get(&monitor).unwrap().to_string(),
                    name: e
                        .get("workspace")
                        .unwrap()
                        .get("name")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                },
            })
        }
    }
    Ok(vec)
}

pub fn get_active_window_address() -> Option<String> {
    let output = Command::new("hyprctl")
        .arg("activewindow")
        .arg("-j")
        .output()
        .unwrap();

    let _vec: Vec<HyprClient> = vec![];

    let out = String::from_utf8(output.stdout).unwrap();

    let json = serde_json::from_str::<serde_json::Value>(out.as_str()).unwrap();

    if let Some(obj) = json.as_object() {
        if let Some(address) = obj.get("address") {
            Some(address.as_str().unwrap().to_string())
        } else {
            None
        }
    } else {
        None
    }
}

pub fn get_monitors() -> HashMap<i64, String> {
    let output = Command::new("hyprctl")
        .arg("monitors")
        .arg("-j")
        .output()
        .unwrap();

    let _vec: Vec<HyprClient> = vec![];

    let out = String::from_utf8(output.stdout).unwrap();

    let json = serde_json::from_str::<serde_json::Value>(out.as_str()).unwrap();

    let mut map = HashMap::new();
    if let Some(arr) = json.as_array() {
        arr.iter().for_each(|e| {
            map.insert(
                e.get("id").unwrap().as_i64().unwrap(),
                e.get("name").unwrap().as_str().unwrap().to_string(),
            );
        })
    }

    map
}
