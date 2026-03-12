use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "localhost", env = "PROXY_SERVER")]
    pub server: String,
    #[arg(long, default_value_t = 8080, env = "PROXY_PORT")]
    pub port: u16,
    #[arg(long)]
    pub service: bool,
    #[arg(short, long, env = "MACHINE_ID")]
    pub machine_id: Option<String>,
    #[arg(short, long, env = "PASSWORD")]
    pub password: Option<String>,
}
