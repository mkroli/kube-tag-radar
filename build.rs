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

fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(migrate());
    println!("cargo:rustc-env=DATABASE_URL={BUILD_DATABASE_URL}");
    println!("cargo:rerun-if-changed=migrations");
}
