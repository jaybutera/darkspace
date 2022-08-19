use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "blackhole", about = "Encrypted voice communications")]
pub struct Opt {
    #[structopt(short)]
    pub call: bool,

    #[structopt(short, long)]
    pub from: String,

    /// Address to connect with, of the form <ip>:<port> i.e. 127.0.0.1:8080
    #[structopt(short, long)]
    pub address: String,
}
