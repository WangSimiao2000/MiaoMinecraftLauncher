# 整合包源生态 — Phase 1 落地 Spec

> 状态：**实施 spec** — 本文档把 [`modpack-source.md`](modpack-source.md) §4.1 / §4.2 / §4.3 收敛为 Phase 1 可执行设计。
> 范围：仅 Phase 1（C 模型基础：schema + 解算 + UI + 安装）。Phase 2（跨版本升级）与 Phase 3（CI 验证）不在本文档。
> 目标：完成本 spec 列出的所有内容后，玩家能从 GUI 选「米奇喵源」中的某个整合包，选 MC 版本，看到兼容性预览，一键安装；安装结果是一个可启动的 MMCL 实例。

---

## 0. 范围与非目标

### 0.1 Phase 1 范围

- 单一内置源：**米奇喵源**（URL 内置）。代码层按多源架构写，UI 层只暴露这一个。
- manifest schema 定型 + 加载 + 缓存。
- mod 兼容性解算（按用户选定的 MC 版本，调 Modrinth 批量接口算每个 mod 的具体版本）。
- 安装流：装 loader → 批量装 mod → 写配置 overlay → 落实例。
- GUI 入口：「+ 新建实例」对话框新增「整合包」Tab；实例详情新增「整合包同步」Tab（Phase 1 只读 + 当前 manifest 版本/mod 状态显示，不做"应用更新"按钮——那是 Phase 2）。

### 0.2 Phase 1 非目标

