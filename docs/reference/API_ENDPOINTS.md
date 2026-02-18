# TickTick API Endpoints

Comprehensive API reference compiled from HAR capture (web app) + CLI source code analysis.
Captured 2026-02-18 via automated Chrome DevTools recording of ticktick.com/webapp.

## How TickTick's API Works

### Architecture

TickTick exposes three API layers:

1. **v1 Open API** (`api.ticktick.com/open/v1`) — Bearer token auth via OAuth2. Documented at <https://developer.ticktick.com/docs/openapi.md>. Used for project and task CRUD. Limited scope.
2. **v2 Session API** (`api.ticktick.com/api/v2`) — Cookie-based session auth (`t=<token>` cookie). Undocumented. Much broader surface: tags, habits, columns, calendar, user preferences, batch operations. The web app uses this exclusively.
3. **v3 Sync API** (`api.ticktick.com/api/v3`) — Checkpoint-based polling (`/batch/check/{checkpoint}`). The web app polls this for incremental state sync.

### Sync Model

The web app uses an **optimistic local-first sync** model:
- On load, fetches full state via `GET /api/v3/batch/check/0` (returns all projects, tasks, tags, filters, habits, etc.)
- Subsequent polls use `GET /api/v3/batch/check/{checkpoint}` where `{checkpoint}` is a monotonic timestamp
- Mutations are batched and sent via `POST /api/v2/batch/task` (tasks), `POST /api/v2/batch/tag` (tags), etc.
- The batch endpoints accept `{ add: [], update: [], delete: [] }` payloads
- UI updates are applied optimistically before the server confirms

### Auth

- **OAuth2**: `POST ticktick.com/oauth/token` for code exchange and token refresh
- **v2 Session**: `POST /api/v2/user/signon?wc=true&remember=true` with username/password, returns session cookie (**NOTE**: TickTick added captcha to signon, making programmatic u/p auth unusable; use the `t` cookie from a browser session instead)
- The web app uses session cookies; the v1 Open API uses Bearer tokens
- Many v2 POST endpoints require an `x-device` header:
  ```json
  {"platform":"web","os":"macOS 10.15.7","device":"Chrome 130.0.0.0","name":"","version":6490,"id":"{deviceId}","channel":"website","campaign":"","websocket":""}
  ```

### CORS

The v1 Open API (`/open/v1/*`) is **CORS-restricted** — cannot be called from the web app's origin via `fetch()`. The web app only uses v2/v3 endpoints. The v1 API is designed for third-party integrations and the official mobile/desktop apps.

### Other Hosts

- `ms.ticktick.com` — Microservices (focus/pomodoro)
- `xapi.ticktick.com` — Analytics/telemetry (`/datacollect/event/push`)
- `s.ticktick.com` — Sentry error reporting

---

## Auth Endpoints

| Method | Host | Path | Notes |
|--------|------|------|-------|
| POST | `ticktick.com` | `/oauth/authorize` | OAuth2 authorization redirect |
| POST | `ticktick.com` | `/oauth/token` | Token exchange + refresh |
| POST | `api.ticktick.com` | `/api/v2/user/signon?wc=true&remember=true` | v2 session login |

## v1 Open API — Bearer Token Auth

Base: `https://api.ticktick.com/open/v1`

### Projects

| Method | Path | Notes |
|--------|------|-------|
| GET | `/project` | List all projects |
| GET | `/project/{projectId}` | Get project by ID |
| POST | `/project` | Create project |
| POST | `/project/{projectId}` | Update project |
| DELETE | `/project/{projectId}` | Delete project |
| GET | `/project/{projectId}/data` | Get project data (tasks + columns) |

### Tasks

| Method | Path | Notes |
|--------|------|-------|
| POST | `/task` | Create task |
| GET | `/project/{projectId}/task/{taskId}` | Get task by ID |
| POST | `/task/{taskId}` | Update task |
| POST | `/project/{projectId}/task/{taskId}/complete` | Complete task (empty body) |
| DELETE | `/project/{projectId}/task/{taskId}` | Delete task |

## v2 Session API — Cookie Auth

Base: `https://api.ticktick.com/api/v2`

### Batch Operations

| Method | Path | Notes |
|--------|------|-------|
| POST | `/batch/task` | Batch create/update/delete tasks (requires `x-device` header) |
| POST | `/batch/taskProject` | Move tasks between projects |
| POST | `/batch/taskParent` | Set subtask parent |
| POST | `/batch/tag` | Batch create/update tags |
| POST | `/batch/project` | Batch create/update/delete projects |
| POST | `/batch/projectGroup` | Batch create/update/delete project groups |
| POST | `/batch/filter` | Batch create/update/delete saved filters |

### Tasks (v2 Batch)

All task mutations go through `POST /batch/task` with `{ add, update, delete }`.

**Important:** `POST /batch/task` requires the `x-device` header (JSON with `platform`, `device`, `version`, `id` fields). Without it, the server returns `access_forbidden`. Other batch endpoints (`batch/taskProject`, `batch/taskParent`) do not require it.

