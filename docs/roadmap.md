# Feature Roadmap & Gap Analysis

Status legend: ✅ Done | 🔧 In Progress | ⬜ Planned | 💡 Idea

## Feature Gap: MMCL vs PCL2

Comparison baseline: PCL2 (Plain Craft Launcher 2) — the most popular Chinese Minecraft launcher.

### Core Functionality

| Feature | MMCL | PCL2 | Priority | Notes |
|---------|------|------|----------|-------|
| Microsoft 正版登录 | ✅ | ✅ | — | Device code flow + auto-refresh |
| 离线登录 | ✅ | ✅ | — | |
| Forge/NeoForge/Fabric/Quilt | ✅ | ✅ | — | |
| LiteLoader | ❌ | ✅ | Low | 已停止维护，不优先 |
| Modrinth 搜索安装 | ✅ | ✅ | — | 含依赖解析 |
| CurseForge 搜索安装 | ✅ | ✅ | — | curseforge/ 模块，需配置 API key |
| 整合包导入导出 (mrpack) | ✅ | ✅ | — | |
| 整合包导出 (CurseForge 格式) | ❌ | ✅ | Medium | |
| Java 自动检测 + 下载 | ✅ | ✅ | — | Mojang / BMCLAPI / Adoptium Temurin / Microsoft 四源可选 |
| BMCLAPI 镜像 | ✅ | ✅ | — | |
| 多下载源自动切换 | ✅ | ✅ | — | 指数退避重试 + mirror 链式切换 |
| 单文件多线程下载 | ❌ | ✅ | Low | 当前只有多文件并发 |
| 资源包/光影包管理 | ✅ | ✅ | — | |
| 世界存档管理 | ✅ | ✅ | — | |
| 实例隔离 | ✅ | ✅ | — | |
| 跨平台 (Win/Mac/Linux) | ✅ | ❌ (Win only) | — | MMCL 优势 |
| CLI 接口 | ✅ | ❌ | — | MMCL 优势 |

### 用户体验

| Feature | MMCL | PCL2 | Priority | Notes |
|---------|------|------|----------|-------|
| 智能崩溃分析 | ✅ | ✅ | — | crash/ 模块，解析 crash-report + latest.log |
| OptiFine 一键安装 | ✅ | ✅ | — | `core/src/modloader/optifine.rs` |
| 自动文件补全 | ✅ | ✅ | — | `core/src/integrity.rs` |
| 自动更新 | ✅ | ✅ | — | update/ 模块，下载+替换+回滚 |
| 第三方皮肤站登录 | ✅ | ✅ | — | authlib-injector Yggdrasil 协议 |
| 自定义离线皮肤 | ❌ | ✅ | Low | |
| 多游戏文件夹管理 | ✅ | ✅ | — | 切换 .minecraft 目录 |
| 版本分类与收藏 | ❌ | ✅ | Low | |
| 内置帮助库 | ✅ | ✅ | — | FAQ 页面 |
| 联机 (P2P/穿透) | ❌ | ✅ | Low | 实现难度极高 |
| 自动备份存档 | ❌ | ✅ | Low | |

### UI / 主题

| Feature | MMCL | PCL2 | Priority | Notes |
|---------|------|------|----------|-------|
| ~~自定义背景图 + 模糊~~ | ❌ | ✅ | — | 已移除，不需要此功能 |
| 主题色切换 | ✅ | ✅ | — | ThemePreset 已启用 + 持久化 |
| 自定义背景音乐 | ❌ | ✅ | Low | |
| 自定义主页 (XAML) | ❌ | ✅ | Low | 不适用于 egui |
| 页面切换动画 | ✅ | ✅ | — | `gui/src/navigation.rs` |
| 功能模块显隐控制 | ❌ | ✅ | Low | |

---

## UI Architecture Comparison

### PCL2 架构 (WPF / VB.NET)

