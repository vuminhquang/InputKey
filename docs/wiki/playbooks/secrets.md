# Repository secrets and privacy

This is a public repository: no secret value, access token, private workspace record, credential dump, signing material or account-only operational metadata may enter tracked files, generated artifacts, documentation, commit messages or logs.

Use an ignored local vault for any needed credential values; keep only purpose-based names and usage guidance in publicly readable procedures. Inject only the minimal target-specific credentials directly into a child process when required, never into a machine-wide environment or printed command output. Do not use a bare credential-read command whose stdout reveals the value. Keep local secret locations ignored before creating material.

Before committing, inspect staged diff and scan for accidental secrets; before any release inspect the output artifact independently. A leaked secret requires containment, rotation and history evaluation, not merely deletion in a later commit. A published repository's history is part of its disclosure surface. The browser typing engine uses packaged local WASM rather than remote typing services; preserve this privacy property.
