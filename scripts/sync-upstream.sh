#!/bin/sh
# The fork's sync with upstream (FORK.md): merges upstream's main into a `sync/upstream-<version>` branch made from the
# fork's main, in a worktree of its own, runs the whole check on this machine (and, where Nix is, the Nix build and the
# whole check in a NixOS VM), and opens a pull request to the fork's main with what upstream changed and which files conflicted. By hand or by an agent:
#   sh scripts/sync-upstream.sh              fetch, merge, check, push the branch, open the pull request
#   sh scripts/sync-upstream.sh --no-pr      the same, but push and open nothing
#   sh scripts/sync-upstream.sh --continue   after resolving conflicts in the worktree it named: commit, check, open
# Options: --ref <upstream ref> (default upstream/main), --base <fork ref> (default origin/main), --dir <worktree
# folder>, --no-nix.
# Exit: 0 done or nothing new, 1 an error, 3 conflicts to resolve, 4 the check failed. Never pushes to upstream.
set -e
say() { printf 'sync-upstream: %s\n' "$*" >&2; }
die() { say "$1"; exit "${2:-1}"; }
UPSTREAM_URL="https://github.com/eliasstravik/herdr-projects.git"
ref=upstream/main; base=origin/main; dir=""; pr=1; nix=1; cont=0
while [ $# -gt 0 ]; do
  case "$1" in
    --ref) ref="$2"; shift ;;
    --dir) dir="$2"; shift ;;
    --base) base="$2"; shift ;;
    --no-pr) pr=0 ;;
    --no-nix) nix=0 ;;
    --continue) cont=1 ;;
    -h|--help) sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) die "unknown option $1 (see --help)" ;;
  esac
  shift
done

here=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$here"
git remote get-url origin >/dev/null 2>&1 || die "this checkout has no remote origin (the fork)"
git remote get-url upstream >/dev/null 2>&1 || { say "adding the remote upstream: $UPSTREAM_URL"; git remote add upstream "$UPSTREAM_URL"; }
slug() { git remote get-url "$1" | sed -E 's#^(git@|ssh://git@|https://)([^/:]+)[:/]##; s#\.git$##; s#/$##'; }
fork=$(slug origin); up=$(slug upstream)
[ "$fork" != "$up" ] || die "origin and upstream are the same repo ($fork): this script only ever pushes to the fork"

state_file() { printf '%s/sync-upstream' "$(git -C "$1" rev-parse --absolute-git-dir)"; }
version_at() { git show "$1:Cargo.toml" | sed -n 's/^version *= *"\([^"]*\)".*/\1/p' | head -n 1; }

if [ "$cont" = 0 ]; then
  say "fetching origin and upstream"
  git fetch --quiet origin
  git fetch --quiet --tags upstream
  git rev-parse --verify --quiet "$ref^{commit}" >/dev/null || die "no such upstream ref: $ref"
  git rev-parse --verify --quiet "$base^{commit}" >/dev/null || die "no such fork ref: $base"
  if git merge-base --is-ancestor "$ref" "$base"; then say "the fork's main already has $ref ($(git rev-parse --short "$ref")): nothing to do"; exit 0; fi
  new=$(version_at "$ref"); old=$(version_at "$base")
  branch="sync/upstream-$new"
  if [ "$new" = "$old" ] || git rev-parse --verify --quiet "refs/heads/$branch" >/dev/null || git rev-parse --verify --quiet "refs/remotes/origin/$branch" >/dev/null; then
    branch="sync/upstream-$new-$(date +%Y-%m-%d)"
  fi
  git rev-parse --verify --quiet "refs/heads/$branch" >/dev/null && die "the branch $branch is there already: finish it (--continue --dir …) or delete it"
  [ -n "$dir" ] || dir="${TMPDIR:-/tmp}/herdr-projects-$(printf '%s' "$branch" | tr '/' '-')"
  [ ! -e "$dir" ] || die "$dir is there already: remove it, or name another with --dir"
  git worktree add --quiet -b "$branch" "$dir" "$base"
  n=$(git rev-list --count --no-merges "$base..$ref")
  say "merging $ref ($new, $n commits) into $branch in $dir"
  cd "$dir"
  st=$(state_file .)
  { echo "ref=$(git rev-parse "$ref")"; echo "base=$(git rev-parse "$base")"; echo "new=$new"; echo "old=$old"; echo "n=$n"; } > "$st"
  if ! git merge --no-ff --no-edit -m "Merge upstream $new ($n commits) into the fork's main" "$ref" >/dev/null 2>&1; then
    git diff --name-only --diff-filter=U > "$st.conflicts"
    say "conflicts in:"; sed 's/^/  /' "$st.conflicts" >&2
    say "resolve them in $dir (keep upstream's code and put the fork's change back on top: FORK.md, 'Conflicts'),"
    say "then run: sh scripts/sync-upstream.sh --continue --dir $dir"
    exit 3
  fi
  : > "$st.conflicts"
