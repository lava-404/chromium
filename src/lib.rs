use std::{collections::HashMap, future::Future, pin::Pin};

use chrono::Utc;
use serde_json::Value;
use sqlx::{Pool, Postgres};
use tokio::task;
use uuid::Uuid;

pub type JobHandler =
    fn(Option<Value>) -> Pin<Box<dyn Future<Output = Option<Value>> + Send>>;

pub struct Chromium {
    pub db_pool: Pool<Postgres>,
    pub worker_pool: Vec<task::JoinHandle<()>>,
    pub functions: HashMap<String, JobHandler>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RegistrationStatus {
    Registered,
    RegistrationFailure,
}

impl Chromium {
    pub fn new(db_pool: Pool<Postgres>) -> Self {
        Self {
            db_pool,
            worker_pool: Vec::new(),
            functions: HashMap::new(),
        }
    }

    pub async fn register(
        &mut self,
        function_handler: String,
        function: JobHandler,
    ) -> RegistrationStatus {
        let function_id = Uuid::new_v4();
        let now = Utc::now();

        let result = sqlx::query(
            r#"
            INSERT INTO functions (created_at, function_name, id)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(now)
        .bind(&function_handler)
        .bind(function_id)
        .execute(&self.db_pool)
        .await;

        match result {
            Ok(_) => {
                self.functions.insert(function_handler, function);
                RegistrationStatus::Registered
            }
            Err(e) => {
                eprintln!("Registration failed: {e}");
                RegistrationStatus::RegistrationFailure
            }
        }
    }

    pub async fn enqueue(
        &mut self,
        function_handler: String,
        payload: Option<Value>,
    ) {
        let function = match self.functions.get(&function_handler) {
            Some(function) => *function,
            None => {
                eprintln!("Handler not registered: {function_handler}");
                return;
            }
        };

        let handle = task::spawn(async move {
            let result = function(payload).await;
            println!("Job completed: {:?}", result);
        });

        self.worker_pool.push(handle);
    }
}

