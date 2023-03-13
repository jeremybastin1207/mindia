# Mindia [Under construction]

Mindia is a flexible tool that helps you manage your media assets.

Place your media assets in virtual folders which can be configured individually.

### Define storages

```go
filesystemStorage := storage.NewFileSystemStorage(&storage.FilesystemStorageConfig{
  MountDir: "./data",
})

s3Storage := storage.NewS3Storage(&storage.S3StorageConfig{
  S3ClientConfig: &storage.S3ClientConfig{
    Bucket:          os.Getenv("BUCKET"),
    Region:          os.Getenv("REGION"),
    Endpoint:        os.Getenv("ENDPOINT"),
    AccessKeyId:     os.Getenv("ACCESS_KEY_ID"),
    SecretAccessKey: os.Getenv("SECRET_ACCESS_KEY"),
  },
})
```

### Define automations

```go
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
```

### Define policies

```go
policy1 := policy.NewPolicy(&policy.PolicyConfig{
ContentTypesAllowed: []string{"image/jpeg", "image/jpg", "image/png"},
ContentLengthMax: 10000000,
})
```

### Define folders

```go
folder1 := folder.NewFolder(&folder.FolderConfig{
  Dir: "/houses",
  Storage: filesystemStorage,
  Backup: s3BackupStorage,
  Automations: automations,
  Policies: policies,
})

folder2 := folder.NewFolder(&folder.FolderConfig{
  Dir: "/users/profile",
  Storage: filesystemStorage,
  Automations: automations,
})
```

### Define api, project and configurer

```go
apiServer := apiserver.NewApiServer(&apiserver.ApiServerConfig{
  Port: 3500,
})

project1 := project.NewProject(&project.ProjectConfig{
  Name: "my project",
  ApiServer: apiServer,
  Folders: []\*folder.Folder{ folder1, folder2 },
})

configurer := configurer.NewConfigurer()
configurer.PersistConfig(project1)

apiServer.Serve()
```
