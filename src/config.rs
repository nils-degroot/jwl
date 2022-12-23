use crate::{Authorization, Config, Context, APPLICATION_NAME, CONFIG_NAME};
use anyhow::Result;
use dialoguer::{FuzzySelect, Input, Password};
use thiserror::Error;

const API_TOKEN: &'_ str = "Api token";
const ACCESS_TOKEN: &'_ str = "Access token";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("A invalid authorization method was selected")]
    FailedToSelectAuthorization,
}

pub fn setup_config() -> Result<()> {
    let jira_domain = Input::<String>::new()
        .with_prompt("Jira domain to connect to")
        .interact_text()?;

    let auth_methods = vec![ACCESS_TOKEN, API_TOKEN];
    let auth_method = FuzzySelect::new()
        .items(&auth_methods)
        .default(0)
        .interact_opt()?
        .ok_or(ConfigError::FailedToSelectAuthorization)?;

    let authorization = match auth_methods.get(auth_method) {
        Some(&API_TOKEN) => prompt_api_token()?,
        Some(&ACCESS_TOKEN) => prompt_access_token()?,
        _ => Err(ConfigError::FailedToSelectAuthorization)?,
    };

    let context = Context::new(None, authorization, jira_domain);
    confy::store(
        APPLICATION_NAME,
        CONFIG_NAME,
        Config::SingleContext(context),
    )?;

    println!("Config created, application ready for use");

    Ok(())
}

fn prompt_api_token() -> Result<Authorization> {
    let username = Input::<String>::new()
        .with_prompt("Username")
        .interact_text()?;

    let api_token = Password::new()
        .with_prompt("Api token")
        .with_confirmation(
            "Api token confirmation",
            "The confirmation differed from the entered api token",
        )
        .interact()?;

    Ok(Authorization::ApiToken {
        username,
        api_token,
    })
}

fn prompt_access_token() -> Result<Authorization> {
    let access_token = Password::new()
        .with_prompt("Access token")
        .with_confirmation(
            "Access token confirmation",
            "The confirmation differed from the entered access token",
        )
        .interact()?;

    Ok(Authorization::AccessToken { access_token })
}
