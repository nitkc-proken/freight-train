use clap::Parser;

#[derive(Parser, Debug)]
pub struct Example {
    #[arg(short, long)]
    name: String,
}

impl Example {
    pub async fn run(&self) {
        println!("Hello, world! {:?}", self.name);
    }
}
