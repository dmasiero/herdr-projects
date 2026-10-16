# Gitea

A repo whose remote is on a Gitea server gets its pull requests there, through `tea`, Gitea's command line, as a
repo on GitHub gets them through `gh`. Everything the board does with a pull request works the same: it pushes the
branch and opens a draft pull request after the check passes, reads its state, waits for its checks, merges it with
`merge.method` at the commit that was reviewed, and notices a pull request merged, closed or reviewed on the server.

## What it needs

- `tea` on the PATH (tea 0.14 or newer: the board uses `tea api`).
- A tea login for the server: `tea login add --url https://gitea.example.org`. `tea login list` shows them. The board
  uses the login whose address or SSH host is the host of the repo's remote `origin`, so the remote may be HTTPS or
  SSH (`ssh://git@gitea.example.org:2222/owner/repo.git`, `git@gitea.example.org:owner/repo.git`).
- Pushing goes through `git` itself, with the remote and the keys you have set up for it.

## Which forge a repo is on

`merge.forge` decides it, `auto` by default:

- `auto`: the host of `origin`. github.com is GitHub, and tea is never asked. A host tea has a login for is Gitea.
  Any other host is GitHub, as before (a GitHub Enterprise host keeps working through gh); where gh then finds no
  GitHub host, its error says the remote is elsewhere and how to reach it through tea.
- `gitea`: the repo is on Gitea. Without tea, or without a login for its host, opening or reading its pull request
  fails with that reason (`tea is not logged in to gitea.example.org …`), and the task waits for you as for a missing
  login.
- `github`: the repo is on GitHub, whatever its host.

Set it for one repo: `herdr-projects set merge.forge gitea --repo dotfiles`.

## How it differs from GitHub

- **A draft** is Gitea's work-in-progress title: the pull request opens as `WIP: <title>`, and the mark comes off the
  title right before it merges.
- **Its checks** (`merge.checks`, `github` by default, which on Gitea means the forge's checks) are the commit statuses
  on the pull request's head: those of Gitea Actions and of any other CI. Pending, the merge waits and says so
  (`Waiting for Gitea checks · 1 of 2 passed`), at most `check.timeout`; failed, nothing merges and the failure goes back
  to the agent with each failing status's name and the link to its log, within `review.rounds`; `warning` passes. A
  status the branch protection requires (by name; a pattern with `*` names none) is waited for until it is listed.
- **With no status yet**, the merge waits a few minutes for Gitea Actions to report one, only where the branch has
  workflows (`.gitea/workflows` or `.github/workflows`) and Actions are on for the repo. A repo with no checks
  configured never waits for one.
- **The review decision** is each reviewer's last say: one asking for changes blocks, one approving passes. Feedback
  wakes the agent as on GitHub, with `tea pr <n> --comments` to read it.
- Gitea has no merge queue, and the board never sets Gitea's own auto-merge: it merges by itself once the checks pass.
- **A pull request merged already** answers `HTTP 405: The PR is already merged` when merged again. The board reads that
  as merged: the task is done, or its newer work goes in a follow-up pull request ([a pull request merged
  already](follow-up.md)).
