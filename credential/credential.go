package credential

import "sync"

type AliyunCred struct {
	AccessKeyID     string
	AccessKeySecret string
}

type ESXiCred struct {
	URL      string
	User     string
	Password string
}

type DeepSeekCred struct {
	APIKey string
}

type Store struct {
	mu         sync.RWMutex
	aliyun     *AliyunCred
	esxi       *ESXiCred
	deepseek   *DeepSeekCred
}

var global = &Store{}

func Global() *Store {
	return global
}

func (s *Store) SetAliyun(c *AliyunCred) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.aliyun = c
}

func (s *Store) GetAliyun() *AliyunCred {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.aliyun
}

func (s *Store) SetESXi(c *ESXiCred) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.esxi = c
}

func (s *Store) GetESXi() *ESXiCred {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.esxi
}

func (s *Store) SetDeepSeek(c *DeepSeekCred) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.deepseek = c
}

func (s *Store) GetDeepSeek() *DeepSeekCred {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.deepseek
}

func (s *Store) HasAliyun() bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.aliyun != nil && s.aliyun.AccessKeyID != "" && s.aliyun.AccessKeySecret != ""
}

func (s *Store) HasESXi() bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.esxi != nil && s.esxi.URL != "" && s.esxi.User != "" && s.esxi.Password != ""
}

func (s *Store) HasDeepSeek() bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.deepseek != nil && s.deepseek.APIKey != ""
}

func (s *Store) Clear() {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.aliyun = nil
	s.esxi = nil
	s.deepseek = nil
}
