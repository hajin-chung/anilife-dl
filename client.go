package main

import (
	"encoding/json"
	"io"
	"net/http"
)

type LifeClient struct {
	*http.Client
	Referer string
}

func NewLifeClient(limit int) *LifeClient {
	return &LifeClient{
		Client:  &http.Client{},
		Referer: "",
	}
}

func (c *LifeClient) Get(url string) (*http.Response, error) {
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("User-Agent", userAgent)
	if len(c.Referer) != 0 {
		req.Header.Set("Referer", c.Referer)
	}
	req.Header.Set("Origin", anilifeUrl)

	return c.Do(req)
}

func (c *LifeClient) GetBody(url string) ([]byte, error) {
	res, err := c.Get(url)
	if err != nil {
		return nil, err
	}
	defer res.Body.Close()

	bytes, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, err
	}
	return bytes, nil
}

func (c *LifeClient) GetJson(url string, v any) error {
	bytes, err := c.GetBody(url)
	if err != nil {
		return err
	}

	err = json.Unmarshal(bytes, v)
	if err != nil {
		return err
	}
	return nil
}