#### Task Object

```json
{
  "id": "a1b2c3d4e5f60000deadbeef",
  "projectId": "inbox123456789",
  "title": "Buy groceries",
  "content": "",                            // description/notes (supports markdown)
  "desc": "",                               // secondary description field
  "sortOrder": 20340976648192,              // sort position (large number)
  "startDate": "2026-02-16T00:35:00.000+0000",
  "dueDate": "2026-02-16T00:35:00.000+0000",
  "timeZone": "America/Chicago",            // IANA timezone
  "isFloating": false,                      // floating = no timezone
  "isAllDay": false,                        // true = date only, no time
  "reminder": "TRIGGER:PT0S",               // legacy single reminder (iCal)
  "reminders": [                            // structured reminders array
    { "id": "a1b2c3d4e5f60001deadbeef", "trigger": "TRIGGER:PT0S" }
  ],
  "repeatFlag": "",                         // iCal RRULE (empty = no repeat)
  "repeatFirstDate": "2026-01-20T06:00:00.000+0000",  // first occurrence (repeating tasks)
  "exDate": [],                             // excluded dates for repeating tasks
  "tags": [],                               // tag names
  "priority": 0,                            // 0=none, 1=low, 3=medium, 5=high
  "status": 0,                              // 0=active, 2=completed
  "items": [],                              // checklist items (see below)
  "progress": 0,                            // 0-100 progress percentage
  "completedTime": "2026-02-16T01:53:13.000+0000",  // when completed (null if active)
  "completedUserId": 123456789,             // who completed it
  "modifiedTime": "2026-02-16T01:53:19.000+0000",
  "createdTime": "2026-02-16T00:25:22.000+0000",
  "creator": 123456789,
  "etag": "abc12345",                      // optimistic concurrency tag
  "deleted": 0,                             // 0=normal, 1=trashed
  "parentId": "",                           // parent task ID (empty = top-level)
  "commentCount": 0,
  "focusSummaries": [],                     // focus/pomodoro session summaries
  "kind": "TEXT",                           // task kind (TEXT, NOTE, etc.)
  "deletedBy": 123456789,                  // (trash only) who deleted
  "deletedTime": 1771308178221             // (trash only) epoch ms
}
```

#### Priority Values

| Value | Level |
|-------|-------|
| `0` | None |
| `1` | Low |
| `3` | Medium |
| `5` | High |

#### Status Values

| Value | Meaning |
|-------|---------|
| `0` | Active (incomplete) |
| `2` | Completed |

#### Checklist Items

```json
{
  "id": "item_1_1771403060826",
  "title": "Step 1",
  "status": 0,       // 0=incomplete, 1=complete
  "sortOrder": 0,     // display order
  "isAllDay": false,              // optional: date for checklist items
  "startDate": null,              // optional: date for checklist items
  "timeZone": "",                 // (server-returned) timezone
  "snoozeReminderTime": null,     // (server-returned) snooze state
  "completedTime": null           // (server-returned) when completed
}
```

#### Reminder Format

Reminders use iCal TRIGGER format:

| Trigger | Meaning |
|---------|---------|
| `TRIGGER:PT0S` | At the time of the event |
| `TRIGGER:P0DT9H0M0S` | At 9:00 AM on the day |
| `TRIGGER:-PT15M` | 15 minutes before |
| `TRIGGER:-PT1H` | 1 hour before |

In batch create, reminders must be sent as structured objects with `id` and `trigger`:
```json
"reminders": [{ "id": "client_generated_id", "trigger": "TRIGGER:P0DT9H0M0S" }]
```

**Important:** Sending reminders as plain string arrays (e.g. `["TRIGGER:PT0S"]`) causes a 500 error. Always use the object format.

Alternatively, the legacy single `reminder` field accepts a plain string:
```json
"reminder": "TRIGGER:PT0S"
```

#### Repeat Rules

Uses iCal RRULE format:

| Pattern | Rule |
|---------|------|
| Daily | `RRULE:FREQ=DAILY;INTERVAL=1` |
| Weekly (MWF) | `RRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE,FR` |
| Monthly | `RRULE:FREQ=MONTHLY;INTERVAL=1` |
| Every 2 weeks | `RRULE:FREQ=WEEKLY;INTERVAL=2` |

#### Date/Time Format

All dates use ISO 8601 with timezone offset:
```
2026-02-16T14:30:00.000+0000    // UTC
2026-02-16T14:30:00.000-0600    // CST (America/Chicago in winter)
```

When `isAllDay: true`, the time portion is `T00:00:00.000` with the timezone offset of the task's `timeZone`.

#### `POST /batch/task` — Create/Update/Delete Tasks

