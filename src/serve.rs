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

use crate::database::Database;
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
    pub fn new(database: Database) -> Serve {
        Serve { database }
    }

    pub async fn serve(self) -> Result<()> {
        let app = Router::new()
            .route("/", get(root))
            .route("/api/list", get(list::list))
            .route("/metrics", get(metrics::metrics))
            .with_state(Arc::new(self));
        let listener = TcpListener::bind("0.0.0.0:8080").await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn root() -> Html<&'static str> {
    Html(include_str!("../assets/index.html"))
}
