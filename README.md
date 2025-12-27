<p align="center">
  <img src="assets/logo.svg" alt="Throttler Logo" width="180" height="180">
</p>

<h1 align="center">Throttler</h1>

<p align="center">
  <strong>A high-performance, Redis-backed distributed rate limiting service written in Rust.</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#api-reference">API</a> •
  <a href="#documentation">Docs</a> •
  <a href="#contributing">Contributing</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/throttler"><img src="https://img.shields.io/crates/v/throttler?style=flat-square&logo=rust&label=crates.io" alt="Crates.io"></a>
  <a href="https://github.com/psenger/throttler/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT License"></a>
  <a href="https://github.com/psenger/throttler/stargazers"><img src="https://img.shields.io/github/stars/psenger/throttler?style=flat-square&logo=github" alt="GitHub Stars"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-1.70%2B-orange?style=flat-square&logo=rust" alt="Rust 1.70+">
  <img src="https://img.shields.io/badge/redis-7.0%2B-red?style=flat-square&logo=redis" alt="Redis 7.0+">
  <img src="https://img.shields.io/badge/tokio-async-blueviolet?style=flat-square" alt="Tokio Async">
  <img src="https://img.shields.io/badge/status-production--ready-brightgreen?style=flat-square" alt="Production Ready">
</p>

---

## Why Throttler?

> *"Our API got slammed at 3 AM. Again. By the time we noticed, the database was toast."*

**Throttler** protects your APIs from abuse with distributed rate limiting that scales horizontally. Built in Rust for maximum performance, it uses Redis for shared state across multiple instances.

| Problem                                      | Throttler Solution                          |
|----------------------------------------------|---------------------------------------------|
| Traffic spikes crashing services             | Token bucket algorithm absorbs bursts       |
| Single-instance rate limiting doesn't scale  | Redis-backed state works across instances   |
| Complex rate limiting logic in every service | Centralized REST API manages all limits     |
| Inconsistent rate limit headers              | Standard `X-RateLimit-*` headers everywhere |

---

## Features

<table>
<tr>
<td width="50%">

### Rate Limiting Engine
- **Token Bucket** — Smooth rate limiting with burst support
- **Sliding Window** — Precise request counting
- **Atomic Operations** — Thread-safe with Lua scripts
- **Auto Refill** — Time-based token replenishment

</td>
<td width="50%">

### Production Ready
- **RESTful API** — Full CRUD for rate configurations
- **Standard Headers** — `X-RateLimit-*`, `Retry-After`
- **Health Checks** — `/health` and `/ready` endpoints
- **Graceful Shutdown** — Completes in-flight requests

</td>
</tr>
<tr>
<td width="50%">

### Performance
- **Async/Await** — Built on Tokio runtime
- **Connection Pooling** — Efficient Redis connections
- **Zero-Copy** — Minimal allocations in hot path
- **Sub-millisecond** — Typical response times

</td>
<td width="50%">

### Observability
- **Prometheus Metrics** — Request rates, latencies
- **Structured Logging** — JSON with configurable levels
- **Redis Commander** — Visual inspection UI
- **Tracing** — Distributed request tracing

</td>
</tr>
</table>

---

## Quick Start

### Prerequisites

