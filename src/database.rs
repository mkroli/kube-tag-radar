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
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

#[derive(Clone)]
pub struct Database {
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
}

#[derive(Clone, sqlx::FromRow, Serialize)]
pub struct Image {
    pub image: String,
    pub image_id: String,
    pub latest_tag: String,
    pub resolved_image_id: Option<String>,
    pub latest_image_id: Option<String>,
    pub version: Option<String>,
    pub latest_version_req: String,
    pub latest_version: Option<String>,
}

#[derive(Clone, sqlx::FromRow, Serialize)]
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
            CREATE TABLE IF NOT EXISTS container (
                namespace TEXT NOT NULL,
                pod TEXT NOT NULL,
                container TEXT NOT NULL,
                image TEXT NOT NULL,
                image_id TEXT NOT NULL,
                latest_tag TEXT NOT NULL,
                latest_version_req TEXT NOT NULL,
                PRIMARY KEY(namespace, pod, container)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        let _ = sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS image (
                image TEXT NOT NULL,
                image_id TEXT NOT NULL,
                latest_tag TEXT NOT NULL,
                latest_version_req TEXT NOT NULL,
                resolved_image_id TEXT,
                latest_image_id TEXT,
                version TEXT,
                latest_version TEXT,
                PRIMARY KEY(image, image_id, latest_tag, latest_version_req)
            )
        "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn truncate_containers(&self) -> Result<()> {
        sqlx::query("DELETE FROM container")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    fn delete_container_query<'q, DB>(
        container: &'q Container,
    ) -> Result<sqlx::query::Query<'q, DB, DB::Arguments<'q>>>
    where
        DB: sqlx::Database,
        std::string::String: sqlx::Type<DB>,
        std::string::String: sqlx::Encode<'q, DB>,
    {
        let q = sqlx::query(
            "DELETE FROM container WHERE namespace = $1 AND pod = $2 AND container = $3",
        )
        .bind(&container.namespace)
        .bind(&container.pod)
        .bind(&container.container);
        Ok(q)
    }

    pub async fn delete_unused_images(&self) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM image WHERE ROWID IN (
                SELECT image.ROWID
                FROM image
                LEFT JOIN container
                ON image.image = container.image
                AND image.image_id = container.image_id
                AND image.latest_tag = container.latest_tag
                AND image.latest_version_req = container.latest_version_req
                WHERE container.image IS NULL AND container.image_id IS NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn replace_container(&self, container: &Container) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        Database::delete_container_query(container)?
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO image (image, image_id, latest_tag, latest_version_req) VALUES ($1, $2, $3, $4)")
            .bind(&container.image)
            .bind(&container.image_id)
            .bind(&container.latest_tag)
            .bind(&container.latest_version_req)
            .execute(&mut *tx).await?;
        sqlx::query("INSERT INTO container (namespace, pod, container, image, image_id, latest_tag, latest_version_req) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(&container.namespace)
            .bind(&container.pod)
            .bind(&container.container)
            .bind(&container.image)
            .bind(&container.image_id)
            .bind(&container.latest_tag)
            .bind(&container.latest_version_req)
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_container(&self, container: &Container) -> Result<()> {
        Database::delete_container_query(container)?
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_image_details(&self, image: &Image) -> Result<()> {
        sqlx::query(
            r#"
                    UPDATE image
                    SET
                        version = $1,
                        latest_version = $2,
                        resolved_image_id = $3,
                        latest_image_id = $4
                    WHERE image = $5
                    AND image_id = $6
                    AND latest_tag = $7
                    AND latest_version_req = $8
                "#,
        )
        .bind(&image.version)
        .bind(&image.latest_version)
        .bind(&image.resolved_image_id)
        .bind(&image.latest_image_id)
        .bind(&image.image)
        .bind(&image.image_id)
        .bind(&image.latest_tag)
        .bind(&image.latest_version_req)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_image(&self) -> Result<Vec<Image>> {
        let images = sqlx::query_as(
            r#"
                SELECT
                    image,
                    image_id,
                    latest_tag,
                    resolved_image_id,
                    latest_image_id,
                    version,
                    latest_version_req,
                    latest_version
                FROM image
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(images)
    }

    pub async fn list_image_with_container(&self) -> Result<Vec<ImageWithContainer>> {
        let images = sqlx::query_as(
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
                    image.latest_version
                FROM container
                JOIN image
                    ON container.image = image.image
                    AND container.image_id = image.image_id
                    AND container.latest_tag = image.latest_tag
                    AND container.latest_version_req = image.latest_version_req
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(images)
    }
}
