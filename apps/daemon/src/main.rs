//! NeuroLoom Daemon - Headless 后台守护进程

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "neuroloom_daemon=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("NeuroLoom Daemon starting...");

    // 初始化核心组件
    tracing::info!("Initializing core components...");

    // 初始化事件存储
    let event_store = nl_durable::EventStore::open("neuroloom.db").await?;
    tracing::info!("Event store initialized");

    // 初始化 Actor Mesh
    let actor_mesh = nl_durable::ActorMesh::new();
    tracing::info!("Actor mesh initialized with {} actors", actor_mesh.count().await);

    // 初始化令牌桶
    let token_bucket = nl_llm::TokenBucket::default_bucket();
    tracing::info!("Token bucket initialized with capacity {}", token_bucket.available_tokens());

    // 初始化 SOP 引擎
    let sop_engine = nl_cognitive::SopEngine::new();
    tracing::info!("SOP engine initialized with {} workflows", sop_engine.count());

    // 初始化记忆索引
    let memory_index = nl_memory::HamtIndex::new();
    tracing::info!("Memory index initialized");

    // 初始化 GraphRAG
    let graph_rag = nl_memory::GraphRAG::new();
    tracing::info!("GraphRAG initialized");

    // 初始化认知引擎
    let mcts_engine = nl_cognitive::MctsEngine::default_engine();
    tracing::info!("MCTS engine initialized");

    // 初始化法庭
    let courtroom = nl_cognitive::Courtroom::default_courtroom();
    tracing::info!("Courtroom initialized");

    // 初始化沙箱
    let sandbox = nl_sandbox::SandboxExecutor::new();
    tracing::info!("Sandbox executor initialized");

    // 初始化 HAP 服务器
    let hap_server = nl_hap::HapServer::default_server();
    tracing::info!("HAP server configured on {}", hap_server.config().addr);

    tracing::info!("NeuroLoom Daemon is ready!");
    tracing::info!("Press Ctrl+C to shutdown...");

    // 等待关闭信号
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down...");

    Ok(())
}
