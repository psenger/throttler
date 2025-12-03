# API Documentation

## Overview

The Throttler service provides RESTful APIs for rate limiting and request throttling. All endpoints return JSON responses and support standard HTTP status codes.

## Base URL

```
http://localhost:8080
```

## Endpoints

### Health & Readiness

#### GET /health

Returns the health status of the service.

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "version": "0.1.0"
}
```

#### GET /ready

Returns the readiness status (checks Redis connectivity).

**Response:**
```json
{
  "status": "ready",
  "redis": "connected"
}
```

### Rate Limiting

#### GET /rate-limit/:key

Retrieve the rate limit configuration for a specific key.

**Example:**
```bash
curl http://localhost:8080/rate-limit/api-key-123
```

**Response:**
```json
{
  "key": "api-key-123",
  "capacity": 100,
  "refill_rate": 10,
  "remaining": 85,
  "reset_time": 1705312260
}
```

**Error Response (404):**
```json
{
  "error": "not_found",
  "message": "No configuration found for key: api-key-123"
}
```

#### POST /rate-limit/:key

Create or update a rate limit configuration.

**Request Body:**
```json
{
  "requests": 100,
  "window_ms": 60000
}
```

**Example:**
```bash
curl -X POST http://localhost:8080/rate-limit/api-key-123 \
  -H "Content-Type: application/json" \
  -d '{"requests": 100, "window_ms": 60000}'
```

**Response:**
```json
{
  "status": "success",
  "message": "Rate limit configuration updated",
  "key": "api-key-123",
  "requests": 100,
  "window_ms": 60000
}
```

#### DELETE /rate-limit/:key

Delete a rate limit configuration.

**Example:**
```bash
curl -X DELETE http://localhost:8080/rate-limit/api-key-123
```

**Response:**
```json
{
  "status": "success",
  "message": "Rate limit configuration deleted",
  "key": "api-key-123"
}
```

#### POST /rate-limit/:key/check

Check if a request should be allowed and consume tokens.

**Request Body:**
```json
{
  "key": "api-key-123",
  "requests": 1,
  "window_ms": 60000,
  "headers": {}
}
```

**Example:**
```bash
curl -X POST http://localhost:8080/rate-limit/api-key-123/check \
  -H "Content-Type: application/json" \
  -d '{"requests": 1}'
```

**Response (Allowed):**
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

**Response (Rate Limited - 429):**
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

## Error Responses

All error responses follow this format:

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
| `not_found` | 404 | Resource not found |
| `rate_limit_exceeded` | 429 | Too many requests |
| `internal_error` | 500 | Server error |
| `configuration_error` | 400 | Invalid configuration |

### Validation Errors

```json
{
  "error": "validation_error",
  "message": "Invalid key format: key-with-invalid-chars!"
}
```

## Rate Limit Headers

All rate-limited responses include these headers:

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Maximum requests allowed |
| `X-RateLimit-Remaining` | Remaining requests in window |
| `X-RateLimit-Reset` | Unix timestamp when limit resets |
| `X-RateLimit-Window` | Window size in milliseconds |
| `Retry-After` | Seconds to wait (only on 429) |

## Request Validation

### Key Format
- Must be alphanumeric with `-`, `_`, `:`, `.` allowed
- Maximum length: 256 characters
- Cannot be empty

### Rate Limit Values
- `requests`: Must be > 0 and <= 10000
- `window_ms`: Must be >= 1000 and <= 86400000 (24 hours)

## Examples

### Basic Rate Limiting

```bash
# Set up rate limit: 10 requests per minute
curl -X POST http://localhost:8080/rate-limit/my-api-key \
  -H "Content-Type: application/json" \
  -d '{"requests": 10, "window_ms": 60000}'

# Check rate limit (consume 1 token)
curl -X POST http://localhost:8080/rate-limit/my-api-key/check \
  -H "Content-Type: application/json" \
  -d '{}'

# Get current status
curl http://localhost:8080/rate-limit/my-api-key
```

### Burst Testing

```bash
# Send 15 rapid requests
for i in {1..15}; do
  curl -s -o /dev/null -w "%{http_code}\n" \
    -X POST http://localhost:8080/rate-limit/burst-test/check \
    -H "Content-Type: application/json" \
    -d '{"requests": 10, "window_ms": 60000}'
done
```
