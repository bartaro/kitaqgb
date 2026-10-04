fn main() {
    if let Err(error) =
        kitaqgb::zx0::cli("kitaqgb-zx0", &std::env::args().skip(1).collect::<Vec<_>>())
    {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
