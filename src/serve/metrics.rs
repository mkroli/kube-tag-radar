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

use crate::database::Image;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::{TypedHeader, headers::ContentType};
use lazy_static::lazy_static;
use prometheus::{Encoder, Registry, TextEncoder};
use prometheus::{GaugeVec, register_gauge_vec};
use std::sync::Arc;

use super::{Serve, ServeError};

lazy_static! {
    static ref CONTAINER_GAUGE: GaugeVec = register_gauge_vec!(
        "kube_tag_radar_container",
        "Available update",
        &[
            "namespace",
            "pod",
            "container",
            "image",
            "image_id",
            "latest_tag",
            "resolved_image_id",
            "latest_image_id",
            "version",
            "latest_version_req",
            "latest_version"
        ]
    )
    .unwrap();
}

pub struct ServeRegistry<'a> {
    registry: &'a Registry,
}

impl IntoResponse for ServeRegistry<'_> {
    fn into_response(self) -> Response {
        let metric_families = self.registry.gather();
        let encoder = TextEncoder::new();
        let mut result = Vec::new();
        let result = match encoder.encode(&metric_families, &mut result) {
            Ok(()) => Ok((TypedHeader(ContentType::text_utf8()), result)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        };
        result.into_response()
    }
}

impl<'a> From<&'a Registry> for ServeRegistry<'a> {
    fn from(registry: &'a Registry) -> Self {
        ServeRegistry { registry }
    }
}

pub async fn metrics<'a>(
    State(serve): State<Arc<Serve>>,
) -> std::result::Result<ServeRegistry<'a>, ServeError> {
    let images = serve.database.list_images().await?;
    let registry = prometheus::default_registry();
    CONTAINER_GAUGE.reset();
    for image in images {
        let update_available = match image {
            Image {
                ref image_id,
                latest_image_id: Some(ref latest_image_id),
                ..
            } if image_id == latest_image_id => 0,
            Image {
                version: Some(ref version),
                latest_version: Some(ref latest_version),
                ..
            } if version == latest_version => 0,
            _ => 1,
        };
        CONTAINER_GAUGE
            .with_label_values(&[
                &image.namespace,
                &image.pod,
                &image.container,
                &image.image,
                &image.image_id,
                &image.latest_tag,
                &image.resolved_image_id.unwrap_or(String::new()),
                &image.latest_image_id.unwrap_or(String::new()),
                &image.version.unwrap_or(String::new()),
                &image.latest_version_req,
                &image.latest_version.unwrap_or(String::new()),
            ])
            .set(update_available.into());
    }
    Ok(registry.into())
}
