package config

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
)

type Config struct {
	Aliyun    AliyunConfig    `json:"aliyun"`
	ESXi      ESXiConfig      `json:"esxi"`
	DeepSeek  DeepSeekConfig  `json:"deepseek"`
	Display   DisplayConfig   `json:"display"`
}

type AliyunConfig struct {
	Region string `json:"region"`
}

type ESXiConfig struct {
	Insecure bool `json:"insecure"`
}

type DeepSeekConfig struct {
	BaseURL string `json:"base_url"`
}

type DisplayConfig struct {
	ShowAliyun   bool `json:"show_aliyun"`
	ShowESXi     bool `json:"show_esxi"`
	ShowDeepSeek bool `json:"show_deepseek"`
}

func DefaultConfig() *Config {
	return &Config{
		Aliyun: AliyunConfig{
			Region: "cn-hangzhou",
		},
		ESXi: ESXiConfig{
			Insecure: true,
		},
		DeepSeek: DeepSeekConfig{
			BaseURL: "https://api.deepseek.com",
		},
		Display: DisplayConfig{
			ShowAliyun:   true,
			ShowESXi:     true,
			ShowDeepSeek: true,
		},
	}
}

func ConfigPath() string {
	exe, _ := os.Executable()
	return filepath.Join(filepath.Dir(exe), "config.json")
}

func Load() (*Config, error) {
	path := ConfigPath()
	data, err := os.ReadFile(path)
	if os.IsNotExist(err) {
		cfg := DefaultConfig()
		if err := cfg.Save(); err != nil {
			return nil, fmt.Errorf("create default config: %w", err)
		}
		return cfg, nil
	}
	if err != nil {
		return nil, fmt.Errorf("read config: %w", err)
	}
	cfg := DefaultConfig()
	if err := json.Unmarshal(data, cfg); err != nil {
		return nil, fmt.Errorf("parse config: %w", err)
	}
	return cfg, nil
}

func (c *Config) Save() error {
	data, err := json.MarshalIndent(c, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(ConfigPath(), data, 0644)
}
