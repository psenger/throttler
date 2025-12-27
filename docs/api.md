# API Documentation

Complete reference for the Throttler REST API.

## Table of Contents

- [Overview](#overview)
- [Health Endpoints](#health-endpoints)
- [Rate Limiting Endpoints](#rate-limiting-endpoints)
- [Request/Response Format](#requestresponse-format)
- [Error Handling](#error-handling)
- [Rate Limit Headers](#rate-limit-headers)
- [Examples](#examples)

---

## Overview

The Throttler service provides a RESTful API for rate limiting and request throttling. All endpoints accept and return JSON.

**Base URL:**
```
http://localhost:8080
```

**Content Type:**
```
Content-Type: application/json
```

---

## Health Endpoints

### GET /health

Liveness probe for container orchestrators.

**Request:**
```bash
curl http://localhost:8080/health
```

**Response (200 OK):**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "version": "0.1.0"
}
```

### GET /ready

Readiness probe that verifies Redis connectivity.

**Request:**
```bash
curl http://localhost:8080/ready
```

**Response (200 OK):**
```json
{
  "status": "ready",
  "redis": "connected"
}
```

**Response (503 Service Unavailable):**
```json
{
  "status": "not_ready",
  "redis": "disconnected"
}
```

---

## Rate Limiting Endpoints

### GET /rate-limit/:key

Retrieve the current rate limit configuration and status for a key.

**Request:**
```bash
curl http://localhost:8080/rate-limit/api-key-123
```

**Response (200 OK):**
```json
{
  "key": "api-key-123",
  "capacity": 100,
  "refill_rate": 10,
  "remaining": 85,
  "reset_time": 1705312260
}
```

**Response (404 Not Found):**
```json
{
  "error": "not_found",
  "message": "No configuration found for key: api-key-123"
}
```

---

### POST /rate-limit/:key

Create or update a rate limit configuration.

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `requests` | integer | Yes | Maximum requests per window |
| `window_ms` | integer | Yes | Window size in milliseconds |

**Request:**
```bash
curl -X POST http://localhost:8080/rate-limit/api-key-123 \
  -H "Content-Type: application/json" \
  -d '{"requests": 100, "window_ms": 60000}'
```

**Response (200 OK):**
```json
{
  "status": "success",
  "message": "Rate limit configuration updated",
  "key": "api-key-123",
  "requests": 100,
  "window_ms": 60000
}
```

**Response (400 Bad Request):**
```json
{
  "error": "validation_error",
  "message": "requests must be between 1 and 10000"
}
```

---

### DELETE /rate-limit/:key

Delete a rate limit configuration.

**Request:**
```bash
curl -X DELETE http://localhost:8080/rate-limit/api-key-123
```

**Response (200 OK):**
```json
{
  "status": "success",
  "message": "Rate limit configuration deleted",
  "key": "api-key-123"
}
```

---

### POST /rate-limit/:key/check

Check if a request is allowed and consume tokens.

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `tokens` | integer | No | Tokens to consume (default: 1) |
| `requests` | integer | No | Create if not exists |
| `window_ms` | integer | No | Create if not exists |

**Request:**
```bash
curl -X POST http://localhost:8080/rate-limit/api-key-123/check \
  -H "Content-Type: application/json" \
  -d '{"tokens": 1}'
```

**Response (200 OK - Allowed):**
```json
{
  "allowed": true,
  "remaining": 99,
  "reset_time": 1705312260
}
```

**Response Headers:**
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 99
X-RateLimit-Reset: 1705312260
X-RateLimit-Window: 60000
```

**Response (429 Too Many Requests):**
```json
{
  "error": "rate_limit_exceeded",
  "message": "Rate limit exceeded: 100 requests per 60000ms window. Retry after 30s",
  "retry_after_seconds": 30,
  "limit": 100,
  "window_ms": 60000
}
```

**Response Headers:**
```
Retry-After: 30
X-RateLimit-Limit: 100
X-RateLimit-Window: 60000
```

---

## Request/Response Format

### Key Format

Rate limit keys must follow these rules:

| Rule | Value |
|------|-------|
| Allowed characters | `a-z`, `A-Z`, `0-9`, `-`, `_`, `:`, `.` |
| Maximum length | 256 characters |
| Minimum length | 1 character |

**Valid Keys:**
```
user-123
api_key:production
tier:free:client-456
my.service.endpoint
```

**Invalid Keys:**
```
key with spaces
key/with/slashes
key!with@special#chars
```

### Rate Limit Values

| Field | Minimum | Maximum |
|-------|---------|---------|
| `requests` | 1 | 10,000 |
| `window_ms` | 1,000 (1 second) | 86,400,000 (24 hours) |

---

## Error Handling

All errors follow a consistent format:

```json
{
  "error": "error_code",
  "message": "Human-readable error message"
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `validation_error` | 400 | Request validation failed |
| `invalid_key` | 400 | Key format is invalid |
| `not_found` | 404 | Rate limit config not found |
| `rate_limit_exceeded` | 429 | Too many requests |
| `internal_error` | 500 | Server error |
| `redis_error` | 500 | Redis connection/operation failed |

### Validation Error Examples

```json
{
  "error": "validation_error",
  "message": "Invalid key format: key-with-invalid-chars!"
}
```

```json
{
  "error": "validation_error",
  "message": "requests must be between 1 and 10000"
}
```

```json
{
  "error": "validation_error",
  "message": "window_ms must be between 1000 and 86400000"
}
```

---

## Rate Limit Headers

All rate-limited responses include these headers:

| Header | Description | Example |
|--------|-------------|---------|
| `X-RateLimit-Limit` | Maximum requests allowed | `100` |
| `X-RateLimit-Remaining` | Remaining requests in window | `99` |
| `X-RateLimit-Reset` | Unix timestamp when limit resets | `1705312260` |
| `X-RateLimit-Window` | Window size in milliseconds | `60000` |
| `Retry-After` | Seconds to wait (only on 429) | `30` |

---

## Examples

### Basic Rate Limiting Flow

```bash
# 1. Configure rate limit: 10 requests per minute
curl -X POST http://localhost:8080/rate-limit/my-api-key \
  -H "Content-Type: application/json" \
  -d '{"requests": 10, "window_ms": 60000}'

# 2. Check rate limit (consume 1 token)
curl -X POST http://localhost:8080/rate-limit/my-api-key/check \
  -H "Content-Type: application/json" \
  -d '{}'

# 3. Get current status
curl http://localhost:8080/rate-limit/my-api-key

# 4. Clean up
curl -X DELETE http://localhost:8080/rate-limit/my-api-key
```

### Burst Testing

```bash
# Send 15 rapid requests to a 10-request limit
for i in {1..15}; do
  STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST http://localhost:8080/rate-limit/burst-test/check \
    -H "Content-Type: application/json" \
    -d '{"requests": 10, "window_ms": 60000}')
  echo "Request $i: $STATUS"
done
```

**Expected Output:**
```
Request 1: 200
Request 2: 200
...
Request 10: 200
Request 11: 429
Request 12: 429
...
```

### Tiered Rate Limits

```bash
# Free tier: 10 requests/minute
curl -X POST http://localhost:8080/rate-limit/free:user123 \
  -H "Content-Type: application/json" \
  -d '{"requests": 10, "window_ms": 60000}'

# Pro tier: 100 requests/minute
curl -X POST http://localhost:8080/rate-limit/pro:user456 \
  -H "Content-Type: application/json" \
  -d '{"requests": 100, "window_ms": 60000}'

# Enterprise tier: 1000 requests/minute
curl -X POST http://localhost:8080/rate-limit/enterprise:user789 \
  -H "Content-Type: application/json" \
  -d '{"requests": 1000, "window_ms": 60000}'
```

### Endpoint-Specific Limits

```bash
# Stricter limit for expensive operations (5 per hour)
curl -X POST http://localhost:8080/rate-limit/user123:export \
  -H "Content-Type: application/json" \
  -d '{"requests": 5, "window_ms": 3600000}'

# Normal limit for regular API calls (100 per minute)
curl -X POST http://localhost:8080/rate-limit/user123:api \
  -H "Content-Type: application/json" \
  -d '{"requests": 100, "window_ms": 60000}'
```

---

## SDK Examples

See [Usage Examples](examples.md) for Python, Node.js, and middleware integration examples.
