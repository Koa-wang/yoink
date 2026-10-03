# yoink

一个用 Rust 编写的终端剪贴板管理器。后台守护进程持续监听系统剪贴板，把历史记录存入 SQLite；TUI 里可以搜索、选择、一键复制历史内容。

命令名是 `yoinker`。

## 功能

- **守护进程**：监听剪贴板（macOS 用 changeCount 事件检测），自动记录文本、来源应用、时间戳；相同内容去重（只更新时间戳）；支持忽略规则和自动清理；SQLite 使用 WAL 模式。
- **TUI**：模糊搜索、键盘导航、详情预览窗格、相对时间、复制即退出、删除、固定、清空、`1-9` 快速选择、暗色/亮色主题。
- **CLI**：`get` / `set` / `search` / `history` / `clear` / `rm` / `pin` / `config` 等脚本化命令。

## 安装

```sh
cargo build --release
# 二进制在 target/release/yoinker
```

安装到 PATH：

```sh
cargo install --path .
# 安装到 ~/.cargo/bin/yoinker
```

## 使用

```sh
yoinker                    # 打开 TUI（默认）
yoinker tui                # 同上
yoinker daemon             # 启动后台守护进程（Unix 下会后台化）
yoinker daemon --foreground
yoinker get                # 输出当前剪贴板内容到 stdout
yoinker set "some text"    # 设置剪贴板内容
yoinker search "query"     # 非交互式模糊搜索
yoinker history            # 以纯文本列出历史
yoinker clear              # 清空历史
yoinker rm <id>            # 按 id 删除某条
yoinker pin <id>           # 固定某条（加 --unpin 取消固定）
yoinker config             # 显示配置（加 --edit 用 $EDITOR 打开）
```

### TUI 按键

| 按键 | 功能 |
| --- | --- |
| `j` / `k` / `↑` / `↓` | 上下选择 |
| `g` / `G` | 跳到顶部 / 底部 |
| `Enter` | 复制选中项到剪贴板 |
| `y` | 复制选中项并退出 |
| `/` | 进入搜索模式 |
| `Esc`（搜索中） | 取消搜索 |
| `Enter`（搜索中） | 接受搜索结果 |
| `d` | 删除选中项 |
| `D`（按两次） | 清空全部历史 |
| `p` | 固定 / 取消固定选中项 |
| `t` | 切换暗色 / 亮色主题（自动保存） |
| `1`–`9` | 快速选择前 9 条 |
| `q` / `Esc` / `Ctrl-C` / `Ctrl-D` / `Ctrl-Q` | 退出 |

## 数据与配置

- 配置文件：`$XDG_CONFIG_HOME/yoink/config.toml`
  （macOS 为 `~/Library/Application Support/yoink/config.toml`）
- 数据库：`$XDG_DATA_HOME/yoink/yoink.db`
  （macOS 为 `~/Library/Application Support/yoink/yoink.db`）

```toml
max_entries = 1000            # 最大历史条数
history_days = 30             # 保留天数，过期自动删除
ignored_patterns = []         # 忽略的正则模式列表
theme = "dark"                # "dark" 或 "light"
preview_length = 80           # TUI 列表预览截断长度
relative_time = true          # TUI 显示相对时间（如“3 分钟前”）

[daemon]
poll_interval_ms = 500        # 剪贴板轮询间隔（macOS 用事件检测，此值仅作回退）
```

固定（pinned）的条目不会被自动清理。

## 平台说明

- macOS：守护进程用 `NSPasteboard.changeCount` 检测剪贴板变化，来源应用用 `NSWorkspace` 检测。
- macOS 和 Linux：`yoinker daemon` 会后台化运行。
- Windows：不支持后台化，请用 `yoinker daemon --foreground`。
- 剪贴板来源应用检测为尽力而为，失败时回退为 `unknown`。

## 开发

```sh
cargo test                 # 运行单元测试
cargo build --release
```

GitHub Actions CI 会跑通 `cargo test` 和 release 构建。
