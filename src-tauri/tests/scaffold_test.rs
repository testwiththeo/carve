#[test]
fn test_tauri_config_has_required_windows() {
    let conf_str = include_str!("../tauri.conf.json");
    let conf: serde_json::Value = serde_json::from_str(conf_str).expect("Valid JSON");
    let windows = conf["app"]["windows"].as_array().expect("Windows array");
    let labels: Vec<&str> = windows.iter().filter_map(|w| w["label"].as_str()).collect();
    assert!(labels.contains(&"main"));
    assert!(labels.contains(&"selector"));
    assert!(labels.contains(&"overlay"));

    for w in windows {
        let label = w["label"].as_str().unwrap();
        if label == "selector" || label == "overlay" {
            assert_eq!(
                w["visible"].as_bool(),
                Some(false),
                "{} window must have visible: false",
                label
            );
        }
    }
}
