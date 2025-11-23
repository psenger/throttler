# Deployment Guide

This guide covers various deployment scenarios for the throttler service.

## Prerequisites

- Redis server (version 6.0 or later)
- Rust 1.70+ (for building from source)
- Docker (for containerized deployment)

## Configuration

### Environment Variables

Create a `.env` file based on `.env.example`:

```bash
# Server configuration
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# Redis configuration
REDIS_URL=redis://localhost:6379
REDIS_POOL_SIZE=10
REDIS_TIMEOUT=5000

# Default rate limiting
DEFAULT_CAPACITY=100
DEFAULT_REFILL_RATE=10
DEFAULT_REFILL_PERIOD=60

# Logging
RUST_LOG=throttler=info
```

## Local Development

### 1. Start Redis
```bash
# Using Docker
docker run -d --name redis -p 6379:6379 redis:7-alpine

# Or using local installation
redis-server
```

### 2. Run the service
```bash
# Clone and build
git clone <repository-url>
cd throttler
cargo build --release

# Run with environment file
cargo run --release
```

### 3. Verify deployment
```bash
# Health check
curl http://localhost:8080/health

# Test rate limiting
curl -X POST http://localhost:8080/check \
  -H "Content-Type: application/json" \
  -d '{"key": "test-client", "tokens": 1}'
```

## Docker Deployment

### Single Container

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

### Docker Compose

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
      - SERVER_HOST=0.0.0.0
      - SERVER_PORT=8080
    depends_on:
      - redis
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    restart: unless-stopped

volumes:
  redis_data:
```

```bash
# Deploy with Docker Compose
docker-compose up -d
```

## Kubernetes Deployment

### ConfigMap
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: throttler-config
data:
  SERVER_HOST: "0.0.0.0"
  SERVER_PORT: "8080"
  REDIS_URL: "redis://redis-service:6379"
  RUST_LOG: "throttler=info"
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
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
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
  type: LoadBalancer
```

## Production Considerations

### High Availability
- Deploy multiple instances behind a load balancer
- Use Redis Cluster or Redis Sentinel for Redis HA
- Configure appropriate health checks

### Monitoring
- Set up Prometheus scraping of `/metrics` endpoint
- Configure alerting on health check failures
- Monitor Redis connection pool metrics

### Security
- Use TLS for Redis connections in production
- Run container as non-root user
- Implement network policies in Kubernetes
- Regular security updates for base images

### Performance Tuning
- Adjust Redis connection pool size based on load
- Configure appropriate timeouts
- Monitor memory usage and GC metrics
- Use Redis pipelining for batch operations

### Backup and Recovery
- Regular Redis backups
- Configuration backup strategy
- Disaster recovery procedures

## Troubleshooting

### Common Issues

1. **Redis Connection Failures**
   - Check Redis server status
   - Verify connection string
   - Check network connectivity

2. **High Response Times**
   - Monitor Redis latency
   - Check connection pool exhaustion
   - Review system resource usage

3. **Memory Issues**
   - Monitor Redis memory usage
   - Check for memory leaks in application
   - Adjust Redis maxmemory settings

### Logs and Debugging

```bash
# Enable debug logging
export RUST_LOG=throttler=debug,redis=debug

# View application logs
docker logs throttler

# Monitor Redis
redis-cli monitor
```