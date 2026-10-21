//! System notifications. On macOS, they use the User Notifications framework, which
//! needs the process to have a bundle identifier: the binary must run from inside
//! `review-queue.app` (see `scripts/install-macos.sh`). A click on a notification comes
//! back as `Incoming::Clicked`.

use crate::notify::Notice;

/// Whether notifications can show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// The tool cannot send notifications, for this reason.
    Unavailable(&'static str),
    /// Waiting for the answer to the request for permission.
    Asking,
    Allowed,
    /// Notifications for review-queue are off in the system settings.
    NotAllowed,
}

impl Status {
    pub fn describe(&self) -> String {
        match self {
            Status::Unavailable(reason) => format!("unavailable ({reason})"),
            Status::Asking => "asking for permission…".into(),
            Status::Allowed => "on".into(),
            Status::NotAllowed => "off in System Settings → Notifications → review-queue".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Incoming {
    Clicked { repo: String, number: u64 },
    Status(Status),
}

/// The identifier of the notification for a pull request. It is unique for each
/// notification, and it has the pull request, for the click.
fn identifier(notice: &Notice) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("review-queue:{}#{}:{nanos}", notice.repo, notice.number)
}

/// The pull request in a notification identifier.
fn pull_request(identifier: &str) -> Option<(String, u64)> {
    let rest = identifier.strip_prefix("review-queue:")?;
    let (reference, _) = rest.rsplit_once(':')?;
    let (repo, number) = reference.rsplit_once('#')?;
    Some((repo.to_string(), number.parse().ok()?))
}

#[cfg(target_os = "macos")]
pub use macos::{run_main_loop_until, send, start};

/// Waits until `done` is true. Call it on the main thread.
#[cfg(not(target_os = "macos"))]
pub fn run_main_loop_until(done: impl Fn() -> bool) {
    while !done() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[cfg(not(target_os = "macos"))]
pub fn start(incoming: tokio::sync::mpsc::UnboundedSender<Incoming>) -> Status {
    let _ = incoming;
    Status::Unavailable("only on macOS")
}

#[cfg(not(target_os = "macos"))]
pub fn send(_notice: &Notice) {}

#[cfg(target_os = "macos")]
mod macos {
    use block2::{DynBlock, RcBlock};
    use objc2::rc::Retained;
    use objc2::runtime::{Bool, ProtocolObject};
    use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
    use objc2_foundation::{
        NSBundle, NSDate, NSError, NSObject, NSObjectProtocol, NSRunLoop, NSString,
    };
    use objc2_user_notifications::{
        UNAuthorizationOptions, UNMutableNotificationContent, UNNotification,
        UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
        UNNotificationSound, UNUserNotificationCenter, UNUserNotificationCenterDelegate,
    };
    use tokio::sync::mpsc::UnboundedSender;

    use super::{Incoming, Status, identifier, pull_request};
    use crate::notify::Notice;

    struct Ivars {
        incoming: UnboundedSender<Incoming>,
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "ReviewQueueNotificationDelegate"]
        #[ivars = Ivars]
        struct Delegate;

        unsafe impl NSObjectProtocol for Delegate {}

        unsafe impl UNUserNotificationCenterDelegate for Delegate {
            /// Show the notification even when the tool is the active app.
            #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
            fn will_present(
                &self,
                _center: &UNUserNotificationCenter,
                _notification: &UNNotification,
                done: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
            ) {
                done.call((UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List
                    | UNNotificationPresentationOptions::Sound,));
            }

            #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
            fn did_receive(
                &self,
                _center: &UNUserNotificationCenter,
                response: &UNNotificationResponse,
                done: &DynBlock<dyn Fn()>,
            ) {
                let id = response.notification().request().identifier().to_string();
                if let Some((repo, number)) = pull_request(&id) {
                    let _ = self
                        .ivars()
                        .incoming
                        .send(Incoming::Clicked { repo, number });
                }
                done.call(());
            }
        }
    );

    /// Sets up notifications and asks for permission. The answer comes as
    /// `Incoming::Status`.
    pub fn start(incoming: UnboundedSender<Incoming>) -> Status {
        if NSBundle::mainBundle().bundleIdentifier().is_none() {
            return Status::Unavailable("run review-queue from review-queue.app");
        }
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let delegate = Delegate::alloc().set_ivars(Ivars {
            incoming: incoming.clone(),
        });
        let delegate: Retained<Delegate> = unsafe { msg_send![super(delegate), init] };
        center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        // The center keeps a weak reference: the delegate must live as long as the tool.
        std::mem::forget(delegate);

        let answer = RcBlock::new(move |granted: Bool, _error: *mut NSError| {
            let _ = incoming.send(Incoming::Status(if granted.as_bool() {
                Status::Allowed
            } else {
                Status::NotAllowed
            }));
        });
        center.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
            &answer,
        );
        Status::Asking
    }

    /// Runs the main run loop until `done` is true. Call it on the main thread: the
    /// notification center delivers the clicks there.
    pub fn run_main_loop_until(done: impl Fn() -> bool) {
        let run_loop = NSRunLoop::mainRunLoop();
        while !done() {
            run_loop.runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.1));
        }
    }

    pub fn send(notice: &Notice) {
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&notice.title));
        content.setSubtitle(&NSString::from_str(&notice.subtitle));
        content.setBody(&NSString::from_str(&notice.body));
        content.setSound(Some(&UNNotificationSound::defaultSound()));
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&identifier(notice)),
            &content,
            None,
        );
        UNUserNotificationCenter::currentNotificationCenter()
            .addNotificationRequest_withCompletionHandler(&request, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identifier_has_the_pull_request_for_the_click() {
        let notice = Notice {
            title: String::new(),
            subtitle: String::new(),
            body: String::new(),
            repo: "acme/widgets".into(),
            number: 7,
        };
        assert_eq!(
            pull_request(&identifier(&notice)),
            Some(("acme/widgets".into(), 7))
        );
        assert_eq!(pull_request("other:acme/widgets#7:1"), None);
    }
}
