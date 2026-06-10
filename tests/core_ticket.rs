use lineagent::core::ticket::{Priority, RelationType, Status};

#[test]
fn parses_valid_and_rejects_invalid() {
    assert!("in_progress".parse::<Status>().is_ok());
    assert!("nope".parse::<Status>().is_err());
    assert!("critical".parse::<Priority>().is_ok());
    assert!("high".parse::<Priority>().is_ok());
    assert!("bad_priority".parse::<Priority>().is_err());
}

#[test]
fn status_variants() {
    let cases = ["backlog", "todo", "in_progress", "review", "done", "cancelled"];
    for s in cases {
        assert!(s.parse::<Status>().is_ok(), "failed to parse: {s}");
    }
}

#[test]
fn priority_variants() {
    for p in ["critical", "high", "medium", "low"] {
        assert!(p.parse::<Priority>().is_ok(), "failed to parse: {p}");
    }
}

#[test]
fn relation_type_variants() {
    for r in ["blocks", "duplicates", "relates_to"] {
        assert!(r.parse::<RelationType>().is_ok(), "failed to parse: {r}");
    }
    assert!("child_of".parse::<RelationType>().is_err()); // dropped from v1
}

#[test]
fn defaults() {
    assert_eq!(Status::default().as_str(), "backlog");
    assert_eq!(Priority::default().as_str(), "medium");
}

#[test]
fn serde_roundtrip() {
    let s: Status = serde_json::from_str("\"in_progress\"").unwrap();
    assert_eq!(s.as_str(), "in_progress");
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(json, "\"in_progress\"");
}
