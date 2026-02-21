CREATE MATERIALIZED VIEW IF NOT EXISTS hourly_url_statistics AS
SELECT
    time_bucket('1 hour', created_at) AS bucket,
    normalized_path,
    COUNT(*)::BIGINT AS request_count,
    COALESCE(SUM(response_size_bytes), 0)::BIGINT AS total_bytes_sent,
    COALESCE(SUM(request_size_bytes), 0)::BIGINT AS total_bytes_received,
    COALESCE(SUM(duration_ms), 0)::BIGINT AS total_duration_ms,
    COALESCE(MIN(duration_ms), 0)::BIGINT AS min_duration_ms,
    COALESCE(MAX(duration_ms), 0)::BIGINT AS max_duration_ms,
    COUNT(*) FILTER (WHERE status_code >= 200 AND status_code < 300)::BIGINT AS status_2xx,
    COUNT(*) FILTER (WHERE status_code >= 300 AND status_code < 400)::BIGINT AS status_3xx,
    COUNT(*) FILTER (WHERE status_code >= 400 AND status_code < 500)::BIGINT AS status_4xx,
    COUNT(*) FILTER (WHERE status_code >= 500)::BIGINT AS status_5xx
FROM request_logs
GROUP BY bucket, normalized_path;

CREATE UNIQUE INDEX IF NOT EXISTS hourly_url_statistics_bucket_path_idx ON hourly_url_statistics (bucket, normalized_path);
