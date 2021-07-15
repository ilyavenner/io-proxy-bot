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

//! An util which can proxy of the given executable.

#![deny(missing_docs)]

use std::{process::Stdio, sync::Arc};

use crate::bot::Service;
use snafu::{OptionExt, ResultExt};
use structopt::StructOpt;
use teloxide::prelude::*;
use tokio::{io::BufReader, process::Command, sync::Mutex};

/// Bot items like [context](Context) and [message handler](MessageHandler).
mod bot;

/// Initialization items like argument parser description and logger.
mod init;

#[tokio::main]
async fn main() {}

async fn run() -> Result<(), Error> {
    let init::Opt {
        token,
        master_chat_id,
        path_to_executable,
        pause_duration,
        filter_dictionary,
        is_verbose,
    } = StructOpt::from_args();

    init::setup_logger(is_verbose);
    log::trace!("logger initialized");

    let executable = Command::new(&path_to_executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context(CannotSpawnProcess {
            path_to_executable: &path_to_executable,
        })?;

    log::info!("instance of `{}` spawned", path_to_executable.display());

    let executable_stdin = executable.stdin.context(NoStdIn {
        path_to_executable: &path_to_executable,
    })?;
    log::trace!("`stdin` of instance captured");

    let executable_stdout = executable.stdout.context(NoStdOut {
        path_to_executable: &path_to_executable,
    })?;
    log::trace!("`stdout` of instance captured");

    let executable_stderr = executable.stderr.context(NoStdErr {
        path_to_executable: &path_to_executable,
    })?;
    log::trace!("`stderr` of instance captured");

    let filter_dictionary = filter_dictionary
        .map(|strings| strings.into_iter().collect())
        .unwrap_or_default();

    let service = Arc::new(Service::new(
        master_chat_id,
        Mutex::new(executable_stdin),
        *pause_duration,
        filter_dictionary,
    ));

    let copy_of_service = Arc::clone(&service);
    let reader_of_executable_stdout = tokio::spawn(async move {
        let stdout_reader = BufReader::new(executable_stdout);
        copy_of_service
            .stream_executable_output(stdout_reader)
            .await
    });
    log::trace!("`stdout` reader was ran");

    let copy_of_service = Arc::clone(&service);
    let reader_of_executable_stderr = tokio::spawn(async move {
        let stderr_reader = BufReader::new(executable_stderr);
        service.stream_executable_output(stderr_reader).await
    });
    log::trace!("`stderr` reader was ran");

    log::info!("running the bot...");
    service.run(token).await;

    reader_of_executable_stdout.await.unwrap_or_else(|e| {
        log::error!("cannot join `executable_stdout_reader`: {}", e);
        Ok(())
    })?;

    reader_of_executable_stderr.await.unwrap_or_else(|e| {
        log::error!("cannot join `executable_stdout_reader`: {}", e);
        Ok(())
    })?;

    Ok(())
}
