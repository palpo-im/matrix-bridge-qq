# Matrix-Bridge-QQ TODOs

## 并行任务分组

### Track A: 项目骨架与基础设施（可立即并行）
- [x] A1. 初始化 Rust + Salvo 项目骨架（`Cargo.toml`, `src/main.rs`, `src/lib.rs`）
- [x] A2. 建立配置模块（YAML 解析、校验、示例配置）
- [x] A3. 建立数据库模块（SQLite/Postgres 连接、migrations、基础仓储）
- [x] A4. 建立通用日志与错误处理（tracing + anyhow）

### Track B: Matrix AppService 侧（与 A 并行）
- [x] B1. 实现 Matrix HTTP Client（发消息、建房、邀请、设置 profile）
- [x] B2. 实现 AppService HTTP 入口（`/_matrix/app/v1/transactions/*`）
- [x] B3. 实现 Matrix 事件解析（过滤命令、文本消息、去重）
- [x] B4. 实现 Matrix->QQ 出站路由

### Track C: QQ 接入侧（与 A/B 并行）
- [x] C1. 选定并实现 QQ 协议适配（OneBot v11 webhook + HTTP API）
- [x] C2. 实现 QQ 事件接收（私聊/群聊文本）
- [x] C3. 实现 QQ 发消息客户端（私聊/群聊）
- [x] C4. 实现 QQ 事件签名/鉴权与幂等处理

### Track D: 桥接核心（依赖 A+B+C，可分段并行）
- [x] D1. 设计 portal/user/message 映射模型
- [x] D2. QQ->Matrix：自动创建/绑定房间并转发消息
- [x] D3. Matrix->QQ：从房间反查 QQ 会话并发送
- [x] D4. 消息回写映射（event_id <-> message_id）

### Track E: 运行与交付（后置）
- [x] E1. 补充 `example-config.yaml` 与 README（部署、接入 OneBot）
- [x] E2. 本地编译与关键路径自测
- [x] E3. 输出后续扩展点（图片、撤回、成员同步）

## 任务依赖与并行策略
- A/B/C 三条主线并行开发；D 在 A/B/C 的基础上分段落地。
- 先打通最小闭环：`QQ 文本 -> Matrix` + `Matrix 文本 -> QQ`，再补强幂等与文档。
- 每个 Track 结束即编译一次，减少集成阶段返工。