```
FormMain.xaml (主窗口)
├── Pages/ (页面系统 — 导航栈 + 左右分栏)
│   ├── PageLaunch/       → 启动主页（版本选择 + 一键启动）
│   ├── PageDownload/     → 下载中心（版本/Mod/整合包下载）
│   ├── PageInstance/     → 版本管理（已安装版本列表 + 设置）
│   ├── PageLink/         → 联机大厅
│   ├── PageSetup/        → 全局设置（多 Tab）
│   ├── PageOther/        → 关于/更新/赞助
│   ├── PageSelectLeft/Right  → 左右分栏选择器
│   └── PageSpeedLeft/Right   → 快速启动分栏
│
├── Controls/ (20+ 自定义控件库)
│   ├── MyButton, MyIconButton, MyIconTextButton, MyExtraButton
│   ├── MyCard                → 卡片容器（展开/折叠 + 标题）
│   ├── MyCheckBox, MyRadioBox, MyRadioButton
│   ├── MyComboBox, MyComboBoxItem
│   ├── MySlider, MySliderDot
│   ├── MyTextBox, MySearchBox
│   ├── MyListItem           → 带图标 + 描述的列表项
│   ├── MyLoading            → 加载动画
│   ├── MyHint               → 提示条 (info/warn/error)
│   ├── MyScrollBar, MyScrollViewer
│   ├── MyImage              → 异步加载图片
│   ├── MyPageLeft, MyPageRight → 分栏页面容器
│   ├── MyVirtualizingElement → 虚拟化长列表
│   ├── MyMsg/               → 消息弹窗系统
│   └── Behaviors/           → 附加行为（动画等）
│
└── Modules/ (业务逻辑)
    ├── Base/       → 基础工具（网络/IO/动画/日志）
    ├── Minecraft/  → MC 启动/版本/认证/Mod
    ├── Resource/   → 资源管理
    └── ThirdParty/ → 第三方集成
```

**PCL2 UI 特点:**
- **深层页面导航**: 6 个顶级页面，每个页面有左右分栏子页面，支持导航栈 + 返回
- **完整控件库**: 20+ 自定义控件，每个控件独立文件，有统一的动画和交互风格
- **分栏布局**: 核心范式是 Left + Right 双栏，左栏导航/列表，右栏详情
- **卡片系统**: `MyCard` 是核心容器 — 可展开/折叠、带标题栏
- **虚拟化**: `MyVirtualizingElement` 处理长列表性能
- **消息系统**: 独立 `MyMsg/` 目录实现 Toast/Dialog/Confirm

### MMCL 架构 (egui / Rust)

```
main.rs (入口)
├── app.rs (MiaoApp — 全状态单体结构)
├── icons.rs              → Bootstrap Icons 常量
├── navigation.rs         → NavigationStack + 页面过渡动画
├── toast.rs              → Toast 通知队列
├── theme.rs              → 设计令牌 + 4 套调色板
├── blur.rs               → GPU 高斯模糊 shader (对话框背景)
│
├── views/ (视图渲染 — 函数式)
│   ├── sidebar.rs        → 实例列表 + Settings 按钮
│   ├── detail.rs         → 中央面板路由 (Tab 分发)
│   ├── mods.rs           → Mod 管理 (统一搜索 Modrinth/CurseForge)
│   ├── resources.rs      → 资源包/光影 Tab
│   ├── worlds.rs         → 世界存档 Tab
│   ├── log.rs            → 游戏日志 Tab
│   └── instance_settings.rs → 实例设置 Tab
│
├── dialogs/ (弹窗/全页面)
│   ├── new_instance.rs   → 新建实例
│   ├── settings.rs       → 全局设置（Account/Appearance/Data/Java/About）
│   ├── java_confirm.rs   → Java 下载确认
│   └── setup_wizard.rs   → 首次启动引导 (Language → Java → Done)
│
├── locales/ (i18n JSON 翻译文件)
│   ├── en.json           → English (内置)
│   ├── zh.json           → 中文 (内置)
│   └── *.json            → 社区贡献语言 (运行时从 data_dir/locales/ 加载)
│
├── controller/ (异步业务层)
│   ├── mod.rs            → 事件循环 + 命令分发
│   ├── instance.rs       → 实例创建/启动/导入导出
│   ├── auth.rs           → MS OAuth 登录 + 皮肤获取
│   ├── java.rs           → Java 下载
│   ├── versions.rs       → 版本/加载器获取
│   └── mods.rs           → Modrinth + CurseForge 搜索/安装/更新检测
│
├── widgets/
│   ├── instance_card.rs  → 实例卡片
│   ├── mod_card.rs       → Mod 卡片 (启用/禁用/删除)
│   └── collapsible_card.rs → 可折叠区域
│
└── state.rs + messages.rs → UI 状态 + 命令/事件协议
```

