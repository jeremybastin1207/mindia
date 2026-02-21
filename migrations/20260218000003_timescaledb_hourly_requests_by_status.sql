CREATE MATERIALIZED VIEW IF NOT EXISTS hourly_requests_by_status AS
SELECT
    time_bucket('1 hour', created_at) AS bucket,
    status_code,
    COUNT(*)::BIGINT AS request_count
FROM request_logs
GROUP BY bucket, status_code;

CREATE UNIQUE INDEX IF NOT EXISTS hourly_requests_by_status_bucket_status_idx ON hourly_requests_by_status (bucket, status_code);
