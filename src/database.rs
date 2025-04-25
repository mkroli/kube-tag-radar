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
use serde::Serialize;
use sqlx::{QueryBuilder, SqlitePool, sqlite::SqlitePoolOptions};

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow, Serialize, Debug)]
pub struct Image {
    pub namespace: String,
    pub pod: String,
    pub container: String,
    pub image: String,
    pub image_id: String,
    pub latest_tag: String,
    pub latest_image_id: Option<String>,
    pub version: Option<String>,
    pub latest_version_req: String,
    pub latest_version: Option<String>,
}

impl Database {
    pub async fn new(filename: &str) -> Result<Database> {
        let db_url = format!("sqlite:{filename}?mode=rwc");
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;
        let database = Database { pool };
        database.init().await?;
        Ok(database)
    }

    async fn init(&self) -> Result<()> {
        let _ = sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS image (
                namespace TEXT NOT NULL,
                pod TEXT NOT NULL,
                container TEXT NOT NULL,
                image TEXT NOT NULL,
                image_id TEXT NOT NULL,
                latest_tag TEXT NOT NULL,
                latest_image_id TEXT,
                version TEXT,
                latest_version_req TEXT NOT NULL,
                latest_version TEXT,
                PRIMARY KEY(namespace, pod, container)
            )
        "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<Image>> {
        let images = sqlx::query_as(
            r#"
            SELECT namespace, pod, container, image, image_id, latest_tag, latest_image_id, version, latest_version_req, latest_version FROM image ORDER BY 1, 2, 3
        "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(images)
    }

    pub async fn retain(&self, images: &Vec<Image>) -> Result<()> {
        let mut qb = QueryBuilder::<'_, sqlx::Sqlite>::new("DELETE FROM image");
        if !images.is_empty() {
            qb.push(" WHERE (namespace, pod, container) NOT IN (");
            let mut sep = qb.separated(",");
            for image in images {
                sep.push("(")
                    .push_bind_unseparated(&image.namespace)
                    .push_bind(&image.pod)
                    .push_bind(&image.container)
                    .push_unseparated(")");
            }
            qb.push(")");
        }
        qb.build().execute(&self.pool).await?;
        Ok(())
    }

    pub async fn replace(&self, image: &Image) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM image WHERE namespace = $1 AND pod = $2 AND container = $3")
            .bind(&image.namespace)
            .bind(&image.pod)
            .bind(&image.container)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO image (namespace, pod, container, image, image_id, latest_tag, latest_image_id, version, latest_version_req, latest_version) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",)
            .bind(&image.namespace)
            .bind(&image.pod)
            .bind(&image.container)
            .bind(&image.image)
            .bind(&image.image_id)
            .bind(&image.latest_tag)
            .bind(&image.latest_image_id)
            .bind(&image.version)
            .bind(&image.latest_version_req)
            .bind(&image.latest_version)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
