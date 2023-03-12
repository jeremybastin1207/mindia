package automation

import (
	"path/filepath"

	"github.com/google/uuid"
)

type NamerUuidConfig struct {
}

type NamerUuid struct {
	*NamerUuidConfig `yaml:",inline"`
}

func NewNamerUuid(config *NamerUuidConfig) *NamerUuid {
	return &NamerUuid{
		NamerUuidConfig: config,
	}
}

func (n *NamerUuid) NamerFunc(Name string) string {
	return uuid.New().String() + filepath.Ext(Name)
}
