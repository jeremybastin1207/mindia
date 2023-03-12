package namer

import (
	"mindia/utils"
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

func (n *NamerUuid) Name(filename string) string {
	return uuid.New().String() + filepath.Ext(filename)
}

func (n *NamerUuid) IsOf(name string) bool {
	_, err := uuid.Parse(utils.NameWithoutExt(name))
	return err == nil
}
