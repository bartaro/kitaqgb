fn main() {
    if let Err(error) = kitaqgb::vblank_patch::cli(&std::env::args().skip(1).collect::<Vec<_>>()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
