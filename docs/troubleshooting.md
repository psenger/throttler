# Troubleshooting Guide

This guide covers common issues and their solutions when running the throttler service.

## Common Issues

### Redis Connection Issues

#### Problem: Redis connection refused
```
Error: Failed to connect to Redis: Connection refused (os error 61)
```

**Solutions:**
1. Ensure Redis is running:
   ```bash
   redis-cli ping
   # Should return PONG
   ```

2. Check Redis configuration in `.env`:
   ```env
   REDIS_URL=redis://localhost:6379
   ```

3. Verify Redis is accessible:
   ```bash
   telnet localhost 6379
   ```

4. Check firewall settings and port availability

#### Problem: Redis authentication failed
```
Error: Redis authentication failed: WRONGPASS invalid username-password pair
```

**Solution:**
Update your Redis URL with correct credentials:
```env
REDIS_URL=redis://username:password@localhost:6379
```

### Rate Limiting Issues

#### Problem: Rate limits not working as expected

**Diagnostics:**
1. Check rate limit configuration:
   ```bash
   curl http://localhost:8080/api/v1/config
   ```

2. Verify token bucket parameters:
   - `capacity`: Maximum tokens in bucket
   - `refill_rate`: Tokens added per second
   - `window_seconds`: Time window for rate limiting

3. Test with known request patterns:
   ```bash
   # Send 10 requests quickly
   for i in {1..10}; do
     curl -w "%{http_code}\n" -o /dev/null -s http://localhost:8080/throttle/test-key
   done
   ```

#### Problem: Inconsistent rate limiting across instances

**Solution:**
Ensure all instances use the same Redis instance and key generation strategy:
1. Verify `REDIS_URL` is identical across instances
2. Check `KEY_STRATEGY` configuration
3. Monitor Redis for key conflicts

### Performance Issues

#### Problem: High latency on throttle requests

**Diagnostics:**
1. Check Redis latency:
   ```bash
   redis-cli --latency-history
   ```

2. Monitor application metrics:
   ```bash
   curl http://localhost:8080/health/metrics
   ```

3. Profile request handling:
   - Enable debug logging: `RUST_LOG=debug`
   - Monitor connection pool usage

**Solutions:**
1. Tune Redis connection pool:
   ```env
   REDIS_POOL_SIZE=20
   REDIS_CONNECTION_TIMEOUT=5000
   ```

2. Optimize token bucket parameters:
   - Reduce `window_seconds` for faster refills
   - Increase `capacity` to reduce Redis calls

3. Use Redis clustering for high load scenarios

#### Problem: Memory usage growing over time

**Solution:**
1. Set Redis key expiration:
   ```env
   REDIS_KEY_TTL=3600  # 1 hour
   ```

2. Monitor Redis memory usage:
   ```bash
   redis-cli info memory
   ```

3. Implement periodic cleanup of expired keys

### Configuration Issues

#### Problem: Invalid configuration values
```
Error: Configuration validation failed: capacity must be positive
```

**Solution:**
Validate configuration before starting:
```bash
# Check current config
curl http://localhost:8080/api/v1/config

# Update with valid values
curl -X PUT http://localhost:8080/api/v1/config \
  -H "Content-Type: application/json" \
  -d '{
    "capacity": 100,
    "refill_rate": 10.0,
    "window_seconds": 60
  }'
```

#### Problem: Environment variables not loaded

**Solution:**
1. Verify `.env` file location (project root)
2. Check file permissions: `chmod 644 .env`
3. Use absolute paths for environment files
4. Restart service after changes

### API Issues

#### Problem: 404 Not Found on API endpoints

**Solution:**
Verify correct API paths:
- Throttle check: `POST /throttle/{key}`
- Configuration: `GET/PUT /api/v1/config`
- Health check: `GET /health`
- Metrics: `GET /health/metrics`

#### Problem: Invalid JSON in requests
```
Error: Failed to parse JSON: expected value at line 1 column 1
```

**Solution:**
Validate JSON payload:
```bash
# Valid configuration update
curl -X PUT http://localhost:8080/api/v1/config \
  -H "Content-Type: application/json" \
  -d '{
    "capacity": 100,
    "refill_rate": 10.0,
    "window_seconds": 60
  }'
```

## Debugging Tips

### Enable Debug Logging
```bash
RUST_LOG=debug cargo run
# or
RUST_LOG=throttler=debug cargo run
```

### Check Service Status
```bash
# Health check
curl http://localhost:8080/health

# Detailed metrics
curl http://localhost:8080/health/metrics

# Configuration status
curl http://localhost:8080/api/v1/config
```

### Redis Debugging
```bash
# Monitor Redis commands
redis-cli monitor

# Check key patterns
redis-cli keys "throttle:*"

# Inspect specific key
redis-cli get "throttle:user:123"
```

### Load Testing
```bash
# Simple load test
ab -n 1000 -c 10 http://localhost:8080/throttle/test-key

# With custom headers
ab -n 1000 -c 10 -H "X-API-Key: test" http://localhost:8080/throttle/api-test
```

## Getting Help

If you encounter issues not covered in this guide:

1. Check the [GitHub Issues](https://github.com/your-org/throttler/issues)
2. Enable debug logging and collect relevant log output
3. Include your configuration (sanitized) when reporting issues
4. Provide steps to reproduce the problem

## Common Configuration Examples

### High-throughput API
```env
REDIS_POOL_SIZE=50
THROTTLE_CAPACITY=1000
THROTTLE_REFILL_RATE=100.0
THROTTLE_WINDOW_SECONDS=60
```

### Strict rate limiting
```env
THROTTLE_CAPACITY=10
THROTTLE_REFILL_RATE=1.0
THROTTLE_WINDOW_SECONDS=60
```

### Development/testing
```env
RUST_LOG=debug
REDIS_URL=redis://localhost:6379
THROTTLE_CAPACITY=100
THROTTLE_REFILL_RATE=50.0
```