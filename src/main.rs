use std::{collections::HashMap, env};

use anyhow::{Context, Result};
use goose_providers::{
    api_client::{ApiClient, AuthMethod},
    decision::{DecisionProvider, DecisionQuestion, DecisionRequest},
    typesafe::{TypeSafeProvider, TYPESAFE_DEFAULT_HOST, TYPESAFE_DEFAULT_MODEL},
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = env::var("TYPESAFE_API_KEY").context("TYPESAFE_API_KEY is not set")?;
    let prompt = env::args().skip(1).collect::<Vec<_>>().join(" ");
    let prompt = if prompt.is_empty() {
        "Is Paris the capital of France?".to_string()
    } else {
        prompt
    };

    let client = ApiClient::new_with_tls(
        TYPESAFE_DEFAULT_HOST.to_string(),
        AuthMethod::BearerToken(api_key),
        Some(Default::default()),
    )?;
    let provider = TypeSafeProvider::new(client);
    let request = DecisionRequest {
        model: TYPESAFE_DEFAULT_MODEL.to_string(),
        state: json!({ "prompt": prompt }),
        questions: HashMap::from([(
            "response".to_string(),
            DecisionQuestion::Choice {
                instructions: "Choose the best response to the prompt.".to_string(),
                criteria: HashMap::from([
                    (
                        "yes".to_string(),
                        "A positive or affirmative response".to_string(),
                    ),
                    ("no".to_string(), "A negative response".to_string()),
                ]),
            },
        )]),
    };

    let decision = provider.create_decision(&request).await?;
    println!("{decision:#?}");

    Ok(())
}
