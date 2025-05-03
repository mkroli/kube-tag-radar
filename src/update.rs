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

mod image_ids;
mod latest_image_version;
mod version;

use crate::database::{Database, Image};
use crate::log::LogError;
use anyhow::Result;
use image_ids::ImageIds;
use latest_image_version::LatestImageVersion;
use log::info;
use version::ImageVersion;

pub struct Update {
    database: Database,
}

impl Update {
    pub fn new(database: Database) -> Update {
        Update { database }
    }

    async fn update_image(&self, image: &Image) -> Image {
        info!("Updating {}.", &image.image);
        let version = image.image_version().log_error().flatten();
        let latest_version = image.latest_image_version().await.log_error().flatten();

        let image_ids = image.image_ids(&image.latest_tag.clone()).await.log_error();
        let image_ids = match image_ids {
            Some((resolved_image_id, latest_image_id)) => {
                Some((resolved_image_id, latest_image_id))
            }
            None => match &latest_version {
                Some(latest_version) => image.image_ids(latest_version).await.log_error(),
                None => None,
            },
        };
        let (resolved_image_id, latest_image_id) = match image_ids {
            None => (None, None),
            Some((resolved_image_id, latest_image_id)) => {
                (Some(resolved_image_id), Some(latest_image_id))
            }
        };

        Image {
            version,
            latest_version,
            resolved_image_id,
            latest_image_id,
            ..image.clone()
        }
    }

    pub async fn update_all(&self) -> Result<()> {
        self.database.delete_unused_images().await?;
        let images = self.database.list_image().await?;
        for image in &images {
            let image = self.update_image(image).await;
            self.database.update_image_details(&image).await?;
        }
        Ok(())
    }
}
