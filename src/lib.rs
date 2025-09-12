use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use openai_flows::{
    chat::{ChatModel, ChatOptions},
    OpenAIFlows,
};
use slack_flows::{listen_to_channel, send_message_to_channel, SlackMessage};
use std::env;

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

    listen_to_channel(&workspace, &channel, |sm| handler(sm, &workspace, &channel)).await;
}

async fn handler(sm: SlackMessage, workspace: &str, channel: &str) {
    let chat_id = format!("{}-{}", workspace, channel);

    // 🔹 Fetch launch plan dynamically from GitHub
    let plan_url = "https://raw.githubusercontent.com/thebenewco/Slack-Chatgpt/main/memory/launch_plan.txt";
    let launch_plan = match reqwest::get(plan_url).await {
        Ok(resp) => resp.text().await.unwrap_or_else(|_| "".to_string()),
        Err(_) => "".to_string(),
    };

    // Combine launch plan + user text
    let user_text = format!(
        "Context:\n{}\n\nUser request: {}",
        launch_plan, sm.text
    );

    // Configure the chat model
    let co = ChatOptions {
        model: ChatModel::GPT35Turbo,
        restart: false,
        system_prompt: Some("You are a helpful assistant inside Slack. Always use the Launch Plan context when relevant."),
        ..Default::default()
    };

    let openai = OpenAIFlows::new();

    match openai.chat_completion(&chat_id, &user_text, &co).await {
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