**MMCL UI 特点:**
- **导航栈路由**: Main / Settings / ModDetail，支持 push/pop + 0.2s 淡入动画
- **自定义 Widget 库**: InstanceCard, ModCard, CollapsibleCard
- **图标系统**: Bootstrap Icons 字体内嵌（464KB），15 个矢量图标常量
- **字体**: MiSans Medium 内嵌（7.8MB），统一中英文渲染
- **单体状态**: `MiaoApp` 结构持有所有 UI 状态，各页面共享
- **Immediate Mode**: egui 每帧重绘，Toast 队列 + 动画值插值
- **Controller 分离**: 异步业务逻辑与 UI 渲染通过 mpsc channel 解耦
- **皮肤系统**: Mojang session server → 本地裁剪缓存 → file:// 加载

### 值得借鉴的 PCL2 设计模式

| PCL2 模式 | MMCL 现状 | 建议改进 |
|-----------|----------|---------|
| **自定义控件库** — 20+ 独立可复用控件 | ✅ widgets/ 已有 3 个控件 | 继续抽取：SearchBox, ProgressCard, HintBanner |
| **卡片容器 (MyCard)** — 可展开/折叠 + 标题 | ✅ CollapsibleCard 已实现 | — |
| **深层导航栈** — 页面可任意嵌套 + 返回 | ✅ NavigationStack 已实现 | 可扩展更多 Page 类型 |
| **虚拟化长列表** | egui ScrollArea 全量渲染 | 对 Mod 列表/版本列表使用 egui 的 `show_rows()` 虚拟化 |
| **消息/Toast 系统** | ✅ ToastQueue 已实现 | — |
| **左右分栏范式** | 侧栏 + Tab 内容 | 下载页/Mod 详情页采用左列表右详情的分栏 |
| **提示条 (MyHint)** | Toast 部分覆盖 | 在关键操作前显示 info/warn 提示条 |
| **异步图片加载** | ✅ egui_extras all_loaders | 利用已有能力给 Mod 搜索结果加图标/缩略图 |
| **页面切换动画** | ✅ 0.2s opacity transition | — |

---

## Planned Features (优先级排序)

### Phase 1 — 核心差距补齐 ✅

| # | Feature | 状态 | 模块 |
|---|---------|------|------|
| 1 | CurseForge API 集成（搜索/下载/安装） | ✅ Done | `core/src/curseforge/` |
| 2 | 智能崩溃分析（解析 crash-report + latest.log） | ✅ Done | `core/src/crash/` |
| 3 | 自动更新（一键下载替换） | ✅ Done | `core/src/update/` |
| 4 | 多下载源容灾（自动切换 + 失败重试） | ✅ Done | `core/src/download/manager.rs` |
| 5 | 第三方皮肤站 (authlib-injector) | ✅ Done | `core/src/auth/authlib_injector.rs` |

### Phase 2 — 体验增强 ✅

| # | Feature | 状态 | 模块 |
|---|---------|------|------|
| 6 | UI 主题切换（启用 ThemePreset + 用户自定义色） | ✅ Done | `core/src/config.rs`, `gui/src/theme.rs`, `gui/src/dialogs/settings.rs` |
| 7 | ~~自定义背景图 + 亚克力模糊~~ | ❌ Removed | 已移除，功能不需要 |
| 8 | Toast 通知系统 | ✅ Done | `gui/src/toast.rs` |
| 9 | Widget 抽取（ModCard, InstanceCard, CollapsibleCard） | ✅ Done | `gui/src/widgets/` |
| 10 | 导航栈 + 页面过渡动画 | ✅ Done | `gui/src/navigation.rs` |
| 11 | OptiFine 一键安装 | ✅ Done | `core/src/modloader/optifine.rs` |
| 12 | 文件完整性检查与自动补全 | ✅ Done | `core/src/integrity.rs` |

### Phase 3 — 进阶功能 ✅

