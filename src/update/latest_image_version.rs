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

async fn image_tags(image: &Image) -> Result<Vec<String>> {
    let client_config = ClientConfig::default();
    let client = oci_client::Client::new(client_config);
    let reference = image.image.parse::<Reference>()?;

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
            .await
            .map(|r| r.tags)
            .unwrap_or(Vec::new());
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

struct VersionParser {
    version_formatter_regex: Regex,
    version_req: VersionReq,
}

const VERSION_REGEX: &str = r#"^[vV]?0*(?<major>0|[1-9]\d*)(?:\.0*(?<minor>0|[1-9]\d*))?(?:\.0*(?<patch>0|[1-9]\d*))?(?<suffix>.*)$"#;

impl VersionParser {
    fn new(version_req: VersionReq) -> Result<Self> {
        let version_formatter_regex = Regex::new(VERSION_REGEX)?;
        let vp = VersionParser {
            version_formatter_regex,
            version_req,
        };
        Ok(vp)
    }

    fn parse(&self, version: &str) -> Option<Version> {
        match self.version_formatter_regex.captures(version) {
            Some(caps) => {
                let major = caps.name("major").map_or("0", |c| c.as_str());
                let minor = caps.name("minor").map_or("0", |c| c.as_str());
                let patch = caps.name("patch").map_or("0", |c| c.as_str());
                let suffix = caps.name("suffix").map_or("", |c| c.as_str());
                let version = &format!("{major}.{minor}.{patch}{suffix}");
                match Version::parse(version) {
                    Ok(v) if self.version_req.matches(&v) => Some(v),
                    _ => None,
                }
            }
            None => None,
        }
    }
}

impl UpdateLatestImageVersion for Image {
    async fn update_latest_image_version(&mut self) -> Result<()> {
        let version_req = VersionReq::parse(&self.latest_version_req)?;
        let vp = VersionParser::new(version_req.clone())?;
        let tags = image_tags(self).await?;

        let mut latest_version: Option<String> = None;
        for tag in tags {
            latest_version = match vp.parse(&tag) {
                Some(v) => match &latest_version {
                    Some(lv) => match vp.parse(lv) {
                        Some(lv) if lv >= v => latest_version,
                        _ => Some(tag.to_string()),
                    },
                    _ => Some(tag.to_string()),
                },
                _ => latest_version,
            };
        }
        self.latest_version = latest_version;
        Ok(())
    }
}
