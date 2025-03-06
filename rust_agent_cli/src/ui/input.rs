use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input};

pub fn get_user_input(prompt: &str) -> Result<String> {
    let input = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact()?;
    Ok(input)
}
