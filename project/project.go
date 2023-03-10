package project

import (
	"mindia/apiserver"
	"mindia/folder"
)

type ProjectConfig struct {
	Name      string               `yaml:"name"`
	ApiServer *apiserver.ApiServer `yaml:"api_server"`
	Folders   []*folder.Folder     `yaml:"folders"`
}

type Project struct {
	*ProjectConfig `yaml:",inline"`
}

func NewProject(config *ProjectConfig) *Project {
	p := &Project{
		ProjectConfig: config,
	}
	for _, f := range p.Folders {
		p.ApiServer.AddFolder(f)
	}
	return p
}
