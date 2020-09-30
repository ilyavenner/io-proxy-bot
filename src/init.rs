use std::path::PathBuf;

use carapax::types::Integer;
use clap::AppSettings;
use structopt::StructOpt;

/// An util which provides managing of the Minecraft Bedrock Server.
#[derive(Debug, StructOpt)]
#[structopt(global_settings = &[AppSettings::AllowNegativeNumbers])]
pub struct Opt {
    /// A telegram bot token.
    pub token: String,

    /// A master chat ID.
    pub master_chat_id: Integer,

    /// A path to the server binary.
    pub path_to_binary: PathBuf,
}

/// Setups application logger.
pub fn setup_logger() {
    use fern::{
        colors::{Color, ColoredLevelConfig},
        Dispatch,
    };

    let colors_line = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::White)
        .debug(Color::White)
        .trace(Color::BrightBlack);

    let colors_level = colors_line.clone().info(Color::Green);

    Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{color_line}[{date}] [{target}] [{level}{color_line}] {message}\x1B[0m",
                color_line = format_args!(
                    "\x1B[{}m",
                    colors_line.get_color(&record.level()).to_fg_str()
                ),
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                target = record.target(),
                level = colors_level.color(record.level()),
                message = message,
            ));
        })
        .level(log::LevelFilter::Warn)
        .level_for("rusty_manager_bot", log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .apply()
        .expect("cannot setup logger");
}
