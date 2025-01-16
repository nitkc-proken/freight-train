use clap::Parser;
use commands::Args;

mod commands;
fn main() {
    let args = Args::parse();
    println!("{:#?}", args);
    println!("Hello, world!{:?}", common::add(1, 2));
}
