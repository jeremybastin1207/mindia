package project

import (
	"mindia/apiserver"
	"mindia/folder"
)

type ProjectConfig struct {
	Name      string               `yaml:"name"`
	ApiServer *apiserver.ApiServer `yaml:"api_server"`
}

type Project struct {
	*ProjectConfig `yaml:",inline"`
	Folders        folder.FoldersMap `yaml:"folders"`
}

func NewProject(config *ProjectConfig) *Project {
	return &Project{
		Folders:       folder.FoldersMap{},
		ProjectConfig: config,
	}
}

func (p *Project) AddFolder(f *folder.Folder) {
	p.Folders[f.Dir] = f
	p.ApiServer.AddFolder(f)
}
