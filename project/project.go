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
	return &Project{
		ProjectConfig: config,
	}
}
