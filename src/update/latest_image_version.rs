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
use regex::Regex;
use semver::{Version, VersionReq};

use crate::database::Image;

pub trait UpdateLatestImageVersion {
    async fn update_latest_image_version(&mut self) -> Result<()>;
}

const VERSION_REGEX: &str = r"^v?(.*)$";

async fn image_tags(image: &Image) -> Result<Vec<String>> {
    let client_config = ClientConfig::default();
    let client = oci_client::Client::new(client_config);
    let reference = image.image_id.parse::<Reference>()?;

    let mut tags = Vec::new();
    let mut last: Option<String> = None;
    loop {
        let mut page = client
            .list_tags(
                &reference,
                &oci_client::secrets::RegistryAuth::Anonymous,
                None,
                last.as_deref(),
            )
            .await?
            .tags;
        match page.split_last_mut() {
            Some((l, _)) if Some(l.to_string()) == last => break,
            Some((l, page)) => {
                tags.extend_from_slice(page);
                tags.push(l.to_string());
                last = Some(l.to_string());
            }
            _ => break,
        }
    }
    Ok(tags)
}

impl UpdateLatestImageVersion for Image {
    async fn update_latest_image_version(&mut self) -> Result<()> {
        let version_regex = Regex::new(VERSION_REGEX)?;
        let tags = image_tags(self).await?;
        let version_req = VersionReq::parse(&self.latest_version_req)?;

        let mut latest_version: Option<String> = None;
        for tag in tags {
            latest_version = match version_regex.captures(&tag) {
                Some(caps) => match Version::parse(&caps[1]) {
                    Ok(v) if version_req.matches(&v) => match latest_version {
                        Some(ref lv) => match version_regex.captures(lv) {
                            Some(lvcaps) => match Version::parse(&lvcaps[1]) {
                                Ok(lv) if lv >= v => latest_version,
                                _ => Some(tag.to_string()),
                            },
                            _ => Some(tag.to_string()),
                        },
                        _ => Some(tag.to_string()),
                    },
                    _ => latest_version,
                },
                _ => latest_version,
            };
        }
        self.latest_version = latest_version;
        Ok(())
    }
}
