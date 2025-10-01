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

use std::sync::Arc;

use crate::{
    database::{Database, ImageWithContainer},
    settings::Settings,
};
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use axum_extra::{TypedHeader, headers::ContentType};
use prometheus_client::{
    encoding::text::encode,
    metrics::{family::Family, gauge::Gauge},
    registry::Registry,
};

use super::ServeError;

fn ignored(settings: &Settings, image: &ImageWithContainer) -> bool {
    settings.ignore.iter().any(|i| i.matches(image))
}

fn update_available(image: &ImageWithContainer) -> bool {
    match *image {
        ImageWithContainer {
            resolved_image_id: Some(ref resolved_image_id),
            latest_image_id: Some(ref latest_image_id),
            ..
        } if resolved_image_id == latest_image_id => false,
        ImageWithContainer {
            ref image_id,
            latest_image_id: Some(ref latest_image_id),
            ..
        } if image_id == latest_image_id => false,
        ImageWithContainer {
            version: Some(ref version),
            latest_version: Some(ref latest_version),
            ..
        } if version == latest_version => false,
        _ => true,
    }
}

pub struct ServeMetrics {
    database: Database,
    settings: Settings,
    registry: Registry,
    containers: Family<ImageWithContainer, Gauge>,
}

impl ServeMetrics {
    pub fn new(database: Database, settings: Settings) -> Self {
        let mut registry = Registry::default();
        let containers = Family::<ImageWithContainer, Gauge>::default();
        registry.register(
            "kube_tag_radar_container",
            "Available update",
            containers.clone(),
        );
        ServeMetrics {
            database,
            settings,
            registry,
            containers,
        }
    }

    async fn metrics(&self) -> std::result::Result<Response, ServeError> {
        let images = self.database.list_image_with_container().await?;
        self.containers.clear();
        for image in images {
            let value = if ignored(&self.settings, &image) {
                -1
            } else if update_available(&image) {
                1
            } else {
                0
            };
            self.containers.get_or_create(&image).set(value);
        }
        Ok(self.into_response())
    }
}

impl IntoResponse for &ServeMetrics {
    fn into_response(self) -> Response {
        let mut buffer = String::new();
        let result = match encode(&mut buffer, &self.registry) {
            Ok(()) => Ok((TypedHeader(ContentType::text_utf8()), buffer)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        };
        result.into_response()
    }
}

impl<S> From<ServeMetrics> for Router<S> {
    fn from(serve_metrics: ServeMetrics) -> Self {
        Router::new()
            .route(
                "/",
                get(async |State(serve_metrics): State<Arc<ServeMetrics>>| {
                    serve_metrics.metrics().await
                }),
            )
            .with_state(Arc::new(serve_metrics))
    }
}
