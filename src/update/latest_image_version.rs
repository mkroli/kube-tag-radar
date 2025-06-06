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
use regex::{Match, Regex};
use semver::{Version, VersionReq};

use crate::database::Image;

pub trait LatestImageVersion {
    async fn latest_image_version(&self) -> Result<Option<String>>;
}

async fn image_name_tags(image: &str) -> Result<Vec<String>> {
    let client_config = ClientConfig::default();
    let client = oci_client::Client::new(client_config);
    let reference = image.parse::<Reference>()?;

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

async fn image_tags(image: &Image) -> Result<Vec<String>> {
    match image_name_tags(&image.image_id).await? {
        v if !v.is_empty() => Ok(v),
        _ => image_name_tags(&image.image).await,
    }
}

impl LatestImageVersion for Image {
    async fn latest_image_version(&self) -> Result<Option<String>> {
        let version_req = VersionReq::parse(&self.latest_version_req)?;
        let vp = VersionParser::new(version_req.clone())?;
        let version_regex = Regex::new(&self.latest_version_regex)?;

        let latest_version = image_tags(self)
            .await?
            .into_iter()
            .flat_map(|v| match version_regex.captures(&v) {
                Some(c) if c.len() >= 2 => Some((c[0].to_string(), c[1].to_string())),
                Some(c) if c.len() == 1 => Some((c[0].to_string(), c[0].to_string())),
                _ => None,
            })
            .flat_map(|(v, s)| vp.parse(&s).map(|version| (v, version)))
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(v, _)| v.to_string());
        Ok(latest_version)
    }
}

struct VersionParser {
    version_formatter_regex: Regex,
    version_req: VersionReq,
}

const VERSION_REGEX: &str = r#"^[vV]?(?<major>0|[0-9]\d*)(?:\.0*(?<minor>0|[0-9]\d*))?(?:\.0*(?<patch>0|[0-9]\d*))?(?<suffix>.*)$"#;

impl VersionParser {
    fn new(version_req: VersionReq) -> Result<Self> {
        let version_formatter_regex = Regex::new(VERSION_REGEX)?;
        let vp = VersionParser {
            version_formatter_regex,
            version_req,
        };
        Ok(vp)
    }

    fn best_effort_version(
        major: Option<&str>,
        minor: Option<&str>,
        patch: Option<&str>,
        suffix: Option<&str>,
    ) -> String {
        let (major, minor, patch) = match (major, minor, patch) {
            (Some(major), Some(minor), Some(patch)) => (major, minor, patch),
            (Some(major), Some(minor), None) => (major, minor, "0"),
            (Some(major), None, None) => ("0", "0", major),
            _ => ("0", "0", "0"),
        };
        let suffix = suffix.unwrap_or("");
        format!("{major}.{minor}.{patch}{suffix}")
    }

    fn parse(&self, version: &str) -> Option<Version> {
        fn as_str(c: Match<'_>) -> &str {
            c.as_str()
        }
        match self.version_formatter_regex.captures(version) {
            Some(caps) => {
                let major = caps.name("major").map(as_str);
                let minor = caps.name("minor").map(as_str);
                let patch = caps.name("patch").map(as_str);
                let suffix = caps.name("suffix").map(as_str);
                let version = VersionParser::best_effort_version(major, minor, patch, suffix);
                match Version::parse(&version) {
                    Ok(v) if self.version_req.matches(&v) => Some(v),
                    _ => None,
                }
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_best_effort_version_order() -> Result<()> {
        let vp = VersionParser::new(VersionReq::parse("*")?)?;
        let versions = vec![
            vp.parse("test"),
            vp.parse("2"),
            vp.parse("0.1"),
            vp.parse("0.3"),
            vp.parse("1.0"),
            vp.parse("1.0.1"),
            vp.parse("2.0"),
        ];
        assert!(versions.is_sorted());
        Ok(())
    }
}
