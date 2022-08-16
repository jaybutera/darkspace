use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "blackhole", about = "Encrypted voice communications")]
pub struct Opt {
    #[structopt(short)]
    pub call: bool,
}
