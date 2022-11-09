use std::collections::HashMap;

pub(crate) use data::*;
use reqwest::{
    blocking::{Client, RequestBuilder},
    StatusCode,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    #[error("A invalid base url was used `{base_url}`")]
    InvalidBaseUrl { base_url: String },
    #[error("This user is not authorized for this action")]
    Unauthorized,
    #[error("The {kind} `{name}` was not found, or the user was unauthorized")]
    NotFound { kind: String, name: String },
    #[error("Failed to serialize the response")]
    SerializationError,
    #[error("A unknown api error occured")]
    UnknownError,
}

#[derive(Debug)]
pub(crate) struct WorklogApi {
    client: Client,
    domain: String,
}

impl WorklogApi {
    pub(crate) fn new<S: ToString>(domain: S) -> Self {
        Self {
            client: Client::new(),
            domain: domain.to_string(),
        }
    }

    pub(crate) fn worklogs(
        &self,
        context: ViewWorklogDto,
        authorization: &Authorization,
    ) -> Result<Vec<WorklogResponse>, ApiError> {
        let url = format!(
            "{}/rest/api/2/issue/{}/worklog",
            self.domain,
            context.issue()
        );

        let mut query = HashMap::new();
        if let Some(from) = context.from() {
            query.insert("startedAfter", from.format("%s000").to_string());
        }
        if let Some(until) = context.until() {
            query.insert("startedBefore", until.format("%s000").to_string());
        }
        let query = query.into_iter().collect::<Vec<_>>();

        let response = self
            .client
            .get(url)
            .authorize(authorization)
            .query(&query)
            .send()
            .map_err(|e| match e {
                e if e.is_builder() => ApiError::InvalidBaseUrl {
                    base_url: self.domain.clone(),
                },
                _ => ApiError::UnknownError,
            })?;

        match response.status().clone() {
            s if s.is_success() => Ok(()),
            StatusCode::NOT_FOUND => Err(ApiError::NotFound {
                kind: "issue".to_string(),
                name: context.issue().to_string(),
            }),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ApiError::Unauthorized),
            _ => Err(ApiError::UnknownError),
        }?;

        response
            .json::<PagedWorklogResponse>()
            .map(|r| r.worklogs().to_vec())
            .map_err(|_| ApiError::SerializationError)
    }

    pub(crate) fn create_worklog(
        &self,
        dto: CreateWorklogDto,
        authorization: &Authorization,
    ) -> Result<(), ApiError> {
        let url = format!("{}/rest/api/2/issue/{}/worklog", self.domain, dto.issue());

        let body: WorklogAddBody = dto.clone().into();
        let request = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&body)
            .authorize(authorization);

        let response = request.send().map_err(|_| ApiError::UnknownError)?;
        match response.status() {
            s if s.is_success() => Ok(()),
            StatusCode::NOT_FOUND => Err(ApiError::NotFound {
                kind: "issue".to_string(),
                name: dto.issue().to_string(),
            }),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ApiError::Unauthorized),
            _ => Err(ApiError::UnknownError),
        }
    }
}

mod data {
    use chrono::{Date, DateTime, Utc};

    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct PagedWorklogResponse {
        worklogs: Vec<WorklogResponse>,
    }

    impl PagedWorklogResponse {
        pub(crate) fn worklogs(&self) -> &[WorklogResponse] {
            self.worklogs.as_ref()
        }
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct WorklogResponse {
        author: AuthorResponse,
        comment: Option<String>,
        time_spent: String,
    }

    impl WorklogResponse {
        pub(crate) fn time_spent(&self) -> &str {
            self.time_spent.as_ref()
        }

        pub(crate) fn author(&self) -> &AuthorResponse {
            &self.author
        }

        pub(crate) fn comment(&self) -> Option<&String> {
            self.comment.as_ref()
        }
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct AuthorResponse {
        display_name: String,
    }

    impl AuthorResponse {
        pub(crate) fn display_name(&self) -> &str {
            self.display_name.as_ref()
        }
    }

    #[derive(Debug)]
    pub(crate) struct ViewWorklogDto {
        issue: String,
        from: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
    }

    impl ViewWorklogDto {
        pub(crate) fn new(
            issue: String,
            from: Option<DateTime<Utc>>,
            until: Option<DateTime<Utc>>,
        ) -> Self {
            Self { issue, from, until }
        }

        pub(crate) fn issue(&self) -> &str {
            self.issue.as_ref()
        }

        pub(crate) fn from(&self) -> Option<DateTime<Utc>> {
            self.from
        }

        pub(crate) fn until(&self) -> Option<DateTime<Utc>> {
            self.until
        }
    }

    #[derive(Debug, Clone)]
    pub(crate) struct CreateWorklogDto {
        issue: String,
        comment: Option<String>,
        time_spent: String,
        started: Date<Utc>,
    }

    impl CreateWorklogDto {
        pub(crate) fn new(
            issue: String,
            comment: Option<String>,
            time_spent: String,
            started: Date<Utc>,
        ) -> Self {
            Self {
                issue,
                comment,
                time_spent,
                started,
            }
        }

        pub(crate) fn issue(&self) -> &str {
            self.issue.as_ref()
        }
    }

    impl From<CreateWorklogDto> for WorklogAddBody {
        fn from(dto: CreateWorklogDto) -> Self {
            Self {
                comment: dto.comment,
                time_spent: dto.time_spent,
                started: dto
                    .started
                    .and_hms(12, 0, 0)
                    .format("%FT%X.%3f%z")
                    .to_string(),
            }
        }
    }

    #[derive(Debug, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct WorklogAddBody {
        comment: Option<String>,
        time_spent: String,
        started: String,
    }
}

#[derive(Debug)]
pub(crate) enum Authorization {
    ApiToken { username: String, api_token: String },
    AccessToken { access_token: String },
}

trait Authorize {
    fn authorize(self, authorization: &Authorization) -> Self;
}

impl Authorize for RequestBuilder {
    fn authorize(self, authorization: &Authorization) -> Self {
        match authorization {
            Authorization::ApiToken {
                username,
                api_token,
            } => self.basic_auth(username, Some(api_token)),
            Authorization::AccessToken { access_token } => {
                self.header("Authorization", format!("Bearer {}", access_token))
            }
        }
    }
}
