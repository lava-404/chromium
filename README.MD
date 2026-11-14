# Chromium – Development Progress

## 1. Current Architecture

Chromium is being developed as a Rust-based durable job queue and scheduler.

| Component | Responsibility |
|---|---|
| PostgreSQL | Persistent job storage and lifecycle management |
| Redis | Job buffering and delivery to workers |
| Tokio | Async runtime and concurrent worker execution |
| HashMap | In-memory mapping of function identifiers to Rust handlers |
| Serde | Job serialization and deserialization |

## 2. Implemented So Far

- Defined the `Chromium` core struct with database pool, Redis connection, handler registry and configurable worker count.
- Defined the `Job` model with UUID, function identifier, status, timestamps and optional JSON payload.
- Defined `JobStatus` for tracking job lifecycle states.
- Defined `JobHandler` as an async Rust function signature.
- Implemented Redis connection initialization using `ConnectionManager`.
- Implemented `register()` to register user-defined async handlers.
- Started implementing `enqueue()` to persist jobs in PostgreSQL.
- Implemented `claim_job()` to check Redis queue depth, fetch batches of jobs from PostgreSQL, serialize them and push them into Redis.
- Implemented job deserialization when consuming jobs from Redis.
- Implemented `execute()` to resolve a job's function identifier against the registered handlers.
- Implemented `worker_loop()` to continuously claim and process jobs.
- Introduced `Arc`-based shared ownership to allow multiple Tokio worker tasks to access Chromium concurrently.

## 3. Immediate Next Steps

### Priority 1: Complete Job Enqueueing
- Persist the job payload and optional scheduled timestamp alongside the job ID and function identifier.
- Set the initial status appropriately (`Queued` or `Scheduled`).
- Validate that the requested handler has been registered before enqueueing.

### Priority 2: Implement Atomic Job Claiming
- Prevent multiple Chromium instances from fetching the same jobs.
- Use PostgreSQL transactions with `FOR UPDATE SKIP LOCKED`.
- Ensure jobs are claimed atomically before dispatching them to Redis.
- Introduce a dispatch state or lease mechanism to distinguish jobs buffered in Redis from jobs actively executing.

### Priority 3: Fix Worker Execution
- Make `execute()` await the registered handler directly instead of spawning another Tokio task.
- Ensure each worker processes one job at a time.
- Allow workers to continue claiming jobs after execution finishes.
- Handle unknown function identifiers and execution errors.

### Priority 4: Implement Job Acknowledgement and Recovery
- Prevent jobs from disappearing if a worker crashes after receiving them from Redis.
- Introduce acknowledgement and visibility/recovery mechanisms.
- Consider Redis Streams and consumer groups instead of Redis Lists.
- Ensure PostgreSQL remains the source of truth.

### Priority 5: Implement Job Lifecycle Updates
- Transition jobs through appropriate states:
  - `Queued`
  - `Running`
  - `Completed`
  - `Failed`
  - `Retrying`
  - `Scheduled`
  - `Cancelled`
- Persist execution results and failure information.
- Ensure state transitions are consistent with actual execution.

### Priority 6: Implement Scheduling and Retries
- Execute scheduled jobs only when their scheduled time arrives.
- Add configurable retry policies.
- Implement exponential backoff for failed jobs.
- Recover jobs whose workers crash while executing them.

### Priority 7: Worker Management
- Store worker `JoinHandle`s for lifecycle management.
- Implement graceful shutdown.
- Handle worker panics and unexpected termination.
- Make worker concurrency configurable through Chromium's configuration.

## 4. Current Execution Flow

```text
Application
    |
    v
Chromium SDK
    |
    +---- register handlers
    |
    +---- enqueue jobs
              |
              v
         PostgreSQL
              |
              v
            Redis
              |
              v
         Worker Pool
              |
              v
       Handler Resolution
              |
              v
       Rust Function Execution
              |
              v
       Update Job Status
```

## 5. Longer-Term Direction

Once durable job execution, retries, scheduling and crash recovery are reliable, Chromium can be extended into a durable workflow engine.

This would involve workflow state persistence, execution history, timers, multi-step coordination and recovery from interrupted workflows.