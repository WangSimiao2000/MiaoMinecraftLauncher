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
| Java 自动检测 + 下载 | ✅ | ✅ | — | Adoptium |
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
| 多游戏文件夹管理 | ❌ | ✅ | Medium | 切换 .minecraft 目录 |
| 版本分类与收藏 | ❌ | ✅ | Low | |
| 内置帮助库 | ❌ | ✅ | Low | 新手教程 |
| 联机 (P2P/穿透) | ❌ | ✅ | Low | 实现难度极高 |
| 自动备份存档 | ❌ | ✅ | Low | |

### UI / 主题

| Feature | MMCL | PCL2 | Priority | Notes |
|---------|------|------|----------|-------|
| 自定义背景图 + 模糊 | ✅ | ✅ | — | `gui/src/background.rs` |
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
└── app.rs (MiaoApp — 全状态单体结构)
    ├── views/ (视图渲染 — 函数式)
    │   ├── sidebar.rs      → 实例列表侧边栏
    │   ├── detail.rs       → 中央面板路由 (Tab 分发)
    │   ├── mods.rs         → Mod 管理 Tab
    │   ├── resources.rs    → 资源包/光影 Tab
    │   ├── worlds.rs       → 世界存档 Tab
    │   ├── log.rs          → 游戏日志 Tab
    │   └── instance_settings.rs → 实例设置 Tab
    │
    ├── dialogs/ (弹窗)
    │   ├── new_instance.rs → 新建实例
    │   ├── settings.rs     → 全局设置（全页面）
    │   └── java_confirm.rs → Java 下载确认
    │
    ├── controller/ (异步业务层)
    │   ├── mod.rs          → 事件循环 + 任务调度
    │   ├── instance.rs     → 实例操作
    │   ├── auth.rs         → 登录
    │   ├── java.rs         → Java 下载
    │   ├── versions.rs     → 版本/加载器获取
    │   └── mods.rs         → Mod 搜索安装
    │
    ├── state.rs            → UI 状态定义
    ├── messages.rs         → 命令/事件协议
    └── theme.rs            → 设计令牌 + 样式函数
```

**MMCL UI 特点:**
- **扁平 2 层路由**: Main (侧栏+Tab) / Settings，无导航栈
- **无自定义控件**: 所有 UI 元素直接用 egui 原语 + theme 辅助函数内联构建
- **单体状态**: `MiaoApp` 结构持有 ~30 个字段，所有页面共享
- **Immediate Mode**: egui 每帧重绘，无 retained widget state
- **Controller 分离**: 异步业务逻辑与 UI 渲染解耦（channel 通信）

### 值得借鉴的 PCL2 设计模式

| PCL2 模式 | MMCL 现状 | 建议改进 |
|-----------|----------|---------|
| **自定义控件库** — 20+ 独立可复用控件 | 无独立控件，所有 UI 内联 | 抽取 `widgets/` 目录：ModCard, InstanceCard, SearchBox, ProgressCard, HintBanner |
| **卡片容器 (MyCard)** — 可展开/折叠 + 标题 | 直接用 egui::Frame | 实现 `CollapsibleCard` widget — 设置页/Mod 列表都需要 |
| **深层导航栈** — 页面可任意嵌套 + 返回 | 仅 2 个顶级 AppView | 加入 `NavigationStack<Page>` — 支持 Mod 详情页、版本详情页等深层页面 |
| **虚拟化长列表** | egui ScrollArea 全量渲染 | 对 Mod 列表/版本列表使用 egui 的 `show_rows()` 虚拟化 |
| **消息/Toast 系统** | 仅底部状态栏文字 | 加入 Toast 通知队列 — 成功/警告/错误有不同样式和自动消失 |
| **左右分栏范式** | 侧栏 + Tab 内容 | 下载页/Mod 详情页采用左列表右详情的分栏 |
| **提示条 (MyHint)** | 无 | 在关键操作前显示 info/warn 提示条 |
| **异步图片加载** | egui_extras image_loaders 已启用 | 利用已有能力给 Mod 搜索结果加图标/缩略图 |
| **页面切换动画** | 无任何过渡 | egui 支持 `lerp` 和 `animate_value_with_time`，可做淡入/滑动 |

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
| 7 | 自定义背景图 + 亚克力模糊 | ✅ Done | `gui/src/background.rs` |
| 8 | Toast 通知系统 | ✅ Done | `gui/src/toast.rs` |
| 9 | Widget 抽取（ModCard, InstanceCard, CollapsibleCard） | ✅ Done | `gui/src/widgets/` |
| 10 | 导航栈 + 页面过渡动画 | ✅ Done | `gui/src/navigation.rs` |
| 11 | OptiFine 一键安装 | ✅ Done | `core/src/modloader/optifine.rs` |
| 12 | 文件完整性检查与自动补全 | ✅ Done | `core/src/integrity.rs` |

### Phase 3 — 进阶功能

| # | Feature | 预估工时 | 依赖 |
|---|---------|---------|------|
| 13 | 多游戏文件夹管理 | 0.5 天 | 无 |
| 14 | 版本分类/收藏夹 | 0.5 天 | 无 |
| 15 | 自定义离线皮肤 | 1 天 | 无 |
| 16 | 自动备份存档 | 1 天 | 无 |
| 17 | 虚拟化长列表优化 | 0.5 天 | 无 |
| 18 | 内置帮助 FAQ | 1 天 | 无 |
| 19 | 联机穿透 (P2P) | 5+ 天 | 复杂度极高 |

---

## Non-Feature Improvements

| 项目 | 状态 | 描述 |
|------|------|------|
| CI 跨平台矩阵 | ⬜ | 加 macOS + Windows 构建到 CI |
| CLI 集成测试 | ⬜ | 用 assert_cmd 测试命令行 |
| 去除 async-trait/async-recursion | ⬜ | nightly 原生支持 |
| Dead code 清理 (theme.rs) | ⬜ | 启用或删除 7 个 unused 项 |
| MS_CLIENT_ID 环境变量化 | ⬜ | 安全性改善 |
| 日志系统 (tracing) | ⬜ | GUI 目前无结构化日志 |
| 升级 eframe 0.30 → 0.34+ | ⬜ | wgpu 后端支持 Wayland 窗口透明/圆角；egui/egui_extras/egui_glow 需同步升级；预计 50-100 行 breaking changes 适配 |
