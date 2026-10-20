# Roadmap

## v1

- The Queue mirrors GitHub. It keeps no local state.
- Actions: move the selection, open the pull request in the browser.
- Host: github.com only. No GitHub Enterprise Server.
- Authentication: the token from `gh auth token`. If `GITHUB_TOKEN` is set, it overrides `gh`. If neither gives a token, stop and tell the user to run `gh auth login`.
- Polling: one GraphQL request every 60 seconds by default (configurable). The `r` key refreshes now. The status bar shows the time since the last good update. If a poll fails, keep the last good Queue on the screen and show the error in the status bar.
- Layout: the Queue is grouped under one header for each Priority Tier, with a count. Empty tiers stay on the screen. Each row shows the repository and pull request number, the title, the author, the diff size (+added −deleted), and the Wait Time. A Team Request also shows the Watched Team. A draft pull request shows a `draft` tag, and stays in its normal Priority Tier.
- No bell and no desktop notification when a Review Request arrives.
- The user's own pull requests never go in the Queue, even when a Watched Team is requested.
- The `?` key shows a help page: all actions and their keys, a short description of how the Queue works, and below that, the current config.
- Stack: `ratatui` + `crossterm`, `tokio`, `graphql_client` (with the GitHub schema checked in), `reqwest` (rustls), `serde` + TOML config at `~/.config/review-queue/config.toml`, `jiff` for time.
- The priority logic is a pure function: GitHub pull request data and the current time go in, the ordered Queue comes out. It has no network or TUI dependencies.

## v1.1

- **Snooze** a Review Request until one of these:
  - a delay (for example, 2 hours)
  - a named time boundary (for example, "tomorrow morning" or "before close of business Friday")
- There is no permanent dismissal. The "dismiss" action is a Snooze with a default delay, so the request comes back if it is still pending on GitHub.
- A Snooze ends early when either of these occurs after the Snooze starts:
  - a new request event for the user or a Watched Team (a re-request)
  - a new comment on the pull request that @-mentions the user personally (a team mention does not count)
- Other activity, for example new commits, does not end a Snooze early.
- Working time: the config file sets working hours (default 09:00–17:30) and working days (default Mon–Fri). The tool uses the system local time zone.
- Snooze presets:
  - `1h`, `3h`: wall-clock delays.
  - `Next morning`: the start of the next working day.
  - `Before COB <day>`: 2 hours before the end of that working day.
  - `Custom…`: a typed duration, for example `2d` or `45m`.
- Dismiss is a Snooze of 1 working day. It ends at the same time on the next working day.
- Snoozes are kept in `~/.local/state/review-queue/snoozes.json`, with one Snooze for each pull request.
- The status bar shows the number of snoozed requests. The `z` key shows the snoozed view: snoozed requests in their Priority Tiers, dimmed, with the end time of each Snooze. In that view, `u` ends the Snooze of the selected request, and `s` changes its end time.
- The `s` key opens the snooze picker. The `d` key dismisses (a Snooze of 1 working day).
- `Next morning` is the next start of a working day after now. At 07:00 on a working day, it is 09:00 the same day.
- A personal @-mention is `@<login>` in the text of a comment or review, with no letter, digit, `-`, or `/` after it. A team mention (`@org/team`) does not count.

## v1.2 (delight)

Graphics use the Kitty graphics protocol (Ghostty supports it). If the terminal has no graphics protocol, the tool uses Unicode half-blocks. A `--no-images` flag turns graphics off.

- **Author avatars** in each row of the Queue. The tool caches them on disk.
- **Detail panel**: a large avatar and a summary of the selected pull request.
- **Empty-Queue reward**: an image, which can be animated, when the Queue is empty. For example, a random image from a folder that the user chooses.
- **Arrival effect**: a short animation (about 1 second) on a row that just came into the Queue. This is not a bell.
- No animated avatars.
- The tool drives the animation: it uploads each frame once with its own image ID, and changes the placed image ID on each tick. It does not use the terminal-side animation commands of the protocol.
