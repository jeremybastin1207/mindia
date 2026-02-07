# Database migrations

Migrations run in lexicographic order by filename. Use **one naming scheme** for new migrations:

- **Preferred:** `000000000000XX_short_description.sql` with a zero-padded 12-digit index (e.g. next after `00000000000012` would be `00000000000013_`).
- The set `20260207000001_*` … `20260207000003_*` uses a date-based prefix; leave them as-is for compatibility with existing deployments.

Create new migrations with the next sequential number (e.g. `00000000000013_my_change.sql`).
