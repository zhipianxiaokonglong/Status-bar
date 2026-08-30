package collector

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"time"
)

type DeepSeekBalance struct {
	Balance   float64
	TotalUsed float64
	Currency  string
	UpdatedAt time.Time
}

type DeepSeekCollector struct {
	apiKey  string
	baseURL string
}

func NewDeepSeekCollector(apiKey, baseURL string) *DeepSeekCollector {
	if baseURL == "" {
		baseURL = "https://api.deepseek.com"
	}
	return &DeepSeekCollector{
		apiKey:  apiKey,
		baseURL: baseURL,
	}
}

func parseFloat(v interface{}) float64 {
	switch val := v.(type) {
	case float64:
		return val
	case string:
		f, _ := strconv.ParseFloat(val, 64)
		return f
	case json.Number:
		f, _ := val.Float64()
		return f
	default:
		return 0
	}
}

func (c *DeepSeekCollector) Collect() (*DeepSeekBalance, error) {
	if c.apiKey == "" {
		return nil, fmt.Errorf("DeepSeek API key not configured")
	}

	req, err := http.NewRequest("GET", c.baseURL+"/user/balance", nil)
	if err != nil {
		return nil, fmt.Errorf("create request: %w", err)
	}

	req.Header.Set("Authorization", "Bearer "+c.apiKey)
	req.Header.Set("Content-Type", "application/json")

	client := &http.Client{Timeout: 10 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("request balance: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("read response: %w", err)
	}

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("API error (status %d): %s", resp.StatusCode, string(body))
	}

	var rawResult map[string]interface{}
	if err := json.Unmarshal(body, &rawResult); err != nil {
		return nil, fmt.Errorf("parse response: %w", err)
	}

	balance := &DeepSeekBalance{
		UpdatedAt: time.Now(),
	}

	balanceInfos, ok := rawResult["balance_infos"].([]interface{})
	if !ok {
		return nil, fmt.Errorf("balance_infos not found or invalid type")
	}

	for _, item := range balanceInfos {
		info, ok := item.(map[string]interface{})
		if !ok {
			continue
		}

		currency, _ := info["currency"].(string)
		quotaType, _ := info["quota_type"].(string)

		bal := parseFloat(info["total_balance"])
		used := parseFloat(info["total_consumed"])

		if currency == "CNY" || quotaType == "balance" {
			balance.Balance = bal
			balance.TotalUsed = used
			balance.Currency = currency
			break
		}
	}

	if balance.Currency == "" && len(balanceInfos) > 0 {
		info, ok := balanceInfos[0].(map[string]interface{})
		if ok {
			balance.Balance = parseFloat(info["total_balance"])
			balance.TotalUsed = parseFloat(info["total_consumed"])
			balance.Currency, _ = info["currency"].(string)
		}
	}

	return balance, nil
}
