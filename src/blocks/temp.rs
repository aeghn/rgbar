use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
};

use glob::glob;

pub fn match_type_dir(type_name: &str) -> anyhow::Result<PathBuf> {
    let entries = glob("/sys/class/thermal/thermal_zone*/type")?;
    let mut s = String::new();
    for entry in entries {
        let pathbuf = entry?;
        let file = File::open(&pathbuf)?;
        let mut reader = BufReader::new(file);
        s.clear();
        let _content_size = reader.read_to_string(&mut s);
        if s.trim_end() == type_name {
            match pathbuf.parent() {
                Some(p) => return Ok(p.to_owned()),
                None => anyhow::bail!("no file name"),
            }
        }
    }
    anyhow::bail!("unable to get dir")
}

pub fn read_type_temp(temp_file: &PathBuf) -> anyhow::Result<f64> {
    let mut s = String::new();
    let file = File::open(&temp_file)?;
    let mut reader = BufReader::new(file);

    let _content_size = reader.read_to_string(&mut s)?;
    let temp = i64::from_str_radix(s.trim_end(), 10)?;

    Ok(temp as f64 / 1000.)
}