```json
{
  "add": [{
    "id": "client_generated_id",     // client-generated ID
    "projectId": "inbox123456789",
    "title": "My task",
    "content": "Description here",
    "status": 0,
    "priority": 5,
    "sortOrder": -1099511627776,
    "startDate": "2026-02-19T08:24:20+0000",
    "dueDate": "2026-02-19T08:24:20+0000",
    "isAllDay": false,
    "timeZone": "America/Chicago",
    "reminders": ["TRIGGER:P0DT9H0M0S"],
    "repeatFlag": "RRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE,FR",
    "tags": ["work", "urgent"],
    "items": [
      { "id": "item_1", "title": "Step 1", "status": 0, "sortOrder": 0 }
    ],
    "kind": "TEXT"
  }],
  "update": [{
    "id": "existing_task_id",        // server task ID
    "projectId": "inbox123456789",
    // only include fields to change:
    "title": "Updated title",
    "priority": 3,
    "status": 2,                     // 2 = complete
    "tags": ["new-tag"],
    "items": [/* full checklist array — replaces existing */]
  }],
  "delete": [{
    "taskId": "task_id_to_delete",
    "projectId": "inbox123456789"
  }]
}
```

Response: `{ "id2etag": { "taskId": "newEtag" }, "id2error": {} }`

Notes:
- Server remaps client IDs to server IDs. The `id2etag` response maps server IDs to their etags. Use the server ID for all subsequent operations.
- `update` is a partial update — only include fields you want to change.
- `items` in update replaces the entire checklist (not a merge).
- To complete a task, set `status: 2`. To uncomplete, set `status: 0`.
- `tags` must reference existing tags, or the request may fail.
- Requires `x-device` header (see note above).

#### `POST /batch/taskProject` — Move Tasks Between Projects

```json
[{
  "taskId": "task_id",
  "fromProjectId": "source_project_id",
  "toProjectId": "target_project_id"
}]
```

Response: `{ "id2etag": {}, "id2error": {} }`

#### `POST /batch/taskParent` — Set/Unset Subtask Parent

```json
[{
  "taskId": "child_task_id",
  "parentId": "parent_task_id",       // empty string "" to unset
  "projectId": "project_id"
}]
```

Response: `{ "id2etag": {}, "id2error": {} }`

#### `GET /project/{projectId}/completed` — List Completed Tasks

Returns array of completed task objects for a project. Tasks include `completedTime` and `completedUserId`.

#### `GET /project/all/completedInAll/?limit={n}` — List Completed (All Projects)

Query param `limit` caps results (default 50). The `from`/`to` date params cause 500 errors on the v2 session API — use without date filters.

#### `GET /project/all/trash/page` — Trashed Tasks

Response:
```json
{
  "tasks": [
    { /* task object with deleted=1, deletedBy, deletedTime */ }
  ]
}
```

Trashed tasks have additional fields: `deleted: 1`, `deletedBy: userId`, `deletedTime: epochMs`.

### Tags

| Method | Path | Notes |
|--------|------|-------|
| GET | `/tags` | List all tags |
| POST | `/batch/tag` | Batch create/update tags (see below) |
| PUT | `/tag/rename` | Rename tag (`{ name, newName }`) |
| PUT | `/tag/merge` | Merge source tag into target (`{ name, newName }`) |
| DELETE | `/tag?name={encodedName}` | Delete tag by name |

#### Tag Object

```json
{
  "name": "work",                // primary key (always lowercase)
  "rawName": "Work",             // original casing (set after rename, otherwise absent)
  "label": "Work",               // display label
  "sortOrder": -549755813888,    // sort position
  "sortType": "project",         // "project" | "dueDate" | "tag"
  "color": "#4A90E2",            // hex color (optional)
  "etag": "abc123",              // optimistic concurrency tag
  "parent": "parent_tag_name",   // parent tag name for nesting (optional)
  "type": 1,                     // always 1
  "sortOption": {                // optional
    "groupBy": "project",        // "project" | "tag" | "dueDate"
    "orderBy": "dueDate",        // "dueDate" | "priority" | "sortOrder"
    "order": null                // null | "desc"
  }
}
```

#### `POST /batch/tag` — Create/Update Tags

```json
{
  "add": [{
    "name": "my-tag",
    "label": "my-tag",
    "color": "#FF5733",
    "sortOrder": -1099511627776,
    "sortType": "dueDate",
    "parent": "parent-tag-name"
  }],
  "update": [{
    "name": "existing-tag",
    "color": "#4A90E2",
    "sortOrder": -549755813888,
    "sortType": "project",
    "sortOption": { "groupBy": "tag", "orderBy": "priority", "order": "desc" },
    "parent": "new-parent"
  }]
}
```

Response: `{ "id2etag": { "tagName": "etag" }, "id2error": {} }`

