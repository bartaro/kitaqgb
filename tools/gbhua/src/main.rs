#![cfg_attr(
    all(target_os = "windows", not(feature = "gui-console")),
    windows_subsystem = "windows"
)]

fn main() -> plita::prelude::Result<()> {
    gbhua_lib::run_gui()
}
