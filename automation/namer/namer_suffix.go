package namer

import (
	"fmt"
	"path/filepath"
	"strings"
)

type NamerSuffixConfig struct {
	Suffix string `yaml:"suffix"`
}

type NamerSuffix struct {
	*NamerSuffixConfig `yaml:",inline"`
}

func NewNamerSuffix(config *NamerSuffixConfig) *NamerSuffix {
	return &NamerSuffix{
		NamerSuffixConfig: config,
	}
}

func (n *NamerSuffix) Name(fileame string) string {
	extension := filepath.Ext(fileame)
	basename := strings.TrimSuffix(fileame, extension)
	return fmt.Sprintf("%s_%s%s", basename, n.Suffix, extension)
}

func (n *NamerSuffix) IsOf(name string) bool {
	return strings.Contains(name, n.Suffix)
}