- ❌ 跨 MC 版本升级流程 → Phase 2
- ❌ Pending mod 后台补装 → Phase 2
- ❌ 弃坑 mod 替代 → Phase 2
- ❌ CI 验证 / probe mode / CLI resolver / probe mod → Phase 3
- ❌ 第三方源添加 UI / "未验证" 警告 → Phase 2 末尾或 Phase 3
- ❌ 配置 overlay 的"作者发布更新→玩家选择是否覆盖"流程 → Phase 2（first-install-only 已够 Phase 1）
- ❌ 整合包"分叉"导出 / 分享（[`modpack-source.md` §9](modpack-source.md#L362)）→ 不在 1-3 阶段

---

## 1. 阻塞 §9 决策项的 Phase 1 答案

[`modpack-source.md` §9](modpack-source.md#L361) 列了 10 项待决策。Phase 1 必须先答以下条目，剩余推迟到 Phase 2/3：

| §9 条目 | Phase 1 决策 |
|---|---|
| manifest schema 完整字段 | 见 §2，本 spec 定型 |
| 解算算法边界（循环 required） | 检测到环时报错并中止解算，UI 显示具体环路径；不自动断环。见 §3.4 |
| resolved.json schema | 见 §3.5。Phase 1 仅用作内存结构 + 实例侧落盘缓存，CI 共享留 Phase 3 |
| 第三方作者文档 | 推迟 Phase 2 |
| 配置 overlay "重置为推荐"按钮 | Phase 1 不做按钮，但安装时把 overlay 元信息写入 `instance.toml`，给 Phase 2 留挂钩 |
| probe mod / CI badge / telemetry / 携带存档 / 弃坑替代 / 分叉 | 全部 Phase 2/3 |

剩余未决项（probe mod 实现路径、CI badge 展示、telemetry、携带存档策略、弃坑替代版本约束、分叉导出）在 Phase 1 不涉及，无需在本 spec 决策。

---

## 2. Manifest Schema（定型）

两份 JSON 文件，均通过 HTTPS 公开访问。

### 2.1 `manifest.json`（源根）

```jsonc
{
  "schema_version": 1,                    // u32, 兼容性栅栏；Phase 1 = 1
  "source_id": "miao",                    // [a-z0-9-]{2,32}, 源唯一 ID
  "source_name": "米奇喵整合包源",
  "author": "mickeymiao",
  "homepage": "https://space.bilibili.com/36913332",
  "updated_at": "2026-05-29T10:00:00Z",   // RFC3339
  "packs": [
    {
      "id": "miao-1.21-base",             // [a-z0-9-]{2,64}, 源内唯一
      "display_name": "米奇喵 1.21 Tricky Trials 基础整合包",
      "summary": "短描述（GUI 卡片用），≤120 字符",
      "mc_versions": ["1.21.1", "1.21.4", "1.21.5"],
      "loader": "fabric",                 // "fabric"|"forge"|"neoforge"|"quilt"
      "support_level": "active",          // "active"|"maintenance"|"archived"
      "pack_url": "https://raw.githubusercontent.com/mickeymiao/miao-modpacks/main/packs/miao-1.21-base/pack.json"
    }
  ]
}
```

**校验规则**：
- `schema_version` 缺失或 > 1 → 拒绝加载，UI 显示「源版本不兼容，请升级 MMCL」
- `source_id` 必须匹配正则 `^[a-z0-9-]{2,32}$`
- `pack_url` 必须 https，host 必须在 manifest 同 origin **或** `raw.githubusercontent.com` / `cdn.jsdelivr.net`（Phase 1 白名单，避免任意 URL）
- `mc_versions` 至少 1 个，每项必须匹配 `^\d+\.\d+(\.\d+)?$`

### 2.2 `pack.json`（整合包根）

```jsonc
{
  "schema_version": 1,
  "id": "miao-1.21-base",                 // 必须与 manifest.packs[].id 一致
  "display_name": "米奇喵 1.21 Tricky Trials 基础整合包",
  "version": "2026.5.29",                 // 字符串，作者自定义版本号；用于 Phase 2 升级检测
  "summary": "...",
  "description": "Markdown 长描述（GUI 详情面板用）",
  "loader": "fabric",
  "mc_versions": ["1.21.1", "1.21.4", "1.21.5"],
  "loader_versions": {                    // 可选；缺省时 MMCL 自动选最新稳定 loader 版本
    "1.21.5": "0.16.10"
  },
  "mods": [
    {
      "source": "modrinth",               // "modrinth"|"curseforge"
      "id": "AANobbMI",                   // Modrinth project_id 或 CF mod_id
      "name": "Sodium",                   // 显示名（不参与解算，仅 UI）
      "policy": "auto",                   // "auto"=跟随最新兼容版 | "lock"=锁版本
      "locked_version": null,             // policy=lock 时必填，是 Modrinth version_id 或 CF file_id
      "criticality": "core"               // "core"|"optional"，Phase 1 仅展示，不参与升级判定（升级判定在 Phase 2）
      // deprecated_after / replacement 是 Phase 2 字段，Phase 1 解析时忽略
    }
  ],
  "config_overlay": {
    "base_url": "https://raw.githubusercontent.com/mickeymiao/miao-modpacks/main/packs/miao-1.21-base/config",
    "files": [
      {
        "path": "_common/options.txt",    // 相对 base_url 的路径
        "target": "options.txt",          // 写入实例时的相对路径（相对 instance 根）
        "sha256": "abc123..."             // 必填，下载完校验
      },
      {
        "path": "1.21/config/sodium-options.json",
        "target": "config/sodium-options.json",
        "applies_to_mc": ["1.21.1", "1.21.4", "1.21.5"],  // 缺省 = 全部
        "sha256": "def456..."
      }
    ]
  }
}
```

**配置 overlay 解析**（Phase 1 = first-install-only，[`modpack-source.md` §5.1](modpack-source.md#L277)）：
- `_common/` 下的文件无条件写入
- 其它路径下的文件，若 `applies_to_mc` 包含玩家选定版本 → 写入；否则跳过
- 同一 `target` 出现多次按 `files[]` 顺序后写覆盖前
- 写入前必须 sha256 校验，失败 → 整体安装失败回滚（已写入文件清单见 §4.4）

### 2.3 缓存策略

[`modpack-source.md` §4.3 性能要求](modpack-source.md#L186) 缓存 6-12 小时。Phase 1 取 **6 小时**。

- 缓存目录：`<data-dir>/modpack-sources/<source_id>/`
  - `manifest.json` — 上次拉取的 manifest 原文
  - `manifest.meta.json` — `{ "fetched_at": "RFC3339", "etag": "...", "url": "..." }`
  - `packs/<pack_id>/pack.json` — 同上
  - `packs/<pack_id>/pack.meta.json`
- 加载顺序：本地缓存（若 < 6 小时且非强刷）→ 网络拉取 → 失败兜底用任意 age 的本地缓存（[`modpack-source.md` §5.3 离线优先](modpack-source.md#L293)）
- ETag/If-None-Match 走 reqwest，命中 304 → 仅更新 `fetched_at`

---

## 3. 解算（Resolver）

### 3.1 输入 / 输出

输入：
- `pack: ResolvedPack`（已加载的 pack.json）
- `mc_version: String`（玩家选定，必须 ∈ `pack.mc_versions`）
- `loader: ModLoaderType`（取自 `pack.loader`）

输出：`ResolutionReport`，详见 §3.5。

### 3.2 算法

[`modpack-source.md` §4.3](modpack-source.md#L184) 明确"贪心 + 回溯，不需要 SAT solver"。Phase 1 实现：

1. **声明 mod 拉详情**：
   - Modrinth：一次 `/v2/projects?ids=[...]` + 一次 `/v2/versions?ids=[...]`（如有 lock）+ 按 mod 各一次 `/v2/project/{id}/version?game_versions=[mc]&loaders=[loader]`
   - CurseForge：批量 `/v1/mods` + 单个 `/v1/mods/{id}/files` 过滤
   - 单 pack 解算需在 5 秒内（[`modpack-source.md` §4.3](modpack-source.md#L189)）→ 全部并发，但限制 16 并发（reqwest semaphore）
2. **每个 mod 选版本**：
   - `policy=lock` → 用 `locked_version`，若该版本不支持 mc/loader → 标 `Conflict`
   - `policy=auto` → 候选版本 = 该 project 在 mc + loader 下发布过的所有 version；按发布日期降序取第一个 stable。无候选 → 状态分类按 §3.3
3. **依赖补全**（贪心 + 回溯）：
   - 对每个已选 version 的 `dependencies[]`，类型 = `required` 且不在当前结果集 → 把它当作新 mod 加入待解算队列
   - 类型 = `incompatible` → 检查冲突方是否已在结果集；是 → 报告冲突，**不自动剔除**，留给玩家判断（Phase 1：UI 显示警告，玩家可放弃安装）
   - 回溯触发：dep 自身解算失败时，回到引入它的 mod 尝试次新版本；最多回溯深度 = 3，超过 → 标记为 `Conflict`
4. **环检测**：用 DFS 灰/黑标记，遇到灰节点 → 输出环路径，整体解算失败（不自动断环）

### 3.3 Mod 状态分类

完全对齐 [`modpack-source.md` §4.3](modpack-source.md#L172)：

| 状态 | Phase 1 判定 |
|---|---|
| `Compatible` | Modrinth/CF 在 (mc, loader) 下有 stable 版本 |
| `Pending` | 当前 (mc, loader) 无版本，但该 project 在过去 6 个月有发布过任何版本 |
| `Abandoned` | 该 project 任何版本最近发布 > 6 个月前 |
| `Conflict` | 有候选版本但依赖冲突 / 回溯耗尽 |

### 3.4 循环依赖处理

环路径展示形如 `A → B → C → A`，UI 显示后玩家只能放弃此次解算（Phase 1）。Phase 2 再考虑「弃坑替代」是否能断环。

### 3.5 `ResolutionReport` 数据结构

```rust
// crates/core/src/modpack_source/resolver.rs
pub struct ResolutionReport {
    pub pack_id: String,
    pub pack_version: String,
    pub mc_version: String,
    pub loader: ModLoaderType,
    pub loader_version: String,            // 解算出的 loader 版本
    pub resolved_at: chrono::DateTime<chrono::Utc>,
    pub mods: Vec<ResolvedMod>,
    pub conflicts: Vec<ConflictReport>,    // 来自 incompatible 声明
    pub cycle: Option<Vec<String>>,        // 若检测到环，整体解算视为失败
}

pub struct ResolvedMod {
    pub source: ModSource,                 // Modrinth | CurseForge
    pub project_id: String,
    pub display_name: String,
    pub criticality: Criticality,
    pub status: ResolvedStatus,
    // 仅 Compatible 状态填充
    pub version_id: Option<String>,
    pub file_url: Option<String>,
    pub file_sha512: Option<String>,
    pub file_size: Option<u64>,
    pub file_name: Option<String>,
    pub auto_added_for: Option<String>,    // 若是依赖补全，填上游 mod project_id
}

pub enum ResolvedStatus {
    Compatible,
    Pending { last_release: chrono::DateTime<chrono::Utc> },
    Abandoned { last_release: Option<chrono::DateTime<chrono::Utc>> },
    Conflict { reason: String },
}
```

实例落盘位置：`<instance_dir>/.miao-modpack/resolved.json`，给 Phase 2 升级流程读。

---

## 4. 安装流（Installer）

### 4.1 顺序（必须严格）

1. **预检**：`ResolutionReport.cycle == None` 且没有 `core` mod 是 `Conflict` / `Abandoned`（`Pending` 允许，玩家已确认）。
2. **创建实例**：调用 `core::instance` 现有 API，名称由 GUI 传入；目录 `<data-dir>/instances/<name>/`。
3. **装 loader**：调 `core::modloader`（已存在）按 `loader + loader_version + mc_version`。
4. **批量装 mod**：所有 `Compatible` mod 并发下载（最大 8）→ sha512 校验 → 写到 `<instance>/mods/`。任一失败 → 整体回滚（删整个实例目录）。
5. **写 config overlay**：按 §2.2 规则，所有文件先下到临时目录校 sha256 → 全部通过后原子移动到实例目录。
6. **落实例 metadata**：在 `<instance>/.miao-modpack/` 写：
   - `pack.json`（pack 原文快照）
   - `resolved.json`（§3.5）
   - `source.json`：`{ "source_id": "miao", "source_url": "...", "subscribed_at": "..." }`
7. **更新 `instance.toml`**：新增字段 `modpack_subscription`（见 §4.2），写入。

### 4.2 `Instance` 扩展字段

`crates/core/src/instance/mod.rs` 的 `Instance` struct 新增：

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub modpack_subscription: Option<ModpackSubscription>,

pub struct ModpackSubscription {
    pub source_id: String,
    pub pack_id: String,
    pub pack_version: String,
    pub mc_version: String,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}
```

`#[serde(default)]` 保证旧实例的 toml 仍能读。

### 4.3 取消 / 失败回滚

- 安装期间 GUI 必须能取消（沿用现有 `tokio::sync::CancellationToken` 模式，参考 download 模块）。
- 任一步骤失败或取消：删除已创建的实例目录（`<data-dir>/instances/<name>/`），清空临时下载目录。

### 4.4 进度上报

走现有 `messages.rs` 的 toast/进度通道。Phase 1 上报 4 个粗粒度阶段：`Resolving | InstallingLoader | DownloadingMods(done/total) | WritingConfigs`。

---

## 5. Crate / 模块布局

### 5.1 `core`

新增模块：`crates/core/src/modpack_source/`

```
modpack_source/
├── mod.rs              // pub use; ModpackSource trait
├── manifest.rs         // manifest.json / pack.json types + validation (§2)
├── registry.rs         // 多源注册表（HashMap<source_id, Source>）；Phase 1 内置米奇喵
├── cache.rs            // 缓存层（§2.3）
├── resolver.rs         // ResolutionReport + 解算逻辑（§3）
├── installer.rs        // 安装流（§4）
└── tests/              // 单元测试 + 黄金 manifest fixtures
```

**与现有模块的关系**：
- `resolver.rs` 调用 `crate::modrinth::api` 和 `crate::curseforge`
- `installer.rs` 调用 `crate::modloader`、`crate::download`、`crate::instance`
- 不向 `modmanager`、`modrinth/mrpack` 添加任何依赖（避免回环）

### 5.2 `gui`

- 新文件 `crates/gui/src/dialogs/modpack_browse.rs`：「+ 新建实例」对话框新增的「整合包」Tab 内容。
- 修改 `crates/gui/src/dialogs/new_instance.rs`：增加 Tab 切换，复用现有 mc 版本选择 UI。
- 新文件 `crates/gui/src/views/modpack_sync.rs`：实例详情的「整合包同步」Tab（Phase 1 只读；Phase 2 加按钮）。
- 修改 `crates/gui/src/views/detail.rs`：注册新 Tab。
- 状态：在 `crates/gui/src/state.rs` 加：
  ```rust
  pub modpack_sources: ModpackSourceState,
  ```
  ModpackSourceState 持有缓存的 manifest / 当前选中 pack / 当前 ResolutionReport。

### 5.3 `cli`

Phase 1 **不**给 CLI 加命令。CLI 增量是 Phase 3 的工作（[`modpack-source.md` §4.5](modpack-source.md#L248)）。

### 5.4 内置米奇喵源

写在 `crates/core/src/modpack_source/registry.rs`：

```rust
pub const BUILT_IN_SOURCES: &[BuiltInSource] = &[BuiltInSource {
    source_id: "miao",
    manifest_url: "https://raw.githubusercontent.com/mickeymiao/miao-modpacks/main/manifest.json",
    is_default: true,
}];
```

URL 是占位，正式部署前由作者确认实际仓库路径再换。**不**用环境变量或运行时配置——Phase 1 只内置一个，后续 Phase 2 加用户自添加源时再走 `<data-dir>/modpack-sources/user.toml`。

---

## 6. GUI 集成详情

### 6.1 「+ 新建实例」对话框

现状：单 Tab（自定义 MC 版本 + loader）。

Phase 1 改造：上方加 Tab 切换 `[ 自定义 | 整合包 ]`。整合包 Tab 内容：

```
┌─────────────────────────────────────────────────┐
│ 源:  [米奇喵 ▾]                  [刷新]         │
├─────────────────────────────────────────────────┤
│ 整合包卡片列表（pack.summary + support_level）  │
│   ┌─ 米奇喵 1.21 Tricky Trials 基础整合包 ─┐    │
│   │ active · fabric · 23 mods              │    │
│   │ [选择]                                 │    │
│   └────────────────────────────────────────┘    │
├─────────────────────────────────────────────────┤
│ 选中后展开：                                     │
│   MC 版本:  [1.21.5 ▾]                          │
│   兼容性预览（点"分析"触发解算）:                │
│     ✅ 18  ⚠️ Pending 5  ❌ Conflict 0          │
│     [展开 mod 列表]                              │
│   实例名: [____________]                         │
│   [取消]  [安装]                                 │
└─────────────────────────────────────────────────┘
```

### 6.2 实例详情「整合包同步」Tab

仅当 `instance.modpack_subscription.is_some()` 时显示。Phase 1 内容：

- 当前订阅信息（source_name, pack display_name, pack_version, 安装时间）
- 当前 mod 列表 + 每个 mod 的 ResolvedStatus（来自 `.miao-modpack/resolved.json`）
- "重新解算（不安装）" 按钮 → 重新跑 resolver，把新 report 写回 resolved.json，UI 刷新；不动文件系统其他部分。
- ❌ 不做 "应用更新" / "升级到新 MC 版本"（Phase 2）

### 6.3 i18n

所有新文案在 `crates/gui/locales/{en,zh,ja}.json` 加 key，命名空间 `modpack_source.*`。

---

## 7. 测试策略

### 7.1 `core::modpack_source` 单元测试

- `manifest.rs`：黄金 fixtures（valid / schema_version 太大 / pack_url host 不在白名单 / source_id 非法），全部走过解析
- `cache.rs`：模拟 `now()`，验证 6 小时窗口
- `resolver.rs`：mock Modrinth/CF 客户端（已有 trait 抽象？若无，引入 `ModrinthClient` trait），覆盖：
  - 单 mod auto，命中
  - 单 mod auto，无版本 → Pending
  - lock 命中
  - lock 不支持 → Conflict
  - 依赖补全 1 层
  - 循环依赖 → cycle 报错
  - 回溯 1 次成功

### 7.2 `installer.rs` 集成测试

- 用临时目录 + httpmock 起本地 HTTP 模拟 Modrinth/CF 下载
- 跑完整安装流，断言：
  - 实例目录结构齐全
  - `instance.toml` 有 `modpack_subscription`
  - `.miao-modpack/{pack.json, resolved.json, source.json}` 都存在
  - 失败回滚：故意让其中一个 mod 下载 404，断言整个实例目录被删

### 7.3 GUI

不写自动化 GUI 测试。手动 QA checklist 见 §9。

### 7.4 CI

`cargo test --workspace` 必须 100% 通过；现有 314 个测试不能回归。

---

## 8. 工时拆解（Phase 1 = 10-11 天）

| 任务 | 工时 |
|---|---|
| §2 Manifest schema + 加载 + 缓存 + 单测 | 1.5 天 |
| §3 Resolver（Modrinth + CF + 依赖补全 + 环检测）+ 单测 | 3 天 |
| §4 Installer（loader + 批量 mod + overlay + 回滚）+ 集成测试 | 2.5 天 |
| §6.1 「+ 新建实例」整合包 Tab | 1.5 天 |
| §6.2 「整合包同步」Tab（只读） | 0.5 天 |
| §6.3 i18n + Toast/进度上报接通 | 0.5 天 |
| §9 手动 QA + bug fix | 1 天 |
| 余量 | 0.5-1 天 |
| **合计** | **10.5-11 天** |

---

## 9. 完成判定（Phase 1 Definition of Done）

发版前必须全部通过：

- [ ] `cargo check --workspace` 0 警告（启用 `-D warnings` 等价）
- [ ] `cargo clippy --workspace -- -D warnings` 通过
- [ ] `cargo test --workspace` 100% 通过，新增 ≥ 30 个 modpack_source 测试
- [ ] 手动 QA：
  - [ ] 从 0 装一个米奇喵整合包（选 1.21.5），全部 mod Compatible 时安装成功 → 启动进游戏
  - [ ] 故意选一个有 Pending mod 的 MC 版本，UI 正确分级 → 玩家点继续仍能装上 Compatible 的部分
  - [ ] 断网情况下打开 GUI，已缓存的 manifest 可读，新解算操作显示「网络不可用，已用缓存」
  - [ ] 安装中途取消，实例目录清理干净
  - [ ] 安装失败（人工 hosts 把某个下载 URL 指到 127.0.0.1）→ 实例目录回滚，UI 显示原因
  - [ ] 实例详情「整合包同步」Tab 显示已安装 pack 的 mod 列表和状态
  - [ ] 旧实例（没有 `modpack_subscription`）的 toml 仍能读，详情 Tab 不显示「整合包同步」
- [ ] 三种语言（en/zh/ja）的新文案都有
- [ ] CHANGELOG / Release notes 草稿（描述 Phase 1 范围 + 已知 Phase 2 缺口）

---

## 10. 与 Phase 2 / Phase 3 的接口承诺

为了让 Phase 2/3 不需要返工 Phase 1：

- **resolved.json 落盘** → Phase 2 升级流程直接读
- **`Instance.modpack_subscription`** → Phase 2 知道哪个实例订阅了哪个 pack
- **`pack.json` 快照** → Phase 2 对比新旧 pack version 时有 baseline
- **multi-source registry 抽象** → Phase 2 加用户源不需要改 resolver/installer
- **`config_overlay.files[].sha256`** → Phase 2 配置更新时能精确判断「文件是否被玩家改动」（用安装时记录的原 sha256 对比磁盘当前值）

Phase 1 不预先实现 Phase 2/3 的功能，但**以上接口字段必须在 Phase 1 落地**。
