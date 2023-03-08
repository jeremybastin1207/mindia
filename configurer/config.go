package configurer

import (
	"mindia/apiserver"
	"mindia/folder"
	"mindia/policy"
	"mindia/project"
	"mindia/storage"
)

type PolicyConfig struct {
	Name   string `yaml:"name"`
	Width  int32  `yaml:"width"`
	Height int32  `yaml:"height"`
	Format string `yaml:"format"`
}

type ApiServerConfig struct {
	Port int `yaml:"port"`
}

type StorageConfig struct {
	StorageType string `yaml:"type"`
	MountDir    string `yaml:"mount_dir,omitempty"`
	Bucket      string `yaml:"bucket,omitempty"`
}

type FolderConfig struct {
	Dir     string        `yaml:"dir"`
	Storage StorageConfig `yaml:"storage"`

	Policies []PolicyConfig `yaml:"policies"`
}

type ProjectConfig struct {
	Name      string          `yaml:"project_name"`
	Folders   []FolderConfig  `yaml:"folders"`
	ApiServer ApiServerConfig `yaml:"api_server"`
}

func policyToConfig(p policy.Policy) PolicyConfig {
	return PolicyConfig{
		Name:   p.Name,
		Width:  p.Width,
		Height: p.Height,
		Format: p.Format,
	}
}

func storageToConfig(s storage.Storage) StorageConfig {
	switch v := s.(type) {
	case *storage.S3Storage:
		return StorageConfig{
			StorageType: "s3_storage",
			Bucket:      v.Config().Bucket,
		}
	case *storage.FilesystemStorage:
		return StorageConfig{
			StorageType: "filesystem_storage",
			MountDir:    v.Config().MountDir,
		}
	default:
		return StorageConfig{}
	}
}

func apiServerToConfig(s apiserver.ApiServer) ApiServerConfig {
	return ApiServerConfig{
		Port: s.Port,
	}
}

func folderToConfig(f *folder.Folder) FolderConfig {
	/* 	pp, _ := f.ReadPolicies()

	   	pp2 := []PolicyConfig{}
	   	for _, p := range pp {
	   		pp2 = append(pp2, policyToConfig(*p))
	   	} */

	return FolderConfig{
		Dir:     f.Dir,
		Storage: storageToConfig(f.Storage),
		//Policies: pp2,
	}
}

func projectToConfig(p *project.Project) ProjectConfig {
	pc := ProjectConfig{
		Name:      p.Name,
		ApiServer: apiServerToConfig(*p.ApiServer),
	}
	for _, f := range p.Folders {
		pc.Folders = append(pc.Folders, folderToConfig(f))
	}
	return pc
}
