use nl_llm_v2::client::LlmClient;
use nl_llm_v2::primitive::PrimitiveRequest;
use std::env;

#[tokio::test]
async fn test_poe_chat() {
    let Ok(api_key) = env::var("POE_API_KEY") else {
        eprintln!("skip test_poe_chat: missing POE_API_KEY");
        return;
    };

    let client = LlmClient::from_preset("poe")
        .expect("poe preset should exist")
        .with_api_key(api_key)
        .build();
    
    let req = PrimitiveRequest::single_user_message("Hello, who are you? Please answer in one short sentence.")
        .with_model("GPT-4o");

    let resp = client.complete(&req).await;

    match resp {
        Ok(r) => {
            println!("Response: {}", r.content);
            assert!(!r.content.is_empty(), "模型返回了空的消息");
        }
        Err(e) => {
            let err_msg = e.to_string();
            println!("API returned an error (expected if key has no subscription): {}", err_msg);
            assert!(
                err_msg.contains("subscription_required") || err_msg.contains("requires an active Poe subscription"),
                "Unexpected API error: {}", err_msg
            );
        }
    }
}
