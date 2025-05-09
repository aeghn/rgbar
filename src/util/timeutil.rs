pub fn second_to_human(secs: usize) -> String {
    let minute = secs % 3600 / 60;
    let hour = secs / 3600;

    if hour > 0 {
        format!("{:02}:{:02}", hour, minute)
    } else {
        format!("00:{}", minute)
    }
}
