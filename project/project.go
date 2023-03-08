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

type ProjectArgs struct {
	Name      string
	Folders   []*folder.Folder
	ApiServer *apiserver.ApiServer
}

func NewProject(args *ProjectArgs) *Project {
	folders := folder.FoldersMap{}
	if args.Folders != nil {
		for _, f := range args.Folders {
			folders[f.Dir] = f
			args.ApiServer.AddFolder(f)
		}
	}
	return &Project{
		Name:        args.Name,
		Folders:     folders,
		ApiServer:   args.ApiServer,
		UserManager: iam.NewUserManager(),
	}
}

func (p *Project) AddFolder(f *folder.Folder) {
	p.Folders[f.Dir] = f
	p.ApiServer.AddFolder(f)
}
