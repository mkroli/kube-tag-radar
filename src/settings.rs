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

use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    time::Duration,
};

use crate::database::ImageWithContainer;
use anyhow::Result;
use config::{Config, Environment, File, FileFormat};
use serde::{Deserialize, Deserializer};
use tokio::time::Instant;

#[derive(Deserialize, Clone)]
pub struct Ignore {
    pub namespace: String,
    pub image: String,
}

impl Ignore {
    pub fn matches(&self, container: &ImageWithContainer) -> bool {
        self.namespace == container.namespace && self.image == container.image
    }
}

#[derive(Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "default_database")]
    pub database: String,
    #[serde(deserialize_with = "parse_instant", default = "default_update_delay")]
    pub update_delay: Instant,
    #[serde(
        deserialize_with = "parse_duration",
        default = "default_update_interval"
    )]
    pub update_interval: Duration,
    #[serde(deserialize_with = "parse_duration", default = "default_tick_interval")]
    pub tick_interval: Duration,
    #[serde(default = "default_bind_address")]
    pub bind_address: SocketAddr,
    #[serde(default = "Vec::new")]
    pub ignore: Vec<Ignore>,
}

fn default_database() -> String {
    "./kube-tag-radar.sqlite".to_string()
}

fn default_update_delay() -> Instant {
    Instant::now() + Duration::from_secs(5 * 60)
}

fn default_update_interval() -> Duration {
    Duration::from_secs(3 * 60 * 60)
}

fn default_tick_interval() -> Duration {
    Duration::from_secs(60)
}

fn default_bind_address() -> SocketAddr {
    SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 8080)
}

fn parse_duration<'d, D: Deserializer<'d>>(
    deserializer: D,
) -> std::result::Result<Duration, D::Error> {
    let s: String = Deserialize::deserialize(deserializer)?;
    let d = s
        .parse::<iso8601_duration::Duration>()
        .map_err(|_| serde::de::Error::custom("Cannot parse ISO-8601 duration"))?;
    d.to_std()
        .ok_or(serde::de::Error::custom("Cannot convert to duration"))
}

fn parse_instant<'d, D: Deserializer<'d>>(
    deserializer: D,
) -> std::result::Result<Instant, D::Error> {
    let duration = parse_duration(deserializer)?;
    let start = Instant::now() + duration;
    Ok(start)
}

impl Settings {
    pub fn read(filename: &str) -> Result<Settings> {
        let config = Config::builder()
            .add_source(
                File::with_name(filename)
                    .format(FileFormat::Yaml)
                    .required(false),
            )
            .add_source(Environment::with_prefix("KTR"))
            .build()?;
        let settings = config.try_deserialize()?;
        Ok(settings)
    }
}
