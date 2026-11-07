use std::env;

use chromium::Chromium;
use dotenvy::dotenv;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;

async fn send_email(payload: Option<Value>) -> Option<Value> {
  println!("Sending email to: {:?}", payload);
  Some(Value::Number(1.into()))
}


#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url =
        env::var("DATABASE_URL").expect("MISSING DB URL");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .unwrap();

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS functions (
            id UUID PRIMARY KEY,
            created_at TEXT NOT NULL,
            function_name TEXT NOT NULL UNIQUE
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut chromium = Chromium::new(pool);

    let status = chromium
        .register(
            "send_email".to_string(),
            |payload| Box::pin(send_email(payload)),
        )
        .await;

    println!("Registration: {:?}", status);

    chromium
        .enqueue(
            "send_email".to_string(),
            Some(serde_json::json!({"email": "test@example.com"})),
        )
        .await;

    // Wait for spawned jobs to finish (useful for this basic demo).
    for handle in chromium.worker_pool {
        handle.await.unwrap();
    }
}