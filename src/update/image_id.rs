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
use oci_client::{Reference, client::ClientConfig};

use crate::database::Image;

pub trait UpdateImageId {
    async fn update_image_id(&mut self) -> Result<()>;
}

impl UpdateImageId for Image {
    async fn update_image_id(&mut self) -> Result<()> {
        let client_config = ClientConfig::default();
        let client = oci_client::Client::new(client_config);

        let reference = self.image.parse::<Reference>()?;
        let reference_digest = match reference.digest() {
            Some(digest) => digest.to_string(),
            None => {
                client
                    .fetch_manifest_digest(
                        &reference,
                        &oci_client::secrets::RegistryAuth::Anonymous,
                    )
                    .await?
            }
        };
        let reference = Reference::with_digest(
            reference.registry().to_string(),
            reference.repository().to_string(),
            reference_digest,
        );

        let latest = Reference::with_tag(
            reference.registry().to_string(),
            reference.repository().to_string(),
            self.latest_tag.to_string(),
        );
        let latest_digest = client
            .fetch_manifest_digest(&latest, &oci_client::secrets::RegistryAuth::Anonymous)
            .await?;

        let latest = Reference::with_digest(
            reference.registry().to_string(),
            reference.repository().to_string(),
            latest_digest,
        );

        self.image_id = reference.whole();
        self.latest_image_id = Some(latest.whole());
        Ok(())
    }
}
