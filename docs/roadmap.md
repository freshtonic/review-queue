# Roadmap

## v1 (complete)

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

## v1.1 (complete)

- **Snooze** a Review Request until one of these:
  - a delay (for example, 2 hours)
  - a named time boundary (for example, "tomorrow morning" or "before close of business Friday")
- There is no permanent dismissal. The "dismiss" action is a Snooze with a default delay, so the request comes back if it is still pending on GitHub.
- A Snooze ends early when either of these occurs after the Snooze starts:
  - a new request event for the user or a Watched Team (a re-request)
  - a new comment on the pull request that @-mentions the user personally (a team mention does not count)
- Other activity, for example new commits, does not end a Snooze early.
- Working time: the config file sets working hours (`work_start`, `work_end`; default 09:00–17:30) and working days (`work_days`; default Mon–Fri). The tool uses the system local time zone.
- Snooze presets:
  - `1h`, `3h`: wall-clock delays.
  - `Next morning`: the start of the next working day.
  - `Before COB <day>`: 2 hours before the end of that working day. The picker shows one for each of the next 5 working days.
  - `Custom…`: a typed duration, for example `2d` or `45m`.
- Dismiss is a Snooze of 1 working day. It ends at the same time on the next working day.
- Snoozes are kept in `~/.local/state/review-queue/snoozes.json`, with one Snooze for each pull request.
- The status bar shows the number of snoozed requests. The `z` key shows the snoozed view: snoozed requests in their Priority Tiers, dimmed, with the end time of each Snooze. In that view, `u` ends the Snooze of the selected request, and `s` changes its end time.
- The `s` key opens the snooze picker. The `d` key dismisses (a Snooze of 1 working day).
- `Next morning` is the next start of a working day after now. At 07:00 on a working day, it is 09:00 the same day.
- A personal @-mention is a GitHub mention event (`MentionedEvent`) for the user on the pull request. GitHub creates one when a comment or review mentions the user by login. A team mention does not create one for the members of the team. The query gets these events in the same timeline request as the request events, so the poll costs the same.

## v1.2 (delight, complete)

Graphics use the Kitty graphics protocol (Ghostty supports it). If the terminal has no graphics protocol, the tool uses Unicode half-blocks. A `--no-images` flag turns graphics off.

- **Author avatars** in each row of the Queue, with raster graphics only (a 2-cell half-block avatar is not legible). The tool caches them in `~/.cache/review-queue/avatars/<login>.png` and downloads one again when the file is older than 7 days (GitHub keeps the same avatar URL when a person changes their picture). After a failed download, the row has no avatar, and the tool tries again at the next poll.
- **Detail panel**: a large avatar and a summary of the selected pull request: the full title, repository and number, author, draft, tier, Watched Team, Wait Time, Snooze, branches (`head → base`), labels, number of changed files, diff size, CI status of the last commit, and the first lines of the description. It shows below the list when the body is 24 rows or taller, and uses 40% of the height (at least 12 rows). In the panel, the avatar, title, and fields are at the left, and the description is at the right; a panel narrower than 100 columns has the description below the fields. Without raster graphics, the avatar is half-blocks. The description is plain text: HTML comments, image Markdown, and runs of blank lines are removed, and the text is wrapped and cut with `…` to fit. CI status is the rollup state of the last commit: `✓ passing`, `✗ failing` (failure or error), `● pending` (pending or expected), or `– no checks`.
- **Empty-Queue image**: when the Queue is empty, the pane shows a static image in the style of the "there was an attempt" meme: a crudely drawn five-pointed star with a trailing wavy rainbow, on a dark sky. The text above it is "there was an attempt", and the text below it is "to keep on top of PR reviews". The image is drawn by the code at the pixel size of the pane, so it fills the pane at every size. Without raster graphics, it is text art. (This replaces the particle burst, which was removed.)
- **Arrival effect**: a row that came into the visible Queue at a poll flashes a warm highlight that fades to the background in 1 second, fast at the start and slow at the end. There are no arrivals at the first poll. A pull request back from a Snooze is an arrival; a pull request that was already visible is not. It uses terminal colors only, and the tool redraws 20 times a second only while a fade runs. This is not a bell.
- The `a` key previews the empty-queue image.
- No animated avatars.

## v1.3

- **Count out of view.** The terminal title is `review-queue (<pending>)`, with `, error` after a failed poll, and the tool puts back the old title at exit. `--once` prints the Queue as tab-separated lines, and `--count` prints the number of pending requests. Both apply Snoozes, then exit.
- **Re-review diff.** For a re-review, `Enter`/`o` opens the changes from the commit of the user's latest review to the head commit. With no new commit, or no commit on the review, it opens the whole pull request. `O` always opens the whole pull request.
- **Aging.** The Wait Time is amber when a request is Stale and red when it is Overdue. Only working time counts. The thresholds are `stale_after_hours` (default 4) and `overdue_after_hours` (default 8).
- **Resizable detail panel.** The panel starts at 50% of the body height. `+` and `-` change it in steps of 10%, from 20% to 80%. The panel has at least 12 rows and the list at least 6. The size is not saved between sessions.
