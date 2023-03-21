use clap::Parser;

fn main() {
    let args = clipd::Args::parse();
    if let Err(e) = clipd::run(args) {
        eprintln!("Error: {:?}", e);
    }
}
