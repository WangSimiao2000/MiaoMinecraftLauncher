# 整合包源生态 — Phase 1 落地 Spec

> 状态：**实施 spec（rev2，已完成评审）** — 本文档把 [`modpack-source.md`](modpack-source.md) §4.1 / §4.2 / §4.3 收敛为 Phase 1 可执行设计。
> 范围：仅 Phase 1（C 模型基础：schema + 解算 + UI + 安装）。Phase 2（跨版本升级）与 Phase 3（CI 验证）不在本文档。
> 目标：完成本 spec 列出的所有内容后，玩家能从 GUI 选「米奇喵源」中的某个整合包，选 MC 版本，看到兼容性预览，一键安装；安装结果是一个可启动的 MMCL 实例。

## 修订日志

### rev2 — 评审后修订（基于 Sisyphus 自检 + librarian 事实核查）

合并 21 项评审发现，修订要点：
- **§2 schema**：加 `allowed_hosts`（A1.1）/ `original_size`（A1.3）/ `preserve`（librarian Q7） / `pack_format` 字符串版本（librarian Q7）/ Phase 1 必须解析 `deprecated_after` 并跳过过期 mod（A1.2）
- **§3 resolver**：回溯深度 3→5（A2.1 + librarian Q5）/ `incompatible` 改硬阻塞（A2.2）/ NeoForge `game_versions` bug 必须二次校验（librarian Q4，HIGH）/ 新增 §3.6 限流策略（A6.2 + librarian Q2/Q3）/ `cycle` 节点结构化（A2.5）
- **§4 installer**：staging 改用 `<instance>/.staging/`（A3.1 + librarian Q6，HIGH）/ 实例名冲突预检（A3.2）/ 取消机制改为"完成才能用"（A3.3，降低 Phase 1 工时）/ overlay `preserve` 字段语义实施（librarian Q7）
- **§5 layout**：内部 trait `ResolverDataSource` 用于测试（A5.1）/ 拆 3 PR（A5.3）/ CF 客户端预先加 429 处理（librarian Q3，前置依赖）
- **§7 测试**：抛弃"30 个测试"数字目标，列举 21 个 critical-path 用例（A6.1）
- **§9 DoD**：补 4 项异常场景 QA（A6.3）
- **§10 contract**：`pack_content_hash` 双层防御（A4.1）/ `user.toml` 加载 Phase 1 落地（A4.2）

完整评审表见 §11（评审结论汇总）。

