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

use anyhow::Result;
use log::error;
use stderrlog::Timestamp;

pub trait LogError<T> {
    fn log_error(self) -> Option<T>;
}

impl<T> LogError<T> for Result<T> {
    fn log_error(self) -> Option<T> {
        match self {
            Ok(v) => Some(v),
            Err(e) => {
                error!("Error: {e}");
                None
            }
        }
    }
}

pub fn init() -> Result<()> {
    stderrlog::new()
        .verbosity(log::Level::Info)
        .timestamp(Timestamp::Millisecond)
        .init()?;
    Ok(())
}
