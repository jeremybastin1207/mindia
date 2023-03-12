package automation

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

func (n *NamerSuffix) NamerFunc(Name string) string {
	extension := filepath.Ext(Name)
	basename := strings.TrimSuffix(Name, extension)
	return fmt.Sprintf("%s_%s%s", basename, n.Suffix, extension)
}
