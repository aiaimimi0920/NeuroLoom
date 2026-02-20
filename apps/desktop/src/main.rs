//! NeuroLoom Desktop - 空间流式画布前端

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "neuroloom_desktop=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("NeuroLoom Desktop starting...");

    // TODO: 初始化 Tauri 前端
    // 这里是骨架实现，后续需要集成 Tauri

    tracing::info!("NeuroLoom Desktop is ready!");

    // 保持运行
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down...");

    Ok(())
}
