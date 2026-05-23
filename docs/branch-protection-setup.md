# Branch Protection Setup

Status: active
Last reviewed: 2026-05-23

Use this when closing the `CI is required before merges` production-readiness
gate. This is an external repository setting, so it cannot be proven by a
source commit alone.

Official GitHub references:

- <https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches>
- <https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/managing-a-branch-protection-rule>
- <https://docs.github.com/en/rest/branches/branch-protection?apiVersion=2022-11-28>

## Required Rule

Repository:

```text
RobDavenport/framesmith
```

Branch name pattern:

```text
main
```

Required status check:

```text
Windows Checks
```

GitHub may display this as:

```text
CI / Windows Checks
```

Required settings:

- Require status checks to pass before merging.
- Require branches to be up to date before merging.
- Select the `Windows Checks` check from the `CI` workflow.
- Require a pull request before merging if `main` is a shared production
  branch.
- Do not allow bypassing the above settings, if available for the repository.
- Keep the default protected-branch behavior that blocks force pushes and
  branch deletion.

## Verification

Record evidence before marking the gate complete:

```text
Repository:
Protected branch or ruleset:
Branch pattern:
Required status checks:
Strict up-to-date requirement:
Pull request requirement:
Bypass policy:
Evidence URL or screenshot:
Blocked merge evidence:
Verified by:
Verified date:
```

Acceptable blocked-merge evidence:

- A pull request merge box showing that `Windows Checks` is required before
  merge.
- A branch protection or ruleset settings screen showing `Windows Checks` as a
  required check for `main`.
- An authenticated API response from the branch-protection endpoint showing
  `Windows Checks` in required status checks.

## Current Tooling Limit

The current repository connector can inspect repository metadata and pull
request checks, but it does not expose branch-protection or ruleset mutation.
This workspace also does not have GitHub CLI installed. Configure this setting
through GitHub's web UI or an authenticated admin API client, then paste the
evidence into the active release evidence document.
