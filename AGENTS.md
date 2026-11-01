# Instructions for AI Coding Agents

These instructions apply to the entire repository. Follow any more specific `AGENTS.md` found in a subdirectory as well. If instructions conflict, prefer the most specific applicable project guidance, while preserving the safety rules below.

## Before Making Changes

- Read the project README, contribution guidance, relevant source files, tests, build configuration, and recent history before editing.
- Identify the default branch and work on it directly. Do not create branches or pull requests unless explicitly requested.
- Check the current working tree and latest remote state before writing. Never overwrite unrelated or newer work.
- Understand the reported problem and reproduce it where practical. Prefer fixing root causes over symptoms.
- Preserve existing architecture, public interfaces, user data, compatibility, and project conventions unless a change is necessary and justified.
- Do not assume a language, framework, package manager, build system, or test command. Discover and use what this repository actually provides.

## Issue Triage and Implementation

- Prioritize existing, actionable open issues over speculative features or cosmetic churn.
- Prefer small, independently verifiable issues that can be completed confidently. Work on multiple independent issues during a maintenance session when practical, without mixing unrelated changes into one commit.
- Do not close an issue merely because code was changed. Check the issue's current acceptance criteria and close it only when every criterion that can be completed in scope is genuinely satisfied and verified.
- Use `Fixes #123` in a commit body only when that commit completes the issue and the issue should be closed. Use `Refs #123` for partial work; do not imply closure prematurely.
- Keep issues open when validation requires unavailable hardware, a user-specific environment, an interactive desktop session, credentials, or manual confirmation. Explain exactly what remains to be checked instead of claiming completion.
- Distinguish implementation from scaffolding: documentation, mock data, contract fakes, and test fixtures do not prove that a real backend, device, transfer, or GNOME integration works.
- Do not close user-created issues or mark work as not planned unless the repository owner explicitly directs you to do so.

## Code Quality and Testing

- Follow the repository's established formatting, naming, documentation, dependency, and error-handling conventions.
- Add or update focused tests for behavior changes when the project has a suitable test setup.
- Run relevant available tests, linters, formatters, type checks, and build checks. Do not add CI workflows or checks without a concrete project need.
- Inspect the complete output and exit status of every check. Fix failures caused by your changes; investigate pre-existing failures and report them accurately.
- Treat CI as a feedback loop, not a badge: after each commit, inspect its associated run and logs. If a check fails, diagnose the exact cause and fix it before treating the work as complete; do not leave known failures behind or stack dependent changes on an unexplained failure.
- At the end of each maintenance run, audit every commit created in that run and its associated CI/check result, not only the latest commit. Keep a commit-to-check list until all runs finish. Do not report pending checks as passing.
- Contract fakes and unit tests verify only the behavior they exercise. They do not prove real Valent/D-Bus behavior, network transfers, device compatibility, filesystem safety, or GNOME integration.
- Static checks do not prove runtime, graphical, hardware, or platform integration behavior. In particular, do not claim GNOME Shell/session compatibility without testing in a real GNOME session.
- If a necessary test cannot be run in the available environment, state the limitation clearly and leave validation-dependent work unresolved.

## Git and Commit Practices

- Commit directly to the repository's existing default branch; do not force-push, rewrite history, reset away others' work, or amend published commits.
- Fetch/review the latest branch state before committing. Keep changes focused and avoid including unrelated modifications.
- Use the repository's documented commit convention. If none exists, use clear Conventional Commit messages such as `fix(scope): ...`, `feat(scope): ...`, `docs: ...`, or `test: ...`.
- Include issue references for issue-related commits. Use the repository's `Fixes #N` / `Refs #N` distinction consistently. Do not create empty or misleading commits.
- After committing, verify the commit exists on the intended default branch, inspect the resulting diff/status, and follow its CI run to a final conclusion before reporting it as verified.

## Documentation and Dependencies

- Keep documentation accurate, concise, and consistent with the actual implementation.
- Do not duplicate information already maintained elsewhere or add lengthy documentation without a clear benefit.
- Avoid new dependencies when the existing stack can solve the problem. Justify any dependency or build-system change.
- Never invent supported platforms, installation commands, test results, performance claims, or feature status.

## Reporting

- Summarize concrete changes with file paths and commit identifiers.
- Distinguish verified results from assumptions and unrun checks.
- List remaining manual validation, blockers, and issues intentionally left open.
