use add::*;
use anyhow::Result;
use chrono::{Date, DateTime, Utc};
use clap::{Parser, Subcommand};
use thiserror::Error;
use view::*;

mod config;
mod worklog_api;

pub const APPLICATION_NAME: &'_ str = "jwl";
pub const CONFIG_NAME: &'_ str = "config";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum Config {
    SingleContext(Context),
    MultipleContexts(Vec<Context>),
}

impl Default for Config {
    fn default() -> Self {
        Self::SingleContext(Context::default())
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct Context {
    name: Option<String>,
    authorization: Authorization,
    jira_domain: String,
}

impl Context {
    pub(crate) fn new(
        name: Option<String>,
        authorization: Authorization,
        jira_domain: String,
    ) -> Self {
        Self {
            name,
            authorization,
            jira_domain,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum Authorization {
    ApiToken { username: String, api_token: String },
    AccessToken { access_token: String },
}

impl Default for Authorization {
    fn default() -> Self {
        Self::AccessToken {
            access_token: "".to_string(),
        }
    }
}

impl From<Authorization> for worklog_api::Authorization {
    fn from(auth: Authorization) -> Self {
        match auth {
            Authorization::ApiToken {
                username,
                api_token,
            } => worklog_api::Authorization::ApiToken {
                username,
                api_token,
            },
            Authorization::AccessToken { access_token } => {
                worklog_api::Authorization::AccessToken { access_token }
            }
        }
    }
}

#[derive(Debug, Parser)]
#[command(author, version, about)]
/// Program to create and view worklogs using Jira
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// View all worklogs for a issue and date
    View {
        /// Id of the issue
        issue: String,
        /// Date to filter to, defaults to today
        #[clap(short, long)]
        #[arg(value_parser = string_to_date_mapper)]
        date: Option<Date<Utc>>,
        /// Context to use by name, only required when using a config with multiple contexts
        #[clap(short, long)]
        context: Option<String>,
    },
    /// Create a new worklog
    Add {
        /// Comment to add to the worklog
        #[clap(short, long)]
        comment: Option<String>,
        /// Id of the issue
        issue: String,
        /// The time spent working on the issue as days (#d), hours (#h), or minutes (#m or #)
        time_spend: String,
        /// Date on which the worklog effort was started, defaults to today
        #[clap(short, long)]
        #[arg(value_parser = string_to_date_mapper)]
        date: Option<Date<Utc>>,
        /// Context to use by name, only required when using a config with multiple contexts
        #[clap(short, long)]
        context: Option<String>,
    },
    /// Setup the configuration using a prompt
    Config,
}

fn string_to_date_mapper(input: &'_ str) -> Result<Date<Utc>, String> {
    format!("{}T00:00:00Z", input)
        .parse::<DateTime<Utc>>()
        .map(|d| d.date())
        .map_err(|_| {
            "Could not parse to a valid date, dates should have format `yyyy-mm-dd`".to_string()
        })
}

fn main() -> anyhow::Result<()> {
    match Args::parse().command {
        Commands::View {
            date,
            issue,
            context,
        } => {
            let context = read_config(context)?;
            view_worklog(context, ViewContext::new(date.unwrap_or_else(today), issue))
        }
        Commands::Add {
            date,
            issue,
            comment,
            time_spend,
            context,
        } => {
            let context = read_config(context)?;

            add_worklog(
                context,
                AddContext::new(date.unwrap_or_else(today), issue, comment, time_spend),
            )
        }
        Commands::Config => config::setup_config(),
    }
}

fn today() -> Date<Utc> {
    Utc::now().date()
}

#[derive(Debug, Error)]
enum ReadConfigError {
    #[error("When using multiple contexts, a context name should be passed")]
    NoContextNameGiven,
    #[error("Context `{0}` was not found")]
    ContextNotFound(String),
}

fn read_config(context: Option<String>) -> Result<Context> {
    let config = confy::load::<Config>(APPLICATION_NAME, CONFIG_NAME)?;

    match config {
        Config::SingleContext(context) => Ok(context),
        Config::MultipleContexts(contexts) => {
            let name = context.ok_or(ReadConfigError::NoContextNameGiven)?;

            let context = contexts
                .into_iter()
                .find(|c| c.name == Some(name.clone()))
                .ok_or(ReadConfigError::ContextNotFound(name))?;

            Ok(context)
        }
    }
}

mod view {
    use crate::{
        worklog_api::{ViewWorklogDto, WorklogApi},
        Context,
    };
    use chrono::{Date, Utc};

    #[derive(Debug, Clone)]
    pub(crate) struct ViewContext {
        date: Date<Utc>,
        issue: String,
    }

    impl ViewContext {
        pub(crate) fn new(date: Date<Utc>, issue: String) -> Self {
            Self { date, issue }
        }
    }

    impl From<ViewContext> for ViewWorklogDto {
        fn from(context: ViewContext) -> Self {
            ViewWorklogDto::new(
                context.issue,
                Some(context.date.and_hms(0, 0, 0)),
                Some(context.date.and_hms(23, 59, 59)),
            )
        }
    }

    pub(crate) fn view_worklog(config: Context, context: ViewContext) -> anyhow::Result<()> {
        let body = WorklogApi::new(config.jira_domain.clone())
            .worklogs(context.clone().into(), &config.authorization.into())?;

        for log in body.iter() {
            println!(
                "> {} <{}> `{}` {}",
                context.issue,
                log.author().display_name(),
                log.time_spent(),
                log.comment().unwrap_or(&String::from("`no comment`"))
            );
        }

        Ok(())
    }
}

mod add {
    use crate::worklog_api::{CreateWorklogDto, WorklogApi};
    use crate::Context;
    use chrono::{Date, Utc};

    #[derive(Debug)]
    pub(crate) struct AddContext {
        date: Date<Utc>,
        issue: String,
        comment: Option<String>,
        time_spend: String,
    }

    impl AddContext {
        pub(crate) fn new(
            date: Date<Utc>,
            issue: String,
            comment: Option<String>,
            time_spend: String,
        ) -> Self {
            Self {
                date,
                issue,
                comment,
                time_spend,
            }
        }
    }

    impl From<AddContext> for CreateWorklogDto {
        fn from(context: AddContext) -> Self {
            CreateWorklogDto::new(
                context.issue,
                context.comment,
                context.time_spend,
                context.date,
            )
        }
    }

    pub(crate) fn add_worklog(config: Context, context: AddContext) -> anyhow::Result<()> {
        WorklogApi::new(config.jira_domain)
            .create_worklog(context.into(), &config.authorization.into())?;

        Ok(())
    }
}
