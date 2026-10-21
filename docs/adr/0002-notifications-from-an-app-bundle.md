# Notifications come from the tool itself, in an app bundle

On macOS, review-queue sends notifications with the User Notifications framework (`UNUserNotificationCenter`), from its own process. The framework sends a notification only from a process that has a bundle identifier, so the binary runs from inside `review-queue.app` (made by `scripts/install-macos.sh`, signed ad hoc). A click on a notification comes back to the running tool, which selects the row of the pull request and brings the terminal app to the front.

## Considered Options

- **`mac-notification-sys` (`NSUserNotification`).** The notification shows, but on current macOS the framework never reports delivery or a click, and the notification belongs to Script Editor: a click opens Script Editor. Rejected.
- **`osascript -e 'display notification'`.** The same as above: a click opens Script Editor, and the tool gets no click. Rejected.
- **Terminal notifications (OSC 9 / OSC 777).** herdr does not pass them through to Ghostty, so they do not show. Rejected.
- **A helper app that reports clicks to the tool.** It works, but it is a second program to build, sign, and install. Not necessary: the tool can be the app.

## Consequences

- `cargo install` alone gives a tool without notifications. The help page says why. The install script puts a small script on the `PATH`, not a symbolic link: through a link, macOS finds no bundle.
- After the first install, the user turns on notifications for review-queue in System Settings. macOS did not show a permission prompt for the ad-hoc signed app.
- The notification center delivers clicks on the main run loop. The tool runs on its own thread, and the main thread runs the run loop.
- A click also makes macOS start a second copy of the app, with no terminal. That copy stops at once.
