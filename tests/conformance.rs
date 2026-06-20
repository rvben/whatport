//! `whatport schema` must validate against the published clispec v0.2 JSON
//! Schema (vendored at schemas/clispec-v0.2.json).

#[test]
fn schema_conforms_to_clispec_v0_2() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/clispec-v0.2.json"))
            .expect("vendored clispec schema is valid JSON");

    let instance = whatport::schema::contract();
    let validator = jsonschema::validator_for(&schema).expect("compile clispec schema");

    if !validator.is_valid(&instance) {
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| format!("{} at {}", e, e.instance_path()))
            .collect();
        panic!(
            "whatport schema does not conform to clispec v0.2:\n{}",
            errors.join("\n")
        );
    }
}

#[test]
fn schema_declares_the_expected_shape() {
    let v = whatport::schema::contract();
    assert_eq!(v["clispec"], "0.2");
    assert_eq!(v["name"], "whatport");

    let commands = v["commands"].as_array().unwrap();
    // The kill command is the one mutating command; the rest are read-only.
    let kill = commands.iter().find(|c| c["name"] == "kill").unwrap();
    assert_eq!(kill["mutating"], true);
    for name in ["list", "inspect", "schema"] {
        let c = commands.iter().find(|c| c["name"] == name).unwrap();
        assert_eq!(c["mutating"], false, "{name} must be read-only");
    }
    assert!(v["errors"].as_array().is_some_and(|e| !e.is_empty()));
    assert!(v["global_args"].as_array().is_some_and(|g| !g.is_empty()));
}
