use std::collections::VecDeque;
use axum::{routing::get, Router};
enum Status {
    Pending,
    Running,
    Completed
}
struct Job {
        id: i32,
        function_handler: String,
        status: Status
}
impl Job<T> {
    fn new(function_handler: String) -> Self {

        Self {
            id: 0,
            function_handler,
            status: Status::Pending
        }
        
    }
}

#[tokio::main]
async fn main() {
    let queue  = VecDeque::new();
    let app = Router::new()
    .route("/enqueue", get(enqueue))
    .route("/register",)

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    println!("Hello, world!");
}

async fn enqueue(function_handler: String, payload: Option<String>) {

}