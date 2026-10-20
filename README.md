# review-queue

A terminal UI that shows the GitHub pull requests that are waiting for your review, in priority order.

> **Status:** v1 (the queue), v1.1 (snooze), and v1.2 (graphics) are complete. The `docs/roadmap.md` file records the decisions.

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

The filtering and priority rules are also tested with a corpus of synthetic GitHub API responses in `tests/corpus/`. Each scenario is a directory with:

- `response.json`: a synthetic response to the query in `graphql/review_requests.graphql`
- `config.toml`: the config for the scenario (optional)
- `expected.txt`: the expected queue, one item per line: `<tier> <repo>#<number> [<team>]`

To add a case, add a directory. You do not write code. Write `expected.txt` by hand from the rules, not from the output of the tool. Do not put real API responses in the corpus.

## Launch

```sh
review-queue
```

Options:

| Option | Description |
|---|---|
| `--interval <SECONDS>` | The time between two polls. The default is 60. This overrides the config file. |
| `--no-images` | Do not use raster graphics. |

## Configuration

The config file is `~/.config/review-queue/config.toml` (or `$XDG_CONFIG_HOME/review-queue/config.toml`, if `XDG_CONFIG_HOME` is set). The file is optional. If it is not there, the tool uses the defaults.

```toml
# The time between two polls, in seconds.
interval_secs = 60

# The teams that count for your queue, as "org/team-slug".
# Requests to teams that are not in this list are not shown, even when you are a member.
# If the list is empty, the queue shows only requests to you personally.
watched_teams = ["acme/platform"]

# Your working time, for the snooze choices. The tool uses the local time zone of your computer.
work_start = "09:00"
work_end = "17:30"
work_days = ["Mon", "Tue", "Wed", "Thu", "Fri"]
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
| `s` | Snooze the selected pull request. In the snoozed view, change the end time of its snooze |
| `d` | Dismiss: snooze the selected pull request for 1 working day |
| `z` | Show or close the snoozed view |
| `u` | In the snoozed view: end the snooze of the selected pull request |
| `a` | Preview the empty-queue image (`a` or `esc` closes it) |
| `tab` | Move the focus to the detail panel, or back to the list |
| `j`/`k`, `pgdn`/`pgup`, `space` | With the detail panel focused: scroll the description by 1 or 10 lines |
| `?` | Show or close the help page and the current config |
| `esc` | Close the help page, the snoozed view, or the snooze picker, or give the focus back to the list |
| `q` | Quit |

In the snooze picker, push `1` to `9` to choose:

| Choice | Snooze ends |
|---|---|
| 1 hour, 3 hours | After that time |
| Next morning | At the next start of a working day. Before working hours, this is the same day |
| Before COB *day* | 2 hours before the end of the working day, on each of the next 5 working days |
| Custom… | After a duration that you type, for example `45m`, `3h`, `2d`, or `1h30m` |

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

**Removing an item.** To remove a pull request from the queue, submit your review, or ask the author to remove the request.

**Detail panel.** In a terminal with 24 rows or more for the queue, a panel below the list shows the selected pull request: the author's avatar, the full title, the tier, the wait time, the snooze end, the branches, the number of changed files, the CI status of the last commit, the labels, and the description as plain text. With raster graphics, each row also starts with the author's avatar. Avatars are cached in `~/.cache/review-queue/avatars/` (or `$XDG_CACHE_HOME/review-queue/avatars/`) and downloaded again after 7 days.

**New arrivals.** When a pull request comes into the queue at a poll (for example, a new request, or a snooze that ended), its row flashes and fades back to normal in 1 second.

**Empty queue.** When nothing waits for your review, the pane shows a hand-drawn star with a rainbow trail: "there was an attempt … to keep on top of PR reviews". With raster graphics (for example, Ghostty), the image is drawn at the size of the pane. In other terminals, and with `--no-images`, it is colored text art. Push `a` to preview it at any time. To iterate on the image, render it to a file: `cargo run --example attempt_png -- 1200 520 out.png`.

**Snooze.** A snooze hides a pull request until a time that you choose. Nothing is hidden forever: dismiss (`d`) is a snooze of 1 working day, at the same time on the next working day. A snooze ends early when either of these occurs after the snooze started:

- The author requests your review again (for you or for a watched team).
- A comment or review mentions you personally (`@your-login`). A mention of a team does not end a snooze.

The status bar shows the number of snoozed pull requests. Push `z` to see them, with the end time of each snooze. The snoozes are kept in `~/.local/state/review-queue/snoozes.json` (or `$XDG_STATE_HOME/review-queue/snoozes.json`). When a pull request leaves the queue, its snooze is deleted.
