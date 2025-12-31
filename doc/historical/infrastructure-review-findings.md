# Infrastructure Review Findings Report

**Date**: 2024  
**Reviewer**: Infrastructure Audit  
**Scope**: AWS Infrastructure (Terraform) + Global Infrastructure Code (mindia-infra crate)

---

## Executive Summary

This comprehensive review examined AWS infrastructure configurations and global infrastructure code across the Mindia project. The review identified **23 findings** across security, scalability, cost optimization, and best practices categories.

### Findings Summary

- **Critical Issues**: 3
- **High Priority**: 7
- **Medium Priority**: 8
- **Low Priority**: 5

### Key Recommendations

1. **Security**: Restrict CORS origins, implement RDS Proxy, move EC2 to private subnets
2. **Scalability**: Add RDS Proxy, implement Target Tracking scaling policies, document connection pool calculations
3. **Cost**: Reduce NAT Gateway costs with VPC endpoints, add incomplete multipart upload cleanup
4. **High Availability**: Fix EC2 terraform to use 3 AZs, add NAT Gateways for private subnet access

---

## 1. Security Findings

### 1.1 CRITICAL: CORS Allows All Origins

**Severity**: Critical  
**Location**: 
- `deployment/terraform/s3.tf:47`
- `deployment/ec2/terraform/s3.tf:47`
- `deployment/terraform/cloudfront.tf:118`
- `deployment/ec2/terraform/cloudfront.tf:109`
- `deployment/terraform/secrets.tf:59`
- `deployment/ec2/terraform/secrets.tf:58`

**Issue**: 
- S3 CORS configuration allows `allowed_origins = ["*"]`
- CloudFront response headers policy allows `items = ["*"]`
- Secrets Manager default CORS_ORIGINS is `"*"`

**Risk**: 
- Allows any origin to make cross-origin requests
- Potential for CSRF attacks
- Data exfiltration risk

**Recommendation**:
```hcl
# In s3.tf
cors_rule {
  allowed_origins = var.cors_allowed_origins  # Use specific origins
  # ...
}

# In cloudfront.tf
cors_config {
  access_control_allow_origins {
    items = var.cors_allowed_origins  # List of specific origins
  }
}

# In secrets.tf
CORS_ORIGINS = join(",", var.cors_allowed_origins)
```

**Action**: Add `cors_allowed_origins` variable with default empty list, require explicit configuration in production.

---

### 1.2 HIGH: ECR IAM Policy Uses Wildcard Resource

**Severity**: High  
**Location**: `deployment/ec2/terraform/iam.tf:86`

**Issue**: 
```hcl
Resource = "*"  # Allows access to all ECR repositories
```

**Risk**: 
- Violates principle of least privilege
- If IAM role is compromised, attacker can access all ECR repositories in account

**Recommendation**:
```hcl
Resource = [
  "arn:aws:ecr:${var.aws_region}:${data.aws_caller_identity.current.account_id}:repository/${var.project_name}",
  "arn:aws:ecr:${var.aws_region}:${data.aws_caller_identity.current.account_id}:repository/${var.project_name}/*"
]
```

**Action**: Restrict ECR policy to specific project repository.

---

### 1.3 HIGH: EC2 Instances in Public Subnets

**Severity**: High  
**Location**: `deployment/ec2/terraform/vpc.tf:22-32`, `deployment/ec2/terraform/ec2.tf:122`

**Issue**: 
- EC2 instances are deployed in public subnets
- Auto Scaling Groups use public subnets
- Instances have public IPs

**Risk**: 
- Direct exposure to internet
- Increased attack surface
- No defense-in-depth

**Recommendation**:
- Move EC2 instances to private subnets
- Add NAT Gateways (currently missing in EC2 terraform)
- Use ALB in public subnets, EC2 in private subnets
- Remove `map_public_ip_on_launch = true` from private subnets

**Action**: 
1. Add private subnets to EC2 terraform (similar to main terraform)
2. Add NAT Gateways for outbound internet access
3. Update Auto Scaling Groups to use private subnets
4. Verify ALB can reach instances in private subnets

---

### 1.4 MEDIUM: RDS Security Group Allows All Private Subnets

**Severity**: Medium  
**Location**: `deployment/terraform/rds.tf:22`

**Issue**: 
```hcl
cidr_blocks = aws_subnet.private[*].cidr_block
```

**Risk**: 
- Any resource in private subnets can access RDS
- No restriction to specific security groups
- If private subnet is compromised, RDS is accessible

