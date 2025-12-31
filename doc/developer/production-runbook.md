# Production Deployment Runbook

Step-by-step guide for deploying Mindia to production with checklists and verification procedures.

## Pre-Deployment Checklist

### Environment Preparation

- [ ] All environment variables configured and verified
- [ ] Database migrations tested in staging
- [ ] Database backup created
- [ ] Secrets rotated and stored securely
- [ ] CORS origins configured (no wildcards)
- [ ] Health checks verified
- [ ] Monitoring and alerting configured
- [ ] Rollback plan documented

### Code Verification

- [ ] All tests passing
- [ ] Security audit completed
- [ ] Code review approved
- [ ] Version tagged in Git
- [ ] Changelog updated
- [ ] Documentation updated

### Infrastructure

- [ ] Database connection pool limits verified
- [ ] Storage backend accessible and configured
- [ ] CDN configured (if applicable)
- [ ] Load balancer configured
- [ ] SSL/TLS certificates valid
- [ ] Firewall rules configured

## Deployment Process

### 1. Pre-Deployment Verification

```bash
# Verify environment
make validate-env

# Run health checks on current deployment
curl https://api.example.com/health

# Verify database connectivity
psql $DATABASE_URL -c "SELECT 1"
```

### 2. Create Backup

```bash
# Database backup with pg_dump
pg_dump "$DATABASE_URL" --format=custom --file="backup-$(date +%Y%m%d-%H%M%S).dump"

# Verify backup
ls -lh *.dump
```

### 3. Deploy Application

#### Option A: Docker Deployment

```bash
# Build image
docker build -t mindia:latest .

# Tag for registry
docker tag mindia:latest registry.example.com/mindia:v1.0.0

# Push to registry
docker push registry.example.com/mindia:v1.0.0

# Deploy (example with docker-compose)
docker-compose pull
docker-compose up -d --no-deps app
```

#### Option B: Fly.io Deployment

```bash
# Set secrets
flyctl secrets set JWT_SECRET="..." -a mindia
flyctl secrets set DATABASE_URL="..." -a mindia

# Deploy
flyctl deploy -a mindia
```

#### Option C: Kubernetes Deployment

See the [Deployment Guide](deployment.md) for Kubernetes deployment instructions.

```bash
# Example deployment commands
kubectl apply -f deployment.yaml

# Wait for rollout
kubectl rollout status deployment/mindia -n production

# Verify pods
kubectl get pods -n production
```

### 4. Post-Deployment Verification

#### Health Check

```bash
# Basic health check
curl https://api.example.com/health

# Expected response:
# {
#   "status": "healthy",
#   "database": "healthy",
#   "storage": "healthy",
#   ...
# }
```

#### Functional Tests

```bash
# Test authentication (use MASTER_API_KEY or a tenant API key)
export TOKEN="your-master-api-key-or-api-key"

# Test image upload
curl -X POST https://api.example.com/api/v0/images \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@test.jpg"

# Test image retrieval
curl https://api.example.com/api/v0/images/$IMAGE_ID \
  -H "Authorization: Bearer $TOKEN"
```

#### Performance Verification

```bash
# Check response times
curl -w "@curl-format.txt" -o /dev/null -s \
  https://api.example.com/health

# Monitor logs
tail -f /var/log/mindia/app.log | grep ERROR
```

### 5. Monitoring Verification

- [ ] Application logs showing no errors
- [ ] Metrics dashboard showing normal values
- [ ] Error rate within acceptable limits
- [ ] Response times within SLA
- [ ] Database connection pool healthy
- [ ] Storage operations successful

## Rollback Procedure

### Immediate Rollback (< 5 minutes)

```bash
# Docker
docker-compose rollback

# Kubernetes
kubectl rollout undo deployment/mindia -n production

# Fly.io
flyctl releases -a mindia
flyctl releases rollback <previous-release> -a mindia
```

### Database Rollback

```bash
# Restore from backup
./scripts/restore-database.sh backups/backup-$(date +%Y%m%d).sql

# Verify restoration
psql $DATABASE_URL -c "SELECT COUNT(*) FROM images"
```

### Post-Rollback Verification

- [ ] Health checks passing
- [ ] Application functional
- [ ] No data loss
- [ ] Monitoring shows recovery

## Emergency Procedures

### Service Unavailable

1. Check health endpoint: `curl https://api.example.com/health`
2. Check application logs for errors
3. Check database connectivity
4. Check storage backend connectivity
5. Restart service if needed
6. Escalate if issue persists

### Database Issues

1. Check database connection pool
2. Verify database is accessible
3. Check for long-running queries
4. Review database logs
5. Consider read replica if available
6. Restore from backup if data corruption

### Storage Issues

1. Verify storage backend connectivity
2. Check storage quota/limits
3. Verify credentials/permissions
4. Check storage service status
5. Switch to backup storage if available

### High Error Rate

1. Check application logs for patterns
2. Review recent deployments
3. Check external dependencies
4. Consider rate limiting
5. Scale up if resource-constrained
6. Rollback if recent deployment issue

## Post-Deployment Tasks

### Immediate (0-1 hour)

- [ ] Monitor error rates
- [ ] Verify all critical endpoints
- [ ] Check performance metrics
- [ ] Review application logs
- [ ] Notify team of deployment

### Short-term (1-24 hours)

- [ ] Monitor for 24 hours
- [ ] Review metrics and logs
- [ ] Gather user feedback
- [ ] Document any issues
- [ ] Update runbook if needed

### Long-term (1 week)

- [ ] Performance review
- [ ] Cost analysis
- [ ] User feedback analysis
- [ ] Plan next deployment
- [ ] Update documentation

## Communication

### Deployment Notifications

- Pre-deployment: Notify team 24 hours before
- During deployment: Update status channel
- Post-deployment: Confirm successful deployment
- Issues: Immediate notification to on-call

### Status Updates

Use a status page or communication channel to provide:
- Deployment start time
- Expected completion time
- Current status
- Any issues encountered
- Resolution steps

## Troubleshooting

### Common Issues

**Database Connection Errors**
- Verify DATABASE_URL is correct
- Check database is accessible
- Verify connection pool limits
- Check for network issues

**Storage Upload Failures**
- Verify storage credentials
- Check storage quota
- Verify network connectivity
- Check file size limits

**High Latency**
- Check database query performance
- Review application logs
- Check external API dependencies
- Consider scaling resources

**Memory Issues**
- Check memory usage
- Review for memory leaks
- Consider increasing resources
- Review image/video processing limits

## Success Criteria

Deployment is considered successful when:

- [ ] All health checks passing
- [ ] No critical errors in logs
- [ ] Response times within SLA
- [ ] All functional tests passing
- [ ] Monitoring shows normal operation
- [ ] No user-reported issues

## Next Steps

After successful deployment:

1. Update deployment documentation
2. Archive deployment logs
3. Schedule post-mortem if issues occurred
4. Plan next deployment
5. Update version numbers

