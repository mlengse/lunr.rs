pub fn warn(message: &str) {
    eprintln!("WARNING: {}", message);
}

pub fn as_string(obj: Option<&dyn std::fmt::Display>) -> String {
    match obj {
        Some(val) => val.to_string(),
        None => String::new(),
    }
}
