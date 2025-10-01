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

mod list;
mod metrics;

use crate::{database::Database, serve::metrics::ServeMetrics, settings::Settings};
use anyhow::Result;
use axum::{
    Router,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};
use std::sync::Arc;
use tokio::net::TcpListener;

pub struct Serve {
    settings: Settings,
    database: Database,
}

struct ServeError(anyhow::Error);

impl IntoResponse for ServeError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for ServeError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl Serve {
    pub fn new(settings: Settings, database: Database) -> Serve {
        Serve { settings, database }
    }

    pub async fn serve(self) -> Result<()> {
        let listener = TcpListener::bind(&self.settings.bind_address).await?;
        let serve_metrics = ServeMetrics::new(self.database.clone(), self.settings.clone());
        let app = Router::new()
            .route("/", get(root))
            .route("/api/list", get(list::list))
            .nest("/metrics", serve_metrics.into())
            .with_state(Arc::new(self));
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn root() -> Html<&'static str> {
    Html(include_str!("../assets/index.html"))
}
