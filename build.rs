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

use sqlx::sqlite::SqlitePoolOptions;
use tokio::fs::create_dir_all;

const BUILD_DATABASE_URL: &str = "sqlite:target/sqlx/build.sqlite?mode=rwc";

async fn migrate() {
    create_dir_all("target/sqlx").await.unwrap();
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(BUILD_DATABASE_URL)
        .await
        .unwrap();
    let () = sqlx::migrate!("./migrations").run(&pool).await.unwrap();
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    migrate().await;
    println!("cargo:rustc-env=DATABASE_URL={BUILD_DATABASE_URL}");
    println!("cargo:rerun-if-changed=migrations");
}