| # | Feature | 状态 | 模块 |
|---|---------|------|------|
| 13 | 多游戏文件夹管理 | ✅ Done | `gui/src/dialogs/settings.rs` |
| 14 | 虚拟化长列表优化 | ✅ Done | `show_rows()` 全面应用 |
| 15 | 内置帮助 FAQ | ✅ Done | `gui/src/dialogs/settings.rs` render_tab_help |
| 16 | Mod 搜索分页 (Load More) | ✅ Done | Modrinth offset + CurseForge index |
| 17 | 已安装 Mod 更新检测 | ✅ Done | SHA-1 hash → Modrinth update API |
| 18 | 拖拽导入 Mod/资源包 | ✅ Done | `gui/src/views/mods.rs` handle_file_drop |
| 19 | 首次启动引导 | ✅ Done | `gui/src/dialogs/setup_wizard.rs` |
| 20 | i18n 外置化 (JSON locale files) | ✅ Done | `gui/locales/` + runtime loading |

### Phase 3.5 — Post-beta.3 增强 ✅

| # | Feature | 状态 | 模块 |
|---|---------|------|------|
| P1 | Mojang JRE 安装源（替换 Adoptium） | ✅ Done | `core/src/java/install.rs`, `core/src/java/mojang.rs` |
| P2 | Java 安装源选择器 | ✅ Done | `gui/src/dialogs/settings.rs` |
| P3 | 启动前文件完整性检查 + 自动修复 | ✅ Done | `core/src/integrity.rs` 接入启动流程 |
| P4 | 自定义主题 (TOML 文件) | ✅ Done | `core/src/custom_theme.rs` + `<data_dir>/themes/` |
| P5 | 新增 Sakura / Light 主题 + Morandi 调色 | ✅ Done | `gui/src/theme.rs` |
| P6 | 日语本地化 + 系统 CJK 字体回退 | ✅ Done | `gui/locales/ja.json`, `gui/src/main.rs::find_system_fallback_font` |
| P7 | Windows 控制台抑制（启动 Java 不闪窗） | ✅ Done | `core/src/process.rs` |
| P8 | 多 Java 下载源（BMCLAPI / Adoptium Temurin / Microsoft）+ archive 模式安装器 | ✅ Done | `core/src/java/{adoptium,microsoft,bmclapi,extract}.rs` |

### Phase 4 — 未来计划

| # | Feature | 预估工时 | 依赖 |
|---|---------|---------|------|
| 21 | 版本分类/收藏夹 | 0.5 天 | 无 |
| 22 | 自定义离线皮肤 | 1 天 | 无 |
| 23 | 自动备份存档 | 1 天 | 无 |
| 24 | 实例克隆/复制 | 0.5 天 | 无 |
| 25 | 联机穿透 (P2P) | 5+ 天 | 复杂度极高 |

---

## Non-Feature Improvements

| 项目 | 状态 | 描述 |
|------|------|------|
| CI 跨平台矩阵 | ✅ | Linux + Windows + macOS (x86_64 + aarch64) 全部进 release.yml；macOS 产出 .app bundle |
| CLI 集成测试 | ✅ | `crates/cli/tests/cli.rs` 用 assert_cmd，12 个测试，沙箱化 XDG/APPDATA |
| 去除 async-trait/async-recursion | ✅ | nightly 原生 async fn in trait |
| Dead code 清理 (theme.rs) | ✅ | 已清理 |
| MS_CLIENT_ID 环境变量化 | ✅ | `option_env!("MS_CLIENT_ID")` + 默认值 fallback，对齐 CURSEFORGE_API_KEY |
| 日志系统 (tracing) | ✅ | GUI: stderr + 每日滚动文件 (`<data_dir>/logs/mmcl.log.*`) + panic hook |
| 升级 eframe 0.30 → 0.34+ | ✅ | 已升级到 egui 0.34 |
| 动画系统 | ✅ | Spring 物理 + MD3 easing + egui_animation |
| 自动替换二进制 (self_update) | ❌ | 移除：代码从未接入 UI，三平台一致用"跳浏览器手动下"流程；macOS .app bundle 替换需要 Sparkle/Tauri-style 实现，未来再考虑 |
| 全局动效覆盖 | ✅ | 所有交互组件 hover/select/transition 有动效 |
