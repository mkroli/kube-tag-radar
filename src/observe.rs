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

use crate::{
    database::{Container, Database, PodInfo},
    settings::{Override, Settings},
};
use anyhow::Result;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    Api, Client,
    runtime::{
        WatchStreamExt, watcher,
        watcher::{Config, Event},
    },
};
use std::{collections::BTreeMap, pin::pin};
use tokio_stream::StreamExt;

pub struct Observe {
    database: Database,
}

fn pod_settings(
    overrides: &Option<&Override>,
    annotations: &BTreeMap<String, String>,
    container: &str,
) -> (String, String, String) {
    let from_annotations = |t: &str| -> Option<String> {
        annotations
            .get(&format!("kube-tag-radar.mkroli.com/{container}.{t}"))
            .or_else(|| annotations.get(&format!("kube-tag-radar.mkroli.com/{t}")))
            .cloned()
    };
    let tag = from_annotations("tag")
        .or(overrides.and_then(|o| o.tag.clone()))
        .unwrap_or("latest".to_string());
    let req = from_annotations("version_req")
        .or(overrides.and_then(|o| o.version_req.clone()))
        .unwrap_or("*".to_string());
    let regex = from_annotations("version_regex")
        .or(overrides.and_then(|o| o.version_regex.clone()))
        .unwrap_or(".*".to_string());
    (tag, req, regex)
}

impl PodInfo for Pod {
    fn namespace(&self) -> Option<String> {
        self.metadata.namespace.clone()
    }

    fn name(&self) -> Option<String> {
        self.metadata.name.clone()
    }

    fn containers(&self, settings: &Settings) -> Vec<Container> {
        let mut containers = Vec::new();

        let annotations = match &self.metadata.annotations {
            Some(a) => a,
            None => &BTreeMap::new(),
        };

        if let (Some(namespace), Some(pod_name), Some(status)) =
            (PodInfo::namespace(self), PodInfo::name(self), &self.status)
            && let Some(container_statuses) = &status.container_statuses
        {
            for c in container_statuses {
                let overrides = settings.find_override(&namespace, &pod_name, &c.name);
                let (latest_tag, latest_version_req, latest_version_regex) =
                    pod_settings(&overrides, annotations, &c.name);

                let container = Container {
                    namespace: namespace.to_string(),
                    pod: pod_name.to_string(),
                    container: c.name.to_string(),
                    image: c.image.to_string(),
                    image_id: c.image_id.to_string(),
                    latest_tag: latest_tag.to_string(),
                    latest_version_req: latest_version_req.to_string(),
                    latest_version_regex: latest_version_regex.to_string(),
                };
                containers.push(container);
            }
        }
        containers
    }
}

impl Observe {
    pub fn new(database: Database) -> Observe {
        Observe { database }
    }

    pub async fn observe(&self) -> Result<()> {
        let client = Client::try_default().await?;
        let api = Api::<Pod>::all(client);
        let mut changes = pin!(watcher(api, Config::default()).default_backoff());
        self.database.truncate_containers().await?;
        while let Some(event) = changes.try_next().await? {
            match event {
                Event::Delete(pod) => {
                    self.database.delete_pod(&pod).await?;
                }
                Event::InitApply(pod) | Event::Apply(pod) => {
                    self.database.replace_pod(&pod).await?;
                }
                _ => (),
            }
        }
        Ok(())
    }
}
