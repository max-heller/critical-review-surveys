use futures::prelude::*;
use serde::Deserialize;
use std::{
    env,
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse environment variables
    dotenv::dotenv()?;
    let endpoint = env::var("QUALTRICS_ENDPOINT")?;
    let token = env::var("QUALTRICS_TOKEN")?;
    let user = env::var("QUALTRICS_USER_ID")?;

    // Initialize qualtrics client
    let client = Client::new(endpoint, token, user);

    if let [_, template_survey_id, course_file] = &env::args().collect::<Vec<_>>()[..] {
        // Open course file
        let f = File::open(course_file)?;
        let courses = BufReader::new(f).lines().collect::<Result<Vec<String>, _>>()?;

        // Duplicate a survey for each course
        let survey_ids = courses.iter().map(|course| {
            let survey_name = course; // TODO
            client
                .duplicate(template_survey_id, survey_name)
                .map(|response| response.map(|r| r.result.id))
        });

        // Wait on futures
        let survey_ids = future::try_join_all(survey_ids).await?;

        // Print out each course/survey pair
        for (course, survey_id) in courses.iter().zip(survey_ids) {
            println!("{},{}", course, survey_id);
        }
    } else {
        println!("usage: cargo run <template survey id> <course csv>")
    }
    Ok(())
}

struct Client {
    client: reqwest::Client,
    endpoint: String,
    token: String,
    user: String,
}

impl Client {
    fn new<E, T, U>(endpoint: E, token: T, user: U) -> Self
    where
        E: AsRef<str>,
        T: AsRef<str>,
        U: AsRef<str>,
    {
        Self {
            client: reqwest::Client::new(),
            endpoint: endpoint.as_ref().into(),
            token: token.as_ref().into(),
            user: user.as_ref().into(),
        }
    }

    async fn copy<S, D, C>(
        &self,
        survey_id: S,
        dest_owner_id: D,
        copy_name: C,
    ) -> Result<CopyResponse, reqwest::Error>
    where
        S: AsRef<str>,
        D: AsRef<str>,
        C: AsRef<str>,
    {
        self.client
            .post(&self.endpoint)
            .header("X-API-TOKEN", &self.token)
            .header("X-COPY-SOURCE", survey_id.as_ref())
            .header("X-COPY-DESTINATION-OWNER", dest_owner_id.as_ref())
            .header("Content-Type", "application/json")
            .body(format!("{{\"projectName\": \"{}\"}}", copy_name.as_ref()))
            .send()
            .await?
            .json::<CopyResponse>()
            .await
    }

    async fn duplicate<S, C>(&self, survey_id: S, copy_name: C) -> Result<CopyResponse, reqwest::Error>
    where
        S: AsRef<str>,
        C: AsRef<str>,
    {
        self.copy(survey_id, &self.user, copy_name).await
    }

    async fn update(&self, survey_id: &str) -> Result<(), Box<dyn Error>> {
        let res = self
            .client
            .put(&format!("{}/{}", &self.endpoint, survey_id))
            .header("X-API-TOKEN", &self.token)
            .header("Content-Type", "application/json")
            .body(
                r#"
{
    "name": "New Survey Name",
    "isActive": true,
    "expiration": {
    	"startDate":"2016-01-01T01:00:00Z", 
        "endDate":"2016-03-01T01:00:00Z"
    }
}
        "#,
            )
            .send()
            .await?
            .text()
            .await?;
        println!("{}", res);

        Ok(())
    }
}

#[derive(Deserialize)]
struct CopyResponse {
    result: ResultResponse,
}

#[derive(Deserialize)]
struct ResultResponse {
    id: String,
}
