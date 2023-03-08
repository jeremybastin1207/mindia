package storage

import (
	"encoding/json"
	"io"
	"io/ioutil"
	"net/http"
	"strings"
)

type DropboxError struct {
	Status     string
	StatusCode int
	Summary    string `json:"error_summary"`
}

func (e *DropboxError) Error() string {
	return e.Summary
}

type DropboxConfig struct {
	HTTPClient  *http.Client
	AccessToken string
}

func NewConfig(accessToken string) *DropboxConfig {
	return &DropboxConfig{
		HTTPClient:  http.DefaultClient,
		AccessToken: accessToken,
	}
}

type DropboxClient struct {
	*DropboxConfig
}

func NewDropboxClient(config *DropboxConfig) *DropboxClient {
	return &DropboxClient{
		DropboxConfig: config,
	}
}

func (c *DropboxClient) do(req *http.Request) (io.ReadCloser, int64, error) {
	res, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, 0, err
	}

	if res.StatusCode < 400 {
		return res.Body, res.ContentLength, err
	}

	defer res.Body.Close()

	e := &DropboxError{
		Status:     http.StatusText(res.StatusCode),
		StatusCode: res.StatusCode,
	}

	kind := res.Header.Get("Content-Type")

	if strings.Contains(kind, "text/plain") {
		if b, err := ioutil.ReadAll(res.Body); err == nil {
			e.Summary = string(b)
			return nil, 0, e
		}
		return nil, 0, err
	}

	if err := json.NewDecoder(res.Body).Decode(e); err != nil {
		return nil, 0, err
	}

	return nil, 0, e
}
