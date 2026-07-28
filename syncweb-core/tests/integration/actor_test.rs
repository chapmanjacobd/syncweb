use std::sync::atomic::{AtomicUsize, Ordering};

use syncweb_core::sync::actor::{Actor, ActorPanic};

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
async fn test_actor_panic_returns_error() {
    let call_count = AtomicUsize::new(0);
    let handle = Actor::spawn(move |msg: String| {
        let count = call_count.fetch_add(1, Ordering::SeqCst);
        async move {
            assert!(msg != "panic", "intentional test panic");
            format!("ok:{count}")
        }
    });

    let result = handle.request("panic".to_owned()).await;
    assert!(result.is_err(), "actor should return error on panic, not default");
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("actor handler panicked"),
        "error should describe the panic, got: {err_str}"
    );

    let result2 = handle.request("after".to_owned()).await;
    assert!(result2.is_ok(), "actor should still work after a panicking message");
    assert_eq!(result2.unwrap(), "ok:1");

    assert!(!handle.is_closed(), "actor should still be alive");
}

#[tokio::test]
async fn test_actor_panic_concurrent_callers() {
    let handle = Actor::spawn(|msg: String| async move {
        assert!(msg != "panic", "intentional test panic");
        msg
    });

    let r1 = handle.request("panic".to_owned()).await;
    let r2 = handle.request("no panic".to_owned()).await;

    assert!(r1.is_err(), "first caller should get error");
    assert!(r2.is_ok(), "second caller should still get success");
    assert_eq!(r2.unwrap(), "no panic");
}

#[tokio::test]
async fn test_actor_no_panic_still_works() {
    let handle = Actor::spawn(|msg: String| async move { msg });

    let result = handle.request("hello".to_owned()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn test_actor_panic_display() {
    let panic = ActorPanic::new("test error".to_owned());
    assert_eq!(panic.to_string(), "actor handler panicked: test error");
}