Notes:
- `name` is the key for both create and update (tags don't have separate IDs).
- To nest a tag, set `parent` to the parent tag's name.
- To un-nest, set `parent: ""` (empty string). `parent: null` does NOT work.
- `update` is a partial update — only include changed fields.
- No `delete` array — use the `DELETE /tag` endpoint instead.

#### `PUT /tag/rename`

```json
{ "name": "old-name", "newName": "new-name" }
```

Response: empty string. After rename, `rawName` preserves the original name, `name` and `label` reflect the new name.

#### `PUT /tag/merge`

```json
{ "name": "source-tag", "newName": "target-tag" }
```

Response: empty string. The source tag is deleted and all tasks tagged with it are re-tagged with the target.

#### `DELETE /tag?name={encodedName}`

Deletes a single tag. Returns 200 even if the tag doesn't exist.
Deleting a parent tag does NOT cascade-delete children — children become orphaned (parent field is cleared).

### Habits

| Method | Path | Notes |
|--------|------|-------|
| GET | `/habits` | List all habits |
| GET | `/habitSections` | List habit sections (Morning, Afternoon, Night, custom) |
| POST | `/habits/batch` | Batch create/update/delete habits (see below) |
| POST | `/habitSections/batch` | Batch create/update/delete habit sections |
| POST | `/habitCheckins/batch` | Record habit check-ins (see below) |
| POST | `/habitCheckins/query` | Query check-in history (`{ habitIds, afterStamp }`) |
| POST | `/getHabitRecords` | Get habit records (`{ afterStamp, habitIds }`) |
| GET | `/user/preferences/habit?platform=web` | Habit display preferences |

#### Habit Object

```json
{
  "id": "b2c3d4e5f6a10000deadbeef",
  "name": "Drink water",
  "iconRes": "habit_drink_water",       // icon resource key
  "color": "#80F3CD",                   // hex color
  "sortOrder": -274877906944,           // sort position (large negative = top)
  "status": 0,                          // 0=active, 1=archived
  "encouragement": "Stay hydrated!",    // motivational text
  "totalCheckIns": 48,                  // lifetime check-in count
  "createdTime": "2020-01-20T09:16:45.000+0000",
  "modifiedTime": "2024-12-05T14:46:34.000+0000",
  "archivedTime": "2024-12-05T14:46:34.000+0000",  // null if active
  "type": "Real",                       // "Boolean" (yes/no) or "Real" (quantifiable)
  "goal": 6,                            // target value per day
  "step": 1,                            // increment step
  "unit": "Cup",                        // unit label (empty for Boolean)
  "etag": "def67890",                   // optimistic concurrency tag
  "repeatRule": "RRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,TU,WE,TH,FR,SA,SU",  // iCal RRULE
  "reminders": ["09:00"],               // reminder times (HH:MM)
  "recordEnable": false,                // enable record/log feature
  "sectionId": "c3d4e5f6a1b20000deadbeef",  // section ID ("-1" = no section)
  "targetDays": 21,                     // streak target
  "targetStartDate": 20200120,          // YYYYMMDD format
  "completedCycles": 0,                 // completed streak cycles
  "exDates": ["20240113"],              // excluded dates (YYYYMMDD)
  "style": 1                            // display style (null or 1)
}
```

#### Habit Types

| Type | Description | `goal` | `value` in checkin |
|------|-------------|--------|-------------------|
| `Boolean` | Yes/no habit (e.g., "Journal") | `1` | `1` (done) |
| `Real` | Quantifiable habit (e.g., "Drink 8 cups") | target (e.g., `8`) | current progress (e.g., `3`, `5`, `8`) |

#### `POST /habits/batch` — Create/Update/Delete Habits

```json
{
  "add": [{ /* full habit object */ }],
  "update": [{ "id": "...", /* partial fields to update */ }],
  "delete": ["habitId1", "habitId2"]
}
```

Response: `{ "id2etag": { "serverId": "etag" }, "id2error": {} }`

Note: Server may assign a different ID than the client-generated one. The `id2etag` maps server IDs to etags.

**Archive/unarchive** a habit by updating `status`: `0`=active, `1`=archived.
**Move to section** by updating `sectionId`.

#### `POST /habitSections/batch` — Section CRUD

```json
{
  "add": [{ "id": "clientId", "name": "My Section", "sortOrder": -100 }],
  "update": [{ "id": "...", "name": "Renamed" }],
  "delete": ["sectionId"]
}
```

Default sections use underscore-prefixed names: `_morning`, `_afternoon`, `_night`.

#### `POST /habitCheckins/batch` — Record Check-ins

```json
{
  "checkins": [{
    "habitId": "...",
    "status": 2,              // 0=in progress (partial), 2=completed
    "value": 3,               // current value (1 for Boolean, count for Real)
    "checkinStamp": 20260218, // date as YYYYMMDD integer
    "checkinTime": "2026-02-18T08:00:33.351Z"  // ISO timestamp
  }]
}
```

For Real/quantifiable habits, send incremental updates: `value: 3`, then `value: 5`, then `value: 8` (goal).
Set `status: 2` when `value >= goal` (completed), `status: 0` when partial.

#### `POST /habitCheckins/query` — Query Check-in History

```json
{ "habitIds": ["id1", "id2"], "afterStamp": 20260211 }
```

Response: `{ "checkins": { "habitId": [ /* checkin objects */ ] } }`

#### `POST /getHabitRecords` — Get Habit Records

```json
{ "afterStamp": 20260201, "habitIds": ["id1", "id2"] }
```

Response: `{ "habitRecords": { "habitId": [ /* record objects */ ] } }`

### Projects (v2)

| Method | Path | Notes |
|--------|------|-------|
| POST | `/batch/project` | Batch create/update/delete projects (see below) |
| GET | `/project/{projectId}/tasks` | List tasks in a project |
| GET | `/project/{projectId}/completed` | List completed tasks in a project |
| GET | `/project/all/closed` | List archived/closed projects |
| GET | `/project/all/completedInAll/?limit={n}` | Completed across all projects |
| GET | `/project/all/trash/page` | Trashed tasks |

**Note:** `GET /project/{projectId}` returns 405 on v2. Use the v1 Open API for single project fetch, or `GET /api/v3/batch/check/0` for the full list.

#### Project Object

```json
{
  "id": "d4e5f6a1b2c3d4e5f6a1b2c300000000",
  "name": "My Project",
  "color": "#4A90E2",                    // hex color (optional)
  "sortOrder": -549755813888,           // sort position
  "viewMode": "list",                    // "list" | "kanban" | null
  "kind": "TASK",                        // "TASK" | "NOTE"
  "groupId": "group_id",                // project group ID (optional)
  "closed": null,                        // null=active, true=archived
  "permission": null,                    // sharing permissions
  "teamId": null,                        // team membership
  "etag": "abc123",                      // optimistic concurrency tag
  "modifiedTime": "2026-02-18T00:00:00.000+0000",
  "sortType": "sortOrder",              // "sortOrder" | "dueDate" | "priority" | "project"
  "sortOption": {                        // optional
    "groupBy": "project",
    "orderBy": "sortOrder"
  },
  "barHabit": false,                     // show habits bar
  "isOwner": true                        // current user is owner
}
```

#### `POST /batch/project` — Create/Update/Delete Projects

```json
{
  "add": [{
    "id": "client_generated_id",
    "name": "New Project",
    "viewMode": "list",                  // "list" | "kanban"
    "kind": "TASK",                      // "TASK" | "NOTE"
    "color": "#FF5733",
    "sortOrder": -1099511627776,
    "groupId": "group_id"               // optional: assign to group
  }],
  "update": [{
    "id": "server_project_id",
    // partial update — only include fields to change:
    "name": "Renamed",
    "color": "#4A90E2",
    "closed": true                       // archive the project
  }],
  "delete": ["project_id_1", "project_id_2"]
}
```

Response: `{ "id2etag": { "projectId": "etag" }, "id2error": {} }`

Notes:
- Server remaps client IDs to server IDs in `id2etag`.
- `closed: true` archives, `closed: false` unarchives (server normalizes to `null`).
- `groupId: null` does NOT clear a project's group assignment. This appears to be an API limitation — no known workaround to ungroup a project.
- `viewMode: "kanban"` creates a kanban project (columns can be added separately).

### Project Groups

| Method | Path | Notes |
|--------|------|-------|
| POST | `/batch/projectGroup` | Batch create/update/delete project groups (see below) |

Groups are listed via `GET /api/v3/batch/check/0` → `projectGroups[]`.

#### Project Group Object

```json
{
  "id": "e5f6a1b2c3d40000deadbeef",
  "name": "Work",
  "sortOrder": -549755813888,
  "showAll": null,                       // show all tasks in group
  "viewMode": null,                      // "list" | "kanban" | null
  "sortType": null,                      // sort type for grouped view
  "sortOption": null,                    // sort options
  "teamId": null,
  "etag": "abc123"
}
```

#### `POST /batch/projectGroup` — Create/Update/Delete Groups

```json
{
  "add": [{
    "id": "client_generated_id",
    "name": "New Group",
    "sortOrder": -1099511627776
  }],
  "update": [{
    "id": "server_group_id",
    "name": "Renamed Group",
    "sortOrder": -2199023255552
  }],
  "delete": ["group_id_1"]
}
```

Response: `{ "id2etag": { "groupId": "etag" }, "id2error": {} }`

Notes:
- Assign projects to a group by updating the project's `groupId` field via `POST /batch/project`.
- Deleting a group does NOT un-assign its projects — they retain a stale `groupId`.
- Neither `groupId: ""` nor `groupId: null` clears a project's group assignment.

### Filters (Saved Views)

| Method | Path | Notes |
|--------|------|-------|
| POST | `/batch/filter` | Batch create/update/delete filters (see below) |

Filters are listed via `GET /api/v3/batch/check/0` → `filters[]`.

#### Filter Object

```json
{
  "id": "e5f6a1b2c3d40001deadbeef",
  "name": "High Priority",
  "rule": "{\"and\":[...],\"type\":0,\"version\":1}",  // JSON-encoded rule
  "sortOrder": -549755813888,
  "sortType": "dueDate",                // "dueDate" | "project" | "priority"
  "etag": "abc123"
}
```

#### Filter Rule Format

The `rule` field is a JSON string with this structure:

```json
{
  "and": [
    {
      "conditionName": "priority",      // condition type
      "or": [5],                        // values (OR'd)
      "conditionType": 1               // always 1
    },
    {
      "conditionName": "dueDate",
      "or": ["overdue"],
      "conditionType": 1
    }
  ],
  "type": 0,                           // always 0
  "version": 1                         // always 1
}
```

Multiple conditions in `and` are AND'd together. Values within `or` are OR'd.

#### Known Condition Names and Values

| Condition | Example Values | Notes |
|-----------|---------------|-------|
| `priority` | `1` (low), `3` (medium), `5` (high) | Numeric priority levels |
| `dueDate` | `"overdue"`, `"nodue"`, `"today"`, `"span(0~2)"` | Date range predicates |
| `tag` | `"work"`, `"personal"` | Tag name strings |
| `listOrGroup` | Nested condition objects | Project/group membership |

#### `POST /batch/filter` — Create/Update/Delete Filters

```json
{
  "add": [{
    "id": "client_generated_id",
    "name": "My Filter",
    "rule": "{\"and\":[{\"conditionName\":\"priority\",\"or\":[5],\"conditionType\":1}],\"type\":0,\"version\":1}",
    "sortOrder": -1099511627776,
    "sortType": "dueDate"
  }],
  "update": [{
    "id": "server_filter_id",
    "name": "Renamed Filter",
    "rule": "..."                        // new rule JSON string
  }],
  "delete": ["filter_id_1", "filter_id_2"]
}
```

Response: `{ "id2etag": { "filterId": "etag" }, "id2error": {} }`

### Columns (Kanban)

| Method | Path | Notes |
|--------|------|-------|
| GET | `/column?from=0` | List all columns (returns `{ update: [...] }`) |
| GET | `/column/project/{projectId}` | Columns for a specific project (returns array) |
| POST | `/column` | Create a single column (limited — see notes) |

**Note:** `POST /batch/column` returns 405 (not available). `PUT /column` and `DELETE /column/{id}` also return 405. Column CRUD via v2 is very limited. Use `POST /column` with a single object body for creation — it returns 200 but with an empty `id2etag`. Tasks can be assigned to columns by setting `columnId` in the task object.

#### Column Object

```json
{
  "id": "f6a1b2c3d4e50000deadbeef",
  "projectId": "d4e5f6a1b2c3d4e5f6a1b2c300000000",
  "name": "To Do",
  "sortOrder": 0,
  "createdTime": "2019-05-28T15:36:32.485+0000",
  "modifiedTime": "2019-05-28T15:36:32.485+0000",
  "etag": "ghi12345"
}
```

Tasks have a `columnId` field for kanban column assignment. Set it during task create or update via `POST /batch/task`.

### Calendar

| Method | Path | Notes |
|--------|------|-------|
| GET | `/calendar/subscription` | Calendar subscriptions (returns array) |
| GET | `/calendar/third/accounts` | Third-party calendar accounts (see below) |
| GET | `/calendar/archivedEvent` | Archived calendar events (returns array) |
| POST | `/calendar/bind/events/all` | Query bound events by date range (see below) |
| POST | `/calendar/bind/events/outlook` | Query Outlook events (`{ begin, end }`) |
| GET | `/calendar/cache/{id}` | Cached calendar data |

#### Third-Party Calendar Accounts

`GET /calendar/third/accounts` returns:

```json
{
  "accounts": [{
    "id": "...",
    "account": "user@gmail.com",
    "site": "google",                    // "google" | "outlook" | etc.
    "createdTime": "...",
    "modifiedTime": "...",
    "calendars": [{
      "id": "...",
      "name": "Main Calendar",
      "show": "calendar",               // "calendar" | "hidden"
      "mobileShow": "calendar",         // "calendar" | "hidden"
      "color": "#cabdbf",
      "outId": "...@import.calendar.google.com",
      "timeZone": "America/Chicago",
      "visible": true,
      "accessRole": "reader",           // "reader" | "owner" | "writer"
      "hidden": false
    }]
  }]
}
```

#### Query Bound Events

`POST /calendar/bind/events/all` with `{ "begin": "ISO date", "end": "ISO date" }` returns:

```json
{
  "events": [{
    "id": "calendar_id",
    "name": "Calendar Name",
    "color": "#cabdbf",
    "events": [{
      "id": "...",
      "uid": "...@google.com",
      "title": "Event Title",
      "dueStart": "2026-02-16T00:00:00.000+0000",
      "dueEnd": "2026-02-17T00:00:00.000+0000",
      "isAllDay": true,
      "timezone": "America/Chicago",     // optional (absent for all-day)
      "etag": "..."
    }]
  }]
}
```

### Pomodoro / Focus (read endpoints on `api.ticktick.com/api/v2`)

| Method | Path | Notes |
|--------|------|-------|
| GET | `/pomodoro/record/0` | Pomodoro records (checkpoint-based) |
| GET | `/pomodoros?from={ms}&to={ms}` | List pomodoros in time range |
| GET | `/pomodoros/statistics/generalForDesktop` | Stats (today/total count & duration) |
| GET | `/pomodoros/timeline` | Full session timeline with tasks |
| GET | `/pomodoros/timing?from={ms}&to={ms}` | Active timing entries |
| GET | `/timer` | Timer state |
| GET | `/user/preferences/pomodoro` | Pomo settings (duration, breaks, goals) |

### User / Preferences

| Method | Path | Notes |
|--------|------|-------|
| GET | `/user/profile` | User profile (name, email, picture, locale) |
| GET | `/user/status` | Account status (pro, subscription, inboxId) |
| GET | `/user/mfa` | MFA configuration |
| GET | `/user/userBindingInfo` | Linked accounts |
| GET | `/user/preferences/settings?includeWeb=true` | All settings (66 fields — see below) |
| PUT | `/user/preferences/settings?includeWeb=true` | Update settings |
| GET | `/user/preferences/dailyReminder` | Daily reminder config (see below) |
| GET | `/user/preferences/pomodoro` | Pomodoro preferences (see below) |
| GET | `/user/preferences/habit?platform=web` | Habit display preferences |
| GET | `/user/preferences/ext?mtime=0` | Extension preferences |
| GET | `/user/preferences/featurePrompt` | Feature prompt prefs |
| GET | `/user/preferences/pluginSettings` | Plugin settings |
| GET | `/configs/limits` | Account limits — free vs pro vs team (see below) |
| GET | `/templates` | Task/project templates |
| GET | `/notification/unread` | Unread notification count |

#### User Status

```json
{
  "userId": "123456789",
  "inboxId": "inbox123456789",          // default project for tasks
  "pro": true,
  "subscribeType": "stripe_subscribe",  // "stripe_subscribe" | "apple_subscribe" | etc.
  "subscribeFreq": "Year",             // "Year" | "Month"
  "proStartDate": "2025-05-14T00:38:09.000+0000",
  "proEndDate": "2026-05-15T00:38:05.000+0000",
  "freeTrial": false,
  "teamUser": false
}
```

#### Settings (Key Fields)

The settings object has 66 fields. Notable ones:

| Field | Example | Notes |
|-------|---------|-------|
| `timeZone` | `"America/Chicago"` | User's timezone |
| `startDayOfWeek` | `"MON"` | `"MON"` \| `"SUN"` \| `"SAT"` |
| `theme` | `"night"` | `"night"` \| `"light"` |
| `defaultPriority` | `0` | Default for new tasks |
| `defaultDueDate` | `0` | Default due date offset |
| `defaultRemindBefore` | `"TRIGGER:-PT30M"` | Default reminder |
| `defaultReminds` | `["TRIGGER:PT0S"]` | Default reminders array |
| `defaultADReminders` | `["TRIGGER:P0DT9H0M0S"]` | All-day task reminders |
| `defaultTimeDuration` | `30` | Default time block (minutes) |
| `showMeridiem` | `true` | 12h vs 24h time |
| `language` | `"en_US"` | UI language |
| `showCompleted` | `false` | Show completed tasks |
| `showPomodoro` | `false` | Show pomodoro widget |
| `removeDate` | `true` | Auto-parse dates from text |
| `removeTag` | `true` | Auto-parse tags from text |
| `sortTypeOfInbox` | `"priority"` | Inbox sort |
| `sortTypeOfToday` | `"project"` | Today sort |
| `webCalendarViewType` | `"y"` | Calendar view (`"y"` = year, etc.) |

#### Daily Reminder

```json
{
  "enable": true,
  "dailyReminders": ["07:00", "20:00"],     // HH:MM times
  "notifyOptions": ["OVERDUE", "TODAY"],     // what to remind about
  "weekDays": ["MO","TU","WE","TH","FR","SA","SU"],
  "holidayNotify": true
}
```

#### Pomodoro Preferences

```json
{
  "pomoDuration": 20,                        // focus duration (minutes)
  "shortBreakDuration": 5,
  "longBreakDuration": 15,
  "longBreakInterval": 3,                    // pomos before long break
  "pomoGoal": 4,                             // daily goal
  "focusDuration": 240,                      // daily focus goal (minutes)
  "autoPomo": false,                         // auto-start next pomo
  "autoBreak": false,                        // auto-start break
  "soundsOn": true,
  "lightsOn": true,
  "mindfulnessEnabled": true
}
```

#### Account Limits

`GET /configs/limits` returns limits for free, pro, and team tiers:

| Limit | Free | Pro | Team |
|-------|------|-----|------|
| `projectNumber` | 9 | 299 | 499 |
| `projectTaskNumber` | 99 | 999 | 2000 |
| `subtaskNumber` | 19 | 199 | 199 |
| `shareUserNumber` | 2 | 29 | 29 |
| `reminderNumber` | 2 | 5 | 5 |
| `habitNumber` | 5 | 299 | 299 |
| `kanbanNumber` | 19 | 19 | 19 |
| `attachmentSize` | 10MB | 20MB | 100MB |
| `timerNumber` | 3 | 49 | 49 |

### Misc

| Method | Path | Notes |
|--------|------|-------|
| GET | `/chatmind/bind/check` | AI assistant binding check |
| POST | `/push/register` | Register push notifications |
| POST | `/task/closeRemind` | Close/dismiss a reminder |

## v3 Sync API

Base: `https://api.ticktick.com/api/v3`

| Method | Path | Notes |
|--------|------|-------|
| GET | `/batch/check/0` | Full state sync (initial load) |
| GET | `/batch/check/{checkpoint}` | Incremental sync (poll for changes) |

The batch/check response contains the full account state:
- `syncTaskBean.update[]` — tasks
- `projectProfiles[]` — projects (includes `id`, `name`, `groupId`, `viewMode`, `kind`, `closed`)
- `projectGroups[]` — project groups/folders
- `tags[]` — tags
- `filters[]` — saved filters
- `columns.update[]` — kanban columns
- `inboxId` — inbox project ID
- `checkPoint` — monotonic timestamp for incremental sync

## v1 Misc

| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/v1/attachment/isUnderQuota` | Attachment storage quota check |
| GET | `/pub/api/v1/app/config?from=login` | App configuration |

## Other Hosts

| Method | Host | Path | Notes |
|--------|------|------|-------|
| POST | `ms.ticktick.com` | `/focus/batch/focusOp` | Focus session control (see below) |
| POST | `xapi.ticktick.com` | `/datacollect/event/push` | Analytics telemetry |
| POST | `s.ticktick.com` | `/api/3/envelope/` | Sentry error reporting |

---

## Focus / Pomodoro Control Protocol

All focus/pomodoro timer operations go through a single endpoint:

```
POST https://ms.ticktick.com/focus/batch/focusOp
```

### Request Format

```json
{
  "lastPoint": 1771401110003,       // last sync checkpoint (monotonic timestamp)
  "opList": [                        // list of operations to apply
    {
      "id": "{opId}",               // unique operation ID
      "oId": "{focusSessionId}",    // the focus session being operated on
      "oType": 0,                   // 0 = pomodoro
      "op": "start",                // operation type (see below)
      "duration": 20,               // pomo duration in minutes
      "firstFocusId": "{sessionId}",// ID of the first session in the cycle
      "focusOnId": "",              // task ID being focused on (empty = no task)
      "autoPomoLeft": 5,            // auto-pomo sessions remaining
      "pomoCount": 1,               // current pomo count in cycle
      "manual": true,               // user-initiated vs auto
      "note": "",                   // session note
      "time": "2026-02-18T07:51:40.942+0000"  // operation timestamp
    }
  ]
}
```

### Operations (`op` field)

| Op | Description | When |
|----|-------------|------|
| `start` | Start a new pomodoro session | User clicks Start |
| `pause` | Pause the running timer | User clicks Pause |
| `continue` | Resume a paused timer | User clicks Continue |
| `drop` | Give up / abandon the session (timer incomplete) | User quits early |
| `exit` | Exit the focus session entirely | Sent after `drop`, or after natural completion |

Notes:
- `drop` + `exit` are sent together when the user quits a session early
- When a pomo completes naturally, a `break` operation starts the break timer
- Multiple ops can be sent in a single `opList` batch

### Focus Types (`oType` field)

| oType | Mode | duration | Notes |
|-------|------|----------|-------|
| `0` | Pomodoro | `20` (or user setting) | Countdown timer, has `autoPomoLeft`, `pomoCount`, `firstFocusId` |
| `1` | Stopwatch | `0` | Count-up timer, `firstFocusId` is empty, `autoPomoLeft` and `pomoCount` are `0` |

Both modes use the same `op` values (`start`, `pause`, `continue`, `drop`, `exit`) and the same response format.

### Response Format

```json
{
  "point": 1771401182942,           // new sync checkpoint
  "current": {
    "id": "{sessionId}",
    "type": 0,                      // 0 = pomodoro
    "status": 0,                    // 0=running, 1=paused, 2=completed, 3=dropped
    "valid": true,
    "exited": false,                // true after exit op
    "firstId": "{sessionId}",
    "firstDid": "{deviceId}",
    "duration": 20,
    "startTime": "...",
    "endTime": "...",               // projected end time
    "autoPomoLeft": 5,
    "pomoCount": 1,
    "focusBreak": {},               // break timer state
    "focusOnLogs": [                // which task was being focused on
      { "id": "{taskId}", "time": "..." }
    ],
    "pauseLogs": [                  // pause/resume history
      { "type": 0, "time": "..." },  // 0=paused, 1=resumed
      { "type": 1, "time": "..." }
    ],
    "focusTasks": [                 // time segments per task
      { "id": "{taskId}", "startTime": "...", "endTime": "..." }
    ]
  }
}
```

### Status Codes

| Status | Meaning |
|--------|---------|
| 0 | Running |
| 1 | Paused |
| 2 | Completed (natural finish) |
| 3 | Dropped (user quit early) |

### Idle Poll

When no operation is needed, send an empty `opList` to poll for current state:

```json
{ "lastPoint": 1755701054962, "opList": [] }
```

---

## HAR File

The full HAR capture is at `ticktick.har` (430 requests, 325 with response bodies).
It contains session cookies — do not share publicly.
