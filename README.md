# Server Status Bar

一个运行在 Windows 上的桌面状态栏，实时监控阿里云 ECS、VMware ESXi 主机和 DeepSeek 账户余额。使用 **Rust + egui/eframe** 原生 GUI。

> 本分支（`rust-egui`）为 Rust 重写版；原 Go + lxn/walk 实现保留在 `dev`/`main` 分支。

## 功能特性

- **阿里云 ECS 监控**: 自动发现所有实例，显示 CPU 占用率、内存占用、运行状态（RPC API 正确 HMAC-SHA1 签名）
- **ESXi 监控**: 通过 VMware Host Client REST API 获取数据，支持 frp 内网穿透访问，显示 CPU 占用率、内存占用、运行中 VM 数量
- **DeepSeek 余额查询**: 实时显示账户余额和已用金额
- **置顶悬浮窗口**: 始终置顶，可拖动，可隐藏到系统托盘
- **自动刷新**: 每 5 秒自动刷新数据（后台线程采集，不阻塞 UI）
- **设置界面**: 多 Tab 窗口编辑各模块凭证（阿里云 / ESXi / DeepSeek，密码字段）
- **托盘菜单**: 显示/隐藏、设置、单独开关各模块、退出

## 安全设计

**凭证仅保存在内存中，绝不写入磁盘。**

- `config.json` 只保存非敏感设置（region、base_url、insecure、模块显隐开关）
- 每次启动程序需在设置窗口重新输入凭证
- 进程退出后内存中的凭证自然释放

## 编译和运行

### 前置要求

- Rust 1.85+（stable）
- Windows 10/11（依赖系统字体 `msyh.ttc` 渲染中文）

### 编译

```bash
cargo build --release
```

### 运行

```bash
cargo run --release
# 或直接运行 target/release/statusbar.exe
```

程序启动后，点击窗口底部的 **设置** 按钮（或右键系统托盘图标选择 **设置...**），在设置窗口中输入各模块凭证。

## 配置文件

程序会在可执行文件同目录下自动生成 `config.json`（仅含非敏感设置）：

```json
{
  "aliyun": {
    "region": "cn-hangzhou"
  },
  "esxi": {
    "insecure": true
  },
  "deepseek": {
    "base_url": "https://api.deepseek.com"
  },
  "display": {
    "show_aliyun": true,
    "show_esxi": true,
    "show_deepseek": true
  }
}
```

## 使用方法

- 窗口始终置顶，可拖动到屏幕任意位置
- 点击窗口底部 **设置** 按钮或右键托盘图标，编辑/更新各模块凭证
- 右键托盘图标可：显示/隐藏、设置、单独开关各模块、退出
- 左键单击托盘图标可快速显示/隐藏窗口

## 凭证获取

### 阿里云

1. 登录阿里云控制台
2. 进入 AccessKey 管理页面
3. 创建或获取 AccessKey ID 与 Secret

### ESXi

- URL: ESXi 管理界面地址
  - 直连: `https://ESXi-IP:443`
  - 通过 frp 穿透: `https://云主机IP:映射端口`
  - 域名: `https://esxi.example.com:443`
- User: 通常为 `root`
- Password: root 用户的密码

### DeepSeek

1. 登录 https://platform.deepseek.com/
2. 进入 API Keys 页面
3. 创建或复制你的 API Key

## 项目结构

```
src/
├── main.rs          # 入口：窗口选项、中文字体加载
├── app.rs           # 主应用：egui 界面、5 秒刷新、托盘事件
├── config.rs        # 非敏感配置读写（config.json）
├── credential.rs    # 内存凭证（不落盘）
├── settings.rs      # 设置窗口（多 Tab）
├── tray.rs          # 系统托盘
└── collector/
    ├── aliyun.rs    # 阿里云 RPC API（HMAC-SHA1 签名）
    ├── esxi.rs      # ESXi Host Client REST API
    └── deepseek.rs  # DeepSeek 余额 API
```
