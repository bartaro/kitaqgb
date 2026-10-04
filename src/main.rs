fn main() -> std::process::ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut session = kitaqgb::observation::Session::default();
    if args.iter().any(|a| a.eq_ignore_ascii_case("--watch"))
        || args
            .first()
            .is_some_and(|a| a.eq_ignore_ascii_case("devserver"))
    {
        let token = kitaqgb::drivers::CancellationToken::new();
        let handler = token.clone();
        if let Err(error) = ctrlc::set_handler(move || handler.cancel()) {
            eprintln!("warning KQ0000: Ctrl+C handler: {error}");
        }
        session.cancellation_token = Some(token);
    }
    std::process::ExitCode::from(kitaqgb::cli::invoke(args, &mut session).clamp(0, 255) as u8)
}
