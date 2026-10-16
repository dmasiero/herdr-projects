# A pull request merged already

Sometimes the board goes to merge a task's pull request and finds it merged already: someone merged it on GitHub or
Gitea, or the board merged it and the work went on after that.

The work can go on after a merge when you keep talking to a done task's agent in its pane
([chatting with a finished task](how-work-gets-done.md#chatting-with-a-finished-task-re-reviews-only-what-changed)). If
the agent then commits more, those commits are on a branch whose pull request is closed: pushing them there brings
nothing into main. This is what happened on OPS-3: four files of work were left out, and Gitea refused the merge with
`HTTP 405: The PR is already merged`.

## What the board does

Before it merges, the board asks GitHub or Gitea for the pull request's state. A refused merge that says "already
merged" (Gitea's HTTP 405, gh's `was already merged`) counts as merged too, not as an unknown error. Then the board
compares the task's branch with the branch it merged into:

- **Nothing new on the branch.** The task is done with that merge. Its row says `Already merged: <the pull request>`,
  and the merge commit is kept as the task's.
- **Newer work on the branch.** The board opens a **follow-up pull request** from the same branch into the same base.
  Its title is the task's with `(follow-up)` at the end. Its description links the first pull request and says why it
  was opened. The follow-up becomes the task's pull request, and the task goes through the finish line again: check,
  review where it is set up, then approval under `merge.by`. The approval you gave was for the first pull request, so
  it does not carry over.

The finish line does the same when the board already knows the pull request is merged: after a chat with a done task,
the new work goes in a follow-up instead of being pushed to the merged pull request. The earlier pull requests are kept
in the task's record (`prEarlier`).

It works the same way on GitHub (through `gh`) and on Gitea (through `tea`, [Gitea](gitea.md)).
