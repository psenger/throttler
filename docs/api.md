# API Documentation

## Overview

The Throttler service provides RESTful APIs for configuring and monitoring rate limiting policies. All endpoints return JSON responses and support standard HTTP status codes.

## Authentication

Currently, the API does not require authentication. This may be added in future versions.

## Base URL

```
http://localhost:8080
```

## Endpoints

### Health Check

#### GET /health

Returns the health status of the service.

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "version": "1.0.0",
  "uptime_seconds": 3600
}
```

### Rate Limit Configuration

#### POST /api/v1/rate-limits

Create a new rate limiting policy.

**Request Body:**
```json
{
  "key": "api_key_123",
  "requests_per_second": 10,
  "burst_size": 20,
  "window_size_seconds": 60
}
```

**Response:**
```json
{
  "id": "rl_abc123",
  "key": "api_key_123",
  "requests_per_second": 10,
  "burst_size": 20,
  "window_size_seconds": 60,
  "created_at": "2024-01-15T10:30:00Z"
}
```

#### GET /api/v1/rate-limits/{id}

Retrieve a specific rate limiting policy.

**Response:**
```json
{
  "id": "rl_abc123",
  "key": "api_key_123",
  "requests_per_second": 10,
  "burst_size": 20,
  "window_size_seconds": 60,
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

#### PUT /api/v1/rate-limits/{id}

Update an existing rate limiting policy.

**Request Body:**
```json
{
  "requests_per_second": 15,
  "burst_size": 30
}
```

#### DELETE /api/v1/rate-limits/{id}

Delete a rate limiting policy.

**Response:**
```json
{
  "message": "Rate limit policy deleted successfully"
}
```

### Request Throttling

#### POST /api/v1/throttle

Check if a request should be throttled.

**Request Body:**
```json
{
  "key": "api_key_123",
  "client_id": "client_456",
  "endpoint": "/api/users"
}
```

**Response (Allowed):**
```json
{
  "allowed": true,
  "remaining": 9,
  "reset_time": "2024-01-15T10:31:00Z",
  "retry_after": null
}
```

**Response (Rate Limited):**
```json
{
  "allowed": false,
  "remaining": 0,
  "reset_time": "2024-01-15T10:31:00Z",
  "retry_after": 30
}
```

### Metrics

#### GET /api/v1/metrics

Retrieve throttling metrics and statistics.

**Response:**
```json
{
  "total_requests": 15420,
  "throttled_requests": 234,
  "active_rate_limits": 45,
  "redis_connections": 5,
  "uptime_seconds": 86400
}
```

## Error Responses

All error responses follow this format:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid request parameters",
    "details": {
      "field": "requests_per_second",
      "reason": "Must be greater than 0"
    }
  }
}
```

### Common Error Codes

- `VALIDATION_ERROR`: Request validation failed
- `NOT_FOUND`: Requested resource not found
- `RATE_LIMIT_EXCEEDED`: Too many requests
- `INTERNAL_ERROR`: Server error
- `REDIS_ERROR`: Redis connection or operation failed

## Status Codes

- `200 OK`: Request successful
- `201 Created`: Resource created successfully
- `400 Bad Request`: Invalid request parameters
- `404 Not Found`: Resource not found
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Server error
