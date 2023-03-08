package main

import (
	"mindia/apiserver"
	"mindia/automation"
	"mindia/configurer"
	"mindia/folder"
	"mindia/project"
	"mindia/storage"
	"mindia/types"
	"mindia/utils"
	"os"

	"github.com/joho/godotenv"
	"github.com/rs/zerolog/log"
)

func loadEnv() {
	if _, err := os.Stat(".env"); err == nil {
		err := godotenv.Load(".env")
		if err != nil {
			utils.ExitErrorf("Unable to load .env, %v", err)
		}
	}
}

func main() {
	loadEnv()

	log.Info().Msg("starting mindia")

	filesystemStorage := storage.NewFileSystemStorage(&storage.FilesystemStorageConfig{
		MountDir: "./data",
	})
	filesystemBackupStorage := storage.NewFileSystemStorage(&storage.FilesystemStorageConfig{
		MountDir: "./data/backup",
	})
	s3Storage := storage.NewS3Storage(&storage.S3StorageConfig{
		S3ClientConfig: &storage.S3ClientConfig{
			Bucket:          "test-mindia-bucket",
			Region:          "ams3",
			Endpoint:        "https://ams3.digitaloceanspaces.com",
			AccessKeyId:     os.Getenv("ACCESS_KEY_ID"),
			SecretAccessKey: os.Getenv("SECRET_ACCESS_KEY"),
		},
	})
	s3BackupStorage := storage.NewS3Storage(&storage.S3StorageConfig{
		S3ClientConfig: &storage.S3ClientConfig{
			Bucket:          "test-backup-mindia-bucket",
			Region:          "ams3",
			Endpoint:        "https://ams3.digitaloceanspaces.com",
			AccessKeyId:     os.Getenv("ACCESS_KEY_ID"),
			SecretAccessKey: os.Getenv("SECRET_ACCESS_KEY"),
		},
	})

	automation1 := automation.NewAutomation(&automation.AutomationConfig{
		Name: "automation1",
		Steps: []automation.AutomationStep{
			automation.NewNamer(&automation.NamerConfig{
				Suffix: "xl",
			}),
			automation.NewResizer(&automation.ResizerConfig{
				Size: types.Size{
					Width:  200,
					Height: 200,
				},
			}),
		},
	})
	automation2 := automation.NewAutomation(&automation.AutomationConfig{
		Name: "automation2",
		Steps: []automation.AutomationStep{
			automation.NewNamer(&automation.NamerConfig{
				Suffix: "md",
			}),
			automation.NewResizer(&automation.ResizerConfig{
				Size: types.Size{
					Width:  100,
					Height: 100,
				},
			}),
		},
	})

	folder1 := folder.NewFolder(&folder.FolderConfig{
		Dir:     "/houses",
		Storage: filesystemStorage,
		Backup:  filesystemBackupStorage,
		Automations: []*automation.Automation{
			automation1,
			automation2,
		},
	})
	folder2 := folder.NewFolder(&folder.FolderConfig{
		Dir:     "/houses/garden",
		Storage: filesystemStorage,
		Automations: []*automation.Automation{
			automation1,
		},
	})
	folder3 := folder.NewFolder(&folder.FolderConfig{
		Dir:     "/users",
		Storage: s3Storage,
		Backup:  s3BackupStorage,
		Automations: []*automation.Automation{
			automation1,
		},
	})
	folder4 := folder.NewFolder(&folder.FolderConfig{
		Dir:     "/users/company",
		Storage: s3Storage,
		Automations: []*automation.Automation{
			automation1,
		},
	})

	apiServer := apiserver.NewApiServer(&apiserver.ApiServerConfig{
		Port: 3500,
	})

	project1 := project.NewProject(&project.ProjectConfig{
		Name:      "ae",
		ApiServer: apiServer,
		Folders: []*folder.Folder{
			folder1,
			folder2,
			folder3,
			folder4,
		},
	})

	configurer := configurer.NewConfigurer()
	configurer.PersistConfig(project1)

	apiServer.Serve()
}
