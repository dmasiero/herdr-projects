# Gitea

**A repo on a Gitea server gets its pull requests there, through `tea`**, Gitea's command line, as a repo on GitHub
gets them through `gh`. Everything the board does with a pull request works the same: it pushes the branch, opens a
draft pull request after the check passes, reads its state, waits for its checks, merges it with `merge.method` at the
commit that was reviewed, and notices a pull request merged, closed or reviewed on the server. A task started from a
named branch (`workspace.from develop`) opens its pull request into that branch, as on GitHub.

## What you need

- `tea` 0.14 or newer on the PATH (the board uses `tea api`).
- A tea login for the server: `tea login add --url https://gitea.example.org`. `tea login list` shows them. The board
  uses the login whose address or SSH host is the host of the remote `origin`, so the remote may be HTTPS or SSH
  (`ssh://git@gitea.example.org:2222/owner/repo.git`, `git@gitea.example.org:owner/repo.git`).
- Pushing goes through `git` itself, with the remote and keys you already use.

## Which forge a repo is on

`merge.forge` decides, `auto` by default:

| `merge.forge` | The repo is on |
|---|---|
| `auto` | worked out from the host of `origin`. github.com is GitHub, and tea is never asked. A host tea has a login for is Gitea. Any other host is GitHub, as before (GitHub Enterprise keeps working through gh); where gh then finds no GitHub host, its error says the remote is elsewhere and how to reach it through tea |
| `gitea` | Gitea. Without tea, or without a login for its host, opening or reading its pull request fails with that reason (`tea is not logged in to gitea.example.org …`), and the task waits for you as for a missing login |
| `github` | GitHub, whatever its host |

For one repo: `herdr-projects set merge.forge gitea --repo dotfiles`.

## How it differs from GitHub

- **A draft is a `WIP:` title**, as Gitea has it. The pull request opens as `WIP: <title>`, and the mark comes off right
  before it merges.
- **Its checks are the commit statuses on the pull request's head**: those of Gitea Actions and of any other CI
  (`merge.checks: github`, the default, means the forge's checks). Pending, the merge waits and says so
  (`Waiting for Gitea checks · 1 of 2 passed`), at most `check.timeout`. Failed, nothing merges and the failure goes
  back to the agent with each failing status's name and the link to its log, within `review.rounds`. `warning` passes.
  A status the branch protection requires (by name; a pattern with `*` names none) is waited for until it is listed.
- **With no status yet**, the merge waits a few minutes for Gitea Actions to report one, only where the branch has
  workflows (`.gitea/workflows` or `.github/workflows`) and Actions are on for the repo. A repo with no checks never
  waits for one.
- **The review decision is each reviewer's last say**: one asking for changes blocks, one approving passes. Feedback
  wakes the agent as on GitHub, with `tea pr <n> --comments` to read it.
- **No merge queue, no auto-merge.** The board never sets Gitea's own auto-merge: it merges by itself once the checks
  pass.
