#[cfg(feature = "cli")]
use clap::Parser;

#[cfg_attr(feature = "cli", derive(Parser))]
#[cfg_attr(feature = "cli", command(name = "myapp", version, about = "A sample Rust application"))]
struct Args {
    #[cfg(feature = "cli")]
    #[arg(short, long, default_value = "world")]
    name: String,

    #[cfg(feature = "cli")]
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    env_logger::init();

    #[cfg(feature = "cli")]
    let args = Args::parse();

    #[cfg(feature = "cli")]
    let name = &args.name;
    #[cfg(not(feature = "cli"))]
    let name = "world";

    log::info!("greeting {name}");
    println!("Hello, {name}!");
}
