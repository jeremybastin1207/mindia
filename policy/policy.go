package policy

import (
	"fmt"
	"path/filepath"
	"strings"
)

type Policy struct {
	Name            string `json:"name"`
	Width           int32  `json:"width"`
	Height          int32  `json:"height"`
	Format          string `json:"format"`
	TransformerName string `json:"transformer_name"`
}

type PolicyInput struct {
	Name            string
	Width           int32
	Height          int32
	Format          string
	TransformerName string
}

func NewPolicy(p *PolicyInput) Policy {
	return Policy{
		Name:            p.Name,
		Width:           p.Width,
		Height:          p.Height,
		Format:          p.Format,
		TransformerName: p.TransformerName,
	}
}

func (p *Policy) GetName(filename string) string {
	extension := filepath.Ext(filename)
	basename := strings.TrimSuffix(filename, extension)

	return fmt.Sprintf("%s_%s%s", basename, p.Name, extension)
}

func (p *Policy) IsOf(name string) bool {
	return strings.Contains(name, p.Name)
}
