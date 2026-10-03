use any_context_core_rs::storage::NativeConfigDb;

#[test]
fn test_workspace_settings_isolation_and_persistence() {
    let db = NativeConfigDb::open_in_memory().expect("open memory db");

    // 1. Initial Default workspace must strictly be 'strict', web_search=false
    let default_mode = db.get_workspace_grounding_mode("Default").expect("get default mode");
    assert_eq!(default_mode, "strict", "New Default workspace must default to strict");
    assert!(!db.get_workspace_web_search("Default").expect("default web"));

    // 2. Configure Workspace A (Proactive, Web ON, custom model)
    db.create_workspace("ResearchLab", Some("AI Research"))
        .expect("create ResearchLab");
    db.set_workspace_grounding_mode("ResearchLab", "proactive")
        .expect("set proactive");
    db.set_workspace_web_search("ResearchLab", true)
        .expect("set web search true");
    db.set_workspace_model("ResearchLab", "claude-3-5-sonnet")
        .expect("set model");

    // 3. Configure Workspace B (Hybrid, Web OFF, custom model)
    db.create_workspace("LegalAudit", Some("Contracts"))
        .expect("create LegalAudit");
    db.set_workspace_grounding_mode("LegalAudit", "hybrid")
        .expect("set hybrid");
    db.set_workspace_web_search("LegalAudit", false)
        .expect("set web search false");
    db.set_workspace_model("LegalAudit", "gpt-4o")
        .expect("set model");

    // 4. Verify 100% hermetic isolation across all workspaces
    assert_eq!(
        db.get_workspace_grounding_mode("Default").expect("get"),
        "strict",
        "Default workspace must NOT be polluted by ResearchLab or LegalAudit"
    );
    assert!(
        !db.get_workspace_web_search("Default").expect("get"),
        "Default web search must remain false"
    );

    assert_eq!(
        db.get_workspace_grounding_mode("ResearchLab").expect("get"),
        "proactive",
        "ResearchLab must strictly preserve proactive mode"
    );
    assert!(
        db.get_workspace_web_search("ResearchLab").expect("get"),
        "ResearchLab web search must remain true"
    );
    assert_eq!(
        db.get_workspace_model("ResearchLab").expect("get"),
        "claude-3-5-sonnet"
    );

    assert_eq!(
        db.get_workspace_grounding_mode("LegalAudit").expect("get"),
        "hybrid",
        "LegalAudit must strictly preserve hybrid mode"
    );
    assert!(
        !db.get_workspace_web_search("LegalAudit").expect("get"),
        "LegalAudit web search must remain false"
    );
    assert_eq!(
        db.get_workspace_model("LegalAudit").expect("get"),
        "gpt-4o"
    );

    // 5. Newly created workspace starts with strict
    db.create_workspace("FreshWorkspace", None).expect("create fresh");
    assert_eq!(
        db.get_workspace_grounding_mode("FreshWorkspace").expect("get"),
        "strict"
    );
    assert!(!db.get_workspace_web_search("FreshWorkspace").expect("get"));
}
