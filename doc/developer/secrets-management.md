# Secrets Management

Comprehensive guide for managing secrets and sensitive configuration in Mindia.

## Overview

Mindia requires several secrets for operation:
- **JWT_SECRET**: Token signing key
- **DATABASE_URL**: Database connection string with credentials
- **AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY**: S3 access credentials
- **API Keys**: External service API keys

## Principles

1. **Never commit secrets to version control**
2. **Use environment variables or secrets managers**
3. **Rotate secrets regularly**
4. **Use different secrets per environment**
5. **Limit secret access (principle of least privilege)**
6. **Audit secret access**

## Secret Generation

### JWT Secret

```bash
# Generate secure random secret (32+ characters)
openssl rand -hex 32

# Or using /dev/urandom
head -c 32 /dev/urandom | base64

# Or using Python
python3 -c "import secrets; print(secrets.token_hex(32))"
```

### Database Password

```bash
# Generate strong password
openssl rand -base64 24

# Or using pwgen
pwgen -s 32 1
```

### API Keys

Generate API keys through your cloud provider's console or CLI:
- AWS: IAM user access keys
- Database: Database user creation
- External services: Service-specific key generation

## Storage Methods

### Environment Variables

**Development/Testing**

```bash
# .env file (gitignored)
JWT_SECRET=your-secret-here
DATABASE_URL=postgresql://user:pass@localhost/mindia
```

**Production**

Use environment variables set by your deployment platform:
- Docker: `docker run -e JWT_SECRET=...`
- Kubernetes: ConfigMaps/Secrets
- Fly.io: `flyctl secrets set`
- Systemd: Environment files

### AWS Secrets Manager

```bash
# Store secret
aws secretsmanager create-secret \
  --name mindia/jwt-secret \
  --secret-string "your-secret-here"

# Retrieve in application
aws secretsmanager get-secret-value \
  --secret-id mindia/jwt-secret \
  --query SecretString --output text
```

**Integration Example**

```rust
// Load from AWS Secrets Manager
let jwt_secret = load_from_secrets_manager("mindia/jwt-secret").await?;
```

### HashiCorp Vault

```bash
# Store secret
vault kv put secret/mindia jwt_secret="your-secret-here"

# Retrieve
vault kv get -field=jwt_secret secret/mindia
```

### Kubernetes Secrets

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: mindia-secrets
type: Opaque
stringData:
  jwt-secret: your-secret-here
  database-url: postgresql://...
```

Reference in deployment:

```yaml
env:
  - name: JWT_SECRET
    valueFrom:
      secretKeyRef:
        name: mindia-secrets
        key: jwt-secret
```

### Docker Secrets

```bash
# Create secret
echo "your-secret-here" | docker secret create jwt_secret -

# Use in service
docker service create \
  --secret jwt_secret \
  --env JWT_SECRET_FILE=/run/secrets/jwt_secret \
  mindia:latest
```

## Secret Rotation

### JWT Secret Rotation

**Strategy**: Support multiple secrets during rotation period.

1. **Add new secret** to environment
2. **Update application** to accept both old and new secrets
3. **Wait for token expiry** (all tokens using old secret expire)
4. **Remove old secret** from environment

**Implementation**:

```rust
// Support multiple JWT secrets during rotation
let secrets = vec![
    env::var("JWT_SECRET").unwrap(),
    env::var("JWT_SECRET_OLD").ok(),
].into_iter().flatten().collect();
```

### Database Password Rotation

1. **Create new database user** with new password
2. **Update DATABASE_URL** in secrets manager
3. **Restart application** (will use new credentials)
4. **Verify connectivity**
5. **Remove old user** after verification period

### AWS Credentials Rotation

1. **Create new IAM user** with S3 permissions
2. **Generate new access keys**
3. **Update AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY**
4. **Restart application**
5. **Verify S3 operations**
6. **Delete old access keys** after verification

## Best Practices

### Development

- Use `.env` file (gitignored)
- Never commit `.env` to version control
- Use different secrets than production
- Rotate secrets periodically

### Staging

- Use secrets manager or environment variables
- Use separate secrets from production
- Test secret rotation procedures
- Monitor secret access

### Production

- **Always use secrets manager** (AWS Secrets Manager, Vault, etc.)
- Use IAM roles when possible (instead of access keys)
- Enable secret versioning
- Audit secret access
- Rotate secrets regularly (quarterly recommended)
- Use different secrets per environment

## Security Considerations

### Access Control

- Limit who can access secrets
- Use IAM roles/policies
- Enable audit logging
- Review access regularly

### Encryption

- Encrypt secrets at rest
- Use TLS for secret transmission
- Enable encryption in secrets manager
- Use encrypted environment variables when possible

### Monitoring

- Monitor secret access
- Alert on unusual access patterns
- Log secret rotation events
- Track secret age

## Secret Validation

Mindia validates secrets on startup:

- **JWT_SECRET**: Must be at least 32 characters
- **DATABASE_URL**: Must be valid PostgreSQL connection string
- **AWS credentials**: Validated on first S3 operation

## Emergency Procedures

### Compromised Secret

1. **Immediately rotate** the compromised secret
2. **Revoke** old secret/credentials
3. **Restart** application with new secret
4. **Monitor** for unauthorized access
5. **Audit** access logs
6. **Notify** security team

### Secret Loss

1. **Regenerate** secret using secure method
2. **Update** in secrets manager
3. **Restart** application
4. **Verify** functionality
5. **Update** documentation

## Tools

- **AWS Secrets Manager**: Managed secrets service
- **HashiCorp Vault**: Self-hosted secrets manager
- **Kubernetes Secrets**: Native K8s secret management
- **Docker Secrets**: Docker Swarm secret management
- **1Password Secrets Automation**: Enterprise secrets management

## Checklist

### Initial Setup

- [ ] Generate secure secrets
- [ ] Store in secrets manager
- [ ] Configure application access
- [ ] Test secret retrieval
- [ ] Document secret locations
- [ ] Set up rotation schedule

### Regular Maintenance

- [ ] Review secret access logs
- [ ] Rotate secrets quarterly
- [ ] Update documentation
- [ ] Test rotation procedures
- [ ] Audit secret permissions

## Next Steps

- [ ] Choose secrets management solution
- [ ] Migrate existing secrets
- [ ] Set up rotation schedule
- [ ] Configure monitoring
- [ ] Document procedures

