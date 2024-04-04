#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(non_snake_case)]

use crate::args::{CliArgs, VerbosityLevel};
use crate::config::CONFIG_NAME;
use clap::Parser;
use directories::ProjectDirs;
use std::fs;
use std::sync::Arc;
use tracing::*;

use crate::database::establish_connection;
use crate::ui::open_gui;

mod args;
mod config;
mod database;
mod source;
mod ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let args = CliArgs::parse();

    // setup logging (tracing)
    let subscriber = tracing_subscriber::fmt()
        .with_file(false)
        .with_line_number(false)
        .with_max_level(match args.verbosity {
            VerbosityLevel::TRACE => Level::TRACE,
            VerbosityLevel::DEBUG => Level::DEBUG,
            VerbosityLevel::INFO => Level::INFO,
            VerbosityLevel::WARN => Level::WARN,
            VerbosityLevel::ERROR => Level::ERROR,
        })
        .finish();

    subscriber::set_global_default(subscriber).unwrap();

    if args.reset_database || args.reset_config {
        if args.reset_database {
            debug!("Deleting DB file");

            let db_path = ProjectDirs::from("com", "tgz39", "saveit")
                .unwrap()
                .data_dir()
                .to_owned();
            let db_loc = format!(
                "{}/{}",
                &db_path.to_str().unwrap().to_owned(),
                db_version!()
            );

            fs::remove_file(db_loc).expect("Error deleting DB file");
        }
        if args.reset_config {
            debug!("Deleting config file");

            let loc = confy::get_configuration_file_path(CONFIG_NAME, None)
                .expect("Error loading config");

            fs::remove_file(loc).expect("Error deleting config file");
        }
        return Ok(());
    }

    // setup database
    debug!("Executing database migrations...");
    let pool = Arc::new(
        establish_connection()
            .await
            .expect("Error connection to database"),
    );

    // setup table
    sqlx::migrate!("./migrations")
        .run(&*pool)
        .await
        .expect("Error executing database migrations");

    // open GUI
    open_gui(pool.clone()).expect("Error opening GUI");

    Ok(())
}
