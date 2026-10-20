# review-queue

A terminal UI that shows the GitHub pull requests that are waiting for your review, in priority order.

> **Status:** v1 is in development. This README describes v1. The `docs/roadmap.md` file gives the plan for v1.1 (snooze) and v1.2 (graphics).

## Purpose

GitHub notifications are not a good list of the reviews that you must do. A notification stays after the review request is complete, and it goes out of view when you read it on a different device.

`review-queue` uses the current state of each open pull request. A pull request is in the queue only while GitHub lists you, or a team on your allowlist, as a requested reviewer. When you submit your review, or when the author removes the request, the pull request goes out of the queue at the next poll.

The tool sorts the queue so that the most important review is at the top. For the terms that this README uses, refer to `CONTEXT.md`.

## Requirements

- A Rust toolchain (stable). Install it with [rustup](https://rustup.rs).
- The GitHub CLI, [`gh`](https://cli.github.com), installed and logged in to github.com:

  ```sh
  gh auth login
  ```

  The tool gets its token from `gh auth token`. If you set the `GITHUB_TOKEN` environment variable, the tool uses that token and does not use `gh`. The token must have the `repo` and `read:org` scopes.

- github.com only. The tool does not support GitHub Enterprise Server.

## Install

```sh
git clone <repository-url> review-queue
cd review-queue
cargo install --path .
```

This puts the `review-queue` binary in `~/.cargo/bin`. Make sure that this directory is on your `PATH`.

## Build

```sh
cargo build            # debug build
cargo build --release  # release build, in target/release/review-queue
```

## Test

```sh
cargo test
```

The priority logic is a pure function. Its tests do not use the network or the terminal.

## Launch

```sh
review-queue
```

Options:

| Option | Description |
|---|---|
| `--interval <SECONDS>` | The time between two polls. The default is 60. This overrides the config file. |

## Configuration

The config file is `~/.config/review-queue/config.toml` (or `$XDG_CONFIG_HOME/review-queue/config.toml`, if `XDG_CONFIG_HOME` is set). The file is optional. If it is not there, the tool uses the defaults.

```toml
# The time between two polls, in seconds.
interval_secs = 60

# The teams that count for your queue, as "org/team-slug".
# Requests to teams that are not in this list are not shown, even when you are a member.
# If the list is empty, the queue shows only requests to you personally.
watched_teams = ["acme/platform"]
```

## Help

Push `?` in the tool to see this help page. The help page also shows your current config.

### Actions

| Key | Action |
|---|---|
| `j` or `↓` | Move the selection down |
| `k` or `↑` | Move the selection up |
| `Enter` | Open the selected pull request in the browser |
| `r` | Refresh the queue now |
| `?` | Show or close the help page and the current config |
| `q` | Quit |

### How the queue works

**What goes in the queue.** A pull request is in the queue when all of these conditions are true:

- The pull request is open.
- GitHub lists you, or one of your watched teams, as a requested reviewer.
- You are not the author of the pull request.

**Priority tiers.** The queue has four tiers. The tool shows them in this order:

1. **Personal re-review**: GitHub requests your review, and you submitted a review on this pull request before.
2. **Personal first review**: GitHub requests your review, and you did not submit a review on this pull request before.
3. **Team re-review**: GitHub requests the review of a watched team, and you submitted a review on this pull request before.
4. **Team first review**: GitHub requests the review of a watched team, and you did not submit a review on this pull request before.

A review in any state (approved, changes requested, commented, or dismissed) counts as a prior review. A comment without a review does not count. If a pull request requests both you and one of your teams, it is a personal request.

**Order in a tier.** In each tier, the request that has waited longest is at the top. The wait time starts at the latest request event, so a re-request starts the wait time again. Pull requests from bot accounts (for example, Dependabot or Renovate) come after all other pull requests in the same tier.

**Rows.** Each row shows the repository and the pull request number, the title, the author, the diff size (`+added −deleted`), and the wait time. A team request also shows the team. A draft pull request has a `draft` tag. A draft stays in its usual tier.

**Polling.** The tool asks GitHub for the queue at each interval (60 seconds by default). Push `r` to refresh immediately. The status bar shows the time since the last good update. If a poll fails, the tool keeps the last good queue on the screen and shows the error in the status bar.

**No local state.** In v1, the queue shows only the state on GitHub. To remove a pull request from the queue, submit your review or ask the author to remove the request.
