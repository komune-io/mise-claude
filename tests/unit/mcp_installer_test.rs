use chord::installer::mcp::pick_first_binary;

#[test]
fn pick_first_binary_returns_lexically_first_name() {
    let mut names = vec!["b-bin".to_string(), "a-bin".to_string(), "c-bin".to_string()];
    assert_eq!(pick_first_binary(&mut names), Some("a-bin".to_string()));
}

#[test]
fn pick_first_binary_is_stable_regardless_of_input_order() {
    let mut a = vec!["x".to_string(), "y".to_string(), "z".to_string()];
    let mut b = vec!["z".to_string(), "y".to_string(), "x".to_string()];
    assert_eq!(pick_first_binary(&mut a), pick_first_binary(&mut b));
}

#[test]
fn pick_first_binary_returns_none_for_empty_input() {
    let mut empty: Vec<String> = vec![];
    assert_eq!(pick_first_binary(&mut empty), None);
}
