CREATE MATERIALIZED VIEW IF NOT EXISTS hourly_requests_by_method AS
SELECT
    time_bucket('1 hour', created_at) AS bucket,
    method,
    COUNT(*)::BIGINT AS request_count
FROM request_logs
GROUP BY bucket, method;

CREATE UNIQUE INDEX IF NOT EXISTS hourly_requests_by_method_bucket_method_idx ON hourly_requests_by_method (bucket, method);
