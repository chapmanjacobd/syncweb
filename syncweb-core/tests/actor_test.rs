use std::sync::atomic::{AtomicUsize, Ordering};

use syncweb_core::sync::actor::Actor;

#[tokio::test]
async fn test_actor_handles_messages() {
    let handle = Actor::spawn(|msg: String| async move { format!("reply:{msg}") });

    let reply = handle.request("hello".to_owned()).await;
    assert!(reply.is_ok());
    assert_eq!(reply.unwrap(), "reply:hello");

    let reply2 = handle.request("world".to_owned()).await;
    assert!(reply2.is_ok());
    assert_eq!(reply2.unwrap(), "reply:world");
}

#[tokio::test]
async fn test_actor_panic_isolation() {
    let call_count = AtomicUsize::new(0);
    let handle = Actor::spawn(move |msg: String| {
        let count = call_count.fetch_add(1, Ordering::SeqCst);
        async move {
            assert_ne!(msg, "panic", "intentional test panic");
            format!("ok:{count}")
        }
    });

    let result = handle.request("panic".to_owned()).await;
    assert!(
        result.is_ok(),
        "actor should return default on panic, not propagate error"
    );
    assert_eq!(result.unwrap(), String::default(), "default for String is empty");

    let result2 = handle.request("after".to_owned()).await;
    assert!(result2.is_ok(), "actor should still work after a panicking message");
    assert_eq!(result2.unwrap(), "ok:1");

    assert!(!handle.is_closed(), "actor should still be alive");
}