else
  [ -n "$dir" ] || die "--continue needs the worktree's --dir"
  cd "$dir"
  st=$(state_file .)
  [ -f "$st" ] || die "$dir has no sync in progress"
  [ -z "$(git diff --name-only --diff-filter=U)" ] || die "$dir still has conflicts: $(git diff --name-only --diff-filter=U | tr '\n' ' ')" 3
  if git rev-parse --verify --quiet MERGE_HEAD >/dev/null; then git add -A && git commit --quiet --no-edit; fi
fi
branch=$(git rev-parse --abbrev-ref HEAD)
. "$st"
case "$branch" in sync/upstream-*) ;; *) die "$dir is on $branch, not a sync/upstream-* branch" ;; esac

# A new upstream version starts the fork's own number again: 1.2.3+masiero.1.
part=$(cat FORK_VERSION)
if [ "$new" != "$old" ] && [ "${part##*.}" != 1 ]; then
  printf '%s.1' "${part%.*}" > FORK_VERSION
  git commit --quiet -m "Fork version $new+$(cat FORK_VERSION)" FORK_VERSION
fi

# The whole check: every test, the fork's Gitea (tea faked) and NixOS ones among them, on the paths of this machine
# (NixOS's on NixOS, plain Linux's elsewhere). The other system's run is the NixOS VM below, or CI's.
say "running the whole check (npm run check -- --all)"
log="$st.check.log"
if ! npm run check -- --all > "$log" 2>&1; then tail -n 40 "$log" >&2; die "the check failed: fix it in $dir, commit, then --continue (log: $log)" 4; fi
checked=$(grep -E '^(Tests:|The check)' "$log" | tr '\n' ' ')
if [ -n "$(git status --porcelain)" ]; then git add -A && git commit --quiet -m "Write the made tables again after the upstream merge"; fi
nixsaid="not run (--no-nix)"
if [ "$nix" = 1 ] && command -v nix >/dev/null 2>&1; then
  say "nix flake check and nix build"
  nix flake check > "$st.nix.log" 2>&1 && nix build --no-link >> "$st.nix.log" 2>&1 || { tail -n 30 "$st.nix.log" >&2; die "the Nix build failed (log: $st.nix.log)" 4; }
  nixsaid="nix flake check and nix build passed"
  if [ -w /dev/kvm ] && [ "$(uname -s)" = Linux ]; then
    say "the whole check on NixOS, in a VM (nix build .#nixos-check)"
    nix build --no-link -L .#nixos-check >> "$st.nix.log" 2>&1 || { tail -n 40 "$st.nix.log" >&2; die "the check in the NixOS VM failed (log: $st.nix.log)" 4; }
    nixsaid="$nixsaid; the whole check in a NixOS VM passed"
  else nixsaid="$nixsaid; the NixOS VM check not run here (no KVM): CI's NixOS job runs it"; fi
elif [ "$nix" = 1 ]; then nixsaid="not run: no nix on this machine (CI's Nix and NixOS jobs run them)"; fi
[ -f /etc/NIXOS ] && where="NixOS ($(uname -m))" || where="$(uname -s) $(uname -m)"

body="$st.pr.md"
{
  echo "Merges upstream ($up) \`$(git rev-parse --short "$ref")\` into the fork's main: **$old → $new**, $n commits."
  echo
  echo "**Merge this pull request with \"Create a merge commit\"**, never squash or rebase: the next sync then sees these"
  echo "commits as merged (FORK.md)."
  echo
  echo "## Upstream's changes"
  echo
  git log --no-merges --format='- %s (`%h`)' "$base..$ref"
  echo
  echo "## Conflicts"
  echo
  if [ -s "$st.conflicts" ]; then sed 's/^/- `/; s/$/`/' "$st.conflicts"; echo; echo "Resolved by hand: see the merge commit's diff against upstream (\`git diff $ref HEAD -- <file>\`)."; else echo "None."; fi
  echo
  echo "## Checked on $where"
  echo
  echo "- \`npm run check -- --all\` on this machine: $checked"
  echo "- Nix: $nixsaid"
  echo "- CI on this pull request: the whole check on Ubuntu (Check), the fork's tests, Nix on Ubuntu and the whole"
  echo "  check in a NixOS VM (Fork). Merge only when both workflows pass."
  echo
  echo "## The fork's layer after the merge"
  echo
  echo '```'
  git diff --stat "$ref" HEAD | tail -n 1
  echo '```'
} > "$body"

if [ "$pr" = 0 ]; then say "done, nothing pushed (--no-pr). Branch $branch in $dir; the pull request's text: $body"; cat "$body"; exit 0; fi
command -v gh >/dev/null 2>&1 || die "gh is not installed: push $branch and open the pull request by hand (text: $body)"
say "pushing $branch to origin ($fork) and opening the pull request"
git push --quiet -u origin "$branch"
gh pr create --repo "$fork" --base main --head "$branch" --title "Sync with upstream $new" --body-file "$body"
