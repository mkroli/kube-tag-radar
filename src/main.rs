/*
 * Copyright 2025 Michael Krolikowski
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

mod database;
mod log;
mod observe;
mod serve;
mod settings;
mod update;

use ::log::info;
use anyhow::Result;
use clap::Parser;
use database::Database;
use log::LogError;
use observe::Observe;
use serve::Serve;
use settings::Settings;
use update::Update;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[clap(default_value = "config.yaml")]
    config_file: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    log::init()?;
    let cli = Cli::parse();
    let settings = Settings::read(&cli.config_file)?;
    let update_delay = settings.update_delay;
    let database = Database::new(settings.clone()).await?;

    let serve_task = {
        let serve = Serve::new(settings.clone(), database.clone());
        tokio::spawn(async move { serve.serve().await })
    };

    let observe_task = {
        let observe = Observe::new(database.clone());
        tokio::spawn(async move { observe.observe().await })
    };

    let update_task = {
        let tick_interval = settings.tick_interval;
        let update = Update::new(settings, database);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval_at(update_delay, tick_interval);
            loop {
                interval.tick().await;
                update.update_all().await.log_error();
            }
        })
    };

    info!("Started");
    tokio::select! {
        r = serve_task => r,
        r = observe_task => r,
        r = update_task => r,
    }??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::Cli;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
