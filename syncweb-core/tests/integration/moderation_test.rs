use ed25519_dalek::SigningKey;
use iroh_blobs::Hash;
use syncweb_core::indexing::ReportRecord;

fn content() -> Hash {
    Hash::from([0_u8; 32])
}

fn key_pair() -> SigningKey {
    let bytes = [1_u8; 32];
    SigningKey::from_bytes(&bytes)
}

fn other_key() -> SigningKey {
    let bytes = [2_u8; 32];
    SigningKey::from_bytes(&bytes)
}

#[test]
fn test_report_record_is_signed() {
    let key = key_pair();
    let report = ReportRecord::new(content(), "spam content".into(), 1000)
        .sign_with(&key)
        .unwrap();
    assert!(report.reporter.is_some());
    assert!(report.signature.is_some());
    assert!(report.verify(&key.verifying_key()).is_ok());
}

#[test]
fn test_report_record_rejects_unsigned_verification() {
    let report = ReportRecord::new(content(), "spam content".into(), 1000);
    assert!(report.reporter.is_none());
    assert!(report.signature.is_none());
    assert!(report.verify(&key_pair().verifying_key()).is_err());
}

#[test]
fn test_report_signature_mismatch_detected() {
    let key_a = key_pair();
    let key_b = other_key();
    let report = ReportRecord::new(content(), "bad".into(), 1000)
        .sign_with(&key_a)
        .unwrap();
    assert!(report.verify(&key_b.verifying_key()).is_err());
}

#[test]
fn test_report_tampering_detected() {
    let key = key_pair();
    let mut report = ReportRecord::new(content(), "bad".into(), 1000)
        .sign_with(&key)
        .unwrap();
    report.reason = "tampered".to_owned();
    assert!(report.verify(&key.verifying_key()).is_err());
}
