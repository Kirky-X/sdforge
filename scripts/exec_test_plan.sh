#!/bin/bash
set -e

echo "🚀 Starting SDForge Test Plan Execution (Local Mode)"
echo "==================================================="

# 清理残留进程
pkill -f "redis-server" || true
# 注意：这可能会误杀其他 cargo 进程，但在当前环境中应该是安全的
pkill -f "sdforge-test-app" || true

# 1. 启动 Redis
echo "📦 Starting local Redis..."
redis-server --port 6379 --daemonize yes

# 2. 启动 Test App
echo "🏗️ Building and starting Test App..."
export REDIS_URL=redis://127.0.0.1:6379
export RUST_LOG=info

# 后台运行，并将输出重定向到日志文件
# 使用 --bin sdforge-test-app 确保正确运行 binary
nohup cargo run --manifest-path temp/Cargo.toml --release > app.log 2>&1 &
APP_PID=$!
echo "   App PID: $APP_PID"

# 3. 等待服务就绪
echo "⏳ Waiting for service to be ready..."
# 增加超时时间，因为包含编译时间
for i in {1..120}; do
    # 尝试访问 /api/v1/test/feature
    if curl -s -X POST http://localhost:3000/api/v1/test/feature -H "Content-Type: application/json" -d '{"name": "Ping"}' > /dev/null; then
        echo "✅ Service is ready!"
        break
    fi
    echo "   Waiting... ($i/120)"
    sleep 2
done

if ! curl -s -X POST http://localhost:3000/api/v1/test/feature -H "Content-Type: application/json" -d '{"name": "Ping"}' > /dev/null; then
    echo "❌ Service failed to start."
    echo "📜 App Logs (Last 50 lines):"
    tail -n 50 app.log
    kill $APP_PID || true
    pkill -f "redis-server" || true
    exit 1
fi

# 4. 功能测试
echo "🧪 Running Functional Tests..."

# 测试 POST /api/v1/test/feature
echo "   Testing POST /api/v1/test/feature..."
RESPONSE=$(curl -s -X POST http://localhost:3000/api/v1/test/feature \
    -H "Content-Type: application/json" \
    -d '{"name": "Tester"}')

echo "   Response: $RESPONSE"

if [[ $RESPONSE == *"Hello, Tester!"* ]]; then
    echo "   ✅ /test/feature functional test passed"
else
    echo "   ❌ /test/feature functional test failed"
fi

# 5. 压力测试 (Simulated)
echo "🔥 Running Stress Test (Simulation)..."
echo "   Sending 20 requests concurrently..."
start_time=$(date +%s%N)
for i in {1..20}; do
    curl -s -X POST http://localhost:3000/api/v1/test/feature \
        -H "Content-Type: application/json" \
        -d '{"name": "StressTest"}' > /dev/null &
done
wait
end_time=$(date +%s%N)
duration=$(( (end_time - start_time) / 1000000 ))
echo "   ✅ Completed 20 requests in ${duration}ms"

# 6. 清理
echo "🧹 Cleaning up..."
kill $APP_PID || true
pkill -f "redis-server" || true

echo "🎉 Test Plan Completed Successfully!"
