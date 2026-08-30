package ui

import (
	"fmt"
	"strings"
	"time"

	"github.com/lxn/walk"
	"github.com/lxn/walk/declarative"
	"github.com/lxn/win"

	"statusbar/collector"
	"statusbar/config"
	"statusbar/credential"
)

type StatusBar struct {
	mw        *walk.MainWindow
	tray      *walk.NotifyIcon
	config    *config.Config
	aliyunLbl *walk.Label
	esxiLbl   *walk.Label
	dsLbl     *walk.Label
	statusLbl *walk.Label
}

func New(cfg *config.Config) *StatusBar {
	return &StatusBar{
		config: cfg,
	}
}

func (sb *StatusBar) Run() {
	var tmpAliyunLbl, tmpEsxiLbl, tmpDsLbl, tmpStatusLbl *walk.Label

	err := declarative.MainWindow{
		AssignTo: &sb.mw,
		Title:    "Server Status Bar",
		MinSize:  declarative.Size{Width: 400, Height: 230},
		MaxSize:  declarative.Size{Width: 400, Height: 230},
		Layout:   declarative.VBox{Margins: declarative.Margins{Left: 12, Top: 8, Right: 12, Bottom: 8}, Spacing: 4},
		Children: []declarative.Widget{
			declarative.Label{
				Text: "  Server Monitor",
				Font: declarative.Font{Family: "Microsoft YaHei", PointSize: 13, Bold: true},
			},
			declarative.Label{
				AssignTo: &tmpAliyunLbl,
				Text:     "  阿里云: [未配置]",
			},
			declarative.Label{
				AssignTo: &tmpEsxiLbl,
				Text:     "  ESXi: [未配置]",
			},
			declarative.Label{
				AssignTo: &tmpDsLbl,
				Text:     "  DeepSeek: [未配置]",
			},
			declarative.Label{
				AssignTo: &tmpStatusLbl,
				Text:     "  上次刷新: --:--:--",
			},
			declarative.HSpacer{},
			declarative.PushButton{
				Text: "设置",
				OnClicked: func() {
					sb.openSettings()
				},
			},
		},
	}.Create()

	if err != nil {
		fmt.Printf("创建窗口失败: %v\n", err)
		return
	}

	sb.aliyunLbl = tmpAliyunLbl
	sb.esxiLbl = tmpEsxiLbl
	sb.dsLbl = tmpDsLbl
	sb.statusLbl = tmpStatusLbl

	// Always on top
	hwnd := sb.mw.Handle()
	win.SetWindowPos(hwnd, win.HWND_TOPMOST, 0, 0, 0, 0,
		uint32(win.SWP_NOMOVE|win.SWP_NOSIZE))

	// Center on screen
	screenWidth := int(win.GetSystemMetrics(0))
	screenHeight := int(win.GetSystemMetrics(1))
	clientRect := sb.mw.ClientBoundsPixels()
	x := (screenWidth - clientRect.Width) / 2
	y := (screenHeight - clientRect.Height) / 2
	win.SetWindowPos(hwnd, 0, int32(x), int32(y), 0, 0,
		uint32(win.SWP_NOSIZE|win.SWP_NOZORDER))

	// Setup tray icon
	sb.setupTray()

	// Show window
	sb.mw.Show()

	// Start refresh
	go sb.refreshLoop()

	sb.mw.Run()
	if sb.tray != nil {
		sb.tray.Dispose()
	}
	credential.Global().Clear()
	walk.App().Exit(0)
}

func (sb *StatusBar) setupTray() {
	var err error
	sb.tray, err = walk.NewNotifyIcon(sb.mw)
	if err != nil {
		fmt.Printf("创建托盘图标失败: %v\n", err)
		return
	}

	if icon := sb.mw.Icon(); icon != nil {
		sb.tray.SetIcon(icon)
	}
	sb.tray.SetToolTip("Server Status Bar")
	sb.tray.SetVisible(true)

	menu := sb.tray.ContextMenu()

	// Show/Hide
	showHideAction := walk.NewAction()
	showHideAction.SetText("显示/隐藏")
	showHideAction.Triggered().Attach(func() {
		if sb.mw.Visible() {
			sb.mw.Hide()
		} else {
			sb.mw.Show()
		}
	})
	menu.Actions().Add(showHideAction)

	menu.Actions().Add(walk.NewSeparatorAction())

	// Settings
	settingsAction := walk.NewAction()
	settingsAction.SetText("设置...")
	settingsAction.Triggered().Attach(func() {
		sb.openSettings()
	})
	menu.Actions().Add(settingsAction)

	menu.Actions().Add(walk.NewSeparatorAction())

	// Aliyun toggle
	aliyunAction := walk.NewAction()
	aliyunAction.SetText("阿里云")
	aliyunAction.SetCheckable(true)
	aliyunAction.SetChecked(true)
	aliyunAction.Triggered().Attach(func() {
		sb.config.Display.ShowAliyun = !sb.config.Display.ShowAliyun
		sb.config.Save()
		aliyunAction.SetChecked(sb.config.Display.ShowAliyun)
		sb.doRefresh()
	})
	menu.Actions().Add(aliyunAction)

	// ESXi toggle
	esxiAction := walk.NewAction()
	esxiAction.SetText("ESXi")
	esxiAction.SetCheckable(true)
	esxiAction.SetChecked(true)
	esxiAction.Triggered().Attach(func() {
		sb.config.Display.ShowESXi = !sb.config.Display.ShowESXi
		sb.config.Save()
		esxiAction.SetChecked(sb.config.Display.ShowESXi)
		sb.doRefresh()
	})
	menu.Actions().Add(esxiAction)

	// DeepSeek toggle
	dsAction := walk.NewAction()
	dsAction.SetText("DeepSeek")
	dsAction.SetCheckable(true)
	dsAction.SetChecked(true)
	dsAction.Triggered().Attach(func() {
		sb.config.Display.ShowDeepSeek = !sb.config.Display.ShowDeepSeek
		sb.config.Save()
		dsAction.SetChecked(sb.config.Display.ShowDeepSeek)
		sb.doRefresh()
	})
	menu.Actions().Add(dsAction)

	menu.Actions().Add(walk.NewSeparatorAction())

	// Quit
	quitAction := walk.NewAction()
	quitAction.SetText("退出")
	quitAction.Triggered().Attach(func() {
		walk.App().Exit(0)
	})
	menu.Actions().Add(quitAction)

	// Click tray icon to show/hide
	sb.tray.MouseUp().Attach(func(x, y int, button walk.MouseButton) {
		if button == walk.LeftButton {
			if sb.mw.Visible() {
				sb.mw.Hide()
			} else {
				sb.mw.Show()
			}
		}
	})
}

