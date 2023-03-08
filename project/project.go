package project

import (
	"mindia/apiserver"
	"mindia/folder"
	"mindia/iam"
)

type Project struct {
	Name        string
	Folders     folder.FoldersMap
	ApiServer   *apiserver.ApiServer
	UserManager *iam.UserManager
}

type ProjectInput struct {
	Name      string
	Folders   []*folder.Folder
	ApiServer *apiserver.ApiServer
}

func NewProject(in *ProjectInput) *Project {
	folders := folder.FoldersMap{}
	if in.Folders != nil {
		for _, f := range in.Folders {
			folders[f.Dir] = f
			in.ApiServer.AddFolder(f)
		}
	}
	return &Project{
		Name:        in.Name,
		Folders:     folders,
		ApiServer:   in.ApiServer,
		UserManager: iam.NewUserManager(),
	}
}

func (p *Project) AddFolder(f *folder.Folder) {
	p.Folders[f.Dir] = f
	p.ApiServer.AddFolder(f)
}
