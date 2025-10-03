# Throttler

A lightweight Rust web API rate limiting and request throttling service that provides configurable rate limiting middleware for web APIs to prevent abuse and ensure fair resource usage.

## Features

- **Token Bucket Rate Limiting**: Implements efficient token bucket algorithm for smooth rate limiting
- **Redis-backed Distributed Throttling**: Scales across multiple instances with Redis persistence
- **RESTful Configuration API**: Dynamic configuration management via HTTP endpoints
- **Configurable Rate Limits**: Support for different rate limits per API key or user tier
- **Real-time Statistics**: Monitor usage patterns and rate limit statistics

## Installation

### Prerequisites
- Rust 1.70+ 
- Redis server

### Build from Source

```bash
git clone https://github.com/yourusername/throttler.git
cd throttler
cargo build --release
```

## Usage

### Basic Usage

1. Start Redis server:
```bash
redis-server
```

2. Run throttler with default configuration:
```bash
cargo run
# or
./target/release/throttler
```

3. Run with custom configuration:
```bash
cargo run -- --config custom-config.toml --bind 127.0.0.1:8080
```

### API Endpoints

**Check Rate Limit:**
```bash
curl -X POST http://localhost:3000/api/v1/throttle/user:123
```

**Get Configuration:**
```bash
curl http://localhost:3000/api/v1/config
```

**Update Configuration:**
```bash
curl -X POST http://localhost:3000/api/v1/config \
  -H "Content-Type: application/json" \
  -d '{"requests_per_second": 100}'
```

**Get Statistics:**
```bash
curl http://localhost:3000/api/v1/stats/user:123
```

**Health Check:**
```bash
curl http://localhost:3000/health
```

### Configuration

Create `config.toml`:
```toml
redis_url = "redis://localhost:6379"

[default_limits]
requests_per_second = 50
burst_capacity = 100
window_seconds = 60

[custom_limits.premium]
requests_per_second = 1000
burst_capacity = 2000
window_seconds = 60

[custom_limits.basic]
requests_per_second = 10
burst_capacity = 20
window_seconds = 60
```

### Environment Variables

```bash
THROTTLER_REDIS_URL=redis://localhost:6379
THROTTLER_DEFAULT_LIMITS_REQUESTS_PER_SECOND=50
THROTTLER_DEFAULT_LIMITS_BURST_CAPACITY=100
```

## License

MIT