func (sb *StatusBar) openSettings() {
	if sb.mw == nil {
		return
	}
	showSettingsDialog(sb.mw)
	sb.updateLabels()
}

func (sb *StatusBar) updateLabels() {
	if !sb.config.Display.ShowAliyun || !credential.Global().HasAliyun() {
		sb.safeSetText(sb.aliyunLbl, "  阿里云: [未配置]")
	}
	if !sb.config.Display.ShowESXi || !credential.Global().HasESXi() {
		sb.safeSetText(sb.esxiLbl, "  ESXi: [未配置]")
	}
	if !sb.config.Display.ShowDeepSeek || !credential.Global().HasDeepSeek() {
		sb.safeSetText(sb.dsLbl, "  DeepSeek: [未配置]")
	}
	sb.doRefresh()
}

func (sb *StatusBar) refreshLoop() {
	ticker := time.NewTicker(5 * time.Second)
	defer ticker.Stop()

	sb.doRefresh()

	for range ticker.C {
		sb.doRefresh()
	}
}

func (sb *StatusBar) doRefresh() {
	if sb.config.Display.ShowAliyun && credential.Global().HasAliyun() {
		go sb.refreshAliyun()
	} else {
		sb.safeSetText(sb.aliyunLbl, "  阿里云: [未配置]")
	}

	if sb.config.Display.ShowESXi && credential.Global().HasESXi() {
		go sb.refreshESXi()
	} else {
		sb.safeSetText(sb.esxiLbl, "  ESXi: [未配置]")
	}

	if sb.config.Display.ShowDeepSeek && credential.Global().HasDeepSeek() {
		go sb.refreshDeepSeek()
	} else {
		sb.safeSetText(sb.dsLbl, "  DeepSeek: [未配置]")
	}

	now := time.Now().Format("15:04:05")
	sb.safeSetText(sb.statusLbl, fmt.Sprintf("  上次刷新: %s", now))
}

func isBlank(s string) bool {
	return strings.TrimSpace(s) == ""
}

func (sb *StatusBar) safeSetText(lbl *walk.Label, text string) {
	if lbl == nil || sb.mw == nil {
		return
	}
	sb.mw.Synchronize(func() {
		if lbl.IsDisposed() {
			return
		}
		lbl.SetText(text)
	})
}

func (sb *StatusBar) refreshAliyun() {
	cred := credential.Global().GetAliyun()
	if cred == nil {
		return
	}

	c, err := collector.NewAliyunCollector(sb.config.Aliyun.Region, cred.AccessKeyID, cred.AccessKeySecret)
	if err != nil {
		sb.safeSetText(sb.aliyunLbl, fmt.Sprintf("  阿里云: 连接失败 - %v", err))
		return
	}

	instances, err := c.Collect()
	if err != nil {
		sb.safeSetText(sb.aliyunLbl, fmt.Sprintf("  阿里云: 查询失败 - %v", err))
		return
	}

	if len(instances) == 0 {
		sb.safeSetText(sb.aliyunLbl, "  阿里云: 无实例")
		return
	}

	for _, inst := range instances {
		text := fmt.Sprintf("  ECS: %s | CPU: %.1f%% | 内存: %.1f%% | %s",
			inst.InstanceName, inst.CPUUsage, inst.MemoryUsage, inst.Status)
		sb.safeSetText(sb.aliyunLbl, text)
	}
}

func (sb *StatusBar) refreshESXi() {
	cred := credential.Global().GetESXi()
	if cred == nil {
		return
	}

	c := collector.NewESXiCollector()
	info, err := c.Collect(cred.URL, cred.User, cred.Password)
	if err != nil {
		sb.safeSetText(sb.esxiLbl, fmt.Sprintf("  ESXi: 连接失败 - %v", err))
		return
	}

	text := fmt.Sprintf("  ESXi: %s | CPU: %.1f%% | 内存: %.1f%% | VM: %d/%d",
		info.HostName, info.CPUUsage, info.MemoryUsage, info.RunningVMs, info.TotalVMs)
	sb.safeSetText(sb.esxiLbl, text)
}

func (sb *StatusBar) refreshDeepSeek() {
	cred := credential.Global().GetDeepSeek()
	if cred == nil {
		return
	}

	c := collector.NewDeepSeekCollector(cred.APIKey, sb.config.DeepSeek.BaseURL)
	balance, err := c.Collect()
	if err != nil {
		sb.safeSetText(sb.dsLbl, fmt.Sprintf("  DeepSeek: 查询失败 - %v", err))
		return
	}

	text := fmt.Sprintf("  DeepSeek: 余额 ¥%.2f | 已用 ¥%.2f", balance.Balance, balance.TotalUsed)
	sb.safeSetText(sb.dsLbl, text)
}
