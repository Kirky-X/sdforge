//! 02_mcp_tool - MCP 协议示例
//!
//! 这个示例演示如何使用 Axiom 框架创建 MCP (Model Context Protocol) 工具。
//!
//! 运行方式:
//! ```bash
//! cargo run --bin 02_mcp_tool
//! ```
//!
//! MCP 协议用于 AI 工具集成，工具通过 stdio 与 AI 模型交互。

use axiom::prelude::*;
use axiom::service_api;
use serde::{Deserialize, Serialize};

// ============================================================================
// 数据模型
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct CalculatorResult {
    operation: String,
    operand1: f64,
    operand2: f64,
    result: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct WeatherInfo {
    city: String,
    temperature: f64,
    humidity: f64,
    condition: String,
}

// ============================================================================
// MCP 工具定义
// ============================================================================

/// 加法计算
#[service_api(
    name = "add_numbers",
    version = "v1",
    description = "Add two numbers together",
    tool_name = "add"
)]
async fn add_numbers(a: f64, b: f64) -> Result<CalculatorResult, ApiError> {
    Ok(CalculatorResult {
        operation: "addition".to_string(),
        operand1: a,
        operand2: b,
        result: a + b,
    })
}

/// 减法计算
#[service_api(
    name = "subtract_numbers",
    version = "v1",
    description = "Subtract second number from first number",
    tool_name = "subtract"
)]
async fn subtract_numbers(a: f64, b: f64) -> Result<CalculatorResult, ApiError> {
    Ok(CalculatorResult {
        operation: "subtraction".to_string(),
        operand1: a,
        operand2: b,
        result: a - b,
    })
}

/// 乘法计算
#[service_api(
    name = "multiply_numbers",
    version = "v1",
    description = "Multiply two numbers",
    tool_name = "multiply"
)]
async fn multiply_numbers(a: f64, b: f64) -> Result<CalculatorResult, ApiError> {
    Ok(CalculatorResult {
        operation: "multiplication".to_string(),
        operand1: a,
        operand2: b,
        result: a * b,
    })
}

/// 除法计算
#[service_api(
    name = "divide_numbers",
    version = "v1",
    description = "Divide first number by second number",
    tool_name = "divide"
)]
async fn divide_numbers(a: f64, b: f64) -> Result<CalculatorResult, ApiError> {
    if b == 0.0 {
        return Err(ApiError::InvalidInput {
            message: "Division by zero is not allowed".to_string(),
            field: Some("b".to_string()),
            value: Some(serde_json::json!(b)),
        });
    }

    Ok(CalculatorResult {
        operation: "division".to_string(),
        operand1: a,
        operand2: b,
        result: a / b,
    })
}

/// 获取天气信息（模拟）
#[service_api(
    name = "get_weather",
    version = "v1",
    description = "Get current weather information for a city",
    tool_name = "weather"
)]
async fn get_weather(city: String) -> Result<WeatherInfo, ApiError> {
    if city.is_empty() {
        return Err(ApiError::InvalidInput {
            message: "City name cannot be empty".to_string(),
            field: Some("city".to_string()),
            value: Some(serde_json::json!(city)),
        });
    }

    // 模拟天气数据
    Ok(WeatherInfo {
        city: city.clone(),
        temperature: 22.5,
        humidity: 65.0,
        condition: "Partly Cloudy".to_string(),
    })
}

/// 文本分析
#[service_api(
    name = "analyze_text",
    version = "v1",
    description = "Analyze text and return statistics",
    tool_name = "analyze_text"
)]
async fn analyze_text(text: String) -> Result<serde_json::Value, ApiError> {
    if text.is_empty() {
        return Err(ApiError::InvalidInput {
            message: "Text cannot be empty".to_string(),
            field: Some("text".to_string()),
            value: Some(serde_json::json!(text)),
        });
    }

    let word_count = text.split_whitespace().count();
    let char_count = text.chars().count();
    let line_count = text.lines().count();

    Ok(serde_json::json!({
        "word_count": word_count,
        "char_count": char_count,
        "line_count": line_count,
        "has_numbers": text.chars().any(|c| c.is_numeric()),
        "has_uppercase": text.chars().any(|c| c.is_uppercase()),
        "has_lowercase": text.chars().any(|c| c.is_lowercase())
    }))
}

// ============================================================================
// 主函数
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    println!("========================================");
    println!("Axiom MCP 工具示例");
    println!("========================================");
    println!();

    // 构建 MCP 服务器
    let server = axiom::mcp::build().await;

    println!("✅ MCP 服务器构建成功");
    println!();
    println!("📡 MCP 服务器已启动 (stdio 模式)");
    println!();
    println!("🔧 可用的 MCP 工具:");
    println!("  1. add       - 加法计算");
    println!("  2. subtract  - 减法计算");
    println!("  3. multiply  - 乘法计算");
    println!("  4. divide    - 除法计算");
    println!("  5. weather   - 获取天气信息");
    println!("  6. analyze_text - 文本分析");
    println!();
    println!("📝 工具调用示例 (JSON 格式):");
    println!("  {{");
    println!("    \"tool\": \"add\",");
    println!("    \"arguments\": {{");
    println!("      \"a\": 10,");
    println!("      \"b\": 5");
    println!("    }}");
    println!("  }}");
    println!();
    println!("按 Ctrl+C 停止服务");
    println!("========================================");
    println!();

    // MCP 服务器通过 stdio 运行
    // 实际使用中，MCP 客户端会通过 stdin/stdout 与服务器通信
    println!("等待 MCP 客户端连接...");
    println!("提示: 此示例需要通过 MCP 客户端（如 Claude Desktop）进行测试");
    println!();

    // 模拟一些工具调用（用于演示）
    println!("📊 演示工具调用:");
    println!();

    // 演示加法
    let result = add_numbers(10.0, 5.0).await?;
    println!("  add(10, 5) = {}", serde_json::to_string_pretty(&result)?);

    // 演示天气查询
    let weather = get_weather("Beijing".to_string()).await?;
    println!("  weather('Beijing') = {}", serde_json::to_string_pretty(&weather)?);

    // 演示文本分析
    let analysis = analyze_text("Hello World! This is a test 123.".to_string()).await?;
    println!("  analyze_text(...) = {}", serde_json::to_string_pretty(&analysis)?);

    println!();
    println!("✅ 工具调用演示完成");
    println!();

    // 保持运行（在实际 MCP 服务器中，这里会处理 stdin/stdout）
    tokio::signal::ctrl_c().await?;
    println!("\n👋 MCP 服务器已停止");

    Ok(())
}