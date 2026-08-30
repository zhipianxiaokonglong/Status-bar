# Server Status Bar

一个运行在 Windows 上的桌面状态栏，实时监控阿里云 ECS、VMware ESXi 主机和 DeepSeek 账户余额。使用 Go + [Walk](https://github.com/lxn/walk) 原生 GUI，无 CGO 依赖。

## 功能特性

- **阿里云 ECS 监控**: 自动发现所有实例，显示 CPU 占用率、内存占用、运行状态
- **ESXi 监控**: 通过 VMware Host Client REST API 获取数据，支持 frp 内网穿透访问，显示 CPU 占用率、内存占用、运行中 VM 数量
- **DeepSeek 余额查询**: 实时显示账户余额和已用金额
- **置顶悬浮窗口**: 始终置顶，可拖动，可隐藏到系统托盘
- **自动刷新**: 每 5 秒自动刷新数据
- **设置界面**: 统一窗口编辑各模块凭证（支持粘贴、多 Tab 切换）

## 安全设计

**凭证仅保存在内存中，绝不写入磁盘。**

- `config.json` 只保存非敏感设置（region、base_url、insecure、模块显隐开关）
- 每次启动程序需在设置窗口重新输入凭证
- 凭证在程序退出时被清除

## 编译和运行

### 前置要求

Go 1.21+（本项目在 Go 1.27 下开发验证）

### 编译

```bash
cd statusbar
go mod tidy
go build -o statusbar.exe .
```

### 运行

```bash
./statusbar.exe
```

程序启动后，点击窗口底部的 **设置** 按钮（或右键系统托盘图标选择 **设置...**），在设置窗口中输入各模块凭证。设置窗口采用多 Tab 布局（阿里云 / ESXi / DeepSeek），密码字段支持粘贴。

## 配置文件

程序会在同目录下自动生成 `config.json`（仅含非敏感设置）：

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
- 支持从剪贴板粘贴凭证到密码输入框

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
