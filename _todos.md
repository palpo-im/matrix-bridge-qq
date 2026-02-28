# Matrix-Bridge-QQ 功能完善清单

基于对比 `matrix-bridge-discord` 和 `references/matrix-qq` 项目，列出以下功能缺失点。

## Phase 1: 消息解析与转换 (Message Parsing & Conversion)

- [ ] 1.1 创建 `src/parsers` 模块
  - [ ] 创建 `src/parsers/mod.rs`
  - [ ] 创建 `src/parsers/matrix_parser.rs` - Matrix 消息解析器
  - [ ] 创建 `src/parsers/qq_parser.rs` - QQ 消息解析器
  - [ ] 创建 `src/parsers/common.rs` - 通用消息类型定义

- [ ] 1.2 QQ 消息类型支持
  - [ ] 支持图片消息 (m.image -> QQ image)
  - [ ] 支持视频消息 (m.video -> QQ video)
  - [ ] 支持音频消息 (m.audio -> QQ voice)
  - [ ] 支持文件消息 (m.file -> QQ file)
  - [ ] 支持 @提及 (AtElement)
  - [ ] 支持表情消息 (FaceElement)
  - [ ] 支持回复消息 (ReplyElement)
  - [ ] 支持转发消息 (ForwardMessage)
  - [ ] 支持位置消息 (LocationShare)

- [ ] 1.3 Matrix -> QQ 消息转换
  - [ ] HTML 转 QQ 消息格式
  - [ ] Matrix @mention 转 QQ @
  - [ ] Matrix 媒体转 QQ 媒体

## Phase 2: 媒体处理 (Media Handling)

- [ ] 2.1 创建 `src/media.rs` 模块
  - [ ] 下载 Matrix 媒体文件
  - [ ] 上传媒体到 Matrix
  - [ ] 从 QQ 下载媒体
  - [ ] 上传媒体到 QQ (via OneBot API)
  - [ ] MIME 类型检测
  - [ ] 文件大小限制检查

- [ ] 2.2 音频格式转换
  - [ ] Silk -> Ogg 转换 (QQ -> Matrix)
  - [ ] Ogg -> Silk 转换 (Matrix -> QQ)

## Phase 3: 高级桥接功能 (Advanced Bridge Features)

- [ ] 3.1 消息编辑支持
  - [ ] Matrix 消息编辑转发到 QQ
  - [ ] QQ 消息编辑转发到 Matrix

- [ ] 3.2 消息撤回支持
  - [ ] Matrix 红action 转发到 QQ (删除)
  - [ ] QQ 撤回消息转发到 Matrix

- [ ] 3.3 回复消息支持
  - [ ] Matrix 回复转发到 QQ
  - [ ] QQ 回复转发到 Matrix

- [ ] 3.4 正在输入状态
  - [ ] QQ 输入状态转发到 Matrix

- [ ] 3.5 房间状态同步
  - [ ] 房间名称变更同步
  - [ ] 房间主题变更同步

## Phase 4: 命令系统 (Command System)

- [ ] 4.1 Matrix 端命令
  - [ ] 创建 `src/matrix/command_handler.rs`
  - [ ] 实现 `!qq ping` 命令
  - [ ] 实现 `!qq bridge <group_id>` 命令
  - [ ] 实现 `!qq unbridge` 命令
  - [ ] 权限检查

- [ ] 4.2 QQ 端命令
  - [ ] 创建 `src/qq/command_handler.rs`
  - [ ] 实现基本命令支持

## Phase 5: Web 服务增强 (Web Service Enhancement)

- [ ] 5.1 创建 `src/web` 模块
  - [ ] 创建 `src/web/mod.rs`
  - [ ] 创建 `src/web/health.rs` - 健康检查端点
  - [ ] 创建 `src/web/metrics.rs` - Prometheus 指标端点
  - [ ] 创建 `src/web/provisioning.rs` - 桥接管理 API
  - [ ] 创建 `src/web/thirdparty.rs` - 第三方协议 API

- [ ] 5.2 Provisioning API
  - [ ] GET /_matrix/app/v1/rooms - 列出桥接房间
  - [ ] POST /_matrix/app/v1/bridges - 创建桥接
  - [ ] GET /_matrix/app/v1/bridges/{id} - 获取桥接信息
  - [ ] DELETE /_matrix/app/v1/bridges/{id} - 删除桥接

## Phase 6: 缓存系统 (Caching System)

- [ ] 6.1 创建 `src/cache.rs` 模块
  - [ ] 实现 TimedCache (TTL 缓存)
  - [ ] 实现 AsyncTimedCache (异步 TTL 缓存)
  - [ ] 房间映射缓存
  - [ ] 用户信息缓存

## Phase 7: 数据库增强 (Database Enhancement)

- [ ] 7.1 添加 MySQL 支持
  - [ ] 创建 `src/database/mysql.rs`
  - [ ] MySQL schema 定义

- [ ] 7.2 新增数据表
  - [ ] user_mapping 表 - Matrix 用户与 QQ 用户映射
  - [ ] emoji_mapping 表 - 表情映射缓存

- [ ] 7.3 数据库存储分离
  - [ ] 创建 `src/database/stores/mod.rs`
  - [ ] 创建 MessageStore
  - [ ] 创建 RoomStore
  - [ ] 创建 UserStore

## Phase 8: 配置增强 (Configuration Enhancement)

- [ ] 8.1 新增配置选项
  - [ ] 消息编辑转发开关
  - [ ] 消息删除转发开关
  - [ ] 正在输入状态开关
  - [ ] 房间状态同步开关
  - [ ] 最大文件大小配置
  - [ ] 房间数量限制
  - [ ] 用户数量限制

## Phase 9: 工具模块 (Utility Modules)

- [ ] 9.1 创建 `src/utils` 模块
  - [ ] 创建 `src/utils/mod.rs`
  - [ ] 创建 `src/utils/error.rs` - 自定义错误类型
  - [ ] 创建 `src/utils/formatting.rs` - 格式化工具
  - [ ] 创建 `src/utils/logging.rs` - 日志初始化

## Phase 10: 测试与文档 (Testing & Documentation)

- [ ] 10.1 单元测试
  - [ ] 消息解析器测试
  - [ ] 媒体处理测试
  - [ ] 缓存测试

- [ ] 10.2 文档
  - [ ] 更新 README.md
  - [ ] 添加配置示例说明
  - [ ] 添加 API 文档

---

## 优先级说明

- **Phase 1-2**: 核心功能，必须完成
- **Phase 3-4**: 高级功能，建议完成
- **Phase 5-7**: 增强功能，可选完成
- **Phase 8-10**: 优化功能，最后完成
