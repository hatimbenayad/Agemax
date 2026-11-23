# ADR-001: Windows Account Isolation and Profile Architecture

## Status
Accepted (Validated by Spike S1)

## Context
Age Max needs to host multiple Antigravity agent sessions. Some users want to run agents across different accounts (e.g., personal vs work) for quota or identity separation. On Linux, community tools isolate accounts by redirecting `HOME` and `USERPROFILE`. However, on Windows, authentication mechanisms may interact with the OS-level Windows Credential Manager.

## Empirical Findings (Spike S1)
1. When running `agy.exe` with redirected `HOME` and `USERPROFILE` to a completely isolated folder, `agy` initialized `.gemini/antigravity-cli`, but immediately authenticated successfully without user interaction:
   `auth.go:157] ChainedAuth: authenticated via keyring (effective: keyring)`
   `server_oauth.go:226] OAuth: authenticated successfully as hatimbenayad8@gmail.com`
2. Inspection of Windows Credential Manager revealed a generic credential target:
   `LegacyGeneric:target=gemini:antigravity`
3. Because Windows Credential Manager is scoped to the current Windows logon session (user SID) and not the directory tree, any process running under that Windows user querying `keyring` retrieves the same default credential.
4. Binary string inspection revealed that `agy` natively supports the `AGY_ACCOUNT` environment variable, which selects active identities defined in `%USERPROFILE%/.gemini/google_accounts.json`.

## Decision
1. **Mode A (Default, Day 1)**: Primary sign-in mode. Unlimited concurrent agent sessions run in parallel under the user's default sign-in. Zero credential handling by Age Max.
2. **Mode B (Profiles)**:
   - For users with multiple registered accounts in `google_accounts.json`, Age Max allows setting `AGY_ACCOUNT=<account_id>` per profile/pane.
   - For fully isolated filesystem state (settings, plugins, conversations), Age Max sets `USERPROFILE` and `HOME` to `%LOCALAPPDATA%\AgeMax\profiles\<name>\`.
   - In accordance with the hard security constraint, **Age Max never reads, parses, stores, or proxies token files or Credential Manager entries**. Sign-in is performed interactively by the user inside an official `agy` terminal pane.

## Consequences
- Single-account users experience seamless multi-session multiplexing with zero setup.
- Multi-account users utilize official `AGY_ACCOUNT` selection or isolated profiles without compromising security or violating the zero-credential boundary.
