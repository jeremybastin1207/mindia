# Workflows

Workflows let you define automated processing pipelines that run when media is uploaded or when you trigger them manually. Each workflow is an ordered list of steps (plugins) that run in sequence. You can scope workflows to a subset of media using filters (media type, folder, content type, metadata).

**Feature:** Enable with the `workflow` Cargo feature when building the API.

## Concepts

- **Workflow** – Named definition with ordered steps, filters, and options (e.g. stop on failure).
- **Workflow execution** – A single run of a workflow on one media item. Tracks status and the chain of tasks.
- **Steps** – Plugin-based. Each step runs one plugin (e.g. `aws_rekognition_moderation`, `aws_rekognition`, `replicate_deoldify`) in order. Steps are chained via task dependencies.

## Create a workflow

```http
POST /api/v0/workflows
Authorization: Bearer <api_key>
Content-Type: application/json

{
  "name": "Auto Process Images",
  "description": "Moderate, detect objects, and colorize uploaded images",
  "enabled": true,
  "trigger_on_upload": true,
  "stop_on_failure": true,
  "media_types": ["image"],
  "folder_ids": null,
  "content_types": null,
  "metadata_filter": null,
  "steps": [
    { "action": "plugin", "plugin_name": "aws_rekognition_moderation" },
    { "action": "plugin", "plugin_name": "aws_rekognition" },
    { "action": "plugin", "plugin_name": "replicate_deoldify" }
  ]
}
```

- **trigger_on_upload** – If `true`, matching uploads automatically start this workflow.
- **stop_on_failure** – If `true`, when a step fails, later steps are cancelled. If `false`, the workflow continues.
- **media_types** – Optional. Only run for these types (`image`, `video`, `audio`, `document`). Omit or empty = all types.
- **folder_ids** – Optional. Only run for media in these folders. Omit or empty = any folder.
- **content_types** – Optional. Only run for these MIME types. Omit or empty = any.
- **metadata_filter** – Optional. JSON object; media must have matching metadata keys/values. Omit or empty = any.

## List and manage workflows

- **List:** `GET /api/v0/workflows?limit=50&offset=0`
- **Get:** `GET /api/v0/workflows/:id`
- **Update:** `PUT /api/v0/workflows/:id` (same body shape as create, partial updates)
- **Delete:** `DELETE /api/v0/workflows/:id`

## Trigger on existing media

Run a workflow on a media item that is already stored:

```http
POST /api/v0/workflows/:workflow_id/trigger/:media_id
Authorization: Bearer <api_key>
```

Returns the new workflow execution.

## Execution status

- **List executions for a workflow:** `GET /api/v0/workflows/:id/executions?limit=50&offset=0`
- **Get one execution:** `GET /api/v0/workflow-executions/:id`

Execution status: `pending`, `running`, `completed`, `failed`, `cancelled`. Status is derived from the underlying task chain.

## Webhooks

When a workflow run finishes, webhooks can be triggered:

- **workflow.completed** – All steps completed successfully.
- **workflow.failed** – At least one step failed (and optionally later steps were cancelled).

Configure webhooks for these event types the same way as other events (e.g. `file.uploaded`). The payload includes the media id, workflow execution context, and status.

## Build with workflows

Build the API with the workflow feature:

```bash
cargo build -p mindia-api --features workflow
```

Run migrations so the `workflows` and `workflow_executions` tables (and new webhook event types) exist.
