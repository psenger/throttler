# Usage Examples

## Basic Setup

### 1. Start the Service

```bash
# Clone the repository
git clone https://github.com/your-org/throttler
cd throttler

# Set up environment
cp .env.example .env

# Start Redis (using Docker)
docker run -d -p 6379:6379 redis:alpine

# Build and run
cargo run
```

### 2. Create a Rate Limit Policy

```bash
curl -X POST http://localhost:8080/api/v1/rate-limits \
  -H "Content-Type: application/json" \
  -d '{
    "key": "user_123",
    "requests_per_second": 5,
    "burst_size": 10,
    "window_size_seconds": 60
  }'
```

## Integration Examples

### Python Client

```python
import requests
import time

class ThrottlerClient:
    def __init__(self, base_url="http://localhost:8080"):
        self.base_url = base_url
    
    def create_rate_limit(self, key, rps, burst_size, window_size):
        """Create a new rate limiting policy"""
        response = requests.post(
            f"{self.base_url}/api/v1/rate-limits",
            json={
                "key": key,
                "requests_per_second": rps,
                "burst_size": burst_size,
                "window_size_seconds": window_size
            }
        )
        return response.json()
    
    def check_throttle(self, key, client_id=None, endpoint=None):
        """Check if request should be throttled"""
        data = {"key": key}
        if client_id:
            data["client_id"] = client_id
        if endpoint:
            data["endpoint"] = endpoint
            
        response = requests.post(
            f"{self.base_url}/api/v1/throttle",
            json=data
        )
        return response.json()
    
    def get_metrics(self):
        """Get service metrics"""
        response = requests.get(f"{self.base_url}/api/v1/metrics")
        return response.json()

# Usage example
client = ThrottlerClient()

# Create rate limit
rate_limit = client.create_rate_limit(
    key="api_user_456",
    rps=10,
    burst_size=20,
    window_size=60
)
print(f"Created rate limit: {rate_limit['id']}")

# Test throttling
for i in range(15):
    result = client.check_throttle("api_user_456", client_id="web_client")
    if result["allowed"]:
        print(f"Request {i+1}: Allowed ({result['remaining']} remaining)")
    else:
        print(f"Request {i+1}: Throttled (retry after {result['retry_after']}s)")
        time.sleep(result['retry_after'])
```

### JavaScript/Node.js Client

```javascript
const axios = require('axios');

class ThrottlerClient {
  constructor(baseURL = 'http://localhost:8080') {
    this.client = axios.create({ baseURL });
  }

  async createRateLimit(key, requestsPerSecond, burstSize, windowSize) {
    const response = await this.client.post('/api/v1/rate-limits', {
      key,
      requests_per_second: requestsPerSecond,
      burst_size: burstSize,
      window_size_seconds: windowSize
    });
    return response.data;
  }

  async checkThrottle(key, clientId = null, endpoint = null) {
    const data = { key };
    if (clientId) data.client_id = clientId;
    if (endpoint) data.endpoint = endpoint;
    
    const response = await this.client.post('/api/v1/throttle', data);
    return response.data;
  }

  async getMetrics() {
    const response = await this.client.get('/api/v1/metrics');
    return response.data;
  }
}

// Usage example
(async () => {
  const throttler = new ThrottlerClient();
  
  try {
    // Create rate limit
    const rateLimit = await throttler.createRateLimit(
      'mobile_app_789',
      15,  // 15 requests per second
      30,  // burst size of 30
      60   // 60 second window
    );
    console.log('Created rate limit:', rateLimit.id);
    
    // Test multiple requests
    for (let i = 0; i < 20; i++) {
      const result = await throttler.checkThrottle(
        'mobile_app_789',
        'ios_client_v2',
        '/api/data'
      );
      
      if (result.allowed) {
        console.log(`Request ${i+1}: ✅ Allowed (${result.remaining} remaining)`);
        // Simulate API call
        await new Promise(resolve => setTimeout(resolve, 100));
      } else {
        console.log(`Request ${i+1}: ❌ Throttled (retry after ${result.retry_after}s)`);
        await new Promise(resolve => setTimeout(resolve, result.retry_after * 1000));
      }
    }
    
    // Get final metrics
    const metrics = await throttler.getMetrics();
    console.log('Final metrics:', metrics);
    
  } catch (error) {
    console.error('Error:', error.response?.data || error.message);
  }
})();
```

### Express.js Middleware

```javascript
const express = require('express');
const axios = require('axios');

// Throttling middleware
function createThrottleMiddleware(throttlerUrl = 'http://localhost:8080') {
  return async (req, res, next) => {
    try {
      // Extract key from API key header or IP address
      const key = req.headers['x-api-key'] || req.ip;
      
      const response = await axios.post(`${throttlerUrl}/api/v1/throttle`, {
        key,
        client_id: req.headers['user-agent'],
        endpoint: req.path
      });
      
      const result = response.data;
      
      // Add rate limit headers
      res.set({
        'X-RateLimit-Remaining': result.remaining,
        'X-RateLimit-Reset': result.reset_time
      });
      
      if (result.allowed) {
        next();
      } else {
        res.set('Retry-After', result.retry_after);
        res.status(429).json({
          error: 'Too Many Requests',
          retry_after: result.retry_after
        });
      }
    } catch (error) {
      console.error('Throttle check failed:', error.message);
      // Fail open - allow request if throttler is down
      next();
    }
  };
}

// Express app setup
const app = express();

// Apply throttling middleware
app.use('/api', createThrottleMiddleware());

// API routes
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

## Docker Compose Example

```yaml
# docker-compose.yml
version: '3.8'

services:
  throttler:
    build: .
    ports:
      - "8080:8080"
    environment:
      - REDIS_URL=redis://redis:6379
      - LOG_LEVEL=info
      - PORT=8080
    depends_on:
      - redis
    restart: unless-stopped

  redis:
    image: redis:alpine
    ports:
      - "6379:6379"
    command: redis-server --appendonly yes
    volumes:
      - redis_data:/data
    restart: unless-stopped

  app:
    build:
      context: ./example-app
    ports:
      - "3000:3000"
    environment:
      - THROTTLER_URL=http://throttler:8080
    depends_on:
      - throttler
    restart: unless-stopped

volumes:
  redis_data:
```

## Load Testing Example

```bash
#!/bin/bash
# load_test.sh

# Create rate limit policy
curl -X POST http://localhost:8080/api/v1/rate-limits \
  -H "Content-Type: application/json" \
  -d '{
    "key": "load_test",
    "requests_per_second": 100,
    "burst_size": 200,
    "window_size_seconds": 60
  }'

# Run load test with different tools

# Using Apache Bench
ab -n 1000 -c 50 -H "Content-Type: application/json" \
   -p throttle_request.json \
   http://localhost:8080/api/v1/throttle

# Using wrk
wrk -t4 -c50 -d30s -s throttle_script.lua http://localhost:8080/api/v1/throttle
```
