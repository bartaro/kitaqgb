use clap::Parser;

fn main() -> anyhow::Result<()> {
    gbseq::cli::HeadlessCli::parse().run()
}