**Recommendation**:
```hcl
# Use security group references instead
ingress {
  description     = "PostgreSQL from application security group"
  from_port       = 5432
  to_port         = 5432
  protocol        = "tcp"
  security_groups = [aws_security_group.ec2.id]
}
```

**Action**: Update RDS security group to reference EC2 security group instead of CIDR blocks.

---

### 1.5 MEDIUM: Secrets Manager Missing Rotation

**Severity**: Medium  
**Location**: `deployment/terraform/secrets.tf`, `deployment/ec2/terraform/secrets.tf`

**Issue**: 
- No automatic secret rotation configured
- RDS password stored in Secrets Manager but no rotation policy
- JWT_SECRET has placeholder value

**Risk**: 
- Long-lived secrets increase compromise risk
- Manual rotation is error-prone

**Recommendation**:
- Enable automatic rotation for RDS password using AWS managed rotation function
- Document manual rotation process for JWT_SECRET
- Add rotation schedule to documentation

**Action**: 
1. Add `aws_secretsmanager_secret_rotation` resource for RDS password
2. Create runbook for JWT_SECRET rotation
3. Add CloudWatch alarm for rotation failures

---

### 1.6 LOW: CloudWatch Log Group ARN Not Specific

**Severity**: Low  
**Location**: `deployment/ec2/terraform/iam.tf:109`

**Issue**: 
```hcl
Resource = [
  "${aws_cloudwatch_log_group.app.arn}:*"
]
```

**Current State**: This is actually correct - the `:*` suffix allows access to all log streams within the log group, which is necessary for CloudWatch Logs.

**Recommendation**: No change needed, but consider adding separate log groups for different log levels if needed.

---

## 2. Scalability Findings

### 2.1 CRITICAL: Missing RDS Proxy

**Severity**: Critical  
**Location**: `deployment/terraform/rds.tf`

**Issue**: 
- No RDS Proxy configured
- Direct connections from application to RDS
- Connection pool limits will be hit with multiple instances

**Impact**: 
- With 5 instances × 20 connections = 100 connections (at RDS limit for t3.medium)
- Cannot scale horizontally without connection exhaustion
- Connection pool exhaustion causes application failures

**Recommendation**:
```hcl
# Add RDS Proxy resource
resource "aws_db_proxy" "main" {
  name                   = "${var.project_name}-${var.environment}-proxy"
  engine_family          = "POSTGRESQL"
  auth {
    auth_scheme = "SECRETS"
    secret_arn  = aws_secretsmanager_secret.rds_password.arn
  }
  vpc_subnet_ids         = aws_subnet.private[*].id
  vpc_security_group_ids = [aws_security_group.rds_proxy.id]
  
  target {
    db_instance_identifier = aws_db_instance.main.id
  }
}
```

**Action**: 
1. Add RDS Proxy configuration
2. Update application to use proxy endpoint
3. Increase `DB_MAX_CONNECTIONS` per instance (proxy handles pooling)
4. Document connection pool sizing with proxy

---

### 2.2 HIGH: SimpleScaling Instead of Target Tracking

**Severity**: High  
**Location**: `deployment/ec2/terraform/ec2.tf:152-169`, `cloudwatch.tf:32-73`

**Issue**: 
- Using SimpleScaling policies triggered by CloudWatch alarms
- Manual threshold configuration
- Less responsive to actual load

**Impact**: 
- Slower scaling response
- Requires manual tuning of thresholds
- Less efficient resource utilization

**Recommendation**:
```hcl
# Replace SimpleScaling with Target Tracking
resource "aws_autoscaling_policy" "target_tracking" {
  name                   = "${var.project_name}-${var.environment}-target-tracking"
  autoscaling_group_name = aws_autoscaling_group.main.name
  policy_type            = "TargetTrackingScaling"

  target_tracking_configuration {
    predefined_metric_specification {
      predefined_metric_type = "ASGAverageCPUUtilization"
    }
    target_value = 70.0
  }
}
```

**Action**: 
1. Replace SimpleScaling policies with Target Tracking
2. Remove manual CloudWatch alarms for scaling (keep for monitoring)
3. Configure separate target tracking for control-plane and media-processor

---

### 2.3 HIGH: Missing Connection Pool Documentation

**Severity**: High  
**Location**: Documentation

**Issue**: 
- Connection pool sizing formula exists in docs but not enforced
- No validation in Terraform variables
- Risk of misconfiguration in production

