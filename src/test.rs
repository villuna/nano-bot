use crate::commands::{action::ActionCommandData, say_hi::SayHiData};

#[test]
fn action_commands_parse() {
    let actions = std::fs::read_to_string("assets/actions.yaml").unwrap();
    let parsed = serde_yaml::from_str::<Vec<ActionCommandData>>(&actions);

    assert!(parsed.is_ok());
    let actions = parsed.unwrap();
    eprintln!("{actions:?}");
}

#[test]
fn say_hi_details_parse() {
    let file = std::fs::read_to_string("assets/say_hi.yaml").unwrap();
    let parsed = serde_yaml::from_str::<Vec<SayHiData>>(&file);

    assert!(parsed.is_ok());
    let data = parsed.unwrap();
    eprintln!("{data:?}");
}
