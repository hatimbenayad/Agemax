# ADR-004: Official Quota Visibility Architecture

## Status
Accepted (Validated by Spike S4)

## Context
Antigravity accounts have usage quotas across model families (Gemini models, 3P models like Claude and GPT). Users want visibility into their remaining quota to plan agent workloads without opening external web portals. We needed to verify if quota can be fetched using official, non-intrusive surfaces.

## Empirical Findings (Spike S4)
1. Executing `agy.exe -p "/usage" --output-format json` prints structured JSON data without prompting:
   - Returns structured `groups` for "Gemini Models" and "Claude and GPT models".
   - Provides exact fields: `window` ("weekly", "5h"), `remaining_fraction` (float between 0.0 and 1.0), and `reset_time` (ISO 8601 UTC timestamp).
   - Execution duration: ~3.0 seconds, consuming 0 prompt tokens (`total_tokens: 0`).
   - Clean exit code 0.

## Decision
1. Implement a background quota service in `agemax-agy` that queries `agy -p "/usage" --output-format json` on a throttled schedule (e.g., every 15 minutes, or upon manual user request in the sidebar/statusline).
2. The query runs with the environment (`USERPROFILE`, `HOME`, `AGY_ACCOUNT`) configured for the active profile.
3. UI displays a clean, minimal quota bar in the sidebar or workspace footer:
   - Gemini: [Weekly: 81% | 5-Hour: 90%]
   - Claude/GPT: [Weekly: 100% | 5-Hour: 100%]
4. If the command fails or if the user is offline, the UI displays the last cached value or gracefully hides the widget without blocking the application.

## Consequences
- Fast, accurate quota monitoring using 100% official CLI interfaces.
- Zero credential handling, zero API reverse-engineering, zero token cost.
