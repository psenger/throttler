# Throttler

A lightweight Rust web API rate limiting and request throttling service with Redis-backed distributed state management.

## Features

- **Multiple Algorithms**: Token bucket and sliding window rate limiting
- **Distributed**: Redis backend for multi-instance deployments
- **RESTful API**: Complete CRUD operations for rate limit configurations
- **High Performance**: Async/await with Tokio runtime
- **Monitoring**: Built-in health checks and metrics endpoints
- **Validation**: Request validation with detailed error responses

## Quick Start

### Prerequisites

- Rust 1.70+
- Docker and Docker Compose (for Redis)

### 1. Clone and Configure

```bash
git clone <repository-url>
cd throttler

# Create environment file
cp .env.example .env
# Edit .env and set DOCKER_REDIS_PASSWORD
```

### 2. Start Redis

```bash
docker compose up -d
```

This starts:
- **Redis** on `localhost:6379` (password protected)
- **Redis Commander** on `http://localhost:8081` (web UI for Redis)

### 3. Build and Run

```bash
cargo build --release
cargo run --release
```

The service starts on `http://localhost:8080`.

### 4. Verify

```bash
# Health check
curl http://localhost:8080/health

# Check rate limit
curl -X POST http://localhost:8080/rate-limit/test-key/check \
  -H "Content-Type: application/json" \
  -d '{"tokens": 1}'
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/ready` | Readiness check |
| GET | `/rate-limit/:key` | Get rate limit config |
| POST | `/rate-limit/:key` | Set rate limit config |
| DELETE | `/rate-limit/:key` | Delete rate limit config |
| POST | `/rate-limit/:key/check` | Check/consume tokens |

## Configuration

Environment variables (see `.env.example`):

| Variable | Default | Description |
|----------|---------|-------------|
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis connection URL |
| `BIND_ADDRESS` | `127.0.0.1:8080` | Server bind address |
| `DEFAULT_CAPACITY` | `100` | Default bucket capacity |
| `DEFAULT_REFILL_RATE` | `10` | Tokens per second |
| `DOCKER_REDIS_PASSWORD` | - | Redis password for Docker |

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
```

## Docker Commands

```bash
# Start services
docker compose up -d

# View logs
docker compose logs -f

# Stop services
docker compose down

# Stop and remove volumes
docker compose down -v
```

## Documentation

- [API Documentation](docs/api.md)
- [Architecture Overview](docs/architecture.md)
- [Deployment Guide](docs/deployment.md)
- [Usage Examples](docs/examples.md)
- [Monitoring Guide](docs/monitoring.md)
- [Troubleshooting](docs/troubleshooting.md)

## License

MIT
