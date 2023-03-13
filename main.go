package main

import (
	"mindia/apiserver"
	"mindia/automation"
	"mindia/automation/namer"
	"mindia/configurer"
	"mindia/folder"
	"mindia/policy"
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

	automationXl := automation.NewAutomation(&automation.AutomationConfig{
		Namer: namer.NewNamerSuffix(&namer.NamerSuffixConfig{
			Suffix: "xl",
		}),
		Steps: []automation.AutomationDoer{
			automation.NewResizer(&automation.ResizerConfig{
				AutomationStepConfig: &automation.AutomationStepConfig{
					Children: []*automation.Automation{},
				},
				Size: types.Size{
					Width:  250,
					Height: 250,
				},
			}),
		},
	})
	automationMd := automation.NewAutomation(&automation.AutomationConfig{
		Namer: namer.NewNamerSuffix(&namer.NamerSuffixConfig{
			Suffix: "md",
		}),
		Steps: []automation.AutomationDoer{
			automation.NewResizer(&automation.ResizerConfig{
				AutomationStepConfig: &automation.AutomationStepConfig{
					Children: []*automation.Automation{},
				},
				Size: types.Size{
					Width:  150,
					Height: 150,
				},
			}),
		},
	})
	automation1 := automation.NewAutomation(&automation.AutomationConfig{
		Namer: namer.NewNamerUuid(&namer.NamerUuidConfig{}),
		Steps: []automation.AutomationDoer{
			automation.NewJpegConverter(&automation.JpegConverterConfig{
				AutomationStepConfig: &automation.AutomationStepConfig{
					Children: []*automation.Automation{
						automationXl,
						automationMd,
					},
				},
			}),
		},
	})
	automations := []*folder.Automation{
		{
			Automation:          automation1,
			ApplyToCurrentFiles: true,
		},
	}

	policy1 := policy.NewPolicy(&policy.PolicyConfig{
		ContentTypesAllowed: []string{"image/jpeg"},
		ContentLengthMax:    10000000,
	})
	policies := []*policy.Policy{policy1}

	folder1 := folder.NewFolder(&folder.FolderConfig{
		Dir:         "/houses",
		Storage:     filesystemStorage,
		Backup:      filesystemBackupStorage,
		Automations: automations,
		Policies:    policies,
	})
	folder2 := folder.NewFolder(&folder.FolderConfig{
		Dir:         "/houses/garden",
		Storage:     filesystemStorage,
		Automations: automations,
	})
	folder3 := folder.NewFolder(&folder.FolderConfig{
		Dir:         "/users",
		Storage:     s3Storage,
		Backup:      s3BackupStorage,
		Automations: automations,
	})
	folder4 := folder.NewFolder(&folder.FolderConfig{
		Dir:         "/users/company",
		Storage:     s3Storage,
		Automations: automations,
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
