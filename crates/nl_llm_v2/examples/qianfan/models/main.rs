//! 百度千帆 模型列表

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  百度千帆 模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("qianfan")
        .expect("Preset should exist")
        .with_api_key("placeholder")
        .build();

    match client.list_models().await {
        Ok(models) => {
            println!("共 {} 个模型:\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {} — {}", i + 1, m.id, m.description);
            }
        }
        Err(e) => { println!("获取失败: {}", e); }
    }

    println!("\n----------------------------------------");
    println!("  常用别名");
    println!("----------------------------------------\n");
    let aliases = [
        ("qianfan / ernie / 文心", "ernie-4.5-turbo-128k"),
        ("4.5", "ernie-4.5-turbo-128k"),
        ("4.0", "ernie-4.0-turbo-128k"),
        ("3.5", "ernie-3.5-128k"),
        ("speed", "ernie-speed-128k"),
        ("lite", "ernie-lite-128k"),
        ("tiny", "ernie-tiny-8k"),
    ];
    for (a, t) in aliases { println!("  '{}' -> '{}'", a, t); }

    println!("\n----------------------------------------");
    println!("  价格说明 (元/千token)");
    println!("----------------------------------------\n");
    println!("  ERNIE 4.5: ¥0.004/¥0.012 (输入/输出)");
    println!("  ERNIE 4.0: ¥0.03/¥0.09");
    println!("  ERNIE 3.5: ¥0.001/¥0.002");
    println!("  ERNIE Speed/Lite/Tiny: 免费");

    println!("\n----------------------------------------");
    println!("  认证配置说明");
    println!("----------------------------------------\n");
    println!("  环境变量: QIANFAN_API_KEY=xxx");
    println!("\n  获取密钥:");
    println!("    1. 注册百度智能云: https://cloud.baidu.com");
    println!("    2. 进入千帆大模型平台: https://qianfan.cloud.baidu.com");
    println!("    3. 创建应用 → 获取 API Key");

    Ok(())
}
