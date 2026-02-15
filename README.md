# matrix-bridge-qq

基于 Rust + Salvo 的 Matrix <-> QQ 双向桥接（OneBot v11 适配）。

## 功能范围（当前实现）

- QQ -> Matrix：
  - 监听 OneBot 反向 HTTP 消息事件（私聊/群聊文本）
  - 自动创建并持久化 portal（QQ 会话 <-> Matrix room）
  - 使用 QQ 虚拟用户（puppet）向 Matrix 房间发消息
- Matrix -> QQ：
  - 接收 AppService transaction
  - 将 `m.room.message` 文本消息转发到 QQ 私聊/群聊
- 基础保障：
  - Matrix transaction 去重
  - 消息去重（QQ message_id / Matrix event_id）
  - OneBot 签名校验（`X-Signature: sha1=...`）

## 快速开始

1. 生成配置模板：

```bash
cargo run -- --generate-config > config.yaml
```

2. 编辑 `config.yaml`，至少修改：

- `homeserver.address`
- `homeserver.domain`
- `appservice.as_token`
- `appservice.hs_token`
- `bridge.onebot.api_base`
- `bridge.onebot.listen_secret` / `bridge.onebot.access_token`

3. 启动桥：

```bash
cargo run -- --config config.yaml
```

4. 对接端点：

- Matrix AppService：
  - `PUT /_matrix/app/v1/transactions/{txn_id}`
  - `GET /_matrix/app/v1/users/{user_id}`
  - `GET /_matrix/app/v1/rooms/{room_alias}`
- QQ OneBot 事件回调：
  - `POST {bridge.onebot.event_path}`（默认 `/qq/events`）

## 技术栈

- Web 框架：`salvo`
- 数据库：`sqlx`（SQLite / PostgreSQL）
- Matrix 交互：AppService + Client-Server API（AS Token）
- QQ 交互：OneBot v11 HTTP API + 反向 HTTP 事件

## 后续可扩展

- 媒体消息（图片/文件/语音）
- 撤回（redaction）
- 群成员同步与名称/头像同步
- 管理命令与权限细化
