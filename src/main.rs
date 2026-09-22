use std::{collections::HashMap, env};

use anyhow::{bail, Context, Result};
use goose_providers::{
    api_client::{ApiClient, AuthMethod},
    decision::{DecisionProvider, DecisionQuestion, DecisionRequest},
    openrouter::{OpenRouterProvider, OPENROUTER_DEFAULT_MODEL},
    typesafe::{TypeSafeProvider, TYPESAFE_DEFAULT_HOST, TYPESAFE_DEFAULT_MODEL},
};
use serde_json::json;

const LOCAL_DEFAULT_HOST: &str = "http://127.0.0.1:8009";
const OPENROUTER_DEFAULT_HOST: &str = "https://openrouter.ai";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Backend {
    Jev,
    OpenRouter,
    Local,
}

impl Backend {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "jev" | "typesafe" => Ok(Self::Jev),
            "openrouter" | "or" => Ok(Self::OpenRouter),
            "local" => Ok(Self::Local),
            other => bail!("unknown backend '{other}' (expected: jev, openrouter, local)"),
        }
    }

    /// Pick a backend from env, defaulting based on which credentials exist.
    fn from_env() -> Result<Self> {
        if let Ok(value) = env::var("DEMO_BACKEND") {
            return Self::parse(&value);
        }
        if env::var("TYPESAFE_API_KEY").is_ok() {
            Ok(Self::Jev)
        } else if env::var("OPENROUTER_API_KEY").is_ok() {
            Ok(Self::OpenRouter)
        } else {
            Ok(Self::Local)
        }
    }
}

fn main_prompt() -> String {
    let prompt = env::args().skip(1).collect::<Vec<_>>().join(" ");
    if prompt.is_empty() {
        "Is Paris the capital of France?".to_string()
    } else {
        prompt
    }
}

fn build_request(model: String, prompt: String) -> DecisionRequest {
    DecisionRequest {
        model,
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
    }
}

fn make_provider(backend: Backend) -> Result<(Box<dyn DecisionProvider>, String)> {
    match backend {
        Backend::Jev => {
            let api_key = env::var("TYPESAFE_API_KEY").context("TYPESAFE_API_KEY is not set")?;
            let host =
                env::var("TYPESAFE_HOST").unwrap_or_else(|_| TYPESAFE_DEFAULT_HOST.to_string());
            let client = ApiClient::new_with_tls(
                host,
                AuthMethod::BearerToken(api_key),
                Some(Default::default()),
            )?;
            let model =
                env::var("DEMO_MODEL").unwrap_or_else(|_| TYPESAFE_DEFAULT_MODEL.to_string());
            Ok((Box::new(TypeSafeProvider::new(client)), model))
        }
        Backend::OpenRouter => {
            let api_key =
                env::var("OPENROUTER_API_KEY").context("OPENROUTER_API_KEY is not set")?;
            let host =
                env::var("OPENROUTER_HOST").unwrap_or_else(|_| OPENROUTER_DEFAULT_HOST.to_string());
            let client = ApiClient::new_with_tls(
                host,
                AuthMethod::BearerToken(api_key),
                Some(Default::default()),
            )?;
            let model =
                env::var("DEMO_MODEL").unwrap_or_else(|_| OPENROUTER_DEFAULT_MODEL.to_string());
            Ok((Box::new(OpenRouterProvider::new(client, None, None)), model))
        }
        Backend::Local => {
            // Local endpoint speaks the TypeSafe/jev API and needs no real auth.
            let host = env::var("LOCAL_HOST").unwrap_or_else(|_| LOCAL_DEFAULT_HOST.to_string());
            let client =
                ApiClient::new_with_tls(host, AuthMethod::BearerToken("unused".into()), None)?;
            let model =
                env::var("DEMO_MODEL").unwrap_or_else(|_| TYPESAFE_DEFAULT_MODEL.to_string());
            Ok((Box::new(TypeSafeProvider::new(client)), model))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let backend = Backend::from_env()?;
    let prompt = main_prompt();
    let (provider, model) = make_provider(backend)?;

    println!("backend: {backend:?}  model: {model}");
    let request = build_request(model, prompt);
    let decision = provider.create_decision(&request).await?;
    println!("{decision:#?}");

    Ok(())
}
