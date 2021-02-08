use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum Error {
    #[snafu(display("{}", source))]
    IoError { source: std::io::Error },

    #[snafu(display("{}", source))]
    ExecuteError { source: carapax::ExecuteError },
}