**Impact**: 
- Teams may set `DB_MAX_CONNECTIONS=20` for all instances
- With 10 instances: 10 × 20 = 200 connections (exceeds RDS limit)
- Application failures under load

**Recommendation**:
1. Add Terraform variable validation:
```hcl
variable "db_max_connections" {
  validation {
    condition = var.db_max_connections <= 100
    error_message = "DB_MAX_CONNECTIONS must account for total instances. Use formula: DB_MAX_CONNECTIONS = RDS_MAX_CONNECTIONS / NUM_INSTANCES"
  }
}
```

2. Add output warning:
```hcl
output "connection_pool_warning" {
  value = "Total connections: ${var.db_max_connections * var.desired_capacity}. Ensure this is less than RDS max_connections."
}
```

**Action**: 
1. Add validation to Terraform variables
2. Update documentation with examples
3. Add pre-deployment checklist item

---

### 2.4 MEDIUM: No Database Connection Monitoring

**Severity**: Medium  
**Location**: `deployment/ec2/terraform/cloudwatch.tf`

**Issue**: 
- No CloudWatch alarms for database connection pool usage
- No metrics for connection wait time
- No alerting for connection exhaustion

**Impact**: 
- Connection issues go undetected until failures occur
- No proactive scaling based on connection pressure

**Recommendation**:
```hcl
# Add RDS connection metrics alarm
resource "aws_cloudwatch_metric_alarm" "rds_connections" {
  alarm_name          = "${var.project_name}-${var.environment}-rds-connections-high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "DatabaseConnections"
  namespace           = "AWS/RDS"
  period              = 300
  statistic           = "Average"
  threshold           = 80  # 80% of max connections
  alarm_description   = "Alert when RDS connections exceed 80%"
  
  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.id
  }
}
```

**Action**: Add CloudWatch alarms for RDS connection metrics.

---

## 3. High Availability Findings

### 3.1 CRITICAL: EC2 Terraform Missing Multi-AZ

**Severity**: Critical  
**Location**: `deployment/ec2/terraform/vpc.tf`

**Issue**: 
- Only 2 public subnets (2 AZs) vs 3 in main terraform
- No private subnets
- No NAT Gateways
- Single point of failure if one AZ goes down

**Impact**: 
- Reduced availability
- Cannot survive AZ failure
- Inconsistent with main terraform configuration

**Recommendation**:
```hcl
# Match main terraform: 3 AZs
resource "aws_subnet" "public" {
  count = 3  # Change from 2 to 3
  # ...
}

# Add private subnets
resource "aws_subnet" "private" {
  count = 3
  # ...
}

# Add NAT Gateways
resource "aws_nat_gateway" "main" {
  count = 3
  # ...
}
```

**Action**: 
1. Update EC2 terraform to use 3 AZs
2. Add private subnets
3. Add NAT Gateways
4. Update Auto Scaling Groups to span 3 AZs

---

### 3.2 HIGH: Missing Pod Disruption Budgets (K8s)

**Severity**: High (if using Kubernetes)  
**Location**: Kubernetes manifests (not in Terraform)

**Issue**: 
- AWS Kubernetes deployment docs mention PDBs but no manifests provided
- No protection against voluntary disruptions

**Impact**: 
- Rolling updates can take down all pods simultaneously
- No guarantee of minimum availability during updates

**Recommendation**: Add PDB manifests to deployment documentation or K8s configs.

---

### 3.3 MEDIUM: RDS Backup Retention Only 7 Days

**Severity**: Medium  
**Location**: `deployment/terraform/rds.tf:86`

**Issue**: 
```hcl
backup_retention_period = var.rds_backup_retention_period  # Default: 7 days
```

**Impact**: 
- Limited recovery window
- May not meet compliance requirements
- Insufficient for some disaster recovery scenarios

**Recommendation**: 
- Increase default to 30 days for production
- Add variable for environment-specific retention
- Consider cross-region backup replication

**Action**: Update default backup retention to 30 days, add environment-based configuration.

---

## 4. Cost Optimization Findings

### 4.1 HIGH: 3 NAT Gateways (High Cost)

**Severity**: High  
**Location**: `deployment/terraform/vpc.tf:60-70`

**Issue**: 
- 3 NAT Gateways (one per AZ) = ~$135/month
- All outbound traffic goes through NAT
- S3 and DynamoDB traffic doesn't need NAT

**Impact**: 
- Unnecessary costs for AWS service traffic
- ~$1,620/year for NAT Gateways

