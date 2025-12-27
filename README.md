<p align="center">
  <img src="assets/logo.svg" alt="Throttler Logo" width="180" height="180">
</p>

<h1 align="center">Throttler</h1>

<p align="center">
  <strong>Distributed rate limiting that scales with your API.</strong>
</p>

<p align="center">
  <a href="#what-is-throttler">What is it?</a> &bull;
  <a href="#features">Features</a> &bull;
  <a href="#quick-start">Quick Start</a> &bull;
  <a href="#architecture">Architecture</a> &bull;
  <a href="#use-cases">Use Cases</a> &bull;
  <a href="#documentation">Documentation</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-1.70%2B-orange?logo=rust" alt="Rust 1.70+">
  <img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT License">
  <img src="https://img.shields.io/badge/redis-7.2-red?logo=redis" alt="Redis 7.2">
  <img src="https://img.shields.io/badge/status-production--ready-brightgreen" alt="Production Ready">
</p>

---

## What is Throttler?

> *"Our API got slammed at 3 AM. Again. By the time we noticed, the database was toast."*

**Throttler** is a high-performance, Redis-backed rate limiting service written in Rust. It provides distributed rate limiting for your APIs with microsecond-level latency, ensuring your services stay protected during traffic spikes without sacrificing performance.

Think of it as **nginx rate limiting meets Redis** — but with a clean REST API, multiple algorithms, and production-grade reliability out of the box.

---

## Features

### Core Rate Limiting Engine

- **Multiple Algorithms** — Token bucket for smooth rate limiting, sliding window for precise counting
- **Distributed State** — Redis-backed for seamless multi-instance deployments
- **Atomic Operations** — Thread-safe token consumption with zero race conditions
- **Automatic Refill** — Time-based token replenishment with overflow protection

### Production-Ready API

- **RESTful Interface** — Complete CRUD operations for rate limit configurations
- **Standard Headers** — `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `Retry-After`
- **Input Validation** — Request validation with detailed error responses
- **Health Checks** — Kubernetes-ready `/health` and `/ready` endpoints

### Performance & Reliability

- **Async/Await** — Built on Tokio for maximum concurrency
- **Connection Pooling** — Efficient Redis connection management
- **Graceful Shutdown** — Clean shutdown with in-flight request completion
- **Fail-Open Option** — Continue serving when Redis is temporarily unavailable

### Observability

- **Prometheus Metrics** — Request rates, latencies, and rate limit violations
- **Structured Logging** — JSON logs with configurable verbosity
- **Redis Commander** — Visual Redis inspection for development

---

## Quick Start

### Prerequisites

- **Rust 1.70+** — [Install Rust](https://rustup.rs/)
- **Docker & Docker Compose** — For Redis

### 1. Clone and Configure

```bash
git clone https://github.com/psenger/throttler.git
cd throttler

# Create environment file
cp .env.example .env

# Set your Redis password in .env
# DOCKER_REDIS_PASSWORD=your_secure_password_here
```

### 2. Start Redis

```bash
docker compose up -d
```

This starts:
- **Redis** on `localhost:6379` (password protected)
- **Redis Commander** at [http://localhost:8081](http://localhost:8081) (visual inspection)

### 3. Build and Run

```bash
cargo build --release
cargo run --release
```

The service starts on `http://localhost:8080`.

### 4. Verify It Works

```bash
# Health check
curl http://localhost:8080/health

# Configure a rate limit: 10 requests per minute
curl -X POST http://localhost:8080/rate-limit/my-api-key \
  -H "Content-Type: application/json" \
  -d '{"requests": 10, "window_ms": 60000}'

# Check rate limit (consume 1 token)
curl -X POST http://localhost:8080/rate-limit/my-api-key/check \
  -H "Content-Type: application/json" \
  -d '{}'
```

**Reference:** [Getting Started Guide](docs/examples.md)

---

## Architecture

```
┌─────────────────┐     ┌──────────────────────────────────┐     ┌─────────────────┐
│   Client Apps   │────▶│         Throttler Service        │────▶│   Redis Cache   │
│   (Your APIs)   │     │                                  │     │   (Distributed) │
└─────────────────┘     └──────────────────────────────────┘     └─────────────────┘
                                       │
                        ┌──────────────┼──────────────┐
                        ▼              ▼              ▼
                   ┌─────────┐   ┌──────────┐   ┌──────────┐
                   │  Token  │   │ Sliding  │   │  Health  │
                   │  Bucket │   │  Window  │   │  Checks  │
                   └─────────┘   └──────────┘   └──────────┘
```

| Component       | Technology | Purpose                              |
|-----------------|------------|--------------------------------------|
| **HTTP Server** | Axum       | High-performance async web framework |
| **Runtime**     | Tokio      | Async task scheduling and I/O        |
| **State Store** | Redis      | Distributed rate limit state         |
| **Algorithms**  | Custom     | Token bucket, sliding window         |
| **Validation**  | Validator  | Request input validation             |

### Data Flow

1. **Request** arrives at Axum HTTP server
2. **Validation** checks key format and parameters
3. **Rate Limiter** queries Redis for current token count
4. **Algorithm** (token bucket/sliding window) computes allowance
5. **Response** includes rate limit headers and allow/deny status

**Reference:** [Architecture Deep Dive](docs/architecture.md)

---

## API Reference

