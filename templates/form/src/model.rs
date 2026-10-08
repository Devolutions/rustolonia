pub fn greeting(name: &str) -> std::result::Result<String, &'static str> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a name before submitting.");
    }
    Ok(format!("Hello, {name}!"))
}
