use agemax_core::model::SurfaceKind;

#[test]
fn test_surface_kind_defaults_and_stretch_serialization() {
    assert_eq!(SurfaceKind::default(), SurfaceKind::Terminal);

    let browser = SurfaceKind::BrowserPreview {
        url: "http://localhost:3000".to_string(),
    };
    let json = serde_json::to_string(&browser).unwrap();
    let roundtrip: SurfaceKind = serde_json::from_str(&json).unwrap();
    assert_eq!(browser, roundtrip);

    let acp = SurfaceKind::AgentCardPanel {
        session_id: Some("conv-42".to_string()),
    };
    let json_acp = serde_json::to_string(&acp).unwrap();
    let roundtrip_acp: SurfaceKind = serde_json::from_str(&json_acp).unwrap();
    assert_eq!(acp, roundtrip_acp);
}
