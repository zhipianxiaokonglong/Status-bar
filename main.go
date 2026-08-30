package main

import (
	"fmt"
	"os"

	"statusbar/config"
	"statusbar/ui"
)

func main() {
	fmt.Println("=== Server Status Bar ===")
	fmt.Println("正在加载配置...")

	cfg, err := config.Load()
	if err != nil {
		fmt.Printf("加载配置失败: %v\n", err)
		fmt.Println("使用默认配置继续...")
		cfg = config.DefaultConfig()
	}

	fmt.Println("启动状态栏...")
	statusBar := ui.New(cfg)
	statusBar.Run()

	os.Exit(0)
}
