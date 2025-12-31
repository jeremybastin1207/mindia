# Analytics

Track usage, monitor traffic, and analyze storage metrics for your Mindia instance.

## Table of Contents

- [Overview](#overview)
- [Traffic Summary](#traffic-summary)
- [URL Statistics](#url-statistics)
- [Storage Metrics](#storage-metrics)
- [Refresh Metrics](#refresh-metrics)
- [Analytics Examples](#analytics-examples)
- [Best Practices](#best-practices)

## Overview

Mindia provides built-in analytics to help you understand usage patterns and resource consumption.

**Available Metrics**:
- ✅ Request traffic (count, volume, response times)
- ✅ Popular URLs and endpoints
- ✅ HTTP status code distribution
- ✅ Storage usage per media type
- ✅ File counts and sizes

**Permissions**:
- **Admin**: Full access to all analytics
- **Member**: Basic storage metrics only
- **Viewer**: Basic storage metrics only

## Traffic Summary

Get comprehensive traffic statistics for your organization.

### Endpoint

```
GET /api/analytics/traffic
```

### Headers

```
Authorization: Bearer <token>
```

### Query Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `start_date` | string | No | Start date (ISO 8601 format) |
| `end_date` | string | No | End date (ISO 8601 format) |
| `limit` | integer | No | Max popular URLs to return (default: 20) |

### Response

**Status**: `200 OK`

```json
{
  "total_requests": 15234,
  "total_bytes_sent": 524288000,
  "total_bytes_received": 104857600,
  "avg_response_time_ms": 45.2,
  "requests_per_method": {
    "GET": 12000,
    "POST": 2500,
    "DELETE": 500,
    "PATCH": 234
  },
  "requests_per_status": {
    "200": 14000,
    "201": 2500,
    "404": 500,
    "500": 234
  },
  "popular_urls": [
    {
      "url": "/api/images",
      "method": "GET",
      "request_count": 5000,
      "avg_response_time_ms": 35.5,
      "total_bytes_sent": 157286400
    },
    {
      "url": "/api/videos/:id/stream/master.m3u8",
      "method": "GET",
      "request_count": 3500,
      "avg_response_time_ms": 120.8,
      "total_bytes_sent": 245760000
    }
  ]
}
```

### Examples

```bash
TOKEN="your-token"

# Get all-time traffic summary
curl https://api.example.com/api/analytics/traffic \
  -H "Authorization: Bearer $TOKEN"

# Get traffic for specific date range
curl "https://api.example.com/api/analytics/traffic?start_date=2024-01-01&end_date=2024-01-31" \
  -H "Authorization: Bearer $TOKEN"

# Get top 10 most popular URLs
curl "https://api.example.com/api/analytics/traffic?limit=10" \
  -H "Authorization: Bearer $TOKEN"
```

```javascript
async function getTrafficSummary(startDate, endDate, limit = 20) {
  const token = localStorage.getItem('token');
  
  const params = new URLSearchParams();
  if (startDate) params.append('start_date', startDate);
  if (endDate) params.append('end_date', endDate);
  params.append('limit', limit.toString());

  const response = await fetch(
    `https://api.example.com/api/analytics/traffic?${params}`,
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  return await response.json();
}

// Usage
const stats = await getTrafficSummary('2024-01-01', '2024-01-31');
console.log('Total requests:', stats.total_requests);
console.log('Avg response time:', stats.avg_response_time_ms, 'ms');
```

## URL Statistics

Get detailed statistics for specific URLs or patterns.

### Endpoint

```
GET /api/analytics/urls
```

### Headers

```
Authorization: Bearer <token>
```

### Query Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `start_date` | string | No | Start date (ISO 8601 format) |
| `end_date` | string | No | End date (ISO 8601 format) |
| `limit` | integer | No | Max results (default: 20) |

### Response

```json
[
  {
    "url": "/api/images",
    "method": "GET",
    "request_count": 5000,
    "avg_response_time_ms": 35.5,
    "total_bytes_sent": 157286400
  },
  {
    "url": "/api/images/:id",
    "method": "GET",
    "request_count": 3200,
    "avg_response_time_ms": 28.3,
    "total_bytes_sent": 102400000
  }
]
```

### Example

```javascript
async function getURLStatistics(limit = 20) {
  const token = localStorage.getItem('token');

  const response = await fetch(
    `https://api.example.com/api/analytics/urls?limit=${limit}`,
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  return await response.json();
}

// Display top URLs
const urls = await getURLStatistics(10);
urls.forEach(stat => {
  console.log(`${stat.method} ${stat.url}: ${stat.request_count} requests`);
});
```

## Storage Metrics

Get current storage usage across all media types.

### Endpoint

```
GET /api/analytics/storage
```

### Headers

```
Authorization: Bearer <token>
```

### Response

```json
{
  "total_files": 1543,
  "total_bytes": 524288000,
  "by_content_type": [
    {
      "content_type": "image/jpeg",
      "file_count": 850,
      "total_bytes": 209715200
    },
    {
      "content_type": "video/mp4",
      "file_count": 120,
      "total_bytes": 262144000
    },
    {
      "content_type": "audio/mpeg",
      "file_count": 450,
      "total_bytes": 41943040
    },
    {
      "content_type": "application/pdf",
      "file_count": 123,
      "total_bytes": 10485760
    }
  ],
  "last_updated": "2024-01-01T12:00:00Z"
}
```

### Examples

```bash
curl https://api.example.com/api/analytics/storage \
  -H "Authorization: Bearer $TOKEN"
```

```javascript
async function getStorageMetrics() {
  const token = localStorage.getItem('token');

  const response = await fetch(
    'https://api.example.com/api/analytics/storage',
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  return await response.json();
}

// Display storage breakdown
const storage = await getStorageMetrics();
console.log(`Total: ${formatBytes(storage.total_bytes)} (${storage.total_files} files)`);

storage.by_content_type.forEach(type => {
  const size = formatBytes(type.total_bytes);
  console.log(`${type.content_type}: ${size} (${type.file_count} files)`);
});

function formatBytes(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
  return (bytes / (1024 * 1024 * 1024)).toFixed(1) + ' GB';
}
```

## Refresh Metrics

Recalculate storage metrics (admin only). Metrics are cached and automatically refreshed every hour.

### Endpoint

```
POST /api/analytics/storage/refresh
```

### Headers

```
Authorization: Bearer <token>
```

### Response

**Status**: `204 No Content`

### Example

```bash
curl -X POST https://api.example.com/api/analytics/storage/refresh \
  -H "Authorization: Bearer $TOKEN"
```

```javascript
async function refreshStorageMetrics() {
  const token = localStorage.getItem('token');

  await fetch('https://api.example.com/api/analytics/storage/refresh', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
    },
  });

  // Wait a moment then fetch fresh metrics
  await new Promise(resolve => setTimeout(resolve, 1000));
  return await getStorageMetrics();
}
```

## Analytics Examples

### React Analytics Dashboard

```tsx
import { useEffect, useState } from 'react';

function AnalyticsDashboard() {
  const [traffic, setTraffic] = useState(null);
  const [storage, setStorage] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadAnalytics();
  }, []);

  async function loadAnalytics() {
    try {
      const [trafficData, storageData] = await Promise.all([
        getTrafficSummary(),
        getStorageMetrics(),
      ]);
      
      setTraffic(trafficData);
      setStorage(storageData);
    } catch (error) {
      console.error('Failed to load analytics:', error);
    } finally {
      setLoading(false);
    }
  }

  if (loading) return <div>Loading analytics...</div>;

  return (
    <div className="analytics-dashboard">
      <div className="metrics-grid">
        <MetricCard
          title="Total Requests"
          value={traffic.total_requests.toLocaleString()}
          icon="📊"
        />
        <MetricCard
          title="Avg Response Time"
          value={`${traffic.avg_response_time_ms.toFixed(1)} ms`}
          icon="⚡"
        />
        <MetricCard
          title="Total Files"
          value={storage.total_files.toLocaleString()}
          icon="📁"
        />
        <MetricCard
          title="Storage Used"
          value={formatBytes(storage.total_bytes)}
          icon="💾"
        />
      </div>

      <div className="charts">
        <div className="chart">
          <h3>Requests by Method</h3>
          <BarChart data={traffic.requests_per_method} />
        </div>

        <div className="chart">
          <h3>Storage by Type</h3>
          <PieChart data={storage.by_content_type} />
        </div>

        <div className="chart">
          <h3>Popular URLs</h3>
          <URLTable urls={traffic.popular_urls} />
        </div>
      </div>
    </div>
  );
}

function MetricCard({ title, value, icon }) {
  return (
    <div className="metric-card">
      <span className="icon">{icon}</span>
      <div className="content">
        <h4>{title}</h4>
        <p className="value">{value}</p>
      </div>
    </div>
  );
}
```

### Date Range Filter

```javascript
function DateRangeAnalytics() {
  const [startDate, setStartDate] = useState('');
  const [endDate, setEndDate] = useState('');
  const [stats, setStats] = useState(null);

  async function loadStats() {
    const data = await getTrafficSummary(startDate, endDate);
    setStats(data);
  }

  return (
    <div>
      <div className="date-range">
        <input
          type="date"
          value={startDate}
          onChange={(e) => setStartDate(e.target.value)}
        />
        <input
          type="date"
          value={endDate}
          onChange={(e) => setEndDate(e.target.value)}
        />
        <button onClick={loadStats}>Load Stats</button>
      </div>

      {stats && (
        <div className="stats">
          <p>Requests: {stats.total_requests}</p>
          <p>Data Sent: {formatBytes(stats.total_bytes_sent)}</p>
          <p>Avg Response: {stats.avg_response_time_ms.toFixed(1)} ms</p>
        </div>
      )}
    </div>
  );
}
```

### Export Analytics

```javascript
async function exportAnalytics(format = 'json') {
  const [traffic, storage] = await Promise.all([
    getTrafficSummary(),
    getStorageMetrics(),
  ]);

  const data = {
    generated_at: new Date().toISOString(),
    traffic,
    storage,
  };

  if (format === 'json') {
    const json = JSON.stringify(data, null, 2);
    downloadFile('analytics.json', json, 'application/json');
  } else if (format === 'csv') {
    const csv = convertToCSV(data);
    downloadFile('analytics.csv', csv, 'text/csv');
  }
}

function downloadFile(filename, content, type) {
  const blob = new Blob([content], { type });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
```

## Best Practices

### 1. Cache Analytics Data

```javascript
class AnalyticsCache {
  constructor(ttl = 300000) { // 5 minutes
    this.cache = new Map();
    this.ttl = ttl;
  }

  async get(key, fetcher) {
    const cached = this.cache.get(key);
    
    if (cached && Date.now() - cached.timestamp < this.ttl) {
      return cached.data;
    }

    const data = await fetcher();
    this.cache.set(key, {
      data,
      timestamp: Date.now(),
    });

    return data;
  }
}

const cache = new AnalyticsCache();

async function getCachedTraffic() {
  return await cache.get('traffic', () => getTrafficSummary());
}
```

### 2. Refresh Storage Metrics Periodically

```javascript
// Refresh storage metrics every hour
setInterval(async () => {
  try {
    await refreshStorageMetrics();
    console.log('Storage metrics refreshed');
  } catch (error) {
    console.error('Failed to refresh metrics:', error);
  }
}, 3600000); // 1 hour
```

### 3. Monitor Key Metrics

```javascript
async function checkHealthMetrics() {
  const traffic = await getTrafficSummary();
  
  // Alert if error rate is high
  const errorCount = (traffic.requests_per_status['500'] || 0) +
                     (traffic.requests_per_status['503'] || 0);
  const errorRate = errorCount / traffic.total_requests;
  
  if (errorRate > 0.05) { // > 5% errors
    alert(`High error rate: ${(errorRate * 100).toFixed(1)}%`);
  }

  // Alert if response time is slow
  if (traffic.avg_response_time_ms > 1000) { // > 1 second
    alert(`Slow response time: ${traffic.avg_response_time_ms}ms`);
  }
}
```

### 4. Visualize Trends

```javascript
async function getWeeklyTrend() {
  const today = new Date();
  const weekAgo = new Date(today - 7 * 24 * 60 * 60 * 1000);
  
  const thisWeek = await getTrafficSummary(
    weekAgo.toISOString(),
    today.toISOString()
  );

  const twoWeeksAgo = new Date(today - 14 * 24 * 60 * 60 * 1000);
  
  const lastWeek = await getTrafficSummary(
    twoWeeksAgo.toISOString(),
    weekAgo.toISOString()
  );

  const change = ((thisWeek.total_requests - lastWeek.total_requests) /
                  lastWeek.total_requests) * 100;

  return {
    thisWeek: thisWeek.total_requests,
    lastWeek: lastWeek.total_requests,
    change: change.toFixed(1) + '%',
    direction: change > 0 ? 'up' : 'down',
  };
}
```

### 5. Set Up Alerts

```javascript
async function monitorStorageQuota(maxBytes) {
  const storage = await getStorageMetrics();
  
  const usagePercent = (storage.total_bytes / maxBytes) * 100;

  if (usagePercent > 90) {
    sendAlert(`Storage at ${usagePercent.toFixed(0)}% capacity!`);
  } else if (usagePercent > 75) {
    console.warn(`Storage at ${usagePercent.toFixed(0)}% capacity`);
  }

  return {
    used: storage.total_bytes,
    max: maxBytes,
    percent: usagePercent,
  };
}
```

## Next Steps

- [Webhooks](webhooks.md) - Set up event notifications
- [Best Practices](best-practices.md) - Optimization and monitoring tips
- [API Reference](api-reference.md) - Complete API documentation

