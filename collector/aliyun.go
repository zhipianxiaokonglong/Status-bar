package collector

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/aliyun/alibaba-cloud-sdk-go/sdk/requests"
	"github.com/aliyun/alibaba-cloud-sdk-go/services/ecs"
)

type ECSInstance struct {
	InstanceID   string
	InstanceName string
	Status       string
	PublicIP     string
	CPUUsage     float64
	MemoryUsage  float64
	Region       string
}

type AliyunCollector struct {
	client *ecs.Client
	ak     string
	sk     string
	region string
}

func NewAliyunCollector(region, accessKeyID, accessKeySecret string) (*AliyunCollector, error) {
	if region == "" {
		region = "cn-hangzhou"
	}

	ecsClient, err := ecs.NewClientWithAccessKey(region, accessKeyID, accessKeySecret)
	if err != nil {
		return nil, fmt.Errorf("create ecs client: %w", err)
	}

	return &AliyunCollector{
		client: ecsClient,
		ak:     accessKeyID,
		sk:     accessKeySecret,
		region: region,
	}, nil
}

func (c *AliyunCollector) Collect() ([]ECSInstance, error) {
	req := ecs.CreateDescribeInstancesRequest()
	req.PageSize = requests.NewInteger(100)

	resp, err := c.client.DescribeInstances(req)
	if err != nil {
		return nil, fmt.Errorf("describe instances: %w", err)
	}

	var instances []ECSInstance
	for _, inst := range resp.Instances.Instance {
		ecsInst := ECSInstance{
			InstanceID:   inst.InstanceId,
			InstanceName: inst.InstanceName,
			Status:       inst.Status,
			Region:       c.region,
		}

		if len(inst.PublicIpAddress.IpAddress) > 0 {
			ecsInst.PublicIP = inst.PublicIpAddress.IpAddress[0]
		}

		ecsInst.CPUUsage = c.getInstanceCPU(inst.InstanceId)
		ecsInst.MemoryUsage = c.getInstanceMemory(inst.InstanceId)

		instances = append(instances, ecsInst)
	}

	return instances, nil
}

func (c *AliyunCollector) getInstanceCPU(instanceID string) float64 {
	url := fmt.Sprintf("https://ecs.%s.aliyuncs.com/?Action=DescribeInstanceMonitorData&InstanceID=%s&MetricName=CPUUtilization&Period=60&Length=1&Format=JSON",
		c.region, instanceID)
	return c.queryMetric(url)
}

func (c *AliyunCollector) getInstanceMemory(instanceID string) float64 {
	url := fmt.Sprintf("https://ecs.%s.aliyuncs.com/?Action=DescribeInstanceMonitorData&InstanceID=%s&MetricName=MemoryUtilization&Period=60&Length=1&Format=JSON",
		c.region, instanceID)
	return c.queryMetric(url)
}

func (c *AliyunCollector) queryMetric(url string) float64 {
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return 0
	}

	timestamp := time.Now().UTC().Format("2006-01-02T15:04:05Z")
	req.Header.Set("Date", timestamp)
	req.Header.Set("Authorization", fmt.Sprintf("acs %s:%s", c.ak, c.sk))

	client := &http.Client{Timeout: 10 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return 0
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return 0
	}

	var result struct {
		MonitorData struct {
			MonitorData []struct {
				Average float64 `json:"Average"`
			} `json:"MonitorData"`
		} `json:"MonitorData"`
	}

	if err := json.Unmarshal(body, &result); err != nil {
		return 0
	}

	if len(result.MonitorData.MonitorData) > 0 {
		return result.MonitorData.MonitorData[0].Average
	}
	return 0
}

func FormatFloat(f float64) string {
	return strconv.FormatFloat(f, 'f', 1, 64)
}

func FormatPercent(val string) string {
	return strings.TrimSpace(val) + "%"
}
