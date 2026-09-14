#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    if let Err(error) = share_to_ytdlp::share_target::run() {
        share_to_ytdlp::share_target::show_fatal_error(&format!("{error:#}"));
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("The Windows Share target is supported only on Windows.");
    std::process::exit(1);
}
