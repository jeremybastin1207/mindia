# Mindia

Mindia is a tool that helps you manage your web applications' medias.

The development is at the beginning and the code is far more an exploration and a sandbox than a production ready code.

```go
	filesystemStorage := storage.NewFileSystemStorage(&storage.FilesystemStorageConfig{
		MountDir: "./data",
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
		ContentTypesAllowed: []string{"image/jpeg", "image/jpg", "image/png"},
		ContentLengthMax:    10000000,
	})
	policies := []*policy.Policy{policy1}

	folder1 := folder.NewFolder(&folder.FolderConfig{
		Dir:         "/houses",
		Storage:     filesystemStorage,
		Backup:      s3BackupStorage,
		Automations: automations,
		Policies:    policies,
	})
	folder2 := folder.NewFolder(&folder.FolderConfig{
		Dir:         "/houses/castle",
		Storage:     filesystemStorage,
		Automations: automations,
	})

	apiServer := apiserver.NewApiServer(&apiserver.ApiServerConfig{
		Port: 3500,
	})

	project1 := project.NewProject(&project.ProjectConfig{
		Name:      "my project",
		ApiServer: apiServer,
		Folders: []*folder.Folder{ folder1, folder2 },
	})

	configurer := configurer.NewConfigurer()
	configurer.PersistConfig(project1)

	apiServer.Serve()
```