**Recommendation**:
```hcl
# Add VPC Endpoints for S3 and DynamoDB
resource "aws_vpc_endpoint" "s3" {
  vpc_id            = aws_vpc.main.id
  service_name      = "com.amazonaws.${var.aws_region}.s3"
  vpc_endpoint_type = "Gateway"
  route_table_ids   = aws_route_table.private[*].id
}

# Consider reducing to 1 NAT Gateway for non-AWS traffic
# Or use VPC endpoints for all AWS services
```

**Estimated Savings**: $1,200-1,500/year (reduce to 1 NAT Gateway + VPC endpoints)

**Action**: 
1. Add VPC endpoints for S3, DynamoDB, Secrets Manager
2. Evaluate reducing to 1 NAT Gateway (single AZ) vs keeping 3 for HA
3. Document trade-off between cost and availability

---

### 4.2 HIGH: Missing Incomplete Multipart Upload Cleanup

**Severity**: High  
**Location**: `deployment/terraform/s3.tf`, `deployment/ec2/terraform/s3.tf`

**Issue**: 
- No lifecycle rule for incomplete multipart uploads
- Failed uploads consume storage and incur costs
- Can accumulate over time

**Impact**: 
- Storage costs for incomplete uploads
- Potential for significant cost if uploads frequently fail

**Recommendation**:
```hcl
# Add to S3 lifecycle configuration
rule {
  id     = "abort-incomplete-multipart-upload"
  status = "Enabled"
  
  abort_incomplete_multipart_upload {
    days_after_initiation = 7
  }
}
```

**Action**: Add incomplete multipart upload cleanup rule to both S3 configurations.

---

### 4.3 MEDIUM: S3 Lifecycle Could Use Intelligent Tiering

**Severity**: Medium  
**Location**: `deployment/terraform/s3.tf:53-85`

**Issue**: 
- Fixed transitions (30 days to IA, 90 days to Glacier)
- No automatic optimization based on access patterns
- May transition rarely-accessed files too early or frequently-accessed files too late

**Impact**: 
- Suboptimal cost/performance balance
- Manual tuning required

**Recommendation**: Consider S3 Intelligent-Tiering for unpredictable access patterns:
```hcl
resource "aws_s3_bucket_intelligent_tiering_configuration" "main" {
  bucket = aws_s3_bucket.main.id
  name   = "EntireBucket"
  
  tiering {
    access_tier = "ARCHIVE_ACCESS"
    days        = 90
  }
  
  tiering {
    access_tier = "DEEP_ARCHIVE_ACCESS"
    days        = 180
  }
}
```

**Action**: Evaluate access patterns and consider Intelligent-Tiering for cost optimization.

---

### 4.4 LOW: CloudWatch Log Retention Only 7 Days

**Severity**: Low  
**Location**: `deployment/ec2/terraform/cloudwatch.tf:4, 14, 24`

**Issue**: 
```hcl
retention_in_days = 7
```

**Impact**: 
- Limited log history for troubleshooting
- May need longer retention for compliance/audit

**Recommendation**: 
- Increase to 30 days for production
- Consider 90 days for compliance requirements
- Use log export to S3 for long-term retention

**Action**: Make retention configurable via variable, default to 30 days for production.

---

## 5. Monitoring and Observability Findings

### 5.1 HIGH: Limited CloudWatch Alarms

**Severity**: High  
**Location**: `deployment/ec2/terraform/cloudwatch.tf`

**Issue**: 
- Only CPU-based alarms for auto-scaling
- Missing: memory, disk, network, database, S3 metrics
- No cost anomaly detection

**Impact**: 
- Issues go undetected
- No proactive alerting
- Cost overruns not detected early

**Recommendation**: Add alarms for:
- Memory utilization
- Disk space/IOPS
- Network throughput/errors
- RDS connection count, CPU, storage
- S3 request errors
- ALB 5xx errors
- Cost anomaly detection

**Action**: Create comprehensive alarm set covering all critical metrics.

---

### 5.2 MEDIUM: No CloudWatch Dashboard for RDS

**Severity**: Medium  
**Location**: `deployment/ec2/terraform/cloudwatch.tf:184-230`

**Issue**: 
- Dashboard only shows EC2 and ALB metrics
- No RDS metrics visualization
- No S3 metrics

**Impact**: 
- Limited visibility into database performance
- Harder to troubleshoot database issues

**Recommendation**: Add RDS and S3 widgets to dashboard.

