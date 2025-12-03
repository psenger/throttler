# Deployment Guide

This guide covers various deployment scenarios for the throttler service.

## Prerequisites

- Rust 1.70+ (for building from source)
- Docker and Docker Compose
- Redis server (version 6.0 or later) for production deployments

## Local Development

### 1. Configure Environment

```bash
# Copy example environment file
cp .env.example .env

# Edit .env and set a secure Redis password
# DOCKER_REDIS_PASSWORD=your_secure_password_here
# REDIS_URL=redis://:your_secure_password_here@127.0.0.1:6379
```

### 2. Start Redis with Docker Compose

```bash
# Start Redis and Redis Commander
docker compose up -d

# Verify services are running
docker compose ps

# View logs
docker compose logs -f redis
```

Services started:
- **Redis**: `localhost:6379` (password protected)
- **Redis Commander**: `http://localhost:8081` (web UI)

### 3. Build and Run the Service

```bash
# Build
cargo build --release

# Run
cargo run --release

# Or run with debug logging
RUST_LOG=debug cargo run
```

### 4. Verify Deployment

```bash
# Health check
curl http://localhost:8080/health

# Readiness check
curl http://localhost:8080/ready

# Test rate limiting
curl -X POST http://localhost:8080/rate-limit/test-key/check \
  -H "Content-Type: application/json" \
  -d '{"tokens": 1}'
```

## Docker Compose Reference

The included `docker-compose.yml` provides:

```yaml
services:
  redis:
    image: redis:7.2-alpine
    container_name: throttler-redis
    command: redis-server /usr/local/etc/redis/redis.conf --requirepass ${DOCKER_REDIS_PASSWORD}
    ports:
      - "127.0.0.1:6379:6379"  # Localhost only
    volumes:
      - ./docker/redis/redis.conf:/usr/local/etc/redis/redis.conf:ro
      - redis-data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "-a", "${DOCKER_REDIS_PASSWORD}", "ping"]
    deploy:
      resources:
        limits:
          cpus: '0.50'
          memory: 512M

  redis-commander:
    image: rediscommander/redis-commander:latest
    container_name: throttler-redis-commander
    environment:
      - REDIS_HOSTS=local:redis:6379:0:${DOCKER_REDIS_PASSWORD}
    ports:
      - "127.0.0.1:8081:8081"
    depends_on:
      redis:
        condition: service_healthy
```

### Docker Compose Commands

```bash
# Start services in background
docker compose up -d

# Stop services
docker compose down

# Stop and remove volumes (clears Redis data)
docker compose down -v

# View logs
docker compose logs -f

# Restart Redis
docker compose restart redis
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `THROTTLER_HOST` | `127.0.0.1` | Server host |
| `THROTTLER_PORT` | `8080` | Server port |
| `DOCKER_REDIS_PASSWORD` | - | Redis password (required) |
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis connection URL |
| `REDIS_MAX_CONNECTIONS` | `10` | Redis connection pool size |
| `REDIS_CONNECTION_TIMEOUT` | `5` | Connection timeout (seconds) |
| `DEFAULT_RATE_LIMIT_CAPACITY` | `100` | Default bucket capacity |
| `DEFAULT_RATE_LIMIT_REFILL` | `10` | Default refill rate |
| `RUST_LOG` | `info` | Log level |

## Production Deployment

### Building for Production

```bash
# Build optimized release binary
cargo build --release

# Binary located at
./target/release/throttler
```

### Docker Image

```dockerfile
# Dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/throttler /usr/local/bin/throttler
EXPOSE 8080
CMD ["throttler"]
```

```bash
# Build and run
docker build -t throttler .
docker run -p 8080:8080 --env-file .env throttler
```

### Docker Compose with Application

```yaml
version: '3.8'

services:
  throttler:
    build: .
    ports:
      - "8080:8080"
    environment:
      - REDIS_URL=redis://:${DOCKER_REDIS_PASSWORD}@redis:6379
      - THROTTLER_HOST=0.0.0.0
      - THROTTLER_PORT=8080
    depends_on:
      redis:
        condition: service_healthy
    restart: unless-stopped

  redis:
    image: redis:7.2-alpine
    command: redis-server --requirepass ${DOCKER_REDIS_PASSWORD}
    volumes:
      - redis_data:/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "redis-cli", "-a", "${DOCKER_REDIS_PASSWORD}", "ping"]
      interval: 10s
      timeout: 5s
      retries: 3

volumes:
  redis_data:
```

## Kubernetes Deployment

### ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: throttler-config
data:
  THROTTLER_HOST: "0.0.0.0"
  THROTTLER_PORT: "8080"
  RUST_LOG: "throttler=info"
```

### Secret

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: throttler-secrets
type: Opaque
stringData:
  REDIS_URL: "redis://:password@redis-service:6379"
```

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: throttler
spec:
  replicas: 3
  selector:
    matchLabels:
      app: throttler
  template:
    metadata:
      labels:
        app: throttler
    spec:
      containers:
      - name: throttler
        image: throttler:latest
        ports:
        - containerPort: 8080
        envFrom:
        - configMapRef:
            name: throttler-config
        - secretRef:
            name: throttler-secrets
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: throttler-service
spec:
  selector:
    app: throttler
  ports:
  - port: 80
    targetPort: 8080
  type: ClusterIP
```

## Production Considerations

### High Availability

- Deploy multiple instances behind a load balancer
- Use Redis Cluster or Redis Sentinel for Redis HA
- Configure health checks in your orchestrator

### Security

- Use TLS for Redis connections in production
- Run containers as non-root user
- Use network policies to restrict access
- Rotate Redis passwords regularly
- Never expose Redis Commander in production

### Monitoring

- Scrape `/metrics` endpoint with Prometheus
- Set up alerts on health check failures
- Monitor Redis connection pool metrics
- Use Redis Commander only in development

### Performance Tuning

- Adjust `REDIS_MAX_CONNECTIONS` based on load
- Configure appropriate timeouts
- Monitor memory usage
- Use Redis pipelining for batch operations
