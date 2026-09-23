// /discord

use crate::state::AppState;
use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::{StatusCode, header::HeaderMap},
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use lazy_static::lazy_static;
use std::sync::Arc;
use twilight_model::{
    application::interaction::{Interaction, InteractionData, InteractionType},
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
};

lazy_static! {
    static ref PUBLIC_KEY: String =
        std::env::var("DISCORD_PUBLIC_KEY").expect("DISCORD_PUBLIC_KEY must be set");
    static ref VERIFYING_KEY: VerifyingKey = {
        let public_key_bytes =
            hex::decode(&*PUBLIC_KEY).expect("Invalid hex in DISCORD_PUBLIC_KEY");
        let public_key_bytes: [u8; 32] = public_key_bytes
            .try_into()
            .expect("Invalid length for public key bytes");
        VerifyingKey::from_bytes(&public_key_bytes).expect("Invalid public key bytes")
    };
}

pub async fn register_commands() {
    let token = std::env::var("DISCORD_BOT_TOKEN").expect("DISCORD_BOT_TOKEN must be set");
    let application_id =
        std::env::var("DISCORD_APPLICATION_ID").expect("DISCORD_APPLICATION_ID must be set");

    let guild_ids = std::env::var("DISCORD_GUILD_IDS")
        .expect("DISCORD_GUILD_IDS must be set")
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();

    let client = reqwest::Client::new();

    for guild_id in guild_ids {
        let url = format!(
            "https://discord.com/api/v10/applications/{}/guilds/{}/commands",
            application_id, guild_id
        );

        let commands = vec![serde_json::json!({
            "name": "force",
            "type": 1,
            "description": "Force release the lease"
        })];

        let response = client
            .put(&url)
            .header("Authorization", format!("Bot {}", token))
            .json(&commands)
            .send()
            .await
            .expect("Failed to send request");

        if !response.status().is_success() {
            panic!(
                "Failed to register commands: {}",
                response.text().await.unwrap_or_default()
            );
        }
    }
}

async fn handle_force(_interaction: &Interaction, state: &Arc<AppState>) -> InteractionResponse {
    let mut session_store = state.session_store.lock().unwrap();
    *session_store = None;

    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("Lease has been forcefully released!!!".to_string()),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    }
}

async fn unknown_command_response() -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("Unknown command.".to_string()),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    }
}

async fn discord_callback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<InteractionResponse>, (StatusCode, String)> {
    let signature = headers
        .get("X-Signature-Ed25519")
        .ok_or((StatusCode::BAD_REQUEST, "Missing signature".to_string()))?
        .to_str()
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid signature format".to_string(),
            )
        })?;

    let signature_bytes = hex::decode(signature)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid signature hex".to_string()))?;
    let signature_bytes: [u8; 64] = signature_bytes.try_into().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid signature length".to_string(),
        )
    })?;
    let signature = Signature::from_bytes(&signature_bytes);

    let timestamp = headers
        .get("X-Signature-Timestamp")
        .ok_or((StatusCode::BAD_REQUEST, "Missing timestamp".to_string()))?
        .to_str()
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid timestamp format".to_string(),
            )
        })?;

    let message = [timestamp.as_bytes(), &body].concat();

    if VERIFYING_KEY.verify(&message, &signature).is_err() {
        return Err((StatusCode::UNAUTHORIZED, "Invalid signature".to_string()));
    }

    let interaction: Interaction = serde_json::from_slice(&body).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Failed to parse interaction".to_string(),
        )
    })?;

    match interaction.kind {
        InteractionType::Ping => {
            return Ok(Json(InteractionResponse {
                kind: InteractionResponseType::Pong,
                data: None,
            }));
        }
        InteractionType::ApplicationCommand => {
            if interaction.is_dm() {
                return Ok(Json(InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        content: Some(
                            "For security, this command cannot be used in DMs!".to_string(),
                        ),
                        flags: Some(MessageFlags::EPHEMERAL),
                        ..Default::default()
                    }),
                }));
            }

            if let Some(InteractionData::ApplicationCommand(command_data)) = &interaction.data {
                let response = match command_data.name.as_str() {
                    "force" => handle_force(&interaction, &state).await,
                    _ => unknown_command_response().await,
                };
                return Ok(Json(response));
            }
        }
        _ => {}
    }

    Err((
        StatusCode::BAD_REQUEST,
        "Unsupported interaction type".to_string(),
    ))
}

pub fn api_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/callback", axum::routing::post(discord_callback))
        .with_state(state)
}
