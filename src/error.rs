use std::io;

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("{}", source))]
    IoError { source: io::Error },

    #[snafu(display("{}", source))]
    ExecuteError { source: carapax::ExecuteError },
}
