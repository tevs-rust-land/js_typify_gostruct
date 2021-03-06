use typify_gostruct::Source;

fn main() -> Result<(), Vec<String>> {
    let example = r#"
    type Region struct {
    Country string `json:"country" binding:"required"`
    State string 
    }
    "#;
    let source = Source::new(example);
    let result = source.transform_to("flow")?;
    println!("{}", result);
    Ok(())
}