- [Rust 1.70+](https://rustup.rs/)
- [Docker](https://docs.docker.com/get-docker/) (for Redis)

### Installation

```bash
# Clone the repository
git clone https://github.com/psenger/throttler.git
cd throttler

# Copy environment configuration
cp .env.example .env

# Start Redis
docker compose up -d

# Build and run
cargo build --release
cargo run --release
```

Server starts at `http://localhost:8080`

### Your First Rate Limit

```bash
# 1. Create a rate limit: 100 requests per 60-second window
curl -X POST http://localhost:8080/rate-limit/my-api-key \
  -H "Content-Type: application/json" \
  -d '{"requests": 100, "window_ms": 60000}'

# 2. Check the rate limit (consumes 1 token)
curl -X POST http://localhost:8080/rate-limit/my-api-key/check

# 3. View current status
curl http://localhost:8080/rate-limit/my-api-key
```

---

## Architecture

```
┌─────────────────┐     ┌──────────────────────────────────┐     ┌─────────────────┐
│   Your APIs     │────▶│         Throttler Service        │────▶│      Redis      │
│   & Services    │◀────│            (Axum)                │◀────│   (State Store) │
└─────────────────┘     └──────────────────────────────────┘     └─────────────────┘
                                       │
                        ┌──────────────┼──────────────┐
                        ▼              ▼              ▼
                   ┌─────────┐   ┌──────────┐   ┌──────────┐
                   │  Token  │   │ Sliding  │   │ Metrics  │
                   │  Bucket │   │  Window  │   │ & Health │
                   └─────────┘   └──────────┘   └──────────┘
```

### Components

| Component     | Technology                                      | Purpose                              |
|---------------|-------------------------------------------------|--------------------------------------|
| HTTP Server   | [Axum](https://github.com/tokio-rs/axum)        | High-performance async web framework |
| Runtime       | [Tokio](https://tokio.rs/)                      | Async task scheduling and I/O        |
| State Store   | [Redis](https://redis.io/)                      | Distributed rate limit state         |
| Serialization | [Serde](https://serde.rs/)                      | JSON request/response handling       |
| Validation    | [Validator](https://github.com/Keats/validator) | Request input validation             |

---

## API Reference

### Endpoints

| Method   | Endpoint                 | Description                    |
|----------|--------------------------|--------------------------------|
| `GET`    | `/health`                | Liveness probe                 |
| `GET`    | `/ready`                 | Readiness probe (checks Redis) |
| `GET`    | `/rate-limit/:key`       | Get rate limit status          |
| `POST`   | `/rate-limit/:key`       | Create/update rate limit       |
| `DELETE` | `/rate-limit/:key`       | Delete rate limit              |
| `POST`   | `/rate-limit/:key/check` | Check and consume tokens       |

### Example: Check Rate Limit

**Request:**
```bash
curl -X POST http://localhost:8080/rate-limit/user-123/check \
  -H "Content-Type: application/json" \
  -d '{"tokens": 1}'
```

**Response (200 OK):**
```json
{
  "allowed": true,
  "remaining": 99,
  "limit": 100
}
```

**Response Headers:**
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 99
```

**Rate Limited Response (429 Too Many Requests):**
```json
{
  "allowed": false,
  "remaining": 0,
  "limit": 100
}
```

**Rate Limited Headers:**
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
Retry-After: 60
```

See [API Documentation](docs/api.md) for complete reference.

---

## Configuration

### Environment Variables

| Variable              | Default                  | Description                             |
|-----------------------|--------------------------|-----------------------------------------|
| `BIND_ADDRESS`        | `127.0.0.1:8080`         | Server bind address                     |
| `REDIS_URL`           | `redis://127.0.0.1:6379` | Redis connection URL                    |
| `DEFAULT_CAPACITY`    | `100`                    | Default bucket capacity                 |
| `DEFAULT_REFILL_RATE` | `10`                     | Default tokens per second               |
| `RUST_LOG`            | `info`                   | Log level (error/warn/info/debug/trace) |

### Docker Compose

The included `docker-compose.yml` provides:

- **Redis** on `localhost:6379`
- **Redis Commander** at `http://localhost:8081` (visual inspection)

```bash
# Start services
docker compose up -d

# View logs
docker compose logs -f

# Stop and cleanup
docker compose down -v
```

---

## Use Cases

### API Gateway Protection

```bash
# Free tier: 100 requests per hour (3600000 ms)
curl -X POST http://localhost:8080/rate-limit/free:client-123 \
  -H "Content-Type: application/json" \
  -d '{"requests": 100, "window_ms": 3600000}'

# Pro tier: 10,000 requests per hour
curl -X POST http://localhost:8080/rate-limit/pro:client-456 \
  -H "Content-Type: application/json" \
  -d '{"requests": 10000, "window_ms": 3600000}'
```

### Microservices Rate Limiting

```bash
# Limit expensive operations: 5 requests per hour
curl -X POST http://localhost:8080/rate-limit/reports:export \
  -H "Content-Type: application/json" \
  -d '{"requests": 5, "window_ms": 3600000}'
```

### Multi-Tenant SaaS

```javascript
// Dynamically set limits based on subscription tier
const limits = {
  free: { requests: 100, window_ms: 3600000 },      // 100/hour
  pro: { requests: 10000, window_ms: 3600000 },     // 10k/hour
  enterprise: { requests: 100000, window_ms: 3600000 } // 100k/hour
};

await fetch(`/rate-limit/${tier}:${customerId}`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(limits[tier])
});
```

---

## Documentation

| Document                                       | Description                            |
|------------------------------------------------|----------------------------------------|
| [API Reference](docs/api.md)                   | Complete endpoint documentation        |
| [Architecture](docs/architecture.md)           | System design and components           |
| [Code Architecture](docs/code-architecture.md) | Source code structure and data flow    |
| [Deployment](docs/deployment.md)               | Docker, Kubernetes, production setup   |
| [Examples](docs/examples.md)                   | Integration examples (Python, Node.js) |
| [Monitoring](docs/monitoring.md)               | Prometheus, Grafana, alerting          |
| [Troubleshooting](docs/troubleshooting.md)     | Common issues and solutions            |
| [Changelog](CHANGELOG.md)                      | Version history and releases           |

---

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Format code
cargo fmt

# Lint code
cargo clippy

# Run specific test
cargo test test_token_bucket -- --nocapture
```

---

## Roadmap

### Implemented

- [x] Token bucket rate limiting
- [x] Sliding window rate limiting
- [x] Redis-backed distributed state
- [x] RESTful API
- [x] Health/readiness endpoints
- [x] Prometheus metrics
- [x] Docker Compose setup
- [x] Comprehensive test suite

### Planned

- [ ] Fixed window algorithm
- [ ] Grafana dashboard templates
- [ ] Helm charts
- [ ] Circuit breaker pattern
- [ ] Request queuing
- [ ] WebSocket notifications

---

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) before submitting changes.

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/throttler.git

# Create a branch
git checkout -b feature/amazing-feature

# Make changes and test
cargo test

# Submit a pull request
```

### Issue Templates

- [Report a Bug](https://github.com/psenger/throttler/issues/new?template=bug_report.md)
- [Request a Feature](https://github.com/psenger/throttler/issues/new?template=feature_request.md)

---

## Security

If you discover a security vulnerability, please email the maintainer directly instead of opening a public issue. See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

---

## License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

---

## Author

**Philip A Senger**

- GitHub: [@psenger](https://github.com/psenger)
- Repository: [github.com/psenger/throttler](https://github.com/psenger/throttler)

---

<p align="center">
  <sub>Built with Rust for teams who take API reliability seriously.</sub>
</p>

<p align="center">
  <a href="#throttler">Back to top</a>
</p>
