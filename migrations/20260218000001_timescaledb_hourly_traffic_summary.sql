-- Regular materialized views (Apache TimescaleDB has no continuous aggregates).
-- Refresh via cron: REFRESH MATERIALIZED VIEW CONCURRENTLY hourly_traffic_summary;
CREATE MATERIALIZED VIEW IF NOT EXISTS hourly_traffic_summary AS
SELECT
    time_bucket('1 hour', created_at) AS bucket,
    COUNT(*)::BIGINT AS total_requests,
    COALESCE(SUM(response_size_bytes), 0)::BIGINT AS total_bytes_sent,
    COALESCE(SUM(request_size_bytes), 0)::BIGINT AS total_bytes_received,
    COALESCE(SUM(duration_ms), 0)::BIGINT AS total_duration_ms
FROM request_logs
GROUP BY bucket;

CREATE UNIQUE INDEX IF NOT EXISTS hourly_traffic_summary_bucket_idx ON hourly_traffic_summary (bucket);
