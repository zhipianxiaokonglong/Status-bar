package collector

import (
	"crypto/tls"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

type ESXiInfo struct {
	HostName    string  `json:"HostName"`
	CPUUsage    float64 `json:"CPUUsage"`
	MemoryUsage float64 `json:"MemoryUsage"`
	TotalMemory uint64  `json:"TotalMemory"`
	UsedMemory  uint64  `json:"UsedMemory"`
	RunningVMs  int     `json:"RunningVMs"`
	TotalVMs    int     `json:"TotalVMs"`
}

type ESXiCollector struct {
	client *http.Client
}

func NewESXiCollector() *ESXiCollector {
	transport := &http.Transport{
		TLSClientConfig: &tls.Config{InsecureSkipVerify: true},
	}
	return &ESXiCollector{
		client: &http.Client{
			Timeout:   15 * time.Second,
			Transport: transport,
		},
	}
}

type esxiSession struct {
	Value string `json:"value"`
}

type esxiHostSummary struct {
	Value struct {
		HostHardwareInfo struct {
			CpuMhz      int64 `json:"cpuMhz"`
			NumCpuCores int64 `json:"numCpuCores"`
			MemorySize  int64 `json:"memorySize"`
		} `json:"hostHardwareInfo"`
		HostHardwareSummary struct {
			Model  string `json:"model"`
			Name   string `json:"name"`
			OtherIdentifiers []struct {
				IdentifierType struct {
					Key string `json:"key"`
				} `json:"identifierType"`
				IdentifierValue string `json:"identifierValue"`
			} `json:"otherIdentifiers"`
		} `json:"hostHardwareSummary"`
		HostSystemResourceInfo struct {
			CpuUsage struct {
				UsageMhz int64 `json:"usageMhz"`
			} `json:"cpuUsage"`
			MemoryUsage struct {
				Usage   int64 `json:"usage"`
				Granted int64 `json:"granted"`
			} `json:"memoryUsage"`
		} `json:"hostSystemResourceInfo"`
	} `json:"value"`
}

type esxiVMList struct {
	Value []struct {
		Name   string `json:"name"`
		Value  string `json:"value"`
		VM     struct {
			Name   string `json:"name"`
			Value  string `json:"value"`
			PowerState string `json:"powerState"`
		} `json:"vm"`
	} `json:"value"`
}

func (c *ESXiCollector) login(baseURL, user, password string) (string, error) {
	url := strings.TrimRight(baseURL, "/") + "/rest/com/vmware/cis/session"

	req, err := http.NewRequest("POST", url, nil)
	if err != nil {
		return "", fmt.Errorf("create request: %w", err)
	}
	req.SetBasicAuth(user, password)
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.client.Do(req)
	if err != nil {
		return "", fmt.Errorf("login request: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return "", fmt.Errorf("login failed (status %d): %s", resp.StatusCode, string(body))
	}

	var session esxiSession
	if err := json.NewDecoder(resp.Body).Decode(&session); err != nil {
		return "", fmt.Errorf("decode session: %w", err)
	}

	return session.Value, nil
}

func (c *ESXiCollector) logout(baseURL, sessionID string) {
	url := strings.TrimRight(baseURL, "/") + "/rest/com/vmware/cis/session"
	req, _ := http.NewRequest("DELETE", url, nil)
	req.Header.Set("vmware-api-session-id", sessionID)
	c.client.Do(req)
}

func (c *ESXiCollector) apiGet(baseURL, sessionID, path string, result interface{}) error {
	url := strings.TrimRight(baseURL, "/") + path

	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return fmt.Errorf("create request: %w", err)
	}
	req.Header.Set("vmware-api-session-id", sessionID)

	resp, err := c.client.Do(req)
	if err != nil {
		return fmt.Errorf("request %s: %w", path, err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("request %s failed (status %d): %s", path, resp.StatusCode, string(body))
	}

	if err := json.NewDecoder(resp.Body).Decode(result); err != nil {
		return fmt.Errorf("decode response: %w", err)
	}
	return nil
}

func (c *ESXiCollector) Collect(rawURL, user, password string) (*ESXiInfo, error) {
	if rawURL == "" {
		return nil, fmt.Errorf("ESXi URL not configured")
	}

	baseURL := rawURL
	if !strings.HasPrefix(baseURL, "http") {
		baseURL = "https://" + baseURL
	}

	// Parse and normalize URL: strip paths like /ui/, /ui, /sdk etc.
	parsed, err := url.Parse(baseURL)
	if err != nil {
		return nil, fmt.Errorf("parse URL: %w", err)
	}
	// Keep only scheme://host:port
	baseURL = fmt.Sprintf("%s://%s", parsed.Scheme, parsed.Host)

	sessionID, err := c.login(baseURL, user, password)
	if err != nil {
		return nil, fmt.Errorf("login: %w", err)
	}
	defer c.logout(baseURL, sessionID)

	info := &ESXiInfo{}

	// Get host hardware and resource info
	var summary esxiHostSummary
	if err := c.apiGet(baseURL, sessionID, "/rest/vmware/host/summary", &summary); err != nil {
		return nil, fmt.Errorf("get host summary: %w", err)
	}

	hw := summary.Value.HostHardwareInfo
	info.TotalMemory = uint64(hw.MemorySize)

	totalCPU := hw.CpuMhz * hw.NumCpuCores
	if totalCPU > 0 {
		info.CPUUsage = float64(summary.Value.HostSystemResourceInfo.CpuUsage.UsageMhz) / float64(totalCPU) * 100
	}

	if hw.MemorySize > 0 {
		info.UsedMemory = uint64(summary.Value.HostSystemResourceInfo.MemoryUsage.Usage)
		info.MemoryUsage = float64(info.UsedMemory) / float64(hw.MemorySize) * 100
	}

	// Get VM list
	var vmList esxiVMList
	if err := c.apiGet(baseURL, sessionID, "/rest/vmware/host/vm-list", &vmList); err == nil {
		info.TotalVMs = len(vmList.Value)
		for _, vm := range vmList.Value {
			if strings.EqualFold(vm.VM.PowerState, "POWERED_ON") {
				info.RunningVMs++
			}
		}
	}

	// Use hostname from hardware summary if available
	if summary.Value.HostHardwareSummary.Name != "" {
		info.HostName = summary.Value.HostHardwareSummary.Name
	}

	return info, nil
}
