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

mod image_id;
mod latest_image_version;
mod version;

use crate::database::{Database, Image};
use crate::log::LogError;
use crate::settings::Settings;
use anyhow::Result;
use image_id::UpdateImageId;
use k8s_openapi::api::core::v1::Pod;
use kube::runtime::reflector::Lookup;
use kube::{Api, api::ListParams};
use latest_image_version::UpdateLatestImageVersion;
use log::info;
use version::UpdateImageVersion;

pub struct Update {
    settings: Settings,
    database: Database,
}

impl Update {
    pub fn new(settings: Settings, database: Database) -> Result<Update> {
        let update = Update { settings, database };
        Ok(update)
    }

    async fn fetch_images(&self) -> Result<Vec<Image>> {
        let mut pcis = Vec::new();
        let client = kube::Client::try_default().await?;
        let pods: Api<Pod> = Api::all(client);
        for p in pods.list(&ListParams::default()).await? {
            let annotations = &p.metadata.annotations.as_ref();
            let latest_tag: String = annotations
                .map(|a| a.get("kube-tag-radar.mkroli.com/tag"))
                .flatten()
                .cloned()
                .unwrap_or("latest".to_string());
            let latest_version_req: String = annotations
                .map(|a| a.get("kube-tag-radar.mkroli.com/version_req"))
                .flatten()
                .cloned()
                .unwrap_or("*".to_string());

            if let (Some(namespace), Some(pod_name), Some(status)) =
                (&p.namespace(), &p.name(), &p.status)
            {
                if let Some(container_statuses) = &status.container_statuses {
                    for c in container_statuses {
                        let image = Image {
                            namespace: namespace.to_string(),
                            pod: pod_name.to_string(),
                            container: c.name.to_string(),
                            image: c.image.to_string(),
                            image_id: c.image_id.to_string(),
                            latest_tag: latest_tag.to_string(),
                            latest_image_id: None,
                            version: None,
                            latest_version_req: latest_version_req.to_string(),
                            latest_version: None,
                        };
                        pcis.push(image);
                    }
                }
            }
        }
        Ok(pcis)
    }

    async fn update_image(&self, image: &mut Image) {
        info!("Updating {}.", &image.image);
        image.update_version().log_error();
        image.update_latest_image_version().await.log_error();
        image.update_image_id().await.log_error();

        if let (None, Some(latest_version)) = (&image.latest_image_id, &image.latest_version) {
            image.latest_tag = latest_version.to_string();
            image.update_image_id().await.log_error();
        }
    }

    pub async fn update_all(&self) -> Result<()> {
        let mut images = self.fetch_images().await?;
        images.retain(|image| {
            !self
                .settings
                .ignore
                .iter()
                .any(|ignore| ignore.matches(image))
        });
        self.database.retain(&images).await?;
        for image in &mut images {
            self.update_image(image).await;
            self.database.replace(image).await?;
        }
        Ok(())
    }
}
