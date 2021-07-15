/*
 * Copyright (c) 2021 Ilya Venner
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use std::{io, path::PathBuf};

use clap::AppSettings;
use fern::colors::{Color, ColoredLevelConfig};
use structopt::StructOpt;

/// An util which can proxy of the given executable.
#[derive(Debug, StructOpt)]
#[structopt(global_settings = &[AppSettings::AllowNegativeNumbers])]
pub struct Opt {
    /// A telegram bot token.
    #[structopt(short = "t", long)]
    pub token: String,

    /// A master chat ID from which the bot will process messages.
    #[structopt(short = "c", long = "chat")]
    pub master_chat_id: i64,

    /// A path to the executable.
    #[structopt(short = "e", long = "executable")]
    pub path_to_executable: PathBuf,

    /// A pause duration between two sending messages.
    #[structopt(short = "p", long, default_value = "2s")]
    pub pause_duration: humantime::Duration,

    /// A strings which will ignored from proxying `stdout`.
    #[structopt(short = "f", long)]
    pub filter_dictionary: Option<Vec<String>>,

    /// Sets logging level to verbose.
    #[structopt(short = "v", long = "verbose")]
    pub is_verbose: bool,
}

/// Setups the application logger.
pub fn setup_logger(is_verbose: bool) {
    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Green)
        .debug(Color::Blue)
        .trace(Color::BrightBlack);

    let application_log_level = if is_verbose {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    };

    fern::Dispatch::new()
        .format(move |f, msg, rec| {
            f.finish(format_args!(
                "[{date} {level:>5}]: {message}",
                level = colors.color(rec.level()),
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                message = msg,
            ))
        })
        .level(log::LevelFilter::Warn)
        .level_for(std::env!("CARGO_PKG_NAME"), application_log_level)
        .chain(io::stdout())
        .apply()
        .expect("cannot set up the logger");
}
