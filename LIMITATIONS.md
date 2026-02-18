# Known Limitations

Features available in the TickTick web/mobile app that are not yet implemented in this CLI.

| Feature | Description | Issue |
|---------|-------------|-------|
| Task uncomplete / reopen | No way to un-complete a task | [#5](https://github.com/karbassi/ticktick-cli/issues/5) |
| Task restore from trash | `task trash` lists items but can't restore them | [#6](https://github.com/karbassi/ticktick-cli/issues/6) |
| Kanban columns | Can't list/create/reorder columns or assign tasks to columns | [#7](https://github.com/karbassi/ticktick-cli/issues/7) |
| Task progress | The `progress` field (0-100%) is not exposed | [#8](https://github.com/karbassi/ticktick-cli/issues/8) |
| Archived projects | No `project list --archived` or `project archive` | [#9](https://github.com/karbassi/ticktick-cli/issues/9) |
| Settings write | `settings` is read-only; updating preferences not supported | [#10](https://github.com/karbassi/ticktick-cli/issues/10) |
| Habit unarchive | `habit archive` exists but no reverse command | [#11](https://github.com/karbassi/ticktick-cli/issues/11) |
| Pomodoro preferences | Can't read/write session duration, break length, daily goal | [#12](https://github.com/karbassi/ticktick-cli/issues/12) |
| Notifications | No command to check unread notifications | [#13](https://github.com/karbassi/ticktick-cli/issues/13) |
| Templates | Can't list or apply task/project templates | [#14](https://github.com/karbassi/ticktick-cli/issues/14) |
| Incremental sync | `sync` always fetches full state; no checkpoint-based polling | [#15](https://github.com/karbassi/ticktick-cli/issues/15) |
| Reminder dismiss | No command to dismiss active reminders | [#16](https://github.com/karbassi/ticktick-cli/issues/16) |
| Account limits | Can't check free/pro tier constraints | [#17](https://github.com/karbassi/ticktick-cli/issues/17) |
