# Monitoring and Observability

This guide covers monitoring, observability, and alerting for the throttler service.

## Metrics Overview

The throttler service exposes Prometheus-compatible metrics at the `/metrics` endpoint.

### Core Metrics

#### Request Metrics
- `throttler_requests_total` - Total number of requests processed
  - Labels: `status` (allowed/denied), `client_id`, `endpoint`
- `throttler_request_duration_seconds` - Request processing time histogram
  - Labels: `endpoint`, `status`

#### Rate Limiting Metrics
- `throttler_rate_limit_hits_total` - Total rate limit violations
  - Labels: `limit_type` (global/per_client), `client_id`
- `throttler_token_bucket_size` - Current token bucket sizes
  - Labels: `bucket_id`, `client_id`
- `throttler_token_bucket_refill_rate` - Token refill rates
  - Labels: `bucket_id`

#### System Metrics
- `throttler_redis_operations_total` - Redis operation count
  - Labels: `operation` (get/set/del), `status` (success/error)
- `throttler_redis_connection_pool_size` - Active Redis connections
- `throttler_memory_usage_bytes` - Service memory usage
- `throttler_active_connections` - Current active HTTP connections

### Health Check Metrics
- `throttler_health_check_status` - Health check results
  - Labels: `component` (redis/service), `status` (healthy/unhealthy)

## Monitoring Setup

### Prometheus Configuration

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

### Grafana Dashboard

Recommended dashboard panels:

1. **Request Rate Panel**
   ```promql
   rate(throttler_requests_total[5m])
   ```

2. **Rate Limit Violations**
   ```promql
   increase(throttler_rate_limit_hits_total[1m])
   ```

3. **Response Time Percentiles**
   ```promql
   histogram_quantile(0.95, rate(throttler_request_duration_seconds_bucket[5m]))
   histogram_quantile(0.50, rate(throttler_request_duration_seconds_bucket[5m]))
   ```

4. **Redis Operations**
   ```promql
   rate(throttler_redis_operations_total[5m])
   ```

5. **Service Health**
   ```promql
   throttler_health_check_status
   ```

## Alerting Rules

### Critical Alerts

```yaml
# Service Down
- alert: ThrottlerServiceDown
  expr: up{job="throttler"} == 0
  for: 1m
  labels:
    severity: critical
  annotations:
    summary: "Throttler service is down"
    description: "Throttler service has been down for more than 1 minute"

# High Error Rate
- alert: ThrottlerHighErrorRate
  expr: |
    (
      rate(throttler_requests_total{status!="allowed"}[5m]) /
      rate(throttler_requests_total[5m])
    ) > 0.1
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "High error rate in throttler service"
    description: "Error rate is {{ $value | humanizePercentage }} for 5 minutes"

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

# Memory Usage
- alert: ThrottlerHighMemoryUsage
  expr: throttler_memory_usage_bytes > 1e9  # 1GB
  for: 15m
  labels:
    severity: warning
  annotations:
    summary: "High memory usage in throttler"
    description: "Memory usage: {{ $value | humanizeBytes }}"
```

## Log Analysis

### Log Levels and Structure

The service uses structured JSON logging:

```json
{
  "timestamp": "2024-01-15T10:30:45Z",
  "level": "INFO",
  "message": "Request processed",
  "client_id": "client123",
  "endpoint": "/api/check",
  "duration_ms": 23,
  "status": "allowed"
}
```

### Key Log Patterns

#### Rate Limit Violations
```bash
# Search for rate limit denials
grep '"status":"denied"' throttler.log | jq .

# Count violations by client
grep '"status":"denied"' throttler.log | jq -r .client_id | sort | uniq -c
```

#### Performance Issues
```bash
# Find slow requests (>1000ms)
grep '"duration_ms"' throttler.log | jq 'select(.duration_ms > 1000)'

# Average response time by endpoint
grep '"duration_ms"' throttler.log | jq -r '[.endpoint, .duration_ms] | @csv'
```

#### Error Analysis
```bash
# Redis connection errors
grep -i "redis.*error" throttler.log

# Configuration errors
grep '"level":"ERROR"' throttler.log | jq 'select(.message | contains("config"))'
```

## Performance Tuning

### Key Performance Indicators

1. **Request Throughput**: Requests per second handled
2. **Response Time**: P50, P95, P99 latencies
3. **Memory Usage**: Steady-state and peak memory consumption
4. **Redis Performance**: Operation latency and error rates

### Optimization Guidelines

#### Memory Optimization
- Monitor token bucket memory usage
- Tune Redis connection pool size
- Set appropriate cleanup intervals

#### Network Optimization
- Use Redis pipelining for batch operations
- Optimize serialization format
- Configure appropriate timeouts

#### Capacity Planning
- Monitor CPU and memory trends
- Plan for traffic spikes
- Set up horizontal scaling triggers

## Troubleshooting Common Issues

### Service Startup Issues
```bash
# Check configuration validation
docker logs throttler-service | grep -i "config"

# Verify Redis connectivity
docker logs throttler-service | grep -i "redis"
```

### High Latency Investigation
1. Check Redis response times
2. Monitor garbage collection metrics
3. Analyze request distribution patterns
4. Review connection pool utilization

### Memory Leaks
1. Monitor heap growth over time
2. Check for uncleaned token buckets
3. Analyze Redis connection lifecycle
4. Review cleanup job effectiveness

## Production Checklist

- [ ] Prometheus scraping configured
- [ ] Grafana dashboards deployed
- [ ] Critical alerts configured
- [ ] Log aggregation setup
- [ ] Performance baselines established
- [ ] Runbook documentation complete
- [ ] On-call procedures defined
- [ ] Capacity planning completed
