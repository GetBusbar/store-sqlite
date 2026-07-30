// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2026 Busbar Inc and contributors

use super::open;
fn expect_err(result: Result<Box<dyn busbar_api::Store>, String>) -> String {
    match result {
        Ok(_) => panic!("expected open() to fail, but it succeeded"),
        Err(e) => e,
    }
}

#[test]
fn malformed_json_is_rejected() {
    let err = expect_err(open("{ this is not json"));
    assert!(
        err.contains("invalid sqlite plugin config"),
        "error should name the config as invalid: {err}"
    );
}

#[test]
fn empty_and_bare_object_configs_use_defaults_and_succeed() {
    // Empty config is NOT an error for this plugin (unlike oidc, sqlite has usable defaults for
    // every field) — both `""` and `"{}"` fall back to the default relative db_path. Both are
    // exercised SEQUENTIALLY in one test (rather than as separate `#[test]`s) because they'd
    // otherwise open the same default-named file concurrently under cargo's parallel test runner
    // and contend on SQLite's file lock. We only assert construction succeeds and clean up
    // immediately so the suite doesn't leave a stray `busbar-governance.db` behind.
    let store = open("").expect("empty config must fall back to defaults and succeed");
    drop(store);
    let _ = std::fs::remove_file("busbar-governance.db");

    let store = open("{}").expect("{} must fall back to defaults and succeed");
    drop(store);
    let _ = std::fs::remove_file("busbar-governance.db");
}

#[test]
fn explicit_in_memory_db_path_is_honored() {
    // Proves `db_path` is actually read (not ignored in favor of the default) via the one value
    // that's observable without touching disk: `:memory:` opens instantly with no file created.
    let store = open(r#"{"db_path": ":memory:"}"#).expect("in-memory db_path must open");
    let key = busbar_api::VirtualKey {
        id: "vk_adapter_test".into(),
        key_hash: "h".into(),
        name: "n".into(),
        allowed_pools: None,
        enabled: true,
        created_at: 1,
        group: None,
        labels: Default::default(),
    };
    store
        .put_key(&key)
        .expect("put_key on the constructed store");
    assert_eq!(
        store.get_key("vk_adapter_test").unwrap().unwrap().id,
        "vk_adapter_test"
    );
}

#[test]
fn explicit_db_path_overrides_the_default_and_creates_that_exact_file() {
    let dir = std::env::temp_dir().join(format!(
        "busbar-sqlite-plugin-open-{}-explicit",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("custom.db");
    let cfg = serde_json::json!({ "db_path": path.to_str().unwrap() }).to_string();
    let store = open(&cfg).expect("explicit db_path must open");
    drop(store);
    assert!(
        path.exists(),
        "open() must create the file at the CONFIGURED path, not the default"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nonexistent_parent_directory_is_a_clean_error_not_a_panic() {
    let dir = std::env::temp_dir().join(format!(
        "busbar-sqlite-plugin-open-{}-missing-parent",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("nested").join("db.sqlite");
    let cfg = serde_json::json!({ "db_path": path.to_str().unwrap() }).to_string();
    let err = expect_err(open(&cfg));
    assert!(!err.is_empty());
    assert!(!dir.exists(), "a failed open must not create the directory");
}
