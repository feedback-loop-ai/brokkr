use super::*;

#[test]
fn doctor_prints_both_dsh_overlay_controls_on_and_off() {
    let dir = workspace_with(None);
    let root = dir.path();
    // An unreadable realms map makes doctor report the declared controls.
    write(root, "realms.json", &json!(null));
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/dsh.json"),
        root.join("adapters/dsh.json"),
    )
    .unwrap();

    let (healthy, readout) = lines(root, &installed(&["dsh"]));
    assert!(healthy, "{readout:#?}");
    let controls: Vec<_> = readout
        .iter()
        .filter_map(|line| line.split_once(" · evidence: "))
        .map(|(controls, _)| controls)
        .collect();
    assert_eq!(
        controls,
        [
            "warn     capabilities native dsh 'web-fetch': declared by the adapter: \
             tools [web_fetch] · ON: tool-web config fetch: true · OFF: tool-web config fetch: false",
            "warn     capabilities native dsh 'web-search': declared by the adapter: \
             tools [web_search] · ON: tool-web config search: true · OFF: tool-web config search: false",
        ],
        "{readout:#?}"
    );
}
