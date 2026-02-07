# Deploy Mindia to AWS (Terraform + Ansible)

This guide describes how to deploy Mindia to AWS using Terraform for infrastructure and Ansible for app and ClamAV configuration. The architecture uses EC2 instances (no ECS/Docker), RDS PostgreSQL with pgvector, S3, Application Load Balancer, and CloudFront CDN.

## Prerequisites

- **AWS account** with credentials configured (e.g. `aws configure` or environment variables).
- **Terraform** >= 1.0 ([install](https://developer.hashicorp.com/terraform/downloads)).
- **Ansible** (for configuring EC2 instances; [install](https://docs.ansible.com/ansible/latest/installation_guide/index.html)).
- **AWS CLI** (for Secrets Manager and S3; [install](https://aws.amazon.com/cli/)).

## Architecture overview

- **VPC**: Public subnets (ALB, NAT) and private subnets (EC2 app tier, RDS).
- **EC2 app tier**: Auto Scaling Group; each instance runs mindia-api (binary) and ClamAV as systemd services. IMDSv2 enforced.
- **RDS**: PostgreSQL 16.x with pgvector; encryption at rest; no public access.
- **S3**: Media bucket and optional releases bucket (for the mindia-api binary).
- **ALB**: Targets EC2 instances on port 8080; health check `/health`. Optionally restricted to CloudFront only.
- **CloudFront**: CDN in front of ALB; cache for media paths; HTTPS redirect.
- **Secrets Manager**: One secret with `DATABASE_URL` and `MASTER_API_KEY` (and optionally `JWT_SECRET`).
- **CloudWatch alarms**: Optional alarms for ALB 5XX count, ALB target response time, RDS CPU, and RDS connections. Set `alarm_sns_topic_arn` to an SNS topic ARN to receive notifications.

See [configuration.md](configuration.md) and [clamav.md](clamav.md) for app-level options.

## 1. Create secrets (before or after first apply)

Terraform can create the app secret and set `DATABASE_URL` from RDS. You must set `MASTER_API_KEY` yourself:

1. After first `terraform apply`, get the secret ARN from output `app_secret_arn`.
2. In AWS Console → Secrets Manager, open the secret and choose **Retrieve secret value** → **Edit**.
3. Set `MASTER_API_KEY` to a secure value (e.g. `openssl rand -hex 32`). Save.
4. Optionally set `JWT_SECRET` if your app uses it.

Alternatively, create the secret manually and pass its ARN as `app_secret_arn`; then Terraform will not create a secret and will use that ARN for the instance profile policy.

## 2. Terraform backend (recommended for production)

For remote state and locking:

1. Create an S3 bucket (e.g. `your-org-terraform-state`) with versioning and server-side encryption.
2. Create a DynamoDB table (e.g. `terraform-state-lock`) with primary key `LockID` (String).
3. In `.deployment/versions.tf`, uncomment the `backend "s3"` block and set `bucket`, `key`, `region`, `dynamodb_table`.

## 3. Run Terraform

```bash
cd .deployment
terraform init
terraform plan -out=tfplan
terraform apply tfplan
```

Note any required variables (e.g. `db_username` may need to be set if not using default). For a first run you can use defaults; ensure `MASTER_API_KEY` is set in the secret after apply (see step 1).

## 4. Build and publish the binary

Build the mindia-api binary and upload it to the releases bucket:

```bash
cargo build --release -p mindia-api
aws s3 cp target/release/mindia-api s3://$(terraform -chdir=.deployment output -raw releases_bucket_name)/mindia-api
```

Or use a versioned key: `s3://.../releases/v1.0.0/mindia-api`.

## 5. Ansible: inventory and variables

**Inventory**: Populate `.deployment/ansible/inventory.yml` with your EC2 instance IPs. You can get private IPs from the EC2 console or by adding an output. Example:

```yaml
all:
  children:
    mindia_app:
      hosts:
        app1:
          ansible_host: 10.0.1.50
        app2:
          ansible_host: 10.0.2.50
      vars:
        ansible_user: ec2-user
```

Use a key that can SSH to the instances (e.g. the key pair you used for the launch template, or SSM Session Manager).

**Variables**: Copy `ansible/group_vars/mindia_app.yml.example` to `group_vars/mindia_app.yml` and set values from Terraform outputs:

- `app_secret_arn`: `terraform -chdir=.deployment output -raw app_secret_arn`
- `cloudfront_url`: `terraform -chdir=.deployment output -raw cloudfront_url`
- `cdn_domains`: CloudFront hostname (e.g. `d1234567890.cloudfront.net`)
- `s3_bucket`: `terraform -chdir=.deployment output -raw media_bucket_name`
- `s3_region`: Your AWS region (e.g. `us-east-1`)
- `releases_bucket`: `terraform -chdir=.deployment output -raw releases_bucket_name`
- `mindia_binary_s3_key`: `mindia-api` or `releases/v1.0.0/mindia-api`

## 6. Run Ansible

```bash
cd .deployment/ansible
ansible-playbook playbook.yml
```

This installs ClamAV, fetches the secret and writes `/etc/mindia/env`, downloads the binary from S3, and starts the mindia-api systemd service.

## 7. CDN and app config

The app receives the CloudFront URL via the env file (Ansible sets `CLOUDFRONT_URL` and `CDN_DOMAINS` from the variables above). After the first deploy, ensure `CORS_ORIGINS` in the secret or env is set to your front-end origins (no `*` in production).

## 8. Scaling and DB connections

Set `DB_MAX_CONNECTIONS` so that **number of instances × DB_MAX_CONNECTIONS ≤ RDS max_connections**. For example, 5 instances × 20 = 100. Add `DB_MAX_CONNECTIONS` to the secret value (or extend the Ansible env template) and redeploy.

## Optional: user data bootstrap

For scale-out instances to be ready without running Ansible, add a user-data script to the launch template that: installs ClamAV, fetches the secret from Secrets Manager, downloads the binary from S3, writes the env file, and starts systemd units. See the plan in the repo for details.

## Optional: GitHub Actions deploy

Add a workflow that: builds the binary, uploads to S3, runs Ansible (via SSH or SSM). Use OIDC or stored credentials for AWS and ensure the inventory is updated (e.g. dynamic EC2 inventory by tag).

## Security

- ALB can be restricted to CloudFront only (`alb_allow_cloudfront_only = true`) so the ALB is not directly internet-facing.
- Secrets are in Secrets Manager; instances use IAM to read. No secrets in Terraform vars or Ansible vars in the repo.
- IMDSv2 is enforced on EC2. RDS and app instances are in private subnets.

See the **Security best practices** section in the deployment plan and [configuration.md](configuration.md).
