use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "blackhole", about = "Encrypted voice communications")]
struct Opt {
    #[structopt(short)]
    call: bool,
}
