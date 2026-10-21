# Review Queue

A terminal tool that shows the pull requests waiting for the user's review, in priority order.

## Language

**Review Request**:
A pending request, in the current state of an open pull request, for the user (or a team the user belongs to) to review it. It exists only while GitHub lists the user or team as a requested reviewer. A pull request that the user authored never has a Review Request for the user.
_Avoid_: Notification, review notification

**Re-review Request**:
A Review Request on a pull request where the user has already submitted at least one review, in any state (approved, changes requested, commented, or dismissed). Ordinary comments without a submitted review do not count. A team request can also be a Re-review Request.
_Avoid_: Follow-up review

**First Review Request**:
A Review Request that is not a Re-review Request.
_Avoid_: New review, initial request

**Personal Request**:
A Review Request that names the user directly. If a pull request requests both the user and one of the user's teams, it is a Personal Request.
_Avoid_: Direct request, individual request

**Team Request**:
A Review Request that names a Watched Team, and does not name the user.
_Avoid_: Group request

**Watched Team**:
A team the user belongs to and has put on the allowlist. Requests to other teams are not Review Requests for this tool, even when the user is a member.
_Avoid_: Subscribed team, my team

**Wait Time**:
The time since the latest request event for a Review Request. A re-request resets it.
_Avoid_: Age, staleness

**Stale Request**:
A Review Request whose Wait Time, counted in the user's working time only, is past the stale threshold but not the overdue threshold.
_Avoid_: Old, aging

**Overdue Request**:
A Review Request whose Wait Time, counted in the user's working time only, is past the overdue threshold.
_Avoid_: Late, expired

**Priority Tier**:
One of four ranks for a Review Request, from highest to lowest: personal re-review, personal first review, team re-review, team first review.
_Avoid_: Bucket, category

**Bot Request**:
A Review Request on a pull request whose author is a bot account (for example, Dependabot or Renovate). It stays in its Priority Tier, but comes last in that tier.
_Avoid_: Automated request

**Snooze**:
A local, temporary hiding of a Review Request until a set time, or until a re-request or a personal @-mention ends it early. There is no permanent dismissal. A Snooze always ends, and the request comes back if it is still pending.
_Avoid_: Dismiss, mute, hide

**Notification**:
A system notification that tells the user about a new Review Request (a first request or a re-request) in one of the Priority Tiers that the user chose. A Snooze that ends by time is not a new request.
_Avoid_: Alert, bell

**Mute**:
A temporary pause of all Notifications until a set time. It hides no Review Request: that is a Snooze. Notifications that occur during a Mute are not sent later.
_Avoid_: Do not disturb, silence

**Queue**:
All current Review Requests for the user, ordered by Priority Tier, then by longest Wait Time first. Inside each Priority Tier, Bot Requests come after all other requests.
_Avoid_: Inbox, feed