---

### 5.3 LOW: OpenTelemetry Endpoint May Be Incorrect

**Severity**: Low  
**Location**: `deployment/terraform/secrets.tf:53`, `deployment/ec2/terraform/secrets.tf:52`

**Issue**: 
```hcl
OTEL_EXPORTER_OTLP_ENDPOINT = "https://logs.${var.aws_region}.amazonaws.com"
```

**Potential Issue**: This endpoint may not be correct for OpenTelemetry. Should be:
- AWS X-Ray: `https://xray.${var.aws_region}.amazonaws.com`
- Or custom OpenTelemetry Collector endpoint

**Action**: Verify and correct OpenTelemetry endpoint configuration.

---

## 6. Code Quality and Best Practices

### 6.1 MEDIUM: Missing Terraform Variable Validation

**Severity**: Medium  
**Location**: Variable files

**Issue**: 
- No validation on critical variables
- Can deploy with invalid configurations
- No warnings for common mistakes

**Recommendation**: Add validation blocks:
```hcl
variable "db_max_connections" {
  validation {
    condition     = var.db_max_connections > 0 && var.db_max_connections <= 100
    error_message = "DB_MAX_CONNECTIONS must be between 1 and 100"
  }
}

variable "s3_bucket_name" {
  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9-]*[a-z0-9]$", var.s3_bucket_name))
    error_message = "S3 bucket name must be valid (lowercase, alphanumeric, hyphens)"
  }
}
```

**Action**: Add validation to all critical variables.

---

### 6.2 MEDIUM: Infrastructure Code - Good Patterns

**Severity**: Medium (Positive Finding)  
**Location**: `mindia-infra/`

**Observations**:
- ✅ Good error handling with `anyhow::Result`
- ✅ Proper use of `Arc` for shared state
- ✅ Comprehensive tracing instrumentation
- ✅ Capacity checking with configurable behavior
- ✅ Rate limiting with token bucket algorithm

**Minor Improvements**:
- Consider adding metrics export for capacity checker
- Add unit tests for rate limiter
- Document expected behavior for capacity checks

---

### 6.3 LOW: Missing Terraform State Backend Configuration

**Severity**: Low  
**Location**: Terraform configurations

**Issue**: 
- No backend configuration in Terraform files
- State management not documented
- Risk of state file loss

**Recommendation**: 
- Add S3 backend configuration
- Add DynamoDB table for state locking
- Document state management procedures

---

## 7. Action Items Summary

### Critical (Immediate Action Required)

1. **Fix CORS Configuration** - Restrict to specific origins
2. **Add RDS Proxy** - Enable horizontal scaling
3. **Fix EC2 Terraform Multi-AZ** - Add 3 AZs, private subnets, NAT Gateways

### High Priority (Within 1 Week)

4. **Restrict ECR IAM Policy** - Remove wildcard resource
5. **Move EC2 to Private Subnets** - Improve security posture
6. **Implement Target Tracking Scaling** - Replace SimpleScaling
7. **Add VPC Endpoints** - Reduce NAT Gateway costs
8. **Add Incomplete Multipart Upload Cleanup** - Reduce S3 costs
9. **Add Comprehensive CloudWatch Alarms** - Improve observability

### Medium Priority (Within 1 Month)

10. **Update RDS Security Group** - Use security group references
11. **Add Secret Rotation** - Improve security
12. **Increase Backup Retention** - Better disaster recovery
13. **Add Connection Pool Monitoring** - Proactive scaling
14. **Add Variable Validation** - Prevent misconfigurations
15. **Add RDS Metrics to Dashboard** - Better visibility

### Low Priority (Backlog)

16. **Increase CloudWatch Log Retention** - Better troubleshooting
17. **Fix OpenTelemetry Endpoint** - Verify configuration
18. **Add Terraform Backend Configuration** - State management
19. **Consider S3 Intelligent Tiering** - Cost optimization
20. **Add Infrastructure Code Tests** - Code quality

---

## 8. Cost Impact Analysis

### Current Monthly Costs (Estimated)

| Service | Configuration | Monthly Cost |
|---------|--------------|--------------|
| NAT Gateways (3) | 3 × $45 | $135 |
| RDS (t3.medium Multi-AZ) | db.t3.medium | ~$150 |
| EC2 Instances | 2-10 × c5.large | $150-750 |
| S3 Storage | 1 TB | ~$23 |
| CloudFront | 1 TB transfer | ~$85 |
| ALB | 1 load balancer | ~$25 |
| **Total** | | **~$568-1,168** |

