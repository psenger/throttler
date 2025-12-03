# Architecture Overview

This document provides a high-level overview of the throttler service architecture and design decisions.

## System Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Client Apps   │────│   Throttler     │────│   Redis Cache   │
│                 │    │   Service       │    │   (Docker)      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │                       │
                       ┌──────┴──────┐         ┌──────┴──────┐
                       │   Axum      │         │   Redis     │
                       │   Server    │         │   Commander │
                       └─────────────┘         └─────────────┘
```

## Docker Infrastructure

The project includes a Docker Compose setup for local development:

```yaml
services:
  redis:           # Redis 7.2-alpine with password auth
  redis-commander: # Web UI for Redis inspection
```

### Redis Configuration

Located at `docker/redis/redis.conf`:
- Memory limit: 256MB with LRU eviction
- Single database (index 0)
- Protected mode enabled
- Connection timeout: 300s

## Core Components

### 1. Rate Limiter (`src/rate_limiter.rs`)
Implements rate limiting with dual-mode support:
- **Local mode**: In-memory HashMap for single-instance deployments
- **Distributed mode**: Redis-backed for multi-instance deployments
- Thread-safe operations with RwLock
- Automatic bucket cleanup for expired entries

### 2. Token Bucket (`src/token_bucket.rs`)
Core algorithm implementation:
- Configurable capacity and refill rate
- Time-based token refill with overflow protection
- Atomic token consumption
- Floating-point precision handling

### 3. Sliding Window (`src/algorithms/sliding_window.rs`)
Alternative rate limiting algorithm:
- Uses Redis sorted sets for timestamp tracking
- Automatic cleanup of expired entries
- More accurate for bursty traffic patterns

### 4. Throttler Service (`src/throttler.rs`)
Main orchestrator that:
- Manages rate limiter instances
- Handles configuration updates
- Provides health check status
- Coordinates Redis client lifecycle

### 5. HTTP Server (`src/server.rs`)
Axum-based HTTP server with:
- RESTful rate limiting endpoints
- Health and readiness checks
- CORS support
- Request/response tracing
- Graceful shutdown handling

### 6. Request Handlers (`src/handlers.rs`)
HTTP request processing with:
- Input validation via `RequestValidator`
- Key generation strategies
- Rate limit header injection (X-RateLimit-*)

### 7. Redis Client (`src/redis.rs`)
Redis integration providing:
- Connection pooling
- Atomic operations for distributed locking
- Token bucket state persistence
- Health ping for connectivity checks

## Data Flow

1. **Request Ingress**: Client → Axum Server → Handlers
2. **Validation**: RequestValidator checks key format and parameters
3. **Key Generation**: KeyGenerator creates composite keys (client IP, headers)
4. **Rate Check**: Throttler → RateLimiter → TokenBucket/SlidingWindow
5. **State Lookup**: Local HashMap or Redis (distributed mode)
6. **Response**: Rate limit headers + allow/deny status

## Design Patterns

### Error Handling (`src/error.rs`)
- Custom `ThrottlerError` enum with specific variants
- Implements `ResponseError` for automatic HTTP status mapping
- Graceful degradation on Redis failures

### Configuration (`src/config.rs`)
- Environment-based configuration with defaults
- Validation at startup via `ConfigValidator`
- Runtime configuration updates through API

### Middleware (`src/middleware.rs`)
- Request/response logging
- Metrics collection
- Error handling wrapper

## Scalability Considerations

### Horizontal Scaling
- Stateless service design (state in Redis)
- Load balancer compatible
- Consistent key hashing for Redis operations

### Performance
- Async/await throughout (Tokio runtime)
- Connection pooling for Redis
- Minimal allocations in hot paths

### Reliability
- Health checks (`/health`, `/ready`)
- Graceful shutdown with signal handling
- Redis connection health monitoring

## Security

- Input validation on all endpoints
- Password-protected Redis (via `DOCKER_REDIS_PASSWORD`)
- Localhost-only port bindings in development
- No sensitive data in logs

## Monitoring

- Prometheus-compatible metrics endpoint
- Structured logging with tracing
- Health endpoint for liveness/readiness probes
- Redis Commander UI for state inspection (http://localhost:8081)
