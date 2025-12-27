# Troubleshooting Guide

Solutions for common issues when running the Throttler service.

## Table of Contents

- [Quick Diagnostics](#quick-diagnostics)
- [Docker Compose Issues](#docker-compose-issues)
- [Redis Connection Issues](#redis-connection-issues)
- [Rate Limiting Issues](#rate-limiting-issues)
- [Performance Issues](#performance-issues)
- [Configuration Issues](#configuration-issues)
- [Build Issues](#build-issues)
- [Debugging Tips](#debugging-tips)
- [Getting Help](#getting-help)

---

## Quick Diagnostics

```bash
# Check if service is running
curl http://localhost:8080/health

# Check Redis connectivity
curl http://localhost:8080/ready

# Check Docker containers
docker compose ps

# View service logs
docker compose logs -f
```

## Common Issues

### Docker Compose Issues

#### Problem: Containers won't start

```
Error: Cannot connect to the Docker daemon
```

**Solutions:**
1. Ensure Docker Desktop is running
2. Check Docker daemon status:
   ```bash
   docker info
   ```

#### Problem: Redis container unhealthy

```bash
docker compose ps
# Shows redis as "unhealthy"
```

**Solutions:**
1. Check Redis password is set in `.env`:
   ```bash
   grep DOCKER_REDIS_PASSWORD .env
   ```

2. View Redis logs:
   ```bash
   docker compose logs redis
   ```

3. Restart Redis:
   ```bash
   docker compose restart redis
   ```

#### Problem: Port already in use

```
Error: Bind for 0.0.0.0:6379 failed: port is already allocated
```

**Solutions:**
1. Find what's using the port:
   ```bash
   lsof -i :6379
   ```

2. Stop conflicting service or change port in `docker-compose.yml`

### Redis Connection Issues

#### Problem: Redis connection refused

```
Error: Failed to connect to Redis: Connection refused (os error 61)
```

**Solutions:**
1. Ensure Redis container is running:
   ```bash
   docker compose ps
   docker compose up -d redis
   ```

2. Check Redis is accessible:
   ```bash
   docker exec -it throttler-redis redis-cli -a $DOCKER_REDIS_PASSWORD ping
   # Should return PONG
   ```

3. Verify Redis URL in `.env`:
   ```env
   REDIS_URL=redis://:your_password@127.0.0.1:6379
   ```

#### Problem: Redis authentication failed

```
Error: WRONGPASS invalid username-password pair
```

**Solutions:**
1. Ensure password matches between `.env` and Docker:
   ```bash
   # Check .env
   grep DOCKER_REDIS_PASSWORD .env
   grep REDIS_URL .env

   # Password in REDIS_URL must match DOCKER_REDIS_PASSWORD
   ```

2. Restart containers after changing password:
   ```bash
   docker compose down
   docker compose up -d
   ```

#### Problem: Redis Commander can't connect

**Solutions:**
1. Ensure Redis is healthy first:
   ```bash
   docker compose ps
   ```

2. Check Redis Commander logs:
   ```bash
   docker compose logs redis-commander
   ```

3. Verify password in environment:
   ```bash
   docker compose config | grep REDIS
   ```

### Rate Limiting Issues

#### Problem: Rate limits not working as expected

**Diagnostics:**
1. Check current configuration:
   ```bash
   curl http://localhost:8080/rate-limit/your-key
   ```

2. Test with known pattern:
   ```bash
   # Send 10 requests quickly
   for i in {1..10}; do
     curl -s -o /dev/null -w "%{http_code}\n" \
       -X POST http://localhost:8080/rate-limit/test-key/check \
       -H "Content-Type: application/json" \
       -d '{}'
   done
   ```

3. Inspect Redis state using Redis Commander:
   ```
   http://localhost:8081
   ```

#### Problem: Inconsistent rate limiting across restarts

**Cause:** Redis data persists between restarts.

**Solutions:**
1. Clear specific key:
   ```bash
   curl -X DELETE http://localhost:8080/rate-limit/your-key
   ```

2. Clear all Redis data (development only):
   ```bash
   docker compose down -v
   docker compose up -d
   ```

### Performance Issues

#### Problem: High latency on requests

**Diagnostics:**
1. Check Redis latency:
   ```bash
   docker exec -it throttler-redis redis-cli -a $DOCKER_REDIS_PASSWORD \
     --latency-history
   ```

2. Enable debug logging:
   ```bash
   RUST_LOG=debug cargo run
   ```

3. Check container resources:
   ```bash
   docker stats
   ```

**Solutions:**
1. Increase Redis connection pool:
   ```env
   REDIS_MAX_CONNECTIONS=20
   ```

2. Check Redis memory usage:
   ```bash
   docker exec -it throttler-redis redis-cli -a $DOCKER_REDIS_PASSWORD \
     INFO memory
   ```

#### Problem: Memory usage growing

**Solutions:**
1. Check Redis memory:
   ```bash
   docker exec -it throttler-redis redis-cli -a $DOCKER_REDIS_PASSWORD \
     INFO memory
   ```

2. List all keys:
   ```bash
   docker exec -it throttler-redis redis-cli -a $DOCKER_REDIS_PASSWORD \
     KEYS "throttler:*" | wc -l
   ```

3. Redis is configured with LRU eviction (`maxmemory-policy allkeys-lru`), so old keys are automatically evicted.

### Configuration Issues

#### Problem: Environment variables not loaded

**Solutions:**
1. Verify `.env` file exists:
   ```bash
   ls -la .env
   ```

2. Check file permissions:
   ```bash
   chmod 644 .env
   ```

3. Source the file manually for testing:
   ```bash
   source .env
   echo $REDIS_URL
   ```

#### Problem: Invalid configuration values

```
Error: Configuration validation failed
```

**Solutions:**
1. Check all required variables are set:
   ```bash
   cat .env.example
   diff .env.example .env
   ```

2. Validate Redis URL format:
   ```env
   # Correct format with password
   REDIS_URL=redis://:password@127.0.0.1:6379

   # Without password (not recommended)
   REDIS_URL=redis://127.0.0.1:6379
   ```

### Build Issues

#### Problem: Cargo build fails

**Solutions:**
1. Update Rust:
   ```bash
   rustup update
   ```

2. Clean and rebuild:
   ```bash
   cargo clean
   cargo build
   ```

3. Check Rust version (requires 1.70+):
   ```bash
   rustc --version
   ```

## Debugging Tips

### Enable Debug Logging

```bash
# All debug logs
RUST_LOG=debug cargo run

# Throttler only
RUST_LOG=throttler=debug cargo run

# Include Redis operations
RUST_LOG=throttler=debug,redis=debug cargo run
```

### Inspect Redis State

```bash
# Connect to Redis
docker exec -it throttler-redis redis-cli -a $DOCKER_REDIS_PASSWORD

# List all throttler keys
KEYS throttler:*

# Get key value
GET throttler:rate_limit:your-key

# Monitor all commands in real-time
MONITOR

# Check key TTL
TTL throttler:rate_limit:your-key
```

### Test Endpoints

```bash
# Health check
curl -v http://localhost:8080/health

# Readiness (includes Redis check)
curl -v http://localhost:8080/ready

# Rate limit check with verbose output
curl -v -X POST http://localhost:8080/rate-limit/test/check \
  -H "Content-Type: application/json" \
  -d '{}'
```

## Getting Help

If you encounter issues not covered here:

1. Check container logs:
   ```bash
   docker compose logs
   ```

2. Enable debug logging and reproduce the issue

3. Check GitHub Issues: https://github.com/your-org/throttler/issues

4. Include in bug reports:
   - Error message
   - Steps to reproduce
   - Environment (OS, Rust version, Docker version)
   - Relevant logs (sanitized)
