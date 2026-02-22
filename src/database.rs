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
use prometheus_client::encoding::EncodeLabelSet;
use serde::Serialize;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use time::OffsetDateTime;

use crate::settings::Settings;

#[derive(Clone)]
pub struct Database {
    settings: Settings,
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
pub struct Container {
    pub namespace: String,
    pub pod: String,
    pub container: String,
    pub image: String,
    pub image_id: String,
    pub latest_tag: String,
    pub latest_version_req: String,
    pub latest_version_regex: String,
}

#[derive(Clone, sqlx::FromRow)]
pub struct Image {
    pub image: String,
    pub image_id: String,
    pub latest_tag: String,
    pub resolved_image_id: Option<String>,
    pub latest_image_id: Option<String>,
    pub version: Option<String>,
    pub latest_version_req: String,
    pub latest_version_regex: String,
    pub latest_version: Option<String>,
    pub last_checked: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ImageWithContainer {
    pub namespace: String,
    pub pod: String,
    pub container: String,
    pub image: String,
    pub image_id: String,
    pub latest_tag: String,
    pub resolved_image_id: Option<String>,
    pub latest_image_id: Option<String>,
    pub version: Option<String>,
    pub latest_version_req: String,
    pub latest_version_regex: String,
    pub latest_version: Option<String>,
}

pub trait PodInfo {
    fn namespace(&self) -> Option<String>;
    fn name(&self) -> Option<String>;
    fn containers(&self, settings: &Settings) -> Vec<Container>;
}

impl Database {
    pub async fn new(settings: Settings) -> Result<Database> {
        let db_url = format!("sqlite:{}?mode=rwc", &settings.database);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;
        let database = Database { settings, pool };
        database.init().await?;
        Ok(database)
    }

    async fn init(&self) -> Result<()> {
        let () = sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub async fn truncate_containers(&self) -> Result<()> {
        sqlx::query!("DELETE FROM container")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_unused_images(&self) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM image WHERE ROWID IN (
                SELECT image.ROWID
                FROM image
                LEFT JOIN container
                ON image.image = container.image
                AND image.image_id = container.image_id
                AND image.latest_tag = container.latest_tag
                AND image.latest_version_req = container.latest_version_req
                AND image.latest_version_regex = container.latest_version_regex
                WHERE container.image IS NULL AND container.image_id IS NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_pod<P: PodInfo>(&self, pod: &P) -> Result<()> {
        if let (Some(namespace), Some(name)) = (pod.namespace(), pod.name()) {
            sqlx::query!(
                "DELETE FROM container WHERE namespace = $1 AND pod = $2",
                namespace,
                name,
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn replace_pod<P: PodInfo>(&self, pod: &P) -> Result<()> {
        if let (Some(namespace), Some(name)) = (pod.namespace(), pod.name()) {
            let mut tx = self.pool.begin().await?;
            sqlx::query!(
                "DELETE FROM container WHERE namespace = $1 AND pod = $2",
                namespace,
                name,
            )
            .execute(&mut *tx)
            .await?;

            for container in pod.containers(&self.settings) {
                sqlx::query!(
                    "INSERT INTO container (namespace, pod, container, image, image_id, latest_tag, latest_version_req, latest_version_regex) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                    container.namespace,
                    container.pod,
                    container.container,
                    container.image,
                    container.image_id,
                    container.latest_tag,
                    container.latest_version_req,
                    container.latest_version_regex,
                )
                .execute(&mut *tx)
                .await?;
                sqlx::query!(
                    "INSERT OR IGNORE INTO image (image, image_id, latest_tag, latest_version_req, latest_version_regex) VALUES ($1, $2, $3, $4, $5)",
                    container.image,
                    container.image_id,
                    container.latest_tag,
                    container.latest_version_req,
                    container.latest_version_regex,
                )
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
        }
        Ok(())
    }

    pub async fn update_image_details(&self, image: &Image) -> Result<()> {
        let now = OffsetDateTime::now_utc();
        sqlx::query!(
            r#"
                    UPDATE image
                    SET
                        version = $1,
                        latest_version = $2,
                        resolved_image_id = $3,
                        latest_image_id = $4,
                        last_checked = $5
                    WHERE image = $6
                    AND image_id = $7
                    AND latest_tag = $8
                    AND latest_version_req = $9
                    AND latest_version_regex = $10
            "#,
            image.version,
            image.latest_version,
            image.resolved_image_id,
            image.latest_image_id,
            now,
            image.image,
            image.image_id,
            image.latest_tag,
            image.latest_version_req,
            image.latest_version_regex,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_image(&self) -> Result<Vec<Image>> {
        let images = sqlx::query_as!(
            Image,
            r#"
                SELECT
                    image,
                    image_id,
                    latest_tag,
                    resolved_image_id,
                    latest_image_id,
                    version,
                    latest_version_req,
                    latest_version_regex,
                    latest_version,
                    last_checked
                FROM image
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(images)
    }

    pub async fn list_image_with_container(&self) -> Result<Vec<ImageWithContainer>> {
        let images = sqlx::query_as!(
            ImageWithContainer,
            r#"
                SELECT
                    container.namespace,
                    container.pod,
                    container.container,
                    image.image,
                    image.image_id,
                    image.latest_tag,
                    image.resolved_image_id,
                    image.latest_image_id,
                    image.version,
                    image.latest_version_req,
                    image.latest_version_regex,
                    image.latest_version
                FROM container
                JOIN image
                    ON container.image = image.image
                    AND container.image_id = image.image_id
                    AND container.latest_tag = image.latest_tag
                    AND container.latest_version_req = image.latest_version_req
                    AND container.latest_version_regex = image.latest_version_regex
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(images)
    }
}
