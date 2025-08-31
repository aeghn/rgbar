use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use chin_tools::{aanyhow, AResult};
use glob::glob;

pub fn match_type_dir(type_name: &str) -> AResult<PathBuf> {
    let entries = glob("/sys/class/thermal/thermal_zone*/type")?;
    let mut s = String::new();
    for entry in entries {
        let pathbuf = entry?;
        let file = File::open(&pathbuf)?;
        let mut reader = BufReader::new(file);
        s.clear();
        let _content_size = reader.read_to_string(&mut s);
        if s.trim_end() == type_name {
            if let Some(p) = pathbuf.parent() { return Ok(p.to_owned()) }
        }
    }
    Err(aanyhow!("unable to get dir"))
}

pub fn read_type_temp(temp_file: &PathBuf) -> AResult<f64> {
    let mut s = String::new();
    let file = File::open(temp_file)?;
    let mut reader = BufReader::new(file);

    let _content_size = reader.read_to_string(&mut s)?;
    let temp = i64::from_str_radix(s.trim_end(), 10)?;

    Ok(temp as f64 / 1000.)
}