### Potential Savings

| Optimization | Current | Optimized | Monthly Savings |
|-------------|---------|-----------|----------------|
| NAT Gateways → VPC Endpoints | $135 | $10 | $125 |
| Incomplete Upload Cleanup | Variable | $0 | $5-20 |
| **Total Potential Savings** | | | **$130-145/month** |

**Annual Savings**: ~$1,560-1,740

---

## 9. Security Posture Summary

### Strengths

- ✅ S3 encryption enabled (AES256)
- ✅ S3 public access blocked
- ✅ RDS encryption enabled
- ✅ Multi-AZ RDS deployment
- ✅ IAM Instance Profiles (no hardcoded credentials)
- ✅ Security groups restrict ingress appropriately
- ✅ CloudFront with security headers
- ✅ Secrets Manager for sensitive data

### Weaknesses

- ❌ CORS allows all origins
- ❌ ECR policy too permissive
- ❌ EC2 in public subnets
- ❌ No secret rotation
- ❌ RDS security group uses CIDR blocks

### Security Score: 7/10

---

## 10. Scalability Assessment

### Current Limitations

1. **Database Connections**: Without RDS Proxy, limited to ~100 connections total
2. **Scaling Policies**: SimpleScaling less responsive than Target Tracking
3. **Multi-AZ**: EC2 terraform only uses 2 AZs

### Scaling Capacity

- **Current**: ~5-10 instances per service (with connection limits)
- **With RDS Proxy**: 20-50+ instances per service
- **Bottleneck**: Database connections (critical blocker)

### Scalability Score: 6/10

---

## 11. Recommendations Priority Matrix

| Priority | Security | Scalability | Cost | HA | Effort |
|----------|----------|------------|------|----|--------|
| Critical | CORS fix | RDS Proxy | - | Multi-AZ fix | Medium |
| High | ECR policy, Private subnets | Target Tracking | NAT/VPC endpoints | - | Low-Medium |
| Medium | Secret rotation, RDS SG | Connection monitoring | S3 cleanup | Backup retention | Low |
| Low | - | - | Intelligent Tiering | - | Low |

---

## 12. Conclusion

The infrastructure review identified significant opportunities for improvement in security, scalability, and cost optimization. The most critical issues are:

1. **CORS configuration** allowing all origins (security risk)
2. **Missing RDS Proxy** limiting horizontal scaling (scalability blocker)
3. **EC2 terraform missing multi-AZ** (availability risk)

Addressing the critical and high-priority items will significantly improve the infrastructure's security posture, scalability, and cost efficiency.

**Estimated Implementation Time**:
- Critical items: 2-3 days
- High priority items: 1 week
- Medium priority items: 2-3 weeks
- Total: ~1 month for all items

---

## Appendix: Files Reviewed

### Terraform Files
- `deployment/terraform/main.tf`
- `deployment/terraform/vpc.tf`
- `deployment/terraform/rds.tf`
- `deployment/terraform/s3.tf`
- `deployment/terraform/cloudfront.tf`
- `deployment/terraform/secrets.tf`
- `deployment/terraform/variables.tf`
- `deployment/ec2/terraform/main.tf`
- `deployment/ec2/terraform/vpc.tf`
- `deployment/ec2/terraform/ec2.tf`
- `deployment/ec2/terraform/iam.tf`
- `deployment/ec2/terraform/alb.tf`
- `deployment/ec2/terraform/s3.tf`
- `deployment/ec2/terraform/cloudfront.tf`
- `deployment/ec2/terraform/cloudwatch.tf`
- `deployment/ec2/terraform/secrets.tf`
- `deployment/ec2/terraform/sqs.tf`

### Infrastructure Code
- `mindia-infra/src/lib.rs`
- `mindia-infra/src/telemetry/mod.rs`
- `mindia-infra/src/capacity/checker.rs`
- `mindia-infra/src/webhook/service.rs`
- `mindia-infra/src/rate_limit/limiter.rs`
- `mindia-media-api/src/setup/database.rs`
- `mindia-control-plane/src/setup/database.rs`
- `mindia-media-processor/src/setup/database.rs`

### Documentation
- Kubernetes deployment documentation (removed - see deployment.md for current options)
- `doc/user/configuration.md`
- `doc/developer/deployment.md`

---

**Report Generated**: 2024  
**Next Review**: Recommended in 3 months or after major infrastructure changes