| Method   | Endpoint                 | Description                     |
|----------|--------------------------|---------------------------------|
| `GET`    | `/health`                | Liveness probe                  |
| `GET`    | `/ready`                 | Readiness probe (checks Redis)  |
| `GET`    | `/rate-limit/:key`       | Get rate limit configuration    |
| `POST`   | `/rate-limit/:key`       | Create/update rate limit        |
| `DELETE` | `/rate-limit/:key`       | Delete rate limit configuration |
| `POST`   | `/rate-limit/:key/check` | Check and consume tokens        |

### Example: Rate Limit Check

```bash
curl -X POST http://localhost:8080/rate-limit/user-123/check \
  -H "Content-Type: application/json" \
  -d '{"tokens": 1}'
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
```

**Reference:** [Full API Documentation](docs/api.md)

---

## Use Cases

### API Gateway Integration

Protect your APIs from abuse with per-client rate limiting:

```bash
# Free tier: 100 requests/hour
curl -X POST http://localhost:8080/rate-limit/free:client-123 \
  -d '{"requests": 100, "window_ms": 3600000}'

# Pro tier: 10,000 requests/hour
curl -X POST http://localhost:8080/rate-limit/pro:client-456 \
  -d '{"requests": 10000, "window_ms": 3600000}'
```

### Microservices Protection

Prevent cascading failures with service-level throttling:

```bash
# Limit database-heavy operations
curl -X POST http://localhost:8080/rate-limit/reports:export \
  -d '{"requests": 5, "window_ms": 3600000}'
```

### Multi-Tenant SaaS

Different rate limits per customer tier — all managed via API:

```javascript
const throttler = new ThrottlerClient();

// Dynamically adjust based on subscription
await throttler.setRateLimit(
  `${plan}:${customerId}`,
  tierLimits[plan].requests,
  tierLimits[plan].windowMs
);
```

**Reference:** [Integration Examples](docs/examples.md)

---

## Configuration

### Environment Variables

| Variable                      | Default                  | Description                             |
|-------------------------------|--------------------------|-----------------------------------------|
| `THROTTLER_HOST`              | `127.0.0.1`              | Server bind address                     |
| `THROTTLER_PORT`              | `8080`                   | Server port                             |
| `REDIS_URL`                   | `redis://127.0.0.1:6379` | Redis connection URL                    |
| `REDIS_MAX_CONNECTIONS`       | `10`                     | Connection pool size                    |
| `DEFAULT_RATE_LIMIT_CAPACITY` | `100`                    | Default bucket capacity                 |
| `DEFAULT_RATE_LIMIT_REFILL`   | `10`                     | Default tokens per second               |
| `RUST_LOG`                    | `info`                   | Log level (error/warn/info/debug/trace) |

**Reference:** [Deployment Guide](docs/deployment.md)

---

## Documentation

| Resource                                       | Description                               |
|------------------------------------------------|-------------------------------------------|
| [API Documentation](docs/api.md)               | Complete endpoint reference with examples |
| [Architecture Overview](docs/architecture.md)  | System design and component details       |
| [Code Architecture](docs/code-architecture.md) | Code structure, diagrams, and data flow   |
| [Deployment Guide](docs/deployment.md)         | Docker, Kubernetes, and production setup  |
| [Usage Examples](docs/examples.md)             | Python, Node.js, and middleware examples  |
| [Monitoring Guide](docs/monitoring.md)         | Prometheus, Grafana, and alerting         |
| [Troubleshooting](docs/troubleshooting.md)     | Common issues and solutions               |
| [Contributing](CONTRIBUTING.md)                | Guidelines for contributors               |
| [Changelog](CHANGELOG.md)                      | Version history and release notes         |

---

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Format code
cargo fmt

# Run linter
cargo clippy

# Run specific test
cargo test test_token_bucket_edge_cases -- --nocapture
```

### Docker Commands

```bash
# Start Redis infrastructure
docker compose up -d

# View logs
docker compose logs -f

# Stop and clean up
docker compose down -v
```

---

## Project Status

### Implemented

- Token bucket rate limiting algorithm
- Sliding window rate limiting algorithm
- Redis-backed distributed state
- RESTful configuration API
- Health and readiness endpoints
- Prometheus-compatible metrics
- Structured JSON logging
- Docker Compose development setup
- Kubernetes deployment manifests
- Comprehensive test suite

### Planned

- Additional algorithms (fixed window, sliding log)
- Grafana dashboard templates
- Helm charts
- Circuit breaker pattern
- Request queuing and backpressure
- WebSocket notifications
- Plugin system for custom algorithms

---

## Contributing

Contributions are welcome! Please read our **[Contributing Guide](CONTRIBUTING.md)** before submitting changes.

### Quick Start for Contributors

```bash
# Clone the repository
git clone https://github.com/psenger/throttler.git
cd throttler

# Install dependencies and run tests
cargo build
cargo test

# Make your changes and submit a PR
```

### Issue Templates

We use GitHub issue templates to streamline contributions:

- **[Report a Bug](https://github.com/psenger/throttler/issues/new?template=bug_report.md)** — Found a problem? Let us know
- **[Request a Feature](https://github.com/psenger/throttler/issues/new?template=feature_request.md)** — Have an idea? We'd love to hear it

---

## License

MIT License — see [LICENSE](LICENSE) for details.

---

<p align="center">
  <sub>Built with Rust for teams who take API reliability seriously.</sub>
</p>
