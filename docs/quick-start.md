# Quick Start Guide

Get Mindia running in 5 minutes! This guide will help you deploy Mindia and start using the API immediately.

## Prerequisites

Before you begin, you'll need:
- A PostgreSQL database (we recommend [Neon](https://neon.tech) for serverless)
- An S3-compatible storage bucket (AWS S3, MinIO, etc.)
- AWS credentials with S3 access

## Option 1: Deploy to Fly.io (Recommended)

The fastest way to get Mindia running in production.

### Step 1: Install Fly CLI

```bash
curl -L https://fly.io/install.sh | sh
```

### Step 2: Sign Up for Services

Create free accounts for:
- **Fly.io**: https://fly.io/app/sign-up
- **Neon Database**: https://console.neon.tech/sign_in
- **AWS** (for S3): https://aws.amazon.com/free

### Step 3: Set Up Database

1. Go to https://console.neon.tech
2. Create a new project
3. Copy the PostgreSQL connection string
4. Make sure it includes `?sslmode=require`

### Step 4: Set Up S3 Storage

1. Create an S3 bucket in AWS Console
2. Create an IAM user with S3 permissions (PutObject, GetObject, DeleteObject)
3. Generate access keys for the IAM user

### Step 5: Configure Fly.io App

```bash
# Clone the repository
git clone <your-repo>
cd mindia

# Edit fly.toml and change the app name to something unique
nano fly.toml

# Change this line:
# app = "mindia"  →  app = "your-unique-name"
```

### Step 6: Set Secrets

```bash
APP_NAME="your-unique-name"

flyctl secrets set \
  DATABASE_URL="postgresql://user:pass@host.neon.tech/db?sslmode=require" \
  S3_BUCKET="your-bucket-name" \
  S3_REGION="us-east-1" \
  AWS_ACCESS_KEY_ID="AKIA..." \
  AWS_SECRET_ACCESS_KEY="..." \
  MASTER_API_KEY="$(openssl rand -hex 32)" \
  -a $APP_NAME
```

### Step 7: Deploy

```bash
# Initial deployment
flyctl launch --copy-config --now

# Or if already initialized
flyctl deploy -a $APP_NAME
```

### Step 8: Get Your Master API Key

Retrieve the master API key you set earlier:

```bash
# View your master API key
flyctl secrets list -a $APP_NAME | grep MASTER_API_KEY
```

Note: For security, Fly.io only shows the key name, not the value. If you need to retrieve it, you'll need to regenerate it.

### Step 9: Test Your Deployment

```bash
# Set your master API key (use the one you generated)
MASTER_API_KEY="your-master-api-key-from-step-6"

# Upload an image
curl -X POST https://your-unique-name.fly.dev/api/images \
  -H "Authorization: Bearer $MASTER_API_KEY" \
  -F "file=@photo.jpg"

# Health check
curl https://your-unique-name.fly.dev/health
```

**🎉 Done!** Your API is live at `https://your-unique-name.fly.dev`

---

## Option 2: Run Locally

Perfect for development and testing.

### Step 1: Install Rust

```bash
# Install from https://rustup.rs/
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart your terminal, then verify
rustc --version
cargo --version
```

### Step 2: Set Up Services

**Database (Neon)**:
1. Sign up at https://console.neon.tech
2. Create a new project
3. Copy the connection string

**Storage (S3)**:
1. Create an S3 bucket in AWS Console
2. Create IAM user with S3 permissions
3. Generate access keys

### Step 3: Configure Environment

```bash
cd mindia

# Copy the example environment file
cp .env.example .env

# Edit with your credentials
nano .env
```

Required settings in `.env`:
```env
DATABASE_URL=postgresql://user:password@host.neon.tech/dbname?sslmode=require
S3_BUCKET=your-bucket-name
S3_REGION=us-east-1
AWS_ACCESS_KEY_ID=your-access-key-id
AWS_SECRET_ACCESS_KEY=your-secret-access-key
MASTER_API_KEY=your-master-api-key-at-least-32-characters-long
```

### Step 4: Run Mindia

```bash
# Run in development mode (with hot reload)
cargo run

# Or build and run release version
cargo build --release
./target/release/mindia
```

The server will start on `http://localhost:3000`.

### Step 5: Test Your Setup

```bash
# Set your master API key (from .env file)
MASTER_API_KEY="your-master-api-key-from-env-file"

# Health check
curl http://localhost:3000/health

# Upload an image
curl -X POST http://localhost:3000/api/images \
  -H "Authorization: Bearer $MASTER_API_KEY" \
  -F "file=@photo.jpg"
```

**🎉 Success!** Your local instance is running at `http://localhost:3000`

---

## Option 3: Docker

Run Mindia in a container.

### Step 1: Install Docker

Download from https://docs.docker.com/get-docker/

### Step 2: Configure

```bash
cd mindia

# Copy and edit environment file
cp .env.example .env
nano .env
```

### Step 3: Run with Docker Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

### Step 4: Test

```bash
curl http://localhost:3000/health
```

---

## Quick API Examples

Once you're up and running, try these common operations:

### Set Your API Key

```bash
# Set your master API key
MASTER_API_KEY="your-master-api-key-here"
API_URL="http://localhost:3000"  # or your deployed URL
```

### Images

```bash
# Upload
curl -X POST $API_URL/api/images \
  -H "Authorization: Bearer $MASTER_API_KEY" \
  -F "file=@photo.jpg"

# List
curl $API_URL/api/images \
  -H "Authorization: Bearer $MASTER_API_KEY"

# Get metadata
curl $API_URL/api/images/$IMAGE_ID \
  -H "Authorization: Bearer $MASTER_API_KEY"

# Download original
curl $API_URL/api/images/$IMAGE_ID/file \
  -H "Authorization: Bearer $MASTER_API_KEY" \
  -o original.jpg

# Get resized (320px wide)
curl "$API_URL/api/images/$IMAGE_ID/-/resize/320x/" \
  -H "Authorization: Bearer $MASTER_API_KEY" \
  -o thumb.jpg

# Delete
curl -X DELETE $API_URL/api/images/$IMAGE_ID \
  -H "Authorization: Bearer $MASTER_API_KEY"
```

### Videos

```bash
# Upload video
curl -X POST $API_URL/api/videos \
  -H "Authorization: Bearer $MASTER_API_KEY" \
  -F "file=@video.mp4"

# Check processing status
curl $API_URL/api/videos/$VIDEO_ID \
  -H "Authorization: Bearer $MASTER_API_KEY"

# Stream (once processed)
# Use the hls_url from the metadata response in a video player
```

### Search

```bash
# Search all media
curl "$API_URL/api/search?q=sunset+beach" \
  -H "Authorization: Bearer $MASTER_API_KEY"

# Search only images
curl "$API_URL/api/search?q=cat&type=image&limit=10" \
  -H "Authorization: Bearer $MASTER_API_KEY"
```

---

## Next Steps

Now that you have Mindia running:

1. **Explore the API** - See the [API Reference](api-reference.md) for all endpoints
2. **Set Up Transformations** - Learn about [Image Transformations](image-transformations.md)
3. **Configure Webhooks** - Get real-time notifications with [Webhooks](webhooks.md)
4. **Enable Semantic Search** - Set up [Semantic Search](semantic-search.md) with Anthropic (Claude)
5. **Production Checklist** - Review [Best Practices](best-practices.md) before going live

## Common Issues

### Database Connection Fails

**Error**: `Failed to connect to database`

**Solution**:
- Verify `DATABASE_URL` is correct
- Ensure connection string includes `?sslmode=require`
- Check IP whitelist in Neon dashboard
- Test connection: `psql $DATABASE_URL -c "SELECT 1"`

### S3 Upload Fails

**Error**: `Failed to upload to S3`

**Solution**:
- Verify S3 bucket exists and is in the correct region
- Check AWS credentials are correct
- Ensure IAM user has PutObject, GetObject, DeleteObject permissions
- Test with AWS CLI: `aws s3 ls s3://your-bucket-name`

### Authentication Fails

**Error**: `Invalid API key` or `Missing authorization header`

**Solution**:
- Ensure `MASTER_API_KEY` is set in your `.env` file
- Must be at least 32 characters long
- Generate with: `openssl rand -hex 32`
- Include the key in your requests: `Authorization: Bearer YOUR_KEY`

### Port Already in Use

**Error**: `Address already in use`

**Solution**:
- Change PORT in `.env` to a different value
- Or kill the process using port 3000: `lsof -ti:3000 | xargs kill -9`

## Getting Help

- **Full Documentation**: See [User Documentation](README.md)
- **Installation Guide**: [Installation](installation.md) for detailed setup
- **Configuration**: [Configuration](configuration.md) for all environment variables
- **API Reference**: [API Reference](api-reference.md) for complete endpoint docs

**Ready to dive deeper?** Check out the full documentation in this folder!

