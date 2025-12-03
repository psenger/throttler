# Usage Examples

## Quick Start

### 1. Start the Service

```bash
# Clone the repository
git clone <repository-url>
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

## Integration Examples

### Python Client

```python
import requests
import time

class ThrottlerClient:
    def __init__(self, base_url="http://localhost:8080"):
        self.base_url = base_url

    def set_rate_limit(self, key, requests_per_window, window_ms):
        """Configure rate limit for a key"""
        response = requests.post(
            f"{self.base_url}/rate-limit/{key}",
            json={
                "requests": requests_per_window,
                "window_ms": window_ms
            }
        )
        return response.json()

    def check_rate_limit(self, key, tokens=1):
        """Check if request is allowed and consume tokens"""
        response = requests.post(
            f"{self.base_url}/rate-limit/{key}/check",
            json={"tokens": tokens}
        )

        # Include headers in response
        result = response.json()
        result["headers"] = {
            "X-RateLimit-Limit": response.headers.get("X-RateLimit-Limit"),
            "X-RateLimit-Remaining": response.headers.get("X-RateLimit-Remaining"),
            "X-RateLimit-Reset": response.headers.get("X-RateLimit-Reset"),
        }
        return result

    def get_status(self, key):
        """Get current rate limit status"""
        response = requests.get(f"{self.base_url}/rate-limit/{key}")
        return response.json()

    def health_check(self):
        """Check service health"""
        response = requests.get(f"{self.base_url}/health")
        return response.json()

# Usage example
client = ThrottlerClient()

# Verify service is running
print("Health:", client.health_check())

# Configure rate limit: 5 requests per minute
client.set_rate_limit("api_user_123", requests_per_window=5, window_ms=60000)

# Test rate limiting
for i in range(7):
    result = client.check_rate_limit("api_user_123")
    if result.get("allowed", True):
        print(f"Request {i+1}: Allowed (remaining: {result['headers']['X-RateLimit-Remaining']})")
    else:
        retry_after = result.get("retry_after_seconds", 0)
        print(f"Request {i+1}: Throttled (retry after {retry_after}s)")
        time.sleep(retry_after)
```

### JavaScript/Node.js Client

```javascript
const axios = require('axios');

class ThrottlerClient {
  constructor(baseURL = 'http://localhost:8080') {
    this.client = axios.create({ baseURL });
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
        resetTime: response.headers['x-ratelimit-reset'],
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
    const response = await this.client.get(`/rate-limit/${key}`);
    return response.data;
  }

  async healthCheck() {
    const response = await this.client.get('/health');
    return response.data;
  }
}

// Usage example
(async () => {
  const throttler = new ThrottlerClient();

  try {
    // Check health
    console.log('Health:', await throttler.healthCheck());

    // Configure rate limit
    await throttler.setRateLimit('mobile_app_789', 10, 60000);

    // Test multiple requests
    for (let i = 0; i < 12; i++) {
      const result = await throttler.checkRateLimit('mobile_app_789');

      if (result.allowed) {
        console.log(`Request ${i+1}: Allowed (${result.remaining} remaining)`);
      } else {
        console.log(`Request ${i+1}: Throttled (retry after ${result.retryAfter}s)`);
        await new Promise(resolve => setTimeout(resolve, result.retryAfter * 1000));
      }
    }
  } catch (error) {
    console.error('Error:', error.message);
  }
})();
```

### Express.js Middleware

```javascript
const express = require('express');
const axios = require('axios');

function createThrottleMiddleware(throttlerUrl = 'http://localhost:8080') {
  return async (req, res, next) => {
    try {
      // Use API key from header or fall back to IP
      const key = req.headers['x-api-key'] || req.ip;

      const response = await axios.post(
        `${throttlerUrl}/rate-limit/${encodeURIComponent(key)}/check`,
        { tokens: 1 }
      );

      // Forward rate limit headers
      res.set({
        'X-RateLimit-Limit': response.headers['x-ratelimit-limit'],
        'X-RateLimit-Remaining': response.headers['x-ratelimit-remaining'],
        'X-RateLimit-Reset': response.headers['x-ratelimit-reset']
      });

      next();
    } catch (error) {
      if (error.response?.status === 429) {
        res.set('Retry-After', error.response.headers['retry-after']);
        return res.status(429).json({
          error: 'Too Many Requests',
          retry_after: error.response.headers['retry-after']
        });
      }

      // Fail open - allow request if throttler is unavailable
      console.error('Throttle check failed:', error.message);
      next();
    }
  };
}

// Express app setup
const app = express();

// Apply throttling middleware to API routes
app.use('/api', createThrottleMiddleware());

app.get('/api/users', (req, res) => {
  res.json({ users: ['Alice', 'Bob', 'Charlie'] });
});

app.get('/api/posts', (req, res) => {
  res.json({ posts: ['Post 1', 'Post 2', 'Post 3'] });
});

app.listen(3000, () => {
  console.log('API server running on port 3000');
});
```

## Docker Compose Development Setup

The project includes a complete Docker Compose setup for local development:

```bash
# Start Redis and Redis Commander
docker compose up -d

# View Redis data in browser
open http://localhost:8081

# Run the throttler service
cargo run
```

### Inspecting Redis State

Use Redis Commander at http://localhost:8081 to:
- View all rate limit keys
- Inspect token bucket state
- Monitor key expiration
- Debug rate limiting behavior

### Using redis-cli

```bash
# Connect to Redis with password
docker exec -it throttler-redis redis-cli -a $DOCKER_REDIS_PASSWORD

# List all throttler keys
KEYS throttler:*

# Inspect a specific key
GET throttler:rate_limit:my-api-key

# Monitor all Redis commands in real-time
MONITOR
```

## Load Testing

### Simple Burst Test

```bash
#!/bin/bash
# burst_test.sh

KEY="load-test-key"
REQUESTS=100

echo "Sending $REQUESTS requests..."

for i in $(seq 1 $REQUESTS); do
  STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST http://localhost:8080/rate-limit/$KEY/check \
    -H "Content-Type: application/json" \
    -d '{}')

  if [ "$STATUS" = "200" ]; then
    echo -n "."
  else
    echo -n "X"
  fi
done

echo ""
echo "Done!"
```

### Using Apache Bench

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

## Common Patterns

### Per-User Rate Limiting

```bash
# Each user gets their own rate limit
for user in user1 user2 user3; do
  curl -X POST "http://localhost:8080/rate-limit/$user" \
    -H "Content-Type: application/json" \
    -d '{"requests": 100, "window_ms": 60000}'
done
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
# Stricter limit for expensive operations
curl -X POST http://localhost:8080/rate-limit/user123:export \
  -H "Content-Type: application/json" \
  -d '{"requests": 5, "window_ms": 3600000}'  # 5 per hour

# Normal limit for regular API calls
curl -X POST http://localhost:8080/rate-limit/user123:api \
  -H "Content-Type: application/json" \
  -d '{"requests": 100, "window_ms": 60000}'  # 100 per minute
```
