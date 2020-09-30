use init::{setup_logger, Opt};
use structopt::StructOpt;

mod init;

fn main() {
    setup_logger();

    let Opt {
        token,
        path_to_binary,
    } = StructOpt::from_args();

    println!("{}, {:?}", token, path_to_binary);
}
