# Pull request state, not notifications, is the source of truth

The Queue comes from the current state of open pull requests (the requested reviewers and teams), not from the GitHub notifications API. A notification records that an event occurred. It stays after the review request is complete or withdrawn, it goes out of view when the user reads it on a different device, and it does not tell a Personal Request from a Team Request or a Re-review Request from a First Review Request. With pull request state, the Queue never shows a stale item and the tool can calculate the Priority Tier.

## Considered Options

- **Notifications API** (`reason: review_requested`). Inexpensive to poll (`Last-Modified` and `304` responses), but it shows events, not pending requests. Rejected.
- **Notifications as a "something changed" signal, then fetch pull request state.** This is faster, but it adds a second data path and depends on the user's notification settings. Rejected for v1. We can add it later without a change to the domain model.

## Consequences

The tool has no "mark as read" concept. A request goes out of the Queue only when GitHub's state changes (or, from v1.1, while a Snooze is active).
