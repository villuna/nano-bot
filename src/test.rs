use crate::commands::action::ActionCommandData;

#[test]
fn action_commands_parse() {
    let actions = std::fs::read_to_string("assets/actions.yaml").unwrap();
    let parsed = serde_yaml::from_str::<Vec<ActionCommandData>>(&actions);

    assert!(parsed.is_ok());
    let actions = parsed.unwrap();
    eprintln!("{actions:?}");
}
