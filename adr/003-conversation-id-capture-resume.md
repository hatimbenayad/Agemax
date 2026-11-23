# ADR-003: Agent Conversation ID Capture and Session Resume

## Status
Accepted (Validated by Spike S3)

## Context
Age Max must support session persistence and one-click resumption of agent conversations across multiplexer restarts. To restore a conversation, Age Max must reliably associate each pane with its active Antigravity conversation ID.

## Empirical Findings (Spike S3)
1. Official documentation in `.gemini/antigravity/builtin/skills/agy-customizations/docs/hooks.md` confirms that `hooks.json` provides lifecycle hooks (`SessionStart`, `PreToolUse`, `PostToolUse`, `Stop`).
2. Every hook payload delivered to `stdin` includes a standardized JSON object:
   ```json
   {
     "conversationId": "...",
     "workspacePaths": ["..."],
     "cwd": "..."
   }
   ```
3. `agy` persists conversations to `<app-data-dir>\conversations\<conversation_id>.db` and records session references in `conversation_summaries.db` / `cache\last_conversations.json`.
4. The official resumption flag is `agy --conversation <id>` (globally resumable from any working directory) or `agy -c` (resumes most recent conversation in the current working directory).

## Decision
Implement a three-tier capture and resumption strategy:
1. **Tier 1 (Deterministic Turn-0 Capture)**:
   - Age Max injects or maintains an internal hook in the workspace/profile (`.agents/hooks.json` or profile hooks) for `SessionStart`.
   - The hook invokes a lightweight notification or IPC ping to Age Max associating the process PID with the `conversationId`.
2. **Tier 2 (Database / Cache Fallback)**:
   - If hooks are bypassed, query `conversation_summaries.db` or read `cache\last_conversations.json` matching the pane's working directory.
3. **Tier 3 (Filesystem Metadata Fallback)**:
   - Locate the most recently modified `.db` file in `<app-data-dir>\conversations\`.
4. **Session Resumption**:
   - Persist pane state to `%LOCALAPPDATA%\AgeMax\session.json`.
   - On relaunch, reconstruct the workspace/tab/pane layout and offer a one-click "Resume Agent" action that launches `agy --conversation <id>`.
   - Never auto-execute untrusted commands from persisted files without user intent.

## Consequences
- Guaranteed conversation ID resolution across restarts.
- Clean separation between terminal layout restoration and agent resumption.