### rev1 — 初稿
基于 [`modpack-source.md`](modpack-source.md) §4 收敛 Phase 1 设计。

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
  "pack_format": "miao:1.0.0",            // semver 风格，主版本不兼容则拒绝；Phase 1 主版本 = 1
  "source_id": "miao",                    // [a-z0-9-]{2,32}, 源唯一 ID
  "source_name": "米奇喵整合包源",
  "author": "mickeymiao",
  "homepage": "https://space.bilibili.com/36913332",
  "updated_at": "2026-05-29T10:00:00Z",   // RFC3339
  "allowed_hosts": [                      // 可选；本源声明允许的 pack/file/overlay URL host
    "raw.githubusercontent.com",
    "cdn.jsdelivr.net"
  ],
  "packs": [
    {
      "id": "miao-1.21-base",             // [a-z0-9.-]{2,64}, 源内唯一（允许 . 因为 MC 版本号常含 .）
      "display_name": "米奇喵 1.21 Tricky Trials 基础整合包",
      "summary": "短描述（GUI 卡片用），≤120 字符",
      "mc_versions": ["1.21.1", "1.21.4", "1.21.5"],
      "loader": "fabric",                 // "fabric"|"forge"|"neoforge"|"quilt"
      "support_level": "active",          // "active"|"maintenance"|"archived"
      "pack_url": "https://raw.githubusercontent.com/WangSimiao2000/miao-modpacks/main/packs/miao-1.21-base/pack.json"
    }
  ]
}
```

**校验规则**：
- `pack_format` 必须匹配 `^[a-z0-9-]+:\d+\.\d+\.\d+$`。MMCL 解析其前缀（命名空间）和主版本号；主版本号 > 当前支持上限 → 拒绝加载，UI 显示「源版本不兼容，请升级 MMCL」。次/补丁版本号忽略，保留前向兼容。Phase 1 仅识别 `miao:1.x.x`；其他命名空间（如 `packwiz:`）拒绝（Phase 1 不做转换）。
- `source_id` 必须匹配正则 `^[a-z0-9-]{2,32}$`
- `pack_url` 必须 https。host 校验顺序：(1) 本 manifest 的 origin host（永远允许）；(2) `allowed_hosts[]` 列出的；(3) Phase 1 内置 fallback 白名单 `raw.githubusercontent.com` / `cdn.jsdelivr.net`（仅当 manifest 未提供 `allowed_hosts` 字段时启用）。三层都不命中 → 拒绝。
  - 此设计让源作者**显式声明**自己的 CDN/镜像 host，无需 MMCL 升级；Phase 2 用户自添加源时，作者写什么 MMCL 信什么（再加"未验证"警示）。
- `mc_versions` 至少 1 个，每项必须匹配 `^\d+\.\d+(\.\d+)?$`
- **未知字段**：所有 schema 解析对未知字段一律静默丢弃（serde 默认行为），不报错。Phase 2 新增字段必须保持 Phase 1 解析时缺省安全。

### 2.2 `pack.json`（整合包根）

```jsonc
{
  "pack_format": "miao:1.0.0",            // 同 manifest.json 的语义
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
      "criticality": "core",              // "core"|"optional"，Phase 1 仅展示，不参与升级判定（升级判定在 Phase 2）
      "deprecated_after": null,           // 可选；MC 版本字符串。Phase 1 行为：选定 mc_version >= deprecated_after 时跳过此 mod，不安装，UI 标 Status::Deprecated
      "replacement": null                 // 可选；替代 mod 的引用 { source, id }。Phase 1 仅记录到 resolved.json，不自动安装替代品（那是 Phase 2 行为）
    }
  ],
  "config_overlay": {
    "base_url": "https://raw.githubusercontent.com/WangSimiao2000/miao-modpacks/main/packs/miao-1.21-base/config",
    "files": [
      {
        "path": "_common/options.txt",    // 相对 base_url 的路径
        "target": "options.txt",          // 写入实例时的相对路径（相对 instance 根）
        "sha256": "abc123...",            // 必填，下载完校验
        "original_size": 2048,            // 可选；作者声明的字节数。Phase 1 落实例时记录到 resolved.json，给 Phase 2 配置变更检测用
        "preserve": false                 // 可选；true = 即使源更新也不覆盖玩家本地修改；Phase 1 写入逻辑相同（first-install-only），但记录到 resolved.json，Phase 2 推送配置更新时遵循
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

**Phase 1 字段处理**：
- `deprecated_after`：Phase 1 解析并执行——选定 `mc_version` 经字符串比较 ≥ `deprecated_after` 时（按 `^\d+\.\d+(\.\d+)?$` 解析后做版本号字典序比较），把该 mod 标 `Status::Deprecated`，**不计入安装清单**，UI 显示「该 mod 已声明在此 MC 版本弃用，已跳过」。
- `replacement`：Phase 1 仅记录到 `resolved.json` 的 `replacement_hint` 字段，**不自动安装**。Phase 2 升级流程消费此字段。
- `criticality`：Phase 1 仅在 UI 展示（红/黄边框区分 core/optional）；不参与"是否阻断安装"判定（Phase 2 升级判定才用）。
- `preserve`：Phase 1 安装行为与缺省相同（写入），但**必须把字段值落到 `resolved.json`**，Phase 2 推送配置更新时遵循（preserve=true → 不覆盖；preserve=false → 询问玩家）。

**配置 overlay 解析**（Phase 1 = first-install-only，[`modpack-source.md` §5.1](modpack-source.md#L277)）：
- `_common/` 下的文件无条件写入
- 其它路径下的文件，若 `applies_to_mc` 包含玩家选定版本 → 写入；否则跳过
- 同一 `target` 出现多次按 `files[]` 顺序后写覆盖前
- 写入前必须 sha256 校验，失败 → 整体安装失败回滚（已写入文件清单见 §4.4）

**Mod 文件 hash 兼容性**：
- Modrinth API 返回 sha1 + sha512；CurseForge API 返回 md5 + sha1（**无 sha512**）。
- `ResolvedMod`（§3.5）的 hash 字段统一改为 `file_sha1: Option<String>` + `file_sha512: Option<String>`，至少一个非空。
- 安装时优先校验 sha512（如有）否则 sha1。

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
     - 实测 Modrinth `/v2/projects?ids=[]` 单次可放 ~700 IDs，30 mod 完全单次完成
     - 若 mod 数 > 500（远超 Phase 1 实际场景），按 100 切片防止越过 Cloudflare URL 长度限制
   - CurseForge：**优先**用 `POST /v1/mods` 批量端点（CF docs `GetModsByIdsListRequestBody`）拉所有 project 详情；按 mod 各一次 `/v1/mods/{id}/files` 过滤——不要逐个 GET（现有 `crates/core/src/curseforge/api.rs:233` 是逐个 GET，本 spec 要求 resolver 用批量端点；现有 GET 路径保留供其它已有功能使用）
   - 并发限制：**Modrinth 16 并发，CurseForge 8 并发**（CF 限制更严，无文档但实测易触发 429）
   - 单 pack 解算 SLA 5 秒（[`modpack-source.md` §4.3](modpack-source.md#L189)）；超 5 秒不超 8 秒 UI 显示 spinner；超 8 秒视为失败。**结果渐进展示**：每个 mod 解算完即推送状态给 UI，不等全部完成
2. **每个 mod 选版本**：
   - `policy=lock` → 用 `locked_version`，若该版本不支持 mc/loader → 标 `Conflict`
   - `policy=auto` → 候选版本 = 该 project 在 mc + loader 下发布过的所有 version；按发布日期降序取第一个 stable。无候选 → 状态分类按 §3.3
   - **NeoForge `game_versions` 字段二次校验（HIGH）**：Modrinth 在 NeoForge 26.x 和 1.21.x 上 `game_versions` 已知会返回 loader 版本而不是 MC 版本（参见 `modrinth/code#6068`、`#4530`、`#5056`）。Phase 1 必须用以下方法二次校验：
     - 候选 version 的 `dependencies[]` 中找类型 = `required` 且 `project_id == "P7dR8mSH"`（Fabric Loader）或对应 NeoForge 项目 id 的依赖；读其 `version_id` → 拉对应 loader 版本 → 取该 loader 的 `game_versions[]` 作为权威 MC 版本来源
     - 若 mod 的 `dependencies[]` 没声明 loader 依赖（罕见），fallback 用 mod 自身的 `game_versions[]` 但**额外**检查 `loaders` 是否包含 `neoforge`，若是则降级标记为 `Status::Ambiguous`
     - 这个二次校验仅对 `loader == NeoForge` 触发，Fabric/Quilt/Forge 不受影响
3. **依赖补全**（贪心 + 回溯）：
   - 对每个已选 version 的 `dependencies[]`，类型 = `required` 且不在当前结果集 → 把它当作新 mod 加入待解算队列
   - 类型 = `incompatible` → 检查冲突方是否已在结果集；是 → **硬阻塞**，整体解算失败，标 `Conflict`，UI 显示「mod A 与 mod B 不兼容，无法同时安装。请联系整合包作者修正声明」+ 一个隐藏在 details 折叠里的"忽略并继续（不推荐）"按钮（需要二次确认弹窗）。理由：incompatible 通常表示运行时崩溃或数据损坏，让玩家默认无门槛"继续"会得到进不去游戏的实例
   - 回溯触发：dep 自身解算失败时，回到引入它的 mod 尝试次新版本；**最多回溯深度 = 5**（基于 librarian 调研：实测最坏依赖链如 `mod → REI → Architectury → Cloth Config → Fabric API` 4 层 + 1 buffer），超过 → 标记为 `Conflict`
   - 总尝试次数预算：mod 总数 × 3，超过 → 整体解算失败
   - 阈值常量集中：`pub const BACKTRACK_MAX_DEPTH: usize = 5; pub const BACKTRACK_MAX_ATTEMPTS_PER_MOD: usize = 3;` 在 `resolver.rs` 顶部定义
4. **环检测**：用 DFS 灰/黑标记，遇到灰节点 → 输出环路径（`Vec<CycleNode>`，节点结构 `{ project_id, display_name }`，方便 UI 展示），整体解算失败（不自动断环）。注：librarian 调研显示主流 Fabric mod 中**没有发现真实的循环 required 依赖**，此分支主要防御作者写错

### 3.3 Mod 状态分类

完全对齐 [`modpack-source.md` §4.3](modpack-source.md#L172)：

| 状态 | Phase 1 判定 |
|---|---|
| `Compatible` | Modrinth/CF 在 (mc, loader) 下有 stable 版本 |
| `Pending` | 当前 (mc, loader) 无版本，但该 project 在过去 6 个月有发布过任何版本 |
| `Abandoned` | 该 project 任何版本最近发布 > 6 个月前 |
| `Conflict` | 有候选版本但依赖冲突 / 回溯耗尽 |
| `Deprecated` | pack.json 中 `deprecated_after` 字段命中当前 mc_version |
| `Ambiguous` | NeoForge `game_versions` 字段二次校验失败（见 §3.2 step 2） |

阈值常量：`pub const ABANDONED_THRESHOLD_DAYS: u32 = 180;` 在 `resolver.rs` 顶部单一定义。Phase 2 可根据真实数据调整。

### 3.4 循环依赖处理

环路径展示形如 `A → B → C → A`（节点结构 `CycleNode { project_id, display_name }`），UI 显示后玩家只能放弃此次解算（Phase 1）。Phase 2 再考虑「弃坑替代」是否能断环。基于 librarian 调研，主流 Fabric mod 中暂无真实循环 required 依赖，本分支主要防御作者声明错误。

### 3.5 `ResolutionReport` 数据结构

```rust
// crates/core/src/modpack_source/resolver.rs
pub struct ResolutionReport {
    pub pack_id: String,
    pub pack_version: String,
    pub pack_content_hash: String,         // sha256 of pack.json 原文；§10 Phase 2 检测包内容变化用
    pub mc_version: String,
    pub loader: ModLoaderType,
    pub loader_version: String,            // 解算出的 loader 版本
    pub resolved_at: chrono::DateTime<chrono::Utc>,
    pub mods: Vec<ResolvedMod>,
    pub overlays: Vec<ResolvedOverlay>,    // 落实例的 config overlay 元信息（含 preserve、original_size）
    pub conflicts: Vec<ConflictReport>,    // 来自 incompatible 声明（硬阻塞）
    pub cycle: Option<Vec<CycleNode>>,     // 若检测到环，整体解算视为失败
}

pub struct CycleNode {
    pub project_id: String,
    pub display_name: String,
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
    pub file_sha1: Option<String>,         // CF 提供
    pub file_sha512: Option<String>,       // Modrinth 提供；至少 sha1/sha512 之一非空
    pub file_size: Option<u64>,
    pub file_name: Option<String>,
    pub auto_added_for: Option<String>,    // 若是依赖补全，填上游 mod project_id
    pub replacement_hint: Option<ReplacementHint>,  // pack.json 的 replacement 字段（Phase 2 消费）
}

pub struct ReplacementHint {
    pub source: ModSource,
    pub id: String,
}

pub struct ResolvedOverlay {
    pub target: String,                    // 实例内相对路径
    pub source_url: String,                // 下载来源
    pub sha256: String,                    // 写入时校验过的
    pub original_size: u64,                // 写入时记录；Phase 2 检测变更用
    pub preserve: bool,                    // 来自 pack.json overlay file 字段
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

pub enum ResolvedStatus {
    Compatible,
    Pending { last_release: chrono::DateTime<chrono::Utc> },
    Abandoned { last_release: Option<chrono::DateTime<chrono::Utc>> },
    Conflict { reason: String },
    Deprecated { since: String },          // mc_version 字符串
    Ambiguous { reason: String },          // NeoForge game_versions 二次校验失败
}
```

实例落盘位置：`<instance_dir>/.miao-modpack/resolved.json`，给 Phase 2 升级流程读。

### 3.6 速率限制与重试策略

基于 librarian 调研：

- **Modrinth**：300 req/min/IP；429 时读 `X-Ratelimit-Reset`（秒数）；headers 还有 `X-Ratelimit-Limit` / `X-Ratelimit-Remaining`。**无 `Retry-After`**。
- **CurseForge**：无文档 rate limit；429 时读 `Retry-After`（秒数或 HTTP date）。
- **现有 CF 客户端零 429 处理**（[`crates/core/src/curseforge/api.rs:33`](file:///home/wangsimiao/Projects/LinuxMinecraftLauncher/crates/core/src/curseforge/api.rs#L33) 直接 `error_for_status()`），**这是 Phase 1 的前置依赖**，必须在 resolver 之前修。

**实现要求**（写在 `crates/core/src/http.rs` 或新建 `crates/core/src/modpack_source/http.rs`，给 resolver 用）：

```rust
pub struct RateLimitedClient {
    inner: reqwest::Client,
    // 每 host 一个 in-memory token bucket / 限流状态
    rate_state: Arc<Mutex<HashMap<String, RateState>>>,
}

// 关键行为：
// 1. 请求前 check rate_state，若已知 reset_at > now → 等待
// 2. 收到 200 + 限流 headers → 更新 rate_state.remaining/reset
// 3. 收到 429 → 读 X-Ratelimit-Reset（Modrinth）或 Retry-After（CF），sleep 后重试
// 4. 收到 5xx → 指数退避（500ms / 2s / 8s），最多重试 3 次
// 5. 上报"等待中"状态给 progress 通道，UI 可以显示「API 限流，X 秒后重试」
```

**主动节流**：
- Modrinth：`X-Ratelimit-Remaining < 30` 时开始限速到 1 req/sec；= 0 时强制等到 reset
- CurseForge：实测 8 并发 + 自然请求间隔（每个 mod 1 次 file API）已较保守；如 429 触发，全局降级到 4 并发 30 秒

---

## 4. 安装流（Installer）

### 4.1 顺序（必须严格）

0. **实例名冲突预检（HIGH）**：在调用 `Instance::create_directories` 之前，检查目标 `<data-dir>/instances/<name>/` **不存在**（即使为空）。已存在 → 立即拒绝安装并 UI 报错「该实例名已存在，请改名」。**不允许覆盖**。理由：[`crates/core/src/instance/mod.rs`](file:///home/wangsimiao/Projects/LinuxMinecraftLauncher/crates/core/src/instance/mod.rs) 现有 `create_directories` 用 `fs::create_dir_all`，对已有目录无报错，盲目走完安装失败回滚会**删除用户已有数据**（如 200 小时存档）。
1. **解算预检**：`ResolutionReport.cycle == None` 且没有 `core` mod 是 `Conflict`（硬阻塞）/ `Abandoned`（`Pending` / `Ambiguous` 允许，玩家已在 GUI 确认；`Deprecated` 不计入安装清单，不阻塞）。
2. **创建实例**：调用 `core::instance` 现有 API，名称由 GUI 传入；目录 `<data-dir>/instances/<name>/`。
3. **装 loader**：调 `core::modloader`（已存在）按 `loader + loader_version + mc_version`。
4. **创建 staging 目录**：`<instance_dir>/.staging/`（同实例目录同挂载点，规避 EXDEV，见 §4.5）。
5. **批量装 mod**：所有 `Compatible` mod 并发下载到 `<instance_dir>/.staging/mods/`（Modrinth 8 并发 / CF 4 并发，比解算阶段更保守因为下载流量大且耗时长）→ 优先 sha512、否则 sha1 校验 → 全部通过后批量 `rename` 到 `<instance_dir>/mods/`。任一失败 → 整体回滚（见 §4.3）。
6. **写 config overlay**：按 §2.2 规则，所有文件先下到 `<instance_dir>/.staging/overlay/` 校 sha256 → 全部通过后逐个 `rename` 到 instance 目标路径（保证同挂载点原子）。
7. **落实例 metadata**：在 `<instance>/.miao-modpack/` 写：
   - `pack.json`（pack 原文快照）
   - `resolved.json`（§3.5，含 `overlays[]` 的 preserve / original_size）
   - `source.json`：`{ "source_id": "miao", "source_url": "...", "manifest_etag": "...", "subscribed_at": "..." }`
8. **更新 `instance.toml`**：新增字段 `modpack_subscription`（见 §4.2），写入。
9. **清理 staging**：成功完成后删除 `<instance_dir>/.staging/`。

### 4.2 `Instance` 扩展字段

`crates/core/src/instance/mod.rs` 的 `Instance` struct 新增：

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub modpack_subscription: Option<ModpackSubscription>,

pub struct ModpackSubscription {
    pub source_id: String,
    pub pack_id: String,
    pub pack_version: String,
    pub pack_content_hash: String,         // sha256 of pack.json 原文；§10 Phase 2 检测包内容变化的双层防御
    pub mc_version: String,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}
```

`#[serde(default)]` 保证旧实例的 toml 仍能读.

### 4.3 取消 / 失败回滚

**Phase 1 决策：放弃"中途取消"**。理由：
- 核查后确认 `crates/core/src/` 中**不存在** `tokio_util::sync::CancellationToken` 使用；现有 download manager 也没结构化取消接口。
- 引入结构化取消是 0.5-1 天工作（要改 `DownloadManager` 公共 API、所有调用方），不在 Phase 1 工时预算里。
- Phase 1 安装总时长预期 < 3 分钟（米奇喵基础整合包 ~30 mod，按平均 50KB/s 已留余量），等待成本可接受。
- GUI 安装期间显示进度 + "请等待，无法取消"，安装完才允许其他操作。

**Phase 1 仍要实现的"非取消式回滚"**：
- 任一步骤异常返回（网络失败、校验失败、磁盘错误）→ catch error → 删除实例目录 + staging 目录 → 上报错误给 UI。
- 删除前先确认目录确为本次安装创建（即步骤 0 的预检确认了"不存在"，所以本次安装期间出现的目录必为本次创建；用户先前已有的目录绝不会到这里）。

**Phase 2 改进**：引入 `tokio_util::sync::CancellationToken`、改 DownloadManager API 支持中断；Phase 1 不做。

### 4.4 进度上报

走现有 `messages.rs` 的 toast/进度通道。Phase 1 上报 6 个粗粒度阶段：`Resolving | RateLimited(reset_at) | InstallingLoader | DownloadingMods(done/total) | WritingOverlay(done/total) | Finalizing`。`RateLimited` 阶段在 §3.6 限流触发时进入，UI 显示倒计时。

### 4.5 阶性移动（Staging）策略

基于 librarian Q6 调研：

- `std::fs::rename` 在 Linux 跨挂载点会返回 `EXDEV`（例如 `/tmp/` 是 tmpfs 而 `~/.local/share/` 是 ext4 的常见情况，Fedora/Arch 默认配置）。
- `std::env::temp_dir()` 不能用于 staging，因为大概率跨挂载点。

**实现要求**：
- staging 路径**必须是 `<instance_dir>/.staging/`**，由 `tempfile::tempdir_in(&instance_dir)` 创建，保证与目标同挂载点。
- 同挂载点 `rename` 是 inode-level 原子操作，多文件批量 `rename` 不是事务但每个文件单独原子；中途失败时部分文件已落地，回滚靠步骤 6 的整体目录删除。
- **EXDEV fallback**：万一用户把 `<instance_dir>/.staging/` 单独挂载（极罕见），`rename` 仍可能 EXDEV → fallback 到 `fs::copy` + `fs::remove_file`，丧失原子性但完成操作；此时日志 warning。
- staging 目录**必须**在 `core::instance::create_directories` 之后创建，且必须在 §4.1 步骤 9 显式删除。崩溃残留的 staging 在下次 MMCL 启动时由实例加载逻辑清理（实例侧扫描 `.staging/` 存在 → 一并清空）。

---

## 5. Crate / 模块布局

### 5.1 `core`

新增模块：`crates/core/src/modpack_source/`

```
modpack_source/
├── mod.rs              // pub use; ModpackSource trait
├── manifest.rs         // manifest.json / pack.json types + validation (§2)
├── registry.rs         // 多源注册表（HashMap<source_id, Source>）；Phase 1 内置米奇喵 + 加载 user.toml
├── cache.rs            // 缓存层（§2.3）
├── http.rs             // RateLimitedClient（§3.6）；含 429 重试逻辑，给 resolver 用
├── resolver.rs         // ResolutionReport + 解算逻辑（§3）；定义内部 trait ResolverDataSource
├── installer.rs        // 安装流（§4）；含 staging 与 EXDEV fallback
└── tests/              // 单元测试 + 黄金 manifest fixtures
```

**与现有模块的关系**：
- `resolver.rs` 通过内部 trait `ResolverDataSource` 调用 Modrinth/CF 客户端，**不直接依赖** `crate::modrinth::api` 或 `crate::curseforge` 的具体类型，便于测试 mock。生产实现 `LiveResolverDataSource` 包装两个真实客户端。
- `installer.rs` 调用 `crate::modloader`、`crate::download`、`crate::instance`
- 不向 `modmanager`、`modrinth/mrpack` 添加任何依赖（避免回环）
- **不复用 `mrpack.rs`**：mrpack 是静态文件清单+zip 解压，本模块是动态解算+远程清单，输入输出差异 > 复用收益。spec 明示此判断防止后续过度抽象。

**Phase 1 前置依赖（必须在 resolver 落地前完成）**：
- [`crates/core/src/curseforge/api.rs`](file:///home/wangsimiao/Projects/LinuxMinecraftLauncher/crates/core/src/curseforge/api.rs) 现有 `get_json` / `post_json` 直接 `error_for_status()`，零 429 处理。Phase 1 必须先重构这两个方法走 `RateLimitedClient`（或在 modpack_source 内独立的 http 层），让 resolver 安全使用。该重构同时让现有 CF 功能（mod 搜索、mod 安装）受益。
- 不重构现有 `modrinth/api.rs` 公共 API；新加 `RateLimitedClient` 让 resolver 调用，旧路径不受影响。

**Registry 加载（A4.2）**：
```rust
// registry.rs
pub fn load_sources() -> Result<HashMap<String, ModpackSource>> {
    let mut sources = HashMap::new();
    // 1. 内置源
    for built_in in BUILT_IN_SOURCES { sources.insert(...); }
    // 2. 用户源（Phase 1 不暴露 UI，但加载逻辑就绪）
    let user_toml_path = data_dir().join("modpack-sources").join("user.toml");
    if user_toml_path.exists() {
        let user_sources: UserSources = toml::from_str(&fs::read_to_string(&user_toml_path)?)?;
        for src in user_sources.sources { sources.insert(...); }
    }
    Ok(sources)
}
```
Phase 2 加 GUI 编辑入口时，`load_sources()` 不需修改。

**PR 切分建议（A5.3）**：
- **PR 1**：`manifest.rs` + `cache.rs` + `http.rs`（含 RateLimitedClient）+ CF 客户端 429 重构 + 单测。审查面：schema 决策 + HTTP 层。
- **PR 2**：`resolver.rs` + `registry.rs` + 内部 trait + 单测。审查面：解算算法 + NeoForge 二次校验。
- **PR 3**：`installer.rs` + GUI Tabs + i18n + 集成测试 + 实例字段扩展。审查面：端到端流程 + 用户数据安全（实例名冲突、staging）。

每个 PR 独立可合可回滚；PR 3 才接通 GUI 入口，所以 PR 1/2 单独合并不影响用户。

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
    manifest_url: "https://raw.githubusercontent.com/WangSimiao2000/miao-modpacks/main/manifest.json",
    is_default: true,
}];
```

仓库 [`WangSimiao2000/miao-modpacks`](https://github.com/WangSimiao2000/miao-modpacks) 已建立，初版 fixture 包 `miao-1.21-base-test`（Sodium + Fabric API + REI，1.21.4 / 1.21.5）已推送 main，作为 Phase 1 实施期间的 end-to-end 验证素材。**不**用环境变量或运行时配置——Phase 1 只内置一个，后续 Phase 2 加用户自添加源时再走 `<data-dir>/modpack-sources/user.toml`。

进入 Phase 2 之前，作者需在 `miao-modpacks` 仓库新增正式整合包（如 `miao-1.21-base`「米奇喵 1.21 基础美化包」），到时把 manifest.json 的 packs 列表加项即可，**MMCL 端无需改动**。

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

### 7.1 `core::modpack_source` 单元测试（critical-path 用例必须实现）

抛弃"≥30 个测试"的数字目标，改为列举 critical-path 用例。**以下每一项必须有对应测试函数**，缺一不可：

**Schema 测试** (`manifest.rs` / `pack.rs`)：
- `t_schema_01_manifest_valid_loads`
- `t_schema_02_pack_format_major_too_high_rejected`
- `t_schema_03_pack_format_minor_higher_accepted`（前向兼容）
- `t_schema_04_pack_format_unknown_namespace_rejected`（如 `packwiz:1.0.0`）
- `t_schema_05_source_id_invalid_chars_rejected`
- `t_schema_06_pack_url_not_in_allowed_hosts_rejected`
- `t_schema_07_pack_url_in_allowed_hosts_accepted`
- `t_schema_08_pack_url_same_origin_always_allowed`
- `t_schema_09_unknown_field_silently_dropped`
- `t_schema_10_pack_with_deprecated_after_parses`
- `t_schema_11_overlay_with_preserve_parses`

**Cache 测试** (`cache.rs`)：
- `t_cache_01_fresh_cache_skips_network`
- `t_cache_02_stale_cache_refetches`
- `t_cache_03_offline_falls_back_to_any_age`
- `t_cache_04_etag_304_keeps_cached_body`

**Resolver 测试** (`resolver.rs`，mock `ResolverDataSource`)：
- `t_resolve_01_single_mod_auto_compatible`
- `t_resolve_02_single_mod_auto_no_version_pending`
- `t_resolve_03_single_mod_auto_abandoned`（last release > 180 days）
- `t_resolve_04_lock_version_compatible`
- `t_resolve_05_lock_version_unsupported_conflict`
- `t_resolve_06_dep_required_auto_added`
- `t_resolve_07_dep_required_chain_5_levels`（验证回溯深度=5 够用）
- `t_resolve_08_dep_incompatible_blocks_install`（硬阻塞）
- `t_resolve_09_dep_cycle_detected_returns_cycle_nodes`
- `t_resolve_10_mc_version_not_in_pack_versions_pre_rejected`
- `t_resolve_11_deprecated_after_skips_mod`
- `t_resolve_12_replacement_hint_recorded_not_installed`
- `t_resolve_13_neoforge_game_versions_ambiguous_detected`（mock 返回 `["26.1"]` 这种 NeoForge bug 模式）
- `t_resolve_14_pack_content_hash_computed_from_raw_json`

**HTTP / 限流测试** (`http.rs`)：
- `t_http_01_modrinth_429_with_xratelimit_reset_retried`
- `t_http_02_modrinth_429_exhausted_returns_error`
- `t_http_03_curseforge_429_with_retry_after_retried`
- `t_http_04_5xx_exponential_backoff_3_retries`
- `t_http_05_xratelimit_remaining_below_30_throttles`

### 7.2 `installer.rs` 集成测试（critical-path 用例必须实现）

用临时目录 + `httpmock` 起本地 HTTP 模拟 Modrinth/CF 下载：

- `t_install_01_happy_path_creates_instance_with_mods_and_overlay`
- `t_install_02_existing_instance_name_rejects_pre_check`（A3.2，**必须验证 user 数据未被删除**）
- `t_install_03_mod_download_404_rolls_back_full_instance`
- `t_install_04_mod_sha512_mismatch_rolls_back`
- `t_install_05_mod_sha1_mismatch_for_curseforge_rolls_back`（CF 没 sha512）
- `t_install_06_overlay_sha256_mismatch_rolls_back`
- `t_install_07_concurrent_download_limit_respected`（Modrinth 8 / CF 4）
- `t_install_08_staging_inside_instance_dir_not_in_tmp`（A3.1）
- `t_install_09_exdev_fallback_to_copy`（用 mock 文件系统模拟 EXDEV）
- `t_install_10_pack_content_hash_persisted_to_modpack_subscription`
- `t_install_11_overlays_with_preserve_persisted_to_resolved_json`
- `t_install_12_resumable_staging_cleanup_on_relaunch`（startup 时清理残留 .staging/）
- `t_install_13_disk_full_during_mod_download_rolls_back`（mock 写失败）

### 7.3 GUI

不写自动化 GUI 测试。手动 QA checklist 见 §9。

### 7.4 CI

`cargo test --workspace` 必须 100% 通过；现有 314 个测试不能回归。新增测试预期 30+ 个（不再作为目标，仅作为参照）。

---

## 8. 工时拆解（Phase 1 = 10-11 天）

| 任务 | 工时 |
|---|---|
| 前置：CF 客户端 429 重构 + RateLimitedClient（§5 前置依赖、§3.6） | 0.5 天 |
| §2 Manifest schema + 加载 + 缓存 + 单测 | 1.5 天 |
| §3 Resolver（Modrinth + CF + 依赖补全 + 环检测 + NeoForge 二次校验）+ 单测 | 3 天 |
| §4 Installer（loader + 批量 mod + overlay + staging/EXDEV + 回滚）+ 集成测试 | 2.5 天 |
| §6.1 「+ 新建实例」整合包 Tab | 1.5 天 |
| §6.2 「整合包同步」Tab（只读） | 0.5 天 |
| §6.3 i18n + Toast/进度上报接通（含 RateLimited 阶段） | 0.5 天 |
| §9 手动 QA + bug fix | 1 天 |
| 余量 | 0.5 天 |
| **合计** | **11.5 天** |

rev2 比 rev1 多 0.5-1 天，主要来自 CF 客户端前置重构 + NeoForge 二次校验。仍在 [`modpack-source.md` §7](modpack-source.md#L331) "10-11 天"工时大盘的合理误差范围（实际可压回 11 天，但承诺 11.5 天保留余量）。

---

## 9. 完成判定（Phase 1 Definition of Done）

发版前必须全部通过：

**自动化检查**（截至 2026-05-31，phase1-work@b4916e8）：
- [x] `cargo check --workspace` 0 警告
- [x] `cargo clippy --workspace --all-targets -- -D warnings` 通过
- [x] `cargo test --workspace` 100% 通过：core 388 / cli 12 / gui 13，无回归
- [x] §7.1 critical-path 测试用例（registry/cache/manifest/resolver）已实现并通过
- [x] §7.2 installer 集成测试 13 项（t_install_01 - t_install_13）已实现并通过

**手动 QA**（截至 2026-05-31，QA session 一覆盖以下条目；剩余 11 项是边界 case 故障注入或需要造假数据，留待 0.3.0-rc 前抽样）：
- [x] 从 0 装一个米奇喵整合包（选 1.21.5），全部 mod Compatible 时安装成功 → 启动进游戏
- [x] 故意选一个有 Pending / Abandoned mod 的 MC 版本（用 Indium 作真实 fixture），UI 正确分级 → 玩家点继续仍能装上 Compatible 的部分；通过新增的 confirm modal（`dialogs/modpack_confirm.rs`）实现 explicit consent
- [ ] 故意选一个 NeoForge 整合包，验证 §3.2 step 2 二次校验生效（米奇喵源现仅含 Fabric pack，需另造 NeoForge fixture）
- [ ] 故意选一个有 incompatible 声明的 mod 组合 → 硬阻塞（**core 测试 t_install_07 已覆盖后端语义；GUI 行为未人工验证**）
- [ ] 故意触发循环依赖（mock fixture）→ UI 展示环路径节点（**core resolver 测试已覆盖 cycle 检测；GUI 渲染未人工验证**）
- [ ] 断网情况下打开 GUI，已缓存的 manifest 可读 — **未实现：GUI 当前未接 `modpack_source::cache`，每次 fetch_manifest 直连网络（known gap）**
- [ ] 安装期间 GUI 显示进度条 + "无法取消" 提示（Phase 1 取消不实现）— **进度条已实现并验证（spinner + InstallProgress 标签）；"无法取消" 文案未加**
- [ ] 安装失败（人工 hosts 把某个下载 URL 指到 127.0.0.1）→ 实例目录回滚（**core 测试 t_install_03/04/05/06 已覆盖回滚；GUI 行为未人工验证**）
- [ ] **A3.2 关键**：同名实例预检拒绝（**core 测试 t_install_02 已覆盖；GUI 复验未人工跑**）
- [x] **A3.1 关键**：staging 在 `<instance>/.staging/`（`installer.rs:128` 静态确认 + 安装时观察一致）
- [ ] 触发 Modrinth API 限流（狂点"重新解算"）→ UI 倒计时（**HTTP client RateLimitedClient 已实现；GUI 未提供"重新解算"按钮所以无法人工触发，遗留 0.3.0-rc**）
- [ ] pack_format 主版本太大 → UI「源版本不兼容」（**core schema validation 已覆盖；GUI 未人工验证错误文案**）
- [ ] manifest 网络超时 → 兜底用本地缓存 — **同 #6，cache 层未在 GUI 接通**
- [ ] Modrinth 503 → 自动重试 3 次后失败（**RateLimitedClient 内置重试；未人工注入 503 验证 UI 错误**）
- [ ] 装到一半磁盘满（填满分区）→ 整体回滚（**核心回滚由 t_install_03-06 覆盖；具体磁盘满路径未人工验证**）
- [x] 实例详情「整合包同步」Tab 显示 mod 列表和状态（含 Deprecated）— 同步 Tab 渲染验证；含状态图标修复 emoji tofu
- [x] 旧实例（无 `modpack_subscription`）的 toml 仍能读，详情 Tab 不显示「整合包同步」— 代码层验证（`detail.rs:148` `is_some()` 检查 + `t_instance_legacy_toml_without_modpack_subscription_loads`）
- [x] `<data-dir>/modpack-sources/user.toml` 写测试源 → 重启 MMCL 后加载 — 修复 GUI 未调用 `load_sources()` 的 bug；e2e 验证用 example.com 源，下拉框出现且 manifest 拉取返回预期 404 不崩溃

**交付物**：
- [x] 三种语言（en/zh/ja）的新文案都有 — 206 keys 三语完全对齐（200 Phase 1 base + 4 confirm modal + 2 resolving overlay）
- [ ] CHANGELOG / Release notes 草稿（描述 Phase 1 范围 + 已知 Phase 2 缺口）— 待 0.3.0-beta.1 release commit 时补

---

## 10. 与 Phase 2 / Phase 3 的接口承诺

为了让 Phase 2/3 不需要返工 Phase 1：

- **`resolved.json` 落盘** → Phase 2 升级流程直接读，含 `mods[]` / `overlays[]` / `pack_content_hash`
- **`Instance.modpack_subscription`**（含 `pack_content_hash`）→ Phase 2 用 `(pack_version, pack_content_hash)` 双层防御检测包变化（防作者忘改 version 字符串但改了 mod 列表的常见错误）
- **`pack.json` 快照** → Phase 2 对比新旧 pack version 时有 baseline
- **`multi-source registry`** → `load_sources()` 在 Phase 1 已支持读 `<data-dir>/modpack-sources/user.toml`；Phase 2 仅加 GUI 编辑入口
- **`allowed_hosts` 字段（schema 层）** → Phase 2 用户自添加源时，作者声明的 host 直接生效，无需 MMCL 升级解锁新 CDN
- **`config_overlay.files[].sha256` + `original_size`** → Phase 2 配置更新时能精确判断「文件是否被玩家改动」（先比 size 再比 hash，size 不变 hash 变 = 高概率行尾/编码自动改写，宽容处理；size 也变 = 玩家明确改过）
- **`config_overlay.files[].preserve`** → Phase 2 推送配置更新时遵循
- **`deprecated_after` / `replacement` 解析** → Phase 1 已识别并按"跳过 / 记录 hint"处理；Phase 2 升级流程直接消费
- **`ResolvedStatus::Deprecated` / `Ambiguous`** → Phase 2 UI 渲染逻辑可直接复用
- **`RateLimitedClient`（§3.6）** → Phase 2 / Phase 3 任何外部 HTTP 都走它，统一限流

Phase 1 不预先实现 Phase 2/3 的功能，但**以上接口字段必须在 Phase 1 落地**。

---

## 11. 评审结论汇总（rev2）

下表是 rev1 → rev2 的所有修订，按严重度排列：

| ID | Axis | 等级 | 修订内容 | 落实节 |
|---|---|---|---|---|
| **A1.1** | Schema | 🔴→已修 | 加 `allowed_hosts` 字段，三层 host 校验顺序 | §2.1 |
| **A2.1** | Resolver | 🔴→已修 | 回溯深度 3→5；加总尝试次数预算；常量集中 | §3.2 step 3 |
| **A2.4 / Q1-Q2** | Resolver | 🔴→已修 | Modrinth 实测 700 IDs/批量 + 限流策略；新增 §3.6 | §3.2 step 1, §3.6 |
| **A3.1 / Q6** | Installer | 🔴→已修 | staging 改 `<instance>/.staging/`；EXDEV fallback；§4.5 新增 | §4.1, §4.5 |
| **A3.2** | Installer | 🔴→已修 | step 0 实例名冲突预检 | §4.1 step 0, §9 关键 QA |
| **librarian Q4** | Resolver | 🔴→已修 | NeoForge `game_versions` 二次校验 | §3.2 step 2 |
| **librarian Q3** | Resolver | 🟡→已修 | CF 客户端 429 重构作为前置依赖 | §5.1, §8 |
| **A1.2** | Schema | 🟡→已修 | `deprecated_after` / `replacement` Phase 1 必须解析 | §2.2 |
| **A1.3** | Schema | 🟡→已修 | overlay 加 `original_size` 字段 | §2.2 |
| **A2.2** | Resolver | 🟡→已修 | `incompatible` 改硬阻塞，details 折叠"忽略并继续" | §3.2 step 3 |
| **A2.3** | Resolver | 🟡→已修 | `ABANDONED_THRESHOLD_DAYS` 常量集中 | §3.3 |
| **A3.3** | Installer | 🟡→已修 | Phase 1 放弃中途取消，改"完成才能用" | §4.3 |
| **A4.1** | Contract | 🟡→已修 | `ModpackSubscription.pack_content_hash` 双层防御 | §4.2, §10 |
| **A5.1** | Layout | 🟡→已修 | 内部 trait `ResolverDataSource` 隔离测试 | §5.1 |
| **A6.1** | Testing | 🟡→已修 | 测试用例列举化，21 个 critical-path | §7.1, §7.2 |
| **A6.2 / Q2** | Testing | 🟡→已修 | 429 / `RateLimited` 进度上报 | §3.6, §4.4 |
| **librarian Q7** | Schema | 🟡→已修 | `pack_format` 字符串版本 + `preserve` 字段 + sha1/sha512 兼容 | §2.1, §2.2 |
| **A1.4** | Schema | 🟢→已修 | 显式声明未知字段丢弃 | §2.1 校验规则尾部 |
| **A2.5** | Resolver | 🟢→已修 | `cycle: Vec<CycleNode>` 结构化 | §3.5 |
| **A3.4** | Installer | 🟢→改实施时加注释 | overlay 顺序无影响，注释由实施者补 | — |
| **A4.2** | Contract | 🟢→已修 | Phase 1 加载 `user.toml` | §5.1, §10 |
| **A5.2** | Layout | 🟢→已修 | 显式不复用 mrpack | §5.1 |
| **A5.3** | Layout | 🟢→已修 | 拆 3 PR | §5.1 |
| **A6.3** | Testing | 🟢→已修 | 加 4 项异常场景 QA | §9 |

**最终结论**：rev2 spec 实施 ready。所有 5 个 CRITICAL 全部修订；所有 8 个 MAJOR 全部修订；所有 8 个 MINOR 全部修订或显式延期。

---

## 12. 实施进度（last updated 2026-06-01，main@v0.3.0-beta.1+1）

### 已完成

phase1-work 已 fast-forward 合并进 main，main 当前位于 `505ee65`，比上次 release tag `v0.2.0-beta.7` 领先 23 commits，413 个测试全绿（core 388 + cli 12 + gui 13），clippy `-D warnings` 全绿。

实现层面 §3-§5 全部落地（schema / cache / RateLimitedClient / resolver / registry / installer / LiveResolverDataSource / LiveInstallExecutor / new-instance modpack tab / detail modpack-sync tab）。

QA + 用户反馈 session 期间修复 / 优化（按时间顺序）：

1. `916d2b4 fix(modpack-source): install vanilla Minecraft before mod loader` — 整合包安装顺序错误（先装 loader 后装 vanilla）
2. `00180b0 fix(launch): fall back through download-mirror chain for vanilla MC manifest/meta` — `download_mirror = Official` 在不可达 piston-meta 的网络上整合包安装失败
3. `b4916e8 feat(modpack-source): GUI Phase 1 QA polish` — `<data-dir>/modpack-sources/user.toml` 未被 GUI 加载 / 同步 Tab emoji tofu / Pending mod 缺 explicit consent UI（resolve 后加 confirm modal）
4. `10c36a6 docs: sync CHANGELOG + Phase 1 spec progress with QA session 1`
5. **`a530a99 fix(gui): show launch progress so users know the game is starting (#11)`** — 解决 issue #11：点击 Launch 后 10–60 秒沉默；现在加进度阶段（Detecting Java… → Verifying files… → Repairing N files… → Extracting natives… → Starting JVM…）+ Toast + 按钮置灰；新增 `AppEvent::LaunchSpawned` 无副作用清 spinner；删除 dead `AppEvent::Error` variant
6. **`505ee65 feat(gui): explain modpack mod statuses via hover tooltips`** — 解决用户对 Pending/Deprecated/Abandoned 文案不理解：confirm modal 状态徽章加 hover tooltip 解释；同步 Tab 计数 chip 化（图标+数字+文字+tooltip）；tooltip delay 0.3s → 0.15s
7. **`0233a40 release: v0.3.0-beta.1`** — bump workspace 版本 + CHANGELOG 定稿，release workflow 自动构建并发布 Linux/Windows/macOS 五个 artifact
8. **post-beta.1：Cache 层接通 GUI** — `controller::modpack_source::handle_fetch_manifest` 接通 `core::modpack_source::cache::Cache`；fresh cache (≤6h TTL) 短路 HTTP；网络成功写回 cache；网络失败回退到 stale cache 并在 browse tab 顶部显示「网络不可达 — 显示本地缓存」橙色横幅；新增 `LauncherConfig::modpack_cache_dir()`、`CacheMeta::fresh_now()`；4 个新 GUI 测试覆盖 4 条路径（fresh hit / 网络成功写 cache / 网络失败 stale fallback / 无 cache 网络失败报错）；3 语 i18n 220 keys。**解锁 §9 DoD #6 + #13**。
9. **post-beta.1：「无法取消」i18n + 「重新解算」按钮** — bottom-bar spinner 在所有 launch / modpack-install 任务期间显示「Cannot be cancelled / 无法取消 / キャンセル不可」muted 提示替代消失的取消 ✕（之前 `cancellable` 谓词只排除 `launch:` 前缀，对 `modpack-install:*` 错误地保留 ✕，而 controller 的 active_tasks 又只跟踪 CreateInstance/DownloadJava，所以 ✕ 实际上是哑按钮）；install-confirm modal 加「Re-resolve / 重新解算 / 再解決」按钮，复用 pending_confirm 状态从 ResolutionReport 读 mc_version、从 pack 读 loader、重发 ResolveModpack；3 语 i18n 总 223 keys。**解锁 §9 DoD #7 + #11**。
10. **post-beta.1：NeoForge fixture（schema 验证 + 集成测试 + GUI 手测准备）** — 在 `crates/core/src/modpack_source/fixtures/` 加 `miao_neoforge_pack.json` + `miao_neoforge_manifest.json`，包含 3 个真实 NeoForge mod（JEI `u6dRKJwZ` / Architectury API `lhGA9TYQ` / AppleSkin `EsAfCjCV`）针对 1.21.1。`t_schema_12b_neoforge_fixture_parses` 验证 schema、loader、mc_versions、mod IDs 全部正确解析。**core 测试覆盖到此为止；§9 DoD #3 的 GUI 端到端验证（resolve → confirm modal → install → 启动游戏看到 mod 加载）需要由用户手动跑**：(a) 复制 fixture 内容到 `WangSimiao2000/miao-modpacks` 仓库的 `packs/miao-1.21.1-neoforge-test/pack.json`；(b) 把 fixture manifest 的 packs 数组项追加到 miao-modpacks 主 `manifest.json`；(c) 在 GUI 选「米奇喵整合包源」→ 选 NeoForge pack → 1.21.1 → 装好确认能启动并看到 JEI 界面。`neoforge_recheck` core 行为已被 `t_resolve_13_neoforge_game_versions_ambiguous_detected` 覆盖，所以这次手测主要看的是 happy path。

issue #12（桌面快捷方式）调研已做（Windows 不发 CLI 是 gap，需要先给 `miao-gui` 加 `--launch` argv 解析，详见 session-2 中 explore agent bg_fedca1ff 的输出），但**用户决定推迟**，未实施。

外部 fixture（github.com/WangSimiao2000/miao-modpacks）也跟进了：
- `df41565` — Indium 加 `deprecated_after: "1.21.1"`（演示 Deprecated 状态）+ 新增 Soft Imprints (`UhxJgB1o`) 作为真·Pending fixture（last_release 2026-05-30、不支持 1.21.4/5/7）
- 端到端验证通过：选 1.21.4 解算结果是 5 Compatible（含 2 个自动依赖补全 Architectury / Cloth Config）+ 1 Pending（Soft Imprints）+ 1 Deprecated（Indium）

§9 DoD 已通过的项见 §9 清单；剩余项要么 core 层测试覆盖（行为正确性），要么是边界 case 故障注入（断网/限流/磁盘满），未在本轮人工跑。

### 已知缺口（按重要性，0.3.0-rc 前应处理）

1. ~~**Cache 层未在 GUI 接通**~~ —— 已完成（见上方第 8 项）。fresh-hit 短路 HTTP；offline 回退 stale cache + UI banner；4 个回归测试。
2. **「重新解算」按钮缺失**：modpack browse tab 没有显式的"重新解算"按钮（只有"刷新"刷的是 manifest 不是 resolution）。后果：§9 第 11 项「狂点重新解算 → 限流倒计时」物理上无法触发。
3. **"无法取消" 提示文案**：spec §4.3 / §9 第 7 项明确要求 Phase 1 不实现取消并需告知用户。当前进度条 UI 没这文案。Issue #11 修复时也保留了这个 gap（launch 也不可取消，spinner ✕ 已隐藏，但没文案说明）。
4. **NeoForge / incompatible / 循环依赖 GUI 行为未验证**：core 测试覆盖了后端语义（t_install_07 incompatible / t_resolve_* cycle / NeoForge game_versions 二次校验），但 GUI 端未人工跑。米奇喵源现仅含 Fabric pack，需另造 NeoForge fixture。
5. **故障注入抽样未跑**：hosts 重定向回滚 / 磁盘满 / Modrinth 503 等，core 测试覆盖回滚语义但 UI 错误展示未验证。
6. **issue #12 桌面快捷方式未做**：用户向功能，需要 (a) `miao-gui --launch <name>` argv 解析，(b) `crates/gui/src/platform.rs` 加 `desktop_dir()` + `write_instance_shortcut()` 跨平台 helper，(c) sidebar instance card 加右键 context_menu。Linux 写 `.desktop` / macOS 写 `.command` / Windows shell-out PowerShell 写 `.lnk`。预估 ~1.5–2 天。

### 接下来要做什么（按优先级）

**P0 — `v0.3.0-beta.1` release 前必做**（0–1 天）：

1. 给 main 加 `release: v0.3.0-beta.1` commit：bump `Cargo.toml` workspace version `0.2.0-beta.7` → `0.3.0-beta.1`；CHANGELOG `[Unreleased]` 改为 `[0.3.0-beta.1] - YYYY-MM-DD`；release notes 直接 lift `[Unreleased]` 现有内容（已写好，含 Phase 1 摘要 + Pending modal + 启动反馈 + tooltip 解释）。参考 v0.2.0-beta.7 的 release commit 格式（`d1b0e69`）。
2. push main，打 tag `v0.3.0-beta.1`，让 release workflow 自动建 release。

**P1 — 0.3.0-rc 前应做**（按 ROI 排，每项独立可做）：

3. ~~**Cache 层接通 GUI**~~（半天）—— ✅ 已完成
4. **「重新解算」按钮**（2 小时）—— 解锁 §9 #11
5. **「无法取消」文案 i18n 三语**（1 小时）—— 解锁 §9 #7 + 同时覆盖 launch 流的"无法取消"
6. **NeoForge fixture** + GUI 跑 §9 #3（半天，外部仓库 + 测试，需要找 NeoForge mod 候选）
7. **抽样故障注入**（半天，hosts 重定向 / 磁盘满 / Modrinth 503 任选 3–5 项）
8. **issue #12 桌面快捷方式**（1.5–2 天，独立功能）

**P2 — Phase 2 设计阶段处理**：

9. user.toml GUI 编辑入口（spec §10「Phase 2 仅加 GUI 编辑入口」）
10. 整合包升级流程（消费 `pack_content_hash` 双层防御，§10）
11. 配置文件 overlay 更新流程（消费 `preserve` / `original_size`，§10）
12. CurseForge `ModStatus.Inactive/Abandoned` 自动检测（librarian 调研发现 CF 有此 API 字段，可作为 `deprecated_after` 之外的辅助信号；Modrinth 没有可靠的字段所以不做）

---

## 13. 文档规划

`docs/` 目录里的现有文档（`architecture.md` / `modpack-source.md` / `modpack-source-phase1-spec.md` / `cross-platform.md` / `roadmap.md` / `testing.md` / `ui-design.md`）**面向开发者 / 维护者**，描述内部架构、设计决策、实现 spec。**不是**给玩家或整合包作者看的。

### 13.1 用户向文档的位置

不放在 `docs/`，按以下二阶段策略推进：

**短期（0.3.0-beta.1 这个月，最小可用方案）**：用 [GitHub Wiki](https://github.com/WangSimiao2000/MiaoMinecraftLauncher/wiki)，零配置即开即用。

**长期（用户量上来 / Phase 2 期间）**：迁移到仓库内 `website/` 子目录 + GitHub Pages（mkdocs / VitePress / Docusaurus 任选），跟代码一起 PR。

### 13.2 0.3.0-beta.1 发布前必做

- [ ] 在 `docs/` 加 `README.md`，明确说明 docs/ 是开发文档，并指向用户文档位置（Wiki 或 website/）。让贡献者一眼分清。
- [ ] `README.md` Features 表加一行 「Modpack Sources」简介，链接到 Wiki / website。
- [ ] 创建 GitHub Wiki 首页（`Home`），列出可用的用户向页面骨架（哪怕暂时是 stub）。

### 13.3 0.3.0-beta.1 之后第一周应写的用户向文档

外部整合包作者要开始照着写自己的 pack 了，需要这些：

#### A. **整合包作者指南**（`Modpack-Author-Guide`）—— 最高优先级

一篇正经文档，覆盖以下事实点（这些是 user session 期间被反复问到的）：

1. **三层模型**：源 → 整合包 → mod 列表
2. **核心范式转变**：mrpack 是静态快照（绑定 1 个 MC 版本），米奇喵源是声明式（一份 pack 跨多个 MC 版本智能适配）
3. **作者 vs 启动器的职责划分**：作者声明意图 / 启动器解算事实
4. **ResolvedStatus 六种状态对比表**（重点）：
   - 每种状态：数据来源 / 触发条件 / 玩家可见行为 / 作者应如何响应
   - **特别澄清** Deprecated vs Abandoned 的判定差异（决策树形式）：
     ```
     这个 mod 在某个 MC 版本上不再适用？
     ├─ 整个 mod 完全死了，没救 → 直接从 pack.json 删
     ├─ 只在新 MC 版本上不适用，旧版本还要用 → deprecated_after: "X"
     └─ 全 MC 版本都还能用，只是作者最近没更新 → 啥都不做（让 resolver 推断为 Abandoned）
     ```
5. **pack.json 字段每一项的「使用场景」**：
   - `policy: "auto"` vs `"lock"` —— 哪种场景下选哪个
   - `criticality: "core"` vs `"optional"` —— 影响什么（Phase 1 仅 UI 颜色，Phase 2 用于安装中止判定）
   - **`deprecated_after`** —— 详细说明 + 真实例子（Indium / Sodium FRAPI）。**重点澄清这个不是 API 接口、不是文本检索，是作者手写的字段**。
   - `replacement` —— Phase 1 仅记录、Phase 2 才消费，但作者现在就该写
   - `config_overlay.files[].preserve` —— first-install-only 含义
   - `loader_versions` —— 何时手写、何时让 MMCL 自动选
6. **跨 MC 版本支持的最佳实践**：
   - "我想让 1.21.0–1.21.7 都用一份 pack" 的写法
   - 已知 mod 在新版本被取代（FRAPI/Indium 案例）
   - 已知 mod 还没出新版本（Pending 模式自然处理）
7. **数据来源的明确说明**（澄清 user 反复问的疑问）：
   - **明确写出**：`deprecated_after` 不是接口，是作者**手写**字段
   - **MMCL 完全没用**：Modrinth `status: "archived"`、CurseForge `ModStatus.Inactive/Abandoned`（Phase 2 计划接 CF 那侧）
   - **MMCL 完全没做**：description 关键词扫描（不靠谱、误报多）
8. **完整可运行示例**：一份带详细注释的 pack.json（每行 // 这个字段表示...），多种场景：纯 fabric / 跨版本 / 有依赖 / 有 config overlay
9. **测试与发布流程**：本地用 MMCL 测试 / 模拟玩家选择不同 MC 版本看 resolved.json / 解读 status 计数器 / 提交到 manifest.json 之前的 sanity checklist

#### B. **玩家 FAQ**（`FAQ`）

- 「我点了安装为什么有些 mod 没装上？」→ 解释 status 分类
- 「为什么 launching 之后还要等很久才出现 MC 窗口？」→ 解释 JVM 冷启动 + classpath + mod loader bootstrap + LWJGL
- 「整合包同步 Tab 的数字什么意思？」→ 解释 chip
- 「兼容、待支持、已弃用、已弃坑、冲突、版本歧义都是什么意思？」→ link 到作者指南的状态表

#### C. **`docs/modpack-source.md` 增补**（保持开发文档定位）

- §4.3 Mod 状态分类的表格里**每行加一个 "数据来源" 列**（pack 声明 / Modrinth API / CF API / resolver 推断）
- §5 升级流程要把 `deprecated_after` 在 Phase 2 升级中怎么消费写清楚

### 13.4 文档间的交叉引用

- 用户向文档（Wiki / website）→ link 回 spec 给好奇心强的作者读
- 开发文档（docs/）→ 在 `docs/README.md` link 出去到用户文档位置
- 双向 link 让两个受众都能从自己关心的入口出发

---

## 下次启动 session 的最快路径

```bash
git checkout main
git pull
git log --oneline -10  # 应看到 505ee65 / a530a99 / 10c36a6 + 之前的 phase1 commits
cargo test --workspace --no-fail-fast  # 确认 413 tests 全绿
```

最直接的 next step：**发 `v0.3.0-beta.1`**。CHANGELOG `[Unreleased]` 已经写好（含 Phase 1 摘要、Pending modal、launch 反馈 #11、tooltip 解释、download-mirror fallback、emoji tofu fix、user.toml 加载 fix），release commit 直接照 v0.2.0-beta.7（`d1b0e69`）的格式 lift 即可。

如果决定先补 P1 一两项再发，从「Cache 层接通到 GUI」开始最值（半天解锁两个 §9 DoD 项）。
