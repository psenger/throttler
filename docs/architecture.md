# Architecture Overview

This document provides a comprehensive overview of the Throttler service architecture, design decisions, and component interactions.

## Table of Contents

- [System Overview](#system-overview)
- [Core Components](#core-components)
- [Data Flow](#data-flow)
- [Rate Limiting Algorithms](#rate-limiting-algorithms)
- [Infrastructure](#infrastructure)
- [Design Patterns](#design-patterns)
- [Scalability](#scalability)
- [Security](#security)

---

## System Overview

Throttler is designed as a stateless rate limiting service that delegates state management to Redis. This architecture enables horizontal scaling while maintaining consistent rate limiting across all instances.

```
                                   Load Balancer
                                        │
                    ┌───────────────────┼───────────────────┐
                    ▼                   ▼                   ▼
            ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
            │  Throttler   │    │  Throttler   │    │  Throttler   │
            │  Instance 1  │    │  Instance 2  │    │  Instance N  │
            └──────┬───────┘    └──────┬───────┘    └──────┬───────┘
                   │                   │                   │
                   └───────────────────┼───────────────────┘
                                       ▼
                              ┌─────────────────┐
                              │     Redis       │
                              │  (Shared State) │
                              └─────────────────┘
```

### Key Design Principles

| Principle              | Implementation                          |
|------------------------|-----------------------------------------|
| **Stateless Services** | All state stored in Redis               |
| **Horizontal Scaling** | Add instances behind load balancer      |
| **Fail-Fast**          | Quick responses, no blocking operations |
| **Observability**      | Metrics, health checks, structured logs |

---

## Core Components

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Throttler Service                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────┐  │
│  │   Server    │───▶│  Handlers   │───▶│      Throttler          │  │
│  │   (Axum)    │    │ (Validation)│    │   (Orchestrator)        │  │
│  └─────────────┘    └─────────────┘    └───────────┬─────────────┘  │
│                                                    │                │
│                           ┌────────────────────────┴────────┐       │
│                           ▼                                 ▼       │
│                    ┌─────────────┐                    ┌───────────┐ │
│                    │ Rate Limiter│                    │   Redis   │ │
│                    │             │                    │   Client  │ │
│                    └──────┬──────┘                    └───────────┘ │
│                           │                                         │
│               ┌───────────┴───────────┐                             │
│               ▼                       ▼                             │
│        ┌─────────────┐        ┌──────────────┐                      │
│        │Token Bucket │        │Sliding Window│                      │
│        │  Algorithm  │        │  Algorithm   │                      │
│        └─────────────┘        └──────────────┘                      │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 1. HTTP Server (`src/server.rs`)

The entry point for all HTTP requests, built on Axum.

**Responsibilities:**
- Route registration and request dispatching
- CORS configuration
- Request/response tracing middleware
- Graceful shutdown handling

**Key Features:**
- Async request handling with Tokio
- Tower middleware integration
- Signal-based shutdown (SIGTERM, SIGINT)

### 2. Request Handlers (`src/handlers.rs`)

Process incoming HTTP requests and validate inputs.

**Responsibilities:**
- Input validation via `RequestValidator`
- Key generation and normalization
- Rate limit header injection
- Error response formatting

**Validation Rules:**
- Keys: alphanumeric with `-`, `_`, `:`, `.` (max 256 chars)
- Requests: 1 to 10,000 per window
- Window: 1 second to 24 hours

### 3. Throttler Service (`src/throttler.rs`)

The main orchestrator that coordinates rate limiting operations.

**Responsibilities:**
- Rate limiter lifecycle management
- Configuration management
- Health check coordination
- Redis client management

### 4. Rate Limiter (`src/rate_limiter.rs`)

Core rate limiting engine with dual-mode support.

**Modes:**

| Mode        | Storage           | Use Case                     |
|-------------|-------------------|------------------------------|
| Local       | In-memory HashMap | Single instance, development |
| Distributed | Redis             | Multi-instance, production   |

**Features:**
- Thread-safe with RwLock
- Automatic bucket cleanup
- Configurable algorithms

### 5. Token Bucket Algorithm (`src/algorithms/token_bucket.rs`)

Implements the token bucket rate limiting algorithm.

```
┌─────────────────────────────────────────────────┐
│                 Token Bucket                    │
│                                                 │
│  Capacity: 100 tokens                           │
│  Refill Rate: 10 tokens/second                  │
│                                                 │
│  ┌──┬──┬──┬──┬──┬──┬──┬──┬──┬──┐                │
│  │██│██│██│██│██│██│  │  │  │  │  (60 tokens)   │
│  └──┴──┴──┴──┴──┴──┴──┴──┴──┴──┘                │
│                                                 │
│  Request: Consume 1 token ──▶ Allowed           │
│  Request: Consume 100 tokens ──▶ Denied         │
│                                                 │
└─────────────────────────────────────────────────┘
```

**Characteristics:**
- Smooth rate limiting
- Allows controlled bursts up to capacity
- Time-based token refill
- Overflow protection

### 6. Sliding Window Algorithm (`src/algorithms/sliding_window.rs`)

Alternative algorithm using Redis sorted sets.

```
Window: 60 seconds
Limit: 10 requests

Timeline:
0s   10s   20s   30s   40s   50s   60s   70s   80s
├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤
│  *  │ * * │  *  │     │ * * │  *  │ * * │     │
│     │     │     │     │     │     │     │     │
└─────────────────────────────────────────┘     │
        Current Window (7 requests)             │
                                    └───────────┘
                                    New Window
```

**Characteristics:**
- Precise request counting
- No burst allowance
- Better for strict rate limiting
- Higher Redis overhead

### 7. Redis Client (`src/redis.rs`)

Redis integration layer for distributed state.

**Features:**
- Connection pooling
- Atomic Lua scripts for race-free operations
- Health ping for connectivity checks
- Automatic reconnection

---

## Data Flow

### Request Processing Pipeline

```
1. HTTP Request
        │
        ▼
2. ┌──────────────┐
   │ Axum Router  │  Route matching
   └──────┬───────┘
          │
          ▼
3. ┌──────────────┐
   │  Middleware  │  Logging, tracing, CORS
   └──────┬───────┘
          │
          ▼
4. ┌──────────────┐
   │   Handler    │  Validation, key generation
   └──────┬───────┘
          │
          ▼
5. ┌──────────────┐
   │  Throttler   │  Rate limit orchestration
   └──────┬───────┘
          │
          ▼
6. ┌──────────────┐
   │ Rate Limiter │  Algorithm execution
   └──────┬───────┘
          │
          ▼
7. ┌──────────────┐
   │    Redis     │  State read/write
   └──────┬───────┘
          │
          ▼
8. HTTP Response (with rate limit headers)
```

### Token Consumption Flow

```rust
// Simplified pseudocode
async fn check_rate_limit(key: &str, tokens: u32) -> Result<RateLimitResult> {
    // 1. Get current bucket state from Redis
    let bucket = redis.get_bucket(key).await?;

    // 2. Refill tokens based on elapsed time
    let refilled = bucket.refill(elapsed_time);

    // 3. Attempt to consume tokens
    if refilled.tokens >= tokens {
        let remaining = refilled.tokens - tokens;
        redis.set_bucket(key, remaining).await?;
        Ok(RateLimitResult::Allowed { remaining })
    } else {
        Ok(RateLimitResult::Denied { retry_after })
    }
}
```

---

## Rate Limiting Algorithms

### Algorithm Comparison

| Feature          | Token Bucket                 | Sliding Window                   |
|------------------|------------------------------|----------------------------------|
| Burst handling   | Allows bursts up to capacity | No bursts allowed                |
| Precision        | Approximate                  | Exact                            |
| Memory usage     | O(1) per key                 | O(requests) per key              |
| Redis operations | GET + SET                    | ZADD + ZCOUNT + ZREMRANGEBYSCORE |
| Best for         | APIs with burst tolerance    | Strict rate enforcement          |

### Token Bucket Details

The token bucket algorithm is the default and recommended choice:

1. **Initialization**: Bucket starts with `capacity` tokens
2. **Refill**: Tokens added at `refill_rate` per second
3. **Consume**: Request removes tokens if available
4. **Overflow**: Tokens capped at `capacity`

```
Time 0:    [████████████████████] 100/100 tokens
Request:   Consume 10 tokens
Time 0+:   [████████████████    ] 90/100 tokens
Time 1s:   [██████████████████  ] 100/100 tokens (refilled)
```

### Sliding Window Details

Uses Redis sorted sets to track individual request timestamps:

1. **Record**: Each request adds timestamp to sorted set
2. **Count**: Count entries within current window
3. **Cleanup**: Remove expired entries
4. **Decision**: Allow if count < limit

---

## Infrastructure

### Docker Compose Setup

```yaml
services:
  redis:
    image: redis:7.2-alpine
    container_name: throttler-redis
    command: redis-server /usr/local/etc/redis/redis.conf
    ports:
      - "127.0.0.1:6379:6379"  # Localhost only
    volumes:
      - ./docker/redis/redis.conf:/usr/local/etc/redis/redis.conf:ro
      - redis-data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
    deploy:
      resources:
        limits:
          cpus: '0.50'
          memory: 512M

  redis-commander:
    image: rediscommander/redis-commander:latest
    container_name: throttler-redis-commander
    ports:
      - "127.0.0.1:8081:8081"  # Development only
    depends_on:
      redis:
        condition: service_healthy
```

### Redis Configuration

Located at `docker/redis/redis.conf`:

| Setting            | Value       | Purpose                      |
|--------------------|-------------|------------------------------|
| `maxmemory`        | 256MB       | Memory limit                 |
| `maxmemory-policy` | allkeys-lru | Eviction policy              |
| `databases`        | 1           | Single database (index 0)    |
| `timeout`          | 300         | Connection timeout (seconds) |
| `protected-mode`   | yes         | Security enforcement         |

---

## Design Patterns

### Error Handling (`src/error.rs`)

Custom error type with automatic HTTP status mapping:

| Error Variant       | HTTP Status | Description             |
|---------------------|-------------|-------------------------|
| `RateLimitExceeded` | 429         | Too many requests       |
| `ValidationError`   | 400         | Invalid input           |
| `InvalidKey`        | 400         | Malformed key           |
| `NotFound`          | 404         | Configuration not found |
| `RedisError`        | 500         | Redis operation failed  |
| `InternalError`     | 500         | Unexpected error        |

### Configuration (`src/config.rs`)

Environment-based configuration with sensible defaults:

```rust
pub struct Config {
    pub host: String,           // Default: "127.0.0.1"
    pub port: u16,              // Default: 8080
    pub redis_url: String,      // Default: "redis://127.0.0.1:6379"
    pub default_capacity: u32,  // Default: 100
    pub default_refill: u32,    // Default: 10
}
```

---

## Scalability

### Horizontal Scaling

Throttler is designed for horizontal scaling:

1. **Stateless Design**: All state in Redis
2. **Load Balancer**: Any load balancing strategy works
3. **Consistent Hashing**: Redis keys are stable across instances
4. **Connection Pooling**: Efficient Redis connection reuse

### Scaling Considerations

| Instances | Redis Connections | Recommended Redis   |
|-----------|-------------------|---------------------|
| 1-3       | 10-30             | Single Redis        |
| 4-10      | 40-100            | Redis with replicas |
| 10+       | 100+              | Redis Cluster       |

### Performance Characteristics

| Metric        | Typical Value              |
|---------------|----------------------------|
| Latency (p50) | < 1ms                      |
| Latency (p99) | < 5ms                      |
| Throughput    | 10,000+ req/s per instance |
| Memory        | ~50MB per instance         |

---

## Security

### Security Measures

| Layer       | Protection                         |
|-------------|------------------------------------|
| **Input**   | Validation on all endpoints        |
| **Redis**   | Password authentication required   |
| **Network** | Localhost-only port bindings (dev) |
| **Logs**    | No sensitive data logged           |
| **Headers** | Standard rate limit headers only   |

### Production Recommendations

1. **TLS for Redis**: Use `rediss://` URLs in production
2. **Network Isolation**: Private network for Redis
3. **Authentication**: Strong Redis passwords
4. **Firewall**: Restrict Redis access to Throttler instances
5. **Secrets Management**: Use vault/secrets manager for passwords

---

## Monitoring Integration

### Health Endpoints

| Endpoint  | Purpose   | Check           |
|-----------|-----------|-----------------|
| `/health` | Liveness  | Service running |
| `/ready`  | Readiness | Redis connected |

### Prometheus Metrics

Key metrics exposed at `/metrics`:

- `throttler_requests_total` - Request count by status
- `throttler_request_duration_seconds` - Latency histogram
- `throttler_rate_limit_hits_total` - Rate limit violations
- `throttler_redis_operations_total` - Redis operation count

See [Monitoring Guide](monitoring.md) for complete details.
