use carve_lib::model::{TimedVectorLedger, VectorElement, VectorToolKind};

#[test]
fn test_vector_element_active_at_timestamp() {
    let elem = VectorElement {
        id: "arrow_1".into(),
        tool: VectorToolKind::Arrow,
        start_ms: 1000,
        end_ms: 4000,
        data: serde_json::json!({ "points": [10, 10, 50, 50] }),
    };
    assert!(!elem.is_active_at(999));
    assert!(elem.is_active_at(1000));
    assert!(elem.is_active_at(2500));
    assert!(elem.is_active_at(4000));
    assert!(!elem.is_active_at(4001));
}

#[test]
fn test_ledger_query_active_elements() {
    let mut ledger = TimedVectorLedger::default();
    ledger.add_element(VectorElement {
        id: "1".into(),
        tool: VectorToolKind::Rectangle,
        start_ms: 500,
        end_ms: 2000,
        data: serde_json::json!({}),
    });
    assert_eq!(ledger.active_at(100).len(), 0);
    assert_eq!(ledger.active_at(1500).len(), 1);
}

#[test]
fn test_all_tool_kinds_and_serde() {
    let tools = vec![
        VectorToolKind::Arrow,
        VectorToolKind::Rectangle,
        VectorToolKind::Ellipse,
        VectorToolKind::Pencil,
        VectorToolKind::CounterBadge,
        VectorToolKind::Text,
        VectorToolKind::Blur,
        VectorToolKind::Highlight,
    ];

    for (i, tool) in tools.into_iter().enumerate() {
        let elem = VectorElement {
            id: format!("elem_{}", i),
            tool,
            start_ms: (i as u64) * 1000,
            end_ms: (i as u64 + 1) * 1000,
            data: serde_json::json!({ "index": i }),
        };
        let serialized = serde_json::to_string(&elem).expect("serialize element");
        let deserialized: VectorElement =
            serde_json::from_str(&serialized).expect("deserialize element");
        assert_eq!(elem, deserialized);
    }
}

#[test]
fn test_ledger_remove_and_clear() {
    let mut ledger = TimedVectorLedger::default();
    ledger.add_element(VectorElement {
        id: "1".into(),
        tool: VectorToolKind::Rectangle,
        start_ms: 500,
        end_ms: 2000,
        data: serde_json::json!({}),
    });
    ledger.add_element(VectorElement {
        id: "2".into(),
        tool: VectorToolKind::Arrow,
        start_ms: 1000,
        end_ms: 3000,
        data: serde_json::json!({}),
    });

    assert_eq!(ledger.len(), 2);
    let removed = ledger.remove_element("1");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().id, "1");
    assert_eq!(ledger.len(), 1);

    ledger.clear();
    assert!(ledger.is_empty());
    assert_eq!(ledger.len(), 0);
}

#[test]
fn test_ledger_monotonic_id_generation() {
    let mut ledger = TimedVectorLedger::default();
    let id1 = ledger.generate_id("arrow");
    let id2 = ledger.generate_id("arrow");
    let id3 = ledger.next_id();
    assert_eq!(id1, "arrow_1");
    assert_eq!(id2, "arrow_2");
    assert_eq!(id3, "elem_3");
}

#[test]
fn test_ledger_serde_roundtrip() {
    let mut ledger = TimedVectorLedger::default();
    ledger.add_element(VectorElement {
        id: "elem_1".into(),
        tool: VectorToolKind::Highlight,
        start_ms: 100,
        end_ms: 500,
        data: serde_json::json!({ "color": "#FFFF00" }),
    });

    let json = serde_json::to_string(&ledger).expect("serialize ledger");
    let deserialized: TimedVectorLedger = serde_json::from_str(&json).expect("deserialize ledger");
    assert_eq!(ledger, deserialized);
}
