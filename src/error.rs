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

use snafu::Snafu;

/// A main error.
#[derive(Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum Error {
    /// Indicates that executable could not be run.
    #[snafu(display("Cannot spawn executable `{}`: {}.", path_to_executable.display(), source))]
    CannotSpawnProcess {
        /// A path to executable.
        path_to_executable: PathBuf,

        /// Source error.
        source: io::Error,
    },

    /// Indicate the running executable have not `stdin` stream.
    #[snafu(display(
        "Cannot find `stdin` stream in the given executable: `{}`.", path_to_executable.display()
    ))]
    NoStdIn {
        /// A path to executable.
        path_to_executable: PathBuf,
    },

    /// Indicate the running executable have not `stdout` stream.
    #[snafu(display(
        "Cannot find `stdout` stream in the given executable: `{}`.", path_to_executable.display()
    ))]
    NoStdOut {
        /// A path to executable.
        path_to_executable: PathBuf,
    },

    /// Indicate the running executable have not `stderr` stream.
    #[snafu(display(
        "Cannot find `stderr` stream in the given executable: `{}`.", path_to_executable.display()
    ))]
    NoStdErr {
        /// A path to executable.
        path_to_executable: PathBuf,
    },

    /// Indicates that Telegram API could not be created.
    #[snafu(display("Cannot create Telegram API: {}.", source))]
    TelegramApi {
        /// Source error.
        source: carapax::ApiError,
    },

    /// Indicates that data cannot be written to the `stdin` stream.
    #[snafu(display("Cannot write to `stdin`: {}.", source))]
    WriteExecutableStdIn {
        /// Source error.
        source: io::Error,
    },

    /// Indicates that bot cannot send the message.
    #[snafu(display("Cannot send text message {}.", source))]
    SendMessage {
        /// Source error.
        source: carapax::ExecuteError,
    },

    /// Indicates that [timeout](tokio::time::timeout) future completed unexpectedly.
    #[snafu(display("An unexpected timeout future completion."))]
    TimeoutCompletion {
        /// Source error.
        source: io::Error,
    },
}
