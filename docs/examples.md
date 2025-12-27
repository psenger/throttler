# Usage Examples

Comprehensive examples for integrating Throttler into your applications.

## Table of Contents

- [Quick Start](#quick-start)
- [SDK Clients](#sdk-clients)
  - [Python](#python-client)
  - [Node.js](#nodejs-client)
- [Framework Integration](#framework-integration)
  - [Express.js Middleware](#expressjs-middleware)
  - [FastAPI Middleware](#fastapi-middleware)
- [Common Patterns](#common-patterns)
- [Load Testing](#load-testing)
- [Debugging](#debugging)

---

## Quick Start

### 1. Start the Service

```bash
# Clone the repository
git clone https://github.com/psenger/throttler.git
cd throttler

# Set up environment
cp .env.example .env
# Edit .env and set DOCKER_REDIS_PASSWORD

# Start Redis with Docker Compose
docker compose up -d

# Build and run
cargo run
```

### 2. Verify Service is Running

```bash
# Health check
curl http://localhost:8080/health

# Readiness check (verifies Redis connection)
curl http://localhost:8080/ready
```

### 3. Basic Rate Limiting

```bash
# Configure rate limit: 10 requests per minute
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

---

## SDK Clients

### Python Client

A full-featured Python client for Throttler:

```python
import requests
import time
from typing import Optional, Dict, Any

class ThrottlerClient:
    """Python client for the Throttler rate limiting service."""

    def __init__(self, base_url: str = "http://localhost:8080"):
        self.base_url = base_url
        self.session = requests.Session()

    def health_check(self) -> Dict[str, Any]:
        """Check service health."""
        response = self.session.get(f"{self.base_url}/health")
        response.raise_for_status()
        return response.json()

    def ready_check(self) -> Dict[str, Any]:
        """Check service readiness (including Redis)."""
        response = self.session.get(f"{self.base_url}/ready")
        response.raise_for_status()
        return response.json()

    def set_rate_limit(
        self,
        key: str,
        requests_per_window: int,
        window_ms: int
    ) -> Dict[str, Any]:
        """Configure rate limit for a key."""
        response = self.session.post(
            f"{self.base_url}/rate-limit/{key}",
            json={
                "requests": requests_per_window,
                "window_ms": window_ms
            }
        )
        response.raise_for_status()
        return response.json()

    def check_rate_limit(
        self,
        key: str,
        tokens: int = 1
    ) -> Dict[str, Any]:
        """Check if request is allowed and consume tokens."""
        response = self.session.post(
            f"{self.base_url}/rate-limit/{key}/check",
            json={"tokens": tokens}
        )

        result = {
            "allowed": response.status_code == 200,
            "status_code": response.status_code,
            "data": response.json(),
            "headers": {
                "limit": response.headers.get("X-RateLimit-Limit"),
                "remaining": response.headers.get("X-RateLimit-Remaining"),
                "reset": response.headers.get("X-RateLimit-Reset"),
                "retry_after": response.headers.get("Retry-After"),
            }
        }
        return result

    def get_status(self, key: str) -> Optional[Dict[str, Any]]:
        """Get current rate limit status."""
        response = self.session.get(f"{self.base_url}/rate-limit/{key}")
        if response.status_code == 404:
            return None
        response.raise_for_status()
        return response.json()

    def delete_rate_limit(self, key: str) -> Dict[str, Any]:
        """Delete rate limit configuration."""
        response = self.session.delete(f"{self.base_url}/rate-limit/{key}")
        response.raise_for_status()
        return response.json()


# Usage example
if __name__ == "__main__":
    client = ThrottlerClient()

    # Verify service is running
    print("Health:", client.health_check())
    print("Ready:", client.ready_check())

    # Configure rate limit: 5 requests per minute
    client.set_rate_limit("api_user_123", requests_per_window=5, window_ms=60000)

    # Test rate limiting
    for i in range(7):
        result = client.check_rate_limit("api_user_123")

        if result["allowed"]:
            remaining = result["headers"]["remaining"]
            print(f"Request {i+1}: Allowed ({remaining} remaining)")
        else:
            retry_after = result["headers"]["retry_after"]
            print(f"Request {i+1}: Throttled (retry after {retry_after}s)")

    # Clean up
    client.delete_rate_limit("api_user_123")
```

### Node.js Client

A TypeScript/JavaScript client for Throttler:

```javascript
const axios = require('axios');

class ThrottlerClient {
  constructor(baseURL = 'http://localhost:8080') {
    this.client = axios.create({
      baseURL,
      headers: { 'Content-Type': 'application/json' }
    });
  }

  async healthCheck() {
    const response = await this.client.get('/health');
    return response.data;
  }

  async readyCheck() {
    const response = await this.client.get('/ready');
    return response.data;
  }

  async setRateLimit(key, requests, windowMs) {
    const response = await this.client.post(`/rate-limit/${key}`, {
      requests,
      window_ms: windowMs
    });
    return response.data;
  }

  async checkRateLimit(key, tokens = 1) {
    try {
      const response = await this.client.post(`/rate-limit/${key}/check`, { tokens });
      return {
        allowed: true,
        remaining: response.headers['x-ratelimit-remaining'],
        reset: response.headers['x-ratelimit-reset'],
        data: response.data
      };
    } catch (error) {
      if (error.response?.status === 429) {
        return {
          allowed: false,
          retryAfter: error.response.headers['retry-after'],
          data: error.response.data
        };
      }
      throw error;
    }
  }

  async getStatus(key) {
    try {
      const response = await this.client.get(`/rate-limit/${key}`);
      return response.data;
    } catch (error) {
      if (error.response?.status === 404) {
        return null;
      }
      throw error;
    }
  }

  async deleteRateLimit(key) {
    const response = await this.client.delete(`/rate-limit/${key}`);
    return response.data;
  }
}

// Usage example
(async () => {
  const throttler = new ThrottlerClient();

  try {
    // Check health
    console.log('Health:', await throttler.healthCheck());
    console.log('Ready:', await throttler.readyCheck());

    // Configure rate limit: 10 requests per minute
    await throttler.setRateLimit('mobile_app_789', 10, 60000);

    // Test multiple requests
    for (let i = 0; i < 12; i++) {
      const result = await throttler.checkRateLimit('mobile_app_789');

      if (result.allowed) {
        console.log(`Request ${i + 1}: Allowed (${result.remaining} remaining)`);
      } else {
        console.log(`Request ${i + 1}: Throttled (retry after ${result.retryAfter}s)`);
        // Wait before retrying
        await new Promise(resolve => setTimeout(resolve, result.retryAfter * 1000));
      }
    }

    // Clean up
    await throttler.deleteRateLimit('mobile_app_789');
  } catch (error) {
    console.error('Error:', error.message);
  }
})();
```

---

## Framework Integration

### Express.js Middleware

Rate limiting middleware for Express applications:

```javascript
const express = require('express');
const axios = require('axios');

/**
 * Creates Express middleware for Throttler rate limiting.
 * @param {Object} options - Configuration options
 * @param {string} options.throttlerUrl - Throttler service URL
 * @param {function} options.keyGenerator - Function to extract rate limit key from request
 * @param {boolean} options.failOpen - Allow requests if Throttler is unavailable
 */
function createThrottleMiddleware(options = {}) {
  const {
    throttlerUrl = 'http://localhost:8080',
    keyGenerator = (req) => req.headers['x-api-key'] || req.ip,
    failOpen = true
  } = options;

  return async (req, res, next) => {
    try {
      const key = keyGenerator(req);

      const response = await axios.post(
        `${throttlerUrl}/rate-limit/${encodeURIComponent(key)}/check`,
        { tokens: 1 },
        { timeout: 1000 } // 1 second timeout
      );

      // Forward rate limit headers to client
      res.set({
        'X-RateLimit-Limit': response.headers['x-ratelimit-limit'],
        'X-RateLimit-Remaining': response.headers['x-ratelimit-remaining'],
        'X-RateLimit-Reset': response.headers['x-ratelimit-reset']
      });

      next();
    } catch (error) {
      if (error.response?.status === 429) {
        // Rate limited
        res.set('Retry-After', error.response.headers['retry-after']);
        return res.status(429).json({
          error: 'Too Many Requests',
          message: error.response.data.message,
          retry_after: error.response.headers['retry-after']
        });
      }

      // Throttler unavailable
      if (failOpen) {
        console.error('Throttle check failed, failing open:', error.message);
        next();
      } else {
        return res.status(503).json({
          error: 'Service Unavailable',
          message: 'Rate limiting service unavailable'
        });
      }
    }
  };
}

// Express app setup
const app = express();

// Apply throttling to all /api routes
app.use('/api', createThrottleMiddleware({
  keyGenerator: (req) => {
    // Use API key if provided, otherwise use IP
    return req.headers['x-api-key'] || req.ip;
  }
}));

app.get('/api/users', (req, res) => {
  res.json({ users: ['Alice', 'Bob', 'Charlie'] });
});

app.get('/api/posts', (req, res) => {
  res.json({ posts: ['Post 1', 'Post 2', 'Post 3'] });
});

// Health endpoint (not rate limited)
app.get('/health', (req, res) => {
  res.json({ status: 'healthy' });
});

app.listen(3000, () => {
  console.log('API server running on http://localhost:3000');
});
```

### FastAPI Middleware

Rate limiting middleware for FastAPI (Python):

```python
from fastapi import FastAPI, Request, HTTPException
from fastapi.responses import JSONResponse
import httpx
from typing import Callable

app = FastAPI()

class ThrottleMiddleware:
    """FastAPI middleware for Throttler rate limiting."""

    def __init__(
        self,
        throttler_url: str = "http://localhost:8080",
        key_generator: Callable[[Request], str] = None,
        fail_open: bool = True
    ):
        self.throttler_url = throttler_url
        self.key_generator = key_generator or self._default_key_generator
        self.fail_open = fail_open

    def _default_key_generator(self, request: Request) -> str:
        """Default: use API key header or client IP."""
        return request.headers.get("x-api-key", request.client.host)

    async def __call__(self, request: Request, call_next):
        # Skip health endpoints
        if request.url.path in ["/health", "/ready"]:
            return await call_next(request)

        try:
            key = self.key_generator(request)

            async with httpx.AsyncClient(timeout=1.0) as client:
                response = await client.post(
                    f"{self.throttler_url}/rate-limit/{key}/check",
                    json={"tokens": 1}
                )

                if response.status_code == 429:
                    data = response.json()
                    return JSONResponse(
                        status_code=429,
                        content={
                            "error": "Too Many Requests",
                            "message": data.get("message"),
                            "retry_after": data.get("retry_after_seconds")
                        },
                        headers={"Retry-After": response.headers.get("retry-after", "60")}
                    )

                # Continue with rate limit headers
                response_obj = await call_next(request)
                response_obj.headers["X-RateLimit-Limit"] = response.headers.get("x-ratelimit-limit", "")
                response_obj.headers["X-RateLimit-Remaining"] = response.headers.get("x-ratelimit-remaining", "")
                response_obj.headers["X-RateLimit-Reset"] = response.headers.get("x-ratelimit-reset", "")
                return response_obj

        except Exception as e:
            if self.fail_open:
                print(f"Throttle check failed, failing open: {e}")
                return await call_next(request)
            else:
                return JSONResponse(
                    status_code=503,
                    content={"error": "Rate limiting service unavailable"}
                )

# Apply middleware
app.middleware("http")(ThrottleMiddleware())

@app.get("/api/users")
async def get_users():
    return {"users": ["Alice", "Bob", "Charlie"]}

@app.get("/health")
async def health():
    return {"status": "healthy"}
```

---

## Common Patterns

### Per-User Rate Limiting

```bash
# Each user gets their own rate limit bucket
for user in user1 user2 user3; do
  curl -X POST "http://localhost:8080/rate-limit/$user" \
    -H "Content-Type: application/json" \
    -d '{"requests": 100, "window_ms": 60000}'
done
```

### Tiered Rate Limits

```bash
# Different limits based on subscription tier
TIERS='{"free": 10, "pro": 100, "enterprise": 1000}'

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
# Different limits for different operations
# Strict limit for expensive exports (5 per hour)
curl -X POST http://localhost:8080/rate-limit/user123:export \
  -H "Content-Type: application/json" \
  -d '{"requests": 5, "window_ms": 3600000}'

# Normal limit for API calls (100 per minute)
curl -X POST http://localhost:8080/rate-limit/user123:api \
  -H "Content-Type: application/json" \
  -d '{"requests": 100, "window_ms": 60000}'

# Generous limit for read operations (1000 per minute)
curl -X POST http://localhost:8080/rate-limit/user123:read \
  -H "Content-Type: application/json" \
  -d '{"requests": 1000, "window_ms": 60000}'
```

### Composite Keys

```bash
# Combine multiple dimensions
# Format: {tier}:{user}:{endpoint}
curl -X POST http://localhost:8080/rate-limit/pro:user123:api \
  -H "Content-Type: application/json" \
  -d '{"requests": 100, "window_ms": 60000}'
```

---

## Load Testing

### Simple Burst Test

```bash
#!/bin/bash
# burst_test.sh - Test rate limiting with rapid requests

KEY="load-test-key"
REQUESTS=100

echo "Configuring rate limit: 50 requests/minute"
curl -s -X POST "http://localhost:8080/rate-limit/$KEY" \
  -H "Content-Type: application/json" \
  -d '{"requests": 50, "window_ms": 60000}'

echo ""
echo "Sending $REQUESTS requests..."

ALLOWED=0
DENIED=0

for i in $(seq 1 $REQUESTS); do
  STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "http://localhost:8080/rate-limit/$KEY/check" \
    -H "Content-Type: application/json" \
    -d '{}')

  if [ "$STATUS" = "200" ]; then
    ALLOWED=$((ALLOWED + 1))
    echo -n "."
  else
    DENIED=$((DENIED + 1))
    echo -n "X"
  fi
done

echo ""
echo "Results: $ALLOWED allowed, $DENIED denied"

# Clean up
curl -s -X DELETE "http://localhost:8080/rate-limit/$KEY"
```

### Using Apache Bench (ab)

```bash
# Create request payload
echo '{}' > /tmp/throttle_request.json

# Run load test: 1000 requests, 50 concurrent
ab -n 1000 -c 50 \
   -H "Content-Type: application/json" \
   -p /tmp/throttle_request.json \
   http://localhost:8080/rate-limit/ab-test/check
```

### Using wrk

```lua
-- throttle_test.lua
wrk.method = "POST"
wrk.body   = "{}"
wrk.headers["Content-Type"] = "application/json"
```

```bash
# Run 30-second load test with 4 threads and 50 connections
wrk -t4 -c50 -d30s -s throttle_test.lua \
    http://localhost:8080/rate-limit/wrk-test/check
```

---

## Debugging

### Inspecting Redis State

Use Redis Commander at http://localhost:8081 to:
- View all rate limit keys (`throttler:*`)
- Inspect token bucket state
- Monitor key TTLs
- Debug rate limiting behavior

### Using redis-cli

```bash
# Connect to Redis with password
docker exec -it throttler-redis redis-cli -a $DOCKER_REDIS_PASSWORD

# List all throttler keys
KEYS throttler:*

# Inspect a specific key
GET throttler:rate_limit:my-api-key

# Check key TTL
TTL throttler:rate_limit:my-api-key

# Monitor all Redis commands in real-time
MONITOR
```

### Enable Debug Logging

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Throttler-only debug logs
RUST_LOG=throttler=debug cargo run

# Include Redis operations
RUST_LOG=throttler=debug,redis=debug cargo run
```
