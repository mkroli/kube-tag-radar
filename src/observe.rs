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
    database::{Container, Database},
    settings::Settings,
};
use anyhow::Result;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    Api, Client,
    runtime::{
        WatchStreamExt,
        reflector::Lookup,
        watcher,
        watcher::{Config, Event},
    },
};
use std::pin::pin;
use tokio_stream::StreamExt;

pub struct Observe {
    settings: Settings,
    database: Database,
}

fn pod_containers(pod: &Pod) -> Result<Vec<Container>> {
    let mut containers = Vec::new();

    let annotations = &pod.metadata.annotations.as_ref();
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
        (&pod.namespace(), &pod.name(), &pod.status)
    {
        if let Some(container_statuses) = &status.container_statuses {
            for c in container_statuses {
                let container = Container {
                    namespace: namespace.to_string(),
                    pod: pod_name.to_string(),
                    container: c.name.to_string(),
                    image: c.image.to_string(),
                    image_id: c.image_id.to_string(),
                    latest_tag: latest_tag.to_string(),
                    latest_version_req: latest_version_req.to_string(),
                };
                containers.push(container);
            }
        }
    }

    Ok(containers)
}

impl Observe {
    pub fn new(settings: Settings, database: Database) -> Observe {
        Observe { settings, database }
    }

    pub async fn observe(&self) -> Result<()> {
        let client = Client::try_default().await?;
        let api = Api::<Pod>::all(client);
        let mut changes = pin!(watcher(api, Config::default()).default_backoff());
        self.database.truncate_containers().await?;
        while let Some(event) = changes.try_next().await? {
            match event {
                Event::Delete(pod) => {
                    for container in pod_containers(&pod)? {
                        self.database.delete_container(&container).await?;
                    }
                }
                Event::InitApply(pod) | Event::Apply(pod) => {
                    for container in pod_containers(&pod)? {
                        if !self
                            .settings
                            .ignore
                            .iter()
                            .any(|ignore| ignore.matches(&container))
                        {
                            self.database.replace_container(&container).await?;
                        }
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }
}
