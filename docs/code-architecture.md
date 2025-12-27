# Code Architecture

This document provides a deep dive into Throttler's code structure, including state flow diagrams, sequence diagrams, and detailed explanations of how components interact.

## Table of Contents

- [Module Dependency Graph](#module-dependency-graph)
- [Request Lifecycle](#request-lifecycle)
- [State Management](#state-management)
- [Sequence Diagrams](#sequence-diagrams)
- [Component Details](#component-details)
- [Error Propagation](#error-propagation)
- [Concurrency Model](#concurrency-model)

---

## Module Dependency Graph

```
                                 ┌─────────────┐
                                 │   main.rs   │
                                 │  (binary)   │
                                 └──────┬──────┘
                                        │
                                        ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                              lib.rs (public API)                           │
│                                                                            │
│   pub mod config;           pub mod throttler;      pub mod error;         │
│   pub mod server;           pub mod rate_limiter;   pub mod redis;         │
│   pub mod handlers;         pub mod token_bucket;   pub mod validation;    │
│   pub mod algorithms;       pub mod health;         pub mod metrics;       │
│   pub mod middleware;       pub mod key_generator;  pub mod response;      │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
                                        │
            ┌───────────────────────────┼───────────────────────────┐
            │                           │                           │
            ▼                           ▼                           ▼
    ┌───────────────┐          ┌───────────────┐          ┌───────────────┐
    │   server.rs   │          │  handlers.rs  │          │   config.rs   │
    │               │◄─────────│               │          │               │
    │  Axum Router  │          │ HTTP Handlers │          │  Config Load  │
    └───────┬───────┘          └───────┬───────┘          └───────────────┘
            │                          │
            │         ┌────────────────┼────────────────┐
            │         │                │                │
            │         ▼                ▼                ▼
            │  ┌────────────┐   ┌────────────┐   ┌────────────┐
            │  │ validation │   │  response  │   │ middleware │
            │  │    .rs     │   │    .rs     │   │    .rs     │
            │  └────────────┘   └────────────┘   └────────────┘
            │
            ▼
    ┌───────────────┐
    │ throttler.rs  │◄──────────────────────────────────────┐
    │               │                                       │
    │ Orchestrator  │                                       │
    └───────┬───────┘                                       │
            │                                               │
            ▼                                               │
    ┌───────────────┐         ┌───────────────┐             │
    │rate_limiter.rs│────────►│   redis.rs    │             │
    │               │         │               │             │
    │  Core Engine  │         │ Redis Client  │             │
    └───────┬───────┘         └───────────────┘             │
            │                                               │
            ▼                                               │
    ┌───────────────┐                                       │
    │token_bucket.rs│                                       │
    │               │                                       │
    │  Algorithm    │                                       │
    └───────────────┘                                       │
            │                                               │
            ▼                                               │
    ┌───────────────────────────────────────────────────────┤
    │                    algorithms/                        │
    │  ┌─────────────┐              ┌───────────────────┐   │
    │  │   mod.rs    │              │ sliding_window.rs │   │
    │  │  (Trait)    │              │  (Alternative)    │   │
    │  └─────────────┘              └───────────────────┘   │
    └───────────────────────────────────────────────────────┘
```

---

## Request Lifecycle

### Overview State Machine

```
                    ┌─────────────────────────────────────────────┐
                    │              REQUEST LIFECYCLE              │
                    └─────────────────────────────────────────────┘

    ┌─────────┐     ┌─────────┐      ┌─────────┐     ┌─────────┐     ┌─────────┐
    │RECEIVED │────►│ ROUTED  │─────►│VALIDATED│────►│PROCESSED│────►│RESPONDED│
    └─────────┘     └─────────┘      └─────────┘     └─────────┘     └─────────┘
         │               │                │               │               │
         │               │                │               │               │
         ▼               ▼                ▼               ▼               ▼
    ┌───────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐
    │Middleware │     │  Axum   │     │Validator│     │Throttler│     │Response │
    │ Logging   │     │ Router  │     │  Check  │     │ + Redis │     │ Format  │
    └───────────┘     └─────────┘     └─────────┘     └─────────┘     └─────────┘
                                           │               │
                                           │               │
                                      ┌────┴────┐     ┌────┴────┐
                                      │  VALID  │     │ ALLOWED │
                                      └────┬────┘     └────┬────┘
                                           │               │
                                      ┌────┴────┐     ┌────┴────┐
                                      │ INVALID │     │ DENIED  │
                                      │  (400)  │     │  (429)  │
                                      └─────────┘     └─────────┘
```

### Detailed Request States

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           REQUEST STATE TRANSITIONS                          │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐                                                            │
│  │   INCOMING   │  HTTP request received by Axum server                      │
│  └──────┬───────┘                                                            │
│         │                                                                    │
│         ▼                                                                    │
│  ┌──────────────┐                                                            │
│  │   LOGGING    │  middleware.rs: Log request method, URI, client IP         │
│  └──────┬───────┘                                                            │
│         │                                                                    │
│         ▼                                                                    │
│  ┌──────────────┐     ┌──────────────┐                                       │
│  │   ROUTING    │────►│  NOT FOUND   │  No matching route → 404              │
│  └──────┬───────┘     └──────────────┘                                       │
│         │                                                                    │
│         ▼                                                                    │
│  ┌──────────────┐     ┌──────────────┐                                       │
│  │  EXTRACTING  │────►│  BAD REQUEST │  JSON parse error → 400               │
│  └──────┬───────┘     └──────────────┘                                       │
│         │                                                                    │
│         ▼                                                                    │
│  ┌──────────────┐     ┌──────────────┐                                       │
│  │  VALIDATING  │────►│   INVALID    │  Key/params invalid → 400             │
│  └──────┬───────┘     └──────────────┘                                       │
│         │                                                                    │
│         ▼                                                                    │
│  ┌──────────────┐                                                            │
│  │  RATE CHECK  │  RateLimiter.check_rate_limit(key)                         │
│  └──────┬───────┘                                                            │
│         │                                                                    │
│         ├─────────────────────────────────┐                                  │
│         │                                 │                                  │
│         ▼                                 ▼                                  │
│  ┌──────────────┐                  ┌──────────────┐                          │
│  │   ALLOWED    │                  │   THROTTLED  │                          │
│  │   (200 OK)   │                  │    (429)     │                          │
│  └──────┬───────┘                  └──────┬───────┘                          │
│         │                                 │                                  │
│         ▼                                 ▼                                  │
│  ┌──────────────────────────────────────────────────────────────────────┐    │
│  │                        RESPONSE HEADERS                              │    │
│  │                                                                      │    │
│  │  X-RateLimit-Limit: 100           ← Maximum allowed requests         │    │
│  │  X-RateLimit-Remaining: 42        ← Tokens left in bucket            │    │
│  │  X-RateLimit-Window: 60000        ← Window size in milliseconds      │    │
│  │  Retry-After: 5                   ← Seconds until tokens refill      │    │
│  │                                     (only on 429)                    │    │
│  └──────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## State Management

### Token Bucket State Machine

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         TOKEN BUCKET STATE MACHINE                           │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│                          ┌─────────────────────┐                             │
│                          │     INITIALIZED     │                             │
│                          │                     │                             │
│                          │  tokens = capacity  │                             │
│                          │  last_refill = now  │                             │
│                          └──────────┬──────────┘                             │
│                                     │                                        │
│                                     ▼                                        │
│                          ┌─────────────────────┐                             │
│          ┌──────────────►│       READY         │◄──────────────┐             │
│          │               │                     │               │             │
│          │               │   tokens > 0        │               │             │
│          │               └──────────┬──────────┘               │             │
│          │                          │                          │             │
│          │            ┌─────────────┴─────────────┐            │             │
│          │            │                           │            │             │
│          │            ▼                           ▼            │             │
│          │   ┌─────────────────┐        ┌─────────────────┐    │             │
│          │   │  CONSUME TOKEN  │        │  TIME ELAPSED   │    │             │
│          │   │                 │        │                 │    │             │
│          │   │  tokens -= 1    │        │  Trigger refill │    │             │
│          │   └────────┬────────┘        └────────┬────────┘    │             │
│          │            │                          │             │             │
│          │            │                          ▼             │             │
│          │            │                 ┌─────────────────┐    │             │
│          │            │                 │     REFILL      │    │             │
│          │            │                 │                 │────┘             │
│          │            │                 │ tokens += rate  │                  │
│          │            │                 │   × elapsed     │                  │
│          │            │                 │                 │                  │
│          │            │                 │ tokens = min(   │                  │
│          │            │                 │   tokens,       │                  │
│          │            │                 │   capacity)     │                  │
│          │            │                 └─────────────────┘                  │
│          │            │                                                      │
│          │            ▼                                                      │
│          │   ┌─────────────────┐                                             │
│          │   │  tokens >= 0?   │                                             │
│          │   └────────┬────────┘                                             │
│          │            │                                                      │
│          │     ┌──────┴──────┐                                               │
│          │     │             │                                               │
│          │    YES           NO                                               │
│          │     │             │                                               │
│          │     │             ▼                                               │
│          │     │    ┌─────────────────┐                                      │
│          └─────┘    │    EXHAUSTED    │                                      │
│                     │                 │                                      │
│                     │  tokens = 0     │                                      │
│                     │  Wait for       │                                      │
│                     │  refill...      │──────────────────────┐               │
│                     └─────────────────┘                      │               │
│                                                              │               │
│                              ┌────────────────────────────────               │
│                              │                                               │
│                              ▼                                               │
│                     ┌─────────────────┐                                      │
│                     │  TIME PASSES    │                                      │
│                     │                 │                                      │
│                     │  Automatic      │                                      │
│                     │  refill on      │─────────────────────►(READY)         │
│                     │  next request   │                                      │
│                     └─────────────────┘                                      │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Bucket Data Structure

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                          TokenBucket Structure                               │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  TokenBucket                                                        │     │
│  ├─────────────────────────────────────────────────────────────────────┤     │
│  │                                                                     │     │
│  │  capacity: u64          ─────►  Maximum tokens (e.g., 100)          │     │
│  │                                                                     │     │
│  │  tokens: f64            ─────►  Current available tokens            │     │
│  │                                 (fractional for precise refill)     │     │
│  │                                                                     │     │
│  │  refill_rate: f64       ─────►  Tokens added per second             │     │
│  │                                 (e.g., 10.0 = 10 tokens/sec)        │     │
│  │                                                                     │     │
│  │  last_refill: Instant   ─────►  Timestamp of last refill calc       │     │
│  │                                 (used for elapsed time)             │     │
│  │                                                                     │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                              │
│  Example State Over Time:                                                    │
│                                                                              │
│  Time 0.0s:  tokens=100.0  capacity=100  rate=10/s                           │
│              ▼ Request consumes 1 token                                      │
│  Time 0.0s:  tokens=99.0                                                     │
│              ▼ 0.5 seconds pass, request arrives                             │
│  Time 0.5s:  tokens=99.0 + (0.5 × 10) = 104.0 → capped to 100.0              │
│              ▼ Request consumes 1 token                                      │
│  Time 0.5s:  tokens=99.0                                                     │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Storage Modes

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           STORAGE MODE COMPARISON                            │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────┐    ┌─────────────────────────────┐          │
│  │     LOCAL MODE (Memory)     │    │   DISTRIBUTED MODE (Redis)  │          │
│  ├─────────────────────────────┤    ├─────────────────────────────┤          │
│  │                             │    │                             │          │
│  │  ┌───────────────────────┐  │    │  ┌───────────────────────┐  │          │
│  │  │ HashMap<String,       │  │    │  │    Redis Server       │  │          │
│  │  │   LocalBucket>        │  │    │  │                       │  │          │
│  │  └───────────────────────┘  │    │  │  KEY: throttle:user1  │  │          │
│  │                             │    │  │  VAL: {json bucket}   │  │          │
│  │  Protected by:              │    │  │                       │  │          │
│  │  Arc<RwLock<...>>           │    │  │  KEY: throttle:user2  │  │          │
│  │                             │    │  │  VAL: {json bucket}   │  │          │
│  │  Pros:                      │    │  └───────────────────────┘  │          │
│  │  • Zero network latency     │    │                             │          │
│  │  • No external dependency   │    │  Connection via:            │          │
│  │  • Simple deployment        │    │  Arc<RedisClient>           │          │
│  │                             │    │                             │          │
│  │  Cons:                      │    │  Pros:                      │          │
│  │  • Single instance only     │    │  • Multi-instance support   │          │
│  │  • State lost on restart    │    │  • Persistent state         │          │
│  │  • No horizontal scaling    │    │  • Horizontal scaling       │          │
│  │                             │    │                             │          │
│  │  Use Case:                  │    │  Cons:                      │          │
│  │  Development, testing,      │    │  • Network latency          │          │
│  │  single-server production   │    │  • External dependency      │          │
│  │                             │    │  • More complex setup       │          │
│  │                             │    │                             │          │
│  │                             │    │  Use Case:                  │          │
│  │                             │    │  Production, multi-server,  │          │
│  │                             │    │  Kubernetes deployments     │          │
│  └─────────────────────────────┘    └─────────────────────────────┘          │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Sequence Diagrams

### Rate Limit Check Sequence

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                    RATE LIMIT CHECK - SUCCESS FLOW                           │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Client          Server         Handler        RateLimiter      Redis        │
│    │               │               │               │               │         │
│    │  POST /rate-limit/key/check   │               │               │         │
│    │──────────────►│               │               │               │         │
│    │               │               │               │               │         │
│    │               │  route()      │               │               │         │
│    │               │──────────────►│               │               │         │
│    │               │               │               │               │         │
│    │               │               │ validate(key) │               │         │
│    │               │               │──────┐        │               │         │
│    │               │               │      │        │               │         │
│    │               │               │◄─────┘ OK     │               │         │
│    │               │               │               │               │         │
│    │               │               │check_rate_limit(key)          │         │
│    │               │               │──────────────►│               │         │
│    │               │               │               │               │         │
│    │               │               │               │ GET bucket    │         │
│    │               │               │               │──────────────►│         │
│    │               │               │               │               │         │
│    │               │               │               │◄──────────────│         │
│    │               │               │               │  bucket data  │         │
│    │               │               │               │               │         │
│    │               │               │               │ refill()      │         │
│    │               │               │               │──────┐        │         │
│    │               │               │               │      │        │         │
│    │               │               │               │◄─────┘        │         │
│    │               │               │               │               │         │
│    │               │               │               │ try_consume(1)│         │
│    │               │               │               │──────┐        │         │
│    │               │               │               │      │        │         │
│    │               │               │               │◄─────┘ OK     │         │
│    │               │               │               │               │         │
│    │               │               │               │ SET bucket    │         │
│    │               │               │               │──────────────►│         │
│    │               │               │               │               │         │
│    │               │               │◄──────────────│               │         │
│    │               │               │ CheckResult{allowed: true,    │         │
│    │               │               │              remaining: 99}   │         │
│    │               │               │               │               │         │
│    │               │◄──────────────│               │               │         │
│    │               │  200 OK       │               │               │         │
│    │               │  + headers    │               │               │         │
│    │               │               │               │               │         │
│    │◄──────────────│               │               │               │         │
│    │  {"allowed": true,            │               │               │         │
│    │   "remaining": 99}            │               │               │         │
│    │               │               │               │               │         │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Rate Limit Exceeded Sequence

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                    RATE LIMIT CHECK - EXCEEDED FLOW                          │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Client          Server         Handler        RateLimiter      Redis        │
│    │               │               │               │               │         │
│    │  POST /rate-limit/key/check   │               │               │         │
│    │──────────────►│               │               │               │         │
│    │               │               │               │               │         │
│    │               │  route()      │               │               │         │
│    │               │──────────────►│               │               │         │
│    │               │               │               │               │         │
│    │               │               │check_rate_limit(key)          │         │
│    │               │               │──────────────►│               │         │
│    │               │               │               │               │         │
│    │               │               │               │ GET bucket    │         │
│    │               │               │               │──────────────►│         │
│    │               │               │               │               │         │
│    │               │               │               │◄──────────────│         │
│    │               │               │               │ {tokens: 0}   │         │
│    │               │               │               │               │         │
│    │               │               │               │ try_consume(1)│         │
│    │               │               │               │──────┐        │         │
│    │               │               │               │      │        │         │
│    │               │               │               │◄─────┘ FAIL   │         │
│    │               │               │               │               │         │
│    │               │               │◄──────────────│               │         │
│    │               │               │ Err(RateLimitExceeded{        │         │
│    │               │               │     retry_after: 5})          │         │
│    │               │               │               │               │         │
│    │               │◄──────────────│               │               │         │
│    │               │  429 Too Many Requests        │               │         │
│    │               │  Retry-After: 5               │               │         │
│    │               │               │               │               │         │
│    │◄──────────────│               │               │               │         │
│    │  {"allowed": false,           │               │               │         │
│    │   "remaining": 0,             │               │               │         │
│    │   "retry_after": 5}           │               │               │         │
│    │               │               │               │               │         │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Configuration Update Sequence

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                      SET RATE LIMIT CONFIGURATION                            │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Client          Server         Handler        Throttler       RateLimiter   │
│    │               │               │               │               │         │
│    │  POST /rate-limit/api-key     │               │               │         │
│    │  {requests: 100, window: 60s} │               │               │         │
│    │──────────────►│               │               │               │         │
│    │               │               │               │               │         │
│    │               │  route()      │               │               │         │
│    │               │──────────────►│               │               │         │
│    │               │               │               │               │         │
│    │               │               │ validate(key, params)         │         │
│    │               │               │──────┐        │               │         │
│    │               │               │      │        │               │         │
│    │               │               │◄─────┘ OK     │               │         │
│    │               │               │               │               │         │
│    │               │               │ set_rule(key, rule)           │         │
│    │               │               │──────────────►│               │         │
│    │               │               │               │               │         │
│    │               │               │               │ store rule    │         │
│    │               │               │               │──────┐        │         │
│    │               │               │               │      │        │         │
│    │               │               │               │◄─────┘        │         │
│    │               │               │               │               │         │
│    │               │               │               │create_bucket(key, rule) │
│    │               │               │               │──────────────►│         │
│    │               │               │               │               │         │
│    │               │               │               │◄──────────────│         │
│    │               │               │               │               │         │
│    │               │               │◄──────────────│               │         │
│    │               │               │    OK         │               │         │
│    │               │               │               │               │         │
│    │               │◄──────────────│               │               │         │
│    │               │  201 Created  │               │               │         │
│    │               │               │               │               │         │
│    │◄──────────────│               │               │               │         │
│    │  {"key": "api-key",           │               │               │         │
│    │   "limit": 100,               │               │               │         │
│    │   "window_ms": 60000}         │               │               │         │
│    │               │               │               │               │         │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Health Check Sequence

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           HEALTH CHECK FLOW                                  │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Client          Server         Handler          Health         Redis        │
│    │               │               │               │               │         │
│    │  GET /ready   │               │               │               │         │
│    │──────────────►│               │               │               │         │
│    │               │               │               │               │         │
│    │               │  route()      │               │               │         │
│    │               │──────────────►│               │               │         │
│    │               │               │               │               │         │
│    │               │               │ check_health()│               │         │
│    │               │               │──────────────►│               │         │
│    │               │               │               │               │         │
│    │               │               │               │ PING          │         │
│    │               │               │               │──────────────►│         │
│    │               │               │               │               │         │
│    │               │               │               │◄──────────────│         │
│    │               │               │               │    PONG       │         │
│    │               │               │               │               │         │
│    │               │               │◄──────────────│               │         │
│    │               │               │ HealthStatus{                 │         │
│    │               │               │   status: "healthy",          │         │
│    │               │               │   redis: "connected"}         │         │
│    │               │               │               │               │         │
│    │               │◄──────────────│               │               │         │
│    │               │  200 OK       │               │               │         │
│    │               │               │               │               │         │
│    │◄──────────────│               │               │               │         │
│    │ {"status": "healthy",         │               │               │         │
│    │  "dependencies": {            │               │               │         │
│    │    "redis": "connected"}}     │               │               │         │
│    │               │               │               │               │         │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Component Details

### File-to-Responsibility Mapping

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         SOURCE FILE RESPONSIBILITIES                         │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  src/                                                                        │
│  ├── main.rs ─────────────────► Binary entry point, starts server            │
│  │                               • Loads configuration                       │
│  │                               • Initializes logging                       │
│  │                               • Starts HTTP server                        │
│  │                                                                           │
│  ├── lib.rs ──────────────────► Library crate root, public exports           │
│  │                               • Re-exports public types                   │
│  │                               • Module declarations                       │
│  │                                                                           │
│  ├── server.rs ───────────────► HTTP server setup                            │
│  │                               • Axum router configuration                 │
│  │                               • Middleware stack (CORS, tracing)          │
│  │                               • Graceful shutdown handling                │
│  │                               • Route definitions                         │
│  │                                                                           │
│  ├── handlers.rs ─────────────► HTTP request handlers                        │
│  │                               • check_rate_limit() - POST /:key/check     │
│  │                               • set_rate_limit() - POST /:key             │
│  │                               • get_rate_limit() - GET /:key              │
│  │                               • delete_rate_limit() - DELETE /:key        │
│  │                               • health() - GET /health                    │
│  │                               • ready() - GET /ready                      │
│  │                                                                           │
│  ├── throttler.rs ────────────► Service orchestrator                         │
│  │                               • Rate limit rule management                │
│  │                               • should_throttle() decision logic          │
│  │                               • Coordinates RateLimiter + Redis           │
│  │                                                                           │
│  ├── rate_limiter.rs ─────────► Core rate limiting engine                    │
│  │                               • check_rate_limit() main entry             │
│  │                               • Local bucket management                   │
│  │                               • Redis integration                         │
│  │                               • Bucket CRUD operations                    │
│  │                                                                           │
│  ├── token_bucket.rs ─────────► Token bucket algorithm                       │
│  │                               • TokenBucket struct                        │
│  │                               • try_consume() atomic consumption          │
│  │                               • refill() time-based refill                │
│  │                               • Overflow protection                       │
│  │                                                                           │
│  ├── redis.rs ────────────────► Redis client wrapper                         │
│  │                               • Connection management                     │
│  │                               • get/set/delete bucket operations          │
│  │                               • Lua scripts for atomicity                 │
│  │                               • Health ping                               │
│  │                                                                           │
│  ├── config.rs ───────────────► Configuration loading                        │
│  │                               • Environment variable parsing              │
│  │                               • Default values                            │
│  │                               • Config struct definition                  │
│  │                                                                           │
│  ├── error.rs ────────────────► Error types and handling                     │
│  │                               • ThrottlerError enum                       │
│  │                               • HTTP status code mapping                  │
│  │                               • From<> implementations                    │
│  │                                                                           │
│  ├── validation.rs ───────────► Input validation                             │
│  │                               • Key format validation                     │
│  │                               • Parameter range validation                │
│  │                               • RequestValidator struct                   │
│  │                                                                           │
│  ├── response.rs ─────────────► Response DTOs                                │
│  │                               • CheckResponse                             │
│  │                               • ConfigResponse                            │
│  │                               • HealthResponse                            │
│  │                                                                           │
│  ├── health.rs ───────────────► Health checking                              │
│  │                               • HealthChecker struct                      │
│  │                               • Redis connectivity check                  │
│  │                               • Uptime tracking                           │
│  │                                                                           │
│  ├── metrics.rs ──────────────► Metrics collection                           │
│  │                               • Request counters                          │
│  │                               • Throttle event tracking                   │
│  │                               • Per-client statistics                     │
│  │                                                                           │
│  ├── key_generator.rs ────────► Rate limit key generation                    │
│  │                               • IP-based keys                             │
│  │                               • API key-based keys                        │
│  │                               • Composite key strategies                  │
│  │                                                                           │
│  ├── middleware.rs ───────────► Request middleware                           │
│  │                               • Request logging                           │
│  │                               • Client IP extraction                      │
│  │                                                                           │
│  └── algorithms/                                                             │
│      ├── mod.rs ──────────────► Algorithm trait definition                   │
│      │                           • RateLimitAlgorithm trait                  │
│      │                           • AlgorithmState struct                     │
│      │                                                                       │
│      └── sliding_window.rs ───► Sliding window algorithm                     │
│                                  • Alternative to token bucket               │
│                                  • Redis sorted set based                    │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Error Propagation

### Error Flow Diagram

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           ERROR PROPAGATION FLOW                             │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │                    Error Source Layers                              │     │
│  ├─────────────────────────────────────────────────────────────────────┤     │
│  │                                                                     │     │
│  │   Redis Layer                    Algorithm Layer                    │     │
│  │   ┌──────────────────┐          ┌──────────────────┐                │     │
│  │   │ redis::RedisError│          │ Insufficient     │                │     │
│  │   │ Connection failed│          │ tokens           │                │     │
│  │   └────────┬─────────┘          └────────┬─────────┘                │     │
│  │            │                             │                          │     │
│  │            │ From<redis::RedisError>     │                          │     │
│  │            │                             │                          │     │
│  │            ▼                             ▼                          │     │
│  │   ┌──────────────────────────────────────────────────────────┐      │     │
│  │   │              ThrottlerError (error.rs)                   │      │     │
│  │   ├──────────────────────────────────────────────────────────┤      │     │
│  │   │                                                          │      │     │
│  │   │  • RedisError(String)      ──────► 500 Internal Error    │      │     │
│  │   │  • ConfigError(String)     ──────► 400 Bad Request       │      │     │
│  │   │  • ValidationError(String) ──────► 400 Bad Request       │      │     │
│  │   │  • InvalidKey(String)      ──────► 400 Bad Request       │      │     │
│  │   │  • RateLimitExceeded{...}  ──────► 429 Too Many Requests │      │     │
│  │   │  • InternalError(String)   ──────► 500 Internal Error    │      │     │
│  │   │  • SerializationError(...) ──────► 500 Internal Error    │      │     │
│  │   │                                                          │      │     │
│  │   └──────────────────────────┬───────────────────────────────┘      │     │
│  │                              │                                      │     │
│  │                              │ impl IntoResponse                    │     │
│  │                              │                                      │     │
│  │                              ▼                                      │     │
│  │   ┌──────────────────────────────────────────────────────────┐      │     │
│  │   │              Axum HTTP Response                          │      │     │
│  │   ├──────────────────────────────────────────────────────────┤      │     │
│  │   │                                                          │      │     │
│  │   │  StatusCode + JSON body + Optional headers               │      │     │
│  │   │                                                          │      │     │
│  │   │  Example 429 response:                                   │      │     │
│  │   │  ┌────────────────────────────────────────────────────┐  │      │     │
│  │   │  │  HTTP/1.1 429 Too Many Requests                    │  │      │     │
│  │   │  │  Retry-After: 5                                    │  │      │     │
│  │   │  │  X-RateLimit-Limit: 100                            │  │      │     │
│  │   │  │  X-RateLimit-Remaining: 0                          │  │      │     │
│  │   │  │  Content-Type: application/json                    │  │      │     │
│  │   │  │                                                    │  │      │     │
│  │   │  │  {"error": "Rate limit exceeded",                  │  │      │     │
│  │   │  │   "retry_after": 5}                                │  │      │     │
│  │   │  └────────────────────────────────────────────────────┘  │      │     │
│  │   │                                                          │      │     │
│  │   └──────────────────────────────────────────────────────────┘      │     │
│  │                                                                     │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Concurrency Model

### Thread Safety Architecture

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         CONCURRENCY MODEL                                    │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │                     Tokio Runtime (Multi-threaded)                  │     │
│  ├─────────────────────────────────────────────────────────────────────┤     │
│  │                                                                     │     │
│  │   Worker Thread 1      Worker Thread 2      Worker Thread N         │     │
│  │   ┌────────────┐       ┌────────────┐       ┌────────────┐          │     │
│  │   │  Request A │       │  Request B │       │  Request C │          │     │
│  │   └─────┬──────┘       └─────┬──────┘       └─────┬──────┘          │     │
│  │         │                    │                    │                 │     │
│  │         └────────────────────┼────────────────────┘                 │     │
│  │                              │                                      │     │
│  │                              ▼                                      │     │
│  │   ┌──────────────────────────────────────────────────────────┐      │     │
│  │   │               Shared Application State                   │      │     │
│  │   │                                                          │      │     │
│  │   │   type SharedState = Arc<RwLock<AppState>>               │      │     │
│  │   │                                                          │      │     │
│  │   │   ┌─────────────────┐    ┌─────────────────┐             │      │     │
│  │   │   │   RateLimiter   │    │  RequestValidator│            │      │     │
│  │   │   │                 │    │                 │             │      │     │
│  │   │   │ Arc<RwLock<     │    │ (stateless)     │             │      │     │
│  │   │   │  HashMap<...>>> │    │                 │             │      │     │
│  │   │   └────────┬────────┘    └─────────────────┘             │      │     │
│  │   │            │                                             │      │     │
│  │   └────────────┼─────────────────────────────────────────────┘      │     │
│  │                │                                                    │     │
│  │                ▼                                                    │     │
│  │   ┌──────────────────────────────────────────────────────────┐      │     │
│  │   │              Lock Acquisition Strategy                   │      │     │
│  │   ├──────────────────────────────────────────────────────────┤      │     │
│  │   │                                                          │      │     │
│  │   │   READ OPERATIONS (concurrent):                          │      │     │
│  │   │   • check_rate_limit()  → .read().await                  │      │     │
│  │   │   • get_rate_limit()    → .read().await                  │      │     │
│  │   │   • health_check()      → .read().await                  │      │     │
│  │   │                                                          │      │     │
│  │   │   WRITE OPERATIONS (exclusive):                          │      │     │
│  │   │   • set_rate_limit()    → .write().await                 │      │     │
│  │   │   • delete_rate_limit() → .write().await                 │      │     │
│  │   │   • bucket modification → .write().await                 │      │     │
│  │   │                                                          │      │     │
│  │   └──────────────────────────────────────────────────────────┘      │     │
│  │                                                                     │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │                    Redis Atomicity (Distributed)                    │     │
│  ├─────────────────────────────────────────────────────────────────────┤     │
│  │                                                                     │     │
│  │   Problem: Multiple instances may read/modify same key              │     │
│  │                                                                     │     │
│  │   Solution: Lua script for atomic operations                        │     │
│  │                                                                     │     │
│  │   ┌────────────────────────────────────────────────────────────┐    │     │
│  │   │  -- Atomic token consumption                               │    │     │
│  │   │  local bucket = redis.call('GET', KEYS[1])                 │    │     │
│  │   │  if bucket then                                            │    │     │
│  │   │      bucket = cjson.decode(bucket)                         │    │     │
│  │   │      -- Refill tokens based on elapsed time                │    │     │
│  │   │      local elapsed = current_time - bucket.last_refill     │    │     │
│  │   │      bucket.tokens = min(capacity,                         │    │     │
│  │   │                          bucket.tokens + elapsed * rate)   │    │     │
│  │   │      -- Try to consume                                     │    │     │
│  │   │      if bucket.tokens >= 1 then                            │    │     │
│  │   │          bucket.tokens = bucket.tokens - 1                 │    │     │
│  │   │          redis.call('SET', KEYS[1], cjson.encode(bucket))  │    │     │
│  │   │          return {1, bucket.tokens}  -- allowed             │    │     │
│  │   │      end                                                   │    │     │
│  │   │      return {0, bucket.tokens}  -- denied                  │    │     │
│  │   │  end                                                       │    │     │
│  │   └────────────────────────────────────────────────────────────┘    │     │
│  │                                                                     │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Related Documentation

- [Architecture Overview](architecture.md) - High-level system design
- [API Documentation](api.md) - Endpoint reference
- [Deployment Guide](deployment.md) - Production deployment
- [Troubleshooting](troubleshooting.md) - Common issues
