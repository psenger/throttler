# Monitoring and Observability

Complete guide to monitoring, observability, and alerting for the Throttler service.

## Table of Contents

- [Health Endpoints](#health-endpoints)
- [Redis Commander](#redis-commander)
- [Prometheus Metrics](#prometheus-metrics)
- [Grafana Dashboards](#grafana-dashboards)
- [Alerting Rules](#alerting-rules)
- [Logging](#logging)
- [Docker Monitoring](#docker-compose-monitoring)
- [Performance Tuning](#performance-tuning)

---

## Health Endpoints

### GET /health

Liveness probe - returns service health status.

```bash
curl http://localhost:8080/health
```

```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "version": "0.1.0"
}
```

### GET /ready

Readiness probe - checks Redis connectivity.

```bash
curl http://localhost:8080/ready
```

```json
{
  "status": "ready",
  "redis": "connected"
}
```

## Redis Commander

For local development, Redis Commander provides a web UI to inspect Redis state:

```bash
# Start with Docker Compose
docker compose up -d

# Open in browser
open http://localhost:8081
```

Features:
- View all rate limit keys (`throttler:*`)
- Inspect token bucket state
- Monitor key TTLs
- Execute Redis commands
- Real-time key updates

## Metrics Overview

The throttler service exposes Prometheus-compatible metrics at the `/metrics` endpoint.

### Core Metrics

#### Request Metrics
- `throttler_requests_total` - Total number of requests processed
  - Labels: `status` (allowed/denied), `key`
- `throttler_request_duration_seconds` - Request processing time histogram
  - Labels: `endpoint`, `status`

#### Rate Limiting Metrics
- `throttler_rate_limit_hits_total` - Total rate limit violations
  - Labels: `key`
- `throttler_tokens_remaining` - Current available tokens per key
  - Labels: `key`

#### System Metrics
- `throttler_redis_operations_total` - Redis operation count
  - Labels: `operation` (get/set/del), `status` (success/error)
- `throttler_redis_latency_seconds` - Redis operation latency
- `throttler_active_connections` - Current active HTTP connections

## Prometheus Configuration

Add the following job to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'throttler'
    static_configs:
      - targets: ['localhost:8080']
    scrape_interval: 15s
    metrics_path: /metrics
    scrape_timeout: 10s
```

## Grafana Dashboards

### Request Rate Panel

```promql
rate(throttler_requests_total[5m])
```

### Rate Limit Violations

```promql
increase(throttler_rate_limit_hits_total[1m])
```

### Response Time Percentiles

```promql
histogram_quantile(0.95, rate(throttler_request_duration_seconds_bucket[5m]))
histogram_quantile(0.50, rate(throttler_request_duration_seconds_bucket[5m]))
```

### Redis Operations

```promql
rate(throttler_redis_operations_total[5m])
```

### Redis Latency

```promql
histogram_quantile(0.99, rate(throttler_redis_latency_seconds_bucket[5m]))
```

## Alerting Rules

### Critical Alerts

```yaml
groups:
  - name: throttler-critical
    rules:
      # Service Down
      - alert: ThrottlerServiceDown
        expr: up{job="throttler"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Throttler service is down"
          description: "Throttler has been down for more than 1 minute"

      # Redis Connection Issues
      - alert: ThrottlerRedisConnectionFailure
        expr: |
          rate(throttler_redis_operations_total{status="error"}[5m]) > 0.1
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Redis connection issues in throttler"
          description: "Redis error rate: {{ $value }} operations/sec"
```

### Warning Alerts

```yaml
groups:
  - name: throttler-warnings
    rules:
      # High Response Time
      - alert: ThrottlerHighLatency
        expr: |
          histogram_quantile(0.95,
            rate(throttler_request_duration_seconds_bucket[5m])
          ) > 0.5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High latency in throttler service"
          description: "95th percentile latency is {{ $value }}s"

      # High Rate Limit Violations
      - alert: ThrottlerHighRateLimitViolations
        expr: |
          rate(throttler_rate_limit_hits_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High rate limit violations"
          description: "Rate limit violations: {{ $value }} per second"
```

## Logging

### Log Configuration

Set log level via environment variable:

```bash
# Available levels: error, warn, info, debug, trace
RUST_LOG=info cargo run

# Debug logging for throttler only
RUST_LOG=throttler=debug cargo run

# Verbose logging including dependencies
RUST_LOG=debug cargo run
```

### Log Format

The service uses structured JSON logging:

```json
{
  "timestamp": "2024-01-15T10:30:45Z",
  "level": "INFO",
  "target": "throttler::handlers",
  "message": "Rate limit check",
  "key": "api-key-123",
  "allowed": true,
  "remaining": 99,
  "duration_ms": 2
}
```

### Log Analysis

#### Rate Limit Violations

```bash
# Search for rate limit denials
grep '"allowed":false' throttler.log | jq .

# Count violations by key
grep '"allowed":false' throttler.log | jq -r .key | sort | uniq -c | sort -rn
```

#### Performance Issues

```bash
# Find slow requests (>100ms)
grep 'duration_ms' throttler.log | jq 'select(.duration_ms > 100)'

# Average response time
grep 'duration_ms' throttler.log | jq '.duration_ms' | awk '{sum+=$1; count++} END {print sum/count}'
```

#### Redis Errors

```bash
# Find Redis connection errors
grep -i "redis.*error" throttler.log

# Redis operation failures
grep '"status":"error"' throttler.log | jq .
```

## Docker Compose Monitoring

### View Container Logs

```bash
# All services
docker compose logs -f

# Redis only
docker compose logs -f redis

# Last 100 lines
docker compose logs --tail=100
```

### Container Health

```bash
# Check container status
docker compose ps

# Detailed container info
docker inspect throttler-redis
```

### Redis Monitoring

```bash
# Connect to Redis CLI
docker exec -it throttler-redis redis-cli -a $DOCKER_REDIS_PASSWORD

# Real-time command monitoring
MONITOR

# Memory usage
INFO memory

# Connected clients
INFO clients

# Key statistics
INFO keyspace
```

## Performance Tuning

### Key Performance Indicators

1. **Request Throughput**: Requests per second handled
2. **Response Time**: P50, P95, P99 latencies
3. **Redis Latency**: Operation latency distribution
4. **Error Rate**: Failed requests percentage

### Optimization Guidelines

#### Redis Connection Pool

```env
# Increase for high-load scenarios
REDIS_MAX_CONNECTIONS=50
REDIS_CONNECTION_TIMEOUT=5
```

#### Memory Management

Monitor Redis memory usage:

```bash
docker exec -it throttler-redis redis-cli -a $DOCKER_REDIS_PASSWORD INFO memory
```

Redis configuration (`docker/redis/redis.conf`):
- `maxmemory 256mb` - Memory limit
- `maxmemory-policy allkeys-lru` - Eviction policy

## Production Checklist

- [ ] Health endpoints accessible
- [ ] Prometheus scraping configured
- [ ] Grafana dashboards deployed
- [ ] Critical alerts configured
- [ ] Log aggregation set up
- [ ] Redis Commander disabled (dev only)
- [ ] Performance baselines established
- [ ] Runbook documentation complete
