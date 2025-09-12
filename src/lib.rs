use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use openai_flows::{
    chat::{ChatModel, ChatOptions},
    OpenAIFlows,
};
use slack_flows::{listen_to_channel, send_message_to_channel, SlackMessage};
use std::env;
use std::fs;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn run() {
    dotenv().ok();
    logger::init();
    let workspace: String = match env::var("slack_workspace") {
        Err(_) => "secondstate".to_string(),
        Ok(name) => name,
    };

    let channel: String = match env::var("slack_channel") {
        Err(_) => "collaborative-chat".to_string(),
        Ok(name) => name,
    };

    log::debug!("Workspace is {} and channel is {}", workspace, channel);

    // listen_to_channel is awaited here just like your working version
    listen_to_channel(&workspace, &channel, |sm| handler(sm, &workspace, &channel)).await;
}

async fn handler(sm: SlackMessage, workspace: &str, channel: &str) {
    let chat_id = format!("{}-{}", workspace, channel);

    // Configure the chat model (unchanged)
    let co = ChatOptions {
        model: ChatModel::GPT35Turbo,
        restart: false,
        system_prompt: Some("You are a helpful assistant inside Slack."),
        ..Default::default()
    };

    let openai = OpenAIFlows::new();

    // sm.text is already a String
    let user_text = sm.text;

    // --- Memory injection: read the launch plan file and prepend as context ---
    let launch_plan = fs::read_to_string("memory/launch_plan.txt").unwrap_or_default();
    // Build the final prompt the model will receive
    // Keep it concise: label the context and then the user request
    let full_text = if launch_plan.trim().is_empty() {
        user_text.clone()
    } else {
        format!(
            "Context (Launch Plan):\n{}\n\nUser request:\n{}",
            launch_plan.trim(),
            user_text
        )
    };

    match openai.chat_completion(&chat_id, &full_text, &co).await {
        Ok(response) => {
            let reply = response.choice;
            send_message_to_channel(workspace, channel, reply).await;
        }
        Err(err) => {
            log::error!("OpenAI call failed: {:?}", err);
            send_message_to_channel(
                workspace,
                channel,
                "⚠️ Sorry, I couldn't process that request.".to_string(),
            )
            .await;
        }
    }
}
