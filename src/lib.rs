use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{Pool, Postgres, postgres::PgRow};
use tokio::task;
use redis::Client;
use redis::aio::ConnectionManager;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use redis::AsyncCommands;

use crate::JobStatus::Scheduled;

pub type JobHandler =
    fn(Option<Value>) -> Pin<Box<dyn Future<Output = Option<Value>> + Send>>;

pub struct Chromium {
    pub db_pool: Pool<Postgres>,
    pub redis: ConnectionManager,
    pub functions: HashMap<String, JobHandler>,
    pub worker_count: i32
}
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Job {
    pub id: Uuid,
    pub job: String,
    pub status: JobStatus,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub payload: Option<Value>
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "status", rename_all = "PascalCase")]
#[serde(rename_all = "PascalCase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Retrying,
    Cancelled,
    Scheduled
}

#[derive(Debug, PartialEq, Eq)]
pub enum RegistrationStatus {
    Registered,
    RegistrationFailure,
}



impl Chromium {
    pub async fn new(&self, db_pool: Pool<Postgres>) -> Self { 
        let client = Client::open("redis://127.0.0.1/").unwrap();

        let redis = ConnectionManager::new(client)
            .await
            .expect("Failed to connect to Redis");
        Self {
            db_pool,
            redis,
            functions: HashMap::new(),
            worker_count: 0
        }
    }

    pub async fn start(self: Arc<Self>) {
        for _ in 0..self.worker_count {
            let chromium = Arc::clone(&self);
    
            let handle = tokio::spawn(async move {
                chromium.worker_loop().await;
            });
    
            // Store handles if you need shutdown/monitoring.
        }
    }

    pub async fn register(
        &mut self,
        function_handler: String,
        function: JobHandler,
    ) -> RegistrationStatus {
        self.functions.insert(function_handler, function);
        RegistrationStatus::Registered        
    }

    pub async fn enqueue(
        &mut self,
        function_handler: String,
        payload: Option<Value>,
        scheduled_at: Option<DateTime<Utc>>
    ) {
        let function_id = Uuid::new_v4();
        let now = Utc::now();

        let result = sqlx::query(
            r#"
            INSERT INTO jobs (created_at, id, job)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(now)
        .bind(function_id)
        .bind(&function_handler)
        .execute(&self.db_pool)
        .await;

        
    }

    async fn claim_job(&self) -> Option<Job> {
        //batched jobs

        let mut redis = self.redis.clone();
        let queue_size: usize = redis
        .llen("chromium:jobs")
        .await
        .expect("Failed to get queue length");
        
        if queue_size < 20 {
            let jobs: Vec<Job> = sqlx::query_as(
                r#"
                SELECT * FROM jobs
                WHERE status = 'Queued'::status
                ORDER BY created_at ASC
                LIMIT 20
                "#
            )
            .fetch_all(&self.db_pool)
            .await
            .expect("Failed to fetch jobs");
        
            for job in jobs {
                let serialized_job = serde_json::to_string(&job)
                    .expect("Failed to serialize job");
        
                let _ : usize = redis.rpush("chromium:jobs", serialized_job)
                    .await
                    .expect("Failed to push job into Redis");
            }
        }

        let job_data: Option<String> = redis.lpop("chromium:jobs", None).await.expect("failed to pop job from queue");
        let job: Option<Job> = if let Some(data) = job_data {
            Some(serde_json::from_str(&data).expect("failed to deserialize job"))
        } else {
            None
        };
        
        job
    }

    async fn execute(&self, job: Job) {
        let function = match self.functions.get(&job.job) {
            Some(function) => *function,
            None => {
                eprintln!("Handler not registered: {}", job.job);
                return;
            }
        };

        let handle = task::spawn(async move {
            let payload: Option<Value> = job.payload;
            let result = function(payload).await;
        }).await;
    }

    async fn worker_loop(&self) {
        loop {
            if let Some(job) = self.claim_job().await {
                self.execute(job).await;
            }
        }
    }


}

