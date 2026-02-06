# ClamAV Virus Scanning

Mindia can scan uploaded files for malware using [ClamAV](https://www.clamav.net/), an open-source antivirus engine. When enabled, every uploaded image, video, audio, or document is scanned before being stored.

## Overview

- **When**: Scans run during upload, before files are written to storage
- **What**: Images, videos, audio, documents (all uploaded media types)
- **How**: Connects to a ClamAV daemon via TCP
- **Timeout**: 30 seconds per scan

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CLAMAV_ENABLED` | `false` | Set to `true` to enable virus scanning |
| `CLAMAV_HOST` | `localhost` | ClamAV daemon hostname or IP |
| `CLAMAV_PORT` | `3310` | ClamAV daemon port |
| `CLAMAV_FAIL_CLOSED` | `true` (production) / `false` (dev) | Reject uploads when ClamAV is unavailable |

### Example

```env
CLAMAV_ENABLED=true
CLAMAV_HOST=localhost
CLAMAV_PORT=3310
CLAMAV_FAIL_CLOSED=true
```

## Fail-Closed vs Fail-Open

| Mode | When ClamAV unavailable | Use case |
|------|-------------------------|----------|
| **Fail-closed** (`CLAMAV_FAIL_CLOSED=true`) | Reject upload with error | Production; security-critical |
| **Fail-open** (`CLAMAV_FAIL_CLOSED=false`) | Allow upload | Development; optional hardening |

In production, `CLAMAV_FAIL_CLOSED` defaults to `true` so uploads are rejected if ClamAV is down, times out, or returns an error. In development, it defaults to `false` so you can test without a running ClamAV daemon.

## Installation

### macOS

```bash
brew install clamav
brew services start clamav
```

### Linux (Debian/Ubuntu)

```bash
sudo apt install clamav clamav-daemon
sudo systemctl start clamav-daemon
```

### Docker

Use the ClamAV-enabled Dockerfile:

```bash
docker build -f Dockerfile.with-clamav -t mindia:clamav .
```

This image includes ClamAV, starts the daemon before the app, and updates virus definitions on build.

## Behavior

### Clean Files

- Scan completes successfully
- File is stored normally
- Logs: `File scan completed: clean`

### Infected Files

- Upload returns `400` with error: `File rejected: virus detected (<virus_name>)`
- File is not stored
- Logs: `File scan detected virus`

### ClamAV Unavailable

- **Fail-closed**: Upload returns `500` with "Virus scanning temporarily unavailable"
- **Fail-open**: Upload succeeds (no scan)

### Timeout

- If scan exceeds 30 seconds:
  - **Fail-closed**: Upload rejected
  - **Fail-open**: Upload allowed

## API Response

The config endpoint exposes ClamAV status:

```bash
GET /api/v0/config
```

Response includes:

```json
{
  "clamav": {
    "enabled": true
  }
}
```

## Best Practices

1. **Production**: Use `CLAMAV_FAIL_CLOSED=true` so unavailable scans block uploads.
2. **Updates**: Run `freshclam` regularly on the ClamAV host to keep signatures current.
3. **Performance**: Expect ~100–500 ms extra latency per upload when scanning.
4. **Memory**: ClamAV typically needs ~2 GB RAM; consider this when sizing hosts.
5. **Testing**: Use the [EICAR test file](https://www.eicar.org/?page_id=3950) to verify detection (with ClamAV enabled, it should be rejected).

## Requirements

- Mindia built with the `clamav` feature (included in default features)
- ClamAV daemon listening on the configured host/port
- Network access from Mindia to the ClamAV daemon

## Related Documentation

- [Configuration](configuration.md) – Full environment variable reference
- [Installation](installation.md) – Setup instructions including ClamAV
- [Images](images.md) – Image upload flow
- [Videos](videos.md) – Video upload flow
