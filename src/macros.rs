// /// Tries to get the value from a specific environment variable.
// pub fn try_get_var(variable: &str, fallback_value: &str) -> String {
//     std::env::var(variable).unwrap_or_else(|_| fallback_value.to_owned())
// }

// #[macro_export]
// /// Logs a [HYBRID] [DEBUG] formatted message to stdout.
// macro_rules! log {
//     ($msg:expr) => {
//         // if try_get_var("HYBRID_LOG", "0") == "1" {
//             println!("[LOG]: {}", $msg)
//         // }
//     };
// }

