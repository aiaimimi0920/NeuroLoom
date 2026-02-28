use crate::client::ClientBuilder;
use crate::model::azure_openai::AzureOpenAiModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::azure_openai::AzureOpenAiExtension;
use crate::site::base::azure::AzureOpenAiSite;
use std::sync::Arc;

/// Azure OpenAI 预设
///
/// ## 使用方式
///
/// Azure OpenAI 需要用户提供自己的 endpoint 和 deployment name：
///
/// ```rust,no_run
/// use nl_llm::LlmClient;
///
/// let client = LlmClient::from_preset("azure_openai")
///     .expect("preset")
///     // 必须设置你的 Azure 资源 endpoint
///     .with_base_url("https://YOUR-RESOURCE.openai.azure.com")
///     // 密钥使用 api-key header
///     .with_api_key("YOUR-AZURE-API-KEY")
///     .build();
///
/// // 模型名 = 你的 deployment name
/// // let req = PrimitiveRequest::single_user_message("hello")
/// //     .with_model("my-gpt4o-deployment");
/// ```
///
/// > **重要**: 必须用 `with_base_url()` 设置你的 Azure 资源端点
/// > 模型名就是你在 Azure 中创建的 deployment 名称
const AZURE_PLACEHOLDER_ENDPOINT: &str = "https://YOUR-RESOURCE.openai.azure.com";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(AzureOpenAiSite::new(AZURE_PLACEHOLDER_ENDPOINT))
        .protocol(OpenAiProtocol {})
        .model_resolver(AzureOpenAiModelResolver::new())
        .with_extension(Arc::new(AzureOpenAiExtension::new()))
        .default_model("gpt-4o")
}